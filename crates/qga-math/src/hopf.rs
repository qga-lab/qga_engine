//! Hopf fibration S³ → S², stereographic projection, fiber sampling.
//!
//! - [`HopfConvention::Classical`] — QGA Chapter 2 / `flux_hopf_lib.hopf.hopf_map`.
//!   Fixture `hopf_hurwitz_v1`. No `||y||` renormalize on unit input.
//! - [`HopfConvention::Kingdom`] — `legacy_portal_map`. Model / portal pin, not Hopf.

use crate::quat::Q;
use glam::{Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HopfConvention {
    #[default]
    Classical,
    Kingdom,
}

/// S³ parametrization (η, ξ₁, ξ₂) → (x₁, x₂, x₃, x₄) with Σ xᵢ² = 1.
pub fn hopf_coordinates(eta: f32, xi1: f32, xi2: f32) -> Q {
    let (ce, se) = (eta.cos(), eta.sin());
    let (c1, s1) = (xi1.cos(), xi1.sin());
    let (c2, s2) = (xi2.cos(), xi2.sin());
    Q::new(ce * c1, ce * s1, se * c2, se * s2)
}

/// Classical real Hopf map (QGA Chapter 2 / `flux_hopf_lib.hopf.hopf_map`).
///
/// `y1=2(x1 x3+x2 x4)`, `y2=2(x1 x4-x2 x3)`, `y3=x1²+x2²-x3²-x4²`.
/// Unit input is not re-normalized by `||y||`.
pub fn hopf_map_classical(q: Q) -> Vec3 {
    let (x1, x2, x3, x4) = (q.w(), q.x(), q.y(), q.z());
    let y1 = 2.0 * (x1 * x3 + x2 * x4);
    let y2 = 2.0 * (x1 * x4 - x2 * x3);
    let y3 = x1 * x1 + x2 * x2 - x3 * x3 - x4 * x4;
    let n2 = x1 * x1 + x2 * x2 + x3 * x3 + x4 * x4;
    if n2 < 1e-14 {
        Vec3::X
    } else if (n2 - 1.0).abs() > 1e-6 {
        Vec3::new(y1, y2, y3) / n2
    } else {
        Vec3::new(y1, y2, y3)
    }
}

/// `legacy_portal_map`. Not a Hopf map. Kept so portal pins stay bit-identical.
pub fn hopf_map_kingdom(q: Q) -> Vec3 {
    let (x1, x2, x3, x4) = (q.w(), q.x(), q.y(), q.z());
    let y1 = x1 * x1 - x2 * x2;
    let y2 = 2.0 * x1 * x2;
    let y3 = 2.0 * (x3 * x4 + x1 * x2);
    let v = Vec3::new(y1, y2, y3);
    let n = v.length();
    if n < 1e-14 {
        Vec3::X
    } else {
        v / n
    }
}

pub fn hopf_map(q: Q, convention: HopfConvention) -> Vec3 {
    match convention {
        HopfConvention::Classical => hopf_map_classical(q),
        HopfConvention::Kingdom => hopf_map_kingdom(q),
    }
}

/// Stereographic projection S³ → ℝ³ (pole at x₄ = −1, TOE-compatible).
pub fn stereographic(q: Q, scale: f32) -> Vec3 {
    let denom = 1.0 - q.z() + 1e-12;
    scale * Vec3::new(q.x() / denom, q.y() / denom, q.w() / denom)
}

#[derive(Clone, Debug)]
pub struct Fiber {
    pub eta: f32,
    pub xi1: f32,
    pub points: Vec<Vec3>,
    pub s3: Vec<Q>,
    pub base: Vec3,
    pub color: Vec3,
}

/// Spread (η, ξ₁) base points for a fiber family. Matches
/// `flux_hopf_lib.hopf.fibration._fiber_base_pairs`.
pub fn fiber_base_pairs(n_fibers: usize, eta_range: (f32, f32)) -> Vec<(f32, f32)> {
    let n_eta = 1.max((n_fibers as f32).sqrt() as usize + 1);
    let n_xi1 = 1.max((n_fibers + n_eta - 1) / n_eta);
    let mut pairs = Vec::with_capacity(n_fibers);
    for ie in 0..n_eta {
        let t = if n_eta == 1 {
            0.5
        } else {
            ie as f32 / (n_eta - 1) as f32
        };
        let eta = eta_range.0 + t * (eta_range.1 - eta_range.0);
        for ix in 0..n_xi1 {
            if pairs.len() >= n_fibers {
                return pairs;
            }
            let xi1 = ix as f32 * (std::f32::consts::TAU / n_xi1 as f32);
            pairs.push((eta, xi1));
        }
    }
    pairs
}

pub fn sample_fiber(
    eta: f32,
    xi1: f32,
    n_points: usize,
    scale: f32,
    convention: HopfConvention,
) -> Fiber {
    let n = n_points.max(8);
    let mut points = Vec::with_capacity(n);
    let mut s3 = Vec::with_capacity(n);
    for i in 0..n {
        let xi2 = i as f32 * (std::f32::consts::TAU / n as f32);
        let q = hopf_coordinates(eta, xi1, xi2);
        s3.push(q);
        points.push(stereographic(q, scale));
    }
    let base = hopf_map(s3[0], convention);
    Fiber {
        eta,
        xi1,
        points,
        s3,
        base,
        // Palette only: R³ tubes are the same S³ circles. The fork is S².
        color: color_from_base(base),
    }
}

pub fn sample_fiber_family(
    n_fibers: usize,
    n_points: usize,
    eta_range: (f32, f32),
    scale: f32,
    convention: HopfConvention,
) -> Vec<Fiber> {
    fiber_base_pairs(n_fibers, eta_range)
        .into_iter()
        .map(|(eta, xi1)| sample_fiber(eta, xi1, n_points, scale, convention))
        .collect()
}

/// Color a fiber from η, cyan → gold (explorer palette).
pub fn color_from_eta(eta: f32) -> Vec3 {
    let t = ((eta - 0.1) / 1.3).clamp(0.0, 1.0);
    let cyan = Vec3::new(0.25, 0.85, 1.0);
    let gold = Vec3::new(0.95, 0.75, 0.28);
    cyan.lerp(gold, t)
}

/// Color from the convention's S² base. Kingdom vs Classical must not match.
/// Direct RGB of (x,y,z) so the fork is the palette, not a second tube geometry.
pub fn color_from_base(base: Vec3) -> Vec3 {
    Vec3::new(
        (base.x * 0.5 + 0.5).clamp(0.08, 0.95),
        (base.y * 0.5 + 0.5).clamp(0.08, 0.95),
        (base.z * 0.5 + 0.5).clamp(0.08, 0.95),
    )
}

/// Recover rough Hopf angles from S³ coordinates (portal chart).
pub fn angles_from_q(q: Q) -> (f32, f32, f32) {
    let (x1, x2, x3, x4) = (q.w(), q.x(), q.y(), q.z());
    let eta = (x3 * x3 + x4 * x4).sqrt().atan2((x1 * x1 + x2 * x2).sqrt());
    let xi1 = x2.atan2(x1);
    let xi2 = x4.atan2(x3);
    (eta, xi1, xi2)
}

pub fn as_vec4_array(q: Q) -> Vec4 {
    q.as_vec4()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_match_python() {
        let q = hopf_coordinates(0.6, 1.2, 0.0);
        assert!((q.w() - 0.29906676).abs() < 1e-5);
        assert!((q.x() - 0.76924505).abs() < 1e-5);
        assert!((q.y() - 0.56464247).abs() < 1e-5);
        assert!(q.z().abs() < 1e-6);
    }

    #[test]
    fn classical_hopf_is_chapter2_formula() {
        let q = hopf_coordinates(0.6, 1.2, 0.0);
        let (x1, x2, x3, x4) = (q.w(), q.x(), q.y(), q.z());
        let y = hopf_map_classical(q);
        let y1 = 2.0 * (x1 * x3 + x2 * x4);
        let y2 = 2.0 * (x1 * x4 - x2 * x3);
        let y3 = x1 * x1 + x2 * x2 - x3 * x3 - x4 * x4;
        assert!((y.x - y1).abs() < 1e-6);
        assert!((y.y - y2).abs() < 1e-6);
        assert!((y.z - y3).abs() < 1e-6);
        assert!((y.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn kingdom_hopf_match_python() {
        let q = hopf_coordinates(0.6, 1.2, 0.0);
        let y = hopf_map_kingdom(q);
        assert!((y.x + 0.6110565).abs() < 1e-5);
        assert!((y.y - 0.5597365).abs() < 1e-5);
        assert!((y.z - 0.5597365).abs() < 1e-5);
    }

    #[test]
    fn stereo_match_python() {
        let q = hopf_coordinates(0.6, 1.2, 0.0);
        let p = stereographic(q, 2.0);
        assert!((p.x - 1.5384901).abs() < 1e-5);
        assert!((p.y - 1.1292849).abs() < 1e-5);
        assert!((p.z - 0.5981335).abs() < 1e-5);
    }

    #[test]
    fn unit_norm() {
        let q = hopf_coordinates(0.7, 0.3, 1.1);
        assert!((q.norm() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn kingdom_and_classical_share_s3_tubes_but_not_base_or_color() {
        let a = sample_fiber(0.6, 1.2, 32, 2.0, HopfConvention::Classical);
        let b = sample_fiber(0.6, 1.2, 32, 2.0, HopfConvention::Kingdom);
        assert_eq!(a.points.len(), b.points.len());
        for (p, q) in a.points.iter().zip(b.points.iter()) {
            assert!((*p - *q).length() < 1e-5, "R³ tubes are the same S³ circles");
        }
        assert!(
            (a.base - b.base).length() > 0.05,
            "S² base must fork: {:?}",
            (a.base, b.base)
        );
        assert!(
            (a.color - b.color).length() > 0.05,
            "displayed color must fork: {:?}",
            (a.color, b.color)
        );
    }
}
