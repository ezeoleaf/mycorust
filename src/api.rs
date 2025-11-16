// API module for headless mode - HTTP endpoints to interact with the simulation

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

use crate::config::SimulationConfig;
use crate::simulation::Simulation;
use ::rand::rngs::StdRng;
use ::rand::SeedableRng;

// Serializable versions of simulation data for API responses
#[derive(Serialize, Clone)]
pub struct HyphaData {
    pub x: f32,
    pub y: f32,
    pub prev_x: f32,
    pub prev_y: f32,
    pub angle: f32,
    pub alive: bool,
    pub energy: f32,
    pub parent: Option<usize>,
    pub age: f32,
    pub strength: f32,
}

#[derive(Serialize, Clone)]
pub struct SporeData {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub alive: bool,
    pub age: f32,
}

#[derive(Serialize, Clone)]
pub struct ConnectionData {
    pub hypha1: usize,
    pub hypha2: usize,
    pub strength: f32,
    pub signal: f32,
}

#[derive(Serialize, Clone)]
pub struct SegmentData {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub age: f32,
}

#[derive(Serialize, Clone)]
pub struct FruitBodyData {
    pub x: f32,
    pub y: f32,
    pub age: f32,
    pub energy: f32,
    pub lifespan: f32,
    pub released_spores: bool,
}

#[derive(Serialize, Clone)]
pub struct WeatherData {
    pub temperature: f32,
    pub humidity: f32,
    pub rain: f32,
    pub temperature_celsius: f32,
    pub growth_multiplier: f32,
}

#[derive(Serialize, Clone)]
pub struct StatsData {
    pub hyphae_count: usize,
    pub spores_count: usize,
    pub connections_count: usize,
    pub fruit_count: usize,
    pub avg_energy: f32,
    pub total_energy: f32,
    pub frame_index: u64,
}

#[derive(Serialize, Clone)]
pub struct SimulationStateResponse {
    pub hyphae: Vec<HyphaData>,
    pub spores: Vec<SporeData>,
    pub connections: Vec<ConnectionData>,
    pub segments: Vec<SegmentData>,
    pub fruit_bodies: Vec<FruitBodyData>,
    pub nutrients: NutrientGridData,
    pub nutrient_memory: Vec<Vec<f32>>,
    pub obstacles: Vec<Vec<bool>>,
    pub weather: WeatherData,
    pub stats: StatsData,
}

#[derive(Serialize, Clone)]
pub struct NutrientGridData {
    pub sugar: Vec<Vec<f32>>,
    pub nitrogen: Vec<Vec<f32>>,
}

#[derive(Deserialize)]
pub struct StepQuery {
    pub steps: Option<usize>,
}

// Shared state for the API server
#[derive(Clone)]
pub struct ApiState {
    pub simulation: Arc<Mutex<Simulation>>,
    pub rng: Arc<Mutex<StdRng>>,
}

impl ApiState {
    pub fn new(sim: Simulation) -> Self {
        let rng = StdRng::from_entropy();
        Self {
            simulation: Arc::new(Mutex::new(sim)),
            rng: Arc::new(Mutex::new(rng)),
        }
    }
}

// Helper function to convert simulation state to API response
fn simulation_to_response(sim: &Simulation) -> SimulationStateResponse {
    let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, total_energy) =
        sim.stats();

    SimulationStateResponse {
        hyphae: sim
            .state
            .hyphae
            .iter()
            .map(|h| HyphaData {
                x: h.x,
                y: h.y,
                prev_x: h.prev_x,
                prev_y: h.prev_y,
                angle: h.angle,
                alive: h.alive,
                energy: h.energy,
                parent: h.parent,
                age: h.age,
                strength: h.strength,
            })
            .collect(),
        spores: sim
            .state
            .spores
            .iter()
            .map(|s| SporeData {
                x: s.x,
                y: s.y,
                vx: s.vx,
                vy: s.vy,
                alive: s.alive,
                age: s.age,
            })
            .collect(),
        connections: sim
            .state
            .connections
            .iter()
            .map(|c| ConnectionData {
                hypha1: c.hypha1,
                hypha2: c.hypha2,
                strength: c.strength,
                signal: c.signal,
            })
            .collect(),
        segments: sim
            .state
            .segments
            .iter()
            .map(|s| SegmentData {
                from_x: s.from.x,
                from_y: s.from.y,
                to_x: s.to.x,
                to_y: s.to.y,
                age: s.age,
            })
            .collect(),
        fruit_bodies: sim
            .state
            .fruit_bodies
            .iter()
            .map(|f| FruitBodyData {
                x: f.x,
                y: f.y,
                age: f.age,
                energy: f.energy,
                lifespan: f.lifespan,
                released_spores: f.released_spores,
            })
            .collect(),
        nutrients: NutrientGridData {
            sugar: sim.state.nutrients.sugar.clone(),
            nitrogen: sim.state.nutrients.nitrogen.clone(),
        },
        nutrient_memory: sim.state.nutrient_memory.clone(),
        obstacles: sim.state.obstacles.clone(),
        weather: WeatherData {
            temperature: sim.state.weather.temperature,
            humidity: sim.state.weather.humidity,
            rain: sim.state.weather.rain,
            temperature_celsius: sim.state.weather.temperature_celsius_approx(),
            growth_multiplier: sim.state.weather.growth_multiplier(),
        },
        stats: StatsData {
            hyphae_count,
            spores_count,
            connections_count,
            fruit_count,
            avg_energy,
            total_energy,
            frame_index: sim.state.frame_index,
        },
    }
}

// GET /state - Get current simulation state
async fn get_state(
    State(api_state): State<ApiState>,
) -> Result<Json<SimulationStateResponse>, StatusCode> {
    let sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(simulation_to_response(&sim)))
}

// GET /stats - Get simulation statistics
async fn get_stats(State(api_state): State<ApiState>) -> Result<Json<StatsData>, StatusCode> {
    let sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, total_energy) =
        sim.stats();
    Ok(Json(StatsData {
        hyphae_count,
        spores_count,
        connections_count,
        fruit_count,
        avg_energy,
        total_energy,
        frame_index: sim.state.frame_index,
    }))
}

// POST /step - Step the simulation forward
async fn step_simulation(
    Query(params): Query<StepQuery>,
    State(api_state): State<ApiState>,
) -> Result<Json<SimulationStateResponse>, StatusCode> {
    let mut sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rng = api_state
        .rng
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let steps = params.steps.unwrap_or(1);

    for _ in 0..steps {
        sim.step(&mut *rng);
    }

    Ok(Json(simulation_to_response(&sim)))
}

// POST /reset - Reset the simulation
async fn reset_simulation(
    State(api_state): State<ApiState>,
) -> Result<Json<SimulationStateResponse>, StatusCode> {
    let mut sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rng = api_state
        .rng
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sim.reset(&mut *rng);

    Ok(Json(simulation_to_response(&sim)))
}

// POST /pause - Toggle pause
async fn pause_simulation(
    State(api_state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sim.toggle_pause();
    Ok(Json(serde_json::json!({ "paused": sim.paused })))
}

// GET /config - Get simulation configuration
async fn get_config(
    State(api_state): State<ApiState>,
) -> Result<Json<SimulationConfig>, StatusCode> {
    let sim = api_state
        .simulation
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sim.config.clone()))
}

// Create the API router
pub fn create_router(api_state: ApiState) -> Router {
    Router::new()
        .route("/state", get(get_state))
        .route("/stats", get(get_stats))
        .route("/step", post(step_simulation))
        .route("/reset", post(reset_simulation))
        .route("/pause", post(pause_simulation))
        .route("/config", get(get_config))
        .layer(CorsLayer::permissive())
        .with_state(api_state)
}

// Run the API server with automatic simulation stepping
pub async fn run_server(api_state: ApiState, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(api_state.clone());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    println!(
        "MycoRust headless API server running on http://localhost:{}",
        port
    );
    println!("Endpoints:");
    println!("  GET  /state  - Get full simulation state");
    println!("  GET  /stats  - Get simulation statistics");
    println!("  POST /step?steps=N - Step simulation N times (default: 1)");
    println!("  POST /reset - Reset simulation");
    println!("  POST /pause - Toggle pause");
    println!("  GET  /config - Get simulation configuration");
    println!();
    println!("Simulation is running automatically at ~60 FPS (respects pause state)");

    // Spawn background task to continuously step the simulation
    let simulation_task = tokio::spawn(simulation_loop(api_state.clone()));

    // Run the server
    let server_handle = tokio::spawn(async move { axum::serve(listener, app).await });

    // Wait for either task to complete
    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = simulation_task => {
            eprintln!("Simulation loop ended unexpectedly");
        }
    }

    Ok(())
}

// Background task that continuously steps the simulation
async fn simulation_loop(api_state: ApiState) {
    // Target FPS for headless mode (similar to UI mode)
    const TARGET_FPS: f32 = 60.0;
    let frame_duration = std::time::Duration::from_secs_f32(1.0 / TARGET_FPS);

    loop {
        let start = std::time::Instant::now();

        // Step simulation if not paused
        {
            let mut sim = match api_state.simulation.lock() {
                Ok(sim) => sim,
                Err(_) => break,
            };

            if !sim.paused {
                let mut rng = match api_state.rng.lock() {
                    Ok(rng) => rng,
                    Err(_) => break,
                };

                // Handle speed multiplier with accumulator (same as UI mode)
                sim.speed_accumulator += sim.speed_multiplier;
                let steps = sim.speed_accumulator.floor() as usize;
                sim.speed_accumulator -= steps as f32;

                for _ in 0..steps {
                    sim.step(&mut *rng);
                }
            }
        }

        // Sleep to maintain target FPS
        let elapsed = start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        }
    }
}
