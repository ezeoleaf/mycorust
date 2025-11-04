use macroquad::prelude::*;

use crate::config::*;
use crate::hypha::Hypha;
use crate::nutrients::nutrient_color;
use crate::types::{Connection, FruitBody, Segment};

pub fn draw_nutrients(nutrients: &[[f32; GRID_SIZE]; GRID_SIZE]) {
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
}

pub fn draw_obstacles(obstacles: &[[bool; GRID_SIZE]; GRID_SIZE]) {
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
}

pub fn draw_segments(segments: &Vec<Segment>) {
    for segment in segments {
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
}

pub fn draw_connections(connections: &Vec<Connection>, hyphae: &Vec<Hypha>) {
    for conn in connections {
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
}

pub fn draw_fruit_bodies(fruit_bodies: &Vec<FruitBody>) {
    for f in fruit_bodies {
        let stem_h = 10.0;
        let stem_w = 3.0;
        let px = f.x * CELL_SIZE;
        let py = f.y * CELL_SIZE;
        // stem
        draw_rectangle(
            px - stem_w / 2.0,
            py - stem_h,
            stem_w,
            stem_h,
            Color::new(0.9, 0.9, 0.8, 0.9),
        );
        // cap
        draw_circle(px, py - stem_h, 6.0, Color::new(0.8, 0.2, 0.2, 0.9));
    }
}

pub fn draw_stats_and_help(
    hyphae_count: usize,
    spores_count: usize,
    connections_count: usize,
    fruit_count: usize,
    avg_energy: f32,
    paused: bool,
) {
    let fps = get_fps();
    let stats_text = format!(
        "Hyphae: {} | Spores: {} | Connections: {} | Fruits: {} | Avg Energy: {:.2} | FPS: {:.0}",
        hyphae_count, spores_count, connections_count, fruit_count, avg_energy, fps
    );
    draw_text(&stats_text, 10.0, 20.0, 20.0, WHITE);
    if paused {
        draw_text("PAUSED - Press SPACE to resume", 10.0, 45.0, 20.0, YELLOW);
    }
    let controls_text =
        "Controls: SPACE=Pause | R=Reset | C=Clear | S=Spawn | N=Nutrients | LMB=Add nutrient";
    draw_text(
        controls_text,
        10.0,
        screen_height() - 20.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
}


