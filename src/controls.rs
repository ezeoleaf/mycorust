use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;

use crate::simulation::Simulation;

pub struct ControlText {
    pub text: &'static str,
    pub font_size: f32,
    pub color: Color,
}

pub fn handle_controls<R: Rng>(sim: &mut Simulation, rng: &mut R) {
    // Keyboard controls
    // Only toggle pause if space is pressed without left mouse (to avoid conflict with pan)
    // Camera handles space+left for panning, so we only toggle pause if left mouse is not down
    if is_key_pressed(KeyCode::Space) && !is_mouse_button_down(MouseButton::Left) {
        sim.toggle_pause();
    }

    if is_key_pressed(KeyCode::R) {
        sim.reset(rng);
    }

    // Clear segments (Shift+C to avoid conflict with camera toggle)
    if (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift))
        && is_key_pressed(KeyCode::C)
    {
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

    if is_key_pressed(KeyCode::I) {
        // Network Intelligence: Toggle memory overlay
        sim.toggle_memory_visibility();
    }

    if is_key_pressed(KeyCode::V) {
        // Performance: Toggle enhanced visualization
        sim.toggle_enhanced_visualization();
    }

    if is_key_pressed(KeyCode::F) {
        // Performance: Toggle flow visualization
        sim.toggle_flow_visualization();
    }

    if is_key_pressed(KeyCode::Key1) {
        // Performance: Toggle stress visualization (number 1 key)
        sim.toggle_stress_visualization();
    }

    if sim.config.camera_enabled {
        // Camera controls
        if is_key_pressed(KeyCode::Home) {
            // Reset camera to default position and zoom
            sim.camera.reset();
        }
        // Toggle camera enabled/disabled (C key, but not when Shift is held for Clear)
        if is_key_pressed(KeyCode::C)
            && !is_key_down(KeyCode::LeftShift)
            && !is_key_down(KeyCode::RightShift)
        {
            sim.toggle_camera();
        }
    }

    // Screenshot (P key)
    if is_key_pressed(KeyCode::P) {
        // Set flag to take screenshot at end of frame
        sim.take_screenshot = true;
    }

    // Help popup (F1 key, or Escape to close when visible)
    if is_key_pressed(KeyCode::F1) {
        sim.toggle_help_popup();
    }
    // Also allow Escape to close the popup
    if sim.help_popup_visible && is_key_pressed(KeyCode::Escape) {
        sim.help_popup_visible = false;
    }

    // Speed controls (use Shift+Arrow to avoid conflict with camera panning)
    if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
        if is_key_pressed(KeyCode::Right) {
            sim.increase_speed();
        }
        if is_key_pressed(KeyCode::Left) {
            sim.decrease_speed();
        }
    }
    if is_key_pressed(KeyCode::Key0) {
        sim.reset_speed();
    }

    // Helper function to convert screen mouse position to world coordinates
    // Extract camera state to avoid borrowing conflicts
    let camera_enabled = sim.camera.enabled;
    let camera_x = sim.camera.x;
    let camera_y = sim.camera.y;
    let camera_zoom = sim.camera.zoom;
    let screen_width = screen_width();
    let screen_height = screen_height();

    let mouse_to_world = |mx: f32, my: f32| -> (f32, f32) {
        if camera_enabled {
            // Camera is enabled, use camera coordinate conversion
            // When camera is enabled, we need to account for zoom and position
            // Simplified: approximate conversion (actual conversion would use camera matrix)
            // At zoom 1.0, viewport shows grid_width units, so 1 screen pixel ≈ grid_width/screen_width world units
            use crate::config::{CELL_SIZE, GRID_SIZE};
            let grid_width = GRID_SIZE as f32 * CELL_SIZE;
            let viewport_width = grid_width / camera_zoom;

            // Convert screen coordinates to world coordinates
            // Screen center maps to camera target
            let world_x = (mx - screen_width * 0.5) * (viewport_width / screen_width) + camera_x;
            let world_y = (my - screen_height * 0.5) * (viewport_width / screen_width) + camera_y;
            (world_x, world_y)
        } else {
            // Camera is disabled, use direct mapping (1:1 screen to world)
            (mx, my)
        }
    };

    if is_key_pressed(KeyCode::S) {
        // Spawn new hypha at mouse position (in world coordinates)
        let (mx, my) = mouse_position();
        let (wx, wy) = mouse_to_world(mx, my);
        let gx = (wx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0);
        let gy = (wy / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0);
        sim.spawn_hypha_at(rng, gx, gy);
    }

    if is_key_pressed(KeyCode::N) {
        let (mx, my) = mouse_position();
        let (wx, wy) = mouse_to_world(mx, my);
        let gx = (wx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (wy / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nutrient_patch(gx, gy);
    }

    if is_key_pressed(KeyCode::T) {
        // Add nitrogen patch at mouse position (in world coordinates)
        let (mx, my) = mouse_position();
        let (wx, wy) = mouse_to_world(mx, my);
        let gx = (wx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (wy / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nitrogen_patch(gx, gy);
    }

    // Mouse interaction (works even when paused)
    // Only add nutrients if not panning (middle mouse or space+left)
    let is_panning = is_mouse_button_down(MouseButton::Middle)
        || (is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left));

    if is_mouse_button_pressed(MouseButton::Left) && !is_panning {
        let (mx, my) = mouse_position();
        let (wx, wy) = mouse_to_world(mx, my);
        let gx = (wx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (wy / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nutrient_cell(gx, gy);
    }

    if is_mouse_button_pressed(MouseButton::Right) && !is_panning {
        // Right click to add nitrogen cell (in world coordinates)
        let (mx, my) = mouse_position();
        let (wx, wy) = mouse_to_world(mx, my);
        let gx = (wx / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        let gy = (wy / sim.config.cell_size).clamp(0.0, sim.config.grid_size as f32 - 1.0) as usize;
        sim.add_nitrogen_cell(gx, gy);
    }
}

pub fn get_controls_text(camera_enabled: bool) -> Vec<ControlText> {
    vec![
        ControlText {
            text: "Controls: SPACE=Pause | R=Reset | Shift+C=Clear | X=Connections | M=Minimap | H=Hyphae | I=Memory",
            font_size: 16.0,
            color: Color::new(1.0, 1.0, 1.0, 0.7),
        },
        ControlText {
            text: "S=Spawn | N=Sugar patch | T=Nitrogen patch | LMB=Sugar | RMB=Nitrogen",
            font_size: 16.0,
            color: Color::new(1.0, 1.0, 1.0, 0.7),
        },
        ControlText {
            text: "Speed Controls: Shift+<- = Slower | Shift+-> = Faster | 0 = Reset to 1x",
            font_size: 16.0,
            color: Color::new(1.0, 1.0, 1.0, 0.7),
        },
        if camera_enabled {
            ControlText {
                text: "Camera: Arrow Keys/WASD=Pan | Mouse Wheel=Zoom | Middle Mouse/Space+LMB=Drag | Home=Reset | C=Toggle | P=Screenshot",
                font_size: 16.0,
                color: Color::new(1.0, 1.0, 1.0, 0.7),
            }
        } else {
            ControlText {
                text: "Camera: Disabled",
                font_size: 16.0,
                color: Color::new(1.0, 1.0, 1.0, 0.7),
            }
        },
        ControlText {
            text: "Visualization: V=Enhanced | F=Flow | 1=Stress",
            font_size: 16.0,
            color: Color::new(1.0, 1.0, 1.0, 0.7),
        },
        ControlText {
            text: "Network Intelligence: Signals (red pulses) | Strong connections (bright/thick) | Memory (purple overlay)",
            font_size: 14.0,
            color: Color::new(0.8, 0.8, 1.0, 0.6),
        },
        ControlText {
            text: "Weather: Affects growth rate, energy consumption, nutrient diffusion and spore germination | Fusion: Hyphae merge when very close",
            font_size: 14.0,
            color: Color::new(0.8, 1.0, 0.8, 0.6),
        },
    ]
}
