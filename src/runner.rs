use crate::coordinator;
use crate::job::{self, Job};
use crate::store;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct Runner {
    workspace_root: PathBuf,
}

impl Runner {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl coordinator::Runner for Runner {
    fn run(
        &self,
        job: &Job,
        job_to_content_hash: &HashMap<job::Key<job::Base>, store::Item>,
    ) -> Result<Workspace> {
        let workspace = Workspace::create(&self.workspace_root, &job.base_key)
            .with_context(|| format!("could not create workspace for {}", job))?;

        workspace
            .set_up_files(job, job_to_content_hash)
            .with_context(|| format!("could not set up workspace files for {}", job))?;

        let mut command: Command = job.into();
        command.current_dir(&workspace);

        // TODO: send stdout, stderr, etc to The Log Zone(tm)
        // TODO: rearrange this so we can stream logs
        let status = command
            .spawn()
            .context("could not run command")?
            .wait()
            .context("command wasn't running")?;

        match status.code() {
            Some(0) => (),
            Some(code) => anyhow::bail!("command failed with the exit code {code}"),
            None => anyhow::bail!("command failed with no exit code (maybe it was killed?)"),
        }

        Ok(workspace)
    }
}
