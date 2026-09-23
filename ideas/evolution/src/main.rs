//! Evolutionary sandbox: mutate growth strategies and select by fitness.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, Debug)]
struct Genome {
    branch_prob: f32,
    wander: f32,
    anastomosis: f32,
    memory_influence: f32,
}

impl Genome {
    fn random(rng: &mut impl Rng) -> Self {
        Self {
            branch_prob: rng.gen_range(0.003..0.02),
            wander: rng.gen_range(0.02..0.12),
            anastomosis: rng.gen_range(1.2..3.5),
            memory_influence: rng.gen_range(0.05..0.35),
        }
    }

    fn mutate(&self, rng: &mut impl Rng) -> Self {
        fn jitter(v: f32, s: f32, lo: f32, hi: f32, rng: &mut impl Rng) -> f32 {
            (v + rng.gen_range(-s..s)).clamp(lo, hi)
        }
        Self {
            branch_prob: jitter(self.branch_prob, 0.003, 0.002, 0.03, rng),
            wander: jitter(self.wander, 0.02, 0.01, 0.2, rng),
            anastomosis: jitter(self.anastomosis, 0.3, 1.0, 4.0, rng),
            memory_influence: jitter(self.memory_influence, 0.05, 0.0, 0.5, rng),
        }
    }

    fn to_config(&self) -> SimulationConfig {
        let mut c = SimulationConfig::default();
        c.grid_size = 56;
        c.max_hyphae = 220;
        c.max_hyphae_branching_threshold = 180;
        c.obstacle_count = 25;
        c.weather_enabled = false;
        c.initial_hyphae_count = 4;
        c.branch_prob = self.branch_prob;
        c.angle_wander_range = self.wander;
        c.anastomosis_distance = self.anastomosis;
        c.memory_influence = self.memory_influence;
        c.record_segments = false;
        c
    }
}

fn fitness(genome: &Genome, seed: u64) -> f32 {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut sim = Simulation::with_config(&mut rng, genome.to_config());
    sim.step_n(&mut rng, 280);
    let m = analyze(&sim);
    m.hyphae_alive as f32 * 0.4 + m.resilience_score() * 80.0 + m.cycle_count.max(0) as f32 * 2.0
        - m.fragmentation * 40.0
}

fn main() {
    let mut rng = StdRng::seed_from_u64(99);
    let pop_size = 8;
    let mut population: Vec<Genome> = (0..pop_size).map(|_| Genome::random(&mut rng)).collect();

    println!("MycoRust evolution");
    println!("Selecting for biomass + resilience, against fragmentation.\n");

    for gen in 0..5 {
        let mut scored: Vec<(f32, Genome)> = population
            .iter()
            .map(|g| (fitness(g, 1000 + gen as u64), g.clone()))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        println!(
            "gen {gen}  best={:.1}  branch={:.4} wander={:.3} fuse={:.2} memory={:.2}",
            scored[0].0,
            scored[0].1.branch_prob,
            scored[0].1.wander,
            scored[0].1.anastomosis,
            scored[0].1.memory_influence
        );
        let elite = scored[0].1.clone();
        let second = scored[1].1.clone();
        population = vec![elite.clone(), second.clone()];
        while population.len() < pop_size {
            let parent = if rng.gen_bool(0.6) {
                &elite
            } else {
                &second
            };
            population.push(parent.mutate(&mut rng));
        }
    }
}
