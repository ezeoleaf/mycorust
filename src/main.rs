use ::rand as external_rand;
use external_rand::thread_rng;
use macroquad::prelude::*;

mod config;
mod controls;
mod hypha;
mod nutrients;
mod simulation;
mod spore;
mod types;
mod visualization;

use config::*;
use controls::handle_controls;
use simulation::Simulation;
use visualization::{
    draw_connections, draw_fruit_bodies, draw_minimap, draw_nutrients, draw_obstacles,
    draw_segments, draw_stats_and_help,
};

#[macroquad::main(window_conf)]
async fn main() {
    let mut rng = thread_rng();
    // Initialize simulation
    let mut sim = Simulation::new(&mut rng);

    loop {
        // Handle player controls
        handle_controls(&mut sim, &mut rng);

        // Blue background every frame so it stays visible
        clear_background(Color::new(0.05, 0.10, 0.35, 1.0));

        // Draw nutrients
        draw_nutrients(&sim.nutrients);

        // Draw obstacles
        draw_obstacles(&sim.obstacles);

        // Redraw all past segments to keep trails visible (with fading)
        draw_segments(&sim.segments);

        // Draw anastomosis connections
        draw_connections(&sim.connections, &sim.hyphae, sim.connections_visible);

        // Draw fruiting bodies
        draw_fruit_bodies(&sim.fruit_bodies);

        // Minimap overlay
        draw_minimap(&sim.nutrients, &sim.hyphae);

        // Update simulation only if not paused
        if !sim.paused {
            sim.step(&mut rng);
        }

        // Calculate statistics via simulation API
        let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, _total_energy) =
            sim.stats();

        // Draw statistics overlay and help
        draw_stats_and_help(
            hyphae_count,
            spores_count,
            connections_count,
            fruit_count,
            avg_energy,
            sim.paused,
        );

        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Mycelium Growth Simulation".to_owned(),
        window_width: (GRID_SIZE as f32 * CELL_SIZE) as i32,
        window_height: (GRID_SIZE as f32 * CELL_SIZE) as i32,
        ..Default::default()
    }
}
