use crate::coordinator;
use crate::job::Job;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use std::fs;
use std::process::Command;

#[cfg(target_family = "unix")]
use std::os::unix::fs::symlink;

#[cfg(target_family = "windows")]
use std::os::windows::fs::symlink_file;

#[derive(Debug, Default)]
pub struct Runner;

impl coordinator::Runner for Runner {
    fn run(&self, job: &Job, workspace: &Workspace) -> Result<()> {
        for file in &job.input_files {
            if let Some(parent_base) = file.parent() {
                let parent = workspace.join(parent_base);

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
            symlink(source, workspace.join(file)).with_context(|| {
                format!("could not symlink `{}` into workspace", file.display())
            })?;

            #[cfg(target_family = "windows")]
            symlink_file(source, workspace.join(file)).with_context(|| {
                format!("could not symlink `{}` into workspace", file.display())
            })?;
        }

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
