//! Mycelium as a climate-response model.
//!
//! Grow a network, apply drought, then measure collapse, fragmentation, and recovery.

use mycorust_core::{analyze, EnvironmentalStress, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 80;
    c.max_hyphae = 400;
    c.max_hyphae_branching_threshold = 320;
    c.obstacle_count = 20;
    c.seasonal_cycles_enabled = false;
    c.initial_hyphae_count = 6;
    c.record_segments = false;
    c
}

fn main() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());

    println!("MycoRust climate indicator");
    println!("Growing a baseline network (400 steps)...");
    sim.step_n(&mut rng, 400);
    let baseline = analyze(&sim);
    println!(
        "  baseline  hyphae={:<4} fragments={:.3} resilience={:.3} moisture={:.3}",
        baseline.hyphae_alive,
        baseline.fragmentation,
        baseline.resilience_score(),
        baseline.mean_moisture
    );

    println!("Applying drought shock...");
    sim.apply_stress(EnvironmentalStress::Drought);
    sim.step_n(&mut rng, 200);
    let shocked = analyze(&sim);
    println!(
        "  drought   hyphae={:<4} fragments={:.3} resilience={:.3} moisture={:.3}",
        shocked.hyphae_alive,
        shocked.fragmentation,
        shocked.resilience_score(),
        shocked.mean_moisture
    );

    println!("Restoring mild conditions (recovery window)...");
    sim.state.weather.restore_baseline();
    sim.step_n(&mut rng, 300);
    let recovered = analyze(&sim);

    let recovery_time = recovered.hyphae_alive as f32 / baseline.hyphae_alive.max(1) as f32;
    println!(
        "  recovery  hyphae={:<4} fragments={:.3} resilience={:.3} biomass_ratio={:.3}",
        recovered.hyphae_alive,
        recovered.fragmentation,
        recovered.resilience_score(),
        recovery_time
    );
    println!();
    println!("Interpretation:");
    println!("  fragmentation  — network collapse / rerouting (higher = more broken)");
    println!("  resilience     — intact + energetic + loop-rich (0–1)");
    println!("  biomass_ratio  — recovered living hyphae vs pre-shock baseline");
}
