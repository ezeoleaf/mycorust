//! Fungal network as a routing algorithm.
//!
//! Two nutrient patches are placed at opposite corners. After growth, we
//! compare the mycelium path with the Euclidean straight-line distance.

use mycorust_core::{
    analyze, nearest_hypha, shortest_path_length, Simulation, SimulationConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 80;
    c.max_hyphae = 500;
    c.max_hyphae_branching_threshold = 400;
    c.obstacle_count = 80;
    c.zones_enabled = false;
    c.weather_enabled = false;
    c.initial_hyphae_count = 3;
    c.record_segments = false;
    c
}

fn main() {
    let mut rng = StdRng::seed_from_u64(11);
    let mut config = experiment_config();
    config.nutrient_regen_rate = 0.0;
    let size = config.grid_size;
    let mut sim = Simulation::with_config(&mut rng, config);

    sim.state.nutrients.clear();
    let src = (8usize, 8usize);
    let dst = (size - 9, size - 9);
    sim.state.nutrients.add_patch(src.0, src.1, 7.0, 1.0, 0.6);
    sim.state.nutrients.add_patch(dst.0, dst.1, 7.0, 1.0, 0.6);

    println!("MycoRust routing");
    println!(
        "Nutrient patches at ({}, {}) and ({}, {})",
        src.0, src.1, dst.0, dst.1
    );
    sim.step_n(&mut rng, 600);

    let metrics = analyze(&sim);
    let start = nearest_hypha(&sim, src.0 as f32, src.1 as f32);
    let goal = nearest_hypha(&sim, dst.0 as f32, dst.1 as f32);

    let euclidean = {
        let dx = (dst.0 as f32) - (src.0 as f32);
        let dy = (dst.1 as f32) - (src.1 as f32);
        (dx * dx + dy * dy).sqrt()
    };

    println!(
        "Network: {} hyphae, {} connections, {:.1} extra loops",
        metrics.hyphae_alive, metrics.connections, metrics.cycle_count as f32
    );

    match (start, goal) {
        (Some(s), Some(g)) => match shortest_path_length(&sim, s, g) {
            Some(path) => {
                let stretch = path / euclidean.max(0.001);
                println!("Euclidean distance: {euclidean:.2}");
                println!("Mycelium path:      {path:.2}");
                println!("Stretch (path/euclid): {stretch:.3}");
                println!(
                    "Redundancy (mean degree): {:.2}",
                    metrics.mean_degree
                );
            }
            None => println!("Network did not connect the two patches."),
        },
        _ => println!("No living hyphae near one of the patches."),
    }
}
