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
        draw_nutrients(&sim.state.nutrients);

        // Draw obstacles
        draw_obstacles(&sim.state.obstacles);

        // Redraw all past segments to keep trails visible (with fading)
        draw_segments(
            &sim.state.segments,
            sim.config.max_segment_age,
            sim.hyphae_visible,
        );

        // Draw anastomosis connections
        draw_connections(
            &sim.state.connections,
            &sim.state.hyphae,
            sim.connections_visible,
        );

        // Draw fruiting bodies with energy transfer visualization
        draw_fruit_bodies(&sim.state.fruit_bodies, &sim.state.hyphae);

        // Minimap overlay
        draw_minimap(&sim.state.nutrients, &sim.state.hyphae, sim.minimap_visible);

        // Update simulation only if not paused
        // Handle speed multiplier with accumulator for fractional speeds
        if !sim.paused {
            sim.speed_accumulator += sim.speed_multiplier;
            let steps = sim.speed_accumulator.floor() as usize;
            sim.speed_accumulator -= steps as f32;

            for _ in 0..steps {
                sim.step(&mut rng);
            }
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
            sim.speed_multiplier,
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
