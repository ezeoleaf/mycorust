use macroquad::prelude::*;
use crate::config::GRID_SIZE;

pub fn nutrient_color(value: f32) -> Color {
    let v = value.clamp(0.0, 1.0);
    Color::new(0.2 + 0.3 * v, 0.3 + 0.5 * v, 0.2, 1.0)
}

pub fn nutrient_gradient(grid: &[[f32; GRID_SIZE]; GRID_SIZE], x: f32, y: f32) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi < 1 || yi < 1 || xi >= GRID_SIZE - 1 || yi >= GRID_SIZE - 1 {
        return (0.0, 0.0);
    }
    // Sobel-like gradient for smoother chemotaxis
    let a11 = grid[xi - 1][yi - 1];
    let a12 = grid[xi - 1][yi];
    let a13 = grid[xi - 1][yi + 1];
    let a21 = grid[xi][yi - 1];
    let a23 = grid[xi][yi + 1];
    let a31 = grid[xi + 1][yi - 1];
    let a32 = grid[xi + 1][yi];
    let a33 = grid[xi + 1][yi + 1];
    let gx = (a31 + 2.0 * a32 + a33) - (a11 + 2.0 * a12 + a13);
    let gy = (a13 + 2.0 * a23 + a33) - (a11 + 2.0 * a21 + a31);
    (gx, gy)
}


