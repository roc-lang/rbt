use crate::coordinator;
use crate::job::Job;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Default)]
pub struct Runner;

impl coordinator::Runner for Runner {
    fn run(&self, job: &Job, workspace: &Workspace) -> Result<()> {
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

        Ok(())
    }
}
