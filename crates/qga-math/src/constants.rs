//! Shared constants. Values match `flux_hopf_lib.constants`.

pub const PHI: f64 = 1.618033988749895;
pub const E: f64 = std::f64::consts::E;
pub const PI: f64 = std::f64::consts::PI;
pub const TAU: f64 = std::f64::consts::TAU;

/// Mystery residual φ² + e² − π².
pub const R_RESIDUAL: f64 = PHI * PHI + E * E - PI * PI;

pub const E_INV2: f64 = 0.1353352832366127; // exp(-2)

/// 360°(1 − 1/φ) / 1000  (flux_hopf_lib naming).
pub const GOLDEN_ANGLE_FRACTION: f64 = 137.50776405003785 / 1000.0;
pub const GOLDEN_ANGLE_RAD: f64 = TAU * (1.0 - 1.0 / PHI);
pub const PHI_INV2: f64 = 1.0 / (PHI * PHI);

pub const DEFAULT_KAPPA: f64 = 0.85;
pub const KAPPA_DOC: f64 = 0.85;
pub const KAPPA_SIM: f64 = 0.89;
pub const DEFAULT_GAUGE_STRENGTH: f64 = 0.88;
pub const DEFAULT_TWIST_RATE: f64 = 12.5;
pub const DEFAULT_MAX_DEPTH: f64 = 56.0;

/// Critical pump–relax horizon used by oam_flux emergence probes.
pub const LAMBDA_T_CRIT: f64 = 2.0;

/// Topological clock lock (rad) used by RubikCone epoch sync.
pub const W_G_LOCK: f64 = 111.408;

/// 350/π observational signature — hypothesis, not a theorem.
pub const WG_350_OVER_PI: f64 = 350.0 / PI;

pub fn theta_crit(kappa: f64) -> f64 {
    PI * (1.0 + kappa)
}

/// κ⋆ = e/π − R/π² that sets B(κ⋆) = R.
pub fn kappa_star() -> f64 {
    E / PI - R_RESIDUAL / (PI * PI)
}

/// Holonomy-gap bound B(κ) = π²(e/π − κ).
pub fn holonomy_b(kappa: f64) -> f64 {
    PI * PI * (E / PI - kappa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_matches_python() {
        assert!((R_RESIDUAL - 0.13748568659118732).abs() < 1e-12);
        assert!((PHI - 1.618033988749895).abs() < 1e-15);
        assert!((W_G_LOCK - 111.408).abs() < 1e-12);
        assert!((theta_crit(0.85) - std::f64::consts::PI * 1.85).abs() < 1e-12);
    }

    #[test]
    fn holonomy_b_at_kappa_star_equals_r() {
        assert!((holonomy_b(kappa_star()) - R_RESIDUAL).abs() < 1e-12);
        assert!((kappa_star() - 0.851).abs() < 5e-3);
        assert!((E_INV2 - (-2.0f64).exp()).abs() < 1e-15);
    }
}

pub const DEFAULT_KAPPA_F: f32 = DEFAULT_KAPPA as f32;
pub const GOLDEN_ANGLE_RAD_F: f32 = GOLDEN_ANGLE_RAD as f32;
pub const W_G_LOCK_F: f32 = W_G_LOCK as f32;
