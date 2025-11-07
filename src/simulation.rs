use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;
use std::collections::HashSet;

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
    pub nutrients_back: NutrientGrid, // Double buffer for diffusion
    pub obstacles: Vec<Vec<bool>>,
    pub hyphae: Vec<Hypha>,
    pub spores: Vec<Spore>,
    pub segments: Vec<Segment>,
    pub connections: Vec<Connection>,
    pub connection_set: HashSet<(usize, usize)>, // Fast lookup for connections
    pub fruit_bodies: Vec<FruitBody>,
    pub fruit_cooldown_timer: f32,
    pub fruiting_failed_attempts: u32,
    pub frame_index: u64,
    // Reusable spatial hash grid to avoid allocations
    pub spatial_grid: Vec<Vec<Vec<usize>>>,
    pub spatial_grid_nx: usize,
    pub spatial_grid_ny: usize,
}

impl SimulationState {
    pub fn new() -> Self {
        // Pre-allocate spatial grid
        let cell_size: f32 = 4.0;
        let grid_size = GRID_SIZE;
        let nx = ((grid_size as f32) / cell_size).ceil() as usize;
        let ny = ((grid_size as f32) / cell_size).ceil() as usize;
        let spatial_grid = vec![vec![Vec::new(); ny]; nx];

        Self {
            nutrients: NutrientGrid::new(),
            nutrients_back: NutrientGrid::new(),
            obstacles: vec![vec![false; GRID_SIZE]; GRID_SIZE],
            hyphae: Vec::new(),
            spores: Vec::new(),
            segments: Vec::new(),
            connections: Vec::new(),
            connection_set: HashSet::new(),
            fruit_bodies: Vec::new(),
            fruit_cooldown_timer: 0.0,
            fruiting_failed_attempts: 0,
            frame_index: 0,
            spatial_grid,
            spatial_grid_nx: nx,
            spatial_grid_ny: ny,
        }
    }
}

// Editor tool types
#[derive(Clone, Copy, PartialEq)]
pub enum EditorTool {
    Sugar,
    Nitrogen,
    Erase,
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
    // Editor mode
    pub editor_mode: bool,
    pub editor_tool: EditorTool,
    pub editor_brush_size: usize,
    pub editor_last_draw_pos: Option<(usize, usize)>,
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
            editor_mode: false,
            editor_tool: EditorTool::Sugar,
            editor_brush_size: 3,
            editor_last_draw_pos: None,
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
    pub fn toggle_editor_mode(&mut self) {
        self.editor_mode = !self.editor_mode;
        if self.editor_mode {
            self.paused = true; // Auto-pause when entering editor mode
        } else {
            self.editor_last_draw_pos = None; // Reset draw position when exiting
        }
    }
    pub fn set_editor_tool(&mut self, tool: EditorTool) {
        self.editor_tool = tool;
    }
    pub fn start_simulation_from_editor<R: Rng>(&mut self, rng: &mut R) {
        // Clear existing hyphae and start fresh
        self.state.hyphae.clear();
        self.state.spores.clear();
        self.state.segments.clear();
        self.state.connections.clear();
        self.state.fruit_bodies.clear();
        self.state.fruit_cooldown_timer = 0.0;

        // Spawn initial hyphae at center
        let center = self.config.grid_size as f32 / 2.0;
        for _ in 0..self.config.initial_hyphae_count {
            let cx = center + rng.gen_range(-10.0..10.0);
            let cy = center + rng.gen_range(-10.0..10.0);
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

        // Exit editor mode and unpause
        self.editor_mode = false;
        self.paused = false;
        self.editor_last_draw_pos = None;
    }
    pub fn editor_draw_at(&mut self, gx: usize, gy: usize) {
        // Only draw if position changed or first draw
        if let Some((last_x, last_y)) = self.editor_last_draw_pos {
            if last_x == gx && last_y == gy {
                return; // Skip if same cell
            }
        }
        self.editor_last_draw_pos = Some((gx, gy));

        let grid_size = self.config.grid_size;
        let brush_size = self.editor_brush_size;

        for dx in -(brush_size as i32)..=(brush_size as i32) {
            for dy in -(brush_size as i32)..=(brush_size as i32) {
                let nx = (gx as i32 + dx).max(0).min(grid_size as i32 - 1) as usize;
                let ny = (gy as i32 + dy).max(0).min(grid_size as i32 - 1) as usize;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist <= brush_size as f32 {
                    let intensity = 1.0 - (dist / brush_size as f32).min(1.0);
                    match self.editor_tool {
                        EditorTool::Sugar => {
                            self.state.nutrients.add_sugar(nx, ny, intensity * 0.1);
                        }
                        EditorTool::Nitrogen => {
                            self.state.nutrients.add_nitrogen(nx, ny, intensity * 0.1);
                        }
                        EditorTool::Erase => {
                            self.state.nutrients.sugar[nx][ny] =
                                (self.state.nutrients.sugar[nx][ny] - intensity * 0.1).max(0.0);
                            self.state.nutrients.nitrogen[nx][ny] =
                                (self.state.nutrients.nitrogen[nx][ny] - intensity * 0.1).max(0.0);
                        }
                    }
                }
            }
        }
    }
    pub fn reset<R: Rng>(&mut self, rng: &mut R) {
        self.state.hyphae.clear();
        self.state.spores.clear();
        self.state.segments.clear();
        self.state.connections.clear();
        self.state.connection_set.clear();
        self.state.fruit_bodies.clear();
        self.state.fruit_cooldown_timer = 0.0;
        self.state.fruiting_failed_attempts = 0;

        // Regenerate nutrients with new realistic distribution
        Self::initialize_realistic_nutrients(&mut self.state.nutrients, self.config.grid_size, rng);
        // Also reset back buffer
        Self::initialize_realistic_nutrients(
            &mut self.state.nutrients_back,
            self.config.grid_size,
            rng,
        );

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

    fn spawn_fruit_body_at<R: Rng>(
        &mut self,
        rng: &mut R,
        grid_x: usize,
        grid_y: usize,
        fallback: bool,
    ) -> bool {
        if grid_x == 0
            || grid_y == 0
            || grid_x >= self.config.grid_size - 1
            || grid_y >= self.config.grid_size - 1
        {
            return false;
        }
        if self.state.obstacles[grid_x][grid_y] {
            return false;
        }

        let jitter = 0.35f32;
        let min_bound = 1.0;
        let max_bound = self.config.grid_size as f32 - 2.0;
        let mut cx = grid_x as f32 + rng.gen_range(-jitter..jitter);
        let mut cy = grid_y as f32 + rng.gen_range(-jitter..jitter);
        cx = cx.clamp(min_bound, max_bound);
        cy = cy.clamp(min_bound, max_bound);

        let lifespan = if self.config.fruiting_lifespan_max > self.config.fruiting_lifespan_min {
            rng.gen_range(self.config.fruiting_lifespan_min..self.config.fruiting_lifespan_max)
        } else {
            self.config.fruiting_lifespan_min
        };
        let initial_release_age =
            (lifespan * self.config.fruiting_spore_release_fraction).clamp(0.0, lifespan);

        self.state.fruit_bodies.push(FruitBody {
            x: cx,
            y: cy,
            age: 0.0,
            energy: 0.0,
            lifespan,
            released_spores: false,
            next_spore_release_age: initial_release_age,
        });

        if fallback {
            let radius = 2;
            let grid_x_i = grid_x as isize;
            let grid_y_i = grid_y as isize;
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    let nx = grid_x_i + dx;
                    let ny = grid_y_i + dy;
                    if nx <= 0
                        || ny <= 0
                        || nx >= self.config.grid_size as isize - 1
                        || ny >= self.config.grid_size as isize - 1
                    {
                        continue;
                    }
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    if dist > radius as f32 {
                        continue;
                    }
                    let falloff = (1.0 - dist / radius as f32).max(0.0);
                    let sugar_amount = self.config.nutrient_regen_rate * 4.0 * falloff;
                    let nitrogen_amount = self.config.nutrient_regen_rate * 2.0 * falloff;
                    self.state
                        .nutrients
                        .add_sugar(nx as usize, ny as usize, sugar_amount);
                    self.state
                        .nutrients
                        .add_nitrogen(nx as usize, ny as usize, nitrogen_amount);
                }
            }
        }

        self.state.fruit_cooldown_timer = self.config.fruiting_cooldown;
        self.state.fruiting_failed_attempts = 0;
        true
    }

    pub fn stats(&self) -> (usize, usize, usize, usize, f32, f32) {
        // Avoid Vec allocation - iterate directly
        let mut hyphae_count = 0;
        let mut total_energy = 0.0f32;
        for h in &self.state.hyphae {
            if h.alive {
                hyphae_count += 1;
                total_energy += h.energy;
            }
        }
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

        // Reuse spatial hash grid - clear and rebuild
        let cell_size: f32 = 4.0;
        let nx = self.state.spatial_grid_nx;
        let ny = self.state.spatial_grid_ny;
        let mut hyphae_positions: Vec<(f32, f32, bool, f32, Option<usize>)> =
            Vec::with_capacity(hyphae_len);

        {
            let buckets = &mut self.state.spatial_grid;

            // Clear all buckets
            for row in buckets.iter_mut() {
                for bucket in row.iter_mut() {
                    bucket.clear();
                }
            }

            // Build spatial hash grid and snapshot positions
            for (i, h) in self.state.hyphae.iter().enumerate() {
                if !h.alive {
                    continue;
                }
                let bx = (h.x / cell_size).floor() as isize;
                let by = (h.y / cell_size).floor() as isize;
                if bx >= 0 && by >= 0 {
                    let bxu = bx as usize;
                    let byu = by as usize;
                    if bxu < nx && byu < ny {
                        buckets[bxu][byu].push(i);
                    }
                }
                // Store snapshot for this hypha
                hyphae_positions.push((h.x, h.y, h.alive, h.energy, h.parent));
            }
            // Pad to match indices
            while hyphae_positions.len() < hyphae_len {
                hyphae_positions.push((0.0, 0.0, false, 0.0, None));
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
                            h.angle +=
                                (grad_angle - h.angle) * self.config.gradient_steering_strength;
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

                // Combined neighbor density and collision check in single iteration
                let mut neighbor_count = 0.0f32;
                let mut too_close = false;
                let bx = (h.x / cell_size).floor() as isize;
                let by = (h.y / cell_size).floor() as isize;
                let density_check_dist_sq = self.config.hyphae_avoidance_distance_sq() * 4.0;
                let collision_check_dist_sq = self.config.hyphae_avoidance_distance_sq();

                let new_x = h.x + h.angle.cos() * self.config.step_size;
                let new_y = h.y + h.angle.sin() * self.config.step_size;

                for gx in (bx - 1)..=(bx + 1) {
                    if too_close {
                        break;
                    }
                    if gx < 0 {
                        continue;
                    }
                    let gux = gx as usize;
                    if gux >= nx {
                        continue;
                    }
                    for gy in (by - 1)..=(by + 1) {
                        if too_close {
                            break;
                        }
                        if gy < 0 {
                            continue;
                        }
                        let guy = gy as usize;
                        if guy >= ny {
                            continue;
                        }
                        for &other_idx in &buckets[gux][guy] {
                            if other_idx == idx || other_idx >= hyphae_positions.len() {
                                continue;
                            }
                            let (other_x, other_y, other_alive, _, _) = hyphae_positions[other_idx];
                            if !other_alive {
                                continue;
                            }
                            // Density check
                            let dx = h.x - other_x;
                            let dy = h.y - other_y;
                            let dist2 = dx * dx + dy * dy;
                            if dist2 < density_check_dist_sq {
                                neighbor_count += 1.0;
                            }
                            // Collision check
                            if !too_close {
                                let dx_new = new_x - other_x;
                                let dy_new = new_y - other_y;
                                let dist2_new = dx_new * dx_new + dy_new * dy_new;
                                if dist2_new < collision_check_dist_sq && dist2_new > 0.001 {
                                    too_close = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                let density_slow = 1.0 / (1.0 + 0.05 * neighbor_count);

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
                    if parent_idx < hyphae_positions.len() {
                        let (parent_x, parent_y, parent_alive, parent_energy, _) =
                            hyphae_positions[parent_idx];
                        if parent_alive {
                            let dx = h.x - parent_x;
                            let dy = h.y - parent_y;
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

            // connections - use spatial hash grid to avoid O(n²) check
            let anastomosis_dist_sq = self.config.anastomosis_distance_sq();
            let mut new_connections: Vec<(usize, usize, f32)> = Vec::new();

            // Use spatial grid for connection checking - check only nearby hyphae
            for i in 0..self.state.hyphae.len() {
                let h1_x = self.state.hyphae[i].x;
                let h1_y = self.state.hyphae[i].y;
                let h1_energy = self.state.hyphae[i].energy;
                if !self.state.hyphae[i].alive {
                    continue;
                }

                // Check neighbors in spatial grid
                let bx = (h1_x / cell_size).floor() as isize;
                let by = (h1_y / cell_size).floor() as isize;

                // Check current and adjacent cells
                for gx in bx.max(0)..=(bx + 1).min(nx as isize - 1) {
                    for gy in by.max(0)..=(by + 1).min(ny as isize - 1) {
                        let gux = gx as usize;
                        let guy = gy as usize;
                        for &j in &buckets[gux][guy] {
                            if j <= i {
                                continue; // Only check pairs once (j > i)
                            }
                            let h2_x = self.state.hyphae[j].x;
                            let h2_y = self.state.hyphae[j].y;
                            let h2_energy = self.state.hyphae[j].energy;
                            if !self.state.hyphae[j].alive {
                                continue;
                            }

                            let dx = h1_x - h2_x;
                            let dy = h1_y - h2_y;
                            let dist2 = dx * dx + dy * dy;
                            if dist2 < anastomosis_dist_sq {
                                // Use HashSet for O(1) lookup instead of O(n) linear search
                                let key = (i, j);
                                if !self.state.connection_set.contains(&key) {
                                    self.state.connection_set.insert(key);
                                    self.state.connections.push(Connection {
                                        hypha1: i,
                                        hypha2: j,
                                    });
                                    let energy_diff = h1_energy - h2_energy;
                                    if energy_diff.abs() > 0.1 {
                                        let transfer = energy_diff * 0.1;
                                        new_connections.push((i, j, transfer));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Apply energy transfers from new connections
            for (i, j, transfer) in new_connections {
                self.state.hyphae[i].energy =
                    (self.state.hyphae[i].energy - transfer).clamp(0.0, 1.0);
                self.state.hyphae[j].energy =
                    (self.state.hyphae[j].energy + transfer).clamp(0.0, 1.0);
            }

            // Clean up dead connections
            let mut dead_connections = Vec::new();
            for (idx, c) in self.state.connections.iter().enumerate() {
                let h1_alive = self
                    .state
                    .hyphae
                    .get(c.hypha1)
                    .map(|h| h.alive)
                    .unwrap_or(false);
                let h2_alive = self
                    .state
                    .hyphae
                    .get(c.hypha2)
                    .map(|h| h.alive)
                    .unwrap_or(false);
                if !h1_alive || !h2_alive {
                    dead_connections.push(idx);
                }
            }

            // Remove dead connections in reverse order to maintain indices
            for &idx in dead_connections.iter().rev() {
                let c = &self.state.connections[idx];
                let key = if c.hypha1 < c.hypha2 {
                    (c.hypha1, c.hypha2)
                } else {
                    (c.hypha2, c.hypha1)
                };
                self.state.connection_set.remove(&key);
                self.state.connections.swap_remove(idx);
            }
        }

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

            // Use double buffering: copy active region to back buffer, diffuse, then copy back
            // We only need to copy the active region plus boundary for diffusion calculations
            let copy_x0 = x0.saturating_sub(1);
            let copy_y0 = y0.saturating_sub(1);
            let copy_x1 = (grid_size - 1).min(x1 + 1);
            let copy_y1 = (grid_size - 1).min(y1 + 1);

            // Copy region needed for diffusion (including boundaries for neighbor access)
            for x in copy_x0..=copy_x1 {
                for y in copy_y0..=copy_y1 {
                    self.state.nutrients_back.sugar[x][y] = self.state.nutrients.sugar[x][y];
                    self.state.nutrients_back.nitrogen[x][y] = self.state.nutrients.nitrogen[x][y];
                }
            }

            // Diffuse both sugar and nitrogen in back buffer (only active region)
            for x in x0..=x1 {
                for y in y0..=y1 {
                    // Sugar diffusion - read from back buffer (which has current values)
                    let avg_sugar = (self.state.nutrients_back.sugar[x + 1][y]
                        + self.state.nutrients_back.sugar[x - 1][y]
                        + self.state.nutrients_back.sugar[x][y + 1]
                        + self.state.nutrients_back.sugar[x][y - 1])
                        * 0.25;
                    self.state.nutrients_back.sugar[x][y] += self.config.diffusion_rate
                        * (avg_sugar - self.state.nutrients_back.sugar[x][y]);

                    // Nitrogen diffusion (slower)
                    let avg_nitrogen = (self.state.nutrients_back.nitrogen[x + 1][y]
                        + self.state.nutrients_back.nitrogen[x - 1][y]
                        + self.state.nutrients_back.nitrogen[x][y + 1]
                        + self.state.nutrients_back.nitrogen[x][y - 1])
                        * 0.25;
                    self.state.nutrients_back.nitrogen[x][y] += self.config.diffusion_rate
                        * 0.7
                        * (avg_nitrogen - self.state.nutrients_back.nitrogen[x][y]);
                }
            }

            // Copy only the diffused active region back to main buffer
            // This preserves the rest of the grid while updating only where needed
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.state.nutrients.sugar[x][y] = self.state.nutrients_back.sugar[x][y];
                    self.state.nutrients.nitrogen[x][y] = self.state.nutrients_back.nitrogen[x][y];
                }
            }
        }

        if self.config.nutrient_regen_rate > 0.0 && self.config.nutrient_regen_samples > 0 {
            let regen_rate = self.config.nutrient_regen_rate;
            let floor = self.config.nutrient_regen_floor;
            let grid_limit = self.config.grid_size - 1;
            for _ in 0..self.config.nutrient_regen_samples {
                let x = rng.gen_range(1..grid_limit);
                let y = rng.gen_range(1..grid_limit);
                let sugar = &mut self.state.nutrients.sugar[x][y];
                if *sugar < floor {
                    *sugar = (*sugar + regen_rate).min(floor);
                }
                let nitrogen = &mut self.state.nutrients.nitrogen[x][y];
                if *nitrogen < floor * 0.6 {
                    *nitrogen = (*nitrogen + regen_rate * 0.6).min(floor * 0.6);
                }
            }
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

        // fruiting - compute stats inline to avoid borrow conflicts
        let mut hyphae_count = 0;
        let mut total_energy = 0.0f32;
        for h in &self.state.hyphae {
            if h.alive {
                hyphae_count += 1;
                total_energy += h.energy;
            }
        }
        let fps = get_fps();
        self.state.fruit_cooldown_timer =
            (self.state.fruit_cooldown_timer - 1.0 / fps.max(1) as f32).max(0.0);
        if self.state.fruit_cooldown_timer <= 0.0
            && hyphae_count >= self.config.fruiting_min_hyphae
            && total_energy >= self.config.fruiting_threshold_total_energy
        {
            let mut weighted_cx = 0.0f32;
            let mut weighted_cy = 0.0f32;
            let mut first_alive_position: Option<(f32, f32)> = None;
            for h in self.state.hyphae.iter().filter(|h| h.alive) {
                weighted_cx += h.x * h.energy;
                weighted_cy += h.y * h.energy;
                if first_alive_position.is_none() {
                    first_alive_position = Some((h.x, h.y));
                }
            }
            let mut cx = if total_energy > 0.0 {
                weighted_cx / total_energy
            } else {
                first_alive_position
                    .map(|(x, _)| x)
                    .unwrap_or(self.config.grid_size as f32 / 2.0)
            };
            let mut cy = if total_energy > 0.0 {
                weighted_cy / total_energy
            } else {
                first_alive_position
                    .map(|(_, y)| y)
                    .unwrap_or(self.config.grid_size as f32 / 2.0)
            };

            // Add slight randomness to avoid stacking and promote exploration
            cx += rng.gen_range(-1.5..1.5);
            cy += rng.gen_range(-1.5..1.5);

            // Clamp to playable area
            let min_bound = 1.0;
            let max_bound = self.config.grid_size as f32 - 2.0;
            cx = cx.clamp(min_bound, max_bound);
            cy = cy.clamp(min_bound, max_bound);

            if in_bounds(cx, cy, self.config.grid_size) {
                let grid_x = cx.round() as usize;
                let grid_y = cy.round() as usize;
                let (target_cell, best_nutrient) = {
                    let mut best_cell = (grid_x, grid_y);
                    let nutrients = &self.state.nutrients;
                    let mut best = nutrients.total_at(grid_x, grid_y);

                    if best < self.config.fruiting_spawn_nutrient_threshold {
                        let search_radius = 6isize;
                        let gx_i = grid_x as isize;
                        let gy_i = grid_y as isize;
                        for dx in -search_radius..=search_radius {
                            for dy in -search_radius..=search_radius {
                                let nx = gx_i + dx;
                                let ny = gy_i + dy;
                                if nx <= 0
                                    || ny <= 0
                                    || nx >= (self.config.grid_size as isize - 1)
                                    || ny >= (self.config.grid_size as isize - 1)
                                {
                                    continue;
                                }
                                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                                if dist > search_radius as f32 {
                                    continue;
                                }
                                let nu = nutrients.total_at(nx as usize, ny as usize);
                                if nu > best {
                                    best = nu;
                                    best_cell = (nx as usize, ny as usize);
                                }
                            }
                        }
                    }

                    (best_cell, best)
                };

                let spawn_threshold = self.config.fruiting_spawn_nutrient_threshold;
                let relaxed_threshold = (spawn_threshold * 0.65).min(spawn_threshold);
                let (tx, ty) = target_cell;
                let mut spawned = false;
                let mut spawn_request: Option<bool> = None;
                let mut attempts_update: Option<u32> = None;

                if best_nutrient >= spawn_threshold
                    || (best_nutrient >= relaxed_threshold && rng.gen_bool(0.35))
                {
                    spawn_request = Some(false);
                } else {
                    let attempts = self.state.fruiting_failed_attempts.saturating_add(1);
                    let max_attempts = self.config.fruiting_failed_attempts_before_fallback.max(1);
                    attempts_update = Some(attempts.min(max_attempts));
                    let fallback_due =
                        attempts >= self.config.fruiting_failed_attempts_before_fallback;
                    if fallback_due {
                        let fallback_allowed = best_nutrient
                            >= self.config.fruiting_fallback_threshold
                            || (hyphae_count >= self.config.fruiting_min_hyphae * 2
                                && total_energy
                                    >= self.config.fruiting_threshold_total_energy * 1.4);
                        if fallback_allowed {
                            spawn_request = Some(true);
                        }
                    }
                }

                if let Some(is_fallback) = spawn_request {
                    spawned = self.spawn_fruit_body_at(rng, tx, ty, is_fallback);
                    if spawned {
                        attempts_update = None;
                    }
                }

                if let Some(value) = attempts_update {
                    if !spawned {
                        self.state.fruiting_failed_attempts = value;
                    }
                }
            }
        }

        // Energy transfer from hyphae to fruiting bodies - use spatial grid and handle lifecycle
        let transfer_radius = 15.0f32;
        let transfer_radius_sq = transfer_radius * transfer_radius;
        let transfer_cell_range = (transfer_radius / cell_size).ceil() as isize;
        let buckets = &self.state.spatial_grid;
        let mut fruit_spore_events: Vec<(f32, f32)> = Vec::new();
        let mut fruit_deaths: Vec<(usize, f32, f32, f32, bool)> = Vec::new();

        for (idx, f) in self.state.fruit_bodies.iter_mut().enumerate() {
            f.age += 0.01;
            let mut total_transfer = 0.0f32;
            let mut transfers: Vec<(usize, f32)> = Vec::new();

            // Use spatial grid to find nearby hyphae
            let fx = f.x;
            let fy = f.y;
            let bx = (fx / cell_size).floor() as isize;
            let by = (fy / cell_size).floor() as isize;

            for gx in
                (bx - transfer_cell_range).max(0)..=(bx + transfer_cell_range).min(nx as isize - 1)
            {
                for gy in (by - transfer_cell_range).max(0)
                    ..=(by + transfer_cell_range).min(ny as isize - 1)
                {
                    let gux = gx as usize;
                    let guy = gy as usize;
                    for &h_idx in &buckets[gux][guy] {
                        let h_ref = &self.state.hyphae[h_idx];
                        if !h_ref.alive || h_ref.energy < 0.1 {
                            continue;
                        }
                        let dx = fx - h_ref.x;
                        let dy = fy - h_ref.y;
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq < transfer_radius_sq && dist_sq > 0.1 {
                            let dist = dist_sq.sqrt();
                            let transfer_rate = 0.01 * (1.0 - dist / transfer_radius).max(0.0);
                            let transfer = (h_ref.energy * transfer_rate).min(0.05);
                            if transfer > 0.001 {
                                transfers.push((h_idx, transfer));
                                total_transfer += transfer;
                            }
                        }
                    }
                }
            }

            // Apply transfers after iteration to avoid borrow conflicts
            for (h_idx, transfer) in transfers {
                self.state.hyphae[h_idx].energy -= transfer;
            }
            f.energy = (f.energy + total_transfer).min(1.0);

            let release_interval =
                (self.config.fruiting_spore_release_interval.max(0.01) * f.lifespan).max(0.1);
            while f.age >= f.next_spore_release_age && f.next_spore_release_age < f.lifespan {
                fruit_spore_events.push((fx, fy));
                f.released_spores = true;
                f.next_spore_release_age += release_interval;
            }

            if f.age >= f.lifespan {
                fruit_deaths.push((idx, fx, fy, f.energy, !f.released_spores));
            }
        }

        // Handle fruit body deaths: nutrient return + removal
        for (idx, fx, fy, energy, needs_final_release) in fruit_deaths.into_iter().rev() {
            if needs_final_release {
                fruit_spore_events.push((fx, fy));
            }
            if energy > 0.0 {
                let nutrient_return = energy * self.config.fruiting_nutrient_return_fraction;
                if nutrient_return > 0.0 {
                    let radius = 3;
                    let center_x = fx.round() as isize;
                    let center_y = fy.round() as isize;
                    for dx in -radius..=radius {
                        for dy in -radius..=radius {
                            let nx = center_x + dx;
                            let ny = center_y + dy;
                            if nx < 0
                                || ny < 0
                                || nx >= self.config.grid_size as isize
                                || ny >= self.config.grid_size as isize
                            {
                                continue;
                            }
                            let dist = ((dx * dx + dy * dy) as f32).sqrt();
                            if dist > radius as f32 {
                                continue;
                            }
                            let falloff = (1.0 - dist / radius as f32).max(0.0);
                            let sugar_amount = nutrient_return * 0.7 * falloff * 0.4;
                            let nitrogen_amount = nutrient_return * 0.3 * falloff * 0.4;
                            self.state
                                .nutrients
                                .add_sugar(nx as usize, ny as usize, sugar_amount);
                            self.state.nutrients.add_nitrogen(
                                nx as usize,
                                ny as usize,
                                nitrogen_amount,
                            );
                        }
                    }
                }
            }
            self.state.fruit_bodies.swap_remove(idx);
        }

        // Process all queued spore releases
        for (fx, fy) in fruit_spore_events {
            let spore_radius = self.config.fruiting_spore_radius.max(1.0);
            for _ in 0..self.config.fruiting_spore_count {
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let distance = rng.gen_range(0.5..spore_radius);
                let sx = fx + angle.cos() * distance;
                let sy = fy + angle.sin() * distance;
                if !in_bounds(sx, sy, self.config.grid_size) {
                    continue;
                }
                let vel_angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let speed = rng.gen_range(0.02..self.config.fruiting_spore_drift.max(0.05));
                let vx = vel_angle.cos() * speed;
                let vy = vel_angle.sin() * speed;
                self.state.spores.push(Spore {
                    x: sx,
                    y: sy,
                    vx,
                    vy,
                    alive: true,
                    age: 0.0,
                });

                // Occasionally germinate immediately into a new hypha for faster colonization
                if rng.gen_bool(0.3) {
                    let hx = sx + rng.gen_range(-0.5..0.5);
                    let hy = sy + rng.gen_range(-0.5..0.5);
                    if in_bounds(hx, hy, self.config.grid_size) {
                        self.spawn_hypha_at(rng, hx, hy);
                    }
                }
            }
        }
    }
}
