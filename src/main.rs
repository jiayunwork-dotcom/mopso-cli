mod archive;
mod cli;
mod config;
mod expr;
mod metrics;
mod mopso;
mod particle;
mod problem;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(e) = cli::run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
