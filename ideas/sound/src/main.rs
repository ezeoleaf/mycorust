//! Map growth into sound: a short WAV whose pitch/amp follow the network.

use mycorust_core::{analyze, Simulation, SimulationConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::fs::File;
use std::io::{BufWriter, Write};

fn experiment_config() -> SimulationConfig {
    let mut c = SimulationConfig::default();
    c.grid_size = 64;
    c.max_hyphae = 350;
    c.max_hyphae_branching_threshold = 280;
    c.obstacle_count = 20;
    c.weather_enabled = true;
    c.seasonal_cycles_enabled = false;
    c.initial_hyphae_count = 5;
    c.record_segments = false;
    c
}

fn write_wav(path: &str, samples: &[f32], sample_rate: u32) {
    let mut file = BufWriter::new(File::create(path).expect("create wav"));
    let data_bytes = (samples.len() * 2) as u32;
    let mut header = Vec::new();
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    header.extend_from_slice(b"WAVE");
    header.extend_from_slice(b"fmt ");
    header.extend_from_slice(&16u32.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    header.extend_from_slice(&2u16.to_le_bytes());
    header.extend_from_slice(&16u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_bytes.to_le_bytes());
    file.write_all(&header).unwrap();
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        file.write_all(&v.to_le_bytes()).unwrap();
    }
}

fn main() {
    let mut rng = StdRng::seed_from_u64(13);
    let mut sim = Simulation::with_config(&mut rng, experiment_config());
    let sample_rate = 22050u32;
    let mut samples = Vec::new();
    let mut phase = 0.0f32;

    println!("MycoRust sound");
    for beat in 0..80 {
        sim.step_n(&mut rng, 8);
        let m = analyze(&sim);
        let tempo = 180.0 + m.hyphae_alive as f32 * 0.8;
        let freq = tempo.clamp(80.0, 880.0);
        let volume = (m.avg_energy * 0.35 + m.mean_degree * 0.08).clamp(0.02, 0.4);
        let harmony = 1.0 + (m.cycle_count.max(0) as f32 * 0.015).min(1.5);
        let n = sample_rate / 12;
        for _ in 0..n {
            phase += freq * harmony * 2.0 * std::f32::consts::PI / sample_rate as f32;
            samples.push(phase.sin() * volume);
        }
        if beat % 20 == 0 {
            println!(
                "  t={:<3} hyphae={:<4} freq={:.0}Hz vol={:.2}",
                beat, m.hyphae_alive, freq, volume
            );
        }
    }

    let path = "mycelium-score.wav";
    write_wav(path, &samples, sample_rate);
    println!("Wrote {path}");
}
