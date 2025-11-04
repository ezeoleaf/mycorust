## MycoRust — Mycelium Growth Simulation (Macroquad)

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

- Nutrient field rendered as a brown→green heatmap (with diffusion over time).
- Hyphae growth: gradient following + small random wander.
- Branching with configurable probability.
- Edge reflection with axis-correct bounce and a slight jitter.
- Obstacle avoidance and hyphae-to-hyphae avoidance to reduce overlaps.
- Energy model: tips consume nutrients locally and die when energy is depleted.
- Anastomosis: nearby hyphae connect; simple energy balancing along connections.
- Trails persist and fade over time; old segments are removed to limit memory growth.
- Live statistics overlay (Hyphae, Spores, Connections, Avg Energy, FPS).

### Controls

- SPACE: Pause/Resume
- R: Reset simulation
- C: Clear trails
- S: Spawn a new hypha at mouse position
- N: Add a nutrient patch at mouse position (small radius)
- Left Mouse Button: Add a single high-nutrient cell at mouse

Controls help is shown at the bottom of the screen.

### Key parameters (in `src/main.rs`)

- GRID/Display

  - `GRID_SIZE: usize` — logical grid resolution; window size = `GRID_SIZE * CELL_SIZE`.
  - `CELL_SIZE: f32` — pixels per grid cell.

- Growth & branching

  - `BRANCH_PROB: f32` — branching probability per step.
  - `STEP_SIZE: f32` — movement step length per frame.
  - `ANGLE_WANDER_RANGE: f32` — random wander added to direction each frame.
  - `GRADIENT_STEERING_STRENGTH: f32` — steering toward nutrient gradient.

- Nutrients

  - `NUTRIENT_DECAY: f32` — max nutrient absorbed per step at a tip.
  - `DIFFUSION_RATE: f32` — how quickly nutrients diffuse across neighbors.
  - `SPORE_GERMINATION_THRESHOLD: f32` — nutrient threshold for spores to germinate.

- Energy

  - `ENERGY_DECAY_RATE: f32` — passive energy decay per step.
  - `MIN_ENERGY_TO_LIVE: f32` — below this, hyphae die.

- Networking

  - `ANASTOMOSIS_DISTANCE: f32` — distance within which hyphae connect.

- Avoidance

  - `HYPHAE_AVOIDANCE_DISTANCE: f32` — hyphae turn away if the next step is closer than this.

- Trails
  - `MAX_SEGMENT_AGE: f32`, `SEGMENT_AGE_INCREMENT: f32` — control trail fading and cleanup.

You can tweak these constants to change speed, density, network behavior, and look of the simulation. Larger `GRID_SIZE` with `--release` gives smoother visuals, but uses more CPU/GPU.

### Notes

- Reflection flips direction on the axis of impact and adds jitter to avoid sticking.
- Nutrients only decay via local consumption by hyphae; a diffusion step spreads nutrients.
- Obstacles can be enabled in the grid; tips reflect and re-steer when colliding.

### Troubleshooting

- If the window is too large, lower `GRID_SIZE` or `CELL_SIZE`.
- If performance is low, use `cargo run --release` and/or reduce `GRID_SIZE`.
- If trails overwhelm the scene, reduce `MAX_SEGMENT_AGE` or increase `SEGMENT_AGE_INCREMENT`.

### License

MIT — see `LICENSE` if present, or adapt for personal use.
