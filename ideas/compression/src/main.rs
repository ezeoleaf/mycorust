//! Living compression: a nutrient "image" is reduced to a mycelial graph.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 64;
    c.max_hyphae = 350;
    c.max_hyphae_branching_threshold = 280;
    c.obstacle_count = 0;
    c.zones_enabled = false;
    c.weather_enabled = false;
    c.initial_hyphae_count = 4;
    c.nutrient_regen_rate = 0.0;
    c.record_segments = false;
    c
}

/// Paint a block-letter "M" into the nutrient field.
fn paint_letter_m(sim: &mut Simulation) {
    sim.state.nutrients.clear();
    let n = sim.config.grid_size;
    let stroke = (n as f32 * 0.08).max(2.0) as usize;
    for y in n / 6..n * 5 / 6 {
        for x in 0..stroke {
            sim.state.nutrients.add_sugar(n / 6 + x, y, 1.0);
            sim.state.nutrients.add_sugar(n * 5 / 6 - x, y, 1.0);
        }
    }
    for t in 0..=20 {
        let x1 = n / 6 + (t * (n / 2 - n / 6)) / 20;
        let y1 = n / 6 + (t * (n / 2 - n / 6)) / 20;
        let x2 = n / 2 + (t * (n * 5 / 6 - n / 2)) / 20;
        let y2 = n / 2 - (t * (n / 2 - n / 6)) / 20;
        for w in 0..stroke {
            if x1 + w < n {
                sim.state.nutrients.add_sugar(x1 + w, y1, 1.0);
            }
            if x2 + w < n {
                sim.state.nutrients.add_sugar(x2 + w, y2, 1.0);
            }
        }
    }
}

fn main() {
    let mut rng = StdRng::seed_from_u64(3);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());
    paint_letter_m(&mut sim);

    let input_cells = sim.state.nutrients.occupied_cells(0.2);
    println!("MycoRust compression");
    println!("Input glyph occupies {input_cells} nutrient cells");

    sim.step_n(&mut rng, 500);
    let metrics = analyze(&sim);
    let graph_size = metrics.hyphae_alive + metrics.connections;
    let ratio = graph_size as f32 / input_cells.max(1) as f32;

    println!(
        "Mycelial graph: {} nodes + {} edges = {} elements",
        metrics.hyphae_alive, metrics.connections, graph_size
    );
    println!("Compression ratio (graph/input): {ratio:.3}");
    println!(
        "What the fungus kept: {} loops, fragmentation={:.3}",
        metrics.cycle_count.max(0),
        metrics.fragmentation
    );
}
