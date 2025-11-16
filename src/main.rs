use ::rand as external_rand;
use clap::Parser;
use external_rand::thread_rng;

mod config;
mod hypha;
mod nutrients;
mod simulation;
mod spore;
mod types;
mod weather;

use config::*;
use simulation::Simulation;

#[cfg(feature = "ui")]
mod camera;
#[cfg(feature = "ui")]
mod controls;
#[cfg(feature = "ui")]
mod visualization;

mod api;

#[cfg(feature = "ui")]
use macroquad::prelude::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run in headless mode (HTTP API server)
    #[arg(long)]
    headless: bool,

    /// Port for headless API server
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Configuration file path (YAML or JSON). If not specified, searches for config.yaml, config.yml, or config.json in current directory.
    #[arg(short, long)]
    config: Option<String>,
}

#[cfg(not(feature = "ui"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Headless mode only
    let args = Args::parse();
    let config = load_config(args.config.as_deref())?;
    headless_main(args.port, config).await
}

#[cfg(feature = "ui")]
#[macroquad::main(window_conf)]
async fn main() {
    let args = Args::parse();

    // Load configuration
    let config = match load_config(args.config.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            std::process::exit(1);
        }
    };

    if args.headless {
        // Run headless mode even with UI feature enabled
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = headless_main(args.port, config).await {
                eprintln!("Error running headless mode: {}", e);
                std::process::exit(1);
            }
        });
    } else {
        // Run UI mode
        ui_main(config).await;
    }
}

/// Load configuration from file or use default
fn load_config(config_path: Option<&str>) -> Result<SimulationConfig, Box<dyn std::error::Error>> {
    if let Some(path) = config_path {
        // User specified a config file
        SimulationConfig::from_file(path)
            .map_err(|e| format!("Failed to load config from {}: {}", path, e).into())
    } else {
        // Try default paths
        Ok(SimulationConfig::from_default_paths())
    }
}

#[cfg(feature = "ui")]
async fn ui_main(config: SimulationConfig) {
    use controls::handle_controls;
    use visualization::{
        draw_connections, draw_fruit_bodies, draw_help_popup, draw_hyphae_enhanced,
        draw_memory_overlay, draw_minimap, draw_nutrients, draw_obstacles, draw_segments,
        draw_stats_and_help,
    };

    let mut rng = thread_rng();
    // Initialize simulation with loaded config
    let mut sim = Simulation::with_config(&mut rng, config);

    loop {
        // Update camera (pan/zoom)
        if sim.config.camera_enabled {
            sim.camera.update(&sim.config);
        }

        // Handle player controls
        handle_controls(&mut sim, &mut rng);

        // Set camera transform for world rendering
        if sim.config.camera_enabled {
            set_camera(&sim.camera.get_camera());
        }

        // Blue background every frame so it stays visible
        clear_background(Color::new(0.05, 0.10, 0.35, 1.0));

        // Draw nutrients
        draw_nutrients(&sim.state.nutrients, &sim.config);

        // Draw obstacles
        draw_obstacles(&sim.state.obstacles, &sim.config);

        // Redraw all past segments to keep trails visible (with fading)
        // Enhanced: Age-based coloring (young=white, old=dark)
        draw_segments(
            &sim.state.segments,
            sim.config.max_segment_age,
            sim.hyphae_visible,
        );

        // Draw anastomosis connections
        draw_connections(
            &sim.state.connections,
            &sim.state.hyphae,
            sim.connections_visible,
            &sim.config,
        );

        // Network Intelligence: Draw memory overlay
        draw_memory_overlay(&sim.state.nutrient_memory, sim.memory_visible, &sim.config);

        // Enhanced Visualization: Draw hyphae with flow/stress coloring
        // Note: This is optional and can impact performance - toggle with 'V'
        if sim.enhanced_visualization {
            draw_hyphae_enhanced(
                &sim.state.hyphae,
                &sim.state.connections,
                sim.show_flow,
                sim.show_stress,
                &sim.hypha_flow_cache,
                &sim.config,
            );
        }

        // Draw fruiting bodies with energy transfer visualization
        draw_fruit_bodies(&sim.state.fruit_bodies, &sim.state.hyphae, &sim.config);

        // Reset camera for UI elements (minimap, stats) - these should not be affected by pan/zoom
        if sim.config.camera_enabled {
            set_camera(&Camera2D::default());
        }

        // Minimap overlay
        draw_minimap(
            &sim.state.nutrients,
            &sim.state.hyphae,
            sim.minimap_visible,
            &sim.config,
        );

        // Update simulation only if not paused
        // Handle speed multiplier with accumulator for fractional speeds
        if !sim.paused {
            sim.speed_accumulator += sim.speed_multiplier;
            let steps = sim.speed_accumulator.floor() as usize;
            sim.speed_accumulator -= steps as f32;

            for _ in 0..steps {
                sim.step(&mut rng);
            }
        }

        // Calculate statistics via simulation API
        let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, _total_energy) =
            sim.stats();

        // Draw statistics overlay (always visible)
        draw_stats_and_help(
            hyphae_count,
            spores_count,
            connections_count,
            fruit_count,
            avg_energy,
            sim.paused,
            sim.speed_multiplier,
            Some(&sim.state.weather),
        );

        // Draw help popup if visible
        if sim.help_popup_visible {
            draw_help_popup(sim.config.camera_enabled);
        } else {
            // Show hint to press F1 for help when popup is not visible
            let hint_text = "Press F1 for controls";
            let hint_font_size = 16.0;
            let hint_width = measure_text(hint_text, None, hint_font_size as u16, 1.0).width;
            draw_text(
                hint_text,
                screen_width() - hint_width - 10.0,
                screen_height() - 25.0,
                hint_font_size,
                Color::new(0.7, 0.7, 0.7, 0.6),
            );
        }

        // Take screenshot if requested
        if sim.take_screenshot {
            sim.take_screenshot = false;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let filename = format!("mycorust_screenshot_{}.png", timestamp);

            // Capture screenshot using macroquad's screen texture
            match capture_screenshot(&filename) {
                Ok(_) => {
                    println!("Screenshot saved: {}", filename);
                }
                Err(e) => {
                    eprintln!("Failed to save screenshot {}: {}", filename, e);
                }
            }
        }

        next_frame().await;
    }
}

#[cfg(feature = "ui")]
fn window_conf() -> Conf {
    // Try to load config to set window size, fall back to defaults if not available
    let config = SimulationConfig::from_default_paths();
    let width = (config.grid_size as f32 * config.cell_size) as i32;
    let height = (config.grid_size as f32 * config.cell_size) as i32;

    Conf {
        window_title: "Mycelium Growth Simulation".to_owned(),
        window_width: width,
        window_height: height,
        ..Default::default()
    }
}

#[cfg(feature = "ui")]
/// Capture a screenshot of the current screen
fn capture_screenshot(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get the screen data from macroquad
    // This returns an Image struct with pixel data in RGBA format
    let screen_image = get_screen_data();

    let width = screen_image.width as u32;
    let height = screen_image.height as u32;
    let bytes = &screen_image.bytes;

    // Convert macroquad Image to image crate format
    // macroquad's Image has bytes in RGBA format, stored row by row
    let mut img = image::RgbaImage::new(width, height);

    // Copy pixels from macroquad Image to image crate format
    // Note: OpenGL typically has origin at bottom-left, but images have origin at top-left
    // So we need to flip vertically
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize * 4;
            if idx + 3 < bytes.len() {
                let r = bytes[idx];
                let g = bytes[idx + 1];
                let b = bytes[idx + 2];
                let a = bytes[idx + 3];

                // Flip vertically: OpenGL has origin at bottom-left, images at top-left
                let img_y = height - 1 - y;
                img.put_pixel(x, img_y, image::Rgba([r, g, b, a]));
            }
        }
    }

    // Save the image as PNG
    img.save(filename)?;

    Ok(())
}

/// Headless mode - runs HTTP API server
async fn headless_main(
    port: u16,
    config: SimulationConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use api::run_server;
    use api::ApiState;
    use simulation::set_headless_mode;

    // Set headless mode flag to avoid calling macroquad functions
    set_headless_mode(true);

    // Initialize simulation with loaded config
    let mut rng = thread_rng();
    let sim = Simulation::with_config(&mut rng, config);

    // Create API state
    let api_state = ApiState::new(sim);

    // Run the server
    run_server(api_state, port).await?;

    Ok(())
}
