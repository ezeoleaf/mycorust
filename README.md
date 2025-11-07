## MycoRust — Mycelium Growth Simulation

A mycelium/hyphae growth simulation in Rust using Macroquad. Hyphae follow nutrient gradients, branch, form connections (anastomosis), avoid collisions, and leave fading trails. Tips reflect off the window edges rather than dying.

### Prerequisites

- Rust toolchain (stable) — install via `https://rustup.rs`

### Run

```bash
cargo run
# or for better performance
cargo run --release
```

### Features

- Nutrient field rendered as a brown→green heatmap (with diffusion over time). Realistic organic patch-based distribution with multiple nutrient types (sugar and nitrogen).
- Hyphae growth: gradient following + small random wander.
- Branching with configurable probability.
- Edge reflection with axis-correct bounce and a slight jitter.
- Obstacle avoidance and hyphae-to-hyphae avoidance to reduce overlaps.
- Energy model: tips consume nutrients locally and die when energy is depleted.
- Anastomosis: nearby hyphae connect; simple energy balancing along connections.
- Trails persist and fade over time; old segments are removed to limit memory growth.
- Live statistics overlay (Hyphae, Spores, Connections, Avg Energy, Speed, FPS).
- Minimap overlay showing nutrients and live hyphae positions.
- Pulsing, age-faded anastomosis connection visualization (toggle).
- Adjustable simulation speed (0.1x to 10x) with visual feedback.
- Performance optimizations: spatial hashing, FPS-based draw decimation, LOD diffusion.
- Scientific tweaks: Sobel chemotaxis and global tropism bias.

### Controls

- SPACE: Pause/Resume
- R: Reset simulation
- C: Clear trails
- X: Toggle connections visibility
- M: Toggle minimap visibility
- H: Toggle hyphae visibility
- S: Spawn a new hypha at mouse position
- N: Add a sugar patch at mouse position (small radius)
- T: Add a nitrogen patch at mouse position
- Left Mouse Button: Add a single sugar cell at mouse
- Right Mouse Button: Add a single nitrogen cell at mouse
- Left Arrow (←): Decrease simulation speed
- Right Arrow (→): Increase simulation speed
- 0: Reset speed to 1x

Controls help is shown at the bottom of the screen.

### Configuration

The simulation uses a `SimulationConfig` struct for all parameters, located in `src/config.rs`. You can customize the simulation by creating a custom config:

```rust
use mycorust::config::SimulationConfig;

let mut config = SimulationConfig::default();
config.grid_size = 300;  // Larger grid
config.branch_prob = 0.005;  // More branching
// ... customize other parameters

let sim = Simulation::with_config(&mut rng, config);
```

### Key parameters (in `SimulationConfig`)

- **GRID/Display**
  - `grid_size: usize` — logical grid resolution; window size = `grid_size * cell_size` (default: 200)
  - `cell_size: f32` — pixels per grid cell (default: 4.0)

- **Growth & branching**
  - `branch_prob: f32` — branching probability per step (default: 0.002)
  - `step_size: f32` — movement step length per frame (default: 0.5)
  - `angle_wander_range: f32` — random wander added to direction each frame (default: 0.05)
  - `gradient_steering_strength: f32` — steering toward nutrient gradient (default: 0.1)

- **Nutrients**
  - `nutrient_decay: f32` — max nutrient absorbed per step at a tip (default: 0.01)
  - `diffusion_rate: f32` — how quickly nutrients diffuse across neighbors (default: 0.05)
  - `spore_germination_threshold: f32` — nutrient threshold for spores to germinate (default: 0.6)
  - `spore_max_age: f32` — maximum age before spores die (default: 5.0)
  - `tropism_angle: f32`, `tropism_strength: f32` — global tropism bias (default: π/4, 0.01)

- **Energy**
  - `energy_decay_rate: f32` — passive energy decay per step (default: 0.999)
  - `min_energy_to_live: f32` — below this, hyphae die (default: 0.01)

- **Networking**
  - `anastomosis_distance: f32` — distance within which hyphae connect (default: 2.0)
  - `connection_flow_rate: f32` — diffusive energy flow along connections (default: 0.02)

- **Avoidance**
  - `hyphae_avoidance_distance: f32` — hyphae turn away if the next step is closer than this (default: 2.0)

- **Trails**
  - `max_segment_age: f32` — maximum age before segments are removed (default: 10.0)
  - `segment_age_increment: f32` — how fast segments age per frame (default: 0.01)

- **Fruiting**
  - `fruiting_min_hyphae: usize` — minimum hyphae count to spawn fruiting bodies (default: 50)
  - `fruiting_threshold_total_energy: f32` — minimum total energy for fruiting (default: 15.0)
  - `fruiting_cooldown: f32` — cooldown between fruiting body spawns (default: 10.0)

- **Initialization**
  - `initial_hyphae_count: usize` — number of hyphae at simulation start (default: 5)
  - `obstacle_count: usize` — number of obstacles in the grid (default: 300)

You can tweak these parameters to change speed, density, network behavior, and look of the simulation. Larger `grid_size` with `--release` gives smoother visuals, but uses more CPU/GPU.

### Notes

- Reflection flips direction on the axis of impact and adds jitter to avoid sticking.
- Nutrients only decay via local consumption by hyphae; a diffusion step spreads nutrients.
- Obstacles can be enabled in the grid; tips reflect and re-steer when colliding.

### Architecture

The simulation is organized into several modules:

- **`simulation.rs`** — Core simulation logic with `Simulation`, `SimulationState`, and `SimulationConfig` structs
- **`config.rs`** — Configuration struct with all simulation parameters
- **`hypha.rs`** — Hypha struct and behavior
- **`spore.rs`** — Spore struct and behavior
- **`nutrients.rs`** — Nutrient grid and gradient calculations
- **`visualization.rs`** — All drawing functions
- **`controls.rs`** — Input handling
- **`types.rs`** — Shared types (Connection, Segment, FruitBody)

The `Simulation` struct contains:
- `state: SimulationState` — All mutable simulation data (nutrients, hyphae, spores, etc.)
- `config: SimulationConfig` — All configuration parameters
- Control flags (`paused`, `connections_visible`, `minimap_visible`)

### Troubleshooting

- If the window is too large, lower `grid_size` or `cell_size` in the config.
- If performance is low, use `cargo run --release` and/or reduce `grid_size`. You can also toggle connections (X) or minimap (M), or lower `max_segment_age`.
- If trails overwhelm the scene, reduce `max_segment_age` or increase `segment_age_increment` in the config.

### License

MIT — see `LICENSE` if present, or adapt for personal use.
