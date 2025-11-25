#[cfg(all(not(test), feature = "ui"))]
use macroquad::prelude::Vec2;

#[cfg(any(test, not(feature = "ui")))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[cfg(any(test, not(feature = "ui")))]
impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub struct Connection {
    pub hypha1: usize,
    pub hypha2: usize,
    // Network intelligence: track connection strength and signals
    pub strength: f32,         // Connection strength (0.0-1.0), increases with flow
    pub signal: f32,           // Current signal strength propagating through
    pub flow_accumulator: f32, // Accumulated nutrient flow for reinforcement learning
}

pub struct Segment {
    pub from: Vec2,
    pub to: Vec2,
    pub age: f32,
}

pub struct FruitBody {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub energy: f32,
    pub lifespan: f32,
    pub released_spores: bool,
    pub next_spore_release_age: f32,
}

// Zone types for contaminants/competitors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoneType {
    None,           // No zone
    Toxic,          // Toxic zone - harms hyphae
    Competitor,     // Competitor zone - consumes nutrients (like Trichoderma)
    Deadwood,       // Deadwood patch - nutrient-rich but may have mild effects
}

#[derive(Clone)]
pub struct Zone {
    pub zone_type: ZoneType,
    pub intensity: f32, // Intensity/strength of the zone (0.0-1.0)
    pub age: f32,       // Age of the zone (for growth/decay)
}
