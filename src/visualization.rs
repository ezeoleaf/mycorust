use macroquad::prelude::*;

use crate::config::*;
use crate::hypha::Hypha;
use crate::nutrients::{nutrient_color, NutrientGrid};
use crate::types::{Connection, FruitBody, Segment};

pub fn draw_nutrients(nutrients: &NutrientGrid) {
    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            let sugar = nutrients.sugar[x][y];
            let nitrogen = nutrients.nitrogen[x][y];
            let color = nutrient_color(sugar, nitrogen);
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

// Network Intelligence: Draw memory overlay (subtle purple/blue tint)
pub fn draw_memory_overlay(memory: &[Vec<f32>], memory_visible: bool) {
    if !memory_visible {
        return;
    }
    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            let mem_val = memory[x][y];
            if mem_val > 0.01 {
                // Subtle purple/blue overlay for memory
                let alpha = mem_val * 0.3; // Subtle overlay
                let purple = Color::new(0.5, 0.2, 0.8, alpha);
                draw_rectangle(
                    x as f32 * CELL_SIZE,
                    y as f32 * CELL_SIZE,
                    CELL_SIZE,
                    CELL_SIZE,
                    purple,
                );
            }
        }
    }
}

pub fn draw_obstacles(obstacles: &[Vec<bool>]) {
    #[allow(clippy::needless_range_loop)]
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

pub fn draw_segments(segments: &[Segment], max_segment_age: f32, hyphae_visible: bool) {
    if !hyphae_visible || segments.is_empty() {
        return;
    }

    // Performance: LOD - only adjust when FPS is actually low
    let fps = get_fps();
    let step = if fps < 30 {
        3
    } else if fps < 45 {
        2
    } else {
        1
    };

    // Performance: Only do spatial culling if we have many segments (optimization)
    let use_culling = segments.len() > 500;
    let (min_x, min_y, max_x, max_y) = if use_culling {
        let screen_width = screen_width();
        let screen_height = screen_height();
        let margin = 100.0;
        (
            -margin,
            -margin,
            screen_width + margin,
            screen_height + margin,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0) // Dummy values, won't be used
    };

    for (i, segment) in segments.iter().enumerate() {
        // Performance: LOD - skip some segments
        if i % step != 0 {
            continue;
        }

        // Performance: Spatial culling (only if many segments)
        if use_culling {
            let mid_x = (segment.from.x + segment.to.x) * 0.5;
            let mid_y = (segment.from.y + segment.to.y) * 0.5;
            if mid_x < min_x || mid_x > max_x || mid_y < min_y || mid_y > max_y {
                continue;
            }
        }

        let age_factor = 1.0 - (segment.age / max_segment_age);
        let alpha = age_factor.clamp(0.0, 1.0);

        // Skip very transparent segments for performance
        if alpha < 0.05 {
            continue;
        }

        // Age-based coloring: young = white, old = dark gray/blue
        let age_normalized = (segment.age / max_segment_age).min(1.0);
        let r = 1.0 - age_normalized * 0.7;
        let g = 1.0 - age_normalized * 0.7;
        let b = 1.0 - age_normalized * 0.5;

        let color = Color::new(r, g, b, alpha);
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

// Enhanced visualization: Draw hyphae with flow intensity and stress coloring
// Performance: Optimized with cached values and efficient lookups
pub fn draw_hyphae_enhanced(
    hyphae: &[crate::hypha::Hypha],
    _connections: &[Connection], // Not used directly, flow comes from cache
    show_flow: bool,
    show_stress: bool,
    hypha_flow_cache: &[f32], // Pre-computed flow values per hypha index
) {
    // Performance: Cache screen dimensions (only call once)
    let screen_width = screen_width();
    let screen_height = screen_height();
    let margin = 50.0;
    let min_x = -margin;
    let min_y = -margin;
    let max_x = screen_width + margin;
    let max_y = screen_height + margin;

    // Performance: Cache FPS and time (only call once)
    let fps = get_fps();
    let t = get_time() as f32;
    let lod_step = if fps < 30 {
        3
    } else if fps < 45 {
        2
    } else {
        1
    };

    // Draw hyphae as points with enhanced coloring
    // Performance: Use iterator with early exits
    for (idx, h) in hyphae.iter().enumerate() {
        if !h.alive {
            continue;
        }

        // Performance: LOD - skip some hyphae
        if idx % lod_step != 0 {
            continue;
        }

        let px = h.x * CELL_SIZE;
        let py = h.y * CELL_SIZE;

        // Performance: Spatial culling - only draw visible hyphae
        if px < min_x || px > max_x || py < min_y || py > max_y {
            continue;
        }

        let mut color = Color::new(1.0, 1.0, 1.0, 0.8);
        let mut radius = 2.0;

        // Environmental stress: low energy = red/orange, high energy = white/blue
        if show_stress {
            let stress = 1.0 - h.energy;
            if stress > 0.3 {
                color.r = 1.0;
                color.g = 0.5 + stress * 0.5;
                color.b = 0.2;
            } else {
                color.r = 0.8 + h.energy * 0.2;
                color.g = 0.8 + h.energy * 0.2;
                color.b = 0.9 + h.energy * 0.1;
            }
        }

        // Nutrient flow intensity: use pre-computed cache
        if show_flow && idx < hypha_flow_cache.len() {
            let flow = hypha_flow_cache[idx];
            if flow > 0.01 {
                let flow_normalized = (flow * 10.0).min(1.0);
                color.g = (color.g + flow_normalized * 0.5).min(1.0);
                color.b = (color.b - flow_normalized * 0.3).max(0.0);
                radius += flow_normalized * 1.5;

                // Pulsing animation for active flow
                let pulse = (t * 3.0 + flow_normalized * 10.0).sin() * 0.2 + 0.8;
                color.a *= pulse;
            }
        }

        // Age-based size variation
        radius += h.age * 0.1;
        radius = radius.min(4.0);

        draw_circle(px, py, radius, color);
    }
}

pub fn draw_connections(connections: &[Connection], hyphae: &[Hypha], connections_visible: bool) {
    if !connections_visible || connections.is_empty() {
        return;
    }

    // Performance: Only do expensive operations if we have many connections
    let fps = get_fps();
    let step = if fps < 30 && connections.len() > 200 {
        2 // Skip every other connection when FPS < 30 and many connections
    } else {
        1
    };

    // Performance: Only do spatial culling if we have many connections
    let use_culling = connections.len() > 100;
    let (min_x, min_y, max_x, max_y) = if use_culling {
        let screen_width = screen_width();
        let screen_height = screen_height();
        let margin = 50.0;
        (
            -margin,
            -margin,
            screen_width + margin,
            screen_height + margin,
        )
    } else {
        (0.0, 0.0, 0.0, 0.0) // Dummy values
    };

    // Performance: Cache time (only call once)
    let t = get_time() as f32;
    let pulse = (t * 2.0).sin() * 0.25 + 0.5; // 0.25..0.75

    for (i, conn) in connections.iter().enumerate() {
        // Performance: LOD
        if i % step != 0 {
            continue;
        }

        if let (Some(h1), Some(h2)) = (hyphae.get(conn.hypha1), hyphae.get(conn.hypha2)) {
            if h1.alive && h2.alive {
                let x1 = h1.x * CELL_SIZE;
                let y1 = h1.y * CELL_SIZE;
                let x2 = h2.x * CELL_SIZE;
                let y2 = h2.y * CELL_SIZE;

                // Performance: Spatial culling (only if enabled)
                if use_culling {
                    let mid_x = (x1 + x2) * 0.5;
                    let mid_y = (y1 + y2) * 0.5;
                    if mid_x < min_x || mid_x > max_x || mid_y < min_y || mid_y > max_y {
                        continue;
                    }
                }

                let avg_age = (h1.age + h2.age) * 0.5;
                let age_fade = (1.0 / (1.0 + avg_age * 0.2)).clamp(0.2, 1.0);

                // Network Intelligence: Visualize connection strength
                // Strong connections are brighter and thicker
                let strength_factor = conn.strength;
                let thickness = 1.0 + strength_factor * 2.0;

                // Network Intelligence: Visualize signals (pulsing red/orange)
                let signal_intensity = conn.signal.min(1.0);
                let base_alpha = (0.4 + pulse * 0.4) * age_fade;

                if signal_intensity > 0.1 {
                    // Signal propagation: red/orange pulsing
                    let signal_pulse = (t * 4.0 + signal_intensity * 10.0).sin() * 0.5 + 0.5;
                    let signal_alpha = base_alpha * signal_intensity * signal_pulse;
                    draw_line(
                        x1,
                        y1,
                        x2,
                        y2,
                        thickness + signal_intensity * 1.0,
                        Color::new(1.0, 0.3 + signal_intensity * 0.4, 0.0, signal_alpha),
                    );
                } else {
                    // Normal connection: green, colored by strength
                    let base_alpha = base_alpha * (0.5 + strength_factor * 0.5);
                    let green = 0.3 + strength_factor * 0.7;
                    draw_line(
                        x1,
                        y1,
                        x2,
                        y2,
                        thickness,
                        Color::new(0.0, green, 0.5, base_alpha),
                    );
                }
            }
        }
    }
}

pub fn draw_minimap(nutrients: &NutrientGrid, hyphae: &[Hypha], minimap_visible: bool) {
    if !minimap_visible {
        return;
    }
    // Minimap size
    let map_scale = 0.25f32;
    let w = GRID_SIZE as f32 * map_scale;
    let h = GRID_SIZE as f32 * map_scale;
    let margin = 8.0f32;
    let x0 = screen_width() - w - margin;
    let y0 = margin;

    // Background
    draw_rectangle(
        x0 - 2.0,
        y0 - 2.0,
        w + 4.0,
        h + 4.0,
        Color::new(0.0, 0.0, 0.0, 0.4),
    );

    // Nutrients heatmap (downsampled)
    let step = 2usize;
    for x in (0..GRID_SIZE).step_by(step) {
        for y in (0..GRID_SIZE).step_by(step) {
            let sugar = nutrients.sugar[x][y];
            let nitrogen = nutrients.nitrogen[x][y];
            let c = nutrient_color(sugar, nitrogen);
            let px = x0 + x as f32 * map_scale;
            let py = y0 + y as f32 * map_scale;
            draw_rectangle(px, py, map_scale * step as f32, map_scale * step as f32, c);
        }
    }

    // Hyphae points
    for hph in hyphae.iter().filter(|h| h.alive) {
        let px = x0 + hph.x * map_scale;
        let py = y0 + hph.y * map_scale;
        draw_circle(px, py, 1.2, Color::new(1.0, 1.0, 1.0, 0.9));
    }
}

pub fn draw_fruit_bodies(fruit_bodies: &[FruitBody], hyphae: &[crate::hypha::Hypha]) {
    for f in fruit_bodies {
        let stem_h = 10.0;
        let stem_w = 3.0;
        let px = f.x * CELL_SIZE;
        let py = f.y * CELL_SIZE;

        // Draw energy transfer lines from nearby hyphae
        // Transfer radius in grid units (matches simulation)
        let transfer_radius_grid = 15.0;
        let transfer_radius_sq = transfer_radius_grid * transfer_radius_grid;
        for h in hyphae.iter().filter(|h| h.alive && h.energy > 0.1) {
            let hx = h.x * CELL_SIZE;
            let hy = h.y * CELL_SIZE;
            // Check distance in grid units
            let dx_grid = f.x - h.x;
            let dy_grid = f.y - h.y;
            let dist_sq_grid = dx_grid * dx_grid + dy_grid * dy_grid;

            if dist_sq_grid < transfer_radius_sq && dist_sq_grid > 0.1 {
                let dist_grid = dist_sq_grid.sqrt();
                let intensity = (1.0 - dist_grid / transfer_radius_grid).max(0.0) * h.energy;
                // Make lines more visible - higher alpha and thicker
                let alpha = (intensity * 0.6 + 0.2).min(0.8);
                let thickness = 2.0 + intensity * 1.5;
                draw_line(hx, hy, px, py, thickness, Color::new(1.0, 0.8, 0.2, alpha));
            }
        }

        // Energy-based size and color
        let energy_factor = f.energy.clamp(0.0, 1.0);
        let cap_size = 6.0 + energy_factor * 4.0;
        let cap_red = 0.8 - energy_factor * 0.3;
        let cap_green = 0.2 + energy_factor * 0.4;

        // stem (brighter with more energy)
        draw_rectangle(
            px - stem_w / 2.0,
            py - stem_h,
            stem_w,
            stem_h,
            Color::new(0.9, 0.9, 0.8, 0.7 + energy_factor * 0.2),
        );
        // cap (grows and changes color with energy)
        draw_circle(
            px,
            py - stem_h,
            cap_size,
            Color::new(cap_red, cap_green, 0.2, 0.9),
        );

        // Energy glow effect
        if energy_factor > 0.3 {
            let glow_alpha = (energy_factor - 0.3) * 0.4;
            draw_circle(
                px,
                py - stem_h,
                cap_size + 2.0,
                Color::new(1.0, 1.0, 0.5, glow_alpha),
            );
        }
    }
}

pub fn draw_stats_and_help(
    hyphae_count: usize,
    spores_count: usize,
    connections_count: usize,
    fruit_count: usize,
    avg_energy: f32,
    paused: bool,
    speed_multiplier: f32,
) {
    let fps = get_fps();
    let stats_part1_text = format!(
        "Hyphae: {} | Spores: {} | Connections: {} | Fruits: {} | Avg Energy: {:.2}",
        hyphae_count, spores_count, connections_count, fruit_count, avg_energy,
    );
    draw_text(&stats_part1_text, 10.0, 20.0, 20.0, WHITE);
    let stats_part2_text = format!("Speed: {:.1}x | FPS: {:.0}", speed_multiplier, fps);
    draw_text(&stats_part2_text, 10.0, 40.0, 20.0, WHITE);
    if paused {
        draw_text("PAUSED - Press SPACE to resume", 10.0, 60.0, 20.0, YELLOW);
    }
    let controls_part1_text =
        "Controls: SPACE=Pause | R=Reset | C=Clear | X=Connections | M=Minimap | H=Hyphae | I=Memory";
    draw_text(
        controls_part1_text,
        10.0,
        screen_height() - 100.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    let controls_part2_text =
        "S=Spawn | N=Sugar patch | T=Nitrogen patch | LMB=Sugar | RMB=Nitrogen";
    draw_text(
        controls_part2_text,
        10.0,
        screen_height() - 80.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    let controls_part3_text = "Speed Controls: <- = Slower | -> = Faster | 0 = Reset to 1x";
    draw_text(
        controls_part3_text,
        10.0,
        screen_height() - 60.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    let visualization_text = "Visualization: V=Enhanced | F=Flow | 1=Stress";
    draw_text(
        visualization_text,
        10.0,
        screen_height() - 40.0,
        16.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
    let network_intel_text = "Network Intelligence: Signals (red pulses) | Strong connections (bright/thick) | Memory (purple overlay)";
    draw_text(
        network_intel_text,
        10.0,
        screen_height() - 20.0,
        14.0,
        Color::new(0.8, 0.8, 1.0, 0.6),
    );
}
