## MycoRust — Mycelium Growth Simulation (Macroquad)

A simple mycelium/hyphae growth simulation written in Rust using Macroquad. Hyphae follow a nutrient gradient, branch stochastically, leave trails, and reflect off the window edges.

### Prerequisites
- Rust toolchain (stable) — install via `https://rustup.rs`

### Run
```bash
cargo run
# or for better performance
cargo run --release
```

### What you’ll see
- Nutrient field rendered as a brown→green heatmap.
- White tips advance following the local nutrient gradient with a small random wander.
- Trails are drawn and persist between frames.
- When a tip reaches a boundary, it reflects and continues.

### Key parameters (in `src/main.rs`)
- `GRID_SIZE: usize` — logical grid resolution. Window size = `GRID_SIZE * CELL_SIZE`.
- `CELL_SIZE: f32` — pixel size per grid cell.
- `BRANCH_PROB: f32` — probability per step that a hypha branches.
- `STEP_SIZE: f32` — movement step length per frame.
- `NUTRIENT_DECAY: f32` — per-step nutrient consumption by tips.
- `nutrient_color(value: f32)` — maps nutrient level to display color.

You can tweak these constants to change speed, density, and look of the simulation. Larger `GRID_SIZE` with `--release` gives smoother visuals, but will use more CPU/GPU.

### Notes
- The reflection logic flips the movement angle on the axis of impact and adds a slight jitter to avoid sticking/ping‑ponging.
- A tiny random wander is applied each frame to keep motion organic.

### Troubleshooting
- If the window is too large for your display, lower `GRID_SIZE` or `CELL_SIZE`.
- If performance is low, use `cargo run --release` and consider reducing `GRID_SIZE`.

### License
MIT — see `LICENSE` if present, or feel free to adapt for your own use.


