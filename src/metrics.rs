use crate::particle::Solution;

pub fn hypervolume(solutions: &[Solution], reference_point: &[f64]) -> f64 {
    if solutions.is_empty() || reference_point.is_empty() {
        return 0.0;
    }
    let n_objs = reference_point.len();
    let feasible: Vec<&Solution> = solutions.iter().filter(|s| s.is_feasible()).collect();
    if feasible.is_empty() {
        return 0.0;
    }

    let mut points: Vec<Vec<f64>> = feasible.iter().map(|s| {
        s.objectives[..n_objs].to_vec()
    }).collect();

    for p in &mut points {
        for k in 0..n_objs {
            p[k] = reference_point[k] - p[k];
            if p[k] < 0.0 {
                p[k] = 0.0;
            }
        }
    }

    if n_objs == 1 {
        return points.iter().map(|p| p[0]).fold(0.0f64, f64::max);
    }
    if n_objs == 2 {
        return hv_2d(&points);
    }

    hv_2d(&points)
}

fn hv_2d(points: &[Vec<f64>]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<&Vec<f64>> = points.iter().collect();
    sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));

    let mut hv = 0.0;
    let mut prev_y = 0.0f64;

    for i in 0..sorted.len() {
        let x = sorted[i][0];
        let y = sorted[i][1];
        if x > 0.0 && y > prev_y {
            hv += x * (y - prev_y);
            prev_y = y;
        }
    }

    hv
}

pub fn igd(solutions: &[Solution], true_pareto: &[Vec<f64>]) -> f64 {
    if solutions.is_empty() || true_pareto.is_empty() {
        return f64::INFINITY;
    }
    let feasible: Vec<&Solution> = solutions.iter().filter(|s| s.is_feasible()).collect();
    if feasible.is_empty() {
        return f64::INFINITY;
    }

    let mut total = 0.0;
    for tp in true_pareto {
        let min_dist = feasible.iter().map(|s| {
            s.objectives.iter().zip(tp.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt()
        }).fold(f64::INFINITY, f64::min);
        total += min_dist;
    }
    total / true_pareto.len() as f64
}

pub fn spacing(solutions: &[Solution]) -> f64 {
    let feasible: Vec<&Solution> = solutions.iter().filter(|s| s.is_feasible()).collect();
    if feasible.len() < 2 {
        return 0.0;
    }

    let n = feasible.len();
    let mut distances = Vec::with_capacity(n);

    for i in 0..n {
        let mut min_d = f64::INFINITY;
        for j in 0..n {
            if i == j { continue; }
            let d: f64 = feasible[i].objectives.iter().zip(feasible[j].objectives.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();
            if d < min_d { min_d = d; }
        }
        distances.push(min_d);
    }

    let mean = distances.iter().sum::<f64>() / n as f64;
    if mean < 1e-12 {
        return 0.0;
    }

    let variance: f64 = distances.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n as f64;
    variance.sqrt()
}

pub fn load_true_pareto(path: &str) -> Result<Vec<Vec<f64>>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read true Pareto file '{}': {}", path, e))?;
    let mut result = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let vals: Result<Vec<f64>, _> = line.split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<f64>())
            .collect();
        match vals {
            Ok(v) => result.push(v),
            Err(_) => continue,
        }
    }
    Ok(result)
}
