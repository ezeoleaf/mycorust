use macroquad::prelude::Vec2;

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
