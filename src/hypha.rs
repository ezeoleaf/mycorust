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
    // Network intelligence: adaptive growth
    pub strength: f32,        // Branch strength (affects growth rate)
    pub signal_received: f32, // Accumulated signals received
    pub last_nutrient_location: Option<(f32, f32)>, // Memory of last nutrient location
}
