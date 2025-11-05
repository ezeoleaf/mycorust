# MycoRust Code Review & Improvement Suggestions

## 🔴 Critical Issues

### ✅ 1. Cargo.toml Edition Fix - COMPLETED

```toml
edition = "2021"
```

### 2. Unused Parent Field

The `parent: Option<usize>` field exists but transport code is commented out (lines 267-274). Either:

- Remove the field if not needed, OR
- Implement proper resource transport between hyphae

### ✅ 3. Energy Death Not Implemented - COMPLETED

Hyphae have energy but never die from starvation. They should die when energy reaches 0.

## 🟡 Code Quality Improvements

### ✅ 1. Extract Magic Numbers to Constants - COMPLETED

Several magic numbers should be constants:

```rust
const ENERGY_DECAY_RATE: f32 = 0.999;
const MIN_ENERGY_TO_LIVE: f32 = 0.01;
const SPORE_GERMINATION_THRESHOLD: f32 = 0.6;
const ANASTOMOSIS_DISTANCE: f32 = 2.0;
const DIFFUSION_RATE: f32 = 0.05;
const GRADIENT_STEERING_STRENGTH: f32 = 0.1;
const ANGLE_WANDER_RANGE: f32 = 0.05;
```

### ✅ 2. Performance: Reduce Segment Memory Growth - COMPLETED

The `segments` vector grows indefinitely. Consider:

- Capping maximum segments
- Using a circular buffer
- Fading old segments (remove after N frames)

Implemented with trail decay system that ages and removes old segments.

### ✅ 3. Anastomosis Logic Issue - COMPLETED

Anastomosis should:

- Connect them in a network
- Enable resource sharing
- Not randomly kill them

### ✅ 4. Bounds Checking Duplication - COMPLETED

Both obstacle check and boundary check did similar bounds validation. Refactored with `in_bounds(x, y)` helper.

## 🟢 New Features to Add

### ✅ 1. **Energy-Based Death System** - COMPLETED

```rust
// In hyphae update loop:
if h.energy < MIN_ENERGY_TO_LIVE {
    h.alive = false;
    continue;
}
```

### ✅ 2. **Resource Transport Between Hyphae** - COMPLETED

Uncomment and fix the parent transport code. Consider distance-based transport:

```rust
if let Some(parent_idx) = h.parent {
    if let Some(parent) = hyphae.get(parent_idx) {
        let dx = h.x - parent.x;
        let dy = h.y - parent.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < MAX_TRANSPORT_DISTANCE {
            // Transfer energy based on distance
            let transfer = 0.001 * h.energy * (1.0 - dist / MAX_TRANSPORT_DISTANCE);
            h.energy -= transfer;
            parent.energy = (parent.energy + transfer).min(1.0);
        }
    }
}
```

### ✅ 3. **Statistics Display** - COMPLETED

Add on-screen stats:

```rust
let stats_text = format!(
    "Hyphae: {} | Spores: {} | Energy Avg: {:.2} | FPS: {:.0}",
    hyphae.len(),
    spores.len(),
    avg_energy,
    get_fps()
);
draw_text(&stats_text, 10.0, 20.0, 20.0, WHITE);
```

### ✅ 4. **Pause/Play Controls** - COMPLETED

```rust
let mut paused = false;
if is_key_pressed(KeyCode::Space) {
    paused = !paused;
}
if !paused {
    // Update simulation
}
```

### ✅ 5. **Hyphae Avoidance** - COMPLETED

Prevent hyphae from growing into each other:

```rust
// Before moving, check if new position would overlap with another hypha
let new_x = h.x + h.angle.cos() * STEP_SIZE;
let new_y = h.y + h.angle.sin() * STEP_SIZE;
let mut too_close = false;
for other in &hyphae {
    if other.alive && other as *const Hypha != h as *const Hypha {
        let dx = new_x - other.x;
        let dy = new_y - other.y;
        if dx * dx + dy * dy < 4.0 { // too close
            too_close = true;
            break;
        }
    }
}
if too_close {
    h.angle += rng.gen_range(-0.5..0.5); // turn away
}
```

### ✅ 6. **Age-Based Characteristics** - COMPLETED

Add age to hyphae and make older hyphae:

- Thicker (wider lines)
- Slower growing
- More likely to branch
- Better at resource transport

### 7. **Visual Improvements**

- **Trail fading**: Older segments fade over time
- **Energy visualization**: Color hyphae by energy level (already partially done)
- **Network visualization**: Highlight connected hyphae differently
- **Growth direction indicators**: Small arrows showing growth direction

### ✅ 8. **Multiple Spawn Points** - COMPLETED

Allow starting from multiple locations:

```rust
let mut hyphae = vec![];
for _ in 0..INITIAL_HYPHAE_COUNT {
    hyphae.push(Hypha {
        x: rng.gen_range(50.0..150.0),
        y: rng.gen_range(50.0..150.0),
        // ... rest
    });
}
```

### ✅ 9. **Keyboard Controls** - COMPLETED

- `R` - Reset simulation
- `C` - Clear all segments
- `S` - Spawn new hypha at mouse
- `N` - Add nutrient patch at mouse
- `SPACE` - Pause/Play
- `LMB` - Add single nutrient

### ✅ 10. **Better Anastomosis** - COMPLETED

Instead of randomly killing, create actual connections:

```rust
struct Connection {
    hypha1: usize,
    hypha2: usize,
    strength: f32,
}
let mut connections: Vec<Connection> = vec![];

// When hyphae get close:
if dist2 < 4.0 {
    connections.push(Connection {
        hypha1: i,
        hypha2: j,
        strength: 1.0,
    });
    // Draw connection line
    draw_line(...);
}
```

### 11. **Fruiting Body Formation**

When network reaches certain size/energy, spawn a fruiting body (mushroom):

```rust
if total_energy > FRUITING_THRESHOLD && !has_fruiting_body {
    spawn_fruiting_body(center_x, center_y);
}
```

### 12. **Competing Networks**

Different hypha groups (different colors) that compete for resources.

### ✅ 13. **Trail Decay** - COMPLETED

Instead of keeping all segments forever:

```rust
struct Segment {
    from: Vec2,
    to: Vec2,
    age: f32,
}
// In update loop:
segments.iter_mut().for_each(|s| s.age += 0.01);
segments.retain(|s| s.age < MAX_SEGMENT_AGE);
// Fade older segments
```

### 14. **Nutrient Source Types**

Different nutrient types (sugar, nitrogen, etc.) that hyphae prefer differently.

### ✅ 15. **Hyphal Density Effects** - COMPLETED

Slower growth in areas with many hyphae (competition).

## 🔧 Code Structure Improvements

### 1. Split into Modules - COMPLETED

- `simulation.rs` - Core simulation logic
- `hypha.rs` - Hypha struct and methods
- `spore.rs` - Spore struct and methods
- `nutrients.rs` - Nutrient grid and diffusion
- `visualization.rs` - Drawing functions

### 2. Configuration Struct

```rust
struct SimulationConfig {
    grid_size: usize,
    cell_size: f32,
    branch_prob: f32,
    step_size: f32,
    // ... etc
}
```

### 3. State Management

```rust
struct SimulationState {
    nutrients: [[f32; GRID_SIZE]; GRID_SIZE],
    obstacles: [[bool; GRID_SIZE]; GRID_SIZE],
    hyphae: Vec<Hypha>,
    spores: Vec<Spore>,
    segments: Vec<Segment>,
}
```

## 🎨 Visual Enhancements

1. ✅ **Color gradients** based on energy/age (connections pulse + age fade)
2. ✅ **Particle effects** when spores germinate
3. ✅ **Pulsing** at anastomosis points
4. ✅ **Depth cues** - older hyphae darker/more transparent
5. ✅ **Minimap** showing overall network structure

## 📊 Performance Optimizations

1. ✅ **Spatial partitioning** for hyphae collision checks (uniform grid buckets)
2. ✅ **Batch rendering** for segments (FPS-based decimation of draw calls)
3. ✅ **LOD (Level of Detail)** - reduce detail when FPS drops (diffusion/frame skip, draw decimation)
4. ✅ **Conditional updates** - only update visible areas (diffusion limited to hyphae bounding box)

## 🧪 Scientific Accuracy

1. ✅ **Chemotaxis** - more realistic gradient following (Sobel gradient, smoother steering)
2. ✅ **Tropism** - response to different stimulations (global tropism vector bias)
3. ✅ **Mycelial network topology** - proper graph structure (connections as graph; energy flow)
4. ✅ **Resource allocation** - more realistic energy distribution (diffusive flow along connections)
