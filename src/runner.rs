use crate::job::{self, Job};
use crate::store;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug)]
pub struct RunnerBuilder {
    workspace_root: PathBuf,
}

impl RunnerBuilder {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl RunnerBuilder {
    pub async fn build(
        &self,
        job: &Job,
        job_to_content_hash: &HashMap<job::Key<job::Base>, store::Item>,
    ) -> Result<Runner> {
        let workspace = Workspace::create(&self.workspace_root, &job.base_key)
            .await
            .with_context(|| format!("could not create workspace for {}", job))?;

        workspace
            .set_up_files(job, job_to_content_hash)
            .await
            .with_context(|| format!("could not set up workspace files for {}", job))?;

        let mut command = Command::from(&job.command);
        command.current_dir(&workspace);
        command.env("HOME", workspace.home_dir());

        Ok(Runner { command, workspace })
    }
}

pub struct Runner {
    command: Command,
    workspace: Workspace,
}

impl Runner {
    pub async fn run(mut self) -> Result<Workspace> {
        // TODO: send stdout, stderr, etc to The Log Zone(tm)
        // TODO: rearrange this so we can stream logs
        let status = self
            .command
            .spawn()
            .context("could not run command")?
            .wait()
            .await
            .context("command wasn't running")?;

        match status.code() {
            Some(0) => (),
            Some(code) => anyhow::bail!("command failed with the exit code {code}"),
            None => anyhow::bail!("command failed with no exit code (maybe it was killed?)"),
        }

        Ok(self.workspace)
    }
}
