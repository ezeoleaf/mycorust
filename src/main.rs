use ::rand as external_rand;
use external_rand::{Rng, thread_rng};
use macroquad::prelude::*;

const GRID_SIZE: usize = 200;
const CELL_SIZE: f32 = 4.0;
const BRANCH_PROB: f32 = 0.002;
const STEP_SIZE: f32 = 0.5;
const NUTRIENT_DECAY: f32 = 0.001;

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
    for _ in 0..100 {
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
    let mut segments: Vec<(Vec2, Vec2)> = Vec::new();

    loop {
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

        // Redraw all past segments to keep trails visible
        for (from, to) in &segments {
            draw_line(from.x, from.y, to.x, to.y, 1.5, WHITE);
        }

        let mut new_hyphae = vec![];
        let hyphae_len = hyphae.len();
        for h in &mut hyphae[..hyphae_len] {
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
                h.angle += (grad_angle - h.angle) * 0.1;
            }

            // Small random wander to avoid directional lock-in
            h.angle += rng.gen_range(-0.05..0.05);

            let xi = h.x as usize;
            let yi = h.y as usize;
            if obstacles[xi][yi] {
                // bounce back or die
                h.angle += std::f32::consts::PI / 2.0;
                h.x = h.prev_x;
                h.y = h.prev_y;
                continue;
            }

            // Move
            h.x += h.angle.cos() * STEP_SIZE;
            h.y += h.angle.sin() * STEP_SIZE;

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
                let absorbed = n.min(0.002);
                h.energy = (h.energy + absorbed).min(1.0);
                nutrients[xi][yi] -= absorbed;
            }

            // Gradual energy decay
            h.energy *= 0.999;

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
            segments.push((from, to));

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

        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                nutrients[x][y] *= 0.9995; // slow natural decay
            }
        }

        // Fusion (anastomosis): merge hyphae that get close
        for i in 0..hyphae.len() {
            for j in (i + 1)..hyphae.len() {
                let dx = hyphae[i].x - hyphae[j].x;
                let dy = hyphae[i].y - hyphae[j].y;
                let dist2 = dx * dx + dy * dy;
                if dist2 < 4.0 {
                    // within ~2 units
                    // Connect by adding a visible connection and stop their movement temporarily
                    segments.push((
                        vec2(hyphae[i].x * CELL_SIZE, hyphae[i].y * CELL_SIZE),
                        vec2(hyphae[j].x * CELL_SIZE, hyphae[j].y * CELL_SIZE),
                    ));

                    // reduce movement (simulate resource exchange)
                    hyphae[i].alive = rng.gen_bool(0.95);
                    hyphae[j].alive = rng.gen_bool(0.95);
                }
            }
        }

        let mut diffused = nutrients.clone();
        for x in 1..GRID_SIZE - 1 {
            for y in 1..GRID_SIZE - 1 {
                let avg = (nutrients[x + 1][y]
                    + nutrients[x - 1][y]
                    + nutrients[x][y + 1]
                    + nutrients[x][y - 1])
                    * 0.25;
                diffused[x][y] += 0.05 * (avg - nutrients[x][y]);
            }
        }
        nutrients = diffused;

        hyphae.extend(new_hyphae);

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
            if nutrients[xi][yi] > 0.6 {
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
        spores.retain(|s| s.alive && s.age < 5.0);

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
