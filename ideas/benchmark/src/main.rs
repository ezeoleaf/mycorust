//! Network intelligence benchmark: maze, transport, memory, shock adaptation.

use mycorust_core::{
    analyze, nearest_hypha, shortest_path_length, EnvironmentalStress, Simulation, SimulationConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn maze_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 72;
    c.max_hyphae = 450;
    c.max_hyphae_branching_threshold = 360;
    c.obstacle_count = 0;
    c.zones_enabled = false;
    c.weather_enabled = true;
    c.seasonal_cycles_enabled = false;
    c.memory_enabled = true;
    c.initial_hyphae_count = 2;
    c.record_segments = false;
    c
}

fn build_maze(sim: &mut Simulation) {
    let n = sim.config.grid_size;
    for x in 0..n {
        for y in 0..n {
            sim.state.obstacles[x][y] = false;
        }
    }
    for x in 0..n {
        if x % 8 == 0 {
            for y in 0..n {
                if y % 10 != 4 {
                    sim.state.obstacles[x][y] = true;
                }
            }
        }
    }
    sim.state.nutrients.clear();
    sim.state.nutrients.add_patch(4, 4, 5.0, 1.0, 0.5);
    sim.state.nutrients.add_patch(n - 5, n - 5, 5.0, 1.0, 0.5);
}

fn main() {
    let mut rng = StdRng::seed_from_u64(19);
    let mut sim = Simulation::with_config(&mut rng, maze_config());
    build_maze(&mut sim);

    println!("MycoRust intelligence benchmark");
    println!("1. Maze transport");
    sim.step_n(&mut rng, 500);
    let n = sim.config.grid_size as f32;
    let start = nearest_hypha(&sim, 4.0, 4.0);
    let goal = nearest_hypha(&sim, n - 5.0, n - 5.0);
    let maze_ok = match (start, goal) {
        (Some(s), Some(g)) => shortest_path_length(&sim, s, g).is_some(),
        _ => false,
    };
    println!("   connected opposite corners: {maze_ok}");

    println!("2. Memory mass after foraging");
    let before_shock = analyze(&sim);
    println!("   memory_mass={:.1}", before_shock.memory_mass);

    println!("3. Shock adaptation");
    sim.apply_stress(EnvironmentalStress::HeatShock);
    sim.step_n(&mut rng, 150);
    let after = analyze(&sim);
    let adapt = after.resilience_score() / before_shock.resilience_score().max(0.001);
    println!(
        "   resilience {:.3} → {:.3} (ratio {adapt:.3})",
        before_shock.resilience_score(),
        after.resilience_score()
    );

    let score = (maze_ok as u8 as f32) * 0.4
        + (before_shock.memory_mass / 200.0).clamp(0.0, 1.0) * 0.3
        + adapt.clamp(0.0, 1.0) * 0.3;
    println!("Composite score (0–1): {score:.3}");
}
