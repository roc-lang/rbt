use crate::coordinator::{RunnableJob, Runner};
use anyhow::Result;

#[derive(Debug, Default)]
pub struct FakeRunner {}

impl Runner for FakeRunner {
    #[tracing::instrument]
    fn run(&self, job: &RunnableJob) -> Result<()> {
        tracing::info!("fake runner \"running\" job");

        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }
}
