use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Output};
use symlink;
use tempfile::Builder;

#[derive(Debug, PartialEq, Eq)]
pub struct Job {
    pub command: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub input_root: PathBuf,
    pub inputs: Vec<PathBuf>,
}

impl Job {
    pub fn run(&self) -> io::Result<Output> {
        let work_dir = Builder::new()
            .prefix(format!("job-{}", self.command).as_str())
            .tempdir()?;

        for input in &self.inputs {
            let meta = fs::metadata(input)?;

            let dest = if let Ok(relative) = input.strip_prefix(&self.input_root) {
                work_dir.path().join(relative)
            } else if input.is_relative() {
                work_dir.path().join(input)
            } else {
                // we can't isolate this file. We'll need a special error case
                // for this.
                todo!();
            };

            // the distinction between file and directory matters on windows
            // (which is why we're using a third-party crate for this; it wraps
            // up the cfg stuff for us.)
            if meta.is_dir() {
                symlink::symlink_dir(input, dest)?;
            } else {
                symlink::symlink_file(input, dest)?;
            }

            println!("{}", input.display());
        }

        let output = Command::new(self.command.as_str())
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
            .output();

        work_dir.close()?;
        output
    }
}

#[cfg(test)]
mod test_job {
    use super::Job;
    use std::collections::HashMap;
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
            input_root: PathBuf::from("."),
            inputs: vec![],
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
            input_root: PathBuf::from("."),
            inputs: vec![],
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
            input_root: PathBuf::from("."),
            inputs: vec![],
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
            input_root: PathBuf::from("."),
            inputs: vec![],
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
            input_root: PathBuf::from("."),
            inputs: vec![],
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
            input_root: temp.path().to_path_buf(),
            inputs: vec![visible],
        };

        let output = job.run().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            ".\n./visible.txt\n".to_string()
        );

        drop(temp);
    }
}
