use crate::coordinator::{self, RunnableJob};
use anyhow::Result;

#[derive(Debug, Default)]
pub struct Runner {}

impl coordinator::Runner for Runner {
    fn run(&self, job: &RunnableJob) -> Result<()> {
        debug_assert!(job.inputs.is_empty(), "we don't handle inputs yet");
        debug_assert!(
            job.input_files.is_empty(),
            "we don't handle input files yet"
        );

        println!("{:#?}", job);

        anyhow::bail!("real runner is unimplemented")
    }
}
