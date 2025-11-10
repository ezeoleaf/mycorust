use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;

use crate::simulation::{EditorTool, Simulation};

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

    if is_key_pressed(KeyCode::H) {
        sim.toggle_hyphae_visibility();
    }

    // Speed controls
    if is_key_pressed(KeyCode::Right) {
        sim.increase_speed();
    }
    if is_key_pressed(KeyCode::Left) {
        sim.decrease_speed();
    }
    if is_key_pressed(KeyCode::Key0) {
        sim.reset_speed();
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

    if is_key_pressed(KeyCode::T) {
        // Add nitrogen patch at mouse position
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nitrogen_patch(gx, gy);
    }

    // Mouse interaction (works even when paused)
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nutrient_cell(gx, gy);
    }

    if is_mouse_button_pressed(MouseButton::Right) {
        // Right click to add nitrogen cell
        let (mx, my) = mouse_position();
        let gx = (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nitrogen_cell(gx, gy);
    }

    // Editor mode controls
    if is_key_pressed(KeyCode::E) {
        sim.toggle_editor_mode();
    }

    if sim.editor_mode {
        // Tool selection
        if is_key_pressed(KeyCode::Key1) {
            sim.set_editor_tool(EditorTool::Sugar);
        }
        if is_key_pressed(KeyCode::Key2) {
            sim.set_editor_tool(EditorTool::Nitrogen);
        }
        if is_key_pressed(KeyCode::Key3) {
            sim.set_editor_tool(EditorTool::Erase);
        }

        // Brush size controls
        if is_key_pressed(KeyCode::Equal) {
            sim.editor_brush_size = (sim.editor_brush_size + 1).min(20);
        }
        if is_key_pressed(KeyCode::Minus) {
            sim.editor_brush_size = (sim.editor_brush_size.saturating_sub(1)).max(1);
        }

        // Start simulation from editor
        if is_key_pressed(KeyCode::Enter) {
            sim.start_simulation_from_editor(rng);
        }

        // Drawing with mouse (both pressed and held)
        if is_mouse_button_down(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let gx =
                (mx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
            let gy =
                (my / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
            sim.editor_draw_at(gx, gy);
        }
    }
}
