//! Discrete two-gyro gauged Hopf step (CPU), matching `flux_hopf_lib.flux`.

use qga_math::{theta_crit, DEFAULT_GAUGE_STRENGTH, DEFAULT_KAPPA, PI, Q};

#[derive(Clone, Copy, Debug)]
pub struct FluxLatticeConfig {
    pub kappa: f32,
    pub gauge_strength: f32,
    pub omega_l: f32,
    pub omega_r: f32,
}

impl Default for FluxLatticeConfig {
    fn default() -> Self {
        Self {
            kappa: DEFAULT_KAPPA as f32,
            gauge_strength: DEFAULT_GAUGE_STRENGTH as f32,
            omega_l: 0.025,
            omega_r: 0.0225,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GaugeStepResult {
    pub quaternion: Q,
    pub twist: f32,
    pub gauge_alpha: f32,
    pub burst: bool,
}

fn small_rotor(omega: f32) -> Q {
    Q::from_axis_angle(glam::Vec3::Z, omega)
}

pub fn two_gyro_gauge_step(
    current: Q,
    twist_history_mean: f32,
    cfg: FluxLatticeConfig,
) -> GaugeStepResult {
    let mut q = current.normalize();
    let delta_l = small_rotor(cfg.omega_l);
    let delta_r = small_rotor(cfg.omega_r).conjugate();
    q = (delta_l * q * delta_r).normalize();
    let avg = twist_history_mean.rem_euclid((2.0 * PI) as f32);
    let alpha = -cfg.gauge_strength * avg - cfg.kappa * avg * 0.1;
    let gauge_rot = Q::new(alpha.cos(), 0.0, 0.0, alpha.sin());
    q = (q * gauge_rot).normalize();
    let twist = 2.0 * q.w().clamp(-1.0, 1.0).acos();
    let t_crit = theta_crit(cfg.kappa as f64) as f32;
    GaugeStepResult {
        quaternion: q,
        twist,
        gauge_alpha: alpha,
        burst: twist > t_crit,
    }
}
