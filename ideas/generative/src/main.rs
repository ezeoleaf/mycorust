//! Generative mycelium art — biology as the noise function.
//!
//! Writes an SVG of hyphal trails to the working directory.

use mycorust_core::{Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::fs;

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 100;
    c.max_hyphae = 600;
    c.max_hyphae_branching_threshold = 480;
    c.obstacle_count = 40;
    c.weather_enabled = true;
    c.seasonal_cycles_enabled = false;
    c.initial_hyphae_count = 8;
    c
}

fn main() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());
    sim.step_n(&mut rng, 500);

    let size = sim.config.grid_size as f32 * sim.config.cell_size;
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{size}" height="{size}">"#
    ));
    svg.push_str(r##"<rect width="100%" height="100%" fill="#0c1a3a"/>"##);

    for seg in &sim.state.segments {
        let fade = 1.0 - (seg.age / sim.config.max_segment_age).clamp(0.0, 1.0);
        let w = 0.6 + fade * 1.2;
        svg.push_str(&format!(
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#d8c9a3" stroke-opacity="{:.3}" stroke-width="{:.2}"/>"##,
            seg.from.x, seg.from.y, seg.to.x, seg.to.y, 0.15 + fade * 0.55, w
        ));
    }

    for conn in &sim.state.connections {
        let h1 = &sim.state.hyphae[conn.hypha1];
        let h2 = &sim.state.hyphae[conn.hypha2];
        svg.push_str(&format!(
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#6fd3a2" stroke-opacity="0.45" stroke-width="1.4"/>"##,
            h1.x * sim.config.cell_size,
            h1.y * sim.config.cell_size,
            h2.x * sim.config.cell_size,
            h2.y * sim.config.cell_size
        ));
    }
    svg.push_str("</svg>");

    let path = "mycelium-art.svg";
    fs::write(path, svg).expect("write svg");
    println!("MycoRust generative");
    println!(
        "Wrote {path} ({} trails, {} fusions)",
        sim.state.segments.len(),
        sim.state.connections.len()
    );
}
