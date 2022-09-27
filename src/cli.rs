use crate::coordinator;
use crate::glue;
use crate::store::Store;
use anyhow::{Context, Result};
use clap::Parser;
use core::mem::MaybeUninit;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(long, default_value = ".rbt")]
    root_dir: PathBuf,

    /// Only useful for testing at the moment
    #[clap(long)]
    print_root_output_paths: bool,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let rbt = Self::load();

        let store = Store::new(self.root_dir.join("store")).context("could not open store")?;

        let mut builder = coordinator::Builder::new(self.root_dir.to_path_buf(), store);
        builder.add_target(&rbt.default);

        let mut coordinator = builder
            .build()
            .context("could not initialize coordinator")?;

        let runner = crate::runner::Runner;

        while coordinator.has_outstanding_work() {
            coordinator.run_next(&runner).context("failed to run job")?;
        }

        if self.print_root_output_paths {
            for root in coordinator.roots() {
                println!(
                    "{}",
                    coordinator
                        .store_path(&root)
                        .context("could not get store path for root")?
                )
            }
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
    #[link_name = "roc__initForHost_1_exposed_generic"]
    fn roc_init(init: *mut crate::glue::Rbt);
}
