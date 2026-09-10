//! Left-multiply = SO(3) double cover (world gauge).
//! Right-multiply = fibre clock (day / spell phase).
//!
//! Software-fact restamp of already-sampled S³. Not a new Hopf map.

use qga_math::{color_from_eta, stereographic, Fiber, Q};
use glam::Vec3;

/// Slow world spin (left). One turn in ~180 s at 1×.
pub const LEFT_SPIN_RATE: f32 = std::f32::consts::TAU / 180.0;
/// Fibre clock (right). One turn in ~40 s at 1×.
pub const RIGHT_PHASE_RATE: f32 = std::f32::consts::TAU / 40.0;

pub fn left_rotor(time: f32) -> Q {
    Q::from_axis_angle(Vec3::Y, time * LEFT_SPIN_RATE)
}

pub fn right_rotor(time: f32) -> Q {
    Q::phase_unit(time * RIGHT_PHASE_RATE)
}

/// q' = L q R, then stereographic. Rebuilds points from `s3` when present,
/// otherwise from (η, ξ₁) so JSON imports without `s3` still clock.
pub fn restamp_fiber(f: &Fiber, left: Q, right: Q, scale: f32) -> Fiber {
    let n = f.points.len().max(f.s3.len()).max(8);
    let mut s3 = Vec::with_capacity(n);
    let mut points = Vec::with_capacity(n);
    if f.s3.len() == n {
        for q in &f.s3 {
            let qn = (left * *q * right).normalize();
            s3.push(qn);
            points.push(stereographic(qn, scale));
        }
    } else {
        for i in 0..n {
            let xi2 = i as f32 * (std::f32::consts::TAU / n as f32);
            let q = qga_math::hopf_coordinates(f.eta, f.xi1, xi2);
            let qn = (left * q * right).normalize();
            s3.push(qn);
            points.push(stereographic(qn, scale));
        }
    }
    let base = if s3.is_empty() {
        f.base
    } else {
        qga_math::hopf_map(s3[0], qga_math::HopfConvention::Classical)
    };
    Fiber {
        eta: f.eta,
        xi1: f.xi1,
        points,
        s3,
        base,
        color: if f.color.length_squared() > 1e-6 {
            f.color
        } else {
            color_from_eta(f.eta)
        },
    }
}

pub fn restamp_family(src: &[Fiber], left: Q, right: Q, scale: f32) -> Vec<Fiber> {
    src.iter()
        .map(|f| restamp_fiber(f, left, right, scale))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qga_math::{sample_fiber, HopfConvention};

    #[test]
    fn identity_gauge_keeps_points() {
        let f = sample_fiber(0.6, 1.2, 16, 2.0, HopfConvention::Classical);
        let g = restamp_fiber(&f, Q::IDENTITY, Q::IDENTITY, 2.0);
        assert_eq!(g.points.len(), f.points.len());
        for (a, b) in f.points.iter().zip(g.points.iter()) {
            assert!((*a - *b).length() < 1e-5);
        }
    }

    #[test]
    fn right_phase_moves_points() {
        let f = sample_fiber(0.6, 1.2, 16, 2.0, HopfConvention::Classical);
        let g = restamp_fiber(&f, Q::IDENTITY, right_rotor(8.0), 2.0);
        let drift: f32 = f
            .points
            .iter()
            .zip(g.points.iter())
            .map(|(a, b)| (*a - *b).length())
            .sum();
        assert!(drift > 0.01, "right-phase should clock the fibre");
    }

    #[test]
    fn left_is_not_right() {
        let f = sample_fiber(0.6, 1.2, 24, 2.0, HopfConvention::Classical);
        let l = restamp_fiber(&f, left_rotor(12.0), Q::IDENTITY, 2.0);
        let r = restamp_fiber(&f, Q::IDENTITY, right_rotor(12.0), 2.0);
        let d: f32 = l
            .points
            .iter()
            .zip(r.points.iter())
            .map(|(a, b)| (*a - *b).length())
            .sum();
        assert!(d > 0.01, "left-multiply is not the fibre clock");
    }
}
