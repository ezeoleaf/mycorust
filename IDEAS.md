# Ideas on the MycoRust core

MycoRust is split so the **growth engine** can be reused without the interactive UI.

```text
mycorust/
├── crates/mycorust-core   # library: step, nutrients, weather, network metrics
├── crates/mycorust        # interactive app (Macroquad UI + headless HTTP API)
├── web/                   # cute in-browser garden (GitHub Pages)
└── ideas/*                # experiments that only depend on the core
```

Every idea crate does the same thing at heart:

1. Build a `SimulationConfig`
2. Create a `Simulation`
3. Seed nutrients, obstacles, or weather
4. Call `sim.step_n(...)`
5. Read `analyze(&sim)` (and sometimes the graph helpers)

```rust
use mycorust_core::{analyze, Simulation, SimulationConfig};

let mut sim = Simulation::with_config(&mut rng, SimulationConfig::default());
sim.step_n(&mut rng, 400);
let metrics = analyze(&sim);
println!("resilience {:.3}", metrics.resilience_score());
```

Run any experiment from the repo root:

```bash
cargo run -p mycorust-climate --release
```

The interactive simulator is unchanged:

```bash
cargo run                    # UI (default member)
cargo run -- --headless      # HTTP API
```

---

## Core API the ideas share

| Piece | What it is for |
| --- | --- |
| `Simulation` / `SimulationConfig` | Grow hyphae, branch, fuse, remember, fruit |
| `NutrientGrid::add_patch` / `clear` | Paint food, images, towns, mazes |
| `Weather` / `EnvironmentalStress` | Drought, flood, heat, cold, pollution |
| `Simulation::apply_stress` | Hold a climate shock on the live network |
| `analyze` → `NetworkMetrics` | Biomass, fragmentation, loops, moisture, memory |
| `SimulationConfig::record_segments` | Hyphal trails; leave **on** for the UI / generative SVG, **off** for other experiments |

`NetworkMetrics` includes living hyphae, connections, energy, connected components, fragmentation (`1 - largest_component / living`), mean degree, cycle count, nutrient mass, memory mass, and a `resilience_score()` in `0..=1`.

---

## Experiments

| # | Idea | Crate | Status | What it runs |
| --- | --- | --- | --- | --- |
| 1 | Climate bio-indicator | `mycorust-climate` | prototype | Grow → drought → recover; prints fragmentation and resilience |
| 2 | Fungal routing | `mycorust-routing` | prototype | Two nutrient patches; mycelium path vs Euclidean stretch |
| 3 | Living compression | `mycorust-compression` | prototype | Nutrient glyph → graph size vs occupied cells |
| 4 | Generative tools | `mycorust-generative` | prototype | Writes `mycelium-art.svg` from trails and fusions |
| 5 | Intelligence benchmark | `mycorust-benchmark` | prototype | Maze connect + memory + shock adaptation score |
| 6 | Urban infrastructure | `mycorust-infrastructure` | prototype | Towns as demand, river as terrain; reports loops / degree |
| 7 | Materials twin | `mycorust-materials` | prototype | Fill, porosity, cord thickness, insulation proxy |
| 8 | Ethics sandbox | `mycorust-ethics` | prototype | Exploit vs steward strategies on the same world |
| 9 | Sound / music | `mycorust-sound` | prototype | Maps growth to a `mycelium-score.wav` |
| 10 | Evolutionary sandbox | `mycorust-evolution` | prototype | Mutates genomes; selects biomass + resilience |

These are **working sketches**, not papers. They exist so the next step is science or design on top of a shared engine, not another copy of the simulator.

---

### 1. Mycelium as a climate-response model

Use growth as a **proxy sensor** for environmental stress: drought, flooding, heat, cold, pollution.

Instead of forecasting weather, you watch the network **collapse, reroute, or recover**.

**Why it is interesting**

- Mycelium is extremely sensitive to moisture and temperature
- Few simulations treat fungi as early-warning sensors
- Outputs map cleanly onto the core metrics: resilience, recovery time, fragmentation

**Who cares:** climate researchers, environmental NGOs, systems ecologists.

```bash
cargo run -p mycorust-climate --release
```

---

### 2. Fungal network optimization as a routing algorithm

Treat the mycelium as a graph:

- **nodes** — nutrient patches / hyphal tips
- **edges** — hyphae and anastomoses
- **flow** — carbon and nitrogen

Let it solve shortest paths, fault-tolerant routing, and congestion, then compare with Dijkstra, A*, or ant-colony methods.

**Why it is interesting**

- Slime molds are famous for this; fungi are not
- Mycelium forms loops and redundancy on its own
- Fungi already balance cost vs robustness

```bash
cargo run -p mycorust-routing --release
```

This is the most paper-shaped idea in the set.

---

### 3. Mycelium as a living compression algorithm

Feed spatial information in as nutrients. The fungus keeps a **minimal connecting structure**.

Example: input image → nutrient field → mycelial graph of what it considers essential.

**Why it is interesting**

- Unconventional / spatial computation
- Cousin of reservoir computing, almost no tooling

```bash
cargo run -p mycorust-compression --release
```

---

### 4. Digital mycelium for creative tools

The same simulator as a generative engine: textures, album art, architectural layouts, textile patterns — parameterized by **biology**, not Perlin noise.

```bash
cargo run -p mycorust-generative --release
# writes mycelium-art.svg
```

Low competition, high aesthetic value.

---

### 5. Mycelium network intelligence benchmark

Score the network the way you would score a model:

- solve a maze
- optimize transport between patches
- remember past nutrient locations
- adapt after a shock

**Why it is interesting**

- Most “bio-inspired AI” ignores fungi
- Reservoir computing with mycelium is barely explored

```bash
cargo run -p mycorust-benchmark --release
```

---

### 6. Urban infrastructure inspired by fungi

Grow road, power, or drainage networks from the bottom up. Constraints become terrain, demand (population), and failure tolerance.

**Why it is interesting**

- Most infrastructure optimization is top-down
- Fungi are adaptive and redundant

```bash
cargo run -p mycorust-infrastructure --release
```

---

### 7. Digital twin for mycelium materials

Simulate composite growth (density, thickness, weak spans) to estimate strength, porosity, and insulation.

Could interest sustainable-materials companies and architecture labs.

```bash
cargo run -p mycorust-materials --release
```

---

### 8. Fungal ethics simulator

Tradeoffs: growth vs sustainability, exploitation vs resilience, cooperation vs competition. Fungi as a metaphor for sharing, collapse, and regeneration.

Fits education, installations, and systems philosophy.

```bash
cargo run -p mycorust-ethics --release
```

---

### 9. Mycelium × sound

Map growth rate → tempo, branching → harmony, nutrient flow → volume, weather → effects. The fungus composes while it grows.

```bash
cargo run -p mycorust-sound --release
# writes mycelium-score.wav
```

---

### 10. Evolutionary fungal sandbox

Multiple strategies (later: species) with mutation, selection, extinction, and symbiosis. Watch dominant tactics emerge and fragile ones disappear.

Almost no open-source tools exist for this.

```bash
cargo run -p mycorust-evolution --release
```

---

## Adding a new idea

1. Copy an existing `ideas/*` crate.
2. Depend only on `mycorust-core` (and `rand` if you need a seeded RNG).
3. Add the member path to the root `Cargo.toml` workspace list.
4. Keep the interactive UI out of the experiment — if you need pictures, write SVG/CSV from core state, or drive the existing headless API.
