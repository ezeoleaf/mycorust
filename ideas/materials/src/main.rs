//! Digital twin metrics for a mycelium composite: density, porosity, thickness.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 80;
    c.max_hyphae = 550;
    c.max_hyphae_branching_threshold = 450;
    c.obstacle_count = 10;
    c.weather_enabled = false;
    c.density_inhibition_enabled = true;
    c.initial_hyphae_count = 10;
    c.record_segments = false;
    c
}

fn main() {
    let mut rng = StdRng::seed_from_u64(5);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());
    sim.step_n(&mut rng, 450);

    let m = analyze(&sim);
    let cells = sim.config.grid_size * sim.config.grid_size;
    let occupied = sim
        .state
        .hyphae
        .iter()
        .filter(|h| h.alive)
        .map(|h| ((h.x as usize), (h.y as usize)))
        .collect::<std::collections::HashSet<_>>()
        .len();
    let fill = occupied as f32 / cells as f32;
    let porosity = 1.0 - fill;
    let thickness = sim
        .state
        .hyphae
        .iter()
        .filter(|h| h.alive)
        .map(|h| 0.4 + h.age.min(20.0) * 0.04)
        .sum::<f32>()
        / m.hyphae_alive.max(1) as f32;

    println!("MycoRust materials twin");
    println!("Fill fraction:     {fill:.3}");
    println!("Porosity:          {porosity:.3}");
    println!("Mean thickness:    {thickness:.3} (age-scaled hyphal cords)");
    println!("Mean density map:  {:.4}", m.mean_density);
    println!("Failure hint:      fragmentation={:.3} (voids / weak spans)", m.fragmentation);
    println!(
        "Insulation proxy:  {:.3} (porosity * intactness)",
        porosity * (1.0 - m.fragmentation)
    );
}
