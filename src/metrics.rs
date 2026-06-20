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
            if p[k] > reference_point[k] {
                p[k] = reference_point[k];
            }
        }
    }

    if n_objs == 1 {
        let max_val = points.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        if max_val <= reference_point[0] {
            return reference_point[0] - max_val;
        }
        return 0.0;
    }
    if n_objs >= 2 {
        return hv_2d(&points, reference_point);
    }

    0.0
}

fn hv_2d(points: &[Vec<f64>], ref_point: &[f64]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let r1 = ref_point[0];
    let r2 = ref_point[1];

    let mut filtered: Vec<(f64, f64)> = points.iter()
        .map(|p| (p[0], p[1]))
        .filter(|(f1, f2)| *f1 <= r1 + 1e-12 && *f2 <= r2 + 1e-12)
        .collect();

    if filtered.is_empty() {
        return 0.0;
    }

    filtered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut nd: Vec<(f64, f64)> = Vec::new();
    for p in filtered {
        let dominated = nd.iter().any(|&(_, y2)| y2 <= p.1 + 1e-12);
        if !dominated {
            nd.push(p);
        }
    }

    let mut hv = 0.0;
    let mut prev_x = r1;

    for &(f1, f2) in nd.iter().rev() {
        if f1 < prev_x {
            hv += (prev_x - f1) * (r2 - f2);
            prev_x = f1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle::Solution;

    fn make_sol(obj: Vec<f64>) -> Solution {
        Solution::new(vec![0.0; 1], obj, 0.0)
    }

    #[test]
    fn test_hypervolume_2d_ideal_zdt1() {
        let mut sols = Vec::new();
        for i in 0..=100 {
            let f1 = i as f64 / 100.0;
            let g = 1.0;
            let f2 = g * (1.0 - (f1 / g).sqrt());
            sols.push(make_sol(vec![f1, f2]));
        }
        let ref_point = &[11.0, 11.0];
        let hv = hypervolume(&sols, ref_point);
        assert!(hv > 119.0 && hv < 121.0);
    }

    #[test]
    fn test_hypervolume_2d_simple() {
        let sols = vec![
            make_sol(vec![2.0, 3.0]),
            make_sol(vec![3.0, 2.0]),
            make_sol(vec![1.0, 5.0]),
        ];
        let ref_point = &[4.0, 6.0];
        let hv = hypervolume(&sols, ref_point);
        let expected = (4.0 - 1.0) * (6.0 - 5.0) + (4.0 - 2.0) * (5.0 - 3.0) + (4.0 - 3.0) * (3.0 - 2.0);
        assert!((hv - expected).abs() < 1e-6);
    }
}
