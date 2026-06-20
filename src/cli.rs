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

    #[arg(long, help = "Number of independent runs", default_value = "1")]
    pub runs: Option<usize>,

    #[arg(long, help = "Stagnation limit for early stopping (generations)")]
    pub stagnation_limit: Option<usize>,

    #[arg(long, help = "Stagnation threshold for early stopping")]
    pub stagnation_threshold: Option<f64>,
}

struct RunResult {
    archive_members: Vec<Solution>,
    convergence: Vec<f64>,
    hv: Option<f64>,
    igd: Option<f64>,
    final_iteration: usize,
    early_stopped: bool,
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
    if let Some(v) = cli.stagnation_limit {
        config.algorithm.stagnation_limit = v;
    }
    if let Some(v) = cli.stagnation_threshold {
        config.algorithm.stagnation_threshold = v;
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
    let num_runs = cli.runs.unwrap_or(1);

    eprintln!("Problem: {} variables, {} objectives",
        problem.num_variables(), problem.num_objectives());
    eprintln!("Algorithm: {} (pop={}, iter={}, archive={})",
        config.algorithm.variant,
        config.algorithm.population_size,
        config.algorithm.max_iterations,
        config.algorithm.archive_size);
    eprintln!("Runs: {}", num_runs);

    let progress_interval = cli.progress_interval.unwrap_or(50);
    let base_seed = cli.seed.unwrap_or_else(|| rand::random::<u64>());

    let true_pareto = if let Some(tp_path) = true_pareto_path {
        Some(metrics::load_true_pareto(tp_path)?)
    } else {
        None
    };

    let mut run_results: Vec<RunResult> = Vec::new();
    let mut best_run_idx = 0;
    let mut best_hv = f64::NEG_INFINITY;
    let mut all_solutions: Vec<Solution> = Vec::new();

    for run_idx in 0..num_runs {
        let run_seed = base_seed.wrapping_add(run_idx as u64);
        let mut rng = rand::rngs::StdRng::seed_from_u64(run_seed);

        let result = mopso::run_mopso(
            &problem,
            &config.algorithm,
            ref_point.as_deref(),
            &mut rng,
            &mut |iter, max_iter, archive_size, hv| {
                if iter % progress_interval == 0 || iter == max_iter {
                    if num_runs > 1 {
                        eprint!("\r  Run {}/{} Gen {}/{} | Archive: {} | HV: {}   ",
                            run_idx + 1, num_runs, iter, max_iter, archive_size,
                            hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));
                    } else {
                        eprint!("\r  Gen {}/{} | Archive: {} | HV: {}   ",
                            iter, max_iter, archive_size,
                            hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));
                    }
                    let _ = std::io::stderr().flush();
                }
            },
        );

        let final_hv = ref_point.as_ref().map(|rp| metrics::hypervolume(&result.archive_members, rp));
        if num_runs > 1 {
            eprint!("\r  Run {}/{} Gen {}/{} | Archive: {} | HV: {}   ",
                run_idx + 1, num_runs, result.final_iteration, config.algorithm.max_iterations,
                result.archive_members.len(),
                final_hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));
        } else {
            eprint!("\r  Gen {}/{} | Archive: {} | HV: {}   ",
                result.final_iteration, config.algorithm.max_iterations,
                result.archive_members.len(),
                final_hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));
        }
        let _ = std::io::stderr().flush();
        eprintln!();

        let hv = final_hv;
        let igd = true_pareto.as_ref().map(|tp| metrics::igd(&result.archive_members, tp));

        if let Some(h) = hv {
            if h > best_hv {
                best_hv = h;
                best_run_idx = run_idx;
            }
        }

        all_solutions.extend(result.archive_members.clone());

        run_results.push(RunResult {
            archive_members: result.archive_members,
            convergence: result.convergence,
            hv,
            igd,
            final_iteration: result.final_iteration,
            early_stopped: result.early_stopped,
        });

        if num_runs > 1 {
            if result.early_stopped {
                eprintln!("  Run {}/{} finished: Early stopped at generation {}",
                    run_idx + 1, num_runs, result.final_iteration);
            } else {
                eprintln!("  Run {}/{} finished: {} solutions",
                    run_idx + 1, num_runs, run_results[run_idx].archive_members.len());
            }
        }
    }

    let output_csv = cli.output_csv.as_deref().unwrap_or(&config.output.pareto_csv);
    let output_json = cli.output_json.as_deref().unwrap_or(&config.output.convergence_json);

    if num_runs == 1 {
        let result = &run_results[0];
        write_csv(&result.archive_members, &problem, output_csv)?;
        write_convergence_json(&result.convergence, output_json)?;

        eprintln!("\n=== Results ===");
        if result.early_stopped {
            eprintln!("Early stopped at generation {}", result.final_iteration);
        }
        eprintln!("Number of Pareto solutions: {}", result.archive_members.len());

        if let Some(hv) = result.hv {
            eprintln!("Hypervolume: {:.6}", hv);
        }

        if let Some(igd_val) = result.igd {
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
    } else {
        let best_result = &run_results[best_run_idx];
        write_csv(&best_result.archive_members, &problem, output_csv)?;
        write_convergence_json(&best_result.convergence, output_json)?;

        let merged_solutions = merge_nondominated(&all_solutions);
        let merged_csv_path = add_suffix_to_filename(output_csv, "_merged");
        write_csv(&merged_solutions, &problem, &merged_csv_path)?;

        eprintln!("\n=== Statistics Summary ===");
        eprintln!("Number of runs: {}", num_runs);
        eprintln!("Best run: Run {} (HV: {})",
            best_run_idx + 1,
            run_results[best_run_idx].hv.map(|v| format!("{:.6}", v)).unwrap_or("N/A".to_string()));

        if ref_point.is_some() {
            let hv_values: Vec<f64> = run_results.iter().filter_map(|r| r.hv).collect();
            if !hv_values.is_empty() {
                let (mean, std) = mean_std(&hv_values);
                let min_val = hv_values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_val = hv_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                eprintln!("\nHypervolume:");
                eprintln!("  Mean:    {:.6}", mean);
                eprintln!("  Std:     {:.6}", std);
                eprintln!("  Best:    {:.6}", max_val);
                eprintln!("  Worst:   {:.6}", min_val);
            }
        }

        if true_pareto.is_some() {
            let igd_values: Vec<f64> = run_results.iter().filter_map(|r| r.igd).collect();
            if !igd_values.is_empty() {
                let (mean, std) = mean_std(&igd_values);
                let min_val = igd_values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max_val = igd_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                eprintln!("\nIGD:");
                eprintln!("  Mean:    {:.6}", mean);
                eprintln!("  Std:     {:.6}", std);
                eprintln!("  Best:    {:.6}", min_val);
                eprintln!("  Worst:   {:.6}", max_val);
            }
        }

        let sol_counts: Vec<usize> = run_results.iter().map(|r| r.archive_members.len()).collect();
        let (sol_mean, sol_std) = mean_std_usize(&sol_counts);
        eprintln!("\nNumber of solutions:");
        eprintln!("  Mean:    {:.2}", sol_mean);
        eprintln!("  Std:     {:.2}", sol_std);

        eprintln!("\nMerged non-dominated solutions: {}", merged_solutions.len());

        if problem.num_objectives() >= 2 {
            print_ascii_scatter(&merged_solutions);
        }

        eprintln!("\nOutputs:");
        eprintln!("  Best run Pareto front: {}", output_csv);
        eprintln!("  Merged Pareto front:   {}", merged_csv_path);
        eprintln!("  Convergence (best run): {}", output_json);
    }

    Ok(())
}

fn mean_std(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n < 1.0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, variance.sqrt())
}

fn mean_std_usize(values: &[usize]) -> (f64, f64) {
    let floats: Vec<f64> = values.iter().map(|&v| v as f64).collect();
    mean_std(&floats)
}

fn merge_nondominated(solutions: &[Solution]) -> Vec<Solution> {
    let mut result: Vec<Solution> = Vec::new();
    for sol in solutions {
        let mut dominated = false;
        let mut to_remove = Vec::new();

        for (i, existing) in result.iter().enumerate() {
            match constraint_dominance(sol, existing) {
                ConstraintDominance::Dominates => {
                    to_remove.push(i);
                }
                ConstraintDominance::Dominated => {
                    dominated = true;
                    break;
                }
                _ => {}
            }
        }

        if !dominated {
            for &i in to_remove.iter().rev() {
                result.remove(i);
            }
            result.push(sol.clone());
        }
    }
    result
}

#[derive(PartialEq)]
enum ConstraintDominance {
    Dominates,
    Dominated,
    Equal,
    Nondominated,
}

fn constraint_dominance(a: &Solution, b: &Solution) -> ConstraintDominance {
    match (a.is_feasible(), b.is_feasible()) {
        (true, true) => match a.dominates(b) {
            crate::particle::Dominance::Dominates => ConstraintDominance::Dominates,
            crate::particle::Dominance::Dominated => ConstraintDominance::Dominated,
            _ => ConstraintDominance::Nondominated,
        },
        (true, false) => ConstraintDominance::Dominates,
        (false, true) => ConstraintDominance::Dominated,
        (false, false) => {
            if a.constraint_violation < b.constraint_violation - 1e-12 {
                ConstraintDominance::Dominates
            } else if b.constraint_violation < a.constraint_violation - 1e-12 {
                ConstraintDominance::Dominated
            } else {
                ConstraintDominance::Equal
            }
        }
    }
}

fn add_suffix_to_filename(path: &str, suffix: &str) -> String {
    if let Some(dot_pos) = path.rfind('.') {
        format!("{}{}{}", &path[..dot_pos], suffix, &path[dot_pos..])
    } else {
        format!("{}{}", path, suffix)
    }
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
