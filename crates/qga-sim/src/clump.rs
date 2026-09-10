//! Bound overdensities in the cosmos disk. Software fact of this snapshot,
//! not Magic-Island theorems. Fed into the tour camera as dwell targets.

use crate::nbody::Particle;
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Clump {
    pub pos: Vec3,
    pub mass: f32,
    pub count: u32,
    pub radius: f32,
}

/// Polar (r, θ) mass bins in the midplane. Skip the star (index 0).
/// Returns up to `max_n` clumps, heaviest first, excluding the central seed.
pub fn detect_clumps(particles: &[Particle], max_n: usize) -> Vec<Clump> {
    const NR: usize = 16;
    const NTH: usize = 24;
    if particles.len() < 32 || max_n == 0 {
        return Vec::new();
    }
    let dust = &particles[1..];
    let mut r_max = 1.0f32;
    for p in dust {
        let r = (p.pos.x * p.pos.x + p.pos.y * p.pos.y).sqrt();
        if r > r_max {
            r_max = r;
        }
    }
    r_max = r_max.max(1.0);
    let mut mass = vec![0.0f32; NR * NTH];
    let mut count = vec![0u32; NR * NTH];
    let mut com = vec![Vec3::ZERO; NR * NTH];
    for p in dust {
        let r = (p.pos.x * p.pos.x + p.pos.y * p.pos.y).sqrt();
        let th = p.pos.y.atan2(p.pos.x);
        let ir = ((r / r_max) * NR as f32).floor() as usize;
        let it = (((th + std::f32::consts::PI) / std::f32::consts::TAU) * NTH as f32).floor()
            as usize;
        let ir = ir.min(NR - 1);
        let it = it.min(NTH - 1);
        let i = ir * NTH + it;
        mass[i] += p.mass;
        count[i] += 1;
        com[i] += p.pos * p.mass;
    }
    let mut occupied: Vec<f32> = mass.iter().copied().filter(|m| *m > 0.0).collect();
    if occupied.is_empty() {
        return Vec::new();
    }
    occupied.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = occupied[occupied.len() / 2];
    let thresh = (median * 3.0).max(median + 1e-6);
    let mut hits: Vec<(usize, f32)> = mass
        .iter()
        .enumerate()
        .filter(|(i, m)| **m > thresh && count[*i] >= 8)
        .map(|(i, m)| (i, *m))
        .collect();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::new();
    for (i, m) in hits.into_iter().take(max_n) {
        if m <= 0.0 {
            continue;
        }
        let pos = com[i] / m;
        // Skip anything sitting on the star.
        if pos.length() < r_max * 0.06 {
            continue;
        }
        let n = count[i];
        let radius = (0.35 + 0.04 * (n as f32).sqrt()).min(2.4);
        out.push(Clump {
            pos,
            mass: m,
            count: n,
            radius,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbody::Particle;

    #[test]
    fn empty_and_star_only() {
        assert!(detect_clumps(&[], 4).is_empty());
        let star = [Particle::new(Vec3::ZERO, Vec3::ZERO, 9.0)];
        assert!(detect_clumps(&star, 4).is_empty());
    }

    #[test]
    fn two_blobs_are_found() {
        let mut p = vec![Particle::new(Vec3::ZERO, Vec3::ZERO, 9.0)];
        for i in 0..40 {
            let a = i as f32 * 0.02;
            p.push(Particle::new(Vec3::new(4.0 + a, 0.1, 0.0), Vec3::ZERO, 0.05));
            p.push(Particle::new(Vec3::new(-3.5, 2.0 + a, 0.0), Vec3::ZERO, 0.05));
        }
        // sprinkle a thin disk so the median is below the blobs
        for i in 0..80 {
            let th = i as f32 * 0.08;
            p.push(Particle::new(
                Vec3::new(6.0 * th.cos(), 6.0 * th.sin(), 0.0),
                Vec3::ZERO,
                0.01,
            ));
        }
        let c = detect_clumps(&p, 8);
        assert!(c.len() >= 2, "got {} clumps", c.len());
        assert!(c.iter().any(|k| k.pos.x > 2.0));
        assert!(c.iter().any(|k| k.pos.x < 0.0));
    }
}
