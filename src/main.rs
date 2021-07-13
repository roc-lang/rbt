mod cache;
mod cli;
mod content_hash;
mod deps;
mod interns;
mod job;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
