use ::rand as external_rand;
use external_rand::thread_rng;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::io;
use std::time::{Duration, Instant};

mod config;
#[cfg(feature = "gui")]
mod controls;
mod hypha;
mod nutrients;
mod simulation;
mod spore;
mod types;
#[cfg(feature = "gui")]
mod visualization;

use simulation::Simulation;

fn main() -> io::Result<()> {
    // Setup panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Use a closure to ensure cleanup happens
    let result = run_simulation(&mut terminal);
    
    // Restore terminal
    restore_terminal();
    
    result
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

fn run_simulation(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    // Check terminal size
    let size = terminal.size()?;
    if size.width < 40 || size.height < 20 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Terminal too small: {}x{}. Need at least 40x20", size.width, size.height)
        ));
    }

    let mut rng = thread_rng();
    let mut sim = Simulation::new(&mut rng);
    let mut last_update = Instant::now();
    let update_interval = Duration::from_millis(50); // ~20 FPS

    loop {
        // Handle input - use non-blocking poll with error handling
        match event::poll(Duration::from_millis(10)) {
            Ok(true) => {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char(' ') => sim.toggle_pause(),
                                KeyCode::Char('r') => sim.reset(&mut rng),
                                KeyCode::Char('c') => sim.clear_segments(),
                                KeyCode::Char('x') => sim.toggle_connections(),
                                KeyCode::Char('m') => sim.toggle_minimap(),
                                KeyCode::Char('h') => sim.toggle_hyphae_visibility(),
                                KeyCode::Char('0') => sim.reset_speed(),
                                KeyCode::Left => sim.decrease_speed(),
                                KeyCode::Right => sim.increase_speed(),
                                _ => {}
                            }
                        }
                    }
                    Ok(_) => {} // Ignore non-key events
                    Err(_) => {} // Ignore read errors
                }
            }
            Ok(false) => {} // No event available
            Err(_) => {} // Ignore poll errors
        }

        // Update simulation
        if !sim.paused && last_update.elapsed() >= update_interval {
            // Handle speed multiplier
            sim.speed_accumulator += sim.speed_multiplier;
            let steps = sim.speed_accumulator.floor() as usize;
            sim.speed_accumulator -= steps as f32;
            
            for _ in 0..steps {
                sim.step(&mut rng);
            }
            last_update = Instant::now();
        }

        // Draw
        terminal.draw(|frame| {
            let size = frame.size();
            
            // Main layout: stats at top, visualization in middle, controls at bottom
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Stats
                    Constraint::Min(0),    // Visualization
                    Constraint::Length(4), // Controls
                ])
                .split(size);

            // Stats
            let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, _total_energy) = sim.stats();
            let stats_text = format!(
                "Hyphae: {} | Spores: {} | Connections: {} | Fruits: {} | Avg Energy: {:.2} | Speed: {:.1}x",
                hyphae_count, spores_count, connections_count, fruit_count, avg_energy, sim.speed_multiplier
            );
            let paused_text = if sim.paused { " [PAUSED]" } else { "" };
            
            let stats = Paragraph::new(format!("{}{}", stats_text, paused_text))
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL).title("Stats"));
            frame.render_widget(stats, chunks[0]);

            // Visualization area
            let vis_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(chunks[1]);

            // Main grid visualization - adapt to terminal size
            let grid_widget = create_grid_widget(&sim, vis_chunks[0]);
            frame.render_widget(grid_widget, vis_chunks[0]);

            // Side panel with info
            let info_panel = create_info_panel(&sim);
            frame.render_widget(info_panel, vis_chunks[1]);

            // Controls help
            let controls_text = "Controls: Q/Esc=Quit | SPACE=Pause | R=Reset | C=Clear | X=Toggle Connections | M=Toggle Minimap | H=Toggle Hyphae | ←/→=Speed | 0=Reset Speed";
            let controls = Paragraph::new(controls_text)
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("Controls"));
            frame.render_widget(controls, chunks[2]);
        })?;
    }
    
    Ok(())
}

fn create_grid_widget(sim: &Simulation, area: Rect) -> impl Widget {
    let grid_size = sim.config.grid_size;
    // Adapt display size to terminal size, leave some margin
    let display_width = (area.width.saturating_sub(2)).min(80) as usize;
    let display_height = (area.height.saturating_sub(2)).min(40) as usize;
    
    // Create a text representation of the grid
    let mut grid_lines = Vec::new();
    
    // Sample the grid at lower resolution for display
    let scale_x = if display_width > 0 {
        grid_size as f32 / display_width as f32
    } else {
        1.0
    };
    let scale_y = if display_height > 0 {
        grid_size as f32 / display_height as f32
    } else {
        1.0
    };
    
    for y in 0..display_height {
        let mut line = String::new();
        for x in 0..display_width {
            let gx = ((x as f32 * scale_x) as usize).min(grid_size.saturating_sub(1));
            let gy = ((y as f32 * scale_y) as usize).min(grid_size.saturating_sub(1));
            
            // Double-check bounds
            if gx >= grid_size || gy >= grid_size {
                line.push(' ');
                continue;
            }
            
            // Check for obstacles
            if sim.state.obstacles[gx][gy] {
                line.push('█');
            }
            // Check for hyphae and segments - check if any hypha or segment falls within this grid cell
            else if sim.hyphae_visible {
                let mut cell_char = ' ';
                // Calculate the bounds of this grid cell
                let cell_min_x = (x as f32 * scale_x).floor();
                let cell_max_x = ((x + 1) as f32 * scale_x).floor();
                let cell_min_y = (y as f32 * scale_y).floor();
                let cell_max_y = ((y + 1) as f32 * scale_y).floor();
                
                // First check for active hyphae (show as *)
                for h in &sim.state.hyphae {
                    if h.alive {
                        // Check if hypha position is within this cell's bounds
                        if h.x >= cell_min_x && h.x < cell_max_x &&
                           h.y >= cell_min_y && h.y < cell_max_y {
                            cell_char = '*';
                            break;
                        }
                    }
                }
                
                // If no active hypha, check for segments (trails) - show as lighter character
                if cell_char == ' ' {
                    // Convert segment coordinates from pixel space to grid space
                    let cell_size = sim.config.cell_size;
                    let cell_center_x = (cell_min_x + cell_max_x) / 2.0;
                    let cell_center_y = (cell_min_y + cell_max_y) / 2.0;
                    let cell_radius = ((cell_max_x - cell_min_x).max(cell_max_y - cell_min_y)) / 2.0;
                    
                    // Track the youngest segment that passes through this cell
                    let mut youngest_age = f32::MAX;
                    let mut best_char = ' ';
                    
                    for seg in &sim.state.segments {
                        // Skip very old segments
                        if seg.age >= 6.0 {
                            continue;
                        }
                        
                        // Convert segment endpoints from pixel to grid coordinates
                        let seg_from_x = seg.from.x / cell_size;
                        let seg_from_y = seg.from.y / cell_size;
                        let seg_to_x = seg.to.x / cell_size;
                        let seg_to_y = seg.to.y / cell_size;
                        
                        // Check if segment endpoint is in this cell
                        let endpoint_in_cell = 
                            (seg_from_x >= cell_min_x && seg_from_x < cell_max_x &&
                             seg_from_y >= cell_min_y && seg_from_y < cell_max_y) ||
                            (seg_to_x >= cell_min_x && seg_to_x < cell_max_x &&
                             seg_to_y >= cell_min_y && seg_to_y < cell_max_y);
                        
                        // Calculate distance from cell center to segment line
                        // Using point-to-line-segment distance formula
                        let dx = seg_to_x - seg_from_x;
                        let dy = seg_to_y - seg_from_y;
                        let seg_len_sq = dx * dx + dy * dy;
                        
                        let dist_to_seg = if seg_len_sq < 0.001 {
                            // Segment is a point
                            ((cell_center_x - seg_from_x).powi(2) + 
                             (cell_center_y - seg_from_y).powi(2)).sqrt()
                        } else {
                            // Project cell center onto segment
                            let t = ((cell_center_x - seg_from_x) * dx + 
                                    (cell_center_y - seg_from_y) * dy) / seg_len_sq;
                            let t_clamped = t.max(0.0).min(1.0);
                            let proj_x = seg_from_x + t_clamped * dx;
                            let proj_y = seg_from_y + t_clamped * dy;
                            ((cell_center_x - proj_x).powi(2) + 
                             (cell_center_y - proj_y).powi(2)).sqrt()
                        };
                        
                        // Check if segment is close enough to the cell
                        // Use a threshold based on cell size and segment length
                        let threshold = cell_radius * 1.2;
                        if endpoint_in_cell || dist_to_seg < threshold {
                            // This segment affects this cell
                            if seg.age < youngest_age {
                                youngest_age = seg.age;
                                // Choose character based on age
                                if seg.age < 3.0 {
                                    best_char = '·';  // Recent segment
                                } else {
                                    best_char = '.';  // Older segment
                                }
                            }
                        }
                    }
                    
                    if best_char != ' ' {
                        cell_char = best_char;
                    }
                }
                
                // If still empty, show nutrients
                if cell_char == ' ' {
                    let sugar = sim.state.nutrients.sugar[gx][gy];
                    let nitrogen = sim.state.nutrients.nitrogen[gx][gy];
                    let total = sugar + nitrogen;
                    
                    if total > 0.8 {
                        cell_char = '▓';
                    } else if total > 0.5 {
                        cell_char = '▒';
                    } else if total > 0.2 {
                        cell_char = '░';
                    }
                }
                
                line.push(cell_char);
            } else {
                // Just show nutrients
                let sugar = sim.state.nutrients.sugar[gx][gy];
                let nitrogen = sim.state.nutrients.nitrogen[gx][gy];
                let total = sugar + nitrogen;
                
                if total > 0.8 {
                    line.push('▓');
                } else if total > 0.5 {
                    line.push('▒');
                } else if total > 0.2 {
                    line.push('░');
                } else {
                    line.push(' ');
                }
            }
        }
        grid_lines.push(Line::from(line));
    }
    
    Paragraph::new(Text::from(grid_lines))
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).title("Simulation Grid"))
}

fn create_info_panel(sim: &Simulation) -> impl Widget {
    let (hyphae_count, spores_count, connections_count, fruit_count, avg_energy, total_energy) = sim.stats();
    
    let info_text = vec![
        Line::from(format!("Grid Size: {}x{}", sim.config.grid_size, sim.config.grid_size)),
        Line::from(format!("Hyphae: {}", hyphae_count)),
        Line::from(format!("Spores: {}", spores_count)),
        Line::from(format!("Connections: {}", connections_count)),
        Line::from(format!("Fruits: {}", fruit_count)),
        Line::from(format!("Avg Energy: {:.2}", avg_energy)),
        Line::from(format!("Total Energy: {:.2}", total_energy)),
        Line::from(format!("Speed: {:.1}x", sim.speed_multiplier)),
        Line::from(format!("Paused: {}", if sim.paused { "Yes" } else { "No" })),
        Line::from(format!("Connections Visible: {}", if sim.connections_visible { "Yes" } else { "No" })),
        Line::from(format!("Minimap Visible: {}", if sim.minimap_visible { "Yes" } else { "No" })),
        Line::from(format!("Hyphae Visible: {}", if sim.hyphae_visible { "Yes" } else { "No" })),
    ];
    
    Paragraph::new(Text::from(info_text))
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Info"))
}

