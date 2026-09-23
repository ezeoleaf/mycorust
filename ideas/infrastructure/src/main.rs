//! Bottom-up infrastructure: nutrients as demand, obstacles as terrain.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 80;
    c.max_hyphae = 500;
    c.max_hyphae_branching_threshold = 400;
    c.obstacle_count = 0;
    c.zones_enabled = false;
    c.weather_enabled = false;
    c.initial_hyphae_count = 5;
    c.record_segments = false;
    c
}

fn main() {
    let mut rng = StdRng::seed_from_u64(21);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());
    let n = sim.config.grid_size;

    // Terrain: a river band that roads/hyphae must go around.
    for x in 0..n {
        for y in n / 2 - 3..n / 2 + 3 {
            sim.state.obstacles[x][y] = x % 12 != 0; // sparse bridges
        }
    }

    // Demand: three towns.
    sim.state.nutrients.clear();
    sim.state.nutrients.add_patch(12, 12, 8.0, 1.0, 0.4);
    sim.state.nutrients.add_patch(n - 12, 14, 8.0, 1.0, 0.4);
    sim.state.nutrients.add_patch(n / 2, n - 12, 9.0, 1.0, 0.5);

    println!("MycoRust infrastructure");
    println!("Towns = nutrient demand, river = obstacle band with rare bridges");
    sim.step_n(&mut rng, 550);

    let m = analyze(&sim);
    println!(
        "Grown network: {} segments as candidate roads, {} junctions",
        sim.state.segments.len(),
        m.connections
    );
    println!(
        "Fault tolerance: {:.2} mean degree, {} loops, fragmentation={:.3}",
        m.mean_degree,
        m.cycle_count.max(0),
        m.fragmentation
    );
    println!("Resilience score: {:.3}", m.resilience_score());
}
