// Global configuration and constants

// Configuration struct for simulation parameters
#[derive(Clone, Debug)]
pub struct SimulationConfig {
    // Grid/display
    pub grid_size: usize,
    pub cell_size: f32,

    // Growth & branching
    pub branch_prob: f32,
    pub step_size: f32,
    pub gradient_steering_strength: f32,
    pub angle_wander_range: f32,

    // Nutrients
    pub nutrient_decay: f32,
    pub diffusion_rate: f32,
    pub spore_germination_threshold: f32,
    pub spore_max_age: f32,

    // Chemotaxis/tropism
    pub tropism_angle: f32,
    pub tropism_strength: f32,

    // Obstacles
    pub obstacle_count: usize,

    // Energy
    pub energy_decay_rate: f32,
    pub min_energy_to_live: f32,

    // Anastomosis
    pub anastomosis_distance: f32,
    pub connection_flow_rate: f32,

    // Hyphae avoidance/density
    pub hyphae_avoidance_distance: f32,

    // Segments/trails
    pub max_segment_age: f32,
    pub segment_age_increment: f32,

    // Fruiting
    pub fruiting_min_hyphae: usize,
    pub fruiting_threshold_total_energy: f32,
    pub fruiting_cooldown: f32,
    pub fruiting_lifespan_min: f32,
    pub fruiting_lifespan_max: f32,
    pub fruiting_spore_release_fraction: f32,
    pub fruiting_spore_count: usize,
    pub fruiting_spore_drift: f32,
    pub fruiting_spore_radius: f32,
    pub fruiting_spawn_nutrient_threshold: f32,
    pub fruiting_nutrient_return_fraction: f32,
    pub fruiting_spore_release_interval: f32,
    pub fruiting_fallback_threshold: f32,
    pub fruiting_failed_attempts_before_fallback: u32,

    // Nutrient regeneration
    pub nutrient_regen_rate: f32,
    pub nutrient_regen_floor: f32,
    pub nutrient_regen_samples: usize,

    // Initialization
    pub initial_hyphae_count: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            grid_size: 200,
            cell_size: 4.0,
            branch_prob: 0.0035,
            step_size: 0.5,
            gradient_steering_strength: 0.1,
            angle_wander_range: 0.05,
            nutrient_decay: 0.01,
            diffusion_rate: 0.05,
            spore_germination_threshold: 0.6,
            spore_max_age: 5.0,
            tropism_angle: std::f32::consts::FRAC_PI_4,
            tropism_strength: 0.01,
            obstacle_count: 300,
            energy_decay_rate: 0.999,
            min_energy_to_live: 0.01,
            anastomosis_distance: 2.0,
            connection_flow_rate: 0.02,
            hyphae_avoidance_distance: 2.0,
            max_segment_age: 10.0,
            segment_age_increment: 0.01,
            fruiting_min_hyphae: 12,
            fruiting_threshold_total_energy: 6.0,
            fruiting_cooldown: 10.0,
            fruiting_lifespan_min: 12.0,
            fruiting_lifespan_max: 20.0,
            fruiting_spore_release_fraction: 0.6,
            fruiting_spore_count: 6,
            fruiting_spore_drift: 0.6,
            fruiting_spore_radius: 9.0,
            fruiting_spawn_nutrient_threshold: 0.38,
            fruiting_nutrient_return_fraction: 0.25,
            fruiting_spore_release_interval: 0.15,
            fruiting_fallback_threshold: 0.2,
            fruiting_failed_attempts_before_fallback: 3,
            nutrient_regen_rate: 0.004,
            nutrient_regen_floor: 0.12,
            nutrient_regen_samples: 120,
            initial_hyphae_count: 5,
        }
    }
}

impl SimulationConfig {
    pub fn anastomosis_distance_sq(&self) -> f32 {
        self.anastomosis_distance * self.anastomosis_distance
    }

    pub fn hyphae_avoidance_distance_sq(&self) -> f32 {
        self.hyphae_avoidance_distance * self.hyphae_avoidance_distance
    }
}

pub const GRID_SIZE: usize = 200;
pub const CELL_SIZE: f32 = 4.0;
