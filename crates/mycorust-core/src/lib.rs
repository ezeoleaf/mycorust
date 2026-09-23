//! Headless mycelium growth engine.
//!
//! This crate is the reusable core of MycoRust: simulation state, nutrient
//! fields, weather, and network analysis. It has no UI or HTTP dependencies.
//! Interactive visualization lives in `mycorust`; experiments live in `ideas/*`.

pub mod config;
pub mod hypha;
pub mod metrics;
pub mod nutrients;
pub mod simulation;
pub mod spore;
pub mod types;
pub mod weather;

pub use config::SimulationConfig;
pub use metrics::{analyze, nearest_hypha, shortest_path_length, NetworkMetrics};
pub use simulation::{Simulation, SimulationState};
pub use types::{Connection, FruitBody, Segment, Vec2, Zone, ZoneType};
pub use weather::{EnvironmentalStress, Season, Weather};
