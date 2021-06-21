use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::Builder;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub working_directory: PathBuf,
    pub inputs: Vec<PathBuf>,
    pub outputs: Vec<PathBuf>,
}

impl Job {
    pub fn run(&self) -> Result<Output> {
        let work_dir = Builder::new()
            .prefix(format!("job-{}", self.command).as_str())
            .tempdir()
            .context("couldn't create a temporary directory for the job")?;

        self.prepare_workspace(work_dir.path())
            .context("couldn't prepare files in the working directory")?;

        let output = self
            .run_job(work_dir.path())
            .context("couldn't run the job")?;

        self.tear_down_workspace(work_dir.path())
            .context("couldn't tear down the workspace")?;

        work_dir
            .close()
            .context("couldn't clean up the job's temporary directory")?;

        Ok(output)
    }

    fn prepare_workspace(&self, work_dir: &Path) -> Result<()> {
        for input in &self.inputs {
            let meta = fs::metadata(input)
                .with_context(|| format!("couldn't read metadata for {}", input.display()))?;

            // Copy all the files over. Note that we can't just create symlinks
            // to the files in all cases because that would allow the job code
            // to modify source files on disk as a side effect. We probably want
            // to allow at least directory symlinks in the future (for caches,
            // for instance) and we'll need to extend the `inputs` concept then.
            if meta.is_dir() {
                for item in WalkDir::new(input) {
                    let entry =
                        item.with_context(|| format!("couldn't walk through {}", input.display()))?;

                    let dest = self
                        .path_in_workspace(work_dir, &entry.path().to_path_buf())
                        .with_context(|| {
                            format!(
                                "couldn't get a path to {} in the workspace",
                                input.display()
                            )
                        })?;

                    if entry.file_type().is_dir() {
                        fs::create_dir(&dest)
                            .with_context(|| format!("couldn't create {}", dest.display()))?
                    } else {
                        // assuming file or symlink
                        fs::copy(entry.path(), &dest).with_context(|| {
                            format!(
                                "couldn't copy {} to {}",
                                entry.path().display(),
                                dest.display()
                            )
                        })?;
                    }
                }
            } else {
                // it's a file, or maybe a symlink
                let dest = self.path_in_workspace(work_dir, input).with_context(|| {
                    format!(
                        "couldn't get a path to {} in the workspace",
                        input.display()
                    )
                })?;

                match dest.parent() {
                    Some(parent) => fs::create_dir_all(parent).with_context(|| format!("couldn't make the parent directories for {}", dest.display()))?,
                    None => bail!("couldn't create the directories leading to {}. That probably means it's at the filesystem root, but we should have excluded that possibility already. This is a bug and should be reported.", dest.display())
                };
                fs::copy(&input, &dest).with_context(|| {
                    format!("couldn't copy {} to {}", input.display(), dest.display())
                })?;
            }
        }

        Ok(())
    }

    fn run_job(&self, work_dir: &Path) -> Result<Output> {
        Command::new(self.command.as_str())
            .args(
                self.arguments
                    .iter()
                    .map(|arg| arg.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            )
            .current_dir(&work_dir)
            // TODO: this is going to have to retain some environment variables
            // for software to work correctly. For example, we'll probably need
            // to provide a fake HOME the way Nix does.
            .env_clear()
            .output()
            .context("couldn't run the command")
    }

    fn tear_down_workspace(&self, work_dir: &Path) -> Result<()> {
        for output in &self.outputs {
            let source = self
                .path_in_workspace(work_dir, &output)
                .context("couldn't determine path for output")?;

            let dest = self.working_directory.join(output);

            match dest.parent() {
                    Some(parent) => fs::create_dir_all(parent).with_context(|| format!("couldn't make the parent directories for {}", dest.display()))?,
                    None => bail!("couldn't create the directories leading to {}. That probably means it's at the filesystem root, but we should have excluded that possibility already. This is a bug and should be reported.", dest.display())
                };
            fs::copy(&source, &dest).with_context(|| {
                format!("couldn't copy {} to {}", source.display(), dest.display())
            })?;
        }

        Ok(())
    }

    fn path_in_workspace(&self, work_dir: &Path, input: &PathBuf) -> Result<PathBuf> {
        if let Ok(relative) = input.strip_prefix(&self.working_directory) {
            Ok(work_dir.join(relative))
        } else if input.is_relative() {
            Ok(work_dir.join(input))
        } else {
            bail!(
                "couldn't isolate {} because it's outside the working directory ({})",
                input.display(),
                self.working_directory.display(),
            );
        }
    }
}

#[cfg(test)]
mod test_job {
    use super::Job;
    use std::collections::HashMap;
    use std::fs;
    use std::fs::File;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn produces_output() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("hello.txt");

        let job = Job {
            command: "bash".to_string(),
            arguments: vec![
                "-c".to_string(),
                format!("echo Hello, World > {}", dest.to_str().unwrap()),
            ],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(output.status.success(), true);

        let contents = std::fs::read_to_string(dest).unwrap();
        assert_eq!(contents.as_str(), "Hello, World\n");
    }

    #[test]
    fn captures_stdout() {
        let job = Job {
            command: "echo".to_string(),
            arguments: vec!["Hello, Stdout!".to_string()],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "Hello, Stdout!\n".to_string()
        );
    }

    #[test]
    fn captures_stderr() {
        let job = Job {
            command: "bash".to_string(),
            arguments: vec!["-c".to_string(), "echo 'Hello, Stderr!' 1>&2".to_string()],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Hello, Stderr!\n".to_string()
        );
    }

    #[test]
    fn reports_a_problem() {
        let job = Job {
            command: "bash".to_string(),
            arguments: vec!["-c".to_string(), "exit 1".to_string()],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(output.status.success(), false);
    }

    #[test]
    fn isolates_environment() {
        let job = Job {
            command: "env".to_string(),
            arguments: vec![],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "".to_string());
    }

    #[test]
    fn only_inputs_are_visible() {
        let temp = tempdir().unwrap();

        let visible = temp.path().join("visible.txt");
        File::create(&visible).unwrap();

        let hidden = temp.path().join("hidden.txt");
        File::create(hidden).unwrap();

        let job = Job {
            command: "find".to_string(),
            arguments: vec![".".to_string()],
            environment: HashMap::default(),
            working_directory: temp.path().to_path_buf(),
            inputs: vec![visible],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            ".\n./visible.txt\n".to_string()
        );

        drop(temp);
    }

    #[test]
    fn files_in_input_directories_are_visible() {
        let temp = tempdir().unwrap();

        let dir = temp.path().join("visible");
        fs::create_dir(&dir).unwrap();
        File::create(&dir.join("a.txt")).unwrap();
        File::create(&dir.join("b.txt")).unwrap();

        let job = Job {
            command: "find".to_string(),
            arguments: vec![".".to_string(), "-type".to_string(), "file".to_string()],
            environment: HashMap::default(),
            working_directory: temp.path().to_path_buf(),
            inputs: vec![dir],
            outputs: vec![],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "./visible/b.txt\n./visible/a.txt\n".to_string()
        )
    }

    #[test]
    fn absolute_paths_outside_the_working_directory_are_not_allowed() {
        let temp = tempdir().unwrap();

        let visible = temp.path().join("outside.txt");
        File::create(&visible).unwrap();

        let job = Job {
            command: "find".to_string(),
            arguments: vec![".".to_string()],
            environment: HashMap::default(),
            working_directory: PathBuf::from("."),
            inputs: vec![visible],
            outputs: vec![],
        };

        let output = job.run();
        assert_eq!(output.is_err(), true);
    }

    #[test]
    fn outputs_are_copied_out() {
        let temp = tempdir().unwrap();

        let job = Job {
            command: "touch".to_string(),
            arguments: vec!["test.txt".to_string()],
            environment: HashMap::default(),
            working_directory: temp.path().to_path_buf(),
            inputs: vec![],
            outputs: vec![PathBuf::from("test.txt")],
        };

        job.run().unwrap();
        assert_eq!(temp.path().join("test.txt").exists(), true);
    }

    // todo: new-but-untracked files are warnings
    // todo: changed-but-untracked files are warnings
}
