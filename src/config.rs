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

    // Initialization
    pub initial_hyphae_count: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            grid_size: 200,
            cell_size: 4.0,
            branch_prob: 0.002,
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
            fruiting_min_hyphae: 50,
            fruiting_threshold_total_energy: 15.0,
            fruiting_cooldown: 10.0,
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
