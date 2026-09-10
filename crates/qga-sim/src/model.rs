//! Model numbers. Not lattice theorems.
//!
//! `qga-math` keeps φ, π, e, Hurwitz geometry, and Hopf maps.
//! This module owns λ_t, W_g, observational 350/π, storm/disk κ, and
//! OAM survival guides. HUD/CLI must label these Model / Hypothesis.

use qga_math::{E, PI, R_RESIDUAL};

pub const DEFAULT_KAPPA: f64 = 0.85;
pub const KAPPA_DOC: f64 = 0.85;
pub const KAPPA_SIM: f64 = 0.89;
pub const DEFAULT_GAUGE_STRENGTH: f64 = 0.88;
pub const DEFAULT_TWIST_RATE: f64 = 12.5;
pub const DEFAULT_MAX_DEPTH: f64 = 56.0;

pub const DEFAULT_KAPPA_F: f32 = DEFAULT_KAPPA as f32;

/// Critical pump–relax horizon used by oam_flux emergence probes. Model.
pub const LAMBDA_T_CRIT: f64 = 2.0;

/// Topological clock lock (rad) used by RubikCone epoch sync. Model.
pub const W_G_LOCK: f64 = 111.408;
pub const W_G_LOCK_F: f32 = W_G_LOCK as f32;

/// 350/π observational signature — Hypothesis, not a theorem. Attack in op5.
pub const WG_350_OVER_PI: f64 = 350.0 / PI;

/// Storm / overchannel threshold θ_crit(κ) = π(1+κ). Model.
pub fn theta_crit(kappa: f64) -> f64 {
    PI * (1.0 + kappa)
}

/// κ⋆ = e/π − R/π² that sets B(κ⋆) = R. OAM analog guide. Model.
pub fn kappa_star() -> f64 {
    E / PI - R_RESIDUAL / (PI * PI)
}

/// Holonomy-gap bound B(κ) = π²(e/π − κ). OAM analog guide. Model.
pub fn holonomy_b(kappa: f64) -> f64 {
    PI * PI * (E / PI - kappa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_locks_are_not_lattice_theorems() {
        assert!((W_G_LOCK - 111.408).abs() < 1e-12);
        assert!((theta_crit(0.85) - std::f64::consts::PI * 1.85).abs() < 1e-12);
        assert!((WG_350_OVER_PI - 350.0 / std::f64::consts::PI).abs() < 1e-12);
        assert!((LAMBDA_T_CRIT - 2.0).abs() < 1e-15);
    }

    #[test]
    fn holonomy_b_at_kappa_star_equals_r() {
        assert!((holonomy_b(kappa_star()) - R_RESIDUAL).abs() < 1e-12);
        assert!((kappa_star() - 0.851).abs() < 5e-3);
    }
}
