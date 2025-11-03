use ::rand as external_rand;
use external_rand::{thread_rng, Rng};
use macroquad::prelude::*;

const GRID_SIZE: usize = 200;
const CELL_SIZE: f32 = 4.0;
const BRANCH_PROB: f32 = 0.002;
const STEP_SIZE: f32 = 0.5;
const NUTRIENT_DECAY: f32 = 0.01;
const OBSTACLE_COUNT: usize = 300;

// Energy constants
const ENERGY_DECAY_RATE: f32 = 0.999;
const MIN_ENERGY_TO_LIVE: f32 = 0.01;

// Spore constants
const SPORE_GERMINATION_THRESHOLD: f32 = 0.6;
const SPORE_MAX_AGE: f32 = 5.0;

// Anastomosis constants
const ANASTOMOSIS_DISTANCE: f32 = 2.0;
const ANASTOMOSIS_DISTANCE_SQ: f32 = ANASTOMOSIS_DISTANCE * ANASTOMOSIS_DISTANCE;

// Diffusion constants
const DIFFUSION_RATE: f32 = 0.05;

// Steering constants
const GRADIENT_STEERING_STRENGTH: f32 = 0.1;
const ANGLE_WANDER_RANGE: f32 = 0.05;

// Hyphae avoidance
const HYPHAE_AVOIDANCE_DISTANCE: f32 = 2.0;
const HYPHAE_AVOIDANCE_DISTANCE_SQ: f32 = HYPHAE_AVOIDANCE_DISTANCE * HYPHAE_AVOIDANCE_DISTANCE;

#[derive(Clone)]
struct Hypha {
    x: f32,
    y: f32,
    prev_x: f32,
    prev_y: f32,
    angle: f32,
    alive: bool,
    energy: f32,
    parent: Option<usize>,
}

#[derive(Clone)]
struct Spore {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    alive: bool,
    age: f32,
}

struct Connection {
    hypha1: usize,
    hypha2: usize,
    strength: f32,
}

struct Segment {
    from: Vec2,
    to: Vec2,
    age: f32,
}

const MAX_SEGMENT_AGE: f32 = 10.0;
const SEGMENT_AGE_INCREMENT: f32 = 0.01;

fn nutrient_color(value: f32) -> Color {
    // Clamp between 0 and 1
    let v = value.clamp(0.0, 1.0);
    // Map nutrients to a brownish-to-green gradient
    Color::new(0.2 + 0.3 * v, 0.3 + 0.5 * v, 0.2, 1.0)
}

fn nutrient_gradient(grid: &[[f32; GRID_SIZE]; GRID_SIZE], x: f32, y: f32) -> (f32, f32) {
    let xi = x as usize;
    let yi = y as usize;
    if xi == 0 || yi == 0 || xi >= GRID_SIZE - 1 || yi >= GRID_SIZE - 1 {
        return (0.0, 0.0);
    }

    let dx = grid[xi + 1][yi] - grid[xi - 1][yi];
    let dy = grid[xi][yi + 1] - grid[xi][yi - 1];
    (dx, dy)
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rng = thread_rng();

    // --- Initialize environment ---
    let mut nutrients = [[0.0f32; GRID_SIZE]; GRID_SIZE];
    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            let dist = ((x as f32 - 100.0).powi(2) + (y as f32 - 100.0).powi(2)).sqrt();
            nutrients[x][y] = (1.0 - dist / 180.0).max(0.0)
                * rng.gen_range(0.7..1.0)
                * (1.0 + rng.gen_range(-0.1..0.1));
        }
    }

    let mut obstacles = [[false; GRID_SIZE]; GRID_SIZE];
    for _ in 0..OBSTACLE_COUNT {
        let x = rng.gen_range(0..GRID_SIZE);
        let y = rng.gen_range(0..GRID_SIZE);
        obstacles[x][y] = true;
    }

    // --- Initialize hyphae ---
    let mut hyphae = vec![Hypha {
        x: GRID_SIZE as f32 / 2.0,
        y: GRID_SIZE as f32 / 2.0,
        prev_x: GRID_SIZE as f32 / 2.0,
        prev_y: GRID_SIZE as f32 / 2.0,
        angle: rng.gen_range(0.0..std::f32::consts::TAU),
        alive: true,
        energy: 0.5,
        parent: None,
    }];

    // --- Initialize spores ---
    let mut spores: Vec<Spore> = Vec::new();

    // Accumulate drawn line segments so they persist frame-to-frame
    let mut segments: Vec<Segment> = Vec::new();

    // Pause/play state
    let mut paused = false;

    // Anastomosis connections
    let mut connections: Vec<Connection> = Vec::new();

    loop {
        // Keyboard controls
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }

        if is_key_pressed(KeyCode::R) {
            // Reset simulation
            hyphae.clear();
            spores.clear();
            segments.clear();
            connections.clear();
            hyphae.push(Hypha {
                x: GRID_SIZE as f32 / 2.0,
                y: GRID_SIZE as f32 / 2.0,
                prev_x: GRID_SIZE as f32 / 2.0,
                prev_y: GRID_SIZE as f32 / 2.0,
                angle: rng.gen_range(0.0..std::f32::consts::TAU),
                alive: true,
                energy: 0.5,
                parent: None,
            });
        }

        if is_key_pressed(KeyCode::C) {
            // Clear all segments
            segments.clear();
        }

        if is_key_pressed(KeyCode::S) {
            // Spawn new hypha at mouse position
            let (mx, my) = mouse_position();
            let gx = (mx / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0);
            let gy = (my / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0);
            hyphae.push(Hypha {
                x: gx,
                y: gy,
                prev_x: gx,
                prev_y: gy,
                angle: rng.gen_range(0.0..std::f32::consts::TAU),
                alive: true,
                energy: 0.5,
                parent: None,
            });
        }

        if is_key_pressed(KeyCode::N) {
            // Add nutrient patch at mouse position
            let (mx, my) = mouse_position();
            let gx = (mx / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0) as usize;
            let gy = (my / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0) as usize;
            // Add nutrients in a small radius
            for dx in -3..=3 {
                for dy in -3..=3 {
                    let nx = (gx as i32 + dx).max(0).min(GRID_SIZE as i32 - 1) as usize;
                    let ny = (gy as i32 + dy).max(0).min(GRID_SIZE as i32 - 1) as usize;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    if dist < 3.0 {
                        nutrients[nx][ny] = 1.0;
                    }
                }
            }
        }

        // Mouse interaction (works even when paused)
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let gx = (mx / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0) as usize;
            let gy = (my / CELL_SIZE).clamp(0.0, GRID_SIZE as f32 - 1.0) as usize;
            nutrients[gx][gy] = 1.0;
        }

        // Blue background every frame so it stays visible
        clear_background(Color::new(0.05, 0.10, 0.35, 1.0));

        // Draw nutrients
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                let v = nutrients[x][y];
                let color = nutrient_color(v);
                draw_rectangle(
                    x as f32 * CELL_SIZE,
                    y as f32 * CELL_SIZE,
                    CELL_SIZE,
                    CELL_SIZE,
                    color,
                );
            }
        }

        // Draw obstacles
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                if obstacles[x][y] {
                    draw_rectangle(
                        x as f32 * CELL_SIZE,
                        y as f32 * CELL_SIZE,
                        CELL_SIZE,
                        CELL_SIZE,
                        Color::new(0.05, 0.05, 0.05, 1.0),
                    );
                }
            }
        }

        // Redraw all past segments to keep trails visible (with fading)
        for segment in &segments {
            let age_factor = 1.0 - (segment.age / MAX_SEGMENT_AGE);
            let alpha = age_factor.clamp(0.0, 1.0);
            let color = Color::new(1.0, 1.0, 1.0, alpha);
            draw_line(
                segment.from.x,
                segment.from.y,
                segment.to.x,
                segment.to.y,
                1.5,
                color,
            );
        }

        // Draw anastomosis connections
        for conn in &connections {
            if let (Some(h1), Some(h2)) = (hyphae.get(conn.hypha1), hyphae.get(conn.hypha2)) {
                if h1.alive && h2.alive {
                    draw_line(
                        h1.x * CELL_SIZE,
                        h1.y * CELL_SIZE,
                        h2.x * CELL_SIZE,
                        h2.y * CELL_SIZE,
                        2.0,
                        Color::new(0.0, 1.0, 0.5, 0.6),
                    );
                }
            }
        }

        // Update simulation only if not paused
        if !paused {
            // Age all segments
            for segment in &mut segments {
                segment.age += SEGMENT_AGE_INCREMENT;
            }
            // Remove old segments
            segments.retain(|s| s.age < MAX_SEGMENT_AGE);

            let mut new_hyphae = vec![];
            let hyphae_len = hyphae.len();

            // Collect all hyphae positions and alive status first to avoid borrow conflicts
            let hyphae_info: Vec<(f32, f32, bool)> =
                hyphae.iter().map(|h| (h.x, h.y, h.alive)).collect();

            for (idx, h) in hyphae[..hyphae_len].iter_mut().enumerate() {
                if !h.alive {
                    continue;
                }

                // Store old position
                h.prev_x = h.x;
                h.prev_y = h.y;

                // Get local gradient; avoid steering when gradient is near zero (edge/flat)
                let (gx, gy) = nutrient_gradient(&nutrients, h.x, h.y);
                let grad_mag = (gx * gx + gy * gy).sqrt();
                if grad_mag > 1e-6 {
                    let grad_angle = gy.atan2(gx);
                    h.angle += (grad_angle - h.angle) * GRADIENT_STEERING_STRENGTH;
                }

                // Small random wander to avoid directional lock-in
                h.angle += rng.gen_range(-ANGLE_WANDER_RANGE..ANGLE_WANDER_RANGE);

                // Hyphae avoidance: check if new position would be too close to another hypha
                let new_x = h.x + h.angle.cos() * STEP_SIZE;
                let new_y = h.y + h.angle.sin() * STEP_SIZE;
                let mut too_close = false;

                for (other_idx, (ox, oy, other_alive)) in hyphae_info.iter().enumerate() {
                    if other_idx == idx || !other_alive {
                        continue;
                    }
                    let dx = new_x - ox;
                    let dy = new_y - oy;
                    let dist2 = dx * dx + dy * dy;
                    if dist2 < HYPHAE_AVOIDANCE_DISTANCE_SQ && dist2 > 0.001 {
                        // Too close to another hypha, turn away
                        too_close = true;
                        break;
                    }
                }

                if too_close {
                    h.angle += rng.gen_range(-0.5..0.5); // turn away from other hypha
                }

                // Move
                h.x += h.angle.cos() * STEP_SIZE;
                h.y += h.angle.sin() * STEP_SIZE;

                // Check if new position is in an obstacle
                let xi = h.x as usize;
                let yi = h.y as usize;
                if xi < GRID_SIZE && yi < GRID_SIZE && obstacles[xi][yi] {
                    // Revert to previous position
                    h.x = h.prev_x;
                    h.y = h.prev_y;

                    // Try to find a clear direction by testing multiple angles
                    let mut found_clear = false;
                    let mut best_angle = h.angle;
                    let mut attempts = 0;

                    // Try several angles around the current direction
                    while !found_clear && attempts < 8 {
                        let test_angle = h.angle + (attempts as f32) * std::f32::consts::PI / 4.0;
                        let test_x = h.x + test_angle.cos() * STEP_SIZE;
                        let test_y = h.y + test_angle.sin() * STEP_SIZE;

                        let test_xi = test_x as usize;
                        let test_yi = test_y as usize;

                        // Check bounds and obstacle
                        if test_xi < GRID_SIZE
                            && test_yi < GRID_SIZE
                            && test_x >= 0.0
                            && test_y >= 0.0
                            && !obstacles[test_xi][test_yi]
                        {
                            best_angle = test_angle;
                            found_clear = true;
                        }
                        attempts += 1;
                    }

                    // If we found a clear direction, use it; otherwise add large random rotation
                    if found_clear {
                        h.angle = best_angle + rng.gen_range(-0.2..0.2); // small jitter
                    } else {
                        // No clear direction found, rotate significantly with jitter
                        h.angle += std::f32::consts::PI + rng.gen_range(-0.5..0.5);
                    }

                    // Normalize angle
                    h.angle = h.angle % std::f32::consts::TAU;
                    if h.angle < 0.0 {
                        h.angle += std::f32::consts::TAU;
                    }

                    // Move in the new direction
                    h.x += h.angle.cos() * STEP_SIZE;
                    h.y += h.angle.sin() * STEP_SIZE;
                }

                // Bounds handling: reflect off walls with small jitter
                if h.x < 1.0
                    || h.x >= GRID_SIZE as f32 - 1.0
                    || h.y < 1.0
                    || h.y >= GRID_SIZE as f32 - 1.0
                {
                    // revert to previous valid position
                    h.x = h.prev_x;
                    h.y = h.prev_y;
                    // reflect based on which wall we hit
                    let min_b = 1.0;
                    let max_b = GRID_SIZE as f32 - 2.0;
                    if h.x <= min_b {
                        h.x = min_b;
                        h.angle = std::f32::consts::PI - h.angle;
                    } else if h.x >= max_b {
                        h.x = max_b;
                        h.angle = std::f32::consts::PI - h.angle;
                    }
                    if h.y <= min_b {
                        h.y = min_b;
                        h.angle = -h.angle;
                    } else if h.y >= max_b {
                        h.y = max_b;
                        h.angle = -h.angle;
                    }
                    // small random jitter to avoid re-hitting the same wall
                    h.angle += rng.gen_range(-0.15..0.15);
                    // step away from wall in the new direction and clamp
                    h.x += h.angle.cos() * STEP_SIZE;
                    h.y += h.angle.sin() * STEP_SIZE;
                    h.x = h.x.clamp(min_b, max_b);
                    h.y = h.y.clamp(min_b, max_b);
                }

                let xi = h.x as usize;
                let yi = h.y as usize;
                // Consume nutrient and maybe spawn spores if starving
                let n = nutrients[xi][yi];
                if n > 0.001 {
                    let absorbed = n.min(NUTRIENT_DECAY);
                    h.energy = (h.energy + absorbed).min(1.0);
                    nutrients[xi][yi] -= absorbed;
                }

                // Gradual energy decay
                h.energy *= ENERGY_DECAY_RATE;

                // Die if energy depleted
                if h.energy < MIN_ENERGY_TO_LIVE {
                    h.alive = false;
                    continue;
                }

                // Transport to parent if exists
                // if let Some(parent_idx) = h.parent {
                //     if let Some(parent) = hyphae.get_mut(parent_idx) {
                //         let transfer = 0.001 * h.energy;
                //         h.energy -= transfer;
                //         parent.energy = (parent.energy + transfer).min(1.0);
                //     }
                // }

                if n < 0.05 && rng.gen_bool(0.001) {
                    spores.push(Spore {
                        x: h.x,
                        y: h.y,
                        vx: rng.gen_range(-0.5..0.5),
                        vy: rng.gen_range(-0.5..0.5),
                        alive: true,
                        age: 0.0,
                    });
                }

                // Branch occasionally
                if rng.r#gen::<f32>() < BRANCH_PROB {
                    let idx = hyphae_len;
                    new_hyphae.push(Hypha {
                        x: h.x,
                        y: h.y,
                        prev_x: h.x,
                        prev_y: h.y,
                        angle: h.angle + rng.gen_range(-1.2..1.2),
                        alive: true,
                        energy: h.energy * 0.5,
                        parent: Some(idx),
                    });
                    h.energy *= 0.5;
                }

                // Draw line trail (white)
                let from = vec2(h.prev_x * CELL_SIZE, h.prev_y * CELL_SIZE);
                let to = vec2(h.x * CELL_SIZE, h.y * CELL_SIZE);
                segments.push(Segment { from, to, age: 0.0 });

                // let strength = nutrients[xi][yi];
                // let color = Color::new(0.8, 0.9, 1.0, (0.2 + strength * 0.8).min(1.0));
                // draw_line(from.x, from.y, to.x, to.y, 1.0 + strength * 2.0, color);
                let energy_color = Color::new(0.8, 0.9, 1.0, h.energy * 0.8 + 0.2);
                draw_line(
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    1.0 + h.energy * 2.0,
                    energy_color,
                );

                //draw_line(from.x, from.y, to.x, to.y, 1.5, WHITE);

                // Draw bright tip
                draw_circle(
                    h.x * CELL_SIZE,
                    h.y * CELL_SIZE,
                    2.5,
                    Color::new(1.0, 1.0, 1.0, 0.95),
                );
            }

            hyphae.extend(new_hyphae);

            // Fusion (anastomosis): create network connections when hyphae get close
            for i in 0..hyphae.len() {
                for j in (i + 1)..hyphae.len() {
                    if !hyphae[i].alive || !hyphae[j].alive {
                        continue;
                    }

                    let dx = hyphae[i].x - hyphae[j].x;
                    let dy = hyphae[i].y - hyphae[j].y;
                    let dist2 = dx * dx + dy * dy;
                    if dist2 < ANASTOMOSIS_DISTANCE_SQ {
                        // within ~2 units
                        // Check if connection already exists
                        let exists = connections.iter().any(|c| {
                            (c.hypha1 == i && c.hypha2 == j) || (c.hypha1 == j && c.hypha2 == i)
                        });

                        if !exists {
                            // Create new connection
                            connections.push(Connection {
                                hypha1: i,
                                hypha2: j,
                                strength: 1.0,
                            });

                            // Enable resource transport between connected hyphae
                            // Transfer energy from one to the other if imbalance
                            let energy_diff = hyphae[i].energy - hyphae[j].energy;
                            if energy_diff.abs() > 0.1 {
                                let transfer = energy_diff * 0.1;
                                hyphae[i].energy -= transfer;
                                hyphae[j].energy += transfer;
                                hyphae[i].energy = hyphae[i].energy.clamp(0.0, 1.0);
                                hyphae[j].energy = hyphae[j].energy.clamp(0.0, 1.0);
                            }
                        }
                    }
                }
            }

            // Remove connections to dead hyphae
            connections.retain(|c| {
                hyphae.get(c.hypha1).map(|h| h.alive).unwrap_or(false)
                    && hyphae.get(c.hypha2).map(|h| h.alive).unwrap_or(false)
            });

            let mut diffused = nutrients.clone();
            for x in 1..GRID_SIZE - 1 {
                for y in 1..GRID_SIZE - 1 {
                    let avg = (nutrients[x + 1][y]
                        + nutrients[x - 1][y]
                        + nutrients[x][y + 1]
                        + nutrients[x][y - 1])
                        * 0.25;
                    diffused[x][y] += DIFFUSION_RATE * (avg - nutrients[x][y]);
                }
            }
            nutrients = diffused;

            let mut new_hyphae_from_spores = vec![];
            for spore in &mut spores {
                if !spore.alive {
                    continue;
                }

                spore.x += spore.vx;
                spore.y += spore.vy;
                spore.age += 0.01;

                // Random drift
                spore.vx += rng.gen_range(-0.02..0.02);
                spore.vy += rng.gen_range(-0.02..0.02);

                // Bounds
                if spore.x < 1.0
                    || spore.x >= GRID_SIZE as f32 - 1.0
                    || spore.y < 1.0
                    || spore.y >= GRID_SIZE as f32 - 1.0
                {
                    spore.alive = false;
                    continue;
                }

                let xi = spore.x as usize;
                let yi = spore.y as usize;

                // Germinate in nutrient-rich zones
                if nutrients[xi][yi] > SPORE_GERMINATION_THRESHOLD {
                    new_hyphae_from_spores.push(Hypha {
                        x: spore.x,
                        y: spore.y,
                        prev_x: spore.x,
                        prev_y: spore.y,
                        angle: rng.gen_range(0.0..std::f32::consts::TAU),
                        alive: true,
                        energy: 0.5,
                        parent: None,
                    });
                    spore.alive = false;
                }

                // Fade spores visually
                draw_circle(
                    spore.x * CELL_SIZE,
                    spore.y * CELL_SIZE,
                    2.0,
                    Color::new(1.0, 0.8, 0.3, 0.5),
                );
            }

            hyphae.extend(new_hyphae_from_spores);
            spores.retain(|s| s.alive && s.age < SPORE_MAX_AGE);
        }

        // Calculate statistics
        let alive_hyphae: Vec<_> = hyphae.iter().filter(|h| h.alive).collect();
        let hyphae_count = alive_hyphae.len();
        let avg_energy = if hyphae_count > 0 {
            alive_hyphae.iter().map(|h| h.energy).sum::<f32>() / hyphae_count as f32
        } else {
            0.0
        };
        let spores_count = spores.iter().filter(|s| s.alive).count();
        let connections_count = connections.len();
        let fps = get_fps();

        // Draw statistics overlay
        let stats_text = format!(
            "Hyphae: {} | Spores: {} | Connections: {} | Avg Energy: {:.2} | FPS: {:.0}",
            hyphae_count, spores_count, connections_count, avg_energy, fps
        );
        draw_text(&stats_text, 10.0, 20.0, 20.0, WHITE);

        // Draw pause indicator and controls help
        if paused {
            draw_text("PAUSED - Press SPACE to resume", 10.0, 45.0, 20.0, YELLOW);
        }

        // Draw controls help
        let controls_text =
            "Controls: SPACE=Pause | R=Reset | C=Clear | S=Spawn | N=Nutrients | LMB=Add nutrient";
        draw_text(
            controls_text,
            10.0,
            screen_height() - 20.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, 0.7),
        );

        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Mycelium Growth Simulation".to_owned(),
        window_width: (GRID_SIZE as f32 * CELL_SIZE) as i32,
        window_height: (GRID_SIZE as f32 * CELL_SIZE) as i32,
        ..Default::default()
    }
}
