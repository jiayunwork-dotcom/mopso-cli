use crate::problem::Problem;
use rand::Rng;

#[derive(Clone)]
pub struct Solution {
    pub position: Vec<f64>,
    pub objectives: Vec<f64>,
    pub constraint_violation: f64,
}

impl Solution {
    pub fn new(position: Vec<f64>, objectives: Vec<f64>, violation: f64) -> Self {
        Solution { position, objectives, constraint_violation: violation }
    }

    pub fn is_feasible(&self) -> bool {
        self.constraint_violation <= 1e-12
    }

    pub fn dominates(&self, other: &Solution) -> Dominance {
        if self.objectives.len() != other.objectives.len() {
            return Dominance::Incomparable;
        }
        let mut better = false;
        let mut worse = false;
        for i in 0..self.objectives.len() {
            if self.objectives[i] < other.objectives[i] - 1e-12 {
                better = true;
            }
            if self.objectives[i] > other.objectives[i] + 1e-12 {
                worse = true;
            }
        }
        if better && !worse {
            Dominance::Dominates
        } else if worse && !better {
            Dominance::Dominated
        } else {
            Dominance::Nondominated
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Dominance {
    Dominates,
    Dominated,
    Nondominated,
    Incomparable,
}

pub fn is_better(a: &Solution, b: &Solution) -> bool {
    match (a.is_feasible(), b.is_feasible()) {
        (true, true) => matches!(a.dominates(b), Dominance::Dominates),
        (true, false) => true,
        (false, true) => false,
        (false, false) => a.constraint_violation < b.constraint_violation,
    }
}

pub struct Particle {
    pub position: Vec<f64>,
    pub velocity: Vec<f64>,
    pub best_position: Vec<f64>,
    pub best_objectives: Vec<f64>,
    pub best_violation: f64,
    pub objectives: Vec<f64>,
    pub violation: f64,
}

impl Particle {
    pub fn new<R: Rng>(rng: &mut R, problem: &Problem) -> Self {
        let lb = problem.lower_bounds();
        let ub = problem.upper_bounds();
        let dim = lb.len();

        let mut position = Vec::with_capacity(dim);
        for i in 0..dim {
            let val = lb[i] + rng.gen::<f64>() * (ub[i] - lb[i]);
            position.push(val);
        }
        let is_int = problem.is_integer();
        for i in 0..dim {
            if is_int[i] {
                position[i] = position[i].round();
            }
        }

        let (objectives, violation) = problem.evaluate(&position);

        let mut velocity = Vec::with_capacity(dim);
        for i in 0..dim {
            let range = ub[i] - lb[i];
            velocity.push((rng.gen::<f64>() * 2.0 - 1.0) * range * 0.1);
        }

        Particle {
            position: position.clone(),
            velocity,
            best_position: position,
            best_objectives: objectives.clone(),
            best_violation: violation,
            objectives,
            violation,
        }
    }
}

pub fn update_particle<R: Rng>(
    particle: &mut Particle,
    leader: &Solution,
    w: f64,
    c1: f64,
    c2: f64,
    problem: &Problem,
    rng: &mut R,
) {
    let lb = problem.lower_bounds();
    let ub = problem.upper_bounds();
    let dim = lb.len();
    let is_int = problem.is_integer();

    let vmax: Vec<f64> = (0..dim).map(|i| (ub[i] - lb[i]) * 0.2).collect();

    for i in 0..dim {
        let r1: f64 = rng.gen();
        let r2: f64 = rng.gen();

        particle.velocity[i] = w * particle.velocity[i]
            + c1 * r1 * (particle.best_position[i] - particle.position[i])
            + c2 * r2 * (leader.position[i] - particle.position[i]);

        if particle.velocity[i] > vmax[i] {
            particle.velocity[i] = vmax[i];
        }
        if particle.velocity[i] < -vmax[i] {
            particle.velocity[i] = -vmax[i];
        }

        particle.position[i] += particle.velocity[i];

        if particle.position[i] < lb[i] {
            particle.position[i] = lb[i] + (lb[i] - particle.position[i]);
            if particle.position[i] > ub[i] {
                particle.position[i] = ub[i];
            }
            particle.velocity[i] *= -0.5;
        }
        if particle.position[i] > ub[i] {
            particle.position[i] = ub[i] - (particle.position[i] - ub[i]);
            if particle.position[i] < lb[i] {
                particle.position[i] = lb[i];
            }
            particle.velocity[i] *= -0.5;
        }

        if is_int[i] {
            particle.position[i] = particle.position[i].round();
        }
    }

    let (objectives, violation) = problem.evaluate(&particle.position);
    particle.objectives = objectives.clone();
    particle.violation = violation;

    let new_sol = Solution::new(particle.position.clone(), objectives.clone(), violation);
    let best_sol = Solution::new(
        particle.best_position.clone(),
        particle.best_objectives.clone(),
        particle.best_violation,
    );

    if is_better(&new_sol, &best_sol) {
        particle.best_position = particle.position.clone();
        particle.best_objectives = objectives;
        particle.best_violation = violation;
    } else if matches!(new_sol.dominates(&best_sol), Dominance::Nondominated) && rng.gen::<f64>() < 0.5 {
        particle.best_position = particle.position.clone();
        particle.best_objectives = objectives;
        particle.best_violation = violation;
    }
}
