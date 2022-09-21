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
