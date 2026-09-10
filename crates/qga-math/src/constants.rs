//! Geometry constants. φ, π, e, and derived residuals.
//!
//! Model numbers (λ_t, W_g, 350/π, storm/disk κ, θ_crit) live in `qga-sim`.
//! A few Model copies stay here so path-dep consumers (`inner_cone`) still
//! compile against the sibling tree. Engine code must import those from
//! `qga_sim`. They are not lattice theorems.

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

pub const GOLDEN_ANGLE_RAD_F: f32 = GOLDEN_ANGLE_RAD as f32;

/// Model copy for path-dep consumers. Engine SoT: `qga_sim::DEFAULT_KAPPA`.
pub const DEFAULT_KAPPA: f64 = 0.85;
pub const DEFAULT_KAPPA_F: f32 = DEFAULT_KAPPA as f32;

/// Model copy for path-dep consumers. Engine SoT: `qga_sim::W_G_LOCK`.
pub const W_G_LOCK: f64 = 111.408;
pub const W_G_LOCK_F: f32 = W_G_LOCK as f32;

/// Model copy for path-dep consumers. Engine SoT: `qga_sim::theta_crit`.
pub fn theta_crit(kappa: f64) -> f64 {
    PI * (1.0 + kappa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_matches_python() {
        assert!((R_RESIDUAL - 0.13748568659118732).abs() < 1e-12);
        assert!((PHI - 1.618033988749895).abs() < 1e-15);
        assert!((E_INV2 - (-2.0f64).exp()).abs() < 1e-15);
    }
}
