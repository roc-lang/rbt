mod bindings;
mod cli;
mod rbt;
use clap::Parser;

fn main() {
    let cli = cli::CLI::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE) // TODO: source log level from CLI args
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    if let Err(problem) = cli.run() {
        tracing::error!("{}", problem);
        std::process::exit(1);
    }
}
