use crate::job;
use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_family = "unix")]
use std::os::unix::fs::symlink;

#[cfg(target_family = "windows")]
use std::os::windows::fs::symlink_file;

#[derive(Debug)]
pub struct Workspace(PathBuf);

impl Workspace {
    pub fn create(root: &Path, key: &job::Key<job::Final>) -> Result<Self> {
        let workspace = Workspace(root.join("workspaces").join(key.to_string()));

        std::fs::create_dir_all(&workspace.0).context("could not create workspace")?;

        Ok(workspace)
    }

    pub fn set_up_files(&self, job: &job::Job) -> Result<()> {
        for file in &job.input_files {
            if let Some(parent_base) = file.parent() {
                let parent = self.join(parent_base);

                if !parent.exists() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("could not create parent for `{}`", file.display())
                    })?;
                }
            }

            // validate that the path exists and is a file
            let meta = file
                .metadata()
                .with_context(|| format!("`{}` does not exist", file.display()))?;

            if meta.is_dir() {
                anyhow::bail!(
                    "`{}` was a directory, but file inputs can only be files",
                    file.display()
                )
            }

            let source = file.absolutize().with_context(|| {
                format!("could not convert `{}` to an absolute path", file.display())
            })?;

            #[cfg(target_family = "unix")]
            symlink(source, self.join(file)).with_context(|| {
                format!("could not symlink `{}` into workspace", file.display())
            })?;

            #[cfg(target_family = "windows")]
            symlink_file(source, workspace.join(file)).with_context(|| {
                format!("could not symlink `{}` into workspace", file.display())
            })?;
        }

        Ok(())
    }

    pub fn join<P: AsRef<Path>>(&self, other: P) -> PathBuf {
        self.0.join(other)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(problem) = std::fs::remove_dir_all(&self.0) {
            log::warn!("problem removing workspace dir: {}", problem);
        };
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glue;
    use roc_std::{RocList, RocStr, RocDict};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn key() -> job::Key<job::Final> {
        job::KeyBuilder::mock().finalize()
    }

    fn job_with_files(files: &[&str]) -> job::Job {
        let glue_job = glue::Job::Job(glue::R1 {
            command: glue::Command {
                tool: glue::Tool::SystemTool(glue::SystemToolPayload {
                    name: RocStr::from("bash"),
                }),
                args: RocList::empty(),
            },
            inputs: RocList::from_slice(&[glue::Input::FromProjectSource(
                files
                    .iter()
                    .map(|name| (*name).into())
                    .collect::<RocList<RocStr>>(),
            )]),
            outputs: RocList::empty(),
            env: RocDict::with_capacity(0),
        });

        job::Job::from_glue(glue_job).unwrap()
    }

    #[test]
    fn sets_up_and_tears_down() {
        let temp = TempDir::new().unwrap();

        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");
        let path = workspace.as_ref().to_path_buf();

        assert!(path.is_dir());

        drop(workspace);

        assert!(!path.exists());
    }

    #[test]
    fn test_sets_up_file() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");

        let job = job_with_files(&[file!()]);
        workspace
            .set_up_files(&job)
            .expect("failed to set up files");

        let path = workspace.join(file!());

        assert!(path.is_symlink());
        assert_eq!(
            PathBuf::from(file!()).absolutize().unwrap(),
            path.read_link().unwrap()
        );
    }

    #[test]
    fn test_rejects_missing_file() {
        let temp = TempDir::new().unwrap();

        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");
        let job = job_with_files(&["does-not-exist"]);

        assert_eq!(
            String::from("`does-not-exist` does not exist"),
            workspace.set_up_files(&job).unwrap_err().to_string(),
        )
    }

    #[test]
    fn test_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::create(temp.path(), &key()).expect("could not create workspace");

        // currently, `file!()` gives us `src/workspace.rs`. This works for us at
        // the moment, but all we really need is a path containing a directory.
        let here = PathBuf::from(file!());
        let parent = here.parent().unwrap();

        let job = job_with_files(&[parent.to_str().unwrap()]);

        assert_eq!(
            format!(
                "`{}` was a directory, but file inputs can only be files",
                parent.display()
            ),
            workspace.set_up_files(&job).unwrap_err().to_string()
        );
    }
}
