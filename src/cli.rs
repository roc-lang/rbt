use crate::coordinator;
use crate::glue;
use crate::store::Store;
use anyhow::{Context, Result};
use clap::Parser;
use core::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use tokio::runtime;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(long, default_value = ".rbt")]
    root_dir: PathBuf,

    /// Only useful for testing at the moment
    #[clap(long)]
    print_root_output_paths: bool,

    /// How many worker threads should we spawn? If unset, we'll calculate a
    /// reasonable number based on the host. If set manually, must be greater
    /// than zero.
    #[clap(long, short('j'))]
    max_local_jobs: Option<NonZeroUsize>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let rbt = Self::load();

        let db = self.open_db().context("could not open rbt's database")?;

        let store = Store::new(
            db.open_tree("store")
                .context("could not open the store database")?,
            self.root_dir.join("store"),
        )
        .context("could not open store")?;

        let mut builder = coordinator::Builder::new(
            store,
            db.open_tree("file_hashes")
                .context("could not open file hashes database")?,
            self.root_dir.join("workspaces"),
            self.max_local_jobs()?,
        );
        builder.add_root(&rbt.default);

        let mut coordinator = builder
            .build()
            .context("could not initialize coordinator")?;

        let runtime = self.async_runtime()?;

        runtime
            .block_on(coordinator.run_all())
            .context("failed to run jobs")?;

        if self.print_root_output_paths {
            for root in coordinator.roots() {
                println!(
                    "{}",
                    coordinator
                        .store_path(root)
                        .context("could not get store path for root")?
                        .path()
                        .display()
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

    pub fn async_runtime(&self) -> Result<runtime::Runtime> {
        let mut builder = runtime::Builder::new_multi_thread();
        builder.enable_io();

        builder.build().context("failed to build async runtime")
    }

    pub fn open_db(&self) -> Result<sled::Db> {
        sled::Config::default()
            .path(self.root_dir.join("db"))
            .mode(sled::Mode::HighThroughput)
            .open()
            .context("could not open sled database")
    }

    fn max_local_jobs(&self) -> Result<NonZeroUsize> {
        if let Some(explicit) = self.max_local_jobs {
            return Ok(explicit);
        }

        std::thread::available_parallelism()
            .context("could not determine a reasonable number of local jobs to run simultaneously")
    }
}

extern "C" {
    #[link_name = "roc__initForHost_1_exposed_generic"]
    fn roc_init(init: *mut crate::glue::Rbt);
}
