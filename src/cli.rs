use crate::coordinator::Coordinator;
use crate::glue;
use anyhow::{Context, Result};
use clap::Parser;
use core::mem::MaybeUninit;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Use a runner that does not actually run any tasks. Only useful for
    /// rbt developers!
    #[clap(long)]
    use_fake_runner: bool,

    #[clap(long, default_value = ".rbt")]
    isolator_root: PathBuf,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let rbt = Self::load();

        let mut coordinator = Coordinator::default();
        coordinator.add_target(rbt.f0.default);

        let runner: Box<dyn crate::coordinator::Runner> = if self.use_fake_runner {
            Box::new(crate::fake_runner::FakeRunner::default())
        } else {
            Box::new(crate::runner::Runner::new(self.isolator_root.to_owned()))
        };

        while coordinator.has_outstanding_work() {
            coordinator.run_next(&runner).context("failed to run job")?;
        }

        Ok(())
    }

    pub fn load() -> glue::Rbt {
        unsafe {
            let mut input = MaybeUninit::uninit();
            roc_init(input.as_mut_ptr());
            input.assume_init()
        }
    }
}

extern "C" {
    #[link_name = "roc__initForHost_1_exposed"]
    fn roc_init(init: *mut crate::glue::Rbt);
}
