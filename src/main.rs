use ::rand as external_rand;
use external_rand::{thread_rng, Rng};
use macroquad::prelude::*;

mod config;
mod hypha;
mod nutrients;
mod spore;
mod types;
mod visualization;

use config::*;
use hypha::Hypha;
use nutrients::nutrient_gradient;
use spore::Spore;
use types::{Connection, FruitBody, Segment};
use visualization::{
    draw_connections, draw_fruit_bodies, draw_nutrients, draw_obstacles, draw_segments,
    draw_stats_and_help,
};

// (constants moved to config)

// (types moved to modules)

// (nutrient helpers moved to nutrients)

#[inline]
fn in_bounds(x: f32, y: f32) -> bool {
    x >= 0.0 && y >= 0.0 && x < GRID_SIZE as f32 && y < GRID_SIZE as f32
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
    const INITIAL_HYPHAE_COUNT: usize = 5;
    let mut hyphae: Vec<Hypha> = Vec::with_capacity(INITIAL_HYPHAE_COUNT);
    for _ in 0..INITIAL_HYPHAE_COUNT {
        let cx = GRID_SIZE as f32 / 2.0 + rng.gen_range(-10.0..10.0);
        let cy = GRID_SIZE as f32 / 2.0 + rng.gen_range(-10.0..10.0);
        hyphae.push(Hypha {
            x: cx,
            y: cy,
            prev_x: cx,
            prev_y: cy,
            angle: rng.gen_range(0.0..std::f32::consts::TAU),
            alive: true,
            energy: 0.5,
            parent: None,
            age: 0.0,
        });
    }

    // --- Initialize spores ---
    let mut spores: Vec<Spore> = Vec::new();

    // Accumulate drawn line segments so they persist frame-to-frame
    let mut segments: Vec<Segment> = Vec::new();

    // Pause/play state
    let mut paused = false;

    // Anastomosis connections
    let mut connections: Vec<Connection> = Vec::new();

    // Fruiting state
    let mut fruit_bodies: Vec<FruitBody> = Vec::new();
    let mut fruit_cooldown_timer: f32 = 0.0;

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
                age: 0.0,
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
                age: 0.0,
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
        draw_nutrients(&nutrients);

        // Draw obstacles
        draw_obstacles(&obstacles);

        // Redraw all past segments to keep trails visible (with fading)
        draw_segments(&segments);

        // Draw anastomosis connections
        draw_connections(&connections, &hyphae);

        // Draw fruiting bodies
        draw_fruit_bodies(&fruit_bodies);

        // Update simulation only if not paused
        if !paused {
            // Age all segments
            for segment in &mut segments {
                segment.age += SEGMENT_AGE_INCREMENT;
            }
            // Remove old segments
            segments.retain(|s| s.age < MAX_SEGMENT_AGE);

            let mut new_hyphae = vec![];
            let mut energy_transfers: Vec<(usize, usize, f32)> = Vec::new();
            let hyphae_len = hyphae.len();

            // Collect all hyphae info first to avoid borrow conflicts
            let hyphae_info: Vec<(f32, f32, bool, f32)> = hyphae
                .iter()
                .map(|h| (h.x, h.y, h.alive, h.energy))
                .collect();

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

                // Hyphal density slowing: count neighbors nearby and reduce effective step
                let mut neighbor_count = 0.0f32;
                for (ox, oy, other_alive, _) in &hyphae_info {
                    if !*other_alive {
                        continue;
                    }
                    let dx = h.x - *ox;
                    let dy = h.y - *oy;
                    if dx * dx + dy * dy < HYPHAE_AVOIDANCE_DISTANCE_SQ * 4.0 {
                        neighbor_count += 1.0;
                    }
                }
                let density_slow = 1.0 / (1.0 + 0.05 * neighbor_count);

                // Hyphae avoidance: check if new position would be too close to another hypha
                let new_x = h.x + h.angle.cos() * STEP_SIZE * density_slow;
                let new_y = h.y + h.angle.sin() * STEP_SIZE * density_slow;
                let mut too_close = false;

                for (other_idx, (ox, oy, other_alive, _)) in hyphae_info.iter().enumerate() {
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
                h.x += h.angle.cos() * STEP_SIZE * density_slow;
                h.y += h.angle.sin() * STEP_SIZE * density_slow;

                // Check if new position is in an obstacle
                let xi = h.x as usize;
                let yi = h.y as usize;
                if in_bounds(h.x, h.y) && obstacles[xi][yi] {
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
                        if in_bounds(test_x, test_y) && !obstacles[test_xi][test_yi] {
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

                // Gradual energy decay and aging
                h.energy *= ENERGY_DECAY_RATE;
                h.age += 0.01;

                // Die if energy depleted
                if h.energy < MIN_ENERGY_TO_LIVE {
                    h.alive = false;
                    continue;
                }

                // Queue energy transport to parent if exists (distance-attenuated)
                if let Some(parent_idx) = h.parent {
                    if parent_idx < hyphae_len {
                        let (px, py, parent_alive, parent_energy) = hyphae_info[parent_idx];
                        if !parent_alive { /* skip */
                        } else {
                            let dx = h.x - px;
                            let dy = h.y - py;
                            let dist = (dx * dx + dy * dy).sqrt();
                            let max_dist = 6.0f32;
                            if dist < max_dist {
                                let transfer_rate = 0.002 * (1.0 - dist / max_dist).max(0.0);
                                let wanted = (h.energy - parent_energy) * 0.5;
                                let transfer = (wanted * transfer_rate).clamp(-0.01, 0.01);
                                if transfer.abs() > 0.0 {
                                    energy_transfers.push((idx, parent_idx, transfer));
                                }
                            }
                        }
                    }
                }

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
                let age_branch_boost = (1.0 + h.age * 0.05).min(2.0);
                if rng.r#gen::<f32>() < BRANCH_PROB * age_branch_boost {
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
                        age: 0.0,
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
                let thickness = (1.0 + h.energy * 2.0) * (1.0 + (h.age * 0.2).min(1.5));
                draw_line(from.x, from.y, to.x, to.y, thickness, energy_color);

                //draw_line(from.x, from.y, to.x, to.y, 1.5, WHITE);

                // Draw bright tip
                // draw_circle(
                //     h.x * CELL_SIZE,
                //     h.y * CELL_SIZE,
                //     2.5,
                //     Color::new(1.0, 1.0, 1.0, 0.95),
                // );
            }

            // Apply queued energy transfers safely after iteration
            for (from, to, amount) in energy_transfers {
                if from < hyphae.len() && to < hyphae.len() {
                    hyphae[from].energy = (hyphae[from].energy - amount).clamp(0.0, 1.0);
                    hyphae[to].energy = (hyphae[to].energy + amount).clamp(0.0, 1.0);
                }
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
                        age: 0.0,
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
        let total_energy: f32 = alive_hyphae.iter().map(|h| h.energy).sum();
        let avg_energy = if hyphae_count > 0 {
            total_energy / hyphae_count as f32
        } else {
            0.0
        };
        let spores_count = spores.iter().filter(|s| s.alive).count();
        let connections_count = connections.len();
        let fps = get_fps();

        // Fruiting logic: cooldown and spawn based on network size and energy
        if !paused {
            fruit_cooldown_timer = (fruit_cooldown_timer - 1.0 / fps.max(1) as f32).max(0.0);
            if fruit_cooldown_timer <= 0.0
                && hyphae_count >= config::FruitingConfig::MIN_HYPHAE
                && total_energy >= config::FruitingConfig::THRESHOLD_TOTAL_ENERGY
            {
                // energy-weighted center
                let mut cx = 0.0f32;
                let mut cy = 0.0f32;
                for h in &alive_hyphae {
                    cx += h.x * h.energy;
                    cy += h.y * h.energy;
                }
                if total_energy > 0.0 {
                    cx /= total_energy;
                    cy /= total_energy;
                } else if let Some(first) = alive_hyphae.first() {
                    cx = first.x;
                    cy = first.y;
                }
                fruit_bodies.push(FruitBody {
                    x: cx,
                    y: cy,
                    age: 0.0,
                });
                fruit_cooldown_timer = config::FruitingConfig::COOLDOWN;
            }
        }

        // Age fruiting bodies
        for f in &mut fruit_bodies {
            f.age += 0.01;
        }

        // Draw statistics overlay and help
        draw_stats_and_help(
            hyphae_count,
            spores_count,
            connections_count,
            fruit_bodies.len(),
            avg_energy,
            paused,
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
