use crate::coordinator::{self, RunnableJob};
use anyhow::Result;

#[derive(Debug, Default)]
pub struct Runner {}

impl coordinator::Runner for Runner {
    fn run(&self, _job: &RunnableJob) -> Result<()> {
        anyhow::bail!("real runner is unimplemented")
    }
}
