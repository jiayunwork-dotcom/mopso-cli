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
        iteration: usize,
        max_iter: usize,
        archive_size: usize,
        hv: Option<f64>,
    },
    ArchiveSnapshot(Vec<Solution>),
    Convergence(Vec<f64>),
    Finished {
        archive: Vec<Solution>,
        convergence: Vec<f64>,
        final_iter: usize,
        early_stopped: bool,
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
        AppMode::Editing => handle_editing_key(app, key),
        AppMode::ExportDialog => handle_export_dialog_key(app, key),
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
                app.prev_field();
            }
        }
        KeyCode::Down => {
            if app.current_panel == Panel::Parameters {
                app.next_field();
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
    let config = app.to_algorithm_config();

    let problem = problem::load_builtin(&problem_name)?;

    let ref_point = app.reference_point.clone();

    app.is_running = true;
    app.current_generation = 0;
    app.archive_count = 0;
    app.current_hv = ref_point.as_ref().map(|_| 0.0);
    app.archive_members = Vec::new();
    app.convergence = Vec::new();
    app.start_time = Some(Instant::now());
    app.elapsed_time = 0.0;
    app.early_stopped = false;
    
    if ref_point.is_some() {
        app.status_message = format!("Running {}... Press S to stop", problem_name);
    } else {
        app.status_message = format!("Running {} (no ref point, HV disabled)... Press S to stop", problem_name);
    }

    let tx = algo_tx.clone();
    let stop = stop_flag.clone();

    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut last_progress = 0usize;
        let mut last_scatter = 0usize;

        let result = mopso::run_mopso(
            &problem,
            &config,
            ref_point.as_deref(),
            &mut rng,
            &mut |iter, max_iter, archive, hv| {
                if stop.load(Ordering::SeqCst) {
                    return false;
                }

                let archive_size = archive.len();

                if iter % 10 == 0 || iter == max_iter || iter - last_progress >= 5 {
                    last_progress = iter;
                    let _ = tx.send(AlgoMessage::Progress {
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
                    let _ = tx.send(AlgoMessage::ArchiveSnapshot(archive_snapshot));
                }

                true
            },
        );

        let _ = tx.send(AlgoMessage::Finished {
            archive: result.archive_members.clone(),
            convergence: result.convergence.clone(),
            final_iter: result.final_iteration,
            early_stopped: result.early_stopped,
        });

        let _ = tx.send(AlgoMessage::ArchiveSnapshot(result.archive_members));
        let _ = tx.send(AlgoMessage::Convergence(result.convergence));
    });

    Ok(())
}

fn handle_algo_message(app: &mut App, msg: AlgoMessage) {
    match msg {
        AlgoMessage::Progress {
            iteration,
            max_iter: _,
            archive_size,
            hv,
        } => {
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
        AlgoMessage::ArchiveSnapshot(archive) => {
            app.archive_members = archive;
            app.archive_version = app.archive_version.wrapping_add(1);
        }
        AlgoMessage::Convergence(conv) => {
            app.convergence = conv;
            app.convergence_version = app.convergence_version.wrapping_add(1);
        }
        AlgoMessage::Finished {
            archive,
            convergence,
            final_iter,
            early_stopped,
        } => {
            app.is_running = false;
            app.mode = AppMode::Normal;
            app.current_generation = final_iter;
            app.archive_count = archive.len();
            app.archive_members = archive;
            app.archive_version = app.archive_version.wrapping_add(1);
            app.convergence = convergence;
            app.convergence_version = app.convergence_version.wrapping_add(1);
            app.early_stopped = early_stopped;

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

fn export_results(app: &App) -> Result<(), String> {
    let csv_path = &app.export_csv_path;
    let json_path = &app.export_json_path;

    let problem = problem::load_builtin(app.current_problem())?;

    write_csv(&app.archive_members, &problem, csv_path)?;
    write_convergence_json(&app.convergence, json_path)?;

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
    wtr.write_record(&header)
        .map_err(|e| format!("CSV write error: {}", e))?;

    for sol in solutions {
        let mut record = Vec::new();
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

fn write_convergence_json(convergence: &[f64], path: &str) -> Result<(), String> {
    let json = serde_json::to_string_pretty(convergence)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("Cannot write '{}': {}", path, e))?;
    Ok(())
}
