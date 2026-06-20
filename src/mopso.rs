use crate::archive::Archive;
use crate::config::{AlgorithmConfig, InertiaWeightConfig};
use crate::metrics;
use crate::particle::{Particle, Solution, update_particle};
use crate::problem::Problem;
use rand::Rng;

pub struct MopsoResult {
    pub archive_members: Vec<Solution>,
    pub convergence: Vec<f64>,
}

pub fn run_mopso<R: Rng>(
    problem: &Problem,
    config: &AlgorithmConfig,
    ref_point: Option<&[f64]>,
    rng: &mut R,
    progress_callback: &mut dyn FnMut(usize, usize, usize, Option<f64>),
) -> MopsoResult {
    let pop_size = config.population_size;
    let max_iter = config.max_iterations;
    let archive_capacity = config.archive_size;
    let c1 = config.c1;
    let c2 = config.c2;
    let grid_divisions = config.grid_divisions;
    let variant = config.variant.to_lowercase();

    let mut archive = Archive::new(archive_capacity, grid_divisions);
    let mut particles: Vec<Particle> = (0..pop_size)
        .map(|_| Particle::new(rng, problem))
        .collect();

    for p in &particles {
        let sol = Solution::new(
            p.best_position.clone(),
            p.best_objectives.clone(),
            p.best_violation,
        );
        archive.try_add(sol);
    }

    let mut convergence: Vec<f64> = Vec::new();
    let mut no_improve_count = 0;
    let mut adaptive_w: f64 = 0.9;

    for iter in 0..max_iter {
        let w = compute_inertia_weight(&config.inertia_weight, iter, max_iter, &variant, adaptive_w);

        for p in &mut particles {
            let leader_idx = archive.select_leader(rng);
            if let Some(idx) = leader_idx {
                if idx < archive.members.len() {
                    let leader_sol = archive.members[idx].clone();
                    update_particle(p, &leader_sol, w, c1, c2, problem, rng);
                }
            }

            let sol = Solution::new(
                p.position.clone(),
                p.objectives.clone(),
                p.violation,
            );
            archive.try_add(sol);
        }

        if !archive.verify_nondominated() {
            let mut new_archive = Archive::new(archive_capacity, grid_divisions);
            let members = std::mem::take(&mut archive.members);
            for m in members {
                new_archive.try_add(m);
            }
            archive = new_archive;
        }

        let hv = if let Some(rp) = ref_point {
            Some(metrics::hypervolume(&archive.members, rp))
        } else {
            None
        };

        if variant == "adaptive" {
            let prev_hv = convergence.last().copied().unwrap_or(-1.0);
            let current_hv = hv.unwrap_or(0.0);
            if current_hv <= prev_hv + 1e-12 {
                no_improve_count += 1;
            } else {
                no_improve_count = 0;
            }

            if no_improve_count > 10 {
                adaptive_w = (adaptive_w + 0.05).min(0.9);
            } else {
                adaptive_w = (adaptive_w - 0.02).max(0.4);
            }
        }

        if let Some(hv_val) = hv {
            convergence.push(hv_val);
        } else {
            convergence.push(archive.members.len() as f64);
        }

        progress_callback(iter + 1, max_iter, archive.members.len(), hv);
    }

    MopsoResult {
        archive_members: archive.members,
        convergence,
    }
}

fn compute_inertia_weight(
    config: &Option<InertiaWeightConfig>,
    iter: usize,
    max_iter: usize,
    variant: &str,
    adaptive_w: f64,
) -> f64 {
    if variant == "adaptive" {
        return adaptive_w;
    }

    match config {
        Some(InertiaWeightConfig::Fixed(v)) => *v,
        Some(InertiaWeightConfig::Linear { from, to }) => {
            let progress = iter as f64 / max_iter.max(1) as f64;
            from + (to - from) * progress
        }
        None => {
            let progress = iter as f64 / max_iter.max(1) as f64;
            0.9 + (0.4 - 0.9) * progress
        }
    }
}
