use crate::config::Config;
use crate::metrics;
use crate::mopso;
use crate::problem;
use rand::SeedableRng;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

struct SingleRunResult {
    hv: Option<f64>,
    archive_size: usize,
    early_stopped: bool,
    elapsed_secs: f64,
    convergence: Vec<f64>,
}

struct ProblemBenchmark {
    name: String,
    config_name: Option<String>,
    runs: Vec<SingleRunResult>,
    error: Option<String>,
}

struct ConfigEntry {
    name: String,
    config: Config,
    overridden_fields: HashSet<&'static str>,
}

struct ConfigLoadError {
    path: String,
    error: String,
}

fn config_name_from_path(path: &str) -> String {
    let file_name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    if file_name.to_lowercase().ends_with(".toml") {
        file_name[..file_name.len() - 5].to_string()
    } else {
        file_name
    }
}

fn apply_cli_overrides(
    config: &mut Config,
    population_size: Option<usize>,
    max_iterations: Option<usize>,
    archive_size: Option<usize>,
    inertia_weight: Option<f64>,
    c1: Option<f64>,
    c2: Option<f64>,
    grid_divisions: Option<usize>,
    variant: Option<&str>,
    stagnation_limit: Option<usize>,
    stagnation_threshold: Option<f64>,
) -> HashSet<&'static str> {
    let mut overridden: HashSet<&'static str> = HashSet::new();
    if let Some(v) = population_size {
        config.algorithm.population_size = v;
        overridden.insert("pop");
    }
    if let Some(v) = max_iterations {
        config.algorithm.max_iterations = v;
        overridden.insert("iter");
    }
    if let Some(v) = archive_size {
        config.algorithm.archive_size = v;
        overridden.insert("archive");
    }
    if let Some(v) = inertia_weight {
        config.algorithm.inertia_weight = Some(crate::config::InertiaWeightConfig::Fixed(v));
        overridden.insert("inertia");
    }
    if let Some(v) = c1 {
        config.algorithm.c1 = v;
        overridden.insert("c1");
    }
    if let Some(v) = c2 {
        config.algorithm.c2 = v;
        overridden.insert("c2");
    }
    if let Some(v) = grid_divisions {
        config.algorithm.grid_divisions = v;
        overridden.insert("grid");
    }
    if let Some(v) = variant {
        config.algorithm.variant = v.to_string();
        overridden.insert("variant");
    }
    if let Some(v) = stagnation_limit {
        config.algorithm.stagnation_limit = v;
        overridden.insert("stag_limit");
    }
    if let Some(v) = stagnation_threshold {
        config.algorithm.stagnation_threshold = v;
        overridden.insert("stag_thresh");
    }
    overridden
}

fn fmt_field<S: std::fmt::Display>(value: S, overridden: bool) -> String {
    if overridden {
        format!("{} (overridden)", value)
    } else {
        format!("{}", value)
    }
}

pub fn run_benchmark(
    problems_str: &str,
    num_runs: usize,
    output_path: &str,
    ref_point_str: Option<&str>,
    config_path: Option<&str>,
    population_size: Option<usize>,
    max_iterations: Option<usize>,
    archive_size: Option<usize>,
    inertia_weight: Option<f64>,
    c1: Option<f64>,
    c2: Option<f64>,
    grid_divisions: Option<usize>,
    variant: Option<&str>,
    stagnation_limit: Option<usize>,
    stagnation_threshold: Option<f64>,
    configs_str: Option<&str>,
    seed: Option<u64>,
) -> Result<(), String> {
    let problem_names: Vec<String> = problems_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if problem_names.is_empty() {
        return Err("No problems specified".to_string());
    }

    let ref_point: Option<Vec<f64>> = if let Some(rp_str) = ref_point_str {
        let parsed: Result<Vec<f64>, _> = rp_str.split(',').map(|s| s.trim().parse()).collect();
        Some(parsed.map_err(|e| format!("Invalid reference point: {}", e))?)
    } else {
        None
    };

    let base_seed = seed.unwrap_or_else(|| rand::random::<u64>());

    let (config_entries, config_errors) = if let Some(cfgs) = configs_str {
        let paths: Vec<&str> = cfgs.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut entries: Vec<ConfigEntry> = Vec::new();
        let mut errors: Vec<ConfigLoadError> = Vec::new();

        for path in &paths {
            match Config::from_file(path) {
                Ok(mut cfg) => {
                    let overridden = apply_cli_overrides(
                        &mut cfg,
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
                    );
                    entries.push(ConfigEntry {
                        name: config_name_from_path(path),
                        config: cfg,
                        overridden_fields: overridden,
                    });
                }
                Err(e) => {
                    errors.push(ConfigLoadError {
                        path: path.to_string(),
                        error: e,
                    });
                }
            }
        }

        (entries, errors)
    } else {
        let mut config = if let Some(path) = config_path {
            Config::from_file(path)?
        } else {
            Config::default()
        };

        let overridden = apply_cli_overrides(
            &mut config,
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
        );

        (vec![ConfigEntry { name: String::new(), config, overridden_fields: overridden }], Vec::new())
    };

    if config_entries.is_empty() {
        return Err("No valid configuration files could be loaded".to_string());
    }

    let num_configs = config_entries.len();
    let total_runs: usize = problem_names.len() * num_configs * num_runs;
    let mut completed = 0usize;
    let mut benchmarks: Vec<ProblemBenchmark> = Vec::new();

    for ce in &config_entries {
        let config_label = if ce.name.is_empty() { "" } else { &ce.name };

        for prob_name in &problem_names {
            let load_result = problem::load_builtin(prob_name);
            let prob = match load_result {
                Ok(p) => p,
                Err(e) => {
                    for skipped in 0..num_runs {
                        completed += 1;
                        if config_label.is_empty() {
                            eprintln!("[{}/{}] {} - SKIPPED (run {}/{}): {}",
                                completed, total_runs, prob_name, skipped + 1, num_runs, e);
                        } else {
                            eprintln!("[{}/{}] {} with {} - SKIPPED (run {}/{}): {}",
                                completed, total_runs, prob_name, config_label, skipped + 1, num_runs, e);
                        }
                    }
                    benchmarks.push(ProblemBenchmark {
                        name: prob_name.clone(),
                        config_name: if config_label.is_empty() { None } else { Some(config_label.to_string()) },
                        runs: Vec::new(),
                        error: Some(e),
                    });
                    continue;
                }
            };

            let mut run_results: Vec<SingleRunResult> = Vec::new();

            for run_idx in 0..num_runs {
                completed += 1;
                if config_label.is_empty() {
                    eprint!("[{}/{}] Running {} (run {}/{})...",
                        completed, total_runs, prob_name, run_idx + 1, num_runs);
                } else {
                    eprint!("[{}/{}] Running {} with {} (run {}/{})...",
                        completed, total_runs, prob_name, config_label, run_idx + 1, num_runs);
                }
                let _ = std::io::stderr().flush();

                let run_seed = base_seed.wrapping_add(completed as u64);
                let mut rng = rand::rngs::StdRng::seed_from_u64(run_seed);

                let start = Instant::now();

                let result = mopso::run_mopso(
                    &prob,
                    &ce.config.algorithm,
                    ref_point.as_deref(),
                    &mut rng,
                    &mut |_iter, _max_iter, _archive, _hv| true,
                );

                let elapsed = start.elapsed().as_secs_f64();

                let hv = ref_point.as_ref().map(|rp| metrics::hypervolume(&result.archive_members, rp));

                run_results.push(SingleRunResult {
                    hv,
                    archive_size: result.archive_members.len(),
                    early_stopped: result.early_stopped,
                    elapsed_secs: elapsed,
                    convergence: result.convergence,
                });

                let hv_display = hv.map(|v| format!("{:.6}", v)).unwrap_or_else(|| "N/A".to_string());
                eprintln!(" done (HV={}, {:.2}s)", hv_display, elapsed);
            }

            benchmarks.push(ProblemBenchmark {
                name: prob_name.clone(),
                config_name: if config_label.is_empty() { None } else { Some(config_label.to_string()) },
                runs: run_results,
                error: None,
            });
        }
    }

    let single_config = config_entries.len() == 1 && config_entries[0].name.is_empty();

    let report = generate_report(&benchmarks, num_runs, &ref_point, &config_entries, single_config, &config_errors);

    std::fs::write(output_path, &report)
        .map_err(|e| format!("Cannot write report to '{}': {}", output_path, e))?;

    eprintln!("\nBenchmark report saved to {}", output_path);

    Ok(())
}

fn build_overview_order(benchmarks: &[ProblemBenchmark], single_config: bool) -> Vec<usize> {
    if single_config {
        (0..benchmarks.len()).collect()
    } else {
        let mut problem_order: Vec<String> = Vec::new();
        for bm in benchmarks {
            if !problem_order.contains(&bm.name) {
                problem_order.push(bm.name.clone());
            }
        }
        let mut config_order: Vec<String> = Vec::new();
        for bm in benchmarks {
            if let Some(ref cn) = bm.config_name {
                if !config_order.contains(cn) {
                    config_order.push(cn.clone());
                }
            }
        }
        let mut result: Vec<usize> = Vec::new();
        for prob_name in &problem_order {
            for cfg_name in &config_order {
                for (i, bm) in benchmarks.iter().enumerate() {
                    if bm.name == *prob_name && bm.config_name.as_deref() == Some(cfg_name.as_str()) {
                        result.push(i);
                    }
                }
            }
        }
        result
    }
}

fn format_config_header(ce: &ConfigEntry) -> String {
    let algo = &ce.config.algorithm;
    let o = &ce.overridden_fields;
    format!(
        "variant={}, pop={}, iter={}, archive={}, c1={}, c2={}, grid={}, stag_limit={}, stag_thresh={}",
        fmt_field(&algo.variant, o.contains("variant")),
        fmt_field(algo.population_size, o.contains("pop")),
        fmt_field(algo.max_iterations, o.contains("iter")),
        fmt_field(algo.archive_size, o.contains("archive")),
        fmt_field(format!("{:.2}", algo.c1), o.contains("c1")),
        fmt_field(format!("{:.2}", algo.c2), o.contains("c2")),
        fmt_field(algo.grid_divisions, o.contains("grid")),
        fmt_field(algo.stagnation_limit, o.contains("stag_limit")),
        fmt_field(format!("{:.0e}", algo.stagnation_threshold), o.contains("stag_thresh")),
    )
}

fn generate_report(
    benchmarks: &[ProblemBenchmark],
    num_runs: usize,
    ref_point: &Option<Vec<f64>>,
    config_entries: &[ConfigEntry],
    single_config: bool,
    config_errors: &[ConfigLoadError],
) -> String {
    let mut md = String::new();

    md.push_str("# MOPSO Benchmark Report\n\n");

    if single_config {
        md.push_str(&format!("**Configuration**: {}\n\n", format_config_header(&config_entries[0])));
    } else {
        for ce in config_entries {
            md.push_str(&format!("**Config `{}`**: {}\n\n", ce.name, format_config_header(ce)));
        }
    }

    md.push_str(&format!("**Runs per problem**: {}\n\n", num_runs));

    if let Some(rp) = ref_point {
        md.push_str(&format!("**Reference point**: [{}]\n\n",
            rp.iter().map(|v| format!("{:.4}", v)).collect::<Vec<_>>().join(", ")));
    } else {
        md.push_str("**Reference point**: not provided (HV columns show N/A)\n\n");
    }

    md.push_str("## Summary\n\n");

    if single_config {
        md.push_str("| Problem | Avg HV | HV Std | Best HV | Worst HV | Avg Archive Size | Avg Time (s) | Early Stop |\n");
        md.push_str("|---------|--------|--------|---------|----------|------------------|--------------|------------|\n");
    } else {
        md.push_str("| Problem | Config | Avg HV | HV Std | Best HV | Worst HV | Avg Archive Size | Avg Time (s) | Early Stop |\n");
        md.push_str("|---------|--------|--------|--------|---------|----------|------------------|--------------|------------|\n");
    }

    let overview_order = build_overview_order(benchmarks, single_config);
    for &idx in &overview_order {
        let bm = &benchmarks[idx];
        if bm.error.is_some() {
            if single_config {
                md.push_str(&format!("| {} | ERROR | ERROR | ERROR | ERROR | ERROR | ERROR | ERROR |\n", bm.name));
            } else {
                let cn = bm.config_name.as_deref().unwrap_or("");
                md.push_str(&format!("| {} | {} | ERROR | ERROR | ERROR | ERROR | ERROR | ERROR | ERROR |\n", bm.name, cn));
            }
            continue;
        }

        let hv_values: Vec<f64> = bm.runs.iter().filter_map(|r| r.hv).collect();
        let avg_archive: f64 = bm.runs.iter().map(|r| r.archive_size as f64).sum::<f64>() / bm.runs.len() as f64;
        let avg_time: f64 = bm.runs.iter().map(|r| r.elapsed_secs).sum::<f64>() / bm.runs.len() as f64;
        let early_stop_count = bm.runs.iter().filter(|r| r.early_stopped).count();

        if ref_point.is_some() && !hv_values.is_empty() {
            let (mean, std) = mean_std(&hv_values);
            let best = hv_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let worst = hv_values.iter().cloned().fold(f64::INFINITY, f64::min);
            if single_config {
                md.push_str(&format!("| {} | {:.6} | {:.6} | {:.6} | {:.6} | {:.1} | {:.2} | {}/{} |\n",
                    bm.name, mean, std, best, worst, avg_archive, avg_time, early_stop_count, bm.runs.len()));
            } else {
                let cn = bm.config_name.as_deref().unwrap_or("");
                md.push_str(&format!("| {} | {} | {:.6} | {:.6} | {:.6} | {:.6} | {:.1} | {:.2} | {}/{} |\n",
                    bm.name, cn, mean, std, best, worst, avg_archive, avg_time, early_stop_count, bm.runs.len()));
            }
        } else {
            if single_config {
                md.push_str(&format!("| {} | N/A | N/A | N/A | N/A | {:.1} | {:.2} | {}/{} |\n",
                    bm.name, avg_archive, avg_time, early_stop_count, bm.runs.len()));
            } else {
                let cn = bm.config_name.as_deref().unwrap_or("");
                md.push_str(&format!("| {} | {} | N/A | N/A | N/A | N/A | {:.1} | {:.2} | {}/{} |\n",
                    bm.name, cn, avg_archive, avg_time, early_stop_count, bm.runs.len()));
            }
        }
    }

    md.push('\n');

    md.push_str("## Details\n\n");

    if single_config {
        for bm in benchmarks {
            md.push_str(&format!("### {}\n\n", bm.name));

            if let Some(ref err) = bm.error {
                md.push_str(&format!("**Error**: {}\n\n", err));
                continue;
            }

            md.push_str("| Run | HV | Archive Size | Time (s) | Early Stop |\n");
            md.push_str("|-----|-----|-------------|----------|------------|\n");

            let mut best_run_idx = 0;
            let mut best_hv = f64::NEG_INFINITY;

            for (i, run) in bm.runs.iter().enumerate() {
                let hv_display = run.hv.map(|v| format!("{:.6}", v)).unwrap_or_else(|| "N/A".to_string());
                let early_display = if run.early_stopped { "Yes" } else { "No" };
                md.push_str(&format!("| {} | {} | {} | {:.2} | {} |\n",
                    i + 1, hv_display, run.archive_size, run.elapsed_secs, early_display));

                if let Some(h) = run.hv {
                    if h > best_hv {
                        best_hv = h;
                        best_run_idx = i;
                    }
                }
            }

            md.push('\n');

            if !bm.runs.is_empty() {
                let best_run = &bm.runs[best_run_idx];
                let convergence_str = best_run.convergence.iter()
                    .map(|v| format!("{:.6}", v))
                    .collect::<Vec<_>>()
                    .join(",");
                if ref_point.is_some() {
                    md.push_str(&format!("**Best run convergence (HV, run {})**:\n\n", best_run_idx + 1));
                } else {
                    md.push_str(&format!("**Best run convergence (reference point not provided; data is archive size per generation, not HV; run {})**:\n\n", best_run_idx + 1));
                }
                md.push_str(&format!("```\n{}\n```\n\n", convergence_str));
            }
        }
    } else {
        let mut current_config: Option<&str> = None;
        for bm in benchmarks {
            let cn = bm.config_name.as_deref().unwrap_or("");
            if current_config != Some(cn) {
                if current_config.is_some() {
                    md.push('\n');
                }
                md.push_str(&format!("#### Config: {}\n\n", cn));
                current_config = Some(cn);
            }

            md.push_str(&format!("##### {} ({})\n\n", bm.name, cn));

            if let Some(ref err) = bm.error {
                md.push_str(&format!("**Error**: {}\n\n", err));
                continue;
            }

            md.push_str("| Run | HV | Archive Size | Time (s) | Early Stop |\n");
            md.push_str("|-----|-----|-------------|----------|------------|\n");

            let mut best_run_idx = 0;
            let mut best_hv = f64::NEG_INFINITY;

            for (i, run) in bm.runs.iter().enumerate() {
                let hv_display = run.hv.map(|v| format!("{:.6}", v)).unwrap_or_else(|| "N/A".to_string());
                let early_display = if run.early_stopped { "Yes" } else { "No" };
                md.push_str(&format!("| {} | {} | {} | {:.2} | {} |\n",
                    i + 1, hv_display, run.archive_size, run.elapsed_secs, early_display));

                if let Some(h) = run.hv {
                    if h > best_hv {
                        best_hv = h;
                        best_run_idx = i;
                    }
                }
            }

            md.push('\n');

            if !bm.runs.is_empty() {
                let best_run = &bm.runs[best_run_idx];
                let convergence_str = best_run.convergence.iter()
                    .map(|v| format!("{:.6}", v))
                    .collect::<Vec<_>>()
                    .join(",");
                if ref_point.is_some() {
                    md.push_str(&format!("**Best run convergence (HV, run {})**:\n\n", best_run_idx + 1));
                } else {
                    md.push_str(&format!("**Best run convergence (reference point not provided; data is archive size per generation, not HV; run {})**:\n\n", best_run_idx + 1));
                }
                md.push_str(&format!("```\n{}\n```\n\n", convergence_str));
            }
        }
    }

    let errors: Vec<&ProblemBenchmark> = benchmarks.iter().filter(|b| b.error.is_some()).collect();
    if !errors.is_empty() || !config_errors.is_empty() {
        md.push_str("## Errors\n\n");

        for bm in errors {
            md.push_str(&format!("- **{}**: {}\n", bm.name, bm.error.as_ref().unwrap()));
        }

        for ce in config_errors {
            md.push_str(&format!("- **Config `{}`**: {}\n", ce.path, ce.error));
        }

        md.push('\n');
    }

    md
}

fn mean_std(values: &[f64]) -> (f64, f64) {
    let n = values.len() as f64;
    if n < 1.0 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<f64>() / n;
    let denom = if n > 1.0 { n - 1.0 } else { 1.0 };
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / denom;
    (mean, variance.sqrt())
}
