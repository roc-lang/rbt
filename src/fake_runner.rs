use crate::coordinator::Runner;
use crate::job::Job;
use crate::workspace::Workspace;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct FakeRunner {}

impl Runner for FakeRunner {
    fn run(&self, job: &Job, _workspace: &Workspace) -> Result<()> {
        log::info!("fake-running job {:?}", job);

        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }
}
