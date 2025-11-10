use macroquad::prelude::Vec2;

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
