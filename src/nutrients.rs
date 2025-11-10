use crate::config::GRID_SIZE;
use macroquad::prelude::*;

// Multi-nutrient grid
#[derive(Clone)]
pub struct NutrientGrid {
    pub sugar: Vec<Vec<f32>>,
    pub nitrogen: Vec<Vec<f32>>,
}

impl NutrientGrid {
    pub fn new() -> Self {
        let size = GRID_SIZE;
        Self {
            sugar: vec![vec![0.0f32; size]; size],
            nitrogen: vec![vec![0.0f32; size]; size],
        }
    }

    pub fn total_at(&self, x: usize, y: usize) -> f32 {
        self.sugar[x][y] + self.nitrogen[x][y] * 0.5 // Nitrogen is less energy-dense
    }

    pub fn add_sugar(&mut self, x: usize, y: usize, amount: f32) {
        self.sugar[x][y] = (self.sugar[x][y] + amount).min(1.0);
    }

    pub fn add_nitrogen(&mut self, x: usize, y: usize, amount: f32) {
        self.nitrogen[x][y] = (self.nitrogen[x][y] + amount).min(1.0);
    }
}

pub fn nutrient_color(sugar: f32, nitrogen: f32) -> Color {
    let s = sugar.clamp(0.0, 1.0);
    let n = nitrogen.clamp(0.0, 1.0);
    // Sugar = brown/green, Nitrogen = blue/purple
    // Blend them together
    let r = 0.2 + 0.3 * s + 0.2 * n;
    let g = 0.3 + 0.5 * s + 0.1 * n;
    let b = 0.2 + 0.3 * n;
    Color::new(r, g, b, 1.0)
}

pub fn nutrient_gradient(grid: &NutrientGrid, x: f32, y: f32) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi < 1 || yi < 1 || xi >= GRID_SIZE - 1 || yi >= GRID_SIZE - 1 {
        return (0.0, 0.0);
    }
    // Sobel-like gradient for smoother chemotaxis
    // Combine both nutrient types with weights
    let mut gx = 0.0f32;
    let mut gy = 0.0f32;

    // Sugar gradient (primary)
    let s11 = grid.sugar[xi - 1][yi - 1];
    let s12 = grid.sugar[xi - 1][yi];
    let s13 = grid.sugar[xi - 1][yi + 1];
    let s21 = grid.sugar[xi][yi - 1];
    let s23 = grid.sugar[xi][yi + 1];
    let s31 = grid.sugar[xi + 1][yi - 1];
    let s32 = grid.sugar[xi + 1][yi];
    let s33 = grid.sugar[xi + 1][yi + 1];
    gx += ((s31 + 2.0 * s32 + s33) - (s11 + 2.0 * s12 + s13)) * 1.0;
    gy += ((s13 + 2.0 * s23 + s33) - (s11 + 2.0 * s21 + s31)) * 1.0;

    // Nitrogen gradient (secondary, weaker)
    let n11 = grid.nitrogen[xi - 1][yi - 1];
    let n12 = grid.nitrogen[xi - 1][yi];
    let n13 = grid.nitrogen[xi - 1][yi + 1];
    let n21 = grid.nitrogen[xi][yi - 1];
    let n23 = grid.nitrogen[xi][yi + 1];
    let n31 = grid.nitrogen[xi + 1][yi - 1];
    let n32 = grid.nitrogen[xi + 1][yi];
    let n33 = grid.nitrogen[xi + 1][yi + 1];
    gx += ((n31 + 2.0 * n32 + n33) - (n11 + 2.0 * n12 + n13)) * 0.5;
    gy += ((n13 + 2.0 * n23 + n33) - (n11 + 2.0 * n21 + n31)) * 0.5;

    (gx, gy)
}

// Compute gradient of memory grid (for network intelligence)
pub fn memory_gradient(memory: &[Vec<f32>], x: f32, y: f32, grid_size: usize) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi < 1 || yi < 1 || xi >= grid_size - 1 || yi >= grid_size - 1 {
        return (0.0, 0.0);
    }
    // Sobel-like gradient for memory
    let m11 = memory[xi - 1][yi - 1];
    let m12 = memory[xi - 1][yi];
    let m13 = memory[xi - 1][yi + 1];
    let m21 = memory[xi][yi - 1];
    let m23 = memory[xi][yi + 1];
    let m31 = memory[xi + 1][yi - 1];
    let m32 = memory[xi + 1][yi];
    let m33 = memory[xi + 1][yi + 1];
    let gx = ((m31 + 2.0 * m32 + m33) - (m11 + 2.0 * m12 + m13)) * 0.5;
    let gy = ((m13 + 2.0 * m23 + m33) - (m11 + 2.0 * m21 + m31)) * 0.5;

    (gx, gy)
}
