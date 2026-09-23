// Multi-nutrient grid
#[derive(Clone)]
pub struct NutrientGrid {
    pub sugar: Vec<Vec<f32>>,
    pub nitrogen: Vec<Vec<f32>>,
}

impl NutrientGrid {
    pub fn new(grid_size: usize) -> Self {
        Self {
            sugar: vec![vec![0.0f32; grid_size]; grid_size],
            nitrogen: vec![vec![0.0f32; grid_size]; grid_size],
        }
    }

    pub fn size(&self) -> usize {
        self.sugar.len()
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

    pub fn clear(&mut self) {
        for row in &mut self.sugar {
            for cell in row {
                *cell = 0.0;
            }
        }
        for row in &mut self.nitrogen {
            for cell in row {
                *cell = 0.0;
            }
        }
    }

    /// Paint a circular nutrient patch. Amounts are added and clamped to 1.0.
    pub fn add_patch(&mut self, cx: usize, cy: usize, radius: f32, sugar: f32, nitrogen: f32) {
        let size = self.size() as isize;
        let r = radius.ceil() as isize;
        for dx in -r..=r {
            for dy in -r..=r {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > radius {
                    continue;
                }
                let x = cx as isize + dx;
                let y = cy as isize + dy;
                if x < 0 || y < 0 || x >= size || y >= size {
                    continue;
                }
                let falloff = 1.0 - (dist / radius.max(0.001));
                self.add_sugar(x as usize, y as usize, sugar * falloff);
                self.add_nitrogen(x as usize, y as usize, nitrogen * falloff);
            }
        }
    }

    pub fn total_mass(&self) -> f32 {
        let mut total = 0.0;
        for x in 0..self.size() {
            for y in 0..self.size() {
                total += self.sugar[x][y] + self.nitrogen[x][y];
            }
        }
        total
    }

    /// Count cells whose combined nutrient exceeds `threshold`.
    pub fn occupied_cells(&self, threshold: f32) -> usize {
        let mut count = 0;
        for x in 0..self.size() {
            for y in 0..self.size() {
                if self.total_at(x, y) > threshold {
                    count += 1;
                }
            }
        }
        count
    }
}

pub fn nutrient_gradient(grid: &NutrientGrid, x: f32, y: f32, grid_size: usize) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi < 1 || yi < 1 || xi >= grid_size - 1 || yi >= grid_size - 1 {
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
