use crate::coordinator::{self, RunnableJob};
use anyhow::{Context, Result};
use std::path::PathBuf;

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
        debug_assert!(job.inputs.is_empty(), "we don't handle inputs yet");
        debug_assert!(
            job.input_files.is_empty(),
            "we don't handle input files yet"
        );

        let build_dir = self.root.join("builds").join(format!("{}", job.id));
        std::fs::create_dir_all(&build_dir)
            .context("could not create build directory to run job")?;

        // convert job.command to an executable thing
        // run it
        // collect the output

        std::fs::remove_dir_all(&build_dir)
            .context("could not clean up the temporary build directory after running the job")?;

        anyhow::bail!("real runner is unimplemented")
    }
}
