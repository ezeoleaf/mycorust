// Global configuration and constants

// Grid/display
pub const GRID_SIZE: usize = 200;
pub const CELL_SIZE: f32 = 4.0;

// Growth & branching
pub const BRANCH_PROB: f32 = 0.002;
pub const STEP_SIZE: f32 = 0.5;
pub const GRADIENT_STEERING_STRENGTH: f32 = 0.1;
pub const ANGLE_WANDER_RANGE: f32 = 0.05;

// Nutrients
pub const NUTRIENT_DECAY: f32 = 0.01;
pub const DIFFUSION_RATE: f32 = 0.05;
pub const SPORE_GERMINATION_THRESHOLD: f32 = 0.6;
pub const SPORE_MAX_AGE: f32 = 5.0;

// Obstacles
pub const OBSTACLE_COUNT: usize = 300;

// Energy
pub const ENERGY_DECAY_RATE: f32 = 0.999;
pub const MIN_ENERGY_TO_LIVE: f32 = 0.01;

// Anastomosis
pub const ANASTOMOSIS_DISTANCE: f32 = 2.0;
pub const ANASTOMOSIS_DISTANCE_SQ: f32 = ANASTOMOSIS_DISTANCE * ANASTOMOSIS_DISTANCE;

// Hyphae avoidance/density
pub const HYPHAE_AVOIDANCE_DISTANCE: f32 = 2.0;
pub const HYPHAE_AVOIDANCE_DISTANCE_SQ: f32 = HYPHAE_AVOIDANCE_DISTANCE * HYPHAE_AVOIDANCE_DISTANCE;

// Segments/trails
pub const MAX_SEGMENT_AGE: f32 = 10.0;
pub const SEGMENT_AGE_INCREMENT: f32 = 0.01;

// Fruiting
pub struct FruitingConfig;
impl FruitingConfig {
    pub const MIN_HYPHAE: usize = 50;
    pub const THRESHOLD_TOTAL_ENERGY: f32 = 15.0;
    pub const COOLDOWN: f32 = 10.0;
}


