#[derive(Clone)]
pub struct Hypha {
    pub x: f32,
    pub y: f32,
    pub prev_x: f32,
    pub prev_y: f32,
    pub angle: f32,
    pub alive: bool,
    pub energy: f32,
    pub parent: Option<usize>,
    pub age: f32,
}


