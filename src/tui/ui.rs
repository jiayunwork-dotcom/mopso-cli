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
        let is_selected = is_focused && app.selected_field == i;
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

    lines.push(Line::from(vec![Span::raw("")]));
    lines.push(Line::from(vec![
        Span::styled(
            "↑/↓: Select | Enter: Edit | Esc: Cancel | P: Switch Problem",
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

    let status_text = if app.is_running {
        "Running"
    } else if app.early_stopped {
        "Early Stopped"
    } else if app.current_generation > 0 {
        "Completed"
    } else {
        "Ready"
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

    lines.push(Line::from(vec![Span::raw("")]));

    if app.is_running {
        lines.push(Line::from(vec![
            Span::styled(
                "● Running... (Press S to stop)",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "○ Press R to start optimization",
                Style::default().fg(Color::DarkGray),
            ),
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

    let f1_min = feasible.iter().map(|s| s.objectives[0]).fold(f64::INFINITY, f64::min);
    let f1_max = feasible.iter().map(|s| s.objectives[0]).fold(f64::NEG_INFINITY, f64::max);
    let f2_min = feasible.iter().map(|s| s.objectives[1]).fold(f64::INFINITY, f64::min);
    let f2_max = feasible.iter().map(|s| s.objectives[1]).fold(f64::NEG_INFINITY, f64::max);

    let f1_range = (f1_max - f1_min).max(1e-12);
    let f2_range = (f2_max - f2_min).max(1e-12);

    let mut grid = vec![vec![0u32; plot_width]; plot_height];

    for s in &feasible {
        let col = ((s.objectives[0] - f1_min) / f1_range * (plot_width - 1) as f64).round() as usize;
        let row = ((s.objectives[1] - f2_min) / f2_range * (plot_height - 1) as f64).round() as usize;
        let col = col.min(plot_width - 1);
        let row = row.min(plot_height - 1);
        let grid_row = plot_height - 1 - row;
        grid[grid_row][col] += 1;
    }

    for (i, row) in grid.iter().enumerate() {
        let _f2_val = f2_min + (f2_range * (plot_height - 1 - i) as f64 / (plot_height - 1).max(1) as f64);
        let label = if i == 0 {
            format!("{:>8.2} ┤", f2_max)
        } else if i == plot_height - 1 {
            format!("{:>8.2} ┤", f2_min)
        } else if i == plot_height / 2 {
            format!("{:>8.2} ┤", (f2_min + f2_max) / 2.0)
        } else {
            format!("{:>8} │", "")
        };

        let mut line_chars = String::new();
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

    let plot_height = height.saturating_sub(3);
    let plot_width = width.saturating_sub(12);

    if plot_height < 2 || plot_width < 5 {
        lines.push(Line::from(vec![Span::raw("Panel too small")]));
        return lines;
    }

    let data = &app.convergence;
    let n_points = data.len();

    let hv_min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let hv_max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let hv_range = (hv_max - hv_min).max(1e-12);

    let mut grid = vec![vec![' '; plot_width]; plot_height];
    let mut latest_point: Option<(usize, usize)> = None;

    for (i, &hv) in data.iter().enumerate() {
        let x = (i as f64 / (n_points - 1).max(1) as f64 * (plot_width - 1) as f64).round() as usize;
        let y = ((hv - hv_min) / hv_range * (plot_height - 1) as f64).round() as usize;
        let x = x.min(plot_width - 1);
        let y = y.min(plot_height - 1);
        let grid_y = plot_height - 1 - y;
        grid[grid_y][x] = '●';
        
        if i == n_points - 1 {
            latest_point = Some((x, grid_y));
        }
    }

    for col in 0..plot_width {
        let mut found = false;
        for row in 0..plot_height {
            if grid[row][col] == '●' {
                found = true;
                for r in row..plot_height {
                    if grid[r][col] == ' ' {
                        grid[r][col] = '│';
                    }
                }
                break;
            }
        }
        if !found && col > 0 {
            let prev_col = col - 1;
            for row in 0..plot_height {
                if grid[row][prev_col] == '●' || grid[row][prev_col] == '│' {
                    if row < plot_height - 1 {
                        grid[row][col] = '─';
                    }
                    break;
                }
            }
        }
    }

    if let Some((x, y)) = latest_point {
        grid[y][x] = '◆';
    }

    let y_label_width = 10;

    for (i, row) in grid.iter().enumerate() {
        let _hv_val = hv_min + (hv_range * (plot_height - 1 - i) as f64 / (plot_height - 1).max(1) as f64);
        let label = if i == 0 {
            format!("{:>8.4} ┤", hv_max)
        } else if i == plot_height - 1 {
            format!("{:>8.4} ┤", hv_min)
        } else if i == plot_height / 2 {
            format!("{:>8.4} ┤", (hv_min + hv_max) / 2.0)
        } else {
            format!("{:>8} │", "")
        };

        let line_str: String = row.iter().collect();
        let full_line = format!("{}{}", label, line_str);

        let mut spans = Vec::new();
        for (j, ch) in full_line.chars().enumerate() {
            if j >= y_label_width && ch == '◆' {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
            } else if j >= y_label_width && ch == '●' {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
            } else if j >= y_label_width && (ch == '│' || ch == '─') {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(Color::Cyan)));
            } else {
                spans.push(Span::raw(ch.to_string()));
            }
        }
        lines.push(Line::from(spans));
    }

    let x_axis_line = format!("{:>8} └{}", "", "─".repeat(plot_width));
    lines.push(Line::from(vec![Span::raw(x_axis_line)]));

    let gen_label_line = if n_points > 1 {
        format!("{:>8}  {:<10}{}{:>10}",
            "",
            0,
            " ".repeat(plot_width.saturating_sub(20)),
            n_points - 1,
        )
    } else {
        format!("{:>8}  {}", "", "0")
    };
    lines.push(Line::from(vec![
        Span::styled(gen_label_line, Style::default().fg(Color::Gray)),
    ]));

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
