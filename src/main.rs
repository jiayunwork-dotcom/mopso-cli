mod archive;
mod benchmark;
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

    if let Some(cli::Commands::Benchmark {
        problems,
        runs,
        output,
        reference_point,
        config,
        population_size,
        max_iterations,
        archive_size,
        inertia_weight,
        c1,
        c2,
        grid_divisions,
        variant,
        stagnation_limit,
        stagnation_threshold,
        configs,
        seed,
    }) = cli.command
    {
        if let Err(e) = benchmark::run_benchmark(
            &problems,
            runs,
            &output,
            reference_point.as_deref(),
            config.as_deref(),
            population_size,
            max_iterations,
            archive_size,
            inertia_weight,
            c1,
            c2,
            grid_divisions,
            variant.as_deref(),
            stagnation_limit,
            stagnation_threshold,
            configs.as_deref(),
            seed,
        ) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else if cli.tui {
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
