use macroquad::prelude::*;

use crate::config::SimulationConfig;
use crate::controls::get_controls_text;
use crate::hypha::Hypha;
use crate::nutrients::{nutrient_color, NutrientGrid};
use crate::types::{Connection, FruitBody, Segment};

pub fn draw_nutrients(nutrients: &NutrientGrid, config: &SimulationConfig) {
    let grid_size = config.grid_size;
    let cell_size = config.cell_size;
    for x in 0..grid_size {
        for y in 0..grid_size {
            let sugar = nutrients.sugar[x][y];
            let nitrogen = nutrients.nitrogen[x][y];
            let color = nutrient_color(sugar, nitrogen);
            draw_rectangle(
                x as f32 * cell_size,
                y as f32 * cell_size,
                cell_size,
                cell_size,
                color,
            );
        }
    }
}

// Network Intelligence: Draw memory overlay (subtle purple/blue tint)
pub fn draw_memory_overlay(memory: &[Vec<f32>], memory_visible: bool, config: &SimulationConfig) {
    if !memory_visible {
        return;
    }
    let grid_size = config.grid_size;
    let cell_size = config.cell_size;
    for x in 0..grid_size {
        for y in 0..grid_size {
            let mem_val = memory[x][y];
            // Lower threshold to show memory values that accumulate over time
            // Memory values are small (0.003 max per update) but accumulate, so threshold of 0.001 is appropriate
            if mem_val > 0.001 {
                // Purple/blue overlay for memory
                // Scale alpha more aggressively to make memory visible even at low values
                // Memory values range from ~0.001 to 1.0, so we scale to make them visible
                let alpha = (mem_val * 0.5).min(0.6); // More visible overlay, capped at 0.6 for readability
                let purple = Color::new(0.6, 0.2, 0.9, alpha); // Brighter purple for better visibility
                draw_rectangle(
                    x as f32 * cell_size,
                    y as f32 * cell_size,
                    cell_size,
                    cell_size,
                    purple,
                );
            }
        }
    }
}

pub fn draw_obstacles(obstacles: &[Vec<bool>], config: &SimulationConfig) {
    let grid_size = config.grid_size;
    let cell_size = config.cell_size;
    #[allow(clippy::needless_range_loop)]
    for x in 0..grid_size {
        for y in 0..grid_size {
            if obstacles[x][y] {
                draw_rectangle(
                    x as f32 * cell_size,
                    y as f32 * cell_size,
                    cell_size,
                    cell_size,
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

    // Performance: Use adaptive quality reduction instead of skipping to prevent blinking
    // This maintains visibility while improving performance
    let fps = get_fps();

    // Calculate quality factor based on FPS (1.0 = full quality, 0.7 = reduced quality)
    let quality_factor = if fps < 20 {
        0.7 // Reduce quality when FPS is very low
    } else if fps < 35 {
        0.85 // Slightly reduce quality when FPS is moderate
    } else {
        1.0 // Full quality when FPS is good
    };

    // Adjust alpha threshold based on quality (skip more transparent segments when quality is low)
    let alpha_threshold = if quality_factor < 0.8 {
        0.03 // Skip more transparent segments
    } else {
        0.02 // Normal threshold
    };

    // Performance: Only do spatial culling if we have many segments (optimization)
    let use_culling = segments.len() > 1500; // Increased threshold to reduce culling overhead
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

    for segment in segments.iter() {
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
        if alpha < alpha_threshold {
            continue;
        }

        // Age-based coloring: young = white, old = dark gray/blue
        let age_normalized = (segment.age / max_segment_age).min(1.0);
        let r = 1.0 - age_normalized * 0.7;
        let g = 1.0 - age_normalized * 0.7;
        let b = 1.0 - age_normalized * 0.5;

        // Reduce line thickness and alpha based on quality factor instead of skipping
        // This maintains visibility while improving performance
        let line_thickness = 1.5 * quality_factor;
        let adjusted_alpha = alpha * (0.7 + quality_factor * 0.3); // Reduce alpha slightly when quality is low

        let color = Color::new(r, g, b, adjusted_alpha);
        draw_line(
            segment.from.x,
            segment.from.y,
            segment.to.x,
            segment.to.y,
            line_thickness,
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
    config: &SimulationConfig,
) {
    let cell_size = config.cell_size;
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

    // Use adaptive quality reduction instead of skipping to prevent blinking
    let quality_factor = if fps < 20 {
        0.7 // Reduce quality when FPS is very low
    } else if fps < 35 {
        0.85 // Slightly reduce quality when FPS is moderate
    } else {
        1.0 // Full quality when FPS is good
    };

    // Draw hyphae as points with enhanced coloring
    // Performance: Use iterator with early exits
    for (idx, h) in hyphae.iter().enumerate() {
        if !h.alive {
            continue;
        }

        let px = h.x * cell_size;
        let py = h.y * cell_size;

        // Performance: Spatial culling - only draw visible hyphae
        if px < min_x || px > max_x || py < min_y || py > max_y {
            continue;
        }

        // Reduce radius and alpha based on quality factor instead of skipping
        let base_radius = 2.0;
        let base_alpha = 0.8;
        let mut color = Color::new(1.0, 1.0, 1.0, base_alpha * quality_factor);
        let mut radius = base_radius * quality_factor;

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

pub fn draw_connections(
    connections: &[Connection],
    hyphae: &[Hypha],
    connections_visible: bool,
    config: &SimulationConfig,
) {
    let cell_size = config.cell_size;
    if !connections_visible || connections.is_empty() {
        return;
    }

    // Performance: Use adaptive quality reduction instead of skipping
    let fps = get_fps();

    // Calculate quality factor based on FPS and connection count
    let quality_factor = if fps < 20 && connections.len() > 300 {
        0.7 // Reduce quality when FPS is very low and many connections
    } else if fps < 35 && connections.len() > 500 {
        0.85 // Slightly reduce quality when FPS is moderate
    } else {
        1.0 // Full quality otherwise
    };

    // Performance: Only do spatial culling if we have many connections
    let use_culling = connections.len() > 400; // Increased threshold to reduce overhead
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

    for conn in connections.iter() {
        if let (Some(h1), Some(h2)) = (hyphae.get(conn.hypha1), hyphae.get(conn.hypha2)) {
            if h1.alive && h2.alive {
                let x1 = h1.x * cell_size;
                let y1 = h1.y * cell_size;
                let x2 = h2.x * cell_size;
                let y2 = h2.y * cell_size;

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
                let base_thickness = 1.0 + strength_factor * 2.0;

                // Apply quality factor to thickness and alpha instead of skipping
                // This maintains visibility while improving performance
                let thickness = base_thickness * quality_factor;

                // Network Intelligence: Visualize signals (pulsing red/orange)
                let signal_intensity = conn.signal.min(1.0);
                let base_alpha_raw = (0.4 + pulse * 0.4) * age_fade;
                let base_alpha = base_alpha_raw * quality_factor; // Apply quality factor to alpha

                if signal_intensity > 0.1 {
                    // Signal propagation: red/orange pulsing
                    let signal_pulse = (t * 4.0 + signal_intensity * 10.0).sin() * 0.5 + 0.5;
                    let signal_alpha = base_alpha * signal_intensity * signal_pulse;
                    let signal_thickness = (thickness + signal_intensity * 1.0) * quality_factor;
                    draw_line(
                        x1,
                        y1,
                        x2,
                        y2,
                        signal_thickness,
                        Color::new(1.0, 0.3 + signal_intensity * 0.4, 0.0, signal_alpha),
                    );
                } else {
                    // Normal connection: green, colored by strength
                    let conn_alpha = base_alpha * (0.5 + strength_factor * 0.5);
                    let green = 0.3 + strength_factor * 0.7;
                    draw_line(
                        x1,
                        y1,
                        x2,
                        y2,
                        thickness,
                        Color::new(0.0, green, 0.5, conn_alpha),
                    );
                }
            }
        }
    }
}

pub fn draw_minimap(
    nutrients: &NutrientGrid,
    hyphae: &[Hypha],
    minimap_visible: bool,
    config: &SimulationConfig,
) {
    if !minimap_visible {
        return;
    }
    let grid_size = config.grid_size;
    // Minimap size
    let map_scale = 0.25f32;
    let w = grid_size as f32 * map_scale;
    let h = grid_size as f32 * map_scale;
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
    for x in (0..grid_size).step_by(step) {
        for y in (0..grid_size).step_by(step) {
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

pub fn draw_fruit_bodies(
    fruit_bodies: &[FruitBody],
    hyphae: &[crate::hypha::Hypha],
    config: &SimulationConfig,
) {
    let cell_size = config.cell_size;
    for f in fruit_bodies {
        let stem_h = 10.0;
        let stem_w = 3.0;
        let px = f.x * cell_size;
        let py = f.y * cell_size;

        // Draw energy transfer lines from nearby hyphae
        // Transfer radius in grid units (matches simulation)
        let transfer_radius_grid = 20.0; // Updated to match simulation
        let transfer_radius_sq = transfer_radius_grid * transfer_radius_grid;
        for h in hyphae.iter().filter(|h| h.alive && h.energy > 0.05) {
            let hx = h.x * cell_size;
            let hy = h.y * cell_size;
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
    weather: Option<&crate::weather::Weather>,
) {
    let fps = get_fps();
    let stats_part1_text = format!(
        "Hyphae: {} | Spores: {} | Connections: {} | Fruits: {} | Avg Energy: {:.2}",
        hyphae_count, spores_count, connections_count, fruit_count, avg_energy,
    );
    draw_text(&stats_part1_text, 10.0, 20.0, 20.0, WHITE);
    let stats_part2_text = format!("Speed: {:.1}x | FPS: {:.0}", speed_multiplier, fps);
    draw_text(&stats_part2_text, 10.0, 40.0, 20.0, WHITE);

    // Weather information
    if let Some(w) = weather {
        let temp_c = w.temperature_celsius_approx();
        let temp_color = if temp_c < 10.0 {
            Color::new(0.5, 0.7, 1.0, 1.0) // Cold: blue
        } else if temp_c > 30.0 {
            Color::new(1.0, 0.5, 0.3, 1.0) // Hot: red-orange
        } else {
            Color::new(0.5, 1.0, 0.5, 1.0) // Optimal: green
        };

        let rain_str = if w.rain > 0.5 {
            "Heavy Rain"
        } else if w.rain > 0.2 {
            "Rain"
        } else if w.rain > 0.0 {
            "Light Rain"
        } else {
            "Clear"
        };

        draw_text(
            &format!(
                "Weather: {:.1}°C | Humidity: {:.0}% | {}",
                temp_c,
                w.humidity * 100.0,
                rain_str
            ),
            10.0,
            60.0,
            18.0,
            temp_color,
        );

        // Growth multiplier indicator
        let growth_mult = w.growth_multiplier();
        let growth_color = if growth_mult > 1.0 {
            GREEN
        } else if growth_mult > 0.7 {
            YELLOW
        } else {
            RED
        };
        draw_text(
            &format!("Growth: {:.1}x", growth_mult),
            10.0,
            80.0,
            16.0,
            growth_color,
        );
    }

    if paused {
        draw_text("PAUSED - Press SPACE to resume", 10.0, 100.0, 20.0, YELLOW);
    }

    // Show hint to press ? for help (only when popup is not visible)
    // This will be controlled by the help_popup_visible flag passed from main
}

/// Draw help popup window with controls
pub fn draw_help_popup(camera_enabled: bool) {
    let screen_width = screen_width();
    let screen_height = screen_height();

    // Popup dimensions
    let popup_width = (screen_width * 0.7).min(700.0);
    let popup_height = (screen_height * 0.8).min(600.0);
    let popup_x = (screen_width - popup_width) / 2.0;
    let popup_y = (screen_height - popup_height) / 2.0;

    // Dark overlay background (semi-transparent)
    draw_rectangle(
        0.0,
        0.0,
        screen_width,
        screen_height,
        Color::new(0.0, 0.0, 0.0, 0.7),
    );

    // Popup background
    draw_rectangle(
        popup_x,
        popup_y,
        popup_width,
        popup_height,
        Color::new(0.1, 0.1, 0.15, 0.95),
    );

    // Popup border
    draw_rectangle_lines(
        popup_x,
        popup_y,
        popup_width,
        popup_height,
        3.0,
        Color::new(0.5, 0.5, 0.7, 1.0),
    );

    // Title
    let title = "Controls & Help";
    let title_font_size = 28.0;
    let title_width = measure_text(title, None, title_font_size as u16, 1.0).width;
    draw_text(
        title,
        popup_x + (popup_width - title_width) / 2.0,
        popup_y + 30.0,
        title_font_size,
        Color::new(1.0, 1.0, 1.0, 1.0),
    );

    // Close hint
    let close_hint = "Press F1 or Escape to close";
    let close_hint_font_size = 16.0;
    let close_hint_width = measure_text(close_hint, None, close_hint_font_size as u16, 1.0).width;
    draw_text(
        close_hint,
        popup_x + (popup_width - close_hint_width) / 2.0,
        popup_y + popup_height - 25.0,
        close_hint_font_size,
        Color::new(0.7, 0.7, 0.7, 1.0),
    );

    // Controls text
    let mut y_offset = popup_y + 70.0;
    let line_spacing = 22.0;
    let margin = 20.0;
    let text_x = popup_x + margin;

    for text in get_controls_text(camera_enabled) {
        // Wrap long lines if needed
        let max_width = popup_width - margin * 2.0;
        let text_width = measure_text(text.text, None, text.font_size as u16, 1.0).width;

        if text_width > max_width {
            // Simple word wrapping (split by " | " separator)
            let parts: Vec<&str> = text.text.split(" | ").collect();
            let mut current_line = String::new();

            for (i, part) in parts.iter().enumerate() {
                let test_line = if current_line.is_empty() {
                    part.to_string()
                } else {
                    format!("{} | {}", current_line, part)
                };
                let test_width = measure_text(&test_line, None, text.font_size as u16, 1.0).width;

                if test_width > max_width && !current_line.is_empty() {
                    // Draw current line and start new one
                    draw_text(&current_line, text_x, y_offset, text.font_size, text.color);
                    y_offset += line_spacing;
                    current_line = part.to_string();
                } else {
                    current_line = test_line;
                }

                // If this is the last part, draw it
                if i == parts.len() - 1 && !current_line.is_empty() {
                    draw_text(&current_line, text_x, y_offset, text.font_size, text.color);
                    y_offset += line_spacing;
                }
            }
        } else {
            draw_text(text.text, text_x, y_offset, text.font_size, text.color);
            y_offset += line_spacing;
        }

        // Add extra spacing for section headers
        if text.font_size == 14.0 {
            y_offset += 5.0;
        }
    }
}
