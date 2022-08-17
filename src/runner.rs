use crate::coordinator::{self, RunnableJob};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default)]
pub struct Runner {
    root: PathBuf,
}

impl Runner {
    pub fn new(root: PathBuf) -> Self {
        Runner { root }
    }
}

impl coordinator::Runner for Runner {
    fn run(&self, job: &RunnableJob) -> Result<()> {
        let workspace = Workspace::create(&self.root, job)?;

        debug_assert!(job.inputs.is_empty(), "we don't handle inputs yet");
        debug_assert!(
            job.input_files.is_empty(),
            "we don't handle input files yet"
        );

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

        let store = Store::create(&self.root, job)?;
        store.take_outputs_from_workspace(job, &workspace)?;

        Ok(())
    }
}

struct Workspace(PathBuf);

impl Workspace {
    fn create(root: &Path, job: &RunnableJob) -> Result<Self> {
        let workspace = Workspace(root.join("workspaces").join(job.id.to_string()));

        std::fs::create_dir_all(&workspace.0).context("could not create workspace")?;

        Ok(workspace)
    }

    fn join<P: AsRef<Path>>(&self, other: P) -> PathBuf {
        self.0.join(other)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        if let Err(problem) = std::fs::remove_dir_all(&self.0) {
            // TODO: this should eventually be a system log line that warns of the error
            eprintln!("[WARNING] problem removing workspace dir: {:}", problem);
        };
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

struct Store(PathBuf);

impl Store {
    fn create(root: &Path, job: &RunnableJob) -> Result<Self> {
        let store = Store(root.join("builds").join(job.id.to_string()));

        std::fs::create_dir_all(&store.0).context("could not create directory to store outputs")?;

        Ok(store)
    }

    fn join<P: AsRef<Path>>(&self, other: P) -> PathBuf {
        self.0.join(other)
    }

    fn take_outputs_from_workspace(&self, job: &RunnableJob, workspace: &Workspace) -> Result<()> {
        for output in job.outputs {
            let output_str = output.as_str();
            let workspace_src = workspace.join(output_str);

            let mut perms = std::fs::metadata(&workspace_src)
                .with_context(|| {
                    format!(
                        "could not find build output `{}`. Did the build produce it?",
                        output_str
                    )
                })?
                .permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(&workspace_src, perms)
                .with_context(|| format!("could not set permissions on `{}`", output_str))?;

            std::fs::rename(&workspace_src, self.join(output_str)).with_context(|| {
                format!(
                    "could not move build output `{}` to the output directory",
                    workspace_src.display()
                )
            })?;
        }

        Ok(())
    }
}
