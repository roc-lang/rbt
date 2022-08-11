use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct CLI {
    /// Temporary while we're working out how to get stuff from Roc
    load_json: Option<PathBuf>,
}

impl CLI {
    #[tracing::instrument]
    pub fn run(&self) -> Result<(), String> {
        tracing::warn!("todo: unimplemented!");
        Err("Hello, World!".to_string())
    }
}
