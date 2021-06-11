mod cache;
mod cli;
mod content_hash;
mod deps;
mod interns;

fn main() -> std::io::Result<()> {
    cli::run()
}
