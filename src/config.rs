use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct VariableConfig {
    pub name: String,
    pub lower: f64,
    pub upper: f64,
    #[serde(default)]
    pub integer: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProblemConfig {
    pub variables: Vec<VariableConfig>,
    pub objectives: Vec<String>,
    #[serde(default)]
    pub inequality_constraints: Vec<String>,
    #[serde(default)]
    pub equality_constraints: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AlgorithmConfig {
    #[serde(default = "default_pop_size")]
    pub population_size: usize,
    #[serde(default = "default_max_iter")]
    pub max_iterations: usize,
    #[serde(default = "default_archive_size")]
    pub archive_size: usize,
    #[serde(default)]
    pub inertia_weight: Option<InertiaWeightConfig>,
    #[serde(default = "default_c1")]
    pub c1: f64,
    #[serde(default = "default_c2")]
    pub c2: f64,
    #[serde(default = "default_grid_divisions")]
    pub grid_divisions: usize,
    #[serde(default)]
    pub variant: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum InertiaWeightConfig {
    Fixed(f64),
    Linear { from: f64, to: f64 },
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        AlgorithmConfig {
            population_size: default_pop_size(),
            max_iterations: default_max_iter(),
            archive_size: default_archive_size(),
            inertia_weight: None,
            c1: default_c1(),
            c2: default_c2(),
            grid_divisions: default_grid_divisions(),
            variant: "standard".to_string(),
        }
    }
}

fn default_pop_size() -> usize { 100 }
fn default_max_iter() -> usize { 500 }
fn default_archive_size() -> usize { 200 }
fn default_c1() -> f64 { 2.0 }
fn default_c2() -> f64 { 2.0 }
fn default_grid_divisions() -> usize { 20 }

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_pareto_csv")]
    pub pareto_csv: String,
    #[serde(default = "default_convergence_json")]
    pub convergence_json: String,
    #[serde(default)]
    pub reference_point: Option<Vec<f64>>,
    #[serde(default)]
    pub true_pareto_file: Option<String>,
}

fn default_pareto_csv() -> String { "pareto_front.csv".to_string() }
fn default_convergence_json() -> String { "convergence.json".to_string() }

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            pareto_csv: default_pareto_csv(),
            convergence_json: default_convergence_json(),
            reference_point: None,
            true_pareto_file: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub problem: Option<ProblemConfig>,
    #[serde(default)]
    pub builtin: Option<String>,
    #[serde(default)]
    pub algorithm: AlgorithmConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            problem: None,
            builtin: None,
            algorithm: AlgorithmConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

impl Config {
    pub fn from_toml(input: &str) -> Result<Self, String> {
        toml::from_str(input).map_err(|e| format!("TOML parse error: {}", e))
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read file '{}': {}", path, e))?;
        Self::from_toml(&content)
    }
}
