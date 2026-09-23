//! Ethics / systems metaphor: growth vs remaining resources vs collapse.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config(exploit: bool) -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 72;
    c.max_hyphae = if exploit { 700 } else { 280 };
    c.max_hyphae_branching_threshold = if exploit { 600 } else { 220 };
    c.branch_prob = if exploit { 0.02 } else { 0.006 };
    c.nutrient_decay = if exploit { 0.03 } else { 0.008 };
    c.nutrient_regen_rate = if exploit { 0.001 } else { 0.006 };
    c.obstacle_count = 15;
    c.weather_enabled = false;
    c.initial_hyphae_count = 6;
    c.record_segments = false;
    c
}

fn run(label: &str, exploit: bool) {
    let mut rng = StdRng::seed_from_u64(if exploit { 1 } else { 2 });
    let mut sim = Simulation::with_config(&mut rng, experiment_config(exploit));
    sim.step_n(&mut rng, 500);
    let m = analyze(&sim);
    let remaining = m.nutrient_mass;
    let collapse = m.fragmentation;
    println!(
        "{label:<12} biomass={:<4} nutrients={:<8.1} fragments={:.3} resilience={:.3}",
        m.hyphae_alive,
        remaining,
        collapse,
        m.resilience_score()
    );
}

fn main() {
    println!("MycoRust ethics sandbox");
    println!("Two strategies on the same world: extract vs steward.\n");
    run("exploit", true);
    run("steward", false);
    println!("\nExploit grows faster, then starves the field.");
    println!("Steward keeps more nutrients and a more intact network.");
}
