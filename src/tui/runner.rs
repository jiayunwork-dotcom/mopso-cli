use crate::tui::app::{App, AppMode, Panel};
use crate::tui::ui;
use crate::problem;
use crate::mopso;
use crate::particle::Solution;
use crate::metrics;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[allow(dead_code)]
pub enum AlgoMessage {
    Progress {
        group_id: usize,
        iteration: usize,
        max_iter: usize,
        archive_size: usize,
        hv: Option<f64>,
    },
    ArchiveSnapshot {
        group_id: usize,
        archive: Vec<Solution>,
    },
    Convergence {
        group_id: usize,
        convergence: Vec<f64>,
    },
    Finished {
        group_id: usize,
        archive: Vec<Solution>,
        convergence: Vec<f64>,
        final_iter: usize,
        early_stopped: bool,
        elapsed: f64,
    },
}

pub fn run_tui() -> Result<(), String> {
    let mut terminal = setup_terminal()?;

    let app = App::new();
    let result = run_app(&mut terminal, app);

    restore_terminal(&mut terminal)?;

    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, String> {
    let mut stdout = std::io::stdout();
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;
    terminal.hide_cursor().map_err(|e| format!("Failed to hide cursor: {}", e))?;
    Ok(terminal)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), String> {
    disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| format!("Failed to leave alternate screen: {}", e))?;
    terminal
        .show_cursor()
        .map_err(|e| format!("Failed to show cursor: {}", e))?;
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), String> {
    let (algo_tx, algo_rx) = mpsc::channel::<AlgoMessage>();
    let stop_flag = Arc::new(AtomicBool::new(false));

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal
            .draw(|f| ui::draw(f, &app))
            .map_err(|e| format!("Draw error: {}", e))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout).map_err(|e| format!("Event poll error: {}", e))? {

            if let Event::Key(key) = event::read().map_err(|e| format!("Event read error: {}", e))? {
                if handle_key_event(&mut app, key, &algo_tx, &stop_flag)? {
                    return Ok(());
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_elapsed();
            last_tick = Instant::now();
        }

        while let Ok(msg) = algo_rx.try_recv() {
            handle_algo_message(&mut app, msg);
        }
    }
}

fn handle_key_event(
    app: &mut App,
    key: KeyEvent,
    algo_tx: &mpsc::Sender<AlgoMessage>,
    stop_flag: &Arc<AtomicBool>,
) -> Result<bool, String> {
    match app.mode {
        AppMode::Editing => {
            if app.editing_from_compare_dialog {
                handle_editing_compare_key(app, key)
            } else {
                handle_editing_key(app, key)
            }
        }
        AppMode::ExportDialog => handle_export_dialog_key(app, key),
        AppMode::CompareGroupDialog => handle_compare_dialog_key(app, key),
        AppMode::Normal | AppMode::Stopping => handle_normal_key(app, key, algo_tx, stop_flag),
    }
}

fn handle_editing_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match key.code {
        KeyCode::Enter => {
            app.finish_editing();
        }
        KeyCode::Esc => {
            app.cancel_editing();
        }
        KeyCode::Backspace => {
            app.backspace();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_editing_compare_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match key.code {
        KeyCode::Enter => {
            app.finish_editing_compare_dialog();
        }
        KeyCode::Esc => {
            app.cancel_compare_dialog();
        }
        KeyCode::Backspace => {
            app.backspace();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_compare_dialog_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match key.code {
        KeyCode::Esc => {
            app.cancel_compare_dialog();
        }
        KeyCode::Enter => {
            app.confirm_compare_dialog();
        }
        KeyCode::Up => {
            app.prev_compare_dialog_field();
        }
        KeyCode::Down => {
            app.next_compare_dialog_field();
        }
        KeyCode::Char(c) => {
            compare_dialog_insert_char(app, c);
        }
        KeyCode::Backspace => {
            compare_dialog_backspace(app);
        }
        _ => {}
    }
    Ok(false)
}

fn compare_dialog_insert_char(app: &mut App, c: char) {
    let field_idx = app.compare_dialog_field;
    match field_idx {
        0 | 1 | 2 => {
            if c.is_ascii_digit() {
                let fields = app.get_compare_fields();
                let mut val = fields[field_idx].value.clone();
                val.push(c);
                if let Ok(v) = val.parse::<usize>() {
                    match field_idx {
                        0 => app.compare_dialog_params.population_size = v,
                        1 => app.compare_dialog_params.max_iterations = v,
                        _ => app.compare_dialog_params.archive_size = v,
                    }
                }
            }
        }
        3 | 4 | 5 => {
            let fields = app.get_compare_fields();
            let mut val = fields[field_idx].value.clone();
            val.push(c);
            if let Ok(v) = val.parse::<f64>() {
                match field_idx {
                    3 => app.compare_dialog_params.inertia_weight = v,
                    4 => app.compare_dialog_params.c1 = v,
                    _ => app.compare_dialog_params.c2 = v,
                }
            }
        }
        6 => {
            let fields = app.get_compare_fields();
            let mut val = fields[field_idx].value.clone();
            val.push(c);
            let v = val.to_lowercase();
            if v == "standard" || v == "adaptive" || v == "s" || v == "a" || v == "st" || v == "ad" {
                app.compare_dialog_params.variant = v;
            } else if val.len() <= 8 {
                app.compare_dialog_params.variant = val;
            }
        }
        _ => {}
    }
}

fn compare_dialog_backspace(app: &mut App) {
    let field_idx = app.compare_dialog_field;
    match field_idx {
        0 => {
            let s = app.compare_dialog_params.population_size.to_string();
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<usize>() {
                    app.compare_dialog_params.population_size = v;
                }
            } else {
                app.compare_dialog_params.population_size = 1;
            }
        }
        1 => {
            let s = app.compare_dialog_params.max_iterations.to_string();
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<usize>() {
                    app.compare_dialog_params.max_iterations = v;
                }
            } else {
                app.compare_dialog_params.max_iterations = 1;
            }
        }
        2 => {
            let s = app.compare_dialog_params.archive_size.to_string();
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<usize>() {
                    app.compare_dialog_params.archive_size = v;
                }
            } else {
                app.compare_dialog_params.archive_size = 1;
            }
        }
        3 => {
            let s = format!("{:.4}", app.compare_dialog_params.inertia_weight);
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<f64>() {
                    app.compare_dialog_params.inertia_weight = v;
                }
            } else {
                app.compare_dialog_params.inertia_weight = 0.0;
            }
        }
        4 => {
            let s = format!("{:.4}", app.compare_dialog_params.c1);
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<f64>() {
                    app.compare_dialog_params.c1 = v;
                }
            } else {
                app.compare_dialog_params.c1 = 0.0;
            }
        }
        5 => {
            let s = format!("{:.4}", app.compare_dialog_params.c2);
            if s.len() > 1 {
                let trimmed = &s[..s.len() - 1];
                if let Ok(v) = trimmed.parse::<f64>() {
                    app.compare_dialog_params.c2 = v;
                }
            } else {
                app.compare_dialog_params.c2 = 0.0;
            }
        }
        6 => {
            let mut s = app.compare_dialog_params.variant.clone();
            if !s.is_empty() {
                s.pop();
                app.compare_dialog_params.variant = s;
            }
        }
        _ => {}
    }
}

fn handle_export_dialog_key(app: &mut App, key: KeyEvent) -> Result<bool, String> {
    match key.code {
        KeyCode::Esc => {
            app.cancel_export_dialog();
        }
        KeyCode::Enter => {
            if let Err(e) = export_results(app) {
                app.status_message = format!("Export failed: {}", e);
            } else {
                app.status_message = format!(
                    "Exported to {} and {}",
                    app.export_csv_path, app.export_json_path
                );
            }
            app.cancel_export_dialog();
        }
        KeyCode::Up => {
            app.prev_export_field();
        }
        KeyCode::Down => {
            app.next_export_field();
        }
        KeyCode::Backspace => {
            app.backspace();
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_normal_key(
    app: &mut App,
    key: KeyEvent,
    algo_tx: &mpsc::Sender<AlgoMessage>,
    stop_flag: &Arc<AtomicBool>,
) -> Result<bool, String> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            if app.is_running {
                stop_flag.store(true, Ordering::SeqCst);
                app.status_message = String::from("Stopping... Please wait.");
                app.mode = AppMode::Stopping;
            } else {
                return Ok(true);
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if !app.is_running {
                start_optimization(app, algo_tx, stop_flag.clone())?;
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if app.is_running {
                stop_flag.store(true, Ordering::SeqCst);
                app.status_message = String::from("Stopping... Please wait.");
                app.mode = AppMode::Stopping;
            }
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if !app.is_running {
                app.start_export_dialog();
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.next_problem();
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if !app.is_running {
                app.start_compare_dialog();
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if !app.is_running && app.current_panel == Panel::Parameters {
                app.delete_selected_compare_group();
            }
        }
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.prev_panel();
            } else {
                app.next_panel();
            }
        }
        KeyCode::BackTab => {
            app.prev_panel();
        }
        KeyCode::Up => {
            if app.current_panel == Panel::Parameters {
                let n_fields = app.get_fields().len();
                if app.selected_field > 0 {
                    app.prev_field();
                } else if !app.compare_groups.is_empty() {
                    app.prev_compare_group_selection();
                }
                let _ = n_fields;
            }
        }
        KeyCode::Down => {
            if app.current_panel == Panel::Parameters {
                let n_fields = app.get_fields().len();
                if app.selected_field < n_fields - 1 {
                    app.next_field();
                } else if !app.compare_groups.is_empty() {
                    app.next_compare_group_selection();
                }
            }
        }
        KeyCode::Enter => {
            if app.current_panel == Panel::Parameters && !app.is_running {
                app.start_editing();
            }
        }
        KeyCode::Esc => {
        }
        _ => {}
    }
    Ok(false)
}

fn start_optimization(
    app: &mut App,
    algo_tx: &mpsc::Sender<AlgoMessage>,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let problem_name = app.current_problem().to_string();
    let ref_point = app.reference_point.clone();

    let is_compare = !app.compare_groups.is_empty();

    if is_compare {
        app.is_compare_run = true;
        app.compare_current_group = 0;
        app.compare_results = vec![None; app.compare_groups.len()];
        app.compare_progress = vec![(0, 0, None); app.compare_groups.len()];
        app.status_message = format!(
            "Compare run: {} groups. Running group 1/{}... Press S to stop",
            app.compare_groups.len(),
            app.compare_groups.len()
        );
    } else {
        app.is_compare_run = false;
    }

    app.is_running = true;
    app.current_generation = 0;
    app.archive_count = 0;
    app.current_hv = ref_point.as_ref().map(|_| 0.0);
    app.archive_members = Vec::new();
    app.convergence = Vec::new();
    app.start_time = Some(Instant::now());
    app.elapsed_time = 0.0;
    app.early_stopped = false;
    app.archive_version = app.archive_version.wrapping_add(1);
    app.convergence_version = app.convergence_version.wrapping_add(1);

    let groups: Vec<(usize, crate::config::AlgorithmConfig)> = if is_compare {
        app.compare_groups
            .iter()
            .enumerate()
            .map(|(i, cp)| (i, app.compare_params_to_config(cp)))
            .collect()
    } else {
        vec![(0, app.to_algorithm_config())]
    };

    let problem = problem::load_builtin(&problem_name)?;
    let tx = algo_tx.clone();
    let stop = stop_flag.clone();
    let rp = ref_point.clone();

    thread::spawn(move || {
        for (group_id, config) in groups {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let group_start = Instant::now();
            let mut rng = rand::thread_rng();
            let mut last_progress = 0usize;
            let mut last_scatter = 0usize;
            let tx_inner = tx.clone();
            let stop_inner = stop.clone();
            let rp_inner = rp.clone();
            let problem_ref = &problem;

            let result = mopso::run_mopso(
                problem_ref,
                &config,
                rp_inner.as_deref(),
                &mut rng,
                &mut |iter, max_iter, archive, hv| {
                    if stop_inner.load(Ordering::SeqCst) {
                        return false;
                    }

                    let archive_size = archive.len();

                    if iter % 10 == 0 || iter == max_iter || iter - last_progress >= 5 {
                        last_progress = iter;
                        let _ = tx_inner.send(AlgoMessage::Progress {
                            group_id,
                            iteration: iter,
                            max_iter,
                            archive_size,
                            hv,
                        });
                    }

                    if iter % 10 == 0 || iter == max_iter || iter - last_scatter >= 10 {
                        last_scatter = iter;
                        let archive_snapshot: Vec<Solution> = archive.iter()
                            .take(200)
                            .cloned()
                            .collect();
                        let _ = tx_inner.send(AlgoMessage::ArchiveSnapshot {
                            group_id,
                            archive: archive_snapshot,
                        });
                    }

                    true
                },
            );

            let elapsed = group_start.elapsed().as_secs_f64();
            let final_hv = rp_inner.as_ref().map(|rp| metrics::hypervolume(&result.archive_members, rp));

            let _ = tx_inner.send(AlgoMessage::Finished {
                group_id,
                archive: result.archive_members.clone(),
                convergence: result.convergence.clone(),
                final_iter: result.final_iteration,
                early_stopped: result.early_stopped,
                elapsed,
            });

            let _ = tx_inner.send(AlgoMessage::ArchiveSnapshot {
                group_id,
                archive: result.archive_members.clone(),
            });
            let _ = tx_inner.send(AlgoMessage::Convergence {
                group_id,
                convergence: result.convergence.clone(),
            });

            let _ = final_hv;
        }
    });

    Ok(())
}

fn handle_algo_message(app: &mut App, msg: AlgoMessage) {
    match msg {
        AlgoMessage::Progress {
            group_id,
            iteration,
            max_iter,
            archive_size,
            hv,
        } => {
            if app.is_compare_run {
                if group_id < app.compare_progress.len() {
                    app.compare_progress[group_id] = (iteration, archive_size, hv);
                }
                if group_id == app.compare_current_group {
                    app.current_generation = iteration;
                    app.archive_count = archive_size;
                    app.current_hv = hv;
                }
            } else {
                app.current_generation = iteration;
                app.archive_count = archive_size;
                app.current_hv = hv;
                if app.convergence.len() < iteration {
                    if let Some(hv_val) = hv {
                        app.convergence.push(hv_val);
                    } else {
                        app.convergence.push(archive_size as f64);
                    }
                    app.convergence_version = app.convergence_version.wrapping_add(1);
                }
            }
            let _ = max_iter;
        }
        AlgoMessage::ArchiveSnapshot { group_id, archive } => {
            if app.is_compare_run {
                if group_id == app.compare_current_group {
                    app.archive_members = archive.clone();
                    app.archive_version = app.archive_version.wrapping_add(1);
                }
            } else {
                app.archive_members = archive;
                app.archive_version = app.archive_version.wrapping_add(1);
            }
        }
        AlgoMessage::Convergence { group_id, convergence } => {
            if !app.is_compare_run {
                app.convergence = convergence;
                app.convergence_version = app.convergence_version.wrapping_add(1);
            }
            let _ = group_id;
        }
        AlgoMessage::Finished {
            group_id,
            archive,
            convergence,
            final_iter,
            early_stopped,
            elapsed,
        } => {
            let final_hv = app.reference_point.as_ref()
                .map(|rp| metrics::hypervolume(&archive, rp));

            if app.is_compare_run {
                if group_id < app.compare_results.len() {
                    app.compare_results[group_id] = Some(crate::tui::app::CompareGroupResult {
                        archive_members: archive.clone(),
                        convergence: convergence.clone(),
                        final_iteration: final_iter,
                        early_stopped,
                        final_hv,
                        elapsed_time: elapsed,
                    });
                }

                if group_id == app.compare_current_group {
                    app.archive_members = archive.clone();
                    app.archive_version = app.archive_version.wrapping_add(1);
                }

                app.compare_current_group += 1;
                let n_groups = app.compare_groups.len();

                if app.compare_current_group >= n_groups {
                    app.is_running = false;
                    app.mode = AppMode::Normal;

                    let mut best_idx = 0usize;
                    let mut best_hv = f64::NEG_INFINITY;
                    for (i, res) in app.compare_results.iter().enumerate() {
                        if let Some(r) = res {
                            if let Some(hv) = r.final_hv {
                                if hv > best_hv {
                                    best_hv = hv;
                                    best_idx = i;
                                }
                            }
                        }
                    }
                    if let Some(Some(best_res)) = app.compare_results.get(best_idx) {
                        app.archive_members = best_res.archive_members.clone();
                        app.convergence = best_res.convergence.clone();
                        app.current_generation = best_res.final_iteration;
                        app.archive_count = best_res.archive_members.len();
                        app.early_stopped = best_res.early_stopped;
                        app.current_hv = best_res.final_hv;
                        app.archive_version = app.archive_version.wrapping_add(1);
                        app.convergence_version = app.convergence_version.wrapping_add(1);
                    }

                    let total_time: f64 = app.compare_results.iter()
                        .filter_map(|r| r.as_ref().map(|x| x.elapsed_time))
                        .sum();
                    app.elapsed_time = total_time;

                    let mut parts: Vec<String> = Vec::new();
                    for (i, res) in app.compare_results.iter().enumerate() {
                        if let Some(r) = res {
                            let hv_str = r.final_hv
                                .map(|h| format!("{:.4}", h))
                                .unwrap_or_else(|| "N/A".to_string());
                            let marker = if i == best_idx && r.final_hv.is_some() { "*" } else { "" };
                            parts.push(format!("G{}: HV={} ({:.2}s){}", i + 1, hv_str, r.elapsed_time, marker));
                        }
                    }
                    app.status_message = format!(
                        "Compare run finished! {} solutions. Summary: {}",
                        app.archive_members.len(),
                        parts.join(" | ")
                    );
                } else {
                    app.status_message = format!(
                        "Compare run: group {}/{} done. Running group {}/{}...",
                        group_id + 1,
                        n_groups,
                        app.compare_current_group + 1,
                        n_groups
                    );
                    app.current_generation = 0;
                    app.archive_count = 0;
                    app.current_hv = app.reference_point.as_ref().map(|_| 0.0);
                }
            } else {
                app.is_running = false;
                app.mode = AppMode::Normal;
                app.current_generation = final_iter;
                app.archive_count = archive.len();
                app.archive_members = archive;
                app.archive_version = app.archive_version.wrapping_add(1);
                app.convergence = convergence;
                app.convergence_version = app.convergence_version.wrapping_add(1);
                app.early_stopped = early_stopped;
                app.elapsed_time = elapsed;

                if let Some(ref rp) = app.reference_point {
                    app.current_hv = Some(metrics::hypervolume(&app.archive_members, rp));
                }

                if early_stopped {
                    app.status_message = format!(
                        "Optimization finished (early stopped at gen {})",
                        final_iter
                    );
                } else {
                    app.status_message = format!(
                        "Optimization finished! {} solutions found.",
                        app.archive_members.len()
                    );
                }
            }
        }
    }
}

fn append_timestamp(path: &str) -> String {
    let now = chrono_local_now();
    if let Some(dot) = path.rfind('.') {
        let (base, ext) = path.split_at(dot);
        format!("{}_{}{}", base, now, ext)
    } else {
        format!("{}_{}", path, now)
    }
}

fn chrono_local_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    let years_since_1970 = secs / 31536000;
    let _ = years_since_1970;
    format!("{}", secs)
}

fn export_results(app: &App) -> Result<(), String> {
    let csv_path = append_timestamp(&app.export_csv_path);
    let json_path = append_timestamp(&app.export_json_path);

    let problem = problem::load_builtin(app.current_problem())?;

    if app.is_compare_run || !app.compare_results.is_empty() {
        write_csv_compare(app, &problem, &csv_path)?;
        write_convergence_json_compare(app, &json_path)?;
    } else {
        write_csv(&app.archive_members, &problem, &csv_path, None)?;
        write_convergence_json(&app.convergence, &json_path)?;
    }

    Ok(())
}

fn write_csv(solutions: &[Solution], problem: &problem::Problem, path: &str, group_id: Option<usize>) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| format!("Cannot create CSV file '{}': {}", path, e))?;

    let mut header = Vec::new();
    if group_id.is_some() {
        header.push("group_id".to_string());
    }
    for v in &problem.var_names {
        header.push(format!("var_{}", v));
    }
    for i in 0..problem.num_objectives() {
        header.push(format!("obj_{}", i));
    }
    header.push("constraint_violation".to_string());
    wtr.write_record(&header)
        .map_err(|e| format!("CSV write error: {}", e))?;

    for sol in solutions {
        let mut record = Vec::new();
        if let Some(gid) = group_id {
            record.push((gid + 1).to_string());
        }
        for &v in &sol.position {
            record.push(format!("{:.8}", v));
        }
        for &o in &sol.objectives {
            record.push(format!("{:.8}", o));
        }
        record.push(format!("{:.8e}", sol.constraint_violation));
        wtr.write_record(&record)
            .map_err(|e| format!("CSV write error: {}", e))?;
    }

    wtr.flush()
        .map_err(|e| format!("CSV flush error: {}", e))?;
    Ok(())
}

fn write_csv_compare(app: &App, problem: &problem::Problem, path: &str) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| format!("Cannot create CSV file '{}': {}", path, e))?;

    let mut header = Vec::new();
    header.push("group_id".to_string());
    for v in &problem.var_names {
        header.push(format!("var_{}", v));
    }
    for i in 0..problem.num_objectives() {
        header.push(format!("obj_{}", i));
    }
    header.push("constraint_violation".to_string());
    wtr.write_record(&header)
        .map_err(|e| format!("CSV write error: {}", e))?;

    for (gid, res) in app.compare_results.iter().enumerate() {
        if let Some(r) = res {
            for sol in &r.archive_members {
                let mut record = Vec::new();
                record.push((gid + 1).to_string());
                for &v in &sol.position {
                    record.push(format!("{:.8}", v));
                }
                for &o in &sol.objectives {
                    record.push(format!("{:.8}", o));
                }
                record.push(format!("{:.8e}", sol.constraint_violation));
                wtr.write_record(&record)
                    .map_err(|e| format!("CSV write error: {}", e))?;
            }
        }
    }

    wtr.flush()
        .map_err(|e| format!("CSV flush error: {}", e))?;
    Ok(())
}

fn write_convergence_json(convergence: &[f64], path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(convergence)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Cannot write '{}': {}", path, e))?;
    Ok(())
}

fn write_convergence_json_compare(app: &App, path: &str) -> Result<(), String> {
    let convergences: Vec<Vec<f64>> = app.compare_results
        .iter()
        .map(|r| r.as_ref().map(|x| x.convergence.clone()).unwrap_or_default())
        .collect();
    let json = serde_json::to_string_pretty(&convergences)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Cannot write '{}': {}", path, e))?;
    Ok(())
}
