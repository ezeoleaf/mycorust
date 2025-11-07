use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;

use crate::config::{SimulationConfig, *};
use crate::hypha::Hypha;
use crate::nutrients::{nutrient_gradient, NutrientGrid};
use crate::spore::Spore;
use crate::types::{Connection, FruitBody, Segment};

#[inline]
fn in_bounds(x: f32, y: f32, grid_size: usize) -> bool {
    x >= 0.0 && y >= 0.0 && x < grid_size as f32 && y < grid_size as f32
}

// Simulation state - contains all mutable state data
pub struct SimulationState {
    pub nutrients: NutrientGrid,
    pub obstacles: [[bool; GRID_SIZE]; GRID_SIZE],
    pub hyphae: Vec<Hypha>,
    pub spores: Vec<Spore>,
    pub segments: Vec<Segment>,
    pub connections: Vec<Connection>,
    pub fruit_bodies: Vec<FruitBody>,
    pub fruit_cooldown_timer: f32,
    pub frame_index: u64,
}

impl SimulationState {
    pub fn new() -> Self {
        Self {
            nutrients: NutrientGrid::new(),
            obstacles: [[false; GRID_SIZE]; GRID_SIZE],
            hyphae: Vec::new(),
            spores: Vec::new(),
            segments: Vec::new(),
            connections: Vec::new(),
            fruit_bodies: Vec::new(),
            fruit_cooldown_timer: 0.0,
            frame_index: 0,
        }
    }
}

// Simulation - contains state, config, and control flags
pub struct Simulation {
    pub state: SimulationState,
    pub config: SimulationConfig,
    pub paused: bool,
    pub connections_visible: bool,
    pub minimap_visible: bool,
    pub hyphae_visible: bool,
    pub speed_multiplier: f32,
    pub speed_accumulator: f32,
}

// Implement Deref for convenience - allows sim.nutrients instead of sim.state.nutrients
impl std::ops::Deref for Simulation {
    type Target = SimulationState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl std::ops::DerefMut for Simulation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl Simulation {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        Self::with_config(rng, SimulationConfig::default())
    }

    pub fn with_config<R: Rng>(rng: &mut R, config: SimulationConfig) -> Self {
        let mut state = SimulationState::new();
        let grid_size = config.grid_size;
        let center = grid_size as f32 / 2.0;

        // Initialize nutrients with realistic organic distribution
        Self::initialize_realistic_nutrients(&mut state.nutrients, grid_size, rng);

        // Initialize obstacles
        for _ in 0..config.obstacle_count {
            let x = rng.gen_range(0..grid_size);
            let y = rng.gen_range(0..grid_size);
            state.obstacles[x][y] = true;
        }

        // Initialize hyphae
        state.hyphae = Vec::with_capacity(config.initial_hyphae_count);
        for _ in 0..config.initial_hyphae_count {
            let cx = center + rng.gen_range(-10.0..10.0);
            let cy = center + rng.gen_range(-10.0..10.0);
            state.hyphae.push(Hypha {
                x: cx,
                y: cy,
                prev_x: cx,
                prev_y: cy,
                angle: rng.gen_range(0.0..std::f32::consts::TAU),
                alive: true,
                energy: 0.5,
                parent: None,
                age: 0.0,
            });
        }

        Self {
            state,
            config,
            paused: false,
            connections_visible: true,
            minimap_visible: false,
            hyphae_visible: true,
            speed_multiplier: 1.0,
            speed_accumulator: 0.0,
        }
    }

    /// Initialize nutrients with a realistic organic distribution
    /// Uses multiple organic patches (like decaying matter) with noise-based variation
    fn initialize_realistic_nutrients<R: Rng>(
        nutrients: &mut NutrientGrid,
        grid_size: usize,
        rng: &mut R,
    ) {
        // Simple noise-like function using multiple octaves
        fn simple_noise(x: f32, y: f32, seed: u64) -> f32 {
            let mut value = 0.0;
            let mut amplitude = 1.0;
            let mut frequency = 0.1;
            let mut max_value = 0.0;

            // 4 octaves for natural variation
            for _ in 0..4 {
                let nx = (x * frequency) as i32;
                let ny = (y * frequency) as i32;
                // Simple hash-based noise
                let hash = ((nx.wrapping_mul(73856093) ^ ny.wrapping_mul(19349663)) as u64)
                    .wrapping_mul(seed);
                let noise = ((hash % 1000) as f32 / 1000.0) * 2.0 - 1.0;
                value += noise * amplitude;
                max_value += amplitude;
                amplitude *= 0.5;
                frequency *= 2.0;
            }
            (value / max_value + 1.0) * 0.5 // Normalize to 0..1
        }

        // Create organic patches for sugar (more widespread, like plant matter)
        let sugar_patches = 8 + rng.gen_range(0..5);
        let mut sugar_field = vec![vec![0.0f32; grid_size]; grid_size];

        for _ in 0..sugar_patches {
            let patch_x = rng.gen_range(0.0..grid_size as f32);
            let patch_y = rng.gen_range(0.0..grid_size as f32);
            let patch_radius = rng.gen_range(15.0..40.0);
            let patch_intensity = rng.gen_range(0.4..0.9);
            let seed = rng.gen::<u64>();

            #[allow(clippy::needless_range_loop)]
            for x in 0..grid_size {
                for y in 0..grid_size {
                    let dx = x as f32 - patch_x;
                    let dy = y as f32 - patch_y;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt();
                    let falloff = (1.0 - (dist / patch_radius).min(1.0)).max(0.0);
                    // Smooth falloff with noise variation
                    let noise = simple_noise(x as f32, y as f32, seed);
                    let contribution = falloff * falloff * patch_intensity * (0.7 + 0.3 * noise);
                    sugar_field[x][y] = (sugar_field[x][y] + contribution).min(1.0);
                }
            }
        }

        // Create concentrated patches for nitrogen (rarer, like animal waste or nitrogen-fixing zones)
        let nitrogen_patches = 3 + rng.gen_range(0..4);
        let mut nitrogen_field = vec![vec![0.0f32; grid_size]; grid_size];

        for _ in 0..nitrogen_patches {
            let patch_x = rng.gen_range(0.0..grid_size as f32);
            let patch_y = rng.gen_range(0.0..grid_size as f32);
            let patch_radius = rng.gen_range(8.0..25.0);
            let patch_intensity = rng.gen_range(0.5..1.0);
            let seed = rng.gen::<u64>();

            #[allow(clippy::needless_range_loop)]
            for x in 0..grid_size {
                for y in 0..grid_size {
                    let dx = x as f32 - patch_x;
                    let dy = y as f32 - patch_y;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt();
                    let falloff = (1.0 - (dist / patch_radius).min(1.0)).max(0.0);
                    // Sharper falloff for nitrogen (more concentrated)
                    let noise = simple_noise(x as f32, y as f32, seed);
                    let contribution =
                        falloff * falloff * falloff * patch_intensity * (0.6 + 0.4 * noise);
                    nitrogen_field[x][y] = (nitrogen_field[x][y] + contribution).min(1.0);
                }
            }
        }

        // Add background noise for natural variation
        let background_seed = rng.gen::<u64>();
        for x in 0..grid_size {
            for y in 0..grid_size {
                let noise = simple_noise(x as f32, y as f32, background_seed);
                // Add subtle background variation
                let bg_sugar = (noise - 0.5) * 0.15; // ±15% variation
                let bg_nitrogen = (noise - 0.5) * 0.1; // ±10% variation

                nutrients.sugar[x][y] = (sugar_field[x][y] + bg_sugar).clamp(0.0, 1.0);
                nutrients.nitrogen[x][y] = (nitrogen_field[x][y] + bg_nitrogen).clamp(0.0, 1.0);
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
    pub fn toggle_connections(&mut self) {
        self.connections_visible = !self.connections_visible;
    }
    pub fn toggle_minimap(&mut self) {
        self.minimap_visible = !self.minimap_visible;
    }
    pub fn toggle_hyphae_visibility(&mut self) {
        self.hyphae_visible = !self.hyphae_visible;
    }
    pub fn increase_speed(&mut self) {
        self.speed_multiplier = (self.speed_multiplier * 1.5).min(10.0);
    }
    pub fn decrease_speed(&mut self) {
        self.speed_multiplier = (self.speed_multiplier / 1.5).max(0.1);
    }
    pub fn reset_speed(&mut self) {
        self.speed_multiplier = 1.0;
    }
    pub fn reset<R: Rng>(&mut self, rng: &mut R) {
        self.state.hyphae.clear();
        self.state.spores.clear();
        self.state.segments.clear();
        self.state.connections.clear();
        self.state.fruit_bodies.clear();
        self.state.fruit_cooldown_timer = 0.0;

        // Regenerate nutrients with new realistic distribution
        Self::initialize_realistic_nutrients(&mut self.state.nutrients, self.config.grid_size, rng);

        let cx = self.config.grid_size as f32 / 2.0;
        let cy = self.config.grid_size as f32 / 2.0;
        self.state.hyphae.push(Hypha {
            x: cx,
            y: cy,
            prev_x: cx,
            prev_y: cy,
            angle: rng.gen_range(0.0..std::f32::consts::TAU),
            alive: true,
            energy: 0.5,
            parent: None,
            age: 0.0,
        });
    }
    pub fn clear_segments(&mut self) {
        self.state.segments.clear();
    }
    pub fn spawn_hypha_at<R: Rng>(&mut self, rng: &mut R, gx: f32, gy: f32) {
        self.state.hyphae.push(Hypha {
            x: gx,
            y: gy,
            prev_x: gx,
            prev_y: gy,
            angle: rng.gen_range(0.0..std::f32::consts::TAU),
            alive: true,
            energy: 0.5,
            parent: None,
            age: 0.0,
        });
    }
    pub fn add_nutrient_patch(&mut self, gx: usize, gy: usize) {
        let grid_size = self.config.grid_size;
        for dx in -3..=3 {
            for dy in -3..=3 {
                let nx = (gx as i32 + dx).max(0).min(grid_size as i32 - 1) as usize;
                let ny = (gy as i32 + dy).max(0).min(grid_size as i32 - 1) as usize;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 3.0 {
                    self.state.nutrients.add_sugar(nx, ny, 1.0);
                }
            }
        }
    }
    pub fn add_nitrogen_patch(&mut self, gx: usize, gy: usize) {
        let grid_size = self.config.grid_size;
        for dx in -3..=3 {
            for dy in -3..=3 {
                let nx = (gx as i32 + dx).max(0).min(grid_size as i32 - 1) as usize;
                let ny = (gy as i32 + dy).max(0).min(grid_size as i32 - 1) as usize;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 3.0 {
                    self.state.nutrients.add_nitrogen(nx, ny, 1.0);
                }
            }
        }
    }
    pub fn add_nutrient_cell(&mut self, gx: usize, gy: usize) {
        self.state.nutrients.add_sugar(gx, gy, 1.0);
    }
    pub fn add_nitrogen_cell(&mut self, gx: usize, gy: usize) {
        self.state.nutrients.add_nitrogen(gx, gy, 1.0);
    }

    pub fn stats(&self) -> (usize, usize, usize, usize, f32, f32) {
        let alive_hyphae: Vec<_> = self.state.hyphae.iter().filter(|h| h.alive).collect();
        let hyphae_count = alive_hyphae.len();
        let total_energy: f32 = alive_hyphae.iter().map(|h| h.energy).sum();
        let avg_energy = if hyphae_count > 0 {
            total_energy / hyphae_count as f32
        } else {
            0.0
        };
        let spores_count = self.state.spores.iter().filter(|s| s.alive).count();
        let connections_count = self.state.connections.len();
        (
            hyphae_count,
            spores_count,
            connections_count,
            self.state.fruit_bodies.len(),
            avg_energy,
            total_energy,
        )
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) {
        self.state.frame_index = self.state.frame_index.wrapping_add(1);

        // Age segments
        for segment in &mut self.state.segments {
            segment.age += self.config.segment_age_increment;
        }
        self.state
            .segments
            .retain(|s| s.age < self.config.max_segment_age);

        let mut new_hyphae = vec![];
        let mut energy_transfers: Vec<(usize, usize, f32)> = Vec::new();
        let hyphae_len = self.state.hyphae.len();

        let hyphae_info: Vec<(f32, f32, bool, f32)> = self
            .state
            .hyphae
            .iter()
            .map(|h| (h.x, h.y, h.alive, h.energy))
            .collect();

        // Spatial hash grid for neighbor queries
        let cell_size: f32 = 4.0;
        let grid_size = self.config.grid_size;
        let nx = ((grid_size as f32) / cell_size).ceil() as usize;
        let ny = ((grid_size as f32) / cell_size).ceil() as usize;
        let mut buckets: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); ny]; nx];
        for (i, (x, y, alive, _)) in hyphae_info.iter().enumerate() {
            if !*alive {
                continue;
            }
            let bx = (*x / cell_size).floor() as isize;
            let by = (*y / cell_size).floor() as isize;
            if bx >= 0 && by >= 0 {
                let bxu = bx as usize;
                let byu = by as usize;
                if bxu < nx && byu < ny {
                    buckets[bxu][byu].push(i);
                }
            }
        }

        for (idx, h) in self.state.hyphae[..hyphae_len].iter_mut().enumerate() {
            if !h.alive {
                continue;
            }

            h.prev_x = h.x;
            h.prev_y = h.y;

            let (mut gx, mut gy) = nutrient_gradient(&self.state.nutrients, h.x, h.y);
            let grad_mag = (gx * gx + gy * gy).sqrt();
            // Only apply gradient steering if gradient is significant (avoid noise from small gradients)
            const MIN_GRADIENT_MAG: f32 = 0.08; // Threshold to ignore numerical noise (increased to avoid bias)
            if grad_mag > MIN_GRADIENT_MAG {
                // Add tropism global bias only when gradient is weak (subtle drift)
                if grad_mag < 0.15 {
                    let tx = self.config.tropism_angle.cos() * self.config.tropism_strength;
                    let ty = self.config.tropism_angle.sin() * self.config.tropism_strength;
                    gx += tx;
                    gy += ty;
                    let new_grad_mag = (gx * gx + gy * gy).sqrt();
                    if new_grad_mag > MIN_GRADIENT_MAG {
                        let grad_angle = gy.atan2(gx);
                        h.angle += (grad_angle - h.angle) * self.config.gradient_steering_strength;
                    }
                } else {
                    let grad_angle = gy.atan2(gx);
                    h.angle += (grad_angle - h.angle) * self.config.gradient_steering_strength;
                }
            }
            // Apply random wander - this should be symmetric, but increase it slightly when gradient is weak
            let wander_boost = if grad_mag < MIN_GRADIENT_MAG {
                1.5
            } else {
                1.0
            };
            h.angle += rng
                .gen_range(-self.config.angle_wander_range..self.config.angle_wander_range)
                * wander_boost;

            let mut neighbor_count = 0.0f32;
            let bx = (h.x / cell_size).floor() as isize;
            let by = (h.y / cell_size).floor() as isize;
            for gx in (bx - 1)..=(bx + 1) {
                for gy in (by - 1)..=(by + 1) {
                    if gx < 0 || gy < 0 {
                        continue;
                    }
                    let gux = gx as usize;
                    let guy = gy as usize;
                    if gux >= nx || guy >= ny {
                        continue;
                    }
                    for &other_idx in &buckets[gux][guy] {
                        if other_idx == idx {
                            continue;
                        }
                        let (ox, oy, other_alive, _) = hyphae_info[other_idx];
                        if !other_alive {
                            continue;
                        }
                        let dx = h.x - ox;
                        let dy = h.y - oy;
                        if dx * dx + dy * dy < self.config.hyphae_avoidance_distance_sq() * 4.0 {
                            neighbor_count += 1.0;
                        }
                    }
                }
            }
            let density_slow = 1.0 / (1.0 + 0.05 * neighbor_count);

            let new_x = h.x + h.angle.cos() * self.config.step_size * density_slow;
            let new_y = h.y + h.angle.sin() * self.config.step_size * density_slow;
            let mut too_close = false;
            for gx in (bx - 1)..=(bx + 1) {
                for gy in (by - 1)..=(by + 1) {
                    if gx < 0 || gy < 0 {
                        continue;
                    }
                    let gux = gx as usize;
                    let guy = gy as usize;
                    if gux >= nx || guy >= ny {
                        continue;
                    }
                    for &other_idx in &buckets[gux][guy] {
                        if other_idx == idx {
                            continue;
                        }
                        let (ox, oy, other_alive, _) = hyphae_info[other_idx];
                        if !other_alive {
                            continue;
                        }
                        let dx = new_x - ox;
                        let dy = new_y - oy;
                        let dist2 = dx * dx + dy * dy;
                        if dist2 < self.config.hyphae_avoidance_distance_sq() && dist2 > 0.001 {
                            too_close = true;
                            break;
                        }
                    }
                    if too_close {
                        break;
                    }
                }
                if too_close {
                    break;
                }
            }
            if too_close {
                h.angle += rng.gen_range(-0.5..0.5);
            }

            h.x += h.angle.cos() * self.config.step_size * density_slow;
            h.y += h.angle.sin() * self.config.step_size * density_slow;

            let xi = h.x as usize;
            let yi = h.y as usize;
            if in_bounds(h.x, h.y, self.config.grid_size) && self.state.obstacles[xi][yi] {
                h.x = h.prev_x;
                h.y = h.prev_y;
                let mut found_clear = false;
                let mut best_angle = h.angle;
                let mut attempts = 0;
                while !found_clear && attempts < 8 {
                    let test_angle = h.angle + (attempts as f32) * std::f32::consts::PI / 4.0;
                    let test_x = h.x + test_angle.cos() * self.config.step_size;
                    let test_y = h.y + test_angle.sin() * self.config.step_size;
                    let test_xi = test_x as usize;
                    let test_yi = test_y as usize;
                    if in_bounds(test_x, test_y, self.config.grid_size)
                        && !self.state.obstacles[test_xi][test_yi]
                    {
                        best_angle = test_angle;
                        found_clear = true;
                    }
                    attempts += 1;
                }
                if found_clear {
                    h.angle = best_angle + rng.gen_range(-0.2..0.2);
                } else {
                    h.angle += std::f32::consts::PI + rng.gen_range(-0.5..0.5);
                }
                h.angle %= std::f32::consts::TAU;
                if h.angle < 0.0 {
                    h.angle += std::f32::consts::TAU;
                }
                h.x += h.angle.cos() * self.config.step_size;
                h.y += h.angle.sin() * self.config.step_size;
            }

            if h.x < 1.0
                || h.x >= self.config.grid_size as f32 - 1.0
                || h.y < 1.0
                || h.y >= self.config.grid_size as f32 - 1.0
            {
                h.x = h.prev_x;
                h.y = h.prev_y;
                let min_b = 1.0;
                let max_b = self.config.grid_size as f32 - 2.0;
                if h.x <= min_b {
                    h.x = min_b;
                    h.angle = std::f32::consts::PI - h.angle;
                } else if h.x >= max_b {
                    h.x = max_b;
                    h.angle = std::f32::consts::PI - h.angle;
                }
                if h.y <= min_b {
                    h.y = min_b;
                    h.angle = -h.angle;
                } else if h.y >= max_b {
                    h.y = max_b;
                    h.angle = -h.angle;
                }
                h.angle += rng.gen_range(-0.15..0.15);
                h.x += h.angle.cos() * self.config.step_size;
                h.y += h.angle.sin() * self.config.step_size;
                h.x = h.x.clamp(min_b, max_b);
                h.y = h.y.clamp(min_b, max_b);
            }

            let xi = h.x as usize;
            let yi = h.y as usize;
            // Consume both sugar (primary) and nitrogen (secondary)
            let sugar = self.state.nutrients.sugar[xi][yi];
            let nitrogen = self.state.nutrients.nitrogen[xi][yi];
            let total_nutrient = sugar + nitrogen * 0.5; // Nitrogen is less energy-dense
            if total_nutrient > 0.001 {
                let absorbed = total_nutrient.min(self.config.nutrient_decay);
                h.energy = (h.energy + absorbed).min(1.0);
                // Consume proportionally from both types
                if sugar > 0.0 {
                    let sugar_absorb = (absorbed * sugar / total_nutrient).min(sugar);
                    self.state.nutrients.sugar[xi][yi] -= sugar_absorb;
                }
                if nitrogen > 0.0 {
                    let nitrogen_absorb =
                        (absorbed * nitrogen * 0.5 / total_nutrient).min(nitrogen);
                    self.state.nutrients.nitrogen[xi][yi] -= nitrogen_absorb;
                }
            }

            h.energy *= self.config.energy_decay_rate;
            h.age += 0.01;
            if h.energy < self.config.min_energy_to_live {
                h.alive = false;
                continue;
            }

            if let Some(parent_idx) = h.parent {
                if parent_idx < hyphae_len {
                    let (px, py, parent_alive, parent_energy) = hyphae_info[parent_idx];
                    if parent_alive {
                        let dx = h.x - px;
                        let dy = h.y - py;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let max_dist = 6.0f32;
                        if dist < max_dist {
                            let transfer_rate = 0.002 * (1.0 - dist / max_dist).max(0.0);
                            let wanted = (h.energy - parent_energy) * 0.5;
                            let transfer = (wanted * transfer_rate).clamp(-0.01, 0.01);
                            if transfer.abs() > 0.0 {
                                energy_transfers.push((idx, parent_idx, transfer));
                            }
                        }
                    }
                }
            }

            if total_nutrient < 0.05 && rng.gen_bool(0.001) {
                self.state.spores.push(Spore {
                    x: h.x,
                    y: h.y,
                    vx: rng.gen_range(-0.5..0.5),
                    vy: rng.gen_range(-0.5..0.5),
                    alive: true,
                    age: 0.0,
                });
            }

            let age_branch_boost = (1.0 + h.age * 0.05).min(2.0);
            if rng.gen::<f32>() < self.config.branch_prob * age_branch_boost {
                let idxp = hyphae_len;
                new_hyphae.push(Hypha {
                    x: h.x,
                    y: h.y,
                    prev_x: h.x,
                    prev_y: h.y,
                    angle: h.angle + rng.gen_range(-1.2..1.2),
                    alive: true,
                    energy: h.energy * 0.5,
                    parent: Some(idxp),
                    age: 0.0,
                });
                h.energy *= 0.5;
            }

            let from = vec2(
                h.prev_x * self.config.cell_size,
                h.prev_y * self.config.cell_size,
            );
            let to = vec2(h.x * self.config.cell_size, h.y * self.config.cell_size);
            self.state.segments.push(Segment { from, to, age: 0.0 });
        }

        for (from, to, amount) in energy_transfers {
            if from < self.state.hyphae.len() && to < self.state.hyphae.len() {
                self.state.hyphae[from].energy =
                    (self.state.hyphae[from].energy - amount).clamp(0.0, 1.0);
                self.state.hyphae[to].energy =
                    (self.state.hyphae[to].energy + amount).clamp(0.0, 1.0);
            }
        }

        self.state.hyphae.extend(new_hyphae);

        // connections
        for i in 0..self.state.hyphae.len() {
            for j in (i + 1)..self.state.hyphae.len() {
                if !self.state.hyphae[i].alive || !self.state.hyphae[j].alive {
                    continue;
                }
                let dx = self.state.hyphae[i].x - self.state.hyphae[j].x;
                let dy = self.state.hyphae[i].y - self.state.hyphae[j].y;
                let dist2 = dx * dx + dy * dy;
                if dist2 < self.config.anastomosis_distance_sq() {
                    let exists = self.state.connections.iter().any(|c| {
                        (c.hypha1 == i && c.hypha2 == j) || (c.hypha1 == j && c.hypha2 == i)
                    });
                    if !exists {
                        self.state.connections.push(Connection {
                            hypha1: i,
                            hypha2: j,
                        });
                        let energy_diff = self.state.hyphae[i].energy - self.state.hyphae[j].energy;
                        if energy_diff.abs() > 0.1 {
                            let transfer = energy_diff * 0.1;
                            self.state.hyphae[i].energy -= transfer;
                            self.state.hyphae[j].energy += transfer;
                            self.state.hyphae[i].energy =
                                self.state.hyphae[i].energy.clamp(0.0, 1.0);
                            self.state.hyphae[j].energy =
                                self.state.hyphae[j].energy.clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }
        self.state.connections.retain(|c| {
            self.state
                .hyphae
                .get(c.hypha1)
                .map(|h| h.alive)
                .unwrap_or(false)
                && self
                    .state
                    .hyphae
                    .get(c.hypha2)
                    .map(|h| h.alive)
                    .unwrap_or(false)
        });

        // Resource allocation along connections (diffusive flow)
        for c in &self.state.connections {
            let (i, j) = if c.hypha1 <= c.hypha2 {
                (c.hypha1, c.hypha2)
            } else {
                (c.hypha2, c.hypha1)
            };
            if j >= self.state.hyphae.len() {
                continue;
            }
            let (left, right) = self.state.hyphae.split_at_mut(j);
            let h1 = &mut left[i];
            let h2 = &mut right[0];
            if !h1.alive || !h2.alive {
                continue;
            }
            let diff = h1.energy - h2.energy;
            let flow = (diff * self.config.connection_flow_rate).clamp(-0.02, 0.02);
            h1.energy = (h1.energy - flow).clamp(0.0, 1.0);
            h2.energy = (h2.energy + flow).clamp(0.0, 1.0);
        }

        // diffuse nutrients (LOD: bounding box + frame skipping)
        let do_diffuse = if get_fps() < 45 {
            (self.state.frame_index % 2) == 0
        } else {
            true
        };
        if do_diffuse {
            // Compute bounding box around alive hyphae
            let grid_size = self.config.grid_size;
            let mut minx = grid_size - 2;
            let mut miny = grid_size - 2;
            let mut maxx = 1;
            let mut maxy = 1;
            for h in self.state.hyphae.iter().filter(|h| h.alive) {
                let xi = h.x as usize;
                let yi = h.y as usize;
                if xi > 0 && yi > 0 && xi < grid_size - 1 && yi < grid_size - 1 {
                    if xi < minx {
                        minx = xi;
                    }
                    if yi < miny {
                        miny = yi;
                    }
                    if xi > maxx {
                        maxx = xi;
                    }
                    if yi > maxy {
                        maxy = yi;
                    }
                }
            }
            let pad = 6usize;
            let x0 = 1.max(minx.saturating_sub(pad));
            let y0 = 1.max(miny.saturating_sub(pad));
            let x1 = (grid_size - 2).min(maxx.saturating_add(pad));
            let y1 = (grid_size - 2).min(maxy.saturating_add(pad));

            let mut diffused = self.state.nutrients.clone();
            // Diffuse both sugar and nitrogen
            for x in x0..=x1 {
                for y in y0..=y1 {
                    // Sugar diffusion
                    let avg_sugar = (self.state.nutrients.sugar[x + 1][y]
                        + self.state.nutrients.sugar[x - 1][y]
                        + self.state.nutrients.sugar[x][y + 1]
                        + self.state.nutrients.sugar[x][y - 1])
                        * 0.25;
                    diffused.sugar[x][y] +=
                        self.config.diffusion_rate * (avg_sugar - self.state.nutrients.sugar[x][y]);

                    // Nitrogen diffusion (slower)
                    let avg_nitrogen = (self.state.nutrients.nitrogen[x + 1][y]
                        + self.state.nutrients.nitrogen[x - 1][y]
                        + self.state.nutrients.nitrogen[x][y + 1]
                        + self.state.nutrients.nitrogen[x][y - 1])
                        * 0.25;
                    diffused.nitrogen[x][y] += self.config.diffusion_rate
                        * 0.7
                        * (avg_nitrogen - self.state.nutrients.nitrogen[x][y]);
                }
            }
            self.state.nutrients = diffused;
        }

        // spores
        let mut new_hyphae_from_spores = vec![];
        for spore in &mut self.state.spores {
            if !spore.alive {
                continue;
            }
            spore.x += spore.vx;
            spore.y += spore.vy;
            spore.age += 0.01;
            spore.vx += rng.gen_range(-0.02..0.02);
            spore.vy += rng.gen_range(-0.02..0.02);
            if spore.x < 1.0
                || spore.x >= self.config.grid_size as f32 - 1.0
                || spore.y < 1.0
                || spore.y >= self.config.grid_size as f32 - 1.0
            {
                spore.alive = false;
                continue;
            }
            let xi = spore.x as usize;
            let yi = spore.y as usize;
            let total_nutrient = self.state.nutrients.total_at(xi, yi);
            if total_nutrient > self.config.spore_germination_threshold {
                new_hyphae_from_spores.push(Hypha {
                    x: spore.x,
                    y: spore.y,
                    prev_x: spore.x,
                    prev_y: spore.y,
                    angle: rng.gen_range(0.0..std::f32::consts::TAU),
                    alive: true,
                    energy: 0.5,
                    parent: None,
                    age: 0.0,
                });
                spore.alive = false;
                // Particle burst at germination
                for k in 0..8 {
                    let a = (k as f32 / 8.0) * std::f32::consts::TAU + rng.gen_range(-0.2..0.2);
                    let r = rng.gen_range(2.0..5.0);
                    let px = spore.x * self.config.cell_size + a.cos() * r;
                    let py = spore.y * self.config.cell_size + a.sin() * r;
                    draw_circle(px, py, 1.5, Color::new(1.0, 0.8, 0.3, 0.6));
                }
            }
        }
        self.state.hyphae.extend(new_hyphae_from_spores);
        self.state
            .spores
            .retain(|s| s.alive && s.age < self.config.spore_max_age);

        // fruiting
        let (hyphae_count, _spores_count, _conn_count, _fruit_count, _avg_energy, total_energy) =
            self.stats();
        let fps = get_fps();
        self.state.fruit_cooldown_timer =
            (self.state.fruit_cooldown_timer - 1.0 / fps.max(1) as f32).max(0.0);
        if self.state.fruit_cooldown_timer <= 0.0
            && hyphae_count >= self.config.fruiting_min_hyphae
            && total_energy >= self.config.fruiting_threshold_total_energy
        {
            let alive_hyphae: Vec<_> = self.state.hyphae.iter().filter(|h| h.alive).collect();
            let mut cx = 0.0f32;
            let mut cy = 0.0f32;
            for h in &alive_hyphae {
                cx += h.x * h.energy;
                cy += h.y * h.energy;
            }
            if total_energy > 0.0 {
                cx /= total_energy;
                cy /= total_energy;
            } else if let Some(first) = alive_hyphae.first() {
                cx = first.x;
                cy = first.y;
            }
            self.state.fruit_bodies.push(FruitBody {
                x: cx,
                y: cy,
                age: 0.0,
                energy: 0.0,
            });
            self.state.fruit_cooldown_timer = self.config.fruiting_cooldown;
        }

        // Energy transfer from hyphae to fruiting bodies
        for f in &mut self.state.fruit_bodies {
            f.age += 0.01;
            let mut total_transfer = 0.0f32;
            let transfer_radius = 15.0f32;
            let transfer_radius_sq = transfer_radius * transfer_radius;

            for h in &mut self.state.hyphae {
                if !h.alive || h.energy < 0.1 {
                    continue;
                }
                let dx = f.x - h.x;
                let dy = f.y - h.y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < transfer_radius_sq && dist_sq > 0.1 {
                    let dist = dist_sq.sqrt();
                    let transfer_rate = 0.01 * (1.0 - dist / transfer_radius).max(0.0);
                    let transfer = (h.energy * transfer_rate).min(0.05);
                    if transfer > 0.001 {
                        h.energy -= transfer;
                        total_transfer += transfer;
                    }
                }
            }
            f.energy = (f.energy + total_transfer).min(1.0);
        }
    }
}
