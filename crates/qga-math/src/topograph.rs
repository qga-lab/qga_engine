//! Flux topographs — value landscapes on the gauged Hopf lattice (QGA Ch. 5–6).

use crate::hopf::{hopf_map, stereographic, HopfConvention};
use crate::lattice::hurwitz_units;
use crate::quat::Q;
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Functional {
    Norm,
    HopfY1,
    HopfHeight,
    Phase,
}

#[derive(Clone, Debug)]
pub struct FluxTopograph {
    pub points: Vec<Q>,
    pub values: Vec<f32>,
    pub edges: Vec<(usize, usize)>,
    pub functional: Functional,
}

pub fn functional_values(points: &[Q], name: Functional) -> Vec<f32> {
    match name {
        Functional::Norm => points
            .iter()
            .map(|q| q.x() * q.x() + q.y() * q.y() + q.z() * q.z())
            .collect(),
        Functional::HopfY1 => points
            .iter()
            .map(|q| hopf_map(*q, HopfConvention::Classical).x)
            .collect(),
        Functional::HopfHeight => points
            .iter()
            .map(|q| hopf_map(*q, HopfConvention::Classical).z)
            .collect(),
        Functional::Phase => points.iter().map(|q| q.z().atan2(q.y())).collect(),
    }
}

pub fn build_flux_topograph(
    points: Vec<Q>,
    edges: Vec<(usize, usize)>,
    functional: Functional,
) -> FluxTopograph {
    let values = functional_values(&points, functional);
    FluxTopograph {
        points,
        values,
        edges,
        functional,
    }
}

/// Sign-change separator edges (undirected).
pub fn detect_separators(topo: &FluxTopograph, threshold: f32) -> Vec<(usize, usize)> {
    let candidates: Vec<(usize, usize)> = if !topo.edges.is_empty() {
        topo.edges
            .iter()
            .map(|&(a, b)| (a.min(b), a.max(b)))
            .collect()
    } else {
        let n = topo.values.len();
        if n > 80 {
            let mut e: Vec<_> = (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect();
            if n > 2 {
                e.push((0, n - 1));
            }
            e
        } else {
            let mut e = Vec::new();
            for i in 0..n {
                for j in (i + 1)..n {
                    e.push((i, j));
                }
            }
            e
        }
    };
    candidates
        .into_iter()
        .filter(|&(i, j)| {
            let vi = topo.values[i] - threshold;
            let vj = topo.values[j] - threshold;
            vi == 0.0 || vj == 0.0 || vi * vj < 0.0
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IslandType {
    Elliptic,
    Hyperbolic,
    ZeroHyperbolic,
    Parabolic,
}

/// Cheap Magic-Island heuristic from value variance + separator density.
/// Pedagogical model (QGA Ch. 5–6), not a uniqueness theorem.
pub fn magic_island_score(topo: &FluxTopograph) -> (f32, IslandType) {
    let n = topo.values.len().max(1) as f32;
    let mean = topo.values.iter().sum::<f32>() / n;
    let var = topo.values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let seps = detect_separators(topo, 0.0);
    let sep_frac = seps.len() as f32 / n;
    let var_term = (-3.0 * var).exp();
    let sep_term = (-((sep_frac - 0.15).powi(2)) / (2.0 * 0.12 * 0.12)).exp();
    let (kind, bonus) = if var < 0.04 && sep_frac < 0.2 {
        (IslandType::Elliptic, 0.25)
    } else if sep_frac > 0.45 {
        (IslandType::Hyperbolic, 0.2)
    } else if var < 0.02 {
        (IslandType::ZeroHyperbolic, 0.35)
    } else {
        (IslandType::Parabolic, 0.05)
    };
    let period_term = (1.0 - var.min(1.0)).max(0.0);
    let score = 0.4 * period_term + 0.3 * var_term + 0.2 * sep_term + bonus;
    (score, kind)
}

/// Residual ley undulation (not the mountain). Used as detail on the Shasta sculpt.
pub fn realm_height(x: f32, z: f32) -> f32 {
    let r = (x * x + z * z).sqrt();
    let eta = (r * 0.18).atan();
    let xi1 = z.atan2(x);
    let q = crate::hopf::hopf_coordinates(eta, xi1, r * 0.27);
    let h = hopf_map_height(q);
    let mut bump = 0.0f32;
    for unit in hurwitz_units() {
        let p = stereographic(unit, 2.0);
        let dx = x - p.x;
        let dz = z - p.z;
        let d2 = dx * dx + dz * dz;
        bump += (-d2 / 1.35).exp() * 1.15;
    }
    h * 1.65 + bump
}

/// Isolated stratovolcano (Mt. Shasta + Shastina analogue) plus ley detail.
pub fn shasta_height(x: f32, z: f32) -> f32 {
    let peak = Vec2::new(4.2, -3.6);
    let p = Vec2::new(x, z);
    let d = (p - peak).length();
    let cone = (1.0 - (d / 10.5).min(1.0)).powf(1.28) * 12.4;
    let d2 = (p - peak - Vec2::new(-2.35, 1.15)).length();
    let shastina = (1.0 - (d2 / 3.4).min(1.0)).powf(1.45) * 5.1;
    let hills = 0.42
        * (0.52 + 0.48 * (x * 0.23 + z * 0.19).sin())
        * (0.50 + 0.50 * (x * 0.11 - z * 0.17).cos());
    let ridgeline = 0.35 * (-(((x + 8.0) * 0.08).powi(2) + ((z - 6.0) * 0.12).powi(2))).exp() * 2.2;
    0.55 + cone.max(shastina) + hills + ridgeline + realm_height(x, z) * 0.16
}

pub const SHASTA_XZ: (f32, f32) = (4.2, -3.6);

fn hopf_map_height(q: Q) -> f32 {
    hopf_map(q, HopfConvention::Classical).z
}

pub fn sanctuary_positions(scale: f32) -> Vec<(Q, Vec3)> {
    hurwitz_units()
        .into_iter()
        .map(|q| (q, stereographic(q, scale)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::hurwitz_units;

    #[test]
    fn topograph_on_hurwitz() {
        let pts = hurwitz_units().to_vec();
        let topo = build_flux_topograph(pts, vec![], Functional::HopfHeight);
        assert_eq!(topo.values.len(), 24);
        let (score, _) = magic_island_score(&topo);
        assert!(score.is_finite());
    }
}
