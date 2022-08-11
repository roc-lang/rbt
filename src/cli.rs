use crate::rbt::Rbt;
use clap::Parser;
use core::mem::MaybeUninit;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct CLI {
    /// Temporary while we're working out how to get stuff from Roc
    #[clap(long)]
    load_from: Option<PathBuf>,
}

impl CLI {
    #[tracing::instrument]
    pub fn run(&self) -> Result<(), String> {
        tracing::warn!("todo: unimplemented!");

        let rbt: Rbt = match &self.load_from {
            Some(path) => self.load_from_json(path)?,
            None => self.load_from_roc(),
        };

        println!("{:#?}", rbt);

        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    pub fn load_from_roc(&self) -> Rbt {
        let rbt = unsafe {
            let mut input = MaybeUninit::uninit();
            roc_init(input.as_mut_ptr());
            input.assume_init()
        };

        rbt.into()
    }

    #[tracing::instrument(level = "debug")]
    pub fn load_from_json(&self, path: &Path) -> Result<Rbt, String> {
        Err("TODO".to_string())
    }
}

extern "C" {
    #[link_name = "roc__initForHost_1_exposed"]
    fn roc_init(init: *mut crate::bindings::Rbt);
}
