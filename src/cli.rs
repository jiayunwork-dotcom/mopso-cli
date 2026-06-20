use crate::config::Config;
use crate::metrics;
use crate::mopso;
use crate::particle::Solution;
use crate::problem;
use clap::Parser;
use rand::SeedableRng;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(name = "mopso-cli", about = "Multi-Objective Particle Swarm Optimization CLI tool")]
pub struct Cli {
    #[arg(short, long, help = "Use a built-in problem (zdt1, zdt2, zdt3, welded_beam, pressure_vessel)")]
    pub builtin: Option<String>,

    #[arg(short, long, help = "Path to TOML configuration file")]
    pub config: Option<String>,

    #[arg(long, help = "Population size", default_value = "100")]
    pub population_size: Option<usize>,

    #[arg(long, help = "Maximum iterations", default_value = "500")]
    pub max_iterations: Option<usize>,

    #[arg(long, help = "Archive capacity", default_value = "200")]
    pub archive_size: Option<usize>,

    #[arg(long, help = "Inertia weight (fixed value)")]
    pub inertia_weight: Option<f64>,

    #[arg(long, help = "Cognitive learning factor", default_value = "2.0")]
    pub c1: Option<f64>,

    #[arg(long, help = "Social learning factor", default_value = "2.0")]
    pub c2: Option<f64>,

    #[arg(long, help = "Grid divisions for leader selection", default_value = "20")]
    pub grid_divisions: Option<usize>,

    #[arg(long, help = "Algorithm variant: standard or adaptive", default_value = "standard")]
    pub variant: Option<String>,

    #[arg(long, help = "Reference point for hypervolume (comma-separated)")]
    pub reference_point: Option<String>,

    #[arg(long, help = "True Pareto front file for IGD calculation")]
    pub true_pareto: Option<String>,

    #[arg(long, help = "Output CSV file for Pareto front", default_value = "pareto_front.csv")]
    pub output_csv: Option<String>,

    #[arg(long, help = "Output JSON file for convergence curve", default_value = "convergence.json")]
    pub output_json: Option<String>,

    #[arg(long, help = "Progress report interval (generations)", default_value = "50")]
    pub progress_interval: Option<usize>,

    #[arg(long, help = "Random seed for reproducibility")]
    pub seed: Option<u64>,
}

pub fn run(cli: Cli) -> Result<(), String> {
    let mut config = if let Some(ref path) = cli.config {
        Config::from_file(path)?
    } else {
        Config::default()
    };

    if let Some(ref builtin) = cli.builtin {
        config.builtin = Some(builtin.clone());
    }

    if let Some(v) = cli.population_size {
        config.algorithm.population_size = v;
    }
    if let Some(v) = cli.max_iterations {
        config.algorithm.max_iterations = v;
    }
    if let Some(v) = cli.archive_size {
        config.algorithm.archive_size = v;
    }
    if let Some(v) = cli.inertia_weight {
        config.algorithm.inertia_weight = Some(crate::config::InertiaWeightConfig::Fixed(v));
    }
    if let Some(v) = cli.c1 {
        config.algorithm.c1 = v;
    }
    if let Some(v) = cli.c2 {
        config.algorithm.c2 = v;
    }
    if let Some(v) = cli.grid_divisions {
        config.algorithm.grid_divisions = v;
    }
    if let Some(ref v) = cli.variant {
        config.algorithm.variant = v.clone();
    }

    let ref_point: Option<Vec<f64>> = if let Some(ref rp_str) = cli.reference_point {
        let parsed: Result<Vec<f64>, _> = rp_str.split(',').map(|s| s.trim().parse()).collect();
        Some(parsed.map_err(|e| format!("Invalid reference point: {}", e))?)
    } else {
        config.output.reference_point.clone()
    };

    let true_pareto_path = cli.true_pareto.as_deref()
        .or(config.output.true_pareto_file.as_deref());

    let problem = problem::resolve_problem(&config)?;

    eprintln!("Problem: {} variables, {} objectives",
        problem.num_variables(), problem.num_objectives());
    eprintln!("Algorithm: {} (pop={}, iter={}, archive={})",
        config.algorithm.variant,
        config.algorithm.population_size,
        config.algorithm.max_iterations,
        config.algorithm.archive_size);

    let progress_interval = cli.progress_interval.unwrap_or(50);
    let mut rng = match cli.seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_entropy(),
    };

    let result = mopso::run_mopso(
        &problem,
        &config.algorithm,
        ref_point.as_deref(),
        &mut rng,
        &mut |iter, max_iter, archive_size, hv| {
            if iter % progress_interval == 0 || iter == max_iter {
                eprint!("\r  Gen {}/{} | Archive: {} | HV: {}   ",
                    iter, max_iter, archive_size,
                    hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));
                let _ = std::io::stderr().flush();
            }
        },
    );
    eprintln!();

    let output_csv = cli.output_csv.as_deref().unwrap_or(&config.output.pareto_csv);
    let output_json = cli.output_json.as_deref().unwrap_or(&config.output.convergence_json);

    write_csv(&result.archive_members, &problem, output_csv)?;
    write_convergence_json(&result.convergence, output_json)?;

    eprintln!("\n=== Results ===");
    eprintln!("Number of Pareto solutions: {}", result.archive_members.len());

    if let Some(ref rp) = ref_point {
        let hv = metrics::hypervolume(&result.archive_members, rp);
        eprintln!("Hypervolume: {:.6}", hv);
    }

    if let Some(tp_path) = true_pareto_path {
        let tp = metrics::load_true_pareto(tp_path)?;
        let igd_val = metrics::igd(&result.archive_members, &tp);
        eprintln!("IGD: {:.6}", igd_val);
    }

    let sp = metrics::spacing(&result.archive_members);
    eprintln!("Spacing: {:.6}", sp);

    let feasible_count = result.archive_members.iter().filter(|s| s.is_feasible()).count();
    eprintln!("Feasible solutions: {}/{}", feasible_count, result.archive_members.len());

    if problem.num_objectives() >= 2 {
        print_ascii_scatter(&result.archive_members);
    }

    eprintln!("\nOutputs:");
    eprintln!("  Pareto front: {}", output_csv);
    eprintln!("  Convergence:  {}", output_json);

    Ok(())
}

fn write_csv(solutions: &[Solution], problem: &problem::Problem, path: &str) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| format!("Cannot create CSV file '{}': {}", path, e))?;

    let mut header = Vec::new();
    for v in &problem.var_names {
        header.push(format!("var_{}", v));
    }
    for i in 0..problem.num_objectives() {
        header.push(format!("obj_{}", i));
    }
    header.push("constraint_violation".to_string());
    wtr.write_record(&header).map_err(|e| format!("CSV write error: {}", e))?;

    for sol in solutions {
        let mut record = Vec::new();
        for &v in &sol.position {
            record.push(format!("{:.8}", v));
        }
        for &o in &sol.objectives {
            record.push(format!("{:.8}", o));
        }
        record.push(format!("{:.8e}", sol.constraint_violation));
        wtr.write_record(&record).map_err(|e| format!("CSV write error: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("CSV flush error: {}", e))?;
    Ok(())
}

fn write_convergence_json(convergence: &[f64], path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(convergence)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Cannot write '{}': {}", path, e))?;
    Ok(())
}

fn print_ascii_scatter(solutions: &[Solution]) {
    if solutions.len() < 2 {
        return;
    }

    let feasible: Vec<&Solution> = solutions.iter().filter(|s| s.is_feasible()).collect();
    if feasible.len() < 2 {
        return;
    }

    let width = 60usize;
    let height = 25usize;

    let f1_min = feasible.iter().map(|s| s.objectives[0]).fold(f64::INFINITY, f64::min);
    let f1_max = feasible.iter().map(|s| s.objectives[0]).fold(f64::NEG_INFINITY, f64::max);
    let f2_min = feasible.iter().map(|s| s.objectives[1]).fold(f64::INFINITY, f64::min);
    let f2_max = feasible.iter().map(|s| s.objectives[1]).fold(f64::NEG_INFINITY, f64::max);

    let f1_range = (f1_max - f1_min).max(1e-12);
    let f2_range = (f2_max - f2_min).max(1e-12);

    let mut grid = vec![vec![' '; width]; height];

    for s in &feasible {
        let col = ((s.objectives[0] - f1_min) / f1_range * (width - 1) as f64).round() as usize;
        let row = ((s.objectives[1] - f2_min) / f2_range * (height - 1) as f64).round() as usize;
        let col = col.min(width - 1);
        let row = row.min(height - 1);
        grid[height - 1 - row][col] = '*';
    }

    eprintln!("\n  Pareto Front Scatter (f1 vs f2):");
    eprintln!("  f2_max={:.4}", f2_max);
    for row in &grid {
        eprintln!("  |{}|", row.iter().collect::<String>());
    }
    eprintln!("  f2_min={:.4}", f2_min);
    eprintln!("  f1_min={:.4}{}", " ".repeat(width - 20), format!("f1_max={:.4}", f1_max));
}
