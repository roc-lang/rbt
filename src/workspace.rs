use crate::job::Job;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Workspace(PathBuf);

impl Workspace {
    pub fn create(root: &Path, job: &Job) -> Result<Self> {
        let workspace = Workspace(root.join("workspaces").join(job.id.to_string()));

        std::fs::create_dir_all(&workspace.0).context("could not create workspace")?;

        Ok(workspace)
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
