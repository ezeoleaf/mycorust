#[cfg(feature = "gui")]
use macroquad::prelude::Vec2;

// Vec2 replacement for TUI mode
#[cfg(not(feature = "gui"))]
#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[cfg(not(feature = "gui"))]
impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

pub struct Connection {
    pub hypha1: usize,
    pub hypha2: usize,
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
}
