use crate::config::AlgorithmConfig;
use crate::particle::Solution;
use ratatui::text::Line;
use std::cell::RefCell;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Parameters,
    Status,
    Scatter,
    Convergence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Editing,
    ExportDialog,
    Stopping,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParameterField {
    pub label: String,
    pub value: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Usize,
    Float,
    String,
}

pub struct App {
    pub current_panel: Panel,
    pub mode: AppMode,
    pub selected_field: usize,
    pub edit_buffer: String,
    pub cursor_pos: usize,

    pub population_size: usize,
    pub max_iterations: usize,
    pub archive_size: usize,
    pub inertia_weight: f64,
    pub c1: f64,
    pub c2: f64,
    pub variant: String,
    pub stagnation_limit: usize,
    pub stagnation_threshold: f64,
    pub reference_point_str: String,

    pub is_running: bool,
    pub current_generation: usize,
    pub archive_count: usize,
    pub current_hv: Option<f64>,
    pub start_time: Option<Instant>,
    pub elapsed_time: f64,
    pub early_stopped: bool,

    pub archive_members: Vec<Solution>,
    pub convergence: Vec<f64>,
    pub reference_point: Option<Vec<f64>>,
    
    pub scatter_cache: RefCell<Option<(u64, usize, usize, Vec<Line<'static>>)>>,
    pub convergence_cache: RefCell<Option<(u64, usize, usize, Vec<Line<'static>>)>>,
    pub archive_version: u64,
    pub convergence_version: u64,

    pub builtin_problems: Vec<&'static str>,
    pub current_problem_idx: usize,

    pub export_csv_path: String,
    pub export_json_path: String,
    pub export_field_idx: usize,

    pub status_message: String,
}

impl App {
    pub fn new() -> Self {
        let default_config = AlgorithmConfig::default();
        let w = match default_config.inertia_weight {
            Some(crate::config::InertiaWeightConfig::Fixed(v)) => v,
            Some(crate::config::InertiaWeightConfig::Linear { from, .. }) => from,
            None => 0.9,
        };

        App {
            current_panel: Panel::Parameters,
            mode: AppMode::Normal,
            selected_field: 0,
            edit_buffer: String::new(),
            cursor_pos: 0,

            population_size: default_config.population_size,
            max_iterations: default_config.max_iterations,
            archive_size: default_config.archive_size,
            inertia_weight: w,
            c1: default_config.c1,
            c2: default_config.c2,
            variant: default_config.variant,
            stagnation_limit: default_config.stagnation_limit,
            stagnation_threshold: default_config.stagnation_threshold,
            reference_point_str: String::new(),

            is_running: false,
            current_generation: 0,
            archive_count: 0,
            current_hv: None,
            start_time: None,
            elapsed_time: 0.0,
            early_stopped: false,

            archive_members: Vec::new(),
            convergence: Vec::new(),
            reference_point: None,
            
            scatter_cache: RefCell::new(None),
            convergence_cache: RefCell::new(None),
            archive_version: 0,
            convergence_version: 0,

            builtin_problems: vec![
                "zdt1", "zdt2", "zdt3", "welded_beam", "pressure_vessel",
            ],
            current_problem_idx: 0,

            export_csv_path: String::from("pareto_front.csv"),
            export_json_path: String::from("convergence.json"),
            export_field_idx: 0,

            status_message: String::from("按 R 开始运行 | Tab 切换面板 | P 切换问题 | E 导出 | Q 退出"),
        }
    }

    pub fn get_fields(&self) -> Vec<ParameterField> {
        vec![
            ParameterField {
                label: String::from("Population Size"),
                value: self.population_size.to_string(),
                field_type: FieldType::Usize,
            },
            ParameterField {
                label: String::from("Max Iterations"),
                value: self.max_iterations.to_string(),
                field_type: FieldType::Usize,
            },
            ParameterField {
                label: String::from("Archive Size"),
                value: self.archive_size.to_string(),
                field_type: FieldType::Usize,
            },
            ParameterField {
                label: String::from("Inertia Weight"),
                value: format!("{:.4}", self.inertia_weight),
                field_type: FieldType::Float,
            },
            ParameterField {
                label: String::from("C1 (Cognitive)"),
                value: format!("{:.4}", self.c1),
                field_type: FieldType::Float,
            },
            ParameterField {
                label: String::from("C2 (Social)"),
                value: format!("{:.4}", self.c2),
                field_type: FieldType::Float,
            },
            ParameterField {
                label: String::from("Variant"),
                value: self.variant.clone(),
                field_type: FieldType::String,
            },
            ParameterField {
                label: String::from("Reference Point"),
                value: if self.reference_point_str.is_empty() {
                    String::from("(empty = no HV)")
                } else {
                    self.reference_point_str.clone()
                },
                field_type: FieldType::String,
            },
            ParameterField {
                label: String::from("Stagnation Limit"),
                value: self.stagnation_limit.to_string(),
                field_type: FieldType::Usize,
            },
            ParameterField {
                label: String::from("Stagnation Threshold"),
                value: format!("{:.2e}", self.stagnation_threshold),
                field_type: FieldType::Float,
            },
        ]
    }

    pub fn next_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Parameters => Panel::Status,
            Panel::Status => Panel::Scatter,
            Panel::Scatter => Panel::Convergence,
            Panel::Convergence => Panel::Parameters,
        };
    }

    pub fn prev_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Parameters => Panel::Convergence,
            Panel::Status => Panel::Parameters,
            Panel::Scatter => Panel::Status,
            Panel::Convergence => Panel::Scatter,
        };
    }

    pub fn next_field(&mut self) {
        let fields = self.get_fields();
        if self.selected_field < fields.len() - 1 {
            self.selected_field += 1;
        }
    }

    pub fn prev_field(&mut self) {
        if self.selected_field > 0 {
            self.selected_field -= 1;
        }
    }

    pub fn start_editing(&mut self) {
        if self.current_panel != Panel::Parameters || self.is_running {
            return;
        }
        let fields = self.get_fields();
        if self.selected_field < fields.len() {
            let value = fields[self.selected_field].value.clone();
            if self.selected_field == 7 && value == "(empty = no HV)" {
                self.edit_buffer = String::new();
            } else {
                self.edit_buffer = value;
            }
            self.cursor_pos = self.edit_buffer.len();
            self.mode = AppMode::Editing;
        }
    }

    pub fn cancel_editing(&mut self) {
        self.mode = AppMode::Normal;
        self.edit_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn finish_editing(&mut self) {
        if self.mode != AppMode::Editing {
            return;
        }

        let buffer = self.edit_buffer.clone();
        match self.selected_field {
            0 => {
                match buffer.parse::<usize>() {
                    Ok(v) if v > 0 => {
                        self.population_size = v;
                        self.status_message = String::from("Parameter updated");
                    }
                    Ok(_) => {
                        self.status_message = String::from("Population size must be > 0");
                    }
                    Err(_) => {
                        self.status_message = format!("Invalid population size: '{}'. Must be a positive integer.", buffer);
                    }
                }
            }
            1 => {
                match buffer.parse::<usize>() {
                    Ok(v) if v > 0 => {
                        self.max_iterations = v;
                        self.status_message = String::from("Parameter updated");
                    }
                    Ok(_) => {
                        self.status_message = String::from("Max iterations must be > 0");
                    }
                    Err(_) => {
                        self.status_message = format!("Invalid max iterations: '{}'. Must be a positive integer.", buffer);
                    }
                }
            }
            2 => {
                match buffer.parse::<usize>() {
                    Ok(v) if v > 0 => {
                        self.archive_size = v;
                        self.status_message = String::from("Parameter updated");
                    }
                    Ok(_) => {
                        self.status_message = String::from("Archive size must be > 0");
                    }
                    Err(_) => {
                        self.status_message = format!("Invalid archive size: '{}'. Must be a positive integer.", buffer);
                    }
                }
            }
            3 => {
                match buffer.parse::<f64>() {
                    Ok(v) => {
                        self.inertia_weight = v;
                        self.status_message = String::from("Parameter updated");
                    }
                    Err(_) => {
                        self.status_message = format!("Invalid inertia weight: '{}'. Must be a number.", buffer);
                    }
                }
            }
            4 => {
                if let Ok(v) = buffer.parse::<f64>() {
                    self.c1 = v;
                    self.status_message = String::from("Parameter updated");
                } else {
                    self.status_message = format!("Invalid value for C1: '{}'. Must be a number.", buffer);
                }
            }
            5 => {
                if let Ok(v) = buffer.parse::<f64>() {
                    self.c2 = v;
                    self.status_message = String::from("Parameter updated");
                } else {
                    self.status_message = format!("Invalid value for C2: '{}'. Must be a number.", buffer);
                }
            }
            6 => {
                let v = buffer.to_lowercase();
                if v == "standard" || v == "adaptive" {
                    self.variant = v;
                    self.status_message = String::from("Parameter updated");
                } else {
                    self.status_message = format!("Invalid variant: '{}'. Must be 'standard' or 'adaptive'.", buffer);
                }
            }
            7 => {
                let trimmed = buffer.trim();
                if trimmed.is_empty() || trimmed == "(empty = no HV)" {
                    self.reference_point_str = String::new();
                    self.reference_point = None;
                    self.status_message = String::from("Reference point cleared (HV disabled)");
                } else {
                    let parsed: Result<Vec<f64>, _> = trimmed
                        .split(',')
                        .map(|s| s.trim().parse::<f64>())
                        .collect();
                    match parsed {
                        Ok(rp) => {
                            if !rp.is_empty() {
                                self.reference_point_str = trimmed.to_string();
                                self.reference_point = Some(rp);
                                self.status_message = format!("Reference point set to [{}]", trimmed);
                            } else {
                                self.status_message = String::from("Reference point cannot be empty");
                            }
                        }
                        Err(_) => {
                            self.status_message = format!(
                                "Invalid reference point: '{}'. Use comma-separated numbers (e.g. '11.0,11.0')",
                                trimmed
                            );
                        }
                    }
                }
            }
            8 => {
                if let Ok(v) = buffer.parse::<usize>() {
                    self.stagnation_limit = v;
                    self.status_message = String::from("Parameter updated");
                } else {
                    self.status_message = format!("Invalid stagnation limit: '{}'. Must be a positive integer.", buffer);
                }
            }
            9 => {
                if let Ok(v) = buffer.parse::<f64>() {
                    self.stagnation_threshold = v;
                    self.status_message = String::from("Parameter updated");
                } else {
                    self.status_message = format!("Invalid stagnation threshold: '{}'. Must be a number.", buffer);
                }
            }
            _ => {}
        }

        self.mode = AppMode::Normal;
        self.edit_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.mode == AppMode::Editing {
            self.edit_buffer.insert(self.cursor_pos, c);
            self.cursor_pos += 1;
        } else if self.mode == AppMode::ExportDialog {
            if self.export_field_idx == 0 {
                self.export_csv_path.insert(self.export_csv_path.len(), c);
            } else {
                self.export_json_path.insert(self.export_json_path.len(), c);
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.mode == AppMode::Editing && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.edit_buffer.remove(self.cursor_pos);
        } else if self.mode == AppMode::ExportDialog {
            if self.export_field_idx == 0 && !self.export_csv_path.is_empty() {
                self.export_csv_path.pop();
            } else if self.export_field_idx == 1 && !self.export_json_path.is_empty() {
                self.export_json_path.pop();
            }
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.mode == AppMode::Editing && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.mode == AppMode::Editing && self.cursor_pos < self.edit_buffer.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn next_problem(&mut self) {
        if self.is_running {
            return;
        }
        self.current_problem_idx = (self.current_problem_idx + 1) % self.builtin_problems.len();
        self.archive_members.clear();
        self.convergence.clear();
        self.current_hv = None;
        self.current_generation = 0;
        self.archive_count = 0;
        self.early_stopped = false;
        self.archive_version = self.archive_version.wrapping_add(1);
        self.convergence_version = self.convergence_version.wrapping_add(1);
        self.status_message = format!(
            "Switched to problem: {}",
            self.builtin_problems[self.current_problem_idx]
        );
    }

    pub fn current_problem(&self) -> &str {
        self.builtin_problems[self.current_problem_idx]
    }

    pub fn start_export_dialog(&mut self) {
        if self.archive_members.is_empty() {
            self.status_message = String::from("No results to export. Run optimization first.");
            return;
        }
        self.mode = AppMode::ExportDialog;
        self.export_field_idx = 0;
    }

    pub fn cancel_export_dialog(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn next_export_field(&mut self) {
        if self.export_field_idx < 1 {
            self.export_field_idx += 1;
        }
    }

    pub fn prev_export_field(&mut self) {
        if self.export_field_idx > 0 {
            self.export_field_idx -= 1;
        }
    }

    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed_time = start.elapsed().as_secs_f64();
        }
    }

    pub fn to_algorithm_config(&self) -> AlgorithmConfig {
        AlgorithmConfig {
            population_size: self.population_size,
            max_iterations: self.max_iterations,
            archive_size: self.archive_size,
            inertia_weight: Some(crate::config::InertiaWeightConfig::Fixed(self.inertia_weight)),
            c1: self.c1,
            c2: self.c2,
            grid_divisions: 20,
            variant: self.variant.clone(),
            stagnation_limit: self.stagnation_limit,
            stagnation_threshold: self.stagnation_threshold,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
