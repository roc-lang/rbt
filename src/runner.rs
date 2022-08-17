use crate::coordinator::{self, RunnableJob};
use crate::rbt;
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
        let workspace = Workspace::create(&self.root, &job)?;

        debug_assert!(job.inputs.is_empty(), "we don't handle inputs yet");
        debug_assert!(
            job.input_files.is_empty(),
            "we don't handle input files yet"
        );

        let mut command = match &job.command.tool {
            // TODO: in the future, we'll also get binaries from other job's output
            rbt::Tool::SystemTool { name } => Command::new(name.to_string()),
        };

        for arg in &job.command.args {
            command.arg(arg.as_str());
        }

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

        let build_dir = self.root.join("builds").join(format!("{}", job.id));
        std::fs::create_dir_all(&build_dir)
            .context("could not create directory to store outputs")?;

        for output in job.outputs {
            let output_str = output.as_str();
            let workspace_src = workspace.join(output_str);

            std::fs::rename(&workspace_src, build_dir.join(output_str)).with_context(|| {
                format!(
                    "could not collect build output `{}`. Did the build produce it?",
                    workspace_src.display()
                )
            })?;
        }

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
