use crate::config::{VariableConfig, ProblemConfig, Config};
use crate::expr::CompiledExpr;

type EvalFn = Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>;
type ConstraintFn = Box<dyn Fn(&[f64]) -> Vec<f64> + Send + Sync>;

pub struct Problem {
    pub variables: Vec<VariableConfig>,
    pub var_names: Vec<String>,
    objective_eval: ObjectiveEval,
    ineq_constraint_eval: ConstraintEval,
    eq_constraint_eval: ConstraintEval,
}

enum ObjectiveEval {
    Expr(Vec<CompiledExpr>),
    Native { f: EvalFn, n_obj: usize },
}

enum ConstraintEval {
    Expr(Vec<CompiledExpr>),
    Native(ConstraintFn),
    None,
}

impl Problem {
    pub fn from_config(cfg: &ProblemConfig) -> Result<Self, String> {
        let var_names: Vec<String> = cfg.variables.iter().map(|v| v.name.clone()).collect();
        let mut objective_exprs = Vec::new();
        for obj_str in &cfg.objectives {
            let expr = CompiledExpr::new(obj_str, &var_names)?;
            objective_exprs.push(expr);
        }
        let mut ineq_exprs = Vec::new();
        for c_str in &cfg.inequality_constraints {
            let expr = CompiledExpr::new(c_str, &var_names)?;
            ineq_exprs.push(expr);
        }
        let mut eq_exprs = Vec::new();
        for c_str in &cfg.equality_constraints {
            let expr = CompiledExpr::new(c_str, &var_names)?;
            eq_exprs.push(expr);
        }
        Ok(Problem {
            variables: cfg.variables.clone(),
            var_names,
            objective_eval: ObjectiveEval::Expr(objective_exprs),
            ineq_constraint_eval: if ineq_exprs.is_empty() { ConstraintEval::None } else { ConstraintEval::Expr(ineq_exprs) },
            eq_constraint_eval: if eq_exprs.is_empty() { ConstraintEval::None } else { ConstraintEval::Expr(eq_exprs) },
        })
    }

    fn new_native(
        variables: Vec<VariableConfig>,
        obj_fn: EvalFn,
        ineq_fn: Option<ConstraintFn>,
        eq_fn: Option<ConstraintFn>,
        n_obj: usize,
    ) -> Self {
        let var_names: Vec<String> = variables.iter().map(|v| v.name.clone()).collect();
        Problem {
            variables,
            var_names,
            objective_eval: ObjectiveEval::Native { f: obj_fn, n_obj },
            ineq_constraint_eval: ineq_fn.map_or(ConstraintEval::None, ConstraintEval::Native),
            eq_constraint_eval: eq_fn.map_or(ConstraintEval::None, ConstraintEval::Native),
        }
    }

    pub fn num_objectives(&self) -> usize {
        match &self.objective_eval {
            ObjectiveEval::Expr(exprs) => exprs.len(),
            ObjectiveEval::Native { n_obj, .. } => *n_obj,
        }
    }

    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    pub fn evaluate(&self, x: &[f64]) -> (Vec<f64>, f64) {
        let objectives = match &self.objective_eval {
            ObjectiveEval::Expr(exprs) => {
                exprs.iter().map(|e| e.eval(x).unwrap_or(f64::NAN)).collect()
            }
            ObjectiveEval::Native { f, .. } => f(x),
        };

        let ineq_vals = match &self.ineq_constraint_eval {
            ConstraintEval::Expr(exprs) => exprs.iter().map(|e| e.eval(x).unwrap_or(0.0)).collect(),
            ConstraintEval::Native(f) => f(x),
            ConstraintEval::None => Vec::new(),
        };

        let eq_vals = match &self.eq_constraint_eval {
            ConstraintEval::Expr(exprs) => exprs.iter().map(|e| e.eval(x).unwrap_or(0.0)).collect(),
            ConstraintEval::Native(f) => f(x),
            ConstraintEval::None => Vec::new(),
        };

        let mut violation = 0.0;
        for v in &ineq_vals {
            if *v > 0.0 { violation += v; }
        }
        for v in &eq_vals {
            violation += v.abs();
        }
        (objectives, violation)
    }

    pub fn lower_bounds(&self) -> Vec<f64> {
        self.variables.iter().map(|v| v.lower).collect()
    }

    pub fn upper_bounds(&self) -> Vec<f64> {
        self.variables.iter().map(|v| v.upper).collect()
    }

    pub fn is_integer(&self) -> Vec<bool> {
        self.variables.iter().map(|v| v.integer).collect()
    }
}

pub fn builtin_zdt1() -> Problem {
    let vars: Vec<VariableConfig> = (0..30).map(|i| VariableConfig {
        name: format!("x{}", i),
        lower: 0.0,
        upper: 1.0,
        integer: false,
    }).collect();
    Problem::new_native(
        vars,
        Box::new(|x: &[f64]| {
            let f1 = x[0];
            let n = x.len() as f64;
            let g = 1.0 + 9.0 / (n - 1.0) * x[1..].iter().sum::<f64>();
            let f2 = g * (1.0 - (f1 / g).sqrt());
            vec![f1, f2]
        }),
        None,
        None,
        2,
    )
}

pub fn builtin_zdt2() -> Problem {
    let vars: Vec<VariableConfig> = (0..30).map(|i| VariableConfig {
        name: format!("x{}", i),
        lower: 0.0,
        upper: 1.0,
        integer: false,
    }).collect();
    Problem::new_native(
        vars,
        Box::new(|x: &[f64]| {
            let f1 = x[0];
            let n = x.len() as f64;
            let g = 1.0 + 9.0 / (n - 1.0) * x[1..].iter().sum::<f64>();
            let f2 = g * (1.0 - (f1 / g).powi(2));
            vec![f1, f2]
        }),
        None,
        None,
        2,
    )
}

pub fn builtin_zdt3() -> Problem {
    let vars: Vec<VariableConfig> = (0..30).map(|i| VariableConfig {
        name: format!("x{}", i),
        lower: 0.0,
        upper: 1.0,
        integer: false,
    }).collect();
    Problem::new_native(
        vars,
        Box::new(|x: &[f64]| {
            let f1 = x[0];
            let n = x.len() as f64;
            let g = 1.0 + 9.0 / (n - 1.0) * x[1..].iter().sum::<f64>();
            let f2 = g * (1.0 - (f1 / g).sqrt() - (f1 / g) * (10.0 * std::f64::consts::PI * f1).sin());
            vec![f1, f2]
        }),
        None,
        None,
        2,
    )
}

pub fn builtin_welded_beam() -> Problem {
    let vars = vec![
        VariableConfig { name: "h".to_string(), lower: 0.125, upper: 5.0, integer: false },
        VariableConfig { name: "l".to_string(), lower: 0.1, upper: 10.0, integer: false },
        VariableConfig { name: "t".to_string(), lower: 0.1, upper: 10.0, integer: false },
        VariableConfig { name: "b".to_string(), lower: 0.125, upper: 5.0, integer: false },
    ];
    Problem::new_native(
        vars,
        Box::new(|x: &[f64]| {
            let h = x[0]; let l = x[1]; let t = x[2]; let b = x[3];
            let f1 = 1.10471 * h * h * l + 0.04811 * t * b * (14.0 + l);
            let f2 = 2.1952 / (t.powi(3) * b);
            vec![f1, f2]
        }),
        Some(Box::new(|x: &[f64]| {
            let h = x[0]; let l = x[1]; let t = x[2]; let b = x[3];
            let p = 6000.0;
            let l_weld = l;
            let tau_prime = p / (std::f64::consts::SQRT_2 * h * l_weld);
            let tau_double_prime = (p * (l_weld + 0.5 * t) * std::f64::consts::SQRT_2) / (2.0 * 0.707 * h * l_weld * (h * h / 12.0 + 0.25 * (h + t).powi(2)));
            let tau = (tau_prime.powi(2) + tau_double_prime.powi(2) + l_weld * tau_prime * tau_double_prime / (std::f64::consts::SQRT_2 * (h * h / 12.0 + 0.25 * (h + t).powi(2)))).sqrt();
            let sigma = 504000.0 / (t * t * b);
            let _delta = 2.1952 / (t.powi(3) * b);
            let pc = 0.25 * 0.707 * h * l_weld * std::f64::consts::SQRT_2 * (h * h / 4.0 + (h + t).powi(2) / 4.0);
            vec![
                13600.0 - tau,
                30000.0 - sigma,
                h - b,
                pc - 6000.0,
            ]
        })),
        None,
        2,
    )
}

pub fn builtin_pressure_vessel() -> Problem {
    let vars = vec![
        VariableConfig { name: "Ts".to_string(), lower: 1.0, upper: 99.0, integer: true },
        VariableConfig { name: "Th".to_string(), lower: 0.0625, upper: 2.0, integer: false },
        VariableConfig { name: "R".to_string(), lower: 10.0, upper: 200.0, integer: false },
        VariableConfig { name: "L".to_string(), lower: 10.0, upper: 200.0, integer: false },
    ];
    Problem::new_native(
        vars,
        Box::new(|x: &[f64]| {
            let ts = x[0]; let th = x[1]; let r = x[2]; let l = x[3];
            let f1 = 0.6224 * ts * r * l + 1.7781 * th * r * r + 3.1661 * ts * ts * l + 19.84 * ts * ts * r;
            vec![f1]
        }),
        Some(Box::new(|x: &[f64]| {
            let ts = x[0]; let th = x[1]; let r = x[2]; let l = x[3];
            let pi = std::f64::consts::PI;
            vec![
                -ts + 0.0193 * r,
                -th + 0.00954 * r,
                -pi * r * r * l - (4.0 / 3.0) * pi * r.powi(3) + 1296000.0,
                l - 240.0,
            ]
        })),
        None,
        1,
    )
}

pub fn load_builtin(name: &str) -> Result<Problem, String> {
    match name.to_lowercase().as_str() {
        "zdt1" => Ok(builtin_zdt1()),
        "zdt2" => Ok(builtin_zdt2()),
        "zdt3" => Ok(builtin_zdt3()),
        "welded_beam" | "weldedbeam" => Ok(builtin_welded_beam()),
        "pressure_vessel" | "pressurevessel" => Ok(builtin_pressure_vessel()),
        _ => Err(format!("Unknown built-in problem: {}. Available: zdt1, zdt2, zdt3, welded_beam, pressure_vessel", name)),
    }
}

pub fn resolve_problem(config: &Config) -> Result<Problem, String> {
    if let Some(ref builtin) = config.builtin {
        load_builtin(builtin)
    } else if let Some(ref prob_cfg) = config.problem {
        Problem::from_config(prob_cfg)
    } else {
        Err("Either 'builtin' or 'problem' must be specified in config".to_string())
    }
}
