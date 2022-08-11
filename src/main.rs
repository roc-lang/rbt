mod bindings;
mod cli;
mod rbt;
use clap::Parser;

fn main() {
    let cli = cli::CLI::parse();
    if let Err(problem) = cli.run() {
        eprintln!("{}", problem);
        std::process::exit(1);
    }
}
