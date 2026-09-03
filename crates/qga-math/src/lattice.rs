//! Gauged Hopf lattice: Hurwitz 24, adjacency, left/right gauge.

use crate::hopf::{angles_from_q, hopf_map, HopfConvention};
use crate::quat::Q;
use glam::Vec3;

/// The 24 Hurwitz units on S³: ±1, ±i, ±j, ±k and (±1±i±j±k)/2.
pub fn hurwitz_units() -> [Q; 24] {
    let mut units = [Q::IDENTITY; 24];
    let mut n = 0;
    for i in 0..4 {
        for &sgn in &[-1.0f32, 1.0] {
            let mut v = [0.0f32; 4];
            v[i] = sgn;
            units[n] = Q::new(v[0], v[1], v[2], v[3]);
            n += 1;
        }
    }
    for &s0 in &[-0.5f32, 0.5] {
        for &s1 in &[-0.5f32, 0.5] {
            for &s2 in &[-0.5f32, 0.5] {
                for &s3 in &[-0.5f32, 0.5] {
                    units[n] = Q::new(s0, s1, s2, s3);
                    n += 1;
                }
            }
        }
    }
    debug_assert_eq!(n, 24);
    units
}

pub fn hopf_project_points(points: &[Q], convention: HopfConvention) -> Vec<Vec3> {
    points
        .iter()
        .copied()
        .map(|q| hopf_map(q, convention))
        .collect()
}

pub fn left_multiply(points: &[Q], u: Q) -> Vec<Q> {
    let u = u.normalize();
    points
        .iter()
        .copied()
        .map(|q| (u * q).normalize())
        .collect()
}

pub fn right_multiply(points: &[Q], u: Q) -> Vec<Q> {
    let u = u.normalize();
    points
        .iter()
        .copied()
        .map(|q| (q * u).normalize())
        .collect()
}

pub fn apply_gauge_step(points: &[Q], side: GaugeSide, unit: Q) -> Vec<Q> {
    match side {
        GaugeSide::Left => left_multiply(points, unit),
        GaugeSide::Right => right_multiply(points, unit),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeSide {
    Left,
    Right,
}

/// Cartesian product sample of Hopf angles → unit quaternions (software lattice).
pub fn sample_angle_lattice(
    n_eta: usize,
    n_xi1: usize,
    n_xi2: usize,
    eta_range: (f32, f32),
) -> Vec<Q> {
    let mut pts = Vec::with_capacity(n_eta * n_xi1 * n_xi2);
    for ie in 0..n_eta {
        let t = if n_eta == 1 {
            0.5
        } else {
            ie as f32 / (n_eta - 1) as f32
        };
        let eta = eta_range.0 + t * (eta_range.1 - eta_range.0);
        for ia in 0..n_xi1 {
            let xi1 = ia as f32 * (std::f32::consts::TAU / n_xi1.max(1) as f32);
            for ib in 0..n_xi2 {
                let xi2 = ib as f32 * (std::f32::consts::TAU / n_xi2.max(1) as f32);
                pts.push(crate::hopf::hopf_coordinates(eta, xi1, xi2));
            }
        }
    }
    pts
}

/// Candidate adjacency (QGA Open Problem 1 model).
/// Returns (along_fiber_edges, inter_fiber_edges).
pub fn candidate_adjacency(
    points: &[Q],
    base_angle_thresh: f32,
    fiber_phase_bins: usize,
    same_fiber_eta_tol: f32,
    same_fiber_xi1_tol: f32,
) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    let n = points.len();
    let mut eta = vec![0.0f32; n];
    let mut xi1 = vec![0.0f32; n];
    let mut xi2 = vec![0.0f32; n];
    for (i, q) in points.iter().enumerate() {
        let (e, a, b) = angles_from_q(*q);
        eta[i] = e;
        xi1[i] = a;
        xi2[i] = b;
    }
    let base = hopf_project_points(points, HopfConvention::Classical);
    let phase_step = std::f32::consts::TAU / fiber_phase_bins.max(1) as f32;

    let mut along = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if (eta[i] - eta[j]).abs() > same_fiber_eta_tol {
                continue;
            }
            let dxi1 = wrapped_delta(xi1[i], xi1[j]);
            if dxi1 > same_fiber_xi1_tol {
                continue;
            }
            let dxi2 = wrapped_delta(xi2[i], xi2[j]);
            if dxi2 <= phase_step * 1.25 + 1e-9 {
                along.push((i, j));
            }
        }
    }
    let mut along_set = std::collections::HashSet::new();
    for &(a, b) in &along {
        along_set.insert((a, b));
        along_set.insert((b, a));
    }
    let mut inter = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if along_set.contains(&(i, j)) {
                continue;
            }
            let c = base[i].dot(base[j]).clamp(-1.0, 1.0);
            if c.acos() <= base_angle_thresh {
                inter.push((i, j));
            }
        }
    }
    (along, inter)
}

fn wrapped_delta(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut d = (a - b).abs() % tau;
    if d > std::f32::consts::PI {
        d = tau - d;
    }
    d
}

pub fn permutes_hurwitz_units(unit: Q, side: GaugeSide) -> bool {
    let moved = apply_gauge_step(&hurwitz_units(), side, unit);
    'outer: for m in moved {
        for u in hurwitz_units() {
            if m.chordal_distance(u, false) <= 1e-5 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_four_units() {
        let u = hurwitz_units();
        assert_eq!(u.len(), 24);
        for q in u {
            assert!((q.norm() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn left_one_is_identity_action() {
        let u = hurwitz_units();
        let moved = left_multiply(&u, Q::IDENTITY);
        for (a, b) in u.iter().zip(moved.iter()) {
            assert!(a.chordal_distance(*b, false) < 1e-6);
        }
    }

    #[test]
    fn i_permutes_hurwitz() {
        let i = Q::new(0.0, 1.0, 0.0, 0.0);
        assert!(permutes_hurwitz_units(i, GaugeSide::Left));
        assert!(permutes_hurwitz_units(i, GaugeSide::Right));
    }
}
