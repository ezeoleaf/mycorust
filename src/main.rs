use ::rand as external_rand;
use external_rand::{Rng, thread_rng};
use macroquad::prelude::*;

const GRID_SIZE: usize = 200;
const CELL_SIZE: f32 = 4.0;
const BRANCH_PROB: f32 = 0.02;
const STEP_SIZE: f32 = 0.5;
const NUTRIENT_DECAY: f32 = 0.002;

#[derive(Clone)]
struct Hypha {
    x: f32,
    y: f32,
    prev_x: f32,
    prev_y: f32,
    angle: f32,
    alive: bool,
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

    // --- Initialize hyphae ---
    let mut hyphae = vec![Hypha {
        x: GRID_SIZE as f32 / 2.0,
        y: GRID_SIZE as f32 / 2.0,
        prev_x: GRID_SIZE as f32 / 2.0,
        prev_y: GRID_SIZE as f32 / 2.0,
        angle: rng.gen_range(0.0..std::f32::consts::TAU),
        alive: true,
    }];

    // Accumulate drawn line segments so they persist frame-to-frame
    let mut segments: Vec<(Vec2, Vec2)> = Vec::new();

    loop {
        // Blue background every frame so it stays visible
        clear_background(Color::new(0.05, 0.10, 0.35, 1.0));

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

        // Redraw all past segments to keep trails visible
        for (from, to) in &segments {
            draw_line(from.x, from.y, to.x, to.y, 1.5, RED);
        }

        let mut new_hyphae = vec![];

        for h in &mut hyphae {
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
            nutrients[xi][yi] = (nutrients[xi][yi] - NUTRIENT_DECAY).max(0.0);

            // Branch occasionally
            if rng.r#gen::<f32>() < BRANCH_PROB {
                new_hyphae.push(Hypha {
                    x: h.x,
                    y: h.y,
                    prev_x: h.x,
                    prev_y: h.y,
                    angle: h.angle + rng.gen_range(-1.2..1.2),
                    alive: true,
                });
            }

            // Draw line trail (white)
            let from = vec2(h.prev_x * CELL_SIZE, h.prev_y * CELL_SIZE);
            let to = vec2(h.x * CELL_SIZE, h.y * CELL_SIZE);
            segments.push((from, to));
            draw_line(from.x, from.y, to.x, to.y, 1.5, WHITE);

            // Draw bright tip
            draw_circle(
                h.x * CELL_SIZE,
                h.y * CELL_SIZE,
                2.5,
                Color::new(1.0, 1.0, 1.0, 0.95),
            );

            for x in 0..GRID_SIZE {
                for y in 0..GRID_SIZE {
                    nutrients[x][y] *= 0.9995; // slow natural decay
                }
            }
        }

        hyphae.extend(new_hyphae);

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
