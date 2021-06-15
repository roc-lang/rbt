mod cache;
mod cli;
mod content_hash;
mod deps;
mod interns;
mod job;

fn main() -> std::io::Result<()> {
    cli::run()
}
