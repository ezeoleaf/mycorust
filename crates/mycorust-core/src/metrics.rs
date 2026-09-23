//! Network-level measurements shared by idea crates.
//!
//! These treat the live mycelium as a graph: hyphae are nodes, anastomosis
//! connections (and parent links) are edges.

use crate::simulation::Simulation;

/// Snapshot of topology, biomass, and environmental load.
#[derive(Clone, Debug, Default)]
pub struct NetworkMetrics {
    pub hyphae_alive: usize,
    pub hyphae_total: usize,
    pub connections: usize,
    pub spores: usize,
    pub fruit_bodies: usize,
    pub total_energy: f32,
    pub avg_energy: f32,
    /// Connected components among living hyphae.
    pub component_count: usize,
    pub largest_component: usize,
    /// 1 - (largest component / living hyphae). 0 = one intact network.
    pub fragmentation: f32,
    /// Average anastomosis degree of living hyphae.
    pub mean_degree: f32,
    /// Extra edges beyond a tree: `connections - hyphae + components`.
    pub cycle_count: i32,
    pub nutrient_mass: f32,
    pub memory_mass: f32,
    pub mean_density: f32,
    pub mean_moisture: f32,
}

/// Build metrics from the current simulation state.
pub fn analyze(sim: &Simulation) -> NetworkMetrics {
    let (hyphae_alive, spores, connections, fruit_bodies, avg_energy, total_energy) = sim.stats();
    let (component_count, largest_component) = components(sim);
    let fragmentation = if hyphae_alive == 0 {
        0.0
    } else {
        1.0 - (largest_component as f32 / hyphae_alive as f32)
    };
    let mean_degree = if hyphae_alive == 0 {
        0.0
    } else {
        (connections as f32 * 2.0) / hyphae_alive as f32
    };
    let cycle_count = connections as i32 - hyphae_alive as i32 + component_count as i32;

    let mut memory_mass = 0.0;
    for row in &sim.state.nutrient_memory {
        for v in row {
            memory_mass += *v;
        }
    }

    let mut density_sum = 0.0;
    let mut density_n = 0usize;
    for row in &sim.state.density_map {
        for v in row {
            density_sum += *v;
            density_n += 1;
        }
    }

    let mut moisture_sum = 0.0;
    let mut moisture_n = 0usize;
    for row in &sim.state.soil_moisture {
        for v in row {
            moisture_sum += *v;
            moisture_n += 1;
        }
    }

    NetworkMetrics {
        hyphae_alive,
        hyphae_total: sim.state.hyphae.len(),
        connections,
        spores,
        fruit_bodies,
        total_energy,
        avg_energy,
        component_count,
        largest_component,
        fragmentation,
        mean_degree,
        cycle_count,
        nutrient_mass: sim.state.nutrients.total_mass(),
        memory_mass,
        mean_density: if density_n == 0 {
            0.0
        } else {
            density_sum / density_n as f32
        },
        mean_moisture: if moisture_n == 0 {
            0.0
        } else {
            moisture_sum / moisture_n as f32
        },
    }
}

fn components(sim: &Simulation) -> (usize, usize) {
    let n = sim.state.hyphae.len();
    if n == 0 {
        return (0, 0);
    }

    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }

    fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
        let mut ra = find(parent, a);
        let mut rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        parent[rb] = ra;
        if rank[ra] == rank[rb] {
            rank[ra] += 1;
        }
    }

    for (i, h) in sim.state.hyphae.iter().enumerate() {
        if !h.alive {
            continue;
        }
        if let Some(p) = h.parent {
            if p < n && sim.state.hyphae[p].alive {
                union(&mut parent, &mut rank, i, p);
            }
        }
    }

    for conn in &sim.state.connections {
        if conn.hypha1 >= n || conn.hypha2 >= n {
            continue;
        }
        if sim.state.hyphae[conn.hypha1].alive && sim.state.hyphae[conn.hypha2].alive {
            union(&mut parent, &mut rank, conn.hypha1, conn.hypha2);
        }
    }

    let mut sizes = vec![0usize; n];
    for (i, h) in sim.state.hyphae.iter().enumerate() {
        if h.alive {
            let root = find(&mut parent, i);
            sizes[root] += 1;
        }
    }

    let mut component_count = 0;
    let mut largest = 0;
    for size in sizes {
        if size > 0 {
            component_count += 1;
            largest = largest.max(size);
        }
    }
    (component_count, largest)
}

/// Euclidean shortest-path length through living hyphae using anastomosis edges
/// and parent links. Returns `None` if start/goal are disconnected.
pub fn shortest_path_length(sim: &Simulation, start: usize, goal: usize) -> Option<f32> {
    let n = sim.state.hyphae.len();
    if start >= n || goal >= n {
        return None;
    }
    if !sim.state.hyphae[start].alive || !sim.state.hyphae[goal].alive {
        return None;
    }

    let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
    let dist_between = |a: usize, b: usize| -> f32 {
        let ha = &sim.state.hyphae[a];
        let hb = &sim.state.hyphae[b];
        let dx = ha.x - hb.x;
        let dy = ha.y - hb.y;
        (dx * dx + dy * dy).sqrt()
    };

    for (i, h) in sim.state.hyphae.iter().enumerate() {
        if !h.alive {
            continue;
        }
        if let Some(p) = h.parent {
            if p < n && sim.state.hyphae[p].alive {
                let d = dist_between(i, p);
                adj[i].push((p, d));
                adj[p].push((i, d));
            }
        }
    }
    for conn in &sim.state.connections {
        if conn.hypha1 >= n || conn.hypha2 >= n {
            continue;
        }
        if sim.state.hyphae[conn.hypha1].alive && sim.state.hyphae[conn.hypha2].alive {
            let d = dist_between(conn.hypha1, conn.hypha2);
            adj[conn.hypha1].push((conn.hypha2, d));
            adj[conn.hypha2].push((conn.hypha1, d));
        }
    }

    let mut dist = vec![f32::INFINITY; n];
    let mut used = vec![false; n];
    dist[start] = 0.0;

    for _ in 0..n {
        let mut u = None;
        let mut best = f32::INFINITY;
        for i in 0..n {
            if !used[i] && dist[i] < best {
                best = dist[i];
                u = Some(i);
            }
        }
        let u = match u {
            Some(u) if best.is_finite() => u,
            _ => break,
        };
        if u == goal {
            return Some(dist[u]);
        }
        used[u] = true;
        for &(v, w) in &adj[u] {
            let nd = dist[u] + w;
            if nd < dist[v] {
                dist[v] = nd;
            }
        }
    }

    if dist[goal].is_finite() {
        Some(dist[goal])
    } else {
        None
    }
}

/// Index of the living hypha closest to a grid point.
pub fn nearest_hypha(sim: &Simulation, x: f32, y: f32) -> Option<usize> {
    let mut best = None;
    let mut best_d = f32::INFINITY;
    for (i, h) in sim.state.hyphae.iter().enumerate() {
        if !h.alive {
            continue;
        }
        let dx = h.x - x;
        let dy = h.y - y;
        let d = dx * dx + dy * dy;
        if d < best_d {
            best_d = d;
            best = Some(i);
        }
    }
    best
}

impl NetworkMetrics {
    /// A 0–1 score: intact, energetic networks score higher.
    pub fn resilience_score(&self) -> f32 {
        if self.hyphae_alive == 0 {
            return 0.0;
        }
        let energy = self.avg_energy.clamp(0.0, 1.0);
        let intact = (1.0 - self.fragmentation).clamp(0.0, 1.0);
        let loops = (self.cycle_count.max(0) as f32 / (self.hyphae_alive as f32 + 1.0)).min(1.0);
        (0.45 * intact + 0.35 * energy + 0.20 * loops).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimulationConfig;
    use crate::simulation::Simulation;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn analyze_on_fresh_sim() {
        let mut rng = StdRng::seed_from_u64(1);
        let sim = Simulation::with_config(&mut rng, SimulationConfig::default());
        let metrics = analyze(&sim);
        assert_eq!(metrics.hyphae_alive, sim.config.initial_hyphae_count);
        assert!(metrics.resilience_score() >= 0.0);
        assert!(metrics.resilience_score() <= 1.0);
    }
}
