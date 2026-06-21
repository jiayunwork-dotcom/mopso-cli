use crate::tui::app::{App, Panel, AppMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, Paragraph, Wrap, Clear},
    Frame,
};

pub fn draw(f: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "MOPSO-CLI - Multi-Objective Particle Swarm Optimization",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(main_chunks[1]);

    draw_parameters_panel(f, app, left_chunks[0]);
    draw_status_panel(f, app, right_chunks[0]);
    draw_scatter_panel(f, app, left_chunks[1]);
    draw_convergence_panel(f, app, right_chunks[1]);

    let status = Paragraph::new(vec![Line::from(vec![Span::raw(
        app.status_message.clone(),
    )])])
    .block(Block::default().borders(Borders::ALL).title(" Help "))
    .style(Style::default().fg(Color::Yellow));
    f.render_widget(status, chunks[2]);

    if app.mode == AppMode::ExportDialog {
        draw_export_dialog(f, app);
    }
    if app.mode == AppMode::CompareGroupDialog || (app.mode == AppMode::Editing && app.editing_from_compare_dialog) {
        draw_compare_group_dialog(f, app);
    }
}

fn draw_parameters_panel(f: &mut Frame<'_>, app: &App, area: Rect) {
    let is_focused = app.current_panel == Panel::Parameters;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(" Parameters [1/4] ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let fields = app.get_fields();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            format!("Problem: {}", app.current_problem()),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![Span::raw("")]));

    for (i, field) in fields.iter().enumerate() {
        let is_selected = is_focused && app.selected_field == i && !app.editing_from_compare_dialog;
        let is_editing = is_selected && app.mode == AppMode::Editing;

        let label_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let value_style = if is_editing {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if is_selected {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let display_value = if is_editing {
            let mut s = app.edit_buffer.clone();
            if s.is_empty() {
                s = " ".to_string();
            }
            s
        } else {
            field.value.clone()
        };

        let prefix = if is_selected { "▶ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(prefix, label_style),
            Span::styled(format!("{:<22}", field.label), label_style),
            Span::styled(display_value, value_style),
        ]));
    }

    if !app.compare_groups.is_empty() {
        lines.push(Line::from(vec![Span::raw("")]));
        lines.push(Line::from(vec![
            Span::styled(
                "── Compare Groups ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]));

        for (i, g) in app.compare_groups.iter().enumerate() {
            let is_selected = is_focused && app.selected_compare_group == i;
            let marker = App::compare_group_marker(i);
            let color = App::compare_group_color(i);

            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let summary = format!(
                "Group{}: pop={} iter={} arch={} w={:.2} c1={:.2} c2={:.2} {}",
                i + 1,
                g.population_size,
                g.max_iterations,
                g.archive_size,
                g.inertia_weight,
                g.c1,
                g.c2,
                g.variant
            );

            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("[{}] ", marker), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(summary, style),
            ]));
        }
    }

    lines.push(Line::from(vec![Span::raw("")]));
    lines.push(Line::from(vec![
        Span::styled(
            "↑/↓: Select | Enter: Edit | C: Add Group | D: Del Group | P: Problem",
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_status_panel(f: &mut Frame<'_>, app: &App, area: Rect) {
    let is_focused = app.current_panel == Panel::Status;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(" Status [2/4] ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let mut lines: Vec<Line> = Vec::new();

    let is_compare = !app.compare_groups.is_empty();

    let status_text = if app.is_running {
        if is_compare {
            format!("Compare Running ({}/{})", app.compare_current_group + 1, app.compare_groups.len())
        } else {
            "Running".to_string()
        }
    } else if app.early_stopped {
        "Early Stopped".to_string()
    } else if app.current_generation > 0 {
        "Completed".to_string()
    } else {
        "Ready".to_string()
    };

    let status_color = if app.is_running {
        Color::Green
    } else if app.early_stopped {
        Color::Yellow
    } else if app.current_generation > 0 {
        Color::Cyan
    } else {
        Color::Gray
    };

    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().fg(Color::Gray)),
        Span::styled(status_text, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
    ]));

    lines.push(Line::from(vec![Span::raw("")]));

    lines.push(Line::from(vec![
        Span::styled("Generation: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{} / {}", app.current_generation, app.max_iterations),
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Archive Size: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{} / {}", app.archive_count, app.archive_size),
            Style::default().fg(Color::White),
        ),
    ]));

    let hv_str = match app.current_hv {
        Some(hv) => format!("{:.6}", hv),
        None => "N/A".to_string(),
    };
    lines.push(Line::from(vec![
        Span::styled("HV Value: ", Style::default().fg(Color::Gray)),
        Span::styled(hv_str, Style::default().fg(Color::White)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Elapsed Time: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:.2}s", app.elapsed_time),
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Early Stopped: ", Style::default().fg(Color::Gray)),
        Span::styled(
            if app.early_stopped { "Yes" } else { "No" },
            Style::default().fg(if app.early_stopped { Color::Yellow } else { Color::White }),
        ),
    ]));

    if is_compare {
        lines.push(Line::from(vec![Span::raw("")]));
        lines.push(Line::from(vec![
            Span::styled("── Compare Summary ──", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));

        let has_any_result = app.compare_results.iter().any(|r| r.is_some());

        if app.is_running && !app.compare_progress.is_empty() {
            for (i, prog) in app.compare_progress.iter().enumerate() {
                let (iter, arch, hv) = prog;
                let marker = App::compare_group_marker(i);
                let color = App::compare_group_color(i);
                let group_params = &app.compare_groups[i];
                let is_current = i == app.compare_current_group;
                let hv_str = hv.map(|h| format!("{:.4}", h)).unwrap_or_else(|| "N/A".to_string());

                let prefix = if is_current { "▶" } else { " " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(if is_current { Color::Yellow } else { Color::Gray })),
                    Span::styled(format!(" [{}] ", marker), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("G{}: gen={}/{} arch={}/{} HV={}",
                            i + 1, iter, group_params.max_iterations, arch, group_params.archive_size, hv_str
                        ),
                        Style::default().fg(if is_current { Color::White } else { Color::Gray }),
                    ),
                ]));
            }
        } else if has_any_result {
            let mut best_idx = 0usize;
            let mut best_hv = f64::NEG_INFINITY;
            let mut has_hv = false;
            for (i, res) in app.compare_results.iter().enumerate() {
                if let Some(r) = res {
                    if let Some(hv) = r.final_hv {
                        has_hv = true;
                        if hv > best_hv {
                            best_hv = hv;
                            best_idx = i;
                        }
                    }
                }
            }

            for (i, res) in app.compare_results.iter().enumerate() {
                if let Some(r) = res {
                    let marker = App::compare_group_marker(i);
                    let color = App::compare_group_color(i);
                    let hv_str = r.final_hv
                        .map(|h| format!("{:.4}", h))
                        .unwrap_or_else(|| "N/A".to_string());
                    let is_best = has_hv && r.final_hv.is_some() && i == best_idx;
                    let star = if is_best { " *" } else { "" };

                    let line_style = if is_best {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(format!("[{}]", marker), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                        Span::styled(" ", Style::default()),
                        Span::styled(
                            format!("G{}: HV={} ({:.2}s){}", i + 1, hv_str, r.elapsed_time, star),
                            line_style,
                        ),
                    ]));
                } else {
                    let marker = App::compare_group_marker(i);
                    let color = App::compare_group_color(i);
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(format!("[{}]", marker), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                        Span::styled(" ", Style::default()),
                        Span::styled(format!("G{}: pending...", i + 1), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        } else {
            for (i, _) in app.compare_groups.iter().enumerate() {
                let marker = App::compare_group_marker(i);
                let color = App::compare_group_color(i);
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(format!("[{}]", marker), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(" ", Style::default()),
                    Span::styled(format!("G{}: ready", i + 1), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
    }

    lines.push(Line::from(vec![Span::raw("")]));

    if app.is_running {
        lines.push(Line::from(vec![
            Span::styled(
                "● Running... (Press S to stop)",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        let hint = if is_compare {
            format!("○ Press R to run {} groups | C: Add | D: Del", app.compare_groups.len())
        } else {
            "○ Press R to start | C: Add compare group".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(hint, Style::default().fg(Color::DarkGray)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_scatter_panel(f: &mut Frame<'_>, app: &App, area: Rect) {
    let is_focused = app.current_panel == Panel::Scatter;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(" Pareto Front [3/4] ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let width = inner_area.width as usize;
    let height = inner_area.height as usize;
    
    let scatter_content = {
        let cache = app.scatter_cache.borrow();
        if let Some((ver, w, h, ref lines)) = *cache {
            if ver == app.archive_version && w == width && h == height {
                lines.clone()
            } else {
                drop(cache);
                let new_lines = render_scatter_plot(app, width, height);
                *app.scatter_cache.borrow_mut() = Some((app.archive_version, width, height, new_lines.clone()));
                new_lines
            }
        } else {
            drop(cache);
            let new_lines = render_scatter_plot(app, width, height);
            *app.scatter_cache.borrow_mut() = Some((app.archive_version, width, height, new_lines.clone()));
            new_lines
        }
    };
    
    let paragraph = Paragraph::new(scatter_content)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, inner_area);
}

fn render_scatter_plot(app: &App, width: usize, height: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    if height < 5 || width < 20 {
        lines.push(Line::from(vec![Span::raw("Panel too small")]));
        return lines;
    }

    let feasible: Vec<&crate::particle::Solution> = app.archive_members
        .iter()
        .filter(|s| s.is_feasible())
        .collect();

    if feasible.is_empty() {
        for _ in 0..height.saturating_sub(2) {
            lines.push(Line::from(vec![Span::raw("")]));
        }
        lines.push(Line::from(vec![
            Span::styled(
                "No data yet. Run optimization first.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        return lines;
    }

    if feasible[0].objectives.len() < 2 {
        lines.push(Line::from(vec![
            Span::styled(
                "Scatter plot requires 2+ objectives",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        return lines;
    }

    let plot_height = height.saturating_sub(3);
    let plot_width = width.saturating_sub(12);

    if plot_height < 2 || plot_width < 5 {
        lines.push(Line::from(vec![Span::raw("Panel too small")]));
        return lines;
    }

    let mut f1_min = f64::INFINITY;
    let mut f1_max = f64::NEG_INFINITY;
    let mut f2_min = f64::INFINITY;
    let mut f2_max = f64::NEG_INFINITY;
    for s in &feasible {
        let f1 = s.objectives[0];
        let f2 = s.objectives[1];
        if f1 < f1_min { f1_min = f1; }
        if f1 > f1_max { f1_max = f1; }
        if f2 < f2_min { f2_min = f2; }
        if f2 > f2_max { f2_max = f2; }
    }

    let f1_range = (f1_max - f1_min).max(1e-12);
    let f2_range = (f2_max - f2_min).max(1e-12);

    let f1_scale = (plot_width - 1) as f64 / f1_range;
    let f2_scale = (plot_height - 1) as f64 / f2_range;

    let mut grid = vec![vec![0u32; plot_width]; plot_height];

    for s in &feasible {
        let col = ((s.objectives[0] - f1_min) * f1_scale).round() as usize;
        let row = ((s.objectives[1] - f2_min) * f2_scale).round() as usize;
        let col = col.min(plot_width - 1);
        let row = row.min(plot_height - 1);
        let grid_row = plot_height - 1 - row;
        grid[grid_row][col] += 1;
    }

    let mid_row = plot_height / 2;
    let pw1 = (plot_height - 1) as f64;

    for (i, row) in grid.iter().enumerate() {
        let _f2_val = f2_min + (f2_range * (plot_height - 1 - i) as f64 / pw1);
        let label = if i == 0 {
            format!("{:>8.2} ┤", f2_max)
        } else if i == plot_height - 1 {
            format!("{:>8.2} ┤", f2_min)
        } else if i == mid_row {
            format!("{:>8.2} ┤", (f2_min + f2_max) / 2.0)
        } else {
            format!("{:>8} │", "")
        };

        let mut line_chars = String::with_capacity(plot_width);
        for &count in row {
            let ch = match count {
                0 => ' ',
                1 => '.',
                2 | 3 => '*',
                _ => '#',
            };
            line_chars.push(ch);
        }

        let full_line = format!("{}{}", label, line_chars);
        lines.push(Line::from(vec![Span::raw(full_line)]));
    }

    let x_axis_line = format!("{:>8} └{}", "", "─".repeat(plot_width));
    lines.push(Line::from(vec![Span::raw(x_axis_line)]));

    let f1_label_line = format!("{:>8}  {:<10.2}{}{:>10.2}",
        "",
        f1_min,
        " ".repeat(plot_width.saturating_sub(20)),
        f1_max,
    );
    lines.push(Line::from(vec![
        Span::styled(f1_label_line, Style::default().fg(Color::Gray)),
    ]));

    let f1_title = format!("{:>8}  {} f1 {}",
        "",
        " ".repeat(plot_width.saturating_sub(8) / 2),
        "",
    );
    lines.push(Line::from(vec![
        Span::styled(f1_title, Style::default().fg(Color::Cyan)),
    ]));

    lines
}

fn draw_convergence_panel(f: &mut Frame<'_>, app: &App, area: Rect) {
    let is_focused = app.current_panel == Panel::Convergence;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(" Convergence [4/4] ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let width = inner_area.width as usize;
    let height = inner_area.height as usize;
    
    let convergence_content = {
        let cache = app.convergence_cache.borrow();
        if let Some((ver, w, h, ref lines)) = *cache {
            if ver == app.convergence_version && w == width && h == height {
                lines.clone()
            } else {
                drop(cache);
                let new_lines = render_convergence_plot(app, width, height);
                *app.convergence_cache.borrow_mut() = Some((app.convergence_version, width, height, new_lines.clone()));
                new_lines
            }
        } else {
            drop(cache);
            let new_lines = render_convergence_plot(app, width, height);
            *app.convergence_cache.borrow_mut() = Some((app.convergence_version, width, height, new_lines.clone()));
            new_lines
        }
    };
    
    let paragraph = Paragraph::new(convergence_content)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, inner_area);
}

fn render_convergence_plot(app: &App, width: usize, height: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    if height < 5 || width < 20 {
        lines.push(Line::from(vec![Span::raw("Panel too small")]));
        return lines;
    }

    if app.reference_point.is_none() {
        for _ in 0..height.saturating_sub(2) {
            lines.push(Line::from(vec![Span::raw("")]));
        }
        lines.push(Line::from(vec![
            Span::styled(
                "No reference point",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        return lines;
    }

    let is_compare = !app.compare_results.is_empty();
    let curves: Vec<(usize, &Vec<f64>)> = if is_compare {
        app.compare_results.iter().enumerate()
            .filter_map(|(i, r)| r.as_ref().map(|res| (i, &res.convergence)))
            .collect()
    } else {
        if app.convergence.is_empty() {
            for _ in 0..height.saturating_sub(2) {
                lines.push(Line::from(vec![Span::raw("")]));
            }
            lines.push(Line::from(vec![
                Span::styled(
                    "No convergence data yet.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            return lines;
        }
        vec![(0, &app.convergence)]
    };

    if curves.is_empty() {
        for _ in 0..height.saturating_sub(2) {
            lines.push(Line::from(vec![Span::raw("")]));
        }
        lines.push(Line::from(vec![
            Span::styled(
                "No convergence data yet.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        return lines;
    }

    let plot_height = height.saturating_sub(4 + if is_compare { 1 } else { 0 });
    let plot_width = width.saturating_sub(12);

    if plot_height < 2 || plot_width < 5 {
        lines.push(Line::from(vec![Span::raw("Panel too small")]));
        return lines;
    }

    let mut hv_min = f64::INFINITY;
    let mut hv_max = f64::NEG_INFINITY;
    let mut max_points = 0usize;
    for (_, data) in &curves {
        for &v in *data {
            if v < hv_min { hv_min = v; }
            if v > hv_max { hv_max = v; }
        }
        if data.len() > max_points { max_points = data.len(); }
    }
    let hv_range = (hv_max - hv_min).max(1e-12);

    let mut grid_chars: Vec<Vec<char>> = vec![vec![' '; plot_width]; plot_height];
    let mut grid_colors: Vec<Vec<Option<Color>>> = vec![vec![None; plot_width]; plot_height];

    for (_gidx, (group_idx, data)) in curves.iter().enumerate() {
        let marker = App::compare_group_marker(*group_idx);
        let color = if is_compare { App::compare_group_color(*group_idx) } else { Color::Green };

        for (i, &hv) in data.iter().enumerate() {
            let x = (i as f64 / (max_points - 1).max(1) as f64 * (plot_width - 1) as f64).round() as usize;
            let y = ((hv - hv_min) / hv_range * (plot_height - 1) as f64).round() as usize;
            let x = x.min(plot_width - 1);
            let y = y.min(plot_height - 1);
            let grid_y = plot_height - 1 - y;
            grid_chars[grid_y][x] = marker;
            grid_colors[grid_y][x] = Some(color);
        }
    }

    for (i, row) in grid_chars.iter().enumerate() {
        let label = if i == 0 {
            format!("{:>8.4} ┤", hv_max)
        } else if i == plot_height - 1 {
            format!("{:>8.4} ┤", hv_min)
        } else if i == plot_height / 2 {
            format!("{:>8.4} ┤", (hv_min + hv_max) / 2.0)
        } else {
            format!("{:>8} │", "")
        };

        let mut spans = Vec::new();
        spans.push(Span::raw(label));
        for (j, &ch) in row.iter().enumerate() {
            let _ = j;
            if ch != ' ' {
                if let Some(color) = grid_colors[i][j] {
                    spans.push(Span::styled(ch.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD)));
                } else {
                    spans.push(Span::raw(ch.to_string()));
                }
            } else {
                spans.push(Span::raw(" ".to_string()));
            }
        }
        lines.push(Line::from(spans));
    }

    let x_axis_line = format!("{:>8} └{}", "", "─".repeat(plot_width));
    lines.push(Line::from(vec![Span::raw(x_axis_line)]));

    let gen_label_line = if max_points > 1 {
        format!("{:>8}  {:<10}{}{:>10}",
            "",
            0,
            " ".repeat(plot_width.saturating_sub(20)),
            max_points - 1,
        )
    } else {
        format!("{:>8}  {}", "", "0")
    };
    lines.push(Line::from(vec![
        Span::styled(gen_label_line, Style::default().fg(Color::Gray)),
    ]));

    if is_compare {
        let mut legend_parts: Vec<Span> = Vec::new();
        legend_parts.push(Span::raw(format!("{:>8}  ", "")));
        for (group_idx, _) in &curves {
            let marker = App::compare_group_marker(*group_idx);
            let color = App::compare_group_color(*group_idx);
            legend_parts.push(Span::styled(
                format!("[{}]G{} ", marker, group_idx + 1),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
        lines.push(Line::from(legend_parts));
    }

    let gen_title = format!("{:>8}  {} Generation {}",
        "",
        " ".repeat(plot_width.saturating_sub(14) / 2),
        "",
    );
    lines.push(Line::from(vec![
        Span::styled(gen_title, Style::default().fg(Color::Cyan)),
    ]));

    lines
}

fn draw_export_dialog(f: &mut Frame<'_>, app: &App) {
    let size = f.size();
    let popup_width = 60;
    let popup_height = 10;
    let popup_x = (size.width.saturating_sub(popup_width)) / 2;
    let popup_y = (size.height.saturating_sub(popup_height)) / 2;

    let area = Rect::new(
        popup_x,
        popup_y,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Export Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(inner);

    let csv_style = if app.export_field_idx == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let csv_field = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("CSV Path: ", Style::default().fg(Color::Gray)),
            Span::styled(app.export_csv_path.clone(), csv_style),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(csv_style));

    f.render_widget(csv_field, chunks[0]);

    let json_style = if app.export_field_idx == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let json_field = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("JSON Path: ", Style::default().fg(Color::Gray)),
            Span::styled(app.export_json_path.clone(), json_style),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(json_style));

    f.render_widget(json_field, chunks[1]);

    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "↑/↓: Switch field | Enter: Confirm | Esc: Cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ]);

    f.render_widget(help, chunks[2]);
}

fn draw_compare_group_dialog(f: &mut Frame<'_>, app: &App) {
    let size = f.size();
    let popup_width = 56;
    let popup_height = 15;
    let popup_x = (size.width.saturating_sub(popup_width)) / 2;
    let popup_y = (size.height.saturating_sub(popup_height)) / 2;

    let area = Rect::new(
        popup_x,
        popup_y,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Compare Group ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(0),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(inner);

    let fields = app.get_compare_fields();
    let mut lines: Vec<Line> = Vec::new();
    let inner_width = inner.width.max(10) as usize;

    for (i, field) in fields.iter().enumerate() {
        let is_selected = app.compare_dialog_field == i;

        if is_selected {
            let prefix = "▶ ";
            let label = format!("{:<22}", field.label);
            let value = field.value.clone();
            let content = format!("{}{}{}", prefix, label, value);
            let pad_len = inner_width.saturating_sub(content.chars().count()).max(0);
            let padded = format!("{}{}", content, " ".repeat(pad_len));

            lines.push(Line::from(vec![
                Span::styled(
                    padded,
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            let prefix = "  ";
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Gray)),
                Span::styled(format!("{:<22}", field.label), Style::default().fg(Color::Gray)),
                Span::styled(field.value.clone(), Style::default().fg(Color::White)),
            ]));
        }
    }

    let fields_paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(fields_paragraph, chunks[0]);

    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "↑/↓: Select | Enter: Edit/Confirm | Esc: Cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ]);
    f.render_widget(help, chunks[1]);
}
