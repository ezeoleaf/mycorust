use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;

use crate::simulation::Simulation;

pub fn handle_controls<R: Rng>(sim: &mut Simulation, rng: &mut R) {
    // Keyboard controls
    if is_key_pressed(KeyCode::Space) {
        sim.toggle_pause();
    }

    if is_key_pressed(KeyCode::R) {
        sim.reset(rng);
    }

    if is_key_pressed(KeyCode::C) {
        sim.clear_segments();
    }

    if is_key_pressed(KeyCode::X) {
        sim.toggle_connections();
    }

    if is_key_pressed(KeyCode::M) {
        sim.toggle_minimap();
    }

    if is_key_pressed(KeyCode::S) {
        // Spawn new hypha at mouse position
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0);
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0);
        sim.spawn_hypha_at(rng, gx, gy);
    }

    if is_key_pressed(KeyCode::N) {
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nutrient_patch(gx, gy);
    }

    // Mouse interaction (works even when paused)
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nutrient_cell(gx, gy);
    }
}
