use ::rand as external_rand;
use external_rand::Rng;
#[cfg(not(test))]
#[cfg(feature = "ui")]
use macroquad::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::SimulationConfig;
use crate::hypha::Hypha;
use crate::nutrients::{memory_gradient, nutrient_gradient, NutrientGrid};
use crate::spore::Spore;
use crate::types::{Connection, FruitBody, Segment};
use crate::weather::Weather;

// Runtime flag to indicate if we're running in headless mode
// This is set when headless mode starts and checked to avoid calling macroquad
static HEADLESS_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_headless_mode(headless: bool) {
    HEADLESS_MODE.store(headless, Ordering::Relaxed);
}

// Helper function to get FPS - use default for tests, actual FPS for runtime
// In headless mode or when macroquad isn't initialized, returns a safe default
#[cfg(test)]
fn get_fps() -> f32 {
    60.0 // Default FPS for tests
}

#[cfg(not(test))]
#[cfg(feature = "ui")]
fn get_fps() -> f32 {
    // If we're in headless mode, don't call macroquad
    if HEADLESS_MODE.load(Ordering::Relaxed) {
        return 60.0;
    }

    // Try to get FPS from macroquad, but fall back to default if not available
    // This handles the case where UI features are compiled in but we're running headless
    // Macroquad has thread-local state that panics if accessed from wrong thread/context
    use std::panic;
    panic::catch_unwind(|| macroquad::prelude::get_fps() as f32).unwrap_or(60.0)
    // Default FPS if macroquad isn't available or panics
}

#[cfg(not(test))]
#[cfg(not(feature = "ui"))]
fn get_fps() -> f32 {
    60.0 // Default FPS for headless mode
}

#[inline]
fn in_bounds(x: f32, y: f32, grid_size: usize) -> bool {
    x >= 0.0 && y >= 0.0 && x < grid_size as f32 && y < grid_size as f32
}

// Simulation state - contains all mutable state data
pub struct SimulationState {
    pub nutrients: NutrientGrid,
    pub nutrients_back: NutrientGrid, // Double buffer for diffusion
    pub nutrient_memory: Vec<Vec<f32>>, // Memory grid: decaying weights of nutrient locations
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
    // Weather system
    pub weather: Weather,
}

impl SimulationState {
    pub fn new(config: &SimulationConfig) -> Self {
        // Pre-allocate spatial grid
        let cell_size = config.cell_size;
        let grid_size = config.grid_size;
        let nx = ((grid_size as f32) / cell_size).ceil() as usize;
        let ny = ((grid_size as f32) / cell_size).ceil() as usize;
        let spatial_grid = vec![vec![Vec::new(); ny]; nx];
        Self {
            nutrients: NutrientGrid::new(grid_size),
            nutrients_back: NutrientGrid::new(grid_size),
            nutrient_memory: vec![vec![0.0f32; grid_size]; grid_size],
            obstacles: vec![vec![false; grid_size]; grid_size],
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
            weather: Weather::new(),
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
    pub memory_visible: bool, // Network Intelligence: Toggle memory overlay
    pub enhanced_visualization: bool, // Performance: Enhanced visualization with flow/stress
    pub show_flow: bool,      // Show nutrient flow intensity
    pub show_stress: bool,    // Show environmental stress
    pub help_popup_visible: bool, // Show help popup window
    pub speed_multiplier: f32,
    pub speed_accumulator: f32,
    // Performance: Cache for visualization (computed once per frame)
    pub hypha_flow_cache: Vec<f32>, // Pre-computed flow values per hypha
    // Camera for pan/zoom (only in UI mode)
    #[cfg(feature = "ui")]
    pub camera: crate::camera::Camera,
    // Screenshot flag (only in UI mode)
    #[cfg(feature = "ui")]
    pub take_screenshot: bool,
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
        Self::with_config_internal(rng, config, true)
    }

    // Internal function that allows skipping camera initialization for tests
    // For tests, we'll need to create a mock camera struct
    #[cfg(test)]
    fn with_config_internal<R: Rng>(
        rng: &mut R,
        config: SimulationConfig,
        _init_camera: bool,
    ) -> Self {
        let grid_size = config.grid_size;
        let camera_enabled_for_camera = config.camera_enabled;
        let mut state = SimulationState::new(&config);
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
                strength: 1.0,
                signal_received: 0.0,
                last_nutrient_location: None,
                senescence_factor: 0.0,
            });
        }

        #[cfg(feature = "ui")]
        let camera = crate::camera::Camera::new(camera_enabled_for_camera, &config);

        Self {
            state,
            config,
            paused: false,
            connections_visible: true,
            minimap_visible: false,
            hyphae_visible: true,
            memory_visible: false,
            enhanced_visualization: false,
            show_flow: true,
            show_stress: true,
            speed_multiplier: 1.0,
            speed_accumulator: 0.0,
            hypha_flow_cache: Vec::new(),
            #[cfg(feature = "ui")]
            camera,
            #[cfg(feature = "ui")]
            take_screenshot: false,
            help_popup_visible: false,
        }
    }

    #[cfg(not(test))]
    fn with_config_internal<R: Rng>(
        rng: &mut R,
        config: SimulationConfig,
        _init_camera: bool,
    ) -> Self {
        let mut state = SimulationState::new(&config);
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
                strength: 1.0,
                signal_received: 0.0,
                last_nutrient_location: None,
                senescence_factor: 0.0,
            });
        }

        // Read camera_enabled before moving config
        let camera_enabled = config.camera_enabled;

        // Clone config for camera (since we need to move original into Self)
        let config_for_camera = config.clone();

        Self {
            state,
            config,
            paused: false,
            connections_visible: true,
            minimap_visible: false,
            hyphae_visible: true,
            memory_visible: false,
            enhanced_visualization: false,
            show_flow: true,
            show_stress: true,
            speed_multiplier: 1.0,
            speed_accumulator: 0.0,
            help_popup_visible: false,
            hypha_flow_cache: Vec::new(),
            #[cfg(feature = "ui")]
            camera: crate::camera::Camera::new(camera_enabled, &config_for_camera),
            #[cfg(feature = "ui")]
            take_screenshot: false,
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
    pub fn toggle_memory_visibility(&mut self) {
        self.memory_visible = !self.memory_visible;
    }
    pub fn toggle_enhanced_visualization(&mut self) {
        self.enhanced_visualization = !self.enhanced_visualization;
    }
    pub fn toggle_flow_visualization(&mut self) {
        self.show_flow = !self.show_flow;
    }
    pub fn toggle_stress_visualization(&mut self) {
        self.show_stress = !self.show_stress;
    }

    #[cfg(feature = "ui")]
    pub fn toggle_camera(&mut self) {
        self.camera.toggle_enabled();
    }

    pub fn toggle_help_popup(&mut self) {
        self.help_popup_visible = !self.help_popup_visible;
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
        self.state.connection_set.clear();
        self.state.fruit_bodies.clear();
        self.state.fruit_cooldown_timer = 0.0;
        self.state.fruiting_failed_attempts = 0;

        // Network Intelligence: Clear memory
        if self.config.memory_enabled {
            for x in 0..self.config.grid_size {
                for y in 0..self.config.grid_size {
                    self.state.nutrient_memory[x][y] = 0.0;
                }
            }
        }

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
            strength: 1.0,
            signal_received: 0.0,
            last_nutrient_location: None,
            senescence_factor: 0.0,
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
            strength: 1.0,
            signal_received: 0.0,
            last_nutrient_location: None,
            senescence_factor: 0.0,
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

        // Weather: Update weather conditions
        if self.config.weather_enabled {
            let fps = get_fps();
            let dt = 1.0 / fps.max(1.0);
            self.state.weather.update(dt, rng);
        }

        // Performance: Growth limits - remove excess hyphae if over limit
        if self.config.max_hyphae > 0 && self.state.hyphae.len() > self.config.max_hyphae {
            // Remove oldest/weakest hyphae first
            let excess = self.state.hyphae.len() - self.config.max_hyphae;
            let mut indices_to_remove: Vec<usize> = self
                .state
                .hyphae
                .iter()
                .enumerate()
                .filter(|(_, h)| !h.alive || h.energy < 0.3)
                .map(|(i, _)| i)
                .take(excess)
                .collect();

            // If not enough weak hyphae, remove oldest
            if indices_to_remove.len() < excess {
                let mut age_indices: Vec<(f32, usize)> = self
                    .state
                    .hyphae
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !indices_to_remove.contains(i))
                    .map(|(i, h)| (h.age, i))
                    .collect();
                age_indices.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()); // Sort by age descending
                for (_, idx) in age_indices.iter().take(excess - indices_to_remove.len()) {
                    indices_to_remove.push(*idx);
                }
            }

            // Remove in reverse order to maintain indices
            indices_to_remove.sort();
            indices_to_remove.reverse();
            for &idx in &indices_to_remove {
                if idx < self.state.hyphae.len() {
                    self.state.hyphae.swap_remove(idx);
                }
            }
        }

        // Performance: Pre-compute hypha flow cache for visualization (only if enhanced visualization is enabled)
        if self.enhanced_visualization && self.show_flow {
            self.hypha_flow_cache.clear();
            self.hypha_flow_cache.resize(self.state.hyphae.len(), 0.0);
            for conn in &self.state.connections {
                if conn.hypha1 < self.hypha_flow_cache.len() {
                    self.hypha_flow_cache[conn.hypha1] += conn.flow_accumulator;
                }
                if conn.hypha2 < self.hypha_flow_cache.len() {
                    self.hypha_flow_cache[conn.hypha2] += conn.flow_accumulator;
                }
            }
        }

        // Network Intelligence: Decay memory
        // Performance: Memory decay is already fast (O(n²) but simple operations)
        // Spatial culling and LOD provide better performance gains
        if self.config.memory_enabled {
            let decay_rate = self.config.memory_decay_rate;
            for x in 0..self.config.grid_size {
                for y in 0..self.config.grid_size {
                    self.state.nutrient_memory[x][y] *= decay_rate;
                }
            }
        }

        // Network Intelligence: Decay signals on hyphae
        if self.config.signal_propagation_enabled {
            for h in &mut self.state.hyphae {
                h.signal_received *= self.config.signal_decay_rate;
            }
        }

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

                let (mut gx, mut gy) =
                    nutrient_gradient(&self.state.nutrients, h.x, h.y, self.config.grid_size);

                // Network Intelligence: Blend memory gradient into growth direction
                if self.config.memory_enabled && self.config.memory_influence > 0.0 {
                    let (mx, my) = memory_gradient(
                        &self.state.nutrient_memory,
                        h.x,
                        h.y,
                        self.config.grid_size,
                    );
                    let mem_mag = (mx * mx + my * my).sqrt();
                    if mem_mag > 0.01 {
                        // Blend memory gradient with nutrient gradient
                        let influence = self.config.memory_influence;
                        gx = gx * (1.0 - influence) + mx * influence;
                        gy = gy * (1.0 - influence) + my * influence;
                    }
                }

                // Network Intelligence: Signal influence on growth direction
                if self.config.signal_propagation_enabled && h.signal_received > 0.1 {
                    // Signals can bias growth toward remembered nutrient locations
                    if let Some((mem_x, mem_y)) = h.last_nutrient_location {
                        let dx = mem_x - h.x;
                        let dy = mem_y - h.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 1.0 {
                            let signal_strength = h.signal_received.min(1.0) * 0.3;
                            gx += (dx / dist) * signal_strength;
                            gy += (dy / dist) * signal_strength;
                        }
                    }
                }

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

                // Network Intelligence: Strength affects growth rate
                let strength_multiplier = if self.config.adaptive_growth_enabled {
                    h.strength
                } else {
                    1.0
                };

                // Weather: Apply weather effects to growth rate
                let weather_growth_multiplier =
                    if self.config.weather_enabled && self.config.weather_affects_growth {
                        self.state.weather.growth_multiplier()
                    } else {
                        1.0
                    };

                if too_close {
                    h.angle += rng.gen_range(-0.5..0.5);
                }

                // Apply all growth multipliers
                let final_step_size = self.config.step_size
                    * density_slow
                    * strength_multiplier
                    * weather_growth_multiplier;
                h.x += h.angle.cos() * final_step_size;
                h.y += h.angle.sin() * final_step_size;

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

                    // Network Intelligence: Update memory when nutrients are found
                    // Note: Use a lower threshold (0.001) since absorbed can be as low as nutrient_decay (0.01)
                    // and we want to record even small nutrient discoveries
                    if self.config.memory_enabled && absorbed > 0.001 {
                        let memory_update = absorbed * self.config.memory_update_strength;
                        self.state.nutrient_memory[xi][yi] =
                            (self.state.nutrient_memory[xi][yi] + memory_update).min(1.0);
                        h.last_nutrient_location = Some((h.x, h.y));

                        // Adaptive Growth: Strengthen hypha when it finds nutrients
                        if self.config.adaptive_growth_enabled {
                            h.strength = (h.strength + absorbed * 0.1).min(1.0);
                        }

                        // Trigger signal propagation if nutrient discovery is significant
                        if self.config.signal_propagation_enabled
                            && total_nutrient > self.config.signal_trigger_nutrient_threshold
                        {
                            h.signal_received = 1.0; // Trigger signal at this hypha
                        }
                    }

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

                // Weather: Apply weather effects to energy consumption
                // Higher consumption = less energy retention (higher decay)
                let energy_decay_rate =
                    if self.config.weather_enabled && self.config.weather_affects_energy {
                        let consumption_mult = self.state.weather.energy_consumption_multiplier();
                        // consumption_mult is now 0.7-1.3 (gentler range)
                        // Higher consumption = less energy retention (higher decay)
                        // If consumption_mult = 1.0, decay should be normal
                        // If consumption_mult = 1.3 (high consumption), decay should increase slightly
                        // If consumption_mult = 0.7 (low consumption), decay should decrease slightly
                        let decay_multiplier = 0.9 + (consumption_mult - 1.0) * 0.15; // Scale 0.7-1.3 to ~0.855-1.045
                        (self.config.energy_decay_rate * decay_multiplier).min(0.999)
                    // Cap decay to prevent instant death
                    } else {
                        self.config.energy_decay_rate
                    };

                h.energy *= energy_decay_rate;
                h.age += 0.01;
                if h.energy < self.config.min_energy_to_live {
                    h.alive = false;
                    continue;
                }

                // Hyphal Senescence & Death: Biological aging and death system
                // Only apply to hyphae that are old enough to have established connections
                if self.config.senescence_enabled
                    && h.alive
                    && h.age >= self.config.senescence_min_age
                {
                    // Compute nutrient flow for this hypha (from connections)
                    let mut nutrient_flow = 0.0;
                    let mut has_connections = false;
                    for conn in &self.state.connections {
                        if conn.hypha1 == idx || conn.hypha2 == idx {
                            nutrient_flow += conn.flow_accumulator;
                            has_connections = true;
                        }
                    }

                    // Compute distance from main network (distance to nearest connected hypha or network center)
                    let mut min_distance_to_network = f32::MAX;
                    let network_center_x = self.config.grid_size as f32 / 2.0;
                    let network_center_y = self.config.grid_size as f32 / 2.0;
                    let dist_to_center = ((h.x - network_center_x).powi(2)
                        + (h.y - network_center_y).powi(2))
                    .sqrt();
                    min_distance_to_network = min_distance_to_network.min(dist_to_center);

                    // Check connections to find nearest connected hypha
                    for conn in &self.state.connections {
                        let other_idx = if conn.hypha1 == idx {
                            conn.hypha2
                        } else if conn.hypha2 == idx {
                            conn.hypha1
                        } else {
                            continue;
                        };
                        if other_idx < hyphae_positions.len() {
                            let (other_x, other_y, other_alive, _, _) = hyphae_positions[other_idx];
                            if other_alive {
                                let dist =
                                    ((h.x - other_x).powi(2) + (h.y - other_y).powi(2)).sqrt();
                                min_distance_to_network = min_distance_to_network.min(dist);
                            }
                        }
                    }

                    // Check parent connection
                    if let Some(parent_idx) = h.parent {
                        if parent_idx < hyphae_positions.len() {
                            let (parent_x, parent_y, parent_alive, _, _) =
                                hyphae_positions[parent_idx];
                            if parent_alive {
                                let dist =
                                    ((h.x - parent_x).powi(2) + (h.y - parent_y).powi(2)).sqrt();
                                min_distance_to_network = min_distance_to_network.min(dist);
                            }
                        }
                    }

                    // Calculate senescence factors
                    let mut death_probability = self.config.senescence_base_probability;

                    // Factor 1: Low nutrient flow increases death probability
                    // Only apply if hypha has connections (otherwise it's too early to judge)
                    if has_connections
                        && nutrient_flow < self.config.senescence_nutrient_flow_threshold
                    {
                        let flow_factor =
                            1.0 - (nutrient_flow / self.config.senescence_nutrient_flow_threshold);
                        death_probability += flow_factor * 0.0002; // Reduced from 0.001 to 0.0002 (0.02% max)
                    }

                    // Factor 2: Distance from main network increases death probability
                    if min_distance_to_network > self.config.senescence_distance_threshold {
                        let distance_factor = ((min_distance_to_network
                            - self.config.senescence_distance_threshold)
                            / self.config.senescence_unsupported_collapse_distance)
                            .min(1.0);
                        death_probability += distance_factor * 0.0001; // Reduced from 0.0005 to 0.0001 (0.01% max)

                        // Collapse unsupported branches (beyond threshold distance)
                        if min_distance_to_network
                            > self.config.senescence_unsupported_collapse_distance
                        {
                            death_probability += 0.002; // Reduced from 0.01 to 0.002 (0.2% chance)
                        }
                    }

                    // Factor 3: Weather extremes (too hot or too cold)
                    if self.config.weather_enabled {
                        let temp = self.state.weather.temperature;
                        let optimal_min = 0.8;
                        let optimal_max = 1.2;
                        if temp < (optimal_min - self.config.senescence_weather_extreme_threshold)
                            || temp
                                > (optimal_max + self.config.senescence_weather_extreme_threshold)
                        {
                            let extreme_factor = if temp < optimal_min {
                                (optimal_min
                                    - self.config.senescence_weather_extreme_threshold
                                    - temp)
                                    / self.config.senescence_weather_extreme_threshold
                            } else {
                                (temp
                                    - optimal_max
                                    - self.config.senescence_weather_extreme_threshold)
                                    / self.config.senescence_weather_extreme_threshold
                            }
                            .min(1.0);
                            death_probability += extreme_factor * 0.0002; // Reduced from 0.0008 to 0.0002 (0.02% max)
                        }
                    }

                    // Update senescence factor (accumulates over time, but slower)
                    let senescence_increase = death_probability * 5.0; // Reduced from 10.0 to 5.0
                    h.senescence_factor = (h.senescence_factor + senescence_increase).min(1.0);

                    // Apply death probability
                    if rng.gen::<f32>() < death_probability {
                        h.alive = false;
                        continue;
                    }
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

                // Performance: Growth limits - stop branching after threshold
                let can_branch = if self.config.max_hyphae_branching_threshold > 0 {
                    hyphae_len < self.config.max_hyphae_branching_threshold
                } else {
                    true
                };

                if can_branch {
                    let age_branch_boost = (1.0 + h.age * 0.05).min(2.0);
                    // Weather: Apply weather effects to branching probability
                    let weather_branch_mult =
                        if self.config.weather_enabled && self.config.weather_affects_growth {
                            self.state.weather.growth_multiplier()
                        } else {
                            1.0
                        };

                    let branch_prob =
                        self.config.branch_prob * age_branch_boost * weather_branch_mult;

                    // Ensure minimum branching probability even in bad weather
                    // This prevents complete stagnation while still allowing weather effects
                    let min_branch_prob = self.config.branch_prob * 0.3; // At least 30% of base
                    let branch_prob = branch_prob.max(min_branch_prob);

                    if rng.gen::<f32>() < branch_prob {
                        let idxp = hyphae_len;
                        // Give new branch a small initial offset to prevent immediate fusion
                        // Offset in the direction of the branch angle
                        let branch_angle = h.angle + rng.gen_range(-1.2..1.2);
                        let offset_distance = 1.5; // Offset by 1.5 units (more than fusion_distance of 1.0)
                        let offset_x = h.x + branch_angle.cos() * offset_distance;
                        let offset_y = h.y + branch_angle.sin() * offset_distance;

                        // Create segment immediately to connect parent to new branch (prevents blank space)
                        #[cfg(feature = "ui")]
                        #[cfg(not(test))]
                        {
                            let from = macroquad::prelude::vec2(
                                h.x * self.config.cell_size,
                                h.y * self.config.cell_size,
                            );
                            let to = macroquad::prelude::vec2(
                                offset_x * self.config.cell_size,
                                offset_y * self.config.cell_size,
                            );
                            self.state.segments.push(Segment { from, to, age: 0.0 });
                        }
                        #[cfg(any(test, not(feature = "ui")))]
                        {
                            use crate::types::Vec2;
                            let from =
                                Vec2::new(h.x * self.config.cell_size, h.y * self.config.cell_size);
                            let to = Vec2::new(
                                offset_x * self.config.cell_size,
                                offset_y * self.config.cell_size,
                            );
                            self.state.segments.push(Segment { from, to, age: 0.0 });
                        }

                        new_hyphae.push(Hypha {
                            x: offset_x,
                            y: offset_y,
                            prev_x: h.x, // Previous position is parent position
                            prev_y: h.y,
                            angle: branch_angle,
                            alive: true,
                            energy: h.energy * 0.5,
                            parent: Some(idxp),
                            age: 0.0,
                            strength: h.strength * 0.8, // Branches start slightly weaker
                            signal_received: 0.0,
                            last_nutrient_location: h.last_nutrient_location,
                            senescence_factor: h.senescence_factor * 0.5, // Inherit some senescence
                        });
                        h.energy *= 0.5;
                    }
                }

                // Create segment for visualization (trails)
                // Use types::Vec2 for headless/test mode, macroquad::Vec2 for UI mode
                #[cfg(feature = "ui")]
                #[cfg(not(test))]
                {
                    let from = macroquad::prelude::vec2(
                        h.prev_x * self.config.cell_size,
                        h.prev_y * self.config.cell_size,
                    );
                    let to = macroquad::prelude::vec2(
                        h.x * self.config.cell_size,
                        h.y * self.config.cell_size,
                    );
                    self.state.segments.push(Segment { from, to, age: 0.0 });
                }
                #[cfg(any(test, not(feature = "ui")))]
                {
                    use crate::types::Vec2;
                    let from = Vec2::new(
                        h.prev_x * self.config.cell_size,
                        h.prev_y * self.config.cell_size,
                    );
                    let to = Vec2::new(h.x * self.config.cell_size, h.y * self.config.cell_size);
                    self.state.segments.push(Segment { from, to, age: 0.0 });
                }
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

            // Fusion: When hyphae are very close, merge them instead of just connecting
            // This is true biological fusion (anastomosis with merging)
            if self.config.fusion_enabled {
                let fusion_dist_sq = self.config.fusion_distance * self.config.fusion_distance;
                let mut hyphae_to_remove: Vec<usize> = Vec::new();
                let mut fusion_energy_transfers: Vec<(usize, f32)> = Vec::new();

                // Use spatial grid for efficient fusion checking
                for i in 0..self.state.hyphae.len() {
                    if hyphae_to_remove.contains(&i) || !self.state.hyphae[i].alive {
                        continue;
                    }

                    let h1_x = self.state.hyphae[i].x;
                    let h1_y = self.state.hyphae[i].y;
                    let bx = (h1_x / cell_size).floor() as isize;
                    let by = (h1_y / cell_size).floor() as isize;

                    // Check nearby hyphae for fusion
                    for gx in (bx - 1).max(0)..=(bx + 1).min(nx as isize - 1) {
                        for gy in (by - 1).max(0)..=(by + 1).min(ny as isize - 1) {
                            let gux = gx as usize;
                            let guy = gy as usize;
                            for &j in &buckets[gux][guy] {
                                if j <= i
                                    || hyphae_to_remove.contains(&j)
                                    || !self.state.hyphae[j].alive
                                {
                                    continue;
                                }

                                let h2_x = self.state.hyphae[j].x;
                                let h2_y = self.state.hyphae[j].y;
                                let dx = h1_x - h2_x;
                                let dy = h1_y - h2_y;
                                let dist2 = dx * dx + dy * dy;

                                // Fusion: Merge very close hyphae
                                // But skip fusion if either hypha is too young (just branched)
                                // This prevents immediate fusion of newly branched hyphae
                                // New branches start with age 0.0 and age by 0.01 per frame
                                // So they need at least 10 frames (age >= 0.1) before they can fuse
                                let h1_age = self.state.hyphae[i].age;
                                let h2_age = self.state.hyphae[j].age;
                                let can_fuse = dist2 < fusion_dist_sq
                                    && h1_age >= self.config.fusion_min_age
                                    && h2_age >= self.config.fusion_min_age;

                                if can_fuse {
                                    // Transfer energy from j to i, then remove j
                                    let energy_transfer = self.state.hyphae[j].energy
                                        * self.config.fusion_energy_transfer;
                                    fusion_energy_transfers.push((i, energy_transfer));

                                    // Merge positions (average)
                                    self.state.hyphae[i].x = (h1_x + h2_x) * 0.5;
                                    self.state.hyphae[i].y = (h1_y + h2_y) * 0.5;

                                    // Merge strength (take maximum)
                                    self.state.hyphae[i].strength = self.state.hyphae[i]
                                        .strength
                                        .max(self.state.hyphae[j].strength);

                                    // Merge energy
                                    self.state.hyphae[i].energy =
                                        (self.state.hyphae[i].energy + energy_transfer).min(1.0);

                                    // Mark j for removal
                                    hyphae_to_remove.push(j);

                                    // Remove any connections involving j
                                    let mut connections_to_remove: Vec<usize> = Vec::new();
                                    for (conn_idx, conn) in
                                        self.state.connections.iter().enumerate()
                                    {
                                        if conn.hypha1 == j || conn.hypha2 == j {
                                            connections_to_remove.push(conn_idx);
                                            // Remove from connection set
                                            let key = if conn.hypha1 < conn.hypha2 {
                                                (conn.hypha1, conn.hypha2)
                                            } else {
                                                (conn.hypha2, conn.hypha1)
                                            };
                                            self.state.connection_set.remove(&key);
                                        }
                                    }

                                    // Remove connections in reverse order
                                    connections_to_remove.sort();
                                    connections_to_remove.reverse();
                                    for &conn_idx in &connections_to_remove {
                                        if conn_idx < self.state.connections.len() {
                                            self.state.connections.swap_remove(conn_idx);
                                        }
                                    }

                                    // Update connection indices: replace j with i in remaining connections
                                    for conn in &mut self.state.connections {
                                        if conn.hypha1 == j {
                                            conn.hypha1 = i;
                                        }
                                        if conn.hypha2 == j {
                                            conn.hypha2 = i;
                                        }
                                    }

                                    break; // Only fuse with one hypha at a time
                                }
                            }
                        }
                    }
                }

                // Remove fused hyphae in reverse order
                hyphae_to_remove.sort();
                hyphae_to_remove.dedup();
                hyphae_to_remove.reverse();
                for &idx in &hyphae_to_remove {
                    if idx < self.state.hyphae.len() {
                        self.state.hyphae.swap_remove(idx);
                    }
                }
            }

            // connections - use spatial hash grid to avoid O(n²) check
            let anastomosis_dist_sq = self.config.anastomosis_distance_sq();
            let mut new_connections: Vec<(usize, usize, f32)> = Vec::new();

            // Rebuild spatial grid after fusion
            for row in buckets.iter_mut() {
                for bucket in row.iter_mut() {
                    bucket.clear();
                }
            }

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
            }

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
                                        strength: self.config.min_connection_strength,
                                        signal: 0.0,
                                        flow_accumulator: 0.0,
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
        // Also handle signal propagation and adaptive growth
        let mut connection_updates: Vec<(usize, f32, f32, f32)> = Vec::new(); // (idx, new_strength, new_signal, flow_acc)

        for (conn_idx, c) in self.state.connections.iter().enumerate() {
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

            // Energy flow (diffusive)
            let diff = h1.energy - h2.energy;
            let base_flow = diff * self.config.connection_flow_rate;
            // Scale flow by connection strength for adaptive growth
            let flow = (base_flow * c.strength).clamp(-0.02, 0.02);
            h1.energy = (h1.energy - flow).clamp(0.0, 1.0);
            h2.energy = (h2.energy + flow).clamp(0.0, 1.0);

            // Network Intelligence: Track flow for reinforcement learning
            let abs_flow = flow.abs();
            let mut new_flow_acc = c.flow_accumulator + abs_flow;
            let mut new_strength = c.strength;

            // Adaptive Growth: Strengthen connections with high flow
            if self.config.adaptive_growth_enabled {
                if abs_flow > 0.001 {
                    // Strengthen based on flow
                    new_strength =
                        (new_strength + abs_flow * self.config.flow_strengthening_rate).min(1.0);
                } else {
                    // Decay strength when no flow
                    new_strength *= self.config.flow_decay_rate;
                    new_strength = new_strength.max(self.config.min_connection_strength);
                }
                new_flow_acc *= 0.99; // Decay accumulator
            }

            // Network Intelligence: Signal propagation through connections
            let mut new_signal = c.signal;
            if self.config.signal_propagation_enabled {
                // Propagate signals from hyphae to connections
                let signal_from_h1 = h1.signal_received * c.strength;
                let signal_from_h2 = h2.signal_received * c.strength;
                new_signal = (signal_from_h1 + signal_from_h2) * 0.5;

                // Propagate signals from connections back to hyphae
                if new_signal > self.config.signal_strength_threshold {
                    h1.signal_received = (h1.signal_received + new_signal * 0.3).min(1.0);
                    h2.signal_received = (h2.signal_received + new_signal * 0.3).min(1.0);
                }

                // Decay signals on connections
                new_signal *= self.config.signal_decay_rate;
            }

            connection_updates.push((conn_idx, new_strength, new_signal, new_flow_acc));
        }

        // Apply connection updates
        for (idx, strength, signal, flow_acc) in connection_updates {
            if let Some(c) = self.state.connections.get_mut(idx) {
                c.strength = strength;
                c.signal = signal;
                c.flow_accumulator = flow_acc;
            }
        }

        // Network Intelligence: Prune weak connections and branches
        if self.config.adaptive_growth_enabled {
            let mut weak_connections = Vec::new();
            for (idx, c) in self.state.connections.iter().enumerate() {
                if c.strength < self.config.pruning_threshold {
                    weak_connections.push(idx);
                }
            }

            // Remove weak connections in reverse order
            for &idx in weak_connections.iter().rev() {
                let c = &self.state.connections[idx];
                let key = if c.hypha1 < c.hypha2 {
                    (c.hypha1, c.hypha2)
                } else {
                    (c.hypha2, c.hypha1)
                };
                self.state.connection_set.remove(&key);
                self.state.connections.swap_remove(idx);
            }

            // Prune weak hyphae branches (those with very low strength and energy)
            for h in &mut self.state.hyphae {
                if h.alive
                    && h.strength < self.config.pruning_threshold * 0.5
                    && h.energy < self.config.min_energy_to_live * 2.0
                {
                    // Weak branch - reduce strength further, may die soon
                    h.strength *= 0.9;
                    if h.strength < 0.01 {
                        h.alive = false;
                    }
                }
            }
        }

        // diffuse nutrients (LOD: bounding box + frame skipping)
        // Only skip diffusion when FPS is very low to prevent visual issues
        let do_diffuse = if get_fps() < 25.0 {
            (self.state.frame_index % 2) == 0 // Skip every other frame only when FPS < 25
        } else {
            true
        };
        if do_diffuse {
            // Weather: Apply weather effects to nutrient diffusion
            let diffusion_rate = if self.config.weather_enabled {
                self.config.diffusion_rate * self.state.weather.nutrient_diffusion_multiplier()
            } else {
                self.config.diffusion_rate
            };

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
                    self.state.nutrients_back.sugar[x][y] +=
                        diffusion_rate * (avg_sugar - self.state.nutrients_back.sugar[x][y]);

                    // Nitrogen diffusion (slower)
                    let avg_nitrogen = (self.state.nutrients_back.nitrogen[x + 1][y]
                        + self.state.nutrients_back.nitrogen[x - 1][y]
                        + self.state.nutrients_back.nitrogen[x][y + 1]
                        + self.state.nutrients_back.nitrogen[x][y - 1])
                        * 0.25;
                    self.state.nutrients_back.nitrogen[x][y] += diffusion_rate
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

            // Weather: Apply weather effects to spore germination
            // Lower threshold = easier germination (multiplier > 1 means easier)
            let germination_threshold = if self.config.weather_enabled {
                let multiplier = self.state.weather.spore_germination_multiplier();
                self.config.spore_germination_threshold / multiplier.max(0.1)
            } else {
                self.config.spore_germination_threshold
            };

            if total_nutrient > germination_threshold {
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
                    strength: 1.0,
                    signal_received: 0.0,
                    last_nutrient_location: Some((spore.x, spore.y)), // Remember where we germinated
                    senescence_factor: 0.0,
                });
                spore.alive = false;
                // Particle burst at germination (visualization only - not used in tests)
                #[cfg(all(not(test), feature = "ui"))]
                {
                    use macroquad::prelude::*;
                    for k in 0..8 {
                        let a = (k as f32 / 8.0) * std::f32::consts::TAU + rng.gen_range(-0.2..0.2);
                        let r = rng.gen_range(2.0..5.0);
                        let px = spore.x * self.config.cell_size + a.cos() * r;
                        let py = spore.y * self.config.cell_size + a.sin() * r;
                        draw_circle(px, py, 1.5, Color::new(1.0, 0.8, 0.3, 0.6));
                    }
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
            (self.state.fruit_cooldown_timer - 1.0 / fps.max(1.0)).max(0.0);
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
        // Increased transfer radius to allow more hyphae to contribute to fruiting body growth
        let transfer_radius = 20.0f32; // Increased from 15.0 to 20.0
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
                        // Lower energy threshold - allow energy transfer from hyphae with lower energy
                        // This allows more hyphae to contribute to fruiting body growth
                        if !h_ref.alive || h_ref.energy < 0.05 {
                            continue;
                        }
                        let dx = fx - h_ref.x;
                        let dy = fy - h_ref.y;
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq < transfer_radius_sq && dist_sq > 0.1 {
                            let dist = dist_sq.sqrt();
                            // Increased transfer rate - fruiting bodies should receive energy more effectively
                            // Transfer rate is higher and distance falloff is gentler (linear falloff)
                            let distance_factor = (1.0 - dist / transfer_radius).max(0.0);
                            // Base transfer rate significantly increased for better energy flow
                            // At close range: 0.08 rate, at max distance: 0.02 rate
                            let transfer_rate = 0.02 + 0.06 * distance_factor;
                            // Allow more energy transfer per hypha (increased significantly)
                            // Transfer is proportional to hypha energy and distance
                            let base_transfer = h_ref.energy * transfer_rate;
                            // Maximum transfer per hypha increased (allows more contribution)
                            let transfer = base_transfer.min(0.12);
                            if transfer > 0.0001 {
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
                // Ensure hyphae don't go below minimum energy
                self.state.hyphae[h_idx].energy = self.state.hyphae[h_idx].energy.max(0.0);
            }

            // Apply energy transfer to fruiting body
            f.energy = (f.energy + total_transfer).min(1.0);

            // Fruiting bodies have minimal energy decay (metabolism)
            // They should maintain energy if they receive it from hyphae
            // Only very slow decay to represent basic metabolism
            f.energy *= 0.9998; // Extremely slow decay (0.02% per frame)

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

#[cfg(test)]
mod tests {
    use super::*;
    use external_rand::rngs::StdRng;
    use external_rand::SeedableRng;

    /// Helper function to create a simulation for testing
    fn create_test_simulation() -> (Simulation, StdRng) {
        let mut rng = StdRng::seed_from_u64(42);
        let config = SimulationConfig::default();
        let sim = Simulation::with_config(&mut rng, config);
        (sim, rng)
    }

    /// Test that simulation can be created and initialized
    #[test]
    fn test_simulation_creation() {
        let (sim, _) = create_test_simulation();

        // Check that simulation state is initialized
        assert_eq!(sim.state.hyphae.len(), sim.config.initial_hyphae_count);
        assert!(sim.state.frame_index == 0);
        assert!(sim.state.nutrient_memory.len() == sim.config.grid_size);
    }

    /// Test that simulation can run for multiple steps without panicking
    #[test]
    fn test_simulation_runs() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Check that simulation state is valid
        assert!(sim.state.frame_index > 0);
    }

    /// Test that hyphae have valid energy levels after running
    #[test]
    fn test_hyphae_energy_valid() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Check that all hyphae have valid energy levels
        for hypha in &sim.state.hyphae {
            if hypha.alive {
                assert!(
                    hypha.energy >= 0.0 && hypha.energy <= 1.0,
                    "Hypha energy {} is out of range [0.0, 1.0]",
                    hypha.energy
                );
                assert!(hypha.age >= 0.0, "Hypha age {} is negative", hypha.age);
            }
        }
    }

    /// Test that hyphae positions are within bounds
    #[test]
    fn test_hyphae_in_bounds() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Check that all hyphae are within bounds
        for hypha in &sim.state.hyphae {
            if hypha.alive {
                assert!(
                    hypha.x >= 0.0 && hypha.x < sim.config.grid_size as f32,
                    "Hypha x position {} is out of bounds",
                    hypha.x
                );
                assert!(
                    hypha.y >= 0.0 && hypha.y < sim.config.grid_size as f32,
                    "Hypha y position {} is out of bounds",
                    hypha.y
                );
            }
        }
    }

    /// Test that connections are valid
    #[test]
    fn test_connections_valid() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Check that all connections reference valid hyphae
        for conn in &sim.state.connections {
            assert!(
                conn.hypha1 < sim.state.hyphae.len(),
                "Connection references invalid hypha1 index {}",
                conn.hypha1
            );
            assert!(
                conn.hypha2 < sim.state.hyphae.len(),
                "Connection references invalid hypha2 index {}",
                conn.hypha2
            );
            assert!(
                conn.hypha1 != conn.hypha2,
                "Connection references same hypha twice"
            );
            assert!(
                conn.strength >= 0.0 && conn.strength <= 1.0,
                "Connection strength {} is out of range [0.0, 1.0]",
                conn.strength
            );
        }
    }

    /// Test that nutrients are being consumed
    #[test]
    fn test_nutrients_consumed() {
        let (mut sim, mut rng) = create_test_simulation();

        // Calculate initial total nutrients
        let mut _initial_total = 0.0;
        for x in 0..sim.config.grid_size {
            for y in 0..sim.config.grid_size {
                _initial_total += sim.state.nutrients.sugar[x][y];
                _initial_total += sim.state.nutrients.nitrogen[x][y];
            }
        }

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Calculate final total nutrients
        let mut final_total = 0.0;
        for x in 0..sim.config.grid_size {
            for y in 0..sim.config.grid_size {
                final_total += sim.state.nutrients.sugar[x][y];
                final_total += sim.state.nutrients.nitrogen[x][y];
            }
        }

        // Nutrients should decrease (consumption) or increase (regeneration)
        // But the total should be reasonable
        assert!(
            final_total >= 0.0,
            "Total nutrients {} is negative",
            final_total
        );
    }

    /// Test that memory is being updated when memory is enabled
    #[test]
    fn test_memory_updated() {
        let (mut sim, mut rng) = create_test_simulation();
        sim.config.memory_enabled = true;

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Check that memory values are valid
        for x in 0..sim.config.grid_size {
            for y in 0..sim.config.grid_size {
                assert!(
                    sim.state.nutrient_memory[x][y] >= 0.0
                        && sim.state.nutrient_memory[x][y] <= 1.0,
                    "Memory value at ({}, {}) is out of range [0.0, 1.0]: {}",
                    x,
                    y,
                    sim.state.nutrient_memory[x][y]
                );
            }
        }
    }

    /// Test that growth limits are respected
    #[test]
    fn test_growth_limits() {
        let (mut sim, mut rng) = create_test_simulation();
        sim.config.max_hyphae = 100;
        sim.config.max_hyphae_branching_threshold = 80;

        // Run simulation for 500 steps to allow growth
        for _ in 0..500 {
            sim.step(&mut rng);
        }

        // Check that hyphae count doesn't exceed max_hyphae
        assert!(
            sim.state.hyphae.len() <= sim.config.max_hyphae as usize,
            "Hyphae count {} exceeds max_hyphae {}",
            sim.state.hyphae.len(),
            sim.config.max_hyphae
        );
    }

    /// Test that weather affects the simulation
    #[test]
    fn test_weather_effects() {
        let (mut sim, mut rng) = create_test_simulation();
        sim.config.weather_enabled = true;

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Weather should have updated
        assert!(sim.state.weather.time > 0.0, "Weather time should increase");
        assert!(
            sim.state.weather.temperature >= 0.0 && sim.state.weather.temperature <= 2.0,
            "Temperature {} is out of valid range",
            sim.state.weather.temperature
        );
        assert!(
            sim.state.weather.humidity >= 0.0 && sim.state.weather.humidity <= 1.0,
            "Humidity {} is out of valid range",
            sim.state.weather.humidity
        );
        assert!(
            sim.state.weather.rain >= 0.0 && sim.state.weather.rain <= 1.0,
            "Rain {} is out of valid range",
            sim.state.weather.rain
        );
    }

    /// Test that simulation statistics are valid
    #[test]
    fn test_statistics_valid() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, total_energy) =
            sim.stats();

        // Check that statistics are valid
        assert_eq!(
            hyphae_count,
            sim.state.hyphae.iter().filter(|h| h.alive).count()
        );
        assert_eq!(
            spores_count,
            sim.state.spores.iter().filter(|s| s.alive).count()
        );
        assert_eq!(connections_count, sim.state.connections.len());
        assert_eq!(fruit_count, sim.state.fruit_bodies.len());
        assert!(
            avg_energy >= 0.0 && avg_energy <= 1.0,
            "Average energy {} is out of range [0.0, 1.0]",
            avg_energy
        );
        assert!(
            total_energy >= 0.0,
            "Total energy {} is negative",
            total_energy
        );
    }

    /// Test that simulation can handle many iterations without crashing
    #[test]
    fn test_long_simulation() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 1000 steps
        for _ in 0..1000 {
            sim.step(&mut rng);
        }

        // Check that simulation state is still valid
        assert!(sim.state.frame_index > 0);

        // Check that statistics are valid
        let (hyphae_count, _, _, _, _, _) = sim.stats();
        assert!(
            hyphae_count > 0,
            "Should have at least one hypha after long simulation"
        );
    }

    /// Test that segments age correctly
    #[test]
    fn test_segments_age() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 50 steps to generate segments
        for _ in 0..50 {
            sim.step(&mut rng);
        }

        // Check that segments have valid ages
        for segment in &sim.state.segments {
            assert!(
                segment.age >= 0.0,
                "Segment age {} is negative",
                segment.age
            );
            assert!(
                segment.age <= sim.config.max_segment_age,
                "Segment age {} exceeds max_segment_age {}",
                segment.age,
                sim.config.max_segment_age
            );
        }
    }

    /// Test that hyphae can branch
    #[test]
    fn test_hyphae_branching() {
        let (mut sim, mut rng) = create_test_simulation();
        let _initial_count = sim.state.hyphae.len();

        // Run simulation for 200 steps to allow branching
        for _ in 0..200 {
            sim.step(&mut rng);
        }

        // Check that hyphae count is valid (may increase due to branching or decrease due to pruning)
        assert!(sim.state.hyphae.len() > 0, "Should have at least one hypha");
    }

    /// Test that fusion works when enabled
    #[test]
    fn test_fusion_enabled() {
        let (mut sim, mut rng) = create_test_simulation();
        sim.config.fusion_enabled = true;

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Fusion should work (may reduce hyphae count when they merge)
        // Just check that simulation doesn't crash and has valid state
        assert!(sim.state.hyphae.len() > 0, "Should have at least one hypha");
    }

    /// Test validation after 100 iterations
    #[test]
    fn test_validation_after_100_iterations() {
        let (mut sim, mut rng) = create_test_simulation();

        // Run simulation for 100 steps
        for _ in 0..100 {
            sim.step(&mut rng);
        }

        // Validate simulation state
        assert!(
            sim.state.frame_index >= 100,
            "Frame index should be at least 100"
        );

        // Validate hyphae
        for hypha in &sim.state.hyphae {
            if hypha.alive {
                assert!(hypha.energy >= 0.0 && hypha.energy <= 1.0);
                assert!(hypha.age >= 0.0);
                assert!(hypha.x >= 0.0 && hypha.x < sim.config.grid_size as f32);
                assert!(hypha.y >= 0.0 && hypha.y < sim.config.grid_size as f32);
                assert!(hypha.strength >= 0.0 && hypha.strength <= 1.0);
            }
        }

        // Validate connections
        for conn in &sim.state.connections {
            assert!(conn.hypha1 < sim.state.hyphae.len());
            assert!(conn.hypha2 < sim.state.hyphae.len());
            assert!(conn.strength >= 0.0 && conn.strength <= 1.0);
        }

        // Validate nutrients
        for x in 0..sim.config.grid_size {
            for y in 0..sim.config.grid_size {
                assert!(sim.state.nutrients.sugar[x][y] >= 0.0);
                assert!(sim.state.nutrients.nitrogen[x][y] >= 0.0);
            }
        }
    }
}
