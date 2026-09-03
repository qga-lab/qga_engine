//! Photonic OAM–flux analog of arXiv:2607.16520 (oam_flux v0.5-preprint).
//!
//! Laguerre–Gaussian packets deposit orbital angular momentum onto a gauged
//! Hopf lattice of flux flywheels. A pump phase at λt ∈ [0, 1] is followed by
//! pure PDE relaxation to λt = 2. Mean survival clusters with the residual
//! R = φ² + e² − π² and the mystery scale e⁻².

use crate::Particle;
use glam::Vec3;
use qga_math::{
    holonomy_b, kappa_star, sample_fiber_family, Fiber, DEFAULT_KAPPA, E_INV2, GOLDEN_ANGLE_RAD,
    GOLDEN_ANGLE_RAD_F, KAPPA_DOC, KAPPA_SIM, LAMBDA_T_CRIT, PHI, Q, R_RESIDUAL,
};
use rayon::prelude::*;

const TWO_PI: f32 = std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OamPhase {
    Pump,
    Relax,
    Hold,
}

impl OamPhase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Pump => "pump",
            Self::Relax => "relax",
            Self::Hold => "hold",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OamConfig {
    pub nx: usize,
    pub dt: f32,
    pub kappa: f32,
    pub ell: i32,
    pub l_max: i32,
    pub kick_strength: f32,
    pub flywheel_sites: usize,
    pub w0: f32,
    pub lambda_t: f32,
    pub pump_fraction: f32,
    pub n_z: usize,
    pub z_end: f32,
    pub nr: usize,
    pub extent: f32,
    pub n_fibers: u32,
    pub n_fiber_pts: u32,
    /// Visual dust count (cosmos-like field). Independent of the twist lattice.
    pub n_motes: u32,
    /// Visual LG waist so the doughnut fills the frame like the nebula disk.
    pub visual_w0: f32,
    pub pump_secs: f32,
    pub relax_secs: f32,
    pub hold_secs: f32,
}

impl Default for OamConfig {
    fn default() -> Self {
        Self {
            nx: 16,
            dt: 0.001,
            kappa: DEFAULT_KAPPA as f32,
            ell: 3,
            l_max: 6,
            kick_strength: 0.06,
            flywheel_sites: 4,
            w0: 1.0,
            lambda_t: LAMBDA_T_CRIT as f32,
            pump_fraction: 0.5,
            n_z: 200,
            z_end: 5.0,
            nr: 128,
            extent: 8.5,
            n_fibers: 128,
            n_fiber_pts: 96,
            n_motes: 65536,
            visual_w0: 1.15,
            pump_secs: 10.0,
            relax_secs: 10.0,
            hold_secs: 3.0,
        }
    }
}

pub fn lambda_t_steps(kappa: f32, dt: f32, lambda_t: f32) -> u32 {
    ((lambda_t / (kappa * dt)).round() as u32).max(1)
}

pub fn golden_quantized_ells(l_max: i32) -> Vec<i32> {
    let golden = GOLDEN_ANGLE_RAD as f32;
    let n_modes = (2 * l_max + 1) as f32;
    let mut scored: Vec<(f32, i32)> = Vec::new();
    for ell in -l_max..=l_max {
        if ell == 0 {
            continue;
        }
        let phase_inc = (ell.abs() as f32) * (TWO_PI / n_modes);
        let k = (phase_inc / golden).round().max(1.0);
        let dist = (phase_inc - k * golden).abs();
        scored.push((dist, ell));
    }
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let n_pick = 1.max(scored.len() / 4);
    let mut out: Vec<i32> = scored.iter().take(n_pick).map(|s| s.1).collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn factorial(n: u32) -> f32 {
    (1..=n).fold(1.0, |a, k| a * k as f32)
}

/// p = 0 Laguerre–Gaussian radial amplitude (L₀^|ℓ| ≡ 1).
pub fn lg_radial(rho: f32, ell: i32, w0: f32) -> f32 {
    let l = ell.unsigned_abs();
    let w0 = w0.max(1e-6);
    let norm = (2.0 / (std::f32::consts::PI * factorial(l))).sqrt() / w0;
    let rw = rho / w0;
    let radial = norm * 2.0f32.powf(l as f32 * 0.5) * rw.powi(l as i32) * (-rw * rw).exp();
    if radial.is_finite() {
        radial
    } else {
        0.0
    }
}

fn lg_radial_table(nr: usize, ell: i32, w0: f32, l_max: i32) -> (Vec<f32>, Vec<f32>) {
    let rho_max = 8.0f32.max(3.0 * (2.0 * l_max as f32 + 1.0).sqrt());
    let mut rho = Vec::with_capacity(nr);
    let mut radial = Vec::with_capacity(nr);
    let mut energy = 0.0;
    let dr = if nr > 1 {
        rho_max / (nr - 1) as f32
    } else {
        1.0
    };
    for i in 0..nr {
        let r = i as f32 * dr;
        let a = lg_radial(r, ell, w0);
        rho.push(r);
        radial.push(a);
        energy += a * a * r * dr;
    }
    let nrm = energy.sqrt().max(1e-12);
    for a in radial.iter_mut() {
        *a /= nrm;
    }
    (rho, radial)
}

fn interp_radial(rho_grid: &[f32], radial: &[f32], r: f32) -> f32 {
    if rho_grid.is_empty() {
        return 0.0;
    }
    if r <= rho_grid[0] {
        return radial[0];
    }
    let last = rho_grid.len() - 1;
    if r >= rho_grid[last] {
        return 0.0;
    }
    let dr = (rho_grid[1] - rho_grid[0]).max(1e-12);
    let t = (r - rho_grid[0]) / dr;
    let i = (t.floor() as usize).min(last - 1);
    let f = t - i as f32;
    radial[i] * (1.0 - f) + radial[i + 1] * f
}

#[inline]
fn lidx(nx: usize, i: usize, j: usize, k: usize) -> usize {
    i + nx * (j + nx * k)
}

#[inline]
fn wrap(i: isize, nx: usize) -> usize {
    let n = nx as isize;
    ((i % n) + n) as usize % nx
}

fn helical_seed(nx: usize, pitch: f32, amplitude: f32) -> Vec<f32> {
    let n = nx * nx * nx;
    let mut theta = vec![0.0f32; n];
    let ds = TWO_PI / nx as f32;
    for i in 0..nx {
        let x = i as f32 * ds;
        for j in 0..nx {
            let y = j as f32 * ds;
            for k in 0..nx {
                let z = k as f32 * ds;
                theta[lidx(nx, i, j, k)] =
                    amplitude * (0.5 + 0.5 * (pitch * (x + 2.0 * y - z)).sin());
            }
        }
    }
    theta
}

#[derive(Clone, Debug)]
pub struct TwistLattice {
    pub nx: usize,
    pub dt: f32,
    pub d_diff: f32,
    pub kappa: f32,
    pub delta_omega: f32,
    pub theta_crit: f32,
    pub theta: Vec<f32>,
    pub theta_initial: Vec<f32>,
    pub momentum_ledger: f32,
}

impl TwistLattice {
    pub fn new(nx: usize, dt: f32, kappa: f32) -> Self {
        let theta = helical_seed(nx, 0.35, 1.2);
        Self {
            nx,
            dt,
            d_diff: 0.05,
            kappa,
            delta_omega: 0.002,
            theta_crit: 5.8,
            theta_initial: theta.clone(),
            theta,
            momentum_ledger: 0.0,
        }
    }

    pub fn mean_twist(&self) -> f32 {
        let n = self.theta.len() as f32;
        self.theta.iter().sum::<f32>() / n.max(1.0)
    }

    pub fn twist_variance(&self) -> f32 {
        let n = self.theta.len() as f32;
        if n < 2.0 {
            return 0.0;
        }
        let mean = self.mean_twist();
        self.theta
            .iter()
            .map(|t| {
                let d = t - mean;
                d * d
            })
            .sum::<f32>()
            / n
    }

    fn laplacian_at(&self, i: usize, j: usize, k: usize) -> f32 {
        let nx = self.nx;
        let t = self.theta[lidx(nx, i, j, k)];
        let s = self.theta[lidx(nx, wrap(i as isize + 1, nx), j, k)]
            + self.theta[lidx(nx, wrap(i as isize - 1, nx), j, k)]
            + self.theta[lidx(nx, i, wrap(j as isize + 1, nx), k)]
            + self.theta[lidx(nx, i, wrap(j as isize - 1, nx), k)]
            + self.theta[lidx(nx, i, j, wrap(k as isize + 1, nx))]
            + self.theta[lidx(nx, i, j, wrap(k as isize - 1, nx))];
        let h = 1.0 / nx as f32;
        (s - 6.0 * t) / (h * h)
    }

    fn grad_sq_numpy(&self, i: usize, j: usize, k: usize) -> f32 {
        let nx = self.nx;
        let t = |ii: usize, jj: usize, kk: usize| self.theta[lidx(nx, ii, jj, kk)];
        let d0 = if i == 0 {
            t(1, j, k) - t(0, j, k)
        } else if i + 1 == nx {
            t(nx - 1, j, k) - t(nx - 2, j, k)
        } else {
            0.5 * (t(i + 1, j, k) - t(i - 1, j, k))
        };
        let d1 = if j == 0 {
            t(i, 1, k) - t(i, 0, k)
        } else if j + 1 == nx {
            t(i, nx - 1, k) - t(i, nx - 2, k)
        } else {
            0.5 * (t(i, j + 1, k) - t(i, j - 1, k))
        };
        let d2 = if k == 0 {
            t(i, j, 1) - t(i, j, 0)
        } else if k + 1 == nx {
            t(i, j, nx - 1) - t(i, j, nx - 2)
        } else {
            0.5 * (t(i, j, k + 1) - t(i, j, k - 1))
        };
        d0 * d0 + d1 * d1 + d2 * d2
    }

    pub fn apply_kick(&mut self, kick: &[f32], photon_momentum: f32) {
        for (th, k) in self.theta.iter_mut().zip(kick.iter()) {
            *th = (*th + k).clamp(0.01, TWO_PI - 0.01);
        }
        self.momentum_ledger -= photon_momentum;
    }

    pub fn relax_step(&mut self) {
        let nx = self.nx;
        let mean = self.mean_twist();
        let gauge = -self.kappa * mean;
        let mut next = self.theta.clone();
        for i in 0..nx {
            for j in 0..nx {
                for k in 0..nx {
                    let idx = lidx(nx, i, j, k);
                    let th = self.theta[idx];
                    let lap = self.laplacian_at(i, j, k);
                    let half = th * 0.5;
                    let s = half.sin();
                    let cot = if s.abs() < 1e-6 {
                        0.0
                    } else {
                        (self.d_diff * 0.5) * half.cos() / s * self.grad_sq_numpy(i, j, k)
                    };
                    let burst = if th > self.theta_crit {
                        -50.0 * (th - self.theta_crit)
                    } else {
                        0.0
                    };
                    let rhs = self.d_diff * lap + cot + self.delta_omega + gauge + burst;
                    next[idx] = (th + self.dt * rhs).clamp(0.01, TWO_PI - 0.01);
                }
            }
        }
        self.theta = next;
    }

    pub fn flywheel_indices(&self, n_sites: usize) -> Vec<(usize, usize, usize)> {
        let stride = 1.max(self.nx / (n_sites + 1));
        (0..n_sites)
            .map(|i| {
                let s = (stride * (i + 1)).min(self.nx - 1);
                (s, s, s)
            })
            .collect()
    }

    pub fn sample(&self, i: usize, j: usize, k: usize) -> f32 {
        self.theta[lidx(
            self.nx,
            i.min(self.nx - 1),
            j.min(self.nx - 1),
            k.min(self.nx - 1),
        )]
    }
}

fn hopf_fiber_coord(nx: usize, i: usize, j: usize, k: usize) -> (f32, f32, f32) {
    let ds = TWO_PI / nx as f32;
    let x = i as f32 * ds;
    let y = j as f32 * ds;
    let z = k as f32 * ds;
    let dx = x - std::f32::consts::PI;
    let dy = y - std::f32::consts::PI;
    let rho = (dx * dx + dy * dy).sqrt();
    let phi = dy.atan2(dx);
    let eta = (x + y + z).rem_euclid(TWO_PI);
    (rho, phi, eta)
}

fn flywheel_mask(nx: usize, sites: &[(usize, usize, usize)]) -> Vec<f32> {
    let n = nx * nx * nx;
    let mut mask = vec![0.0f32; n];
    for &(si, sj, sk) in sites {
        let mut local = vec![0.0f32; n];
        local[lidx(nx, si, sj, sk)] = 1.0;
        for _ in 0..3 {
            let mut blurred = vec![0.0f32; n];
            for i in 0..nx {
                for j in 0..nx {
                    for k in 0..nx {
                        let c = local[lidx(nx, i, j, k)];
                        let sx = local[lidx(nx, wrap(i as isize + 1, nx), j, k)]
                            + local[lidx(nx, wrap(i as isize - 1, nx), j, k)];
                        let sy = local[lidx(nx, i, wrap(j as isize + 1, nx), k)]
                            + local[lidx(nx, i, wrap(j as isize - 1, nx), k)];
                        let sz = local[lidx(nx, i, j, wrap(k as isize + 1, nx))]
                            + local[lidx(nx, i, j, wrap(k as isize - 1, nx))];
                        blurred[lidx(nx, i, j, k)] = 0.25 * (sx + sy + sz) / 3.0 + 0.5 * c;
                    }
                }
            }
            local = blurred;
        }
        for (m, l) in mask.iter_mut().zip(local.iter()) {
            *m += *l;
        }
    }
    let peak = mask.iter().copied().fold(0.0f32, f32::max).max(1e-12);
    for m in mask.iter_mut() {
        *m /= peak;
    }
    mask
}

#[derive(Clone, Debug)]
pub struct OamHud {
    pub lambda_t: f32,
    pub survival: f32,
    pub photon_frac: f32,
    pub ell: i32,
    pub phase: OamPhase,
    pub curve: Vec<(f32, f32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct OamMetrics {
    pub phase: OamPhase,
    pub step: u32,
    pub total_steps: u32,
    pub lambda_t: f32,
    pub tau: f32,
    pub kappa: f32,
    pub ell: i32,
    pub mean_twist: f32,
    pub mean_survival: f32,
    pub residual_feature: f32,
    pub photon_frac: f32,
    pub bound_b: f32,
    pub pump_active: bool,
}

impl OamMetrics {
    pub fn analog_band() -> (f64, f64, f64) {
        (R_RESIDUAL, E_INV2, KAPPA_DOC)
    }
}

/// Fig. 1 (arXiv:2607.16520) color keys.
const FIG_CYAN: Vec3 = Vec3::new(0.20, 0.62, 1.00);
const FIG_ORANGE: Vec3 = Vec3::new(1.00, 0.40, 0.08);
const FIG_GOLD: Vec3 = Vec3::new(1.00, 0.78, 0.38);
const BEAM_Y: f32 = 3.65;
const LATTICE_Y: f32 = -2.05;

#[derive(Clone, Copy)]
struct OamMote {
    /// 0 core, 1 beam envelope, 2 descending helix, 3 transfer rings, 4 lattice dust
    kind: u8,
    origin: Vec3,
    phi0: f32,
    rho: f32,
    mass: f32,
}

pub struct OamDemo {
    pub cfg: OamConfig,
    lattice: TwistLattice,
    rho_grid: Vec<f32>,
    radial: Vec<f32>,
    mask: Vec<f32>,
    flywheels: Vec<(usize, usize, usize)>,
    reservoir: f32,
    p0: f32,
    z_index: usize,
    step_i: u32,
    pump_steps: u32,
    relax_steps: u32,
    post_pump_mean: f32,
    visual_t: f32,
    fibers: Vec<Fiber>,
    motes: Vec<OamMote>,
    survival_curve: Vec<(f32, f32)>,
    pub golden_ells: Vec<i32>,
}

impl OamDemo {
    pub fn new(mut cfg: OamConfig) -> Self {
        if cfg.ell == 0 {
            cfg.ell = 1;
        }
        cfg.ell = cfg.ell.clamp(-cfg.l_max, cfg.l_max);
        let lattice = TwistLattice::new(cfg.nx, cfg.dt, cfg.kappa);
        let (rho_grid, radial) = lg_radial_table(cfg.nr, cfg.ell, cfg.w0, cfg.l_max);
        let flywheels = lattice.flywheel_indices(cfg.flywheel_sites);
        let mask = flywheel_mask(cfg.nx, &flywheels);
        let total = lambda_t_steps(cfg.kappa, cfg.dt, cfg.lambda_t);
        let pump_steps = ((total as f32 * cfg.pump_fraction).round() as u32)
            .clamp(1, total)
            .min(cfg.n_z as u32);
        let relax_steps = total.saturating_sub(pump_steps);
        let p0 = cfg.ell.abs() as f32;
        let mut fibers = sample_fiber_family(
            cfg.n_fibers as usize,
            cfg.n_fiber_pts as usize,
            (0.18, 1.28),
            6.2,
            qga_math::HopfConvention::Kingdom,
        );
        // Shift the Hopf family down so it reads as the torus carpet in Fig. 1.
        for f in fibers.iter_mut() {
            for p in f.points.iter_mut() {
                p.y -= 1.15;
            }
            f.color = FIG_CYAN.lerp(FIG_ORANGE, 0.45);
        }
        let golden_ells = golden_quantized_ells(cfg.l_max);
        fibers.extend(torus_floor(5, 1.55, 0.50, LATTICE_Y, 48));
        fibers.extend(golden_spokes(LATTICE_Y + 0.35, 3.4, 5));
        for (i, _) in golden_ells.iter().enumerate() {
            fibers.push(circle_fiber_at(
                Vec3::new(0.0, LATTICE_Y + 0.12 + 0.08 * i as f32, 0.0),
                0.72 + 0.22 * i as f32,
                64,
                FIG_GOLD,
            ));
        }
        let motes = seed_oam_motes(&cfg, &golden_ells);
        Self {
            cfg,
            lattice,
            rho_grid,
            radial,
            mask,
            flywheels,
            reservoir: p0,
            p0,
            z_index: 0,
            step_i: 0,
            pump_steps,
            relax_steps,
            post_pump_mean: 0.0,
            visual_t: 0.0,
            fibers,
            motes,
            survival_curve: Vec::with_capacity(512),
            golden_ells,
        }
    }

    pub fn reset(&mut self) {
        let cfg = self.cfg;
        *self = Self::new(cfg);
    }

    pub fn set_ell(&mut self, ell: i32) {
        let mut cfg = self.cfg;
        cfg.ell = if ell == 0 {
            1
        } else {
            ell.clamp(-cfg.l_max, cfg.l_max)
        };
        *self = Self::new(cfg);
    }

    pub fn set_kappa(&mut self, kappa: f32) {
        let mut cfg = self.cfg;
        cfg.kappa = kappa.clamp(0.80, 0.90);
        *self = Self::new(cfg);
    }

    pub fn phase(&self) -> OamPhase {
        if self.step_i < self.pump_steps {
            OamPhase::Pump
        } else if self.step_i < self.pump_steps + self.relax_steps {
            OamPhase::Relax
        } else {
            OamPhase::Hold
        }
    }

    pub fn total_steps(&self) -> u32 {
        self.pump_steps + self.relax_steps
    }

    pub fn mote_count(&self) -> usize {
        self.motes.len()
    }

    fn physics_step(&mut self) {
        let pump = self.step_i < self.pump_steps;
        if pump && self.reservoir > 1e-6 {
            let n = self.lattice.theta.len();
            let mut kick = vec![0.0f32; n];
            let nx = self.lattice.nx;
            let rho_max = self.rho_grid.last().copied().unwrap_or(1.0);
            let mut voxel_rho_max = 1e-6f32;
            for i in 0..nx {
                for j in 0..nx {
                    let (rho, _, _) = hopf_fiber_coord(nx, i, j, 0);
                    voxel_rho_max = voxel_rho_max.max(rho);
                }
            }
            let ell = self.cfg.ell;
            let k_s = self.cfg.kick_strength;
            for i in 0..nx {
                for j in 0..nx {
                    for k in 0..nx {
                        let (rho_raw, phi, eta) = hopf_fiber_coord(nx, i, j, k);
                        let rho = rho_raw / voxel_rho_max * rho_max;
                        let radial = interp_radial(&self.rho_grid, &self.radial, rho);
                        let helical = (ell as f32 * phi + 0.5 * eta).cos();
                        let idx = lidx(nx, i, j, k);
                        kick[idx] = k_s * radial * helical * self.mask[idx];
                    }
                }
            }
            let deposited = k_s * kick.iter().map(|x| x.abs()).sum::<f32>();
            let delta_p = deposited.min(self.reservoir);
            self.reservoir = (self.reservoir - delta_p).max(0.0);
            self.lattice.apply_kick(&kick, delta_p);
            if self.z_index + 1 < self.cfg.n_z {
                self.z_index += 1;
            }
        }
        self.lattice.relax_step();
        self.step_i += 1;
        if self.step_i == self.pump_steps {
            self.post_pump_mean = self.lattice.mean_twist();
        }
    }

    pub fn step(&mut self, dt: f32) -> OamMetrics {
        self.visual_t += dt;
        let cycle = self.cfg.pump_secs + self.cfg.relax_secs + self.cfg.hold_secs;
        if self.visual_t >= cycle {
            self.reset();
            return self.metrics();
        }
        let target = if self.visual_t < self.cfg.pump_secs {
            let u = (self.visual_t / self.cfg.pump_secs).clamp(0.0, 1.0);
            (u * self.pump_steps as f32) as u32
        } else if self.visual_t < self.cfg.pump_secs + self.cfg.relax_secs {
            let u = ((self.visual_t - self.cfg.pump_secs) / self.cfg.relax_secs).clamp(0.0, 1.0);
            self.pump_steps + (u * self.relax_steps as f32) as u32
        } else {
            self.total_steps()
        };
        while self.step_i < target {
            self.physics_step();
        }
        let m = self.metrics();
        if self
            .survival_curve
            .last()
            .map(|(lt, _)| m.lambda_t - *lt > 0.012)
            .unwrap_or(true)
        {
            self.survival_curve.push((m.lambda_t, m.mean_survival));
        }
        m
    }

    pub fn visual_time(&self) -> f32 {
        self.visual_t
    }

    pub fn hud(&self) -> OamHud {
        let m = self.metrics();
        OamHud {
            lambda_t: m.lambda_t,
            survival: m.mean_survival,
            photon_frac: m.photon_frac,
            ell: m.ell,
            phase: m.phase,
            curve: self.survival_curve.clone(),
        }
    }

    pub fn metrics(&self) -> OamMetrics {
        let mean = self.lattice.mean_twist();
        let ref_mean = if self.post_pump_mean > 1e-6 {
            self.post_pump_mean
        } else {
            mean
        };
        let survival = if self.phase() == OamPhase::Pump {
            1.0
        } else {
            mean / ref_mean.max(1e-6)
        };
        let total = self.total_steps().max(1) as f32;
        let lambda_t = self.cfg.lambda_t * (self.step_i as f32 / total);
        OamMetrics {
            phase: self.phase(),
            step: self.step_i,
            total_steps: self.total_steps(),
            lambda_t,
            tau: lambda_t / self.cfg.lambda_t.max(1e-6) * 2.0,
            kappa: self.cfg.kappa,
            ell: self.cfg.ell,
            mean_twist: mean,
            mean_survival: survival,
            residual_feature: survival - R_RESIDUAL as f32,
            photon_frac: self.reservoir / self.p0.max(1e-6),
            bound_b: holonomy_b(self.cfg.kappa as f64) as f32,
            pump_active: self.phase() == OamPhase::Pump && self.reservoir > 1e-6,
        }
    }

    pub fn fibers(&mut self) -> &[Fiber] {
        let nx = self.lattice.nx;
        let extent = self.cfg.extent;
        let mean0 = self.lattice.theta_initial.iter().sum::<f32>()
            / self.lattice.theta_initial.len().max(1) as f32;
        let heat_boost = if self.metrics().pump_active {
            0.55
        } else {
            0.28
        };
        for f in self.fibers.iter_mut() {
            let p = f.points[f.points.len() / 2];
            let (i, j, k) = world_to_lattice(p, nx, extent);
            let th = self.lattice.sample(i, j, k);
            let t = ((th - mean0).abs() * 1.4).clamp(0.0, 1.0);
            let radial = (p.x * p.x + p.z * p.z).sqrt();
            let deposit = (1.0 - (radial / 3.5).clamp(0.0, 1.0)) * heat_boost;
            f.color = FIG_CYAN
                .lerp(FIG_ORANGE, 0.35 + 0.45 * t)
                .lerp(FIG_GOLD, deposit);
        }
        &self.fibers
    }

    pub fn hubs(&self) -> Vec<(Vec3, f32, Vec3)> {
        let m = self.metrics();
        let glow = if m.pump_active {
            1.0
        } else {
            0.55 + 0.45 * m.mean_survival
        };
        let mut hubs = Vec::new();
        hubs.push((
            Vec3::ZERO,
            0.22 + 0.10 * m.mean_survival,
            Vec3::new(0.95, 0.85, 1.0) * (0.7 + 0.5 * m.mean_survival),
        ));
        for &(i, j, k) in &self.flywheels {
            let pos = lattice_to_world(i, j, k, self.lattice.nx, self.cfg.extent);
            let th = self.lattice.sample(i, j, k);
            let heat = ((th - 0.6).abs() * 0.35).clamp(0.0, 1.0);
            let color = Vec3::new(1.0, 0.72, 0.28).lerp(Vec3::new(0.95, 0.35, 1.0), heat) * glow;
            let r = 0.18 + 0.16 * heat * glow;
            hubs.push((pos, r, color));
        }
        hubs
    }

    pub fn particles(&self) -> Vec<Particle> {
        let m = self.metrics();
        let ell = self.cfg.ell as f32;
        let spin = self.visual_t * 0.55 * ell.signum().max(1.0);
        let phase_slide = m.lambda_t * 1.6;
        let envelope = 0.50 + 0.50 * m.photon_frac;
        let n = self.motes.len();
        let mut parts = vec![Particle::new(Vec3::ZERO, Vec3::ZERO, 0.0); n];
        parts
            .par_iter_mut()
            .zip(self.motes.par_iter())
            .for_each(|(p, mote)| {
                let (pos, vel, hue, mass) = match mote.kind {
                    0 => {
                        // Persistent τ_TL core: does not spread with λt.
                        let phi = mote.phi0 + spin * 0.35;
                        let pos = mote.origin
                            + Vec3::new(0.0, mote.rho * phi.cos(), mote.rho * phi.sin());
                        let hue = if (mote.origin.x * 0.4 + phase_slide).sin() > 0.0 {
                            0.12
                        } else {
                            0.82
                        };
                        (pos, Vec3::X, hue, mote.mass * 1.15)
                    }
                    1 => {
                        // Diffracting envelope: ρ grows with λt; brightness falls.
                        let spread = 1.0 + 0.72 * m.lambda_t;
                        let fade = (1.0 - 0.28 * m.lambda_t).clamp(0.22, 1.0);
                        let phi = mote.phi0 + spin;
                        let rho = mote.rho * spread;
                        let pos = mote.origin + Vec3::new(0.0, rho * phi.cos(), rho * phi.sin());
                        (
                            pos,
                            Vec3::new(1.0, -phi.sin(), phi.cos()),
                            0.55,
                            mote.mass * fade * envelope.max(0.35),
                        )
                    }
                    2 => {
                        // Descending OAM flux helix (Fig. 1 center).
                        let phi = mote.phi0 + ell * mote.origin.y * 0.85 + spin * 1.6;
                        let pos =
                            Vec3::new(mote.rho * phi.cos(), mote.origin.y, mote.rho * phi.sin());
                        let hue = if (phi - phase_slide).sin() > 0.15 {
                            0.30
                        } else {
                            0.12
                        };
                        (
                            pos,
                            Vec3::new(-phi.sin(), -0.4, phi.cos()),
                            hue,
                            mote.mass * envelope.max(0.4),
                        )
                    }
                    3 => {
                        let phi = mote.phi0 + spin * 0.7;
                        let pos = mote.origin
                            + Vec3::new(mote.rho * phi.cos(), 0.0, mote.rho * phi.sin());
                        (
                            pos,
                            Vec3::new(-phi.sin(), 0.0, phi.cos()),
                            0.30,
                            mote.mass * envelope.max(0.4),
                        )
                    }
                    _ => {
                        let phi = mote.phi0 + spin * 0.12;
                        let pos = mote.origin
                            + Vec3::new(mote.rho * phi.cos(), 0.0, mote.rho * phi.sin());
                        let hue = if (phi * 2.0).sin() > 0.0 { 0.55 } else { 0.30 };
                        (pos, Vec3::new(-phi.sin(), 0.0, phi.cos()), hue, mote.mass)
                    }
                };
                *p = Particle::new(pos, vel, mass).with_hue(hue);
            });
        parts
    }

    pub fn title_suffix(&self) -> String {
        let m = self.metrics();
        let golden: Vec<String> = self.golden_ells.iter().map(|e| e.to_string()).collect();
        format!(
            "{} λt={:.2} S={:.3} ⟨θ⟩={:.3} p/p0={:.2} ℓ={:+} κ={:.3} B={:.3} golden[{}]  R={:.4} e⁻²={:.4} κ⋆={:.3} κsim={:.2}",
            m.phase.name(),
            m.lambda_t,
            m.mean_survival,
            m.mean_twist,
            m.photon_frac,
            m.ell,
            m.kappa,
            m.bound_b,
            golden.join(","),
            R_RESIDUAL,
            E_INV2,
            kappa_star(),
            KAPPA_SIM,
        )
    }
}

fn lattice_to_world(i: usize, j: usize, k: usize, nx: usize, extent: f32) -> Vec3 {
    let s = 2.0 * extent / nx as f32;
    Vec3::new(
        (i as f32 + 0.5) * s - extent,
        (j as f32 + 0.5) * s - extent,
        (k as f32 + 0.5) * s - extent,
    )
}

fn world_to_lattice(p: Vec3, nx: usize, extent: f32) -> (usize, usize, usize) {
    let u = ((p.x / extent) * 0.5 + 0.5) * nx as f32;
    let v = ((p.y / extent) * 0.5 + 0.5) * nx as f32;
    let w = ((p.z / extent) * 0.5 + 0.5) * nx as f32;
    let clamp = |x: f32| x.floor().clamp(0.0, (nx - 1) as f32) as usize;
    (clamp(u), clamp(v), clamp(w))
}

fn splitmix(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn u01(seed: &mut u64) -> f32 {
    (splitmix(seed) >> 11) as f32 / ((1u64 << 53) as f32)
}

fn seed_oam_motes(cfg: &OamConfig, _golden_ells: &[i32]) -> Vec<OamMote> {
    let n = cfg.n_motes.max(512) as usize;
    let ell = cfg.ell;
    let w0 = cfg.visual_w0;
    let r_peak = if ell == 0 {
        0.35 * w0
    } else {
        w0 * (ell.abs() as f32 * 0.5).sqrt()
    };
    let n_core = (n * 8 / 100).max(64);
    let n_beam = (n * 28 / 100).max(256);
    let n_helix = (n * 34 / 100).max(256);
    let n_rings = (n * 10 / 100).max(64);
    let mut seed: u64 = 260_716 + ell.unsigned_abs() as u64 * 13;
    let mut motes = Vec::with_capacity(n);

    for i in 0..n {
        let u_r = u01(&mut seed);
        let u_z = u01(&mut seed);
        let u_t = u01(&mut seed);
        let phi0 = i as f32 * GOLDEN_ANGLE_RAD_F + u_t * 0.05;
        if i < n_core {
            motes.push(OamMote {
                kind: 0,
                origin: Vec3::new((u_z - 0.5) * 12.0, BEAM_Y, 0.0),
                phi0,
                rho: 0.04 + u_r * 0.10,
                mass: 0.55 + u_t * 0.45,
            });
        } else if i < n_core + n_beam {
            let x = (u_z - 0.5) * 12.5;
            let rho = r_peak * (0.55 + 0.85 * u_r);
            motes.push(OamMote {
                kind: 1,
                origin: Vec3::new(x, BEAM_Y, 0.0),
                phi0,
                rho,
                mass: 0.08 + 0.07 * u_t,
            });
        } else if i < n_core + n_beam + n_helix {
            let t = (i - n_core - n_beam) as f32 / n_helix.max(1) as f32;
            let y = BEAM_Y + 0.4 - t * (BEAM_Y + 0.4 - LATTICE_Y);
            let rho = (1.05 - 0.55 * t) * (0.85 + 0.25 * u_r);
            motes.push(OamMote {
                kind: 2,
                origin: Vec3::new(0.0, y, 0.0),
                phi0: phi0 + ell as f32 * t * TWO_PI * 2.4,
                rho,
                mass: 0.16 + 0.12 * (1.0 - t),
            });
        } else if i < n_core + n_beam + n_helix + n_rings {
            let band = ((i as f32) * 0.37).rem_euclid(3.0) as i32;
            let y = BEAM_Y - 0.9 - band as f32 * 1.05;
            let rho = 1.15 + band as f32 * 0.38 + u_r * 0.12;
            motes.push(OamMote {
                kind: 3,
                origin: Vec3::new(0.0, y, 0.0),
                phi0,
                rho,
                mass: 0.10 + 0.04 * u_t,
            });
        } else {
            let ix = ((u_r * 7.0).floor() as i32) - 3;
            let iz = ((u_z * 7.0).floor() as i32) - 3;
            let cx = ix as f32 * 1.45;
            let cz = iz as f32 * 1.45;
            motes.push(OamMote {
                kind: 4,
                origin: Vec3::new(cx, LATTICE_Y + 0.06 * u_t, cz),
                phi0,
                rho: 0.18 + 0.38 * u_t,
                mass: 0.045 + 0.03 * u_r,
            });
        }
    }
    motes
}

/// Closed circle used as a golden-angle spoke in the transverse plane.
pub fn circle_fiber(radius: f32, n: usize, z: f32, color: Vec3) -> Fiber {
    circle_fiber_at(Vec3::new(0.0, 0.0, z), radius, n, color)
}

pub fn circle_fiber_at(center: Vec3, radius: f32, n: usize, color: Vec3) -> Fiber {
    let n = n.max(8);
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let a = i as f32 * TWO_PI / n as f32;
        points.push(center + Vec3::new(radius * a.cos(), 0.0, radius * a.sin()));
    }
    Fiber {
        eta: 0.6,
        xi1: 0.0,
        points,
        s3: vec![Q::IDENTITY; n],
        base: Vec3::Y,
        color,
    }
}

fn golden_spokes(y: f32, length: f32, n: usize) -> Vec<Fiber> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = i as f32 * GOLDEN_ANGLE_RAD_F;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let n_pts = 16;
        let mut points = Vec::with_capacity(n_pts);
        for k in 0..n_pts {
            let t = k as f32 / (n_pts - 1) as f32;
            points.push(dir * (0.15 + t * length) + Vec3::new(0.0, y, 0.0));
        }
        out.push(Fiber {
            eta: 0.4,
            xi1: a,
            points,
            s3: vec![Q::IDENTITY; n_pts],
            base: dir,
            color: FIG_GOLD,
        });
    }
    out
}

fn torus_floor(grid: i32, spacing: f32, radius: f32, y: f32, n_pts: usize) -> Vec<Fiber> {
    let mut out = Vec::new();
    for ix in -grid..=grid {
        for iz in -grid..=grid {
            if ix == 0 && iz == 0 {
                continue;
            }
            let c = Vec3::new(ix as f32 * spacing, y, iz as f32 * spacing);
            let tint = if (ix + iz) % 2 == 0 {
                FIG_CYAN
            } else {
                FIG_ORANGE
            };
            out.push(circle_fiber_at(c, radius, n_pts, tint));
        }
    }
    out
}

pub fn analog_legend() -> String {
    format!(
        "arXiv:2607.16520  R={:.4}  e⁻²={:.4}  φ={:.5}  κ_doc={}  κ⋆={:.4}  λt={}",
        R_RESIDUAL,
        E_INV2,
        PHI,
        KAPPA_DOC,
        kappa_star(),
        LAMBDA_T_CRIT
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambda_t_steps_matches_python() {
        assert_eq!(lambda_t_steps(0.85, 0.001, 2.0), 2353);
    }

    #[test]
    fn golden_ells_match_preprint() {
        let ells = golden_quantized_ells(6);
        assert!(ells.contains(&-5));
        assert!(ells.contains(&-4));
        assert!(ells.contains(&5));
    }

    #[test]
    fn lg_core_and_ring() {
        assert!(lg_radial(0.0, 0, 1.0) > lg_radial(1.0, 0, 1.0));
        assert!(lg_radial(0.0, 3, 1.0).abs() < 1e-6);
        let r_peak = (1.5f32).sqrt();
        let a_peak = lg_radial(r_peak, 3, 1.0);
        assert!(a_peak > lg_radial(0.4, 3, 1.0));
        assert!(a_peak > lg_radial(2.4, 3, 1.0));
    }

    #[test]
    fn helical_seed_positive_mean() {
        let lat = TwistLattice::new(12, 0.001, 0.85);
        assert!(lat.mean_twist() > 0.4);
        assert!(lat.twist_variance() > 0.0);
        assert_eq!(lat.flywheel_indices(4).len(), 4);
    }

    #[test]
    fn short_pump_relax_finite_survival() {
        let mut cfg = OamConfig::default();
        cfg.nx = 8;
        cfg.n_fibers = 8;
        cfg.n_fiber_pts = 16;
        cfg.n_motes = 512;
        cfg.nr = 32;
        cfg.n_z = 20;
        cfg.pump_secs = 0.05;
        cfg.relax_secs = 0.05;
        cfg.hold_secs = 0.01;
        let mut demo = OamDemo::new(cfg);
        for _ in 0..8 {
            demo.physics_step();
        }
        let m = demo.metrics();
        assert!(m.mean_twist.is_finite());
        assert!(m.mean_survival.is_finite());
        assert!(m.mean_survival > 0.0);
        assert!(!demo.particles().is_empty());
        assert!(!demo.hubs().is_empty());
        assert!(!demo.fibers().is_empty());
    }

    #[test]
    fn holonomy_cluster() {
        let b_star = holonomy_b(kappa_star());
        assert!((b_star - R_RESIDUAL).abs() < 1e-12);
        assert!((R_RESIDUAL - 0.1375).abs() < 2e-4);
        assert!((E_INV2 - 0.1353).abs() < 2e-4);
    }
}
