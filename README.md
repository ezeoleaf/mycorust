## MycoRust — Mycelium Growth Simulation

A mycelium/hyphae growth simulation in Rust. Hyphae follow nutrient gradients, branch, form connections (anastomosis), avoid collisions, and leave fading trails. Features advanced network intelligence, weather simulation, adaptive growth, and memory systems.

**Two modes available:**
- **UI Mode**: Interactive visualization using Macroquad (default)
- **Headless Mode**: HTTP API server for integration with custom visualization tools

### Prerequisites

- Rust toolchain (stable) — install via `https://rustup.rs`

### Run

#### UI Mode (Default)
```bash
cargo run
# or for better performance
cargo run --release
```

#### Headless Mode (HTTP API Server)
Run the simulation without a UI, exposing an HTTP API to access simulation state:

```bash
# Headless mode (no UI dependencies)
cargo run --no-default-features -- --headless --port 8080

# Headless mode with UI features compiled (but not used)
cargo run --features ui -- --headless --port 8080

# Default port is 8080 if not specified
cargo run --no-default-features -- --headless
```

The simulation runs automatically at ~60 FPS in the background. You can access the current state via HTTP endpoints (see [Headless Mode & API](#headless-mode--api) section below).

### Run Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_simulation_creation

# Run tests with output
cargo test -- --nocapture
```

The simulation can be tested without requiring a graphics context, making it CI/CD friendly.

### Features

#### Core Simulation
- Nutrient field rendered as a brown→green heatmap (with diffusion over time). Realistic organic patch-based distribution with multiple nutrient types (sugar and nitrogen).
- **Directional Nutrient Flow**: Water flow field that drags nutrients in a specific direction, creating anisotropic diffusion. Flow strength and direction are configurable, and rain increases flow strength. This produces beautiful emergent branching behavior as hyphae follow nutrient gradients that are shaped by water flow.
- Hyphae growth: gradient following + small random wander.
- Branching with configurable probability.
- Edge reflection with axis-correct bounce and a slight jitter.
- Obstacle avoidance and hyphae-to-hyphae avoidance to reduce overlaps.
- Energy model: tips consume nutrients locally and die when energy is depleted.
- **Hyphal Senescence & Death**: Biological aging system where hyphae die based on:
  - Low nutrient flow through connections
  - Distance from main network (unsupported branches collapse)
  - Extreme weather conditions (too hot/cold)
  - Visual decay: brown/grey coloring for dying hyphae
- Anastomosis: nearby hyphae connect; simple energy balancing along connections.
- Trails persist and fade over time; old segments are removed to limit memory growth.
- Live statistics overlay (Hyphae, Spores, Connections, Fruiting Bodies, Avg Energy, Speed, FPS, Weather).
- Minimap overlay showing nutrients and live hyphae positions.
- Pulsing, age-faded anastomosis connection visualization (toggle).
- Adjustable simulation speed (0.1x to 10x) with visual feedback.
- Performance optimizations: spatial hashing, FPS-based draw decimation, adaptive quality reduction.
- Scientific tweaks: Sobel chemotaxis and global tropism bias.

#### Network Intelligence & Computation
- **Signal Propagation**: Internal signaling (electrical/chemical pulses) travels through the mycelium network, triggering behaviors like growth direction changes and resource redistribution. Signals propagate along connections and decay over time.
- **Adaptive Growth**: Reinforces efficient paths and prunes redundant hyphae using reinforcement learning. Branches strengthen with high nutrient flow and weaken/decay with low flow. Weak connections are automatically pruned.
- **Memory & Learning**: Short-term memory of nutrient locations using decaying weights. Memory influences growth direction, allowing hyphae to remember and return to productive areas. Visualized as a purple overlay.

#### Weather Simulation
- **Dynamic Weather System**: Realistic weather conditions affect mycelium growth and behavior:
  - **Temperature**: Affects growth rate and energy consumption
  - **Humidity**: Influences nutrient diffusion and spore germination
  - **Rain**: Impacts overall growth conditions
- Weather conditions change over time, creating dynamic environmental challenges.
- Weather affects growth rate, energy consumption, nutrient diffusion, and spore germination.

#### Growth Management
- **Growth Limits**: Configurable maximum hyphae count to prevent performance degradation.
- **Branching Thresholds**: Stop branching when hyphae count exceeds a threshold.
- **Fusion**: When two hyphae meet very close together, they merge (true biological fusion/anastomosis with merging), combining their energy and taking maximum strength.

#### Visualization & Interaction
- **Enhanced Visualization**: 
  - Age-based coloring (young = white, old = dark)
  - Nutrient flow intensity visualization (green pulses)
  - Environmental stress visualization (red/orange for low energy, white/blue for high energy)
  - **Senescence decay visualization**: Brown/grey coloring for dying hyphae (brown for early decay, grey for advanced decay)
  - Pulsing animations for resource movement and signaling
- **Camera System**: Pan and zoom to explore the network at multiple scales (optional, disabled by default).
- **Screenshot**: Capture high-resolution images of the simulation (P key).
- **Spatial Culling**: Only visible objects are rendered for better performance.

#### Headless Mode & API
- **HTTP API Server**: Run the simulation without a UI, exposing REST endpoints
- **Automatic Simulation**: Simulation runs continuously at ~60 FPS in the background
- **JSON State Access**: Get full simulation state as JSON for custom visualization
- **Control via API**: Step, pause, reset, and query simulation state remotely
- **No Graphics Dependencies**: Headless mode can run without macroquad/OpenGL

#### Testing
- Comprehensive test suite with 15+ validation tests
- Tests run without macroquad (no graphics context required)
- Validates simulation state after multiple iterations
- Tests energy levels, bounds checking, connections, memory, weather, and more

### Controls

#### Basic Controls
- **SPACE**: Pause/Resume (only when not panning)
- **R**: Reset simulation
- **Shift+C**: Clear trails
- **X**: Toggle connections visibility
- **M**: Toggle minimap visibility
- **H**: Toggle hyphae visibility
- **I**: Toggle memory overlay (purple)
- **S**: Spawn a new hypha at mouse position
- **N**: Add a sugar patch at mouse position (small radius)
- **T**: Add a nitrogen patch at mouse position
- **Left Mouse Button**: Add a single sugar cell at mouse
- **Right Mouse Button**: Add a single nitrogen cell at mouse

#### Speed Controls
- **Shift+Left Arrow (←)**: Decrease simulation speed
- **Shift+Right Arrow (→)**: Increase simulation speed
- **0**: Reset speed to 1x

#### Camera Controls (when enabled)
- **Arrow Keys / WASD**: Pan the camera
- **Mouse Wheel**: Zoom in/out (at mouse position)
- **Middle Mouse Button / Space+Left Mouse**: Drag to pan
- **Home**: Reset camera to default position and zoom
- **C**: Toggle camera enabled/disabled
- **P**: Take screenshot (saved as PNG with timestamp)

#### Visualization Controls
- **V**: Toggle enhanced visualization (age/flow/stress coloring)
- **F**: Toggle flow visualization (green pulses)
- **1**: Toggle stress visualization (red/orange for low energy)

Controls help is shown at the bottom of the screen. Press **F1** to toggle the help popup.

### Headless Mode & API

When running in headless mode, the simulation exposes an HTTP API server that allows you to:
- Query the current simulation state
- Control the simulation (step, pause, reset)
- Integrate with custom visualization tools
- Run the simulation on servers without graphics capabilities

#### Starting the Headless Server

```bash
# Run headless mode (default port: 8080)
cargo run --no-default-features -- --headless

# Specify custom port
cargo run --no-default-features -- --headless --port 3000
```

The server will start and print available endpoints. The simulation runs automatically in the background at ~60 FPS.

#### API Endpoints

All endpoints return JSON. CORS is enabled for cross-origin requests.

##### `GET /state`
Get the complete current simulation state.

**Response**: Full `SimulationStateResponse` JSON containing:
- `hyphae`: Array of all hyphae with positions, energy, age, strength, etc.
- `spores`: Array of all spores
- `connections`: Network connections between hyphae
- `segments`: Trail segments (for visualization)
- `fruit_bodies`: Fruiting bodies
- `nutrients`: Sugar and nitrogen grids (2D arrays)
- `nutrient_memory`: Memory grid (2D array)
- `obstacles`: Obstacle grid (2D boolean array)
- `weather`: Current weather conditions (temperature, humidity, rain, multipliers)
- `stats`: Statistics (hyphae count, spores count, connections count, fruit count, avg energy, total energy, frame index)

**Example**:
```bash
curl http://localhost:8080/state | jq '.stats'
```

##### `GET /stats`
Get simulation statistics only (lighter than `/state`).

**Response**: `StatsData` JSON with counts and energy information.

**Example**:
```bash
curl http://localhost:8080/stats
```

##### `POST /step?steps=N`
Manually step the simulation forward N times (default: 1).

**Query Parameters**:
- `steps` (optional): Number of steps to advance (default: 1)

**Response**: Full `SimulationStateResponse` after stepping.

**Example**:
```bash
# Step once
curl -X POST http://localhost:8080/step

# Step 10 times
curl -X POST "http://localhost:8080/step?steps=10"
```

##### `POST /reset`
Reset the simulation to its initial state.

**Response**: Full `SimulationStateResponse` after reset.

**Example**:
```bash
curl -X POST http://localhost:8080/reset
```

##### `POST /pause`
Toggle the pause state of the simulation.

**Response**: JSON with current pause state: `{"paused": true}` or `{"paused": false}`

**Example**:
```bash
curl -X POST http://localhost:8080/pause
```

##### `GET /config`
Get the current simulation configuration.

**Response**: `SimulationConfig` JSON with all parameters.

**Example**:
```bash
curl http://localhost:8080/config | jq '.grid_size'
```

#### Example Usage

```bash
# Start the headless server
cargo run --no-default-features -- --headless

# In another terminal, query the state
curl http://localhost:8080/state | jq '.stats.hyphae_count'

# Get statistics
curl http://localhost:8080/stats

# Pause the simulation
curl -X POST http://localhost:8080/pause

# Step manually (even when paused, manual steps work)
curl -X POST "http://localhost:8080/step?steps=5"

# Resume (unpause)
curl -X POST http://localhost:8080/pause

# Reset the simulation
curl -X POST http://localhost:8080/reset
```

#### Integration Example (Go)

```go
package main

import (
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

type Hypha struct {
    X      float32 `json:"x"`
    Y      float32 `json:"y"`
    Alive  bool    `json:"alive"`
    Energy float32 `json:"energy"`
}

type Stats struct {
    HyphaeCount   int   `json:"hyphae_count"`
    FrameIndex    int64 `json:"frame_index"`
}

type StateResponse struct {
    Hyphae []Hypha `json:"hyphae"`
    Stats  Stats   `json:"stats"`
}

const apiBaseURL = "http://localhost:8080"

func main() {
    // Get current state
    resp, err := http.Get(apiBaseURL + "/state")
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()

    body, err := io.ReadAll(resp.Body)
    if err != nil {
        panic(err)
    }

    var state StateResponse
    if err := json.Unmarshal(body, &state); err != nil {
        panic(err)
    }

    // Access hyphae data
    for _, hypha := range state.Hyphae {
        if hypha.Alive {
            fmt.Printf("Hypha at (%.2f, %.2f) with energy %.3f\n",
                hypha.X, hypha.Y, hypha.Energy)
        }
    }

    // Get statistics
    resp, err = http.Get(apiBaseURL + "/stats")
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()

    body, err = io.ReadAll(resp.Body)
    if err != nil {
        panic(err)
    }

    var stats Stats
    if err := json.Unmarshal(body, &stats); err != nil {
        panic(err)
    }

    fmt.Printf("Active hyphae: %d\n", stats.HyphaeCount)
    fmt.Printf("Frame: %d\n", stats.FrameIndex)

    // Step simulation
    apiURL := apiBaseURL + "/step?steps=10"
    req, err := http.NewRequest("POST", apiURL, nil)
    if err != nil {
        panic(err)
    }

    client := &http.Client{}
    resp, err = client.Do(req)
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()

    fmt.Println("Stepped simulation 10 times")
}
```

#### Integration Example (JavaScript/TypeScript)

```typescript
// Fetch simulation state
const response = await fetch('http://localhost:8080/state');
const state = await response.json();

// Access data
console.log(`Hyphae count: ${state.stats.hyphae_count}`);
console.log(`Frame: ${state.stats.frame_index}`);

// Draw hyphae (example with Canvas API)
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
ctx.clearRect(0, 0, canvas.width, canvas.height);

state.hyphae.forEach(hypha => {
  if (hypha.alive) {
    ctx.fillStyle = `rgba(255, 255, 255, ${hypha.energy})`;
    ctx.fillRect(hypha.x * 4, hypha.y * 4, 2, 2);
  }
});

// Step simulation
await fetch('http://localhost:8080/step?steps=1', { method: 'POST' });
```

### Screenshots

<img width="793" height="797" alt="image" src="https://github.com/user-attachments/assets/7efaf73c-66f8-4376-b319-a34cdd6a5b81" />

<img width="636" height="655" alt="image" src="https://github.com/user-attachments/assets/df870878-a154-47b9-a918-fc552936c246" />

### Configuration

The simulation uses a `SimulationConfig` struct for all parameters, located in `src/config.rs`. You can customize the simulation by creating a custom config:

```rust
use mycorust::config::SimulationConfig;

let mut config = SimulationConfig::default();
config.grid_size = 300;  // Larger grid
config.branch_prob = 0.005;  // More branching
config.camera_enabled = true;  // Enable camera
// ... customize other parameters

let sim = Simulation::with_config(&mut rng, config);
```

### Key parameters (in `SimulationConfig`)

#### Grid/Display
- `grid_size: usize` — logical grid resolution; window size = `grid_size * cell_size` (default: 200)
- `cell_size: f32` — pixels per grid cell (default: 4.0)
- `camera_enabled: bool` — enable camera pan/zoom functionality (default: false)

#### Growth & Branching
- `branch_prob: f32` — branching probability per step (default: 0.008)
- `step_size: f32` — movement step length per frame (default: 0.5)
- `angle_wander_range: f32` — random wander added to direction each frame (default: 0.05)
- `gradient_steering_strength: f32` — steering toward nutrient gradient (default: 0.1)

#### Nutrients
- `nutrient_decay: f32` — max nutrient absorbed per step at a tip (default: 0.01)
- `diffusion_rate: f32` — how quickly nutrients diffuse across neighbors (default: 0.05)
- `spore_germination_threshold: f32` — nutrient threshold for spores to germinate (default: 0.6)
- `spore_max_age: f32` — maximum age before spores die (default: 5.0)
- `tropism_angle: f32`, `tropism_strength: f32` — global tropism bias (default: π/4, 0.01)
- `nutrient_regen_rate: f32` — rate of nutrient regeneration (default: 0.004)
- `nutrient_regen_floor: f32` — minimum nutrient level for regeneration (default: 0.12)

#### Directional Flow (Water Drags Nutrients)
- `flow_enabled: bool` — enable directional nutrient flow (default: true)
- `flow_strength: f32` — strength of directional flow (0.0-1.0) (default: 0.3)
- `flow_direction: f32` — flow direction in radians (0 = right, π/2 = down) (default: π/4)
- `flow_variation: f32` — random variation in flow direction per timestep (default: 0.1)

#### Energy
- `energy_decay_rate: f32` — passive energy decay per step (default: 0.9985)
- `min_energy_to_live: f32` — below this, hyphae die (default: 0.005)

#### Hyphal Senescence & Death
- `senescence_enabled: bool` — enable hyphal senescence and death system (default: true)
- `senescence_base_probability: f32` — base death probability per timestep (0.0-1.0) (default: 0.00001)
- `senescence_nutrient_flow_threshold: f32` — low nutrient flow threshold that increases death probability (default: 0.005)
- `senescence_distance_threshold: f32` — distance from main network that increases death probability (default: 30.0)
- `senescence_weather_extreme_threshold: f32` — weather temperature threshold for extreme conditions (default: 0.3)
- `senescence_unsupported_collapse_distance: f32` — distance beyond which unsupported branches collapse (default: 50.0)
- `senescence_min_age: f32` — minimum age before senescence applies, giving hyphae time to establish (default: 5.0)

#### Networking
- `anastomosis_distance: f32` — distance within which hyphae connect (default: 2.0)
- `connection_flow_rate: f32` — diffusive energy flow along connections (default: 0.02)

#### Avoidance
- `hyphae_avoidance_distance: f32` — hyphae turn away if the next step is closer than this (default: 2.0)

#### Trails
- `max_segment_age: f32` — maximum age before segments are removed (default: 10.0)
- `segment_age_increment: f32` — how fast segments age per frame (default: 0.01)

#### Fruiting
- `fruiting_min_hyphae: usize` — minimum hyphae count to spawn fruiting bodies (default: 12)
- `fruiting_threshold_total_energy: f32` — minimum total energy for fruiting (default: 6.0)
- `fruiting_cooldown: f32` — cooldown between fruiting body spawns (default: 10.0)
- `fruiting_lifespan_min: f32`, `fruiting_lifespan_max: f32` — fruiting body lifespan range (default: 12.0-20.0)
- `fruiting_spore_count: usize` — number of spores released by fruiting bodies (default: 6)
- `fruiting_spore_release_interval: f32` — interval between spore releases (default: 0.15)

#### Network Intelligence: Signal Propagation
- `signal_propagation_enabled: bool` — enable signal propagation (default: true)
- `signal_decay_rate: f32` — rate at which signals decay over time (default: 0.95)
- `signal_strength_threshold: f32` — minimum signal strength to trigger behavior (default: 0.1)
- `signal_trigger_nutrient_threshold: f32` — nutrient threshold to trigger signals (default: 0.5)

#### Network Intelligence: Adaptive Growth
- `adaptive_growth_enabled: bool` — enable adaptive growth (default: true)
- `flow_strengthening_rate: f32` — how fast connections strengthen with flow (default: 0.002)
- `flow_decay_rate: f32` — how fast connection strength decays (default: 0.998)
- `min_connection_strength: f32` — minimum connection strength (default: 0.1)
- `pruning_threshold: f32` — prune branches with strength below this (default: 0.05)

#### Network Intelligence: Memory & Learning
- `memory_enabled: bool` — enable memory system (default: true)
- `memory_decay_rate: f32` — rate at which memory decays over time (default: 0.995)
- `memory_update_strength: f32` — how strongly nutrient discoveries update memory (default: 0.3)
- `memory_influence: f32` — how much memory affects growth direction (0.0-1.0) (default: 0.15)

#### Performance: Growth Limits
- `max_hyphae: usize` — maximum number of hyphae (0 = unlimited) (default: 2000)
- `max_hyphae_branching_threshold: usize` — stop branching when hyphae count exceeds this (default: 1500)

#### Weather
- `weather_enabled: bool` — enable weather system (default: true)
- `weather_affects_growth: bool` — weather affects growth rate (default: true)
- `weather_affects_energy: bool` — weather affects energy consumption (default: true)

#### Fusion
- `fusion_enabled: bool` — enable fusion (default: true)
- `fusion_distance: f32` — distance threshold for fusion (should be < anastomosis_distance) (default: 1.0)
- `fusion_energy_transfer: f32` — energy transfer rate when fusing (default: 0.5)
- `fusion_min_age: f32` — minimum age for hyphae to be eligible for fusion (default: 0.1)

#### Initialization
- `initial_hyphae_count: usize` — number of hyphae at simulation start (default: 5)
- `obstacle_count: usize` — number of obstacles in the grid (default: 300)

You can tweak these parameters to change speed, density, network behavior, and look of the simulation. Larger `grid_size` with `--release` gives smoother visuals, but uses more CPU/GPU.

### Testing

The simulation includes a comprehensive test suite that can run without a graphics context:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_validation_after_100_iterations

# Run tests with output
cargo test -- --nocapture
```

#### Test Coverage
- **Simulation Creation**: Verifies simulation initializes correctly
- **Simulation Runs**: Ensures simulation runs without panicking
- **Hyphae Validation**: Checks energy levels, positions, and bounds
- **Connections Validation**: Validates connection references and strengths
- **Nutrients**: Verifies nutrient consumption and regeneration
- **Memory**: Tests memory system when enabled
- **Growth Limits**: Ensures max_hyphae limit is respected
- **Weather**: Validates weather system updates
- **Statistics**: Checks simulation statistics are accurate
- **Long Simulation**: Runs 1000 steps to ensure stability
- **Segments**: Validates segment aging
- **Branching**: Tests branching behavior
- **Fusion**: Tests fusion when enabled
- **Validation**: Comprehensive validation after 100 iterations

### Notes

- Reflection flips direction on the axis of impact and adds jitter to avoid sticking.
- Nutrients only decay via local consumption by hyphae; a diffusion step spreads nutrients.
- Obstacles can be enabled in the grid; tips reflect and re-steer when colliding.
- Weather conditions change dynamically, affecting growth, energy, and nutrient diffusion.
- Memory system allows hyphae to remember and return to productive nutrient locations.
- Adaptive growth strengthens efficient paths and prunes weak branches automatically.
- Fusion merges very close hyphae, combining their energy and characteristics.

### Architecture

The simulation is organized into several modules:

- **`simulation.rs`** — Core simulation logic with `Simulation`, `SimulationState`, and `SimulationConfig` structs. Includes test suite.
- **`config.rs`** — Configuration struct with all simulation parameters
- **`hypha.rs`** — Hypha struct and behavior
- **`spore.rs`** — Spore struct and behavior
- **`nutrients.rs`** — Nutrient grid and gradient calculations
- **`visualization.rs`** — All drawing functions with enhanced visualization options (UI mode only)
- **`controls.rs`** — Input handling and control text (UI mode only)
- **`types.rs`** — Shared types (Connection, Segment, FruitBody, Vec2)
- **`weather.rs`** — Weather system with temperature, humidity, and rain
- **`camera.rs`** — Camera system for pan/zoom functionality (UI mode only)
- **`api.rs`** — HTTP API server for headless mode with REST endpoints
- **`main.rs`** — Main entry point, supports both UI and headless modes

The `Simulation` struct contains:
- `state: SimulationState` — All mutable simulation data (nutrients, hyphae, spores, connections, memory, weather, etc.)
- `config: SimulationConfig` — All configuration parameters
- Control flags (`paused`, `connections_visible`, `minimap_visible`, `memory_visible`, `enhanced_visualization`, etc.)
- `camera: Camera` — Camera for pan/zoom (when enabled)

### Troubleshooting

#### UI Mode
- If the window is too large, lower `grid_size` or `cell_size` in the config.
- If performance is low, use `cargo run --release` and/or reduce `grid_size`. You can also toggle connections (X), minimap (M), enhanced visualization (V), or lower `max_segment_age`.
- If trails overwhelm the scene, reduce `max_segment_age` or increase `segment_age_increment` in the config.
- If hyphae grow too fast, reduce `branch_prob` or enable growth limits (`max_hyphae`).
- If memory overlay is not visible, ensure `memory_enabled` is true and wait for hyphae to discover nutrients (memory accumulates over time).
- If camera is not working, ensure `camera_enabled` is true in the config.

#### Headless Mode
- If the API server doesn't start, check that the port isn't already in use: `lsof -i :8080`
- If you get connection refused, ensure the server is running and check the port number
- The simulation runs automatically in the background - you don't need to call `/step` unless you want manual control
- Use `POST /pause` to pause the automatic simulation loop
- The simulation respects the pause state, so paused simulations won't advance automatically

#### General
- If tests fail, check that all dependencies are installed: `cargo test --no-run` to verify compilation.
- For headless mode without UI dependencies: `cargo run --no-default-features -- --headless`
- For headless mode with UI features available: `cargo run --features ui -- --headless`

### License

MIT — see `LICENSE` if present, or adapt for personal use.
