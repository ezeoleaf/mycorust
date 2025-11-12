// Camera system for pan and zoom functionality

#[cfg(not(test))]
use macroquad::prelude::*;

#[cfg(test)]
use crate::types::Vec2;

/// Camera state for pan and zoom
pub struct Camera {
    pub enabled: bool, // Whether camera pan/zoom is enabled
    pub x: f32,        // Camera position X (in world coordinates)
    pub y: f32,        // Camera position Y (in world coordinates)
    pub zoom: f32,     // Zoom level (1.0 = normal, >1.0 = zoomed in, <1.0 = zoomed out)
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub pan_speed: f32,
    pub zoom_speed: f32,
    // For smooth panning with mouse drag
    pub last_mouse_pos: Option<Vec2>,
    pub is_panning: bool,
}

impl Camera {
    /// Calculate minimum zoom level to show entire grid filling the screen
    /// Returns the zoom level where the grid exactly fills the viewport
    fn calculate_min_zoom() -> f32 {
        // With our zoom formula: zoom.x = 2.0 * zoom_level / grid_width
        // At zoom_level = 1.0: zoom.x = 2.0 / grid_width
        // This shows exactly grid_width units in the viewport
        // Since grid_width = window_width (800 = 800), this should fill the screen
        // Minimum zoom is 1.0
        1.0
    }

    pub fn new(enabled: bool) -> Self {
        // Center camera on the grid
        // Grid goes from (0, 0) to (GRID_SIZE * CELL_SIZE, GRID_SIZE * CELL_SIZE)
        // So center is at (GRID_SIZE * CELL_SIZE / 2, GRID_SIZE * CELL_SIZE / 2)
        use crate::config::{CELL_SIZE, GRID_SIZE};
        let grid_center_x = (GRID_SIZE as f32 * CELL_SIZE) / 2.0;
        let grid_center_y = (GRID_SIZE as f32 * CELL_SIZE) / 2.0;

        // Calculate minimum zoom to show entire grid filling the screen
        let min_zoom = Self::calculate_min_zoom();

        Self {
            enabled,
            x: grid_center_x,
            y: grid_center_y,
            zoom: min_zoom, // Start at minimum zoom (grid fills screen)
            min_zoom,
            max_zoom: 5.0, // Allow zooming in up to 5x
            pan_speed: 5.0,
            zoom_speed: 0.1,
            last_mouse_pos: None,
            is_panning: false,
        }
    }

    /// Toggle camera enabled state
    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Reset camera to default position and zoom (centered on grid, grid filling screen)
    pub fn reset(&mut self) {
        use crate::config::{CELL_SIZE, GRID_SIZE};
        let grid_center_x = (GRID_SIZE as f32 * CELL_SIZE) / 2.0;
        let grid_center_y = (GRID_SIZE as f32 * CELL_SIZE) / 2.0;
        self.x = grid_center_x;
        self.y = grid_center_y;
        // Update min_zoom in case screen size changed, then set zoom to minimum
        self.min_zoom = Self::calculate_min_zoom();
        self.zoom = self.min_zoom;
    }

    /// Get the camera transform matrix
    /// Returns a default camera if disabled (no transform)
    /// Note: In tests, this returns a default camera (tests don't use camera)
    pub fn get_camera(&self) -> macroquad::prelude::Camera2D {
        // If camera is disabled, return default camera (no transform)
        if !self.enabled {
            return macroquad::prelude::Camera2D::default();
        }

        #[cfg(not(test))]
        {
            use crate::config::{CELL_SIZE, GRID_SIZE};
            use macroquad::math::Rect;
            let grid_width = GRID_SIZE as f32 * CELL_SIZE;
            let grid_height = GRID_SIZE as f32 * CELL_SIZE;

            // Calculate viewport size in world units based on zoom level
            let viewport_width = grid_width / self.zoom;
            let viewport_height = grid_height / self.zoom;

            // Use from_display_rect to ensure correct zoom calculation
            let viewport_rect = Rect {
                x: self.x - viewport_width / 2.0,
                y: self.y - viewport_height / 2.0,
                w: viewport_width,
                h: viewport_height,
            };

            // from_display_rect creates a camera that shows exactly rect.w units horizontally
            // This ensures the grid fills the screen at minimum zoom
            macroquad::prelude::Camera2D::from_display_rect(viewport_rect)
        }

        #[cfg(test)]
        {
            // In tests, return default camera (tests don't actually use camera, but main.rs needs to compile)
            macroquad::prelude::Camera2D::default()
        }
    }

    /// Convert screen coordinates to world coordinates
    /// Uses macroquad's camera system to get the world position
    /// Note: In tests, this returns the input unchanged (no transformation)
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        #[cfg(not(test))]
        {
            // Use macroquad's camera to convert screen to world coordinates
            let camera = self.get_camera();
            camera.screen_to_world(screen_pos)
        }

        #[cfg(test)]
        {
            // Test stub - just return the screen position (no transformation in tests)
            screen_pos
        }
    }

    /// Convert world coordinates to screen coordinates
    /// Uses macroquad's camera system to get the screen position
    // pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
    //     // Use macroquad's camera to convert world to screen coordinates
    //     let camera = self.get_camera();
    //     let screen_pos = camera.world_to_screen(world_pos);
    //     screen_pos
    // }

    /// Pan the camera by delta (in world coordinates)
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    /// Zoom the camera at a specific point (in screen coordinates)
    #[cfg(not(test))]
    pub fn zoom_at(&mut self, screen_x: f32, screen_y: f32, zoom_delta: f32) {
        // Update min_zoom in case screen size changed
        self.min_zoom = Self::calculate_min_zoom();

        // Convert screen point to world coordinates before zoom
        let world_before = self.screen_to_world(macroquad::prelude::vec2(screen_x, screen_y));

        // Apply zoom, but don't allow zooming below minimum
        // Only allow zooming in (positive delta) if we're at minimum, or both directions otherwise
        let new_zoom = if self.zoom <= self.min_zoom + 0.001 && zoom_delta < 0.0 {
            // At minimum zoom, don't allow zooming out
            self.zoom
        } else {
            (self.zoom + zoom_delta).clamp(self.min_zoom, self.max_zoom)
        };
        self.zoom = new_zoom;

        // Convert screen point to world coordinates after zoom
        let world_after = self.screen_to_world(macroquad::prelude::vec2(screen_x, screen_y));

        // Adjust camera position to keep the point under the cursor fixed
        self.x += world_before.x - world_after.x;
        self.y += world_before.y - world_after.y;
    }

    #[cfg(test)]
    pub fn zoom_at(&mut self, _screen_x: f32, _screen_y: f32, zoom_delta: f32) {
        // Test stub - just update zoom
        self.min_zoom = Self::calculate_min_zoom();
        self.zoom = (self.zoom + zoom_delta).clamp(self.min_zoom, self.max_zoom);
    }

    /// Update camera based on input
    /// Only processes input if camera is enabled
    #[cfg(not(test))]
    pub fn update(&mut self) {
        // Don't process input if camera is disabled
        if !self.enabled {
            return;
        }

        // Keyboard panning (arrow keys or WASD, but not when Shift is held for speed control)
        let shift_held = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        if !shift_held {
            let pan_speed = self.pan_speed / self.zoom; // Pan speed scales with zoom
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                self.pan(-pan_speed, 0.0);
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                self.pan(pan_speed, 0.0);
            }
            if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                self.pan(0.0, -pan_speed);
            }
            if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                self.pan(0.0, pan_speed);
            }
        }

        // Mouse wheel zoom
        let mouse_wheel = mouse_wheel().1;
        if mouse_wheel.abs() > 0.0 {
            let mouse_pos = mouse_position();
            let zoom_delta = mouse_wheel * self.zoom_speed * self.zoom;
            self.zoom_at(mouse_pos.0, mouse_pos.1, zoom_delta);
        }

        // Keyboard zoom (+/- keys)
        if is_key_pressed(KeyCode::Equal) {
            let mouse_pos = mouse_position();
            self.zoom_at(mouse_pos.0, mouse_pos.1, self.zoom_speed * self.zoom);
        }
        if is_key_pressed(KeyCode::Minus) {
            let mouse_pos = mouse_position();
            self.zoom_at(mouse_pos.0, mouse_pos.1, -self.zoom_speed * self.zoom);
        }

        // Mouse drag panning (middle mouse button or space + left click)
        // Note: Space+Left is handled here, but we need to avoid conflicts with pause
        // We check for space being held (not just pressed) to distinguish from pause toggle
        let middle_mouse = is_mouse_button_down(MouseButton::Middle);
        let space_held = is_key_down(KeyCode::Space);
        let left_mouse_down = is_mouse_button_down(MouseButton::Left);
        let space_left = space_held && left_mouse_down && !is_key_pressed(KeyCode::Space);

        if middle_mouse || space_left {
            let mouse_pos = macroquad::prelude::vec2(mouse_position().0, mouse_position().1);

            if let Some(last_pos) = self.last_mouse_pos {
                if !self.is_panning {
                    self.is_panning = true;
                }
                // Pan based on mouse movement (inverse because we're moving the camera)
                // Convert screen delta to world delta using camera
                let world_before = self.screen_to_world(last_pos);
                let world_after = self.screen_to_world(mouse_pos);
                let dx = world_before.x - world_after.x;
                let dy = world_before.y - world_after.y;
                self.pan(dx, dy);
            }
            self.last_mouse_pos = Some(mouse_pos);
        } else {
            self.is_panning = false;
            self.last_mouse_pos = None;
        }

        // Reset camera (Home key)
        if is_key_pressed(KeyCode::Home) {
            self.reset();
        }
    }

    #[cfg(test)]
    pub fn update(&mut self) {
        // Test stub - camera update not needed in tests
    }
}
