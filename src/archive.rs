use crate::particle::{Solution, Dominance};
use rand::Rng;

pub struct Archive {
    pub members: Vec<Solution>,
    pub capacity: usize,
    pub grid_divisions: usize,
}

impl Archive {
    pub fn new(capacity: usize, grid_divisions: usize) -> Self {
        Archive {
            members: Vec::new(),
            capacity,
            grid_divisions,
        }
    }

    pub fn try_add(&mut self, solution: Solution) -> bool {
        let mut dominated_indices = Vec::new();
        let mut is_dominated = false;

        for (i, member) in self.members.iter().enumerate() {
            let rel = constraint_dominance(&solution, member);
            match rel {
                CDRelation::Dominates => {
                    dominated_indices.push(i);
                }
                CDRelation::Dominated => {
                    is_dominated = true;
                    break;
                }
                CDRelation::Equal | CDRelation::Nondominated => {}
            }
        }

        if is_dominated {
            return false;
        }

        for &i in dominated_indices.iter().rev() {
            self.members.remove(i);
        }

        self.members.push(solution);

        if self.members.len() > self.capacity {
            self.prune();
        }

        true
    }

    fn prune(&mut self) {
        if self.members.len() <= self.capacity {
            return;
        }
        if self.members.is_empty() {
            return;
        }

        let n_objs = self.members[0].objectives.len();
        if n_objs == 0 {
            while self.members.len() > self.capacity {
                self.members.pop();
            }
            return;
        }

        let crowding = compute_crowding_distances(&self.members);
        let mut indexed: Vec<(usize, f64)> = crowding.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let to_remove = self.members.len() - self.capacity;
        let mut remove_set = std::collections::HashSet::new();
        for i in 0..to_remove {
            remove_set.insert(indexed[i].0);
        }

        let mut new_members = Vec::with_capacity(self.capacity);
        for (i, m) in self.members.drain(..).enumerate() {
            if !remove_set.contains(&i) {
                new_members.push(m);
            }
        }
        self.members = new_members;
    }

    pub fn select_leader<R: Rng>(&self, rng: &mut R) -> Option<usize> {
        if self.members.is_empty() {
            return None;
        }
        if self.members.len() == 1 {
            return Some(0);
        }

        let n_objs = self.members[0].objectives.len();
        let grid_div = self.grid_divisions as f64;

        let mut obj_min = vec![f64::INFINITY; n_objs];
        let mut obj_max = vec![f64::NEG_INFINITY; n_objs];
        for m in &self.members {
            for k in 0..n_objs {
                if m.objectives[k] < obj_min[k] { obj_min[k] = m.objectives[k]; }
                if m.objectives[k] > obj_max[k] { obj_max[k] = m.objectives[k]; }
            }
        }

        let mut grid_coords: Vec<Vec<usize>> = Vec::with_capacity(self.members.len());
        for m in &self.members {
            let mut coords = Vec::with_capacity(n_objs);
            for k in 0..n_objs {
                let range = obj_max[k] - obj_min[k];
                let coord = if range < 1e-12 {
                    self.grid_divisions / 2
                } else {
                    let normalized = (m.objectives[k] - obj_min[k]) / range;
                    let c = (normalized * grid_div) as usize;
                    if c >= self.grid_divisions { self.grid_divisions - 1 } else { c }
                };
                coords.push(coord);
            }
            grid_coords.push(coords);
        }

        let mut grid_counts: std::collections::HashMap<Vec<usize>, usize> = std::collections::HashMap::new();
        for coords in &grid_coords {
            *grid_counts.entry(coords.clone()).or_insert(0) += 1;
        }

        let max_count = *grid_counts.values().max().unwrap_or(&1).max(&1) as f64;
        let mut roulette: Vec<f64> = Vec::with_capacity(self.members.len());
        let mut total = 0.0;
        for coords in &grid_coords {
            let count = *grid_counts.get(coords).unwrap_or(&1) as f64;
            let fitness = max_count - count + 1.0;
            total += fitness;
            roulette.push(total);
        }

        let r = rng.gen::<f64>() * total;
        for (i, &val) in roulette.iter().enumerate() {
            if r <= val {
                return Some(i);
            }
        }
        Some(self.members.len() - 1)
    }

    pub fn verify_nondominated(&self) -> bool {
        for i in 0..self.members.len() {
            for j in 0..self.members.len() {
                if i != j {
                    if matches!(constraint_dominance(&self.members[i], &self.members[j]), CDRelation::Dominates) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

enum CDRelation {
    Dominates,
    Dominated,
    Equal,
    Nondominated,
}

fn constraint_dominance(a: &Solution, b: &Solution) -> CDRelation {
    match (a.is_feasible(), b.is_feasible()) {
        (true, true) => match a.dominates(b) {
            Dominance::Dominates => CDRelation::Dominates,
            Dominance::Dominated => CDRelation::Dominated,
            _ => CDRelation::Nondominated,
        },
        (true, false) => CDRelation::Dominates,
        (false, true) => CDRelation::Dominated,
        (false, false) => {
            if a.constraint_violation < b.constraint_violation - 1e-12 {
                CDRelation::Dominates
            } else if b.constraint_violation < a.constraint_violation - 1e-12 {
                CDRelation::Dominated
            } else {
                CDRelation::Equal
            }
        }
    }
}

fn compute_crowding_distances(solutions: &[Solution]) -> Vec<f64> {
    let n = solutions.len();
    if n <= 2 {
        return vec![f64::INFINITY; n];
    }

    let n_objs = solutions[0].objectives.len();
    let mut distances = vec![0.0; n];

    for k in 0..n_objs {
        let mut indices: Vec<usize> = (0..n).collect();
        indices.sort_by(|&a, &b| {
            solutions[a].objectives[k].partial_cmp(&solutions[b].objectives[k])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        distances[indices[0]] = f64::INFINITY;
        distances[indices[n - 1]] = f64::INFINITY;

        let f_min = solutions[indices[0]].objectives[k];
        let f_max = solutions[indices[n - 1]].objectives[k];
        let range = f_max - f_min;
        if range < 1e-12 {
            continue;
        }

        for i in 1..n - 1 {
            let prev = solutions[indices[i - 1]].objectives[k];
            let next = solutions[indices[i + 1]].objectives[k];
            distances[indices[i]] += (next - prev) / range;
        }
    }

    distances
}
