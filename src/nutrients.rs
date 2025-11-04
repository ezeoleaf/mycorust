use macroquad::prelude::*;
use crate::config::GRID_SIZE;

pub fn nutrient_color(value: f32) -> Color {
    let v = value.clamp(0.0, 1.0);
    Color::new(0.2 + 0.3 * v, 0.3 + 0.5 * v, 0.2, 1.0)
}

pub fn nutrient_gradient(grid: &[[f32; GRID_SIZE]; GRID_SIZE], x: f32, y: f32) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi == 0 || yi == 0 || xi >= GRID_SIZE - 1 || yi >= GRID_SIZE - 1 {
        return (0.0, 0.0);
    }
    let dx = grid[xi + 1][yi] - grid[xi - 1][yi];
    let dy = grid[xi][yi + 1] - grid[xi][yi - 1];
    (dx, dy)
}


