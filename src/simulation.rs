use ::rand as external_rand;
use external_rand::Rng;
use macroquad::prelude::*;

use crate::config::*;
use crate::hypha::Hypha;
use crate::nutrients::nutrient_gradient;
use crate::spore::Spore;
use crate::types::{Connection, FruitBody, Segment};

#[inline]
fn in_bounds(x: f32, y: f32) -> bool {
    x >= 0.0 && y >= 0.0 && x < GRID_SIZE as f32 && y < GRID_SIZE as f32
}

pub struct Simulation {
    pub nutrients: [[f32; GRID_SIZE]; GRID_SIZE],
    pub obstacles: [[bool; GRID_SIZE]; GRID_SIZE],
    pub hyphae: Vec<Hypha>,
    pub spores: Vec<Spore>,
    pub segments: Vec<Segment>,
    pub connections: Vec<Connection>,
    pub fruit_bodies: Vec<FruitBody>,
    pub fruit_cooldown_timer: f32,
    pub paused: bool,
    pub connections_visible: bool,
    pub frame_index: u64,
}

impl Simulation {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
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

        Self {
            nutrients,
            obstacles,
            hyphae,
            spores: Vec::new(),
            segments: Vec::new(),
            connections: Vec::new(),
            fruit_bodies: Vec::new(),
            fruit_cooldown_timer: 0.0,
            paused: false,
            connections_visible: true,
            frame_index: 0,
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
    pub fn toggle_connections(&mut self) {
        self.connections_visible = !self.connections_visible;
    }
    pub fn reset<R: Rng>(&mut self, rng: &mut R) {
        self.hyphae.clear();
        self.spores.clear();
        self.segments.clear();
        self.connections.clear();
        self.fruit_bodies.clear();
        self.fruit_cooldown_timer = 0.0;
        let cx = GRID_SIZE as f32 / 2.0;
        let cy = GRID_SIZE as f32 / 2.0;
        self.hyphae.push(Hypha {
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
    pub fn clear_segments(&mut self) {
        self.segments.clear();
    }
    pub fn spawn_hypha_at<R: Rng>(&mut self, rng: &mut R, gx: f32, gy: f32) {
        self.hyphae.push(Hypha {
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
    pub fn add_nutrient_patch(&mut self, gx: usize, gy: usize) {
        for dx in -3..=3 {
            for dy in -3..=3 {
                let nx = (gx as i32 + dx).max(0).min(GRID_SIZE as i32 - 1) as usize;
                let ny = (gy as i32 + dy).max(0).min(GRID_SIZE as i32 - 1) as usize;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 3.0 {
                    self.nutrients[nx][ny] = 1.0;
                }
            }
        }
    }
    pub fn add_nutrient_cell(&mut self, gx: usize, gy: usize) {
        self.nutrients[gx][gy] = 1.0;
    }

    pub fn stats(&self) -> (usize, usize, usize, usize, f32, f32) {
        let alive_hyphae: Vec<_> = self.hyphae.iter().filter(|h| h.alive).collect();
        let hyphae_count = alive_hyphae.len();
        let total_energy: f32 = alive_hyphae.iter().map(|h| h.energy).sum();
        let avg_energy = if hyphae_count > 0 {
            total_energy / hyphae_count as f32
        } else {
            0.0
        };
        let spores_count = self.spores.iter().filter(|s| s.alive).count();
        let connections_count = self.connections.len();
        (
            hyphae_count,
            spores_count,
            connections_count,
            self.fruit_bodies.len(),
            avg_energy,
            total_energy,
        )
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) {
        self.frame_index = self.frame_index.wrapping_add(1);

        // Age segments
        for segment in &mut self.segments {
            segment.age += SEGMENT_AGE_INCREMENT;
        }
        self.segments.retain(|s| s.age < MAX_SEGMENT_AGE);

        let mut new_hyphae = vec![];
        let mut energy_transfers: Vec<(usize, usize, f32)> = Vec::new();
        let hyphae_len = self.hyphae.len();

        let hyphae_info: Vec<(f32, f32, bool, f32)> = self
            .hyphae
            .iter()
            .map(|h| (h.x, h.y, h.alive, h.energy))
            .collect();

        // Spatial hash grid for neighbor queries
        let cell_size: f32 = 4.0;
        let nx = ((GRID_SIZE as f32) / cell_size).ceil() as usize;
        let ny = ((GRID_SIZE as f32) / cell_size).ceil() as usize;
        let mut buckets: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); ny]; nx];
        for (i, (x, y, alive, _)) in hyphae_info.iter().enumerate() {
            if !*alive {
                continue;
            }
            let bx = (*x / cell_size).floor() as isize;
            let by = (*y / cell_size).floor() as isize;
            if bx >= 0 && by >= 0 {
                let bxu = bx as usize;
                let byu = by as usize;
                if bxu < nx && byu < ny {
                    buckets[bxu][byu].push(i);
                }
            }
        }

        for (idx, h) in self.hyphae[..hyphae_len].iter_mut().enumerate() {
            if !h.alive {
                continue;
            }

            h.prev_x = h.x;
            h.prev_y = h.y;

            let (mut gx, mut gy) = nutrient_gradient(&self.nutrients, h.x, h.y);
            let grad_mag = (gx * gx + gy * gy).sqrt();
            // Only apply gradient steering if gradient is significant (avoid noise from small gradients)
            const MIN_GRADIENT_MAG: f32 = 0.08; // Threshold to ignore numerical noise (increased to avoid bias)
            if grad_mag > MIN_GRADIENT_MAG {
                // Add tropism global bias only when gradient is weak (subtle drift)
                if grad_mag < 0.15 {
                    let tx = TROPISM_ANGLE.cos() * TROPISM_STRENGTH;
                    let ty = TROPISM_ANGLE.sin() * TROPISM_STRENGTH;
                    gx += tx;
                    gy += ty;
                    let new_grad_mag = (gx * gx + gy * gy).sqrt();
                    if new_grad_mag > MIN_GRADIENT_MAG {
                        let grad_angle = gy.atan2(gx);
                        h.angle += (grad_angle - h.angle) * GRADIENT_STEERING_STRENGTH;
                    }
                } else {
                    let grad_angle = gy.atan2(gx);
                    h.angle += (grad_angle - h.angle) * GRADIENT_STEERING_STRENGTH;
                }
            }
            // Apply random wander - this should be symmetric, but increase it slightly when gradient is weak
            let wander_boost = if grad_mag < MIN_GRADIENT_MAG {
                1.5
            } else {
                1.0
            };
            h.angle += rng.gen_range(-ANGLE_WANDER_RANGE..ANGLE_WANDER_RANGE) * wander_boost;

            let mut neighbor_count = 0.0f32;
            let bx = (h.x / cell_size).floor() as isize;
            let by = (h.y / cell_size).floor() as isize;
            for gx in (bx - 1)..=(bx + 1) {
                for gy in (by - 1)..=(by + 1) {
                    if gx < 0 || gy < 0 {
                        continue;
                    }
                    let gux = gx as usize;
                    let guy = gy as usize;
                    if gux >= nx || guy >= ny {
                        continue;
                    }
                    for &other_idx in &buckets[gux][guy] {
                        if other_idx == idx {
                            continue;
                        }
                        let (ox, oy, other_alive, _) = hyphae_info[other_idx];
                        if !other_alive {
                            continue;
                        }
                        let dx = h.x - ox;
                        let dy = h.y - oy;
                        if dx * dx + dy * dy < HYPHAE_AVOIDANCE_DISTANCE_SQ * 4.0 {
                            neighbor_count += 1.0;
                        }
                    }
                }
            }
            let density_slow = 1.0 / (1.0 + 0.05 * neighbor_count);

            let new_x = h.x + h.angle.cos() * STEP_SIZE * density_slow;
            let new_y = h.y + h.angle.sin() * STEP_SIZE * density_slow;
            let mut too_close = false;
            for gx in (bx - 1)..=(bx + 1) {
                for gy in (by - 1)..=(by + 1) {
                    if gx < 0 || gy < 0 {
                        continue;
                    }
                    let gux = gx as usize;
                    let guy = gy as usize;
                    if gux >= nx || guy >= ny {
                        continue;
                    }
                    for &other_idx in &buckets[gux][guy] {
                        if other_idx == idx {
                            continue;
                        }
                        let (ox, oy, other_alive, _) = hyphae_info[other_idx];
                        if !other_alive {
                            continue;
                        }
                        let dx = new_x - ox;
                        let dy = new_y - oy;
                        let dist2 = dx * dx + dy * dy;
                        if dist2 < HYPHAE_AVOIDANCE_DISTANCE_SQ && dist2 > 0.001 {
                            too_close = true;
                            break;
                        }
                    }
                    if too_close {
                        break;
                    }
                }
                if too_close {
                    break;
                }
            }
            if too_close {
                h.angle += rng.gen_range(-0.5..0.5);
            }

            h.x += h.angle.cos() * STEP_SIZE * density_slow;
            h.y += h.angle.sin() * STEP_SIZE * density_slow;

            let xi = h.x as usize;
            let yi = h.y as usize;
            if in_bounds(h.x, h.y) && self.obstacles[xi][yi] {
                h.x = h.prev_x;
                h.y = h.prev_y;
                let mut found_clear = false;
                let mut best_angle = h.angle;
                let mut attempts = 0;
                while !found_clear && attempts < 8 {
                    let test_angle = h.angle + (attempts as f32) * std::f32::consts::PI / 4.0;
                    let test_x = h.x + test_angle.cos() * STEP_SIZE;
                    let test_y = h.y + test_angle.sin() * STEP_SIZE;
                    let test_xi = test_x as usize;
                    let test_yi = test_y as usize;
                    if in_bounds(test_x, test_y) && !self.obstacles[test_xi][test_yi] {
                        best_angle = test_angle;
                        found_clear = true;
                    }
                    attempts += 1;
                }
                if found_clear {
                    h.angle = best_angle + rng.gen_range(-0.2..0.2);
                } else {
                    h.angle += std::f32::consts::PI + rng.gen_range(-0.5..0.5);
                }
                h.angle = h.angle % std::f32::consts::TAU;
                if h.angle < 0.0 {
                    h.angle += std::f32::consts::TAU;
                }
                h.x += h.angle.cos() * STEP_SIZE;
                h.y += h.angle.sin() * STEP_SIZE;
            }

            if h.x < 1.0
                || h.x >= GRID_SIZE as f32 - 1.0
                || h.y < 1.0
                || h.y >= GRID_SIZE as f32 - 1.0
            {
                h.x = h.prev_x;
                h.y = h.prev_y;
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
                h.angle += rng.gen_range(-0.15..0.15);
                h.x += h.angle.cos() * STEP_SIZE;
                h.y += h.angle.sin() * STEP_SIZE;
                h.x = h.x.clamp(min_b, max_b);
                h.y = h.y.clamp(min_b, max_b);
            }

            let xi = h.x as usize;
            let yi = h.y as usize;
            let n = self.nutrients[xi][yi];
            if n > 0.001 {
                let absorbed = n.min(NUTRIENT_DECAY);
                h.energy = (h.energy + absorbed).min(1.0);
                self.nutrients[xi][yi] -= absorbed;
            }

            h.energy *= ENERGY_DECAY_RATE;
            h.age += 0.01;
            if h.energy < MIN_ENERGY_TO_LIVE {
                h.alive = false;
                continue;
            }

            if let Some(parent_idx) = h.parent {
                if parent_idx < hyphae_len {
                    let (px, py, parent_alive, parent_energy) = hyphae_info[parent_idx];
                    if parent_alive {
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
                self.spores.push(Spore {
                    x: h.x,
                    y: h.y,
                    vx: rng.gen_range(-0.5..0.5),
                    vy: rng.gen_range(-0.5..0.5),
                    alive: true,
                    age: 0.0,
                });
            }

            let age_branch_boost = (1.0 + h.age * 0.05).min(2.0);
            if rng.gen::<f32>() < BRANCH_PROB * age_branch_boost {
                let idxp = hyphae_len;
                new_hyphae.push(Hypha {
                    x: h.x,
                    y: h.y,
                    prev_x: h.x,
                    prev_y: h.y,
                    angle: h.angle + rng.gen_range(-1.2..1.2),
                    alive: true,
                    energy: h.energy * 0.5,
                    parent: Some(idxp),
                    age: 0.0,
                });
                h.energy *= 0.5;
            }

            let from = vec2(h.prev_x * CELL_SIZE, h.prev_y * CELL_SIZE);
            let to = vec2(h.x * CELL_SIZE, h.y * CELL_SIZE);
            self.segments.push(Segment { from, to, age: 0.0 });
        }

        for (from, to, amount) in energy_transfers {
            if from < self.hyphae.len() && to < self.hyphae.len() {
                self.hyphae[from].energy = (self.hyphae[from].energy - amount).clamp(0.0, 1.0);
                self.hyphae[to].energy = (self.hyphae[to].energy + amount).clamp(0.0, 1.0);
            }
        }

        self.hyphae.extend(new_hyphae);

        // connections
        for i in 0..self.hyphae.len() {
            for j in (i + 1)..self.hyphae.len() {
                if !self.hyphae[i].alive || !self.hyphae[j].alive {
                    continue;
                }
                let dx = self.hyphae[i].x - self.hyphae[j].x;
                let dy = self.hyphae[i].y - self.hyphae[j].y;
                let dist2 = dx * dx + dy * dy;
                if dist2 < ANASTOMOSIS_DISTANCE_SQ {
                    let exists = self.connections.iter().any(|c| {
                        (c.hypha1 == i && c.hypha2 == j) || (c.hypha1 == j && c.hypha2 == i)
                    });
                    if !exists {
                        self.connections.push(Connection {
                            hypha1: i,
                            hypha2: j,
                        });
                        let energy_diff = self.hyphae[i].energy - self.hyphae[j].energy;
                        if energy_diff.abs() > 0.1 {
                            let transfer = energy_diff * 0.1;
                            self.hyphae[i].energy -= transfer;
                            self.hyphae[j].energy += transfer;
                            self.hyphae[i].energy = self.hyphae[i].energy.clamp(0.0, 1.0);
                            self.hyphae[j].energy = self.hyphae[j].energy.clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }
        self.connections.retain(|c| {
            self.hyphae.get(c.hypha1).map(|h| h.alive).unwrap_or(false)
                && self.hyphae.get(c.hypha2).map(|h| h.alive).unwrap_or(false)
        });

        // Resource allocation along connections (diffusive flow)
        for c in &self.connections {
            let (i, j) = if c.hypha1 <= c.hypha2 {
                (c.hypha1, c.hypha2)
            } else {
                (c.hypha2, c.hypha1)
            };
            if j >= self.hyphae.len() {
                continue;
            }
            let (left, right) = self.hyphae.split_at_mut(j);
            let h1 = &mut left[i];
            let h2 = &mut right[0];
            if !h1.alive || !h2.alive {
                continue;
            }
            let diff = h1.energy - h2.energy;
            let flow = (diff * CONNECTION_FLOW_RATE).clamp(-0.02, 0.02);
            h1.energy = (h1.energy - flow).clamp(0.0, 1.0);
            h2.energy = (h2.energy + flow).clamp(0.0, 1.0);
        }

        // diffuse nutrients (LOD: bounding box + frame skipping)
        let do_diffuse = if get_fps() < 45 {
            (self.frame_index % 2) == 0
        } else {
            true
        };
        if do_diffuse {
            // Compute bounding box around alive hyphae
            let mut minx = GRID_SIZE - 2;
            let mut miny = GRID_SIZE - 2;
            let mut maxx = 1;
            let mut maxy = 1;
            for h in self.hyphae.iter().filter(|h| h.alive) {
                let xi = h.x as usize;
                let yi = h.y as usize;
                if xi > 0 && yi > 0 && xi < GRID_SIZE - 1 && yi < GRID_SIZE - 1 {
                    if xi < minx {
                        minx = xi;
                    }
                    if yi < miny {
                        miny = yi;
                    }
                    if xi > maxx {
                        maxx = xi;
                    }
                    if yi > maxy {
                        maxy = yi;
                    }
                }
            }
            let pad = 6usize;
            let x0 = 1.max(minx.saturating_sub(pad));
            let y0 = 1.max(miny.saturating_sub(pad));
            let x1 = (GRID_SIZE - 2).min(maxx.saturating_add(pad));
            let y1 = (GRID_SIZE - 2).min(maxy.saturating_add(pad));

            let mut diffused = self.nutrients.clone();
            for x in x0..=x1 {
                for y in y0..=y1 {
                    let avg = (self.nutrients[x + 1][y]
                        + self.nutrients[x - 1][y]
                        + self.nutrients[x][y + 1]
                        + self.nutrients[x][y - 1])
                        * 0.25;
                    diffused[x][y] += DIFFUSION_RATE * (avg - self.nutrients[x][y]);
                }
            }
            self.nutrients = diffused;
        }

        // spores
        let mut new_hyphae_from_spores = vec![];
        for spore in &mut self.spores {
            if !spore.alive {
                continue;
            }
            spore.x += spore.vx;
            spore.y += spore.vy;
            spore.age += 0.01;
            spore.vx += rng.gen_range(-0.02..0.02);
            spore.vy += rng.gen_range(-0.02..0.02);
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
            if self.nutrients[xi][yi] > SPORE_GERMINATION_THRESHOLD {
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
                // Particle burst at germination
                for k in 0..8 {
                    let a = (k as f32 / 8.0) * std::f32::consts::TAU + rng.gen_range(-0.2..0.2);
                    let r = rng.gen_range(2.0..5.0);
                    let px = spore.x * CELL_SIZE + a.cos() * r;
                    let py = spore.y * CELL_SIZE + a.sin() * r;
                    draw_circle(px, py, 1.5, Color::new(1.0, 0.8, 0.3, 0.6));
                }
            }
        }
        self.hyphae.extend(new_hyphae_from_spores);
        self.spores.retain(|s| s.alive && s.age < SPORE_MAX_AGE);

        // fruiting
        let (hyphae_count, _spores_count, _conn_count, _fruit_count, _avg_energy, total_energy) =
            self.stats();
        let fps = get_fps();
        self.fruit_cooldown_timer = (self.fruit_cooldown_timer - 1.0 / fps.max(1) as f32).max(0.0);
        if self.fruit_cooldown_timer <= 0.0
            && hyphae_count >= FruitingConfig::MIN_HYPHAE
            && total_energy >= FruitingConfig::THRESHOLD_TOTAL_ENERGY
        {
            let alive_hyphae: Vec<_> = self.hyphae.iter().filter(|h| h.alive).collect();
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
            self.fruit_bodies.push(FruitBody {
                x: cx,
                y: cy,
                age: 0.0,
            });
            self.fruit_cooldown_timer = FruitingConfig::COOLDOWN;
        }
        for f in &mut self.fruit_bodies {
            f.age += 0.01;
        }
    }
}
