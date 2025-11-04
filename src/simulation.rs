use macroquad::prelude::*;
use rand::Rng;

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
        }
    }

    pub fn toggle_pause(&mut self) { self.paused = !self.paused; }
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
    pub fn clear_segments(&mut self) { self.segments.clear(); }
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
                if dist < 3.0 { self.nutrients[nx][ny] = 1.0; }
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
        let avg_energy = if hyphae_count > 0 { total_energy / hyphae_count as f32 } else { 0.0 };
        let spores_count = self.spores.iter().filter(|s| s.alive).count();
        let connections_count = self.connections.len();
        (hyphae_count, spores_count, connections_count, self.fruit_bodies.len(), avg_energy, total_energy)
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) {
        // Age segments
        for segment in &mut self.segments { segment.age += SEGMENT_AGE_INCREMENT; }
        self.segments.retain(|s| s.age < MAX_SEGMENT_AGE);

        let mut new_hyphae = vec![];
        let mut energy_transfers: Vec<(usize, usize, f32)> = Vec::new();
        let hyphae_len = self.hyphae.len();

        let hyphae_info: Vec<(f32, f32, bool, f32)> = self.hyphae
            .iter()
            .map(|h| (h.x, h.y, h.alive, h.energy))
            .collect();

        for (idx, h) in self.hyphae[..hyphae_len].iter_mut().enumerate() {
            if !h.alive { continue; }

            h.prev_x = h.x; h.prev_y = h.y;

            let (gx, gy) = nutrient_gradient(&self.nutrients, h.x, h.y);
            let grad_mag = (gx * gx + gy * gy).sqrt();
            if grad_mag > 1e-6 {
                let grad_angle = gy.atan2(gx);
                h.angle += (grad_angle - h.angle) * GRADIENT_STEERING_STRENGTH;
            }
            h.angle += rng.gen_range(-ANGLE_WANDER_RANGE..ANGLE_WANDER_RANGE);

            let mut neighbor_count = 0.0f32;
            for (ox, oy, other_alive, _) in &hyphae_info {
                if !*other_alive { continue; }
                let dx = h.x - *ox; let dy = h.y - *oy;
                if dx * dx + dy * dy < HYPHAE_AVOIDANCE_DISTANCE_SQ * 4.0 { neighbor_count += 1.0; }
            }
            let density_slow = 1.0 / (1.0 + 0.05 * neighbor_count);

            let new_x = h.x + h.angle.cos() * STEP_SIZE * density_slow;
            let new_y = h.y + h.angle.sin() * STEP_SIZE * density_slow;
            let mut too_close = false;
            for (other_idx, (ox, oy, other_alive, _)) in hyphae_info.iter().enumerate() {
                if other_idx == idx || !other_alive { continue; }
                let dx = new_x - ox; let dy = new_y - oy; let dist2 = dx * dx + dy * dy;
                if dist2 < HYPHAE_AVOIDANCE_DISTANCE_SQ && dist2 > 0.001 { too_close = true; break; }
            }
            if too_close { h.angle += rng.gen_range(-0.5..0.5); }

            h.x += h.angle.cos() * STEP_SIZE * density_slow;
            h.y += h.angle.sin() * STEP_SIZE * density_slow;

            let xi = h.x as usize; let yi = h.y as usize;
            if in_bounds(h.x, h.y) && self.obstacles[xi][yi] {
                h.x = h.prev_x; h.y = h.prev_y;
                let mut found_clear = false; let mut best_angle = h.angle; let mut attempts = 0;
                while !found_clear && attempts < 8 {
                    let test_angle = h.angle + (attempts as f32) * std::f32::consts::PI / 4.0;
                    let test_x = h.x + test_angle.cos() * STEP_SIZE;
                    let test_y = h.y + test_angle.sin() * STEP_SIZE;
                    let test_xi = test_x as usize; let test_yi = test_y as usize;
                    if in_bounds(test_x, test_y) && !self.obstacles[test_xi][test_yi] {
                        best_angle = test_angle; found_clear = true;
                    }
                    attempts += 1;
                }
                if found_clear { h.angle = best_angle + rng.gen_range(-0.2..0.2); }
                else { h.angle += std::f32::consts::PI + rng.gen_range(-0.5..0.5); }
                h.angle = h.angle % std::f32::consts::TAU; if h.angle < 0.0 { h.angle += std::f32::consts::TAU; }
                h.x += h.angle.cos() * STEP_SIZE; h.y += h.angle.sin() * STEP_SIZE;
            }

            if h.x < 1.0 || h.x >= GRID_SIZE as f32 - 1.0 || h.y < 1.0 || h.y >= GRID_SIZE as f32 - 1.0 {
                h.x = h.prev_x; h.y = h.prev_y;
                let min_b = 1.0; let max_b = GRID_SIZE as f32 - 2.0;
                if h.x <= min_b { h.x = min_b; h.angle = std::f32::consts::PI - h.angle; }
                else if h.x >= max_b { h.x = max_b; h.angle = std::f32::consts::PI - h.angle; }
                if h.y <= min_b { h.y = min_b; h.angle = -h.angle; }
                else if h.y >= max_b { h.y = max_b; h.angle = -h.angle; }
                h.angle += rng.gen_range(-0.15..0.15);
                h.x += h.angle.cos() * STEP_SIZE; h.y += h.angle.sin() * STEP_SIZE;
                h.x = h.x.clamp(min_b, max_b); h.y = h.y.clamp(min_b, max_b);
            }

            let xi = h.x as usize; let yi = h.y as usize;
            let n = self.nutrients[xi][yi];
            if n > 0.001 {
                let absorbed = n.min(NUTRIENT_DECAY);
                h.energy = (h.energy + absorbed).min(1.0);
                self.nutrients[xi][yi] -= absorbed;
            }

            h.energy *= ENERGY_DECAY_RATE; h.age += 0.01;
            if h.energy < MIN_ENERGY_TO_LIVE { h.alive = false; continue; }

            if let Some(parent_idx) = h.parent {
                if parent_idx < hyphae_len {
                    let (px, py, parent_alive, parent_energy) = hyphae_info[parent_idx];
                    if parent_alive {
                        let dx = h.x - px; let dy = h.y - py; let dist = (dx * dx + dy * dy).sqrt();
                        let max_dist = 6.0f32;
                        if dist < max_dist {
                            let transfer_rate = 0.002 * (1.0 - dist / max_dist).max(0.0);
                            let wanted = (h.energy - parent_energy) * 0.5;
                            let transfer = (wanted * transfer_rate).clamp(-0.01, 0.01);
                            if transfer.abs() > 0.0 { energy_transfers.push((idx, parent_idx, transfer)); }
                        }
                    }
                }
            }

            if n < 0.05 && rng.gen_bool(0.001) {
                self.spores.push(Spore { x: h.x, y: h.y, vx: rng.gen_range(-0.5..0.5), vy: rng.gen_range(-0.5..0.5), alive: true, age: 0.0 });
            }

            let age_branch_boost = (1.0 + h.age * 0.05).min(2.0);
            if rng.gen::<f32>() < BRANCH_PROB * age_branch_boost {
                let idxp = hyphae_len;
                new_hyphae.push(Hypha { x: h.x, y: h.y, prev_x: h.x, prev_y: h.y, angle: h.angle + rng.gen_range(-1.2..1.2), alive: true, energy: h.energy * 0.5, parent: Some(idxp), age: 0.0 });
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
                if !self.hyphae[i].alive || !self.hyphae[j].alive { continue; }
                let dx = self.hyphae[i].x - self.hyphae[j].x;
                let dy = self.hyphae[i].y - self.hyphae[j].y;
                let dist2 = dx * dx + dy * dy;
                if dist2 < ANASTOMOSIS_DISTANCE_SQ {
                    let exists = self.connections.iter().any(|c| (c.hypha1 == i && c.hypha2 == j) || (c.hypha1 == j && c.hypha2 == i));
                    if !exists {
                        self.connections.push(Connection { hypha1: i, hypha2: j, strength: 1.0 });
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
        self.connections.retain(|c| self.hyphae.get(c.hypha1).map(|h| h.alive).unwrap_or(false) && self.hyphae.get(c.hypha2).map(|h| h.alive).unwrap_or(false));

        // diffuse nutrients
        let mut diffused = self.nutrients.clone();
        for x in 1..GRID_SIZE - 1 {
            for y in 1..GRID_SIZE - 1 {
                let avg = (self.nutrients[x + 1][y] + self.nutrients[x - 1][y] + self.nutrients[x][y + 1] + self.nutrients[x][y - 1]) * 0.25;
                diffused[x][y] += DIFFUSION_RATE * (avg - self.nutrients[x][y]);
            }
        }
        self.nutrients = diffused;

        // spores
        let mut new_hyphae_from_spores = vec![];
        for spore in &mut self.spores {
            if !spore.alive { continue; }
            spore.x += spore.vx; spore.y += spore.vy; spore.age += 0.01;
            spore.vx += rng.gen_range(-0.02..0.02); spore.vy += rng.gen_range(-0.02..0.02);
            if spore.x < 1.0 || spore.x >= GRID_SIZE as f32 - 1.0 || spore.y < 1.0 || spore.y >= GRID_SIZE as f32 - 1.0 { spore.alive = false; continue; }
            let xi = spore.x as usize; let yi = spore.y as usize;
            if self.nutrients[xi][yi] > SPORE_GERMINATION_THRESHOLD {
                new_hyphae_from_spores.push(Hypha { x: spore.x, y: spore.y, prev_x: spore.x, prev_y: spore.y, angle: rng.gen_range(0.0..std::f32::consts::TAU), alive: true, energy: 0.5, parent: None, age: 0.0 });
                spore.alive = false;
            }
        }
        self.hyphae.extend(new_hyphae_from_spores);
        self.spores.retain(|s| s.alive && s.age < SPORE_MAX_AGE);

        // fruiting
        let (hyphae_count, _spores_count, _conn_count, _fruit_count, _avg_energy, total_energy) = self.stats();
        let fps = get_fps();
        self.fruit_cooldown_timer = (self.fruit_cooldown_timer - 1.0 / fps.max(1) as f32).max(0.0);
        if self.fruit_cooldown_timer <= 0.0
            && hyphae_count >= FruitingConfig::MIN_HYPHAE
            && total_energy >= FruitingConfig::THRESHOLD_TOTAL_ENERGY
        {
            let alive_hyphae: Vec<_> = self.hyphae.iter().filter(|h| h.alive).collect();
            let mut cx = 0.0f32; let mut cy = 0.0f32;
            for h in &alive_hyphae { cx += h.x * h.energy; cy += h.y * h.energy; }
            if total_energy > 0.0 { cx /= total_energy; cy /= total_energy; }
            else if let Some(first) = alive_hyphae.first() { cx = first.x; cy = first.y; }
            self.fruit_bodies.push(FruitBody { x: cx, y: cy, age: 0.0 });
            self.fruit_cooldown_timer = FruitingConfig::COOLDOWN;
        }
        for f in &mut self.fruit_bodies { f.age += 0.01; }
    }
}


