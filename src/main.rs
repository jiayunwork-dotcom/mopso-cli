mod archive;
mod cli;
mod config;
mod expr;
mod metrics;
mod mopso;
mod particle;
mod problem;
mod tui;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if cli.tui {
        if let Err(e) = tui::runner::run_tui() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        if let Err(e) = cli::run(cli) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
