use crate::rbt::Rbt;
use clap::Parser;
use core::mem::MaybeUninit;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct CLI {
    /// [temporary] Instead of running the Rbt configuration from Roc, load
    /// it from this JSON.
    #[clap(long)]
    load_from_json: Option<PathBuf>,
}

impl CLI {
    #[tracing::instrument]
    pub fn run(&self) -> Result<(), String> {
        tracing::warn!("todo: unimplemented!");

        let rbt: Rbt = match &self.load_from_json {
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
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);

        serde_json::from_reader(reader).map_err(|e| e.to_string())
    }
}

extern "C" {
    #[link_name = "roc__initForHost_1_exposed"]
    fn roc_init(init: *mut crate::bindings::Rbt);
}
