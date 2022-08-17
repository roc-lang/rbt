use crate::coordinator::{RunnableJob, Runner};
use anyhow::Result;

#[derive(Debug, Default)]
pub struct FakeRunner {}

impl Runner for FakeRunner {
    fn run(&self, job: &RunnableJob) -> Result<()> {
        eprintln!("running job: {:#?}", job);

        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }
}
