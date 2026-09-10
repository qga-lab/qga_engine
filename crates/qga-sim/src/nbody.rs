//! Solar-nebula initial conditions. Integration lives on the GPU.

use glam::Vec3;
use qga_math::{hopf_coordinates, stereographic, GOLDEN_ANGLE_RAD_F};
use rayon::prelude::*;

/// 32-byte particle. Layout must stay in lockstep with `qga-gpu` `GpuParticle`.
///
/// `pad` is overloaded (do not grow a fourth force into it):
/// - `(0, 1]` — display hue for the particle shader
/// - `>= 9.5` — species id as `SPECIES_PAD_BASE + k` (k in 0..6); shader
///   annulus trigger is `pad >= 9.5`. Cleaning this wants a record-layout
///   talk with `qga_gpu`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec3,
    pub mass: f32,
    pub vel: Vec3,
    pub pad: f32,
}

const _: () = assert!(std::mem::size_of::<Particle>() == 32);

impl Particle {
    pub fn new(pos: Vec3, vel: Vec3, mass: f32) -> Self {
        Self {
            pos,
            mass,
            vel,
            pad: 0.0,
        }
    }

    /// `hue` in (0, 1] — particle shader uses HSV coloring (0 keeps cosmos look).
    pub fn with_hue(mut self, hue: f32) -> Self {
        let h = hue.rem_euclid(1.0);
        self.pad = if h < 1e-4 { 1.0 } else { h };
        self
    }

    /// Species tag for visuals|particles. Lives in `pad` as 10+k so OAM hues stay (0,1].
    pub fn with_species(mut self, k: u8) -> Self {
        self.pad = SPECIES_PAD_BASE + (k.min(5) as f32);
        self
    }

    pub fn species(self) -> Option<u8> {
        if self.pad >= 9.5 {
            Some((self.pad - SPECIES_PAD_BASE).round().clamp(0.0, 5.0) as u8)
        } else {
            None
        }
    }
}

/// Packed in `Particle.pad` as `10 + species`. Not a hue.
pub const SPECIES_PAD_BASE: f32 = 10.0;

pub fn ring_radius(world_r0: f32, ell: u8) -> f32 {
    world_r0 * (6.0 / (ell.max(1) as f32)).powf(2.0 / 3.0)
}

/// Contiguous stroma band for species `k` (0 = collarette, 5 = limbus).
/// Kepler centres stay ℓ^{-2/3}; edges meet at midpoints so the annulus fills.
pub fn species_band(world_r0: f32, ell: [u8; 6], k: usize) -> (f32, f32) {
    let ri = ring_radius(world_r0, ell[k]);
    let r_lo = if k == 0 {
        ri * 0.82
    } else {
        0.5 * (ri + ring_radius(world_r0, ell[k - 1]))
    };
    let r_hi = if k == 5 {
        ri * 1.015
    } else {
        0.5 * (ri + ring_radius(world_r0, ell[k + 1]))
    };
    (r_lo, r_hi)
}

#[derive(Clone, Copy, Debug)]
pub struct NebulaConfig {
    pub n: u32,
    pub inner: f32,
    pub outer: f32,
    pub thickness: f32,
    pub central_mass: f32,
    pub disk_mass: f32,
    pub g: f32,
    pub kappa: f32,
    pub softening: f32,
    pub seed: u64,
}

impl Default for NebulaConfig {
    fn default() -> Self {
        Self {
            n: 131_072,
            inner: 0.45,
            outer: 13.5,
            thickness: 0.16,
            central_mass: 9.0,
            disk_mass: 2.4,
            g: 0.42,
            // Weaker than the conduit default so the disk stays thin but not
            // paper-flat — camera crane shots need a little scale in z.
            kappa: 0.52,
            softening: 0.12,
            seed: 111_408,
        }
    }
}

/// Tiny splitmix64 so ICs are reproducible without extra crates.
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

/// Keplerian dusty disk + a cooler outer cushion so the field does not
/// evaporate after the inner spiral fragments.
///
/// Radius mix is biased outward vs uniform-area sampling. Circular speed uses
/// enclosed mass (star + interior disk) and a slight sub-Keplerian drift.
/// Hopf swirl stays as a small QGA signature, not a 3% unbinding kick.
pub fn spawn_nebula(cfg: NebulaConfig) -> Vec<Particle> {
    let n = cfg.n.max(256) as usize;
    let dust_n = n.saturating_sub(1).max(1);
    let m_dust = cfg.disk_mass / dust_n as f32;
    let halo_start = (dust_n * 72) / 100;
    let halo_n = dust_n.saturating_sub(halo_start).max(1);
    let r_halo_in = cfg.outer * 0.82;
    let r_halo_out = cfg.outer * 1.85;

    let mut particles = vec![Particle::new(Vec3::ZERO, Vec3::ZERO, 0.0); n];
    particles[0] = Particle::new(Vec3::ZERO, Vec3::ZERO, cfg.central_mass);

    let mut seed = cfg.seed;
    // Sequential RNG so the sequence is stable; fill in parallel after.
    let mut draws: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(dust_n);
    for _ in 0..dust_n {
        draws.push((
            u01(&mut seed),
            u01(&mut seed),
            u01(&mut seed),
            u01(&mut seed),
        ));
    }

    particles[1..]
        .par_iter_mut()
        .zip(draws.par_iter().copied())
        .enumerate()
        .for_each(|(i, (p, (u_r, u_th, u_z, u_h)))| {
            let halo = i >= halo_start;
            let r = if halo {
                let u = (i - halo_start) as f32 / halo_n as f32;
                (r_halo_in * r_halo_in + u * (r_halo_out * r_halo_out - r_halo_in * r_halo_in))
                    .sqrt()
            } else {
                // u^0.78 on area coordinate → more particles at large r than Σ=const.
                let u = ((i as f32 + 0.5) / halo_start.max(1) as f32).clamp(0.0, 1.0);
                let t = u.powf(0.78);
                (cfg.inner * cfg.inner + t * (cfg.outer * cfg.outer - cfg.inner * cfg.inner)).sqrt()
            };
            let theta = i as f32 * GOLDEN_ANGLE_RAD_F + u_th * 0.035;
            let z_scale = if halo { 0.45 } else { 1.0 };
            let z = (u_z - 0.5)
                * cfg.thickness
                * z_scale
                * (1.0 - (r / r_halo_out).clamp(0.0, 1.0) * 0.5);
            let pos = Vec3::new(r * theta.cos(), r * theta.sin(), z);

            let frac = ((r - cfg.inner) / (r_halo_out - cfg.inner).max(0.2)).clamp(0.0, 1.0);
            let m_enc = cfg.central_mass + cfg.disk_mass * frac.powf(1.35);
            let speed = (cfg.g * m_enc / r.max(0.25)).sqrt() * 0.985;
            let tang = Vec3::new(-theta.sin(), theta.cos(), 0.0);
            let rad = Vec3::new(theta.cos(), theta.sin(), 0.0);
            let mut vel = tang * speed;

            let cool = if halo { 0.45 } else { 1.0 };
            vel += rad * (u_r - 0.5) * speed * 0.010 * cool;
            vel += tang * (u_th - 0.5) * speed * 0.008 * cool;
            vel.z += (u_z - 0.5) * speed * 0.006 * cool;

            let eta = 0.25 + u_h * 0.9;
            let q = hopf_coordinates(eta, theta, u_r * std::f32::consts::TAU);
            let hopf_p = stereographic(q, 0.15);
            vel += Vec3::new(hopf_p.x, hopf_p.y, 0.0).normalize_or_zero() * speed * 0.008 * cool;

            *p = Particle::new(pos, vel, m_dust);
        });

    particles
}

/// Six-species iris disk. Count is `w_i/Σw`. Mass is the same for every type.
///
/// Kepler centres stay ℓ^{-2/3}. Each type fills its annular band (pupil → limbus)
/// with radial crypts so coverage reads as stroma, not a solar-system halo.
pub fn spawn_species_disk(
    cfg: NebulaConfig,
    world_r0: f32,
    ell: [u8; 6],
    weights: [f32; 6],
) -> Vec<Particle> {
    let n = cfg.n.max(256) as usize;
    let dust_n = n.saturating_sub(1).max(1);
    let m_dust = cfg.disk_mass / dust_n as f32;
    let wsum: f32 = weights
        .iter()
        .copied()
        .map(|w| w.max(0.0))
        .sum::<f32>()
        .max(1e-6);
    let mut counts = [0usize; 6];
    let mut assigned = 0usize;
    for k in 0..6 {
        counts[k] = ((dust_n as f32) * (weights[k].max(0.0) / wsum)).round() as usize;
        assigned += counts[k];
    }
    if assigned > dust_n {
        let mut k = 0usize;
        while assigned > dust_n {
            if counts[k] > 0 {
                counts[k] -= 1;
                assigned -= 1;
            }
            k = (k + 1) % 6;
        }
    } else {
        counts[0] += dust_n - assigned;
    }

    let mut particles = vec![Particle::new(Vec3::ZERO, Vec3::ZERO, 0.0); n];
    particles[0] = Particle::new(Vec3::ZERO, Vec3::ZERO, cfg.central_mass);

    const CRYPTS: f32 = 56.0;
    let tau = std::f32::consts::TAU;
    let mut seed = cfg.seed;
    let mut idx = 1usize;
    for k in 0..6 {
        let ni = counts[k];
        let (r_lo, r_hi) = species_band(world_r0, ell, k);
        let r_lo2 = r_lo * r_lo;
        let r_span2 = (r_hi * r_hi - r_lo2).max(1e-6);
        let crypt_mix = 0.84 - 0.08 * k as f32;
        for j in 0..ni {
            if idx >= n {
                break;
            }
            let u_r = u01(&mut seed);
            let u_z = u01(&mut seed);
            let u_h = u01(&mut seed);
            let u_a = (j as f32 + 0.5) / ni.max(1) as f32;
            let mut r = (r_lo2 + u_a * r_span2).sqrt();
            let theta_raw = j as f32 * GOLDEN_ANGLE_RAD_F + (u_r - 0.5) * 0.12;
            let spoke = (theta_raw * CRYPTS / tau).round() * (tau / CRYPTS);
            let theta = spoke * crypt_mix + theta_raw * (1.0 - crypt_mix);
            if k == 0 {
                r *= 1.0 + 0.042 * (theta * 16.0).sin();
            }
            let z = (u_z - 0.5) * cfg.thickness * 0.45;
            let pos = Vec3::new(r * theta.cos(), r * theta.sin(), z);
            let speed = (cfg.g * cfg.central_mass / r.max(0.25)).sqrt() * 0.985;
            let tang = Vec3::new(-theta.sin(), theta.cos(), 0.0);
            let mut vel = tang * speed;
            let eta = 0.25 + u_h * 0.9;
            let q = hopf_coordinates(eta, theta, u_r * tau);
            let hopf_p = stereographic(q, 0.15);
            vel += Vec3::new(hopf_p.x, hopf_p.y, 0.0).normalize_or_zero() * speed * 0.006;
            particles[idx] = Particle::new(pos, vel, m_dust).with_species(k as u8);
            idx += 1;
        }
    }
    particles
}

/// N-body workgroup size. Software fact; matches `nbody.wgsl`.
pub const NBODY_WORKGROUP: u32 = 256;

/// Round particle counts to a multiple of the n-body workgroup size.
pub fn quantize_nbody(n: u32) -> u32 {
    n.max(NBODY_WORKGROUP).div_ceil(NBODY_WORKGROUP) * NBODY_WORKGROUP
}

/// Auto-substep so √κ Δt ≪ 2 and the heaviest Plummer pair is not ballistic.
/// Software-fact host check, not a paper. Caps at 64.
pub fn nbody_substeps(dt: f32, kappa: f32, g: f32, softening: f32, m_heavy: f32) -> u32 {
    let spring = kappa.max(0.0).sqrt() * dt;
    let spring_n = if spring < 0.25 {
        1
    } else {
        (spring / 0.25).ceil() as u32
    };
    let eps = softening.max(1e-6);
    let pair_omega = (g.max(0.0) * m_heavy.max(0.0) / (eps * eps * eps)).sqrt();
    let pair = pair_omega * dt;
    let pair_n = if pair < 0.25 {
        1
    } else {
        (pair / 0.25).ceil() as u32
    };
    spring_n.max(pair_n).clamp(1, 64)
}

/// Cheap cosmos diagnostic: kinetic + midplane spring + star–disk PE + L_z.
/// Pair PE is **not** in the GPU step. Optional host all-pairs U is `--diag-pe`
/// and only when n ≤ [`DIAG_PE_N_MAX`]. Software fact, not a flywheel theorem.
#[derive(Clone, Copy, Debug)]
pub struct CosmosDiag {
    pub kinetic: f32,
    pub spring: f32,
    pub star: f32,
    /// All-pairs Plummer PE when `--diag-pe` ran. `None` means omitted.
    pub pair: Option<f32>,
    pub lz: f32,
    pub n: u32,
}

pub const DIAG_PE_N_MAX: usize = 8192;

impl CosmosDiag {
    /// K + U_spring + (pair PE if present, else star–disk U*).
    pub fn bound(&self) -> f32 {
        self.kinetic + self.spring + self.pair.unwrap_or(self.star)
    }
}

pub fn cosmos_diag(particles: &[Particle], g: f32, kappa: f32) -> CosmosDiag {
    if particles.is_empty() {
        return CosmosDiag {
            kinetic: 0.0,
            spring: 0.0,
            star: 0.0,
            pair: None,
            lz: 0.0,
            n: 0,
        };
    }
    let star_m = particles[0].mass;
    let mut kinetic = 0.0f32;
    let mut spring = 0.0f32;
    let mut star = 0.0f32;
    let mut lz = 0.0f32;
    for (i, p) in particles.iter().enumerate() {
        let v2 = p.vel.dot(p.vel);
        kinetic += 0.5 * p.mass * v2;
        spring += 0.5 * kappa * p.mass * p.pos.z * p.pos.z;
        lz += p.mass * (p.pos.x * p.vel.y - p.pos.y * p.vel.x);
        if i > 0 {
            let r = p.pos.length().max(1e-6);
            star += -g * star_m * p.mass / r;
        }
    }
    CosmosDiag {
        kinetic,
        spring,
        star,
        pair: None,
        lz,
        n: particles.len() as u32,
    }
}

/// Host all-pairs Plummer PE. Matches the kernel's 1.01 ε² self-cut.
/// Not a GPU force term. No-op (pair stays None) when n > [`DIAG_PE_N_MAX`].
pub fn cosmos_diag_pe(particles: &[Particle], g: f32, kappa: f32, eps2: f32) -> CosmosDiag {
    let mut d = cosmos_diag(particles, g, kappa);
    if particles.len() > DIAG_PE_N_MAX || particles.len() < 2 {
        return d;
    }
    let cut = eps2 * 1.01;
    let mut pair = 0.0f32;
    for i in 0..particles.len() {
        for j in (i + 1)..particles.len() {
            let r = particles[j].pos - particles[i].pos;
            let r2 = r.dot(r) + eps2;
            if r2 > cut {
                pair += -g * particles[i].mass * particles[j].mass / r2.sqrt();
            }
        }
    }
    d.pair = Some(pair);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nebula_count_and_star() {
        let cfg = NebulaConfig {
            n: 1024,
            ..NebulaConfig::default()
        };
        let p = spawn_nebula(cfg);
        assert_eq!(p.len(), 1024);
        assert!(p[0].mass > 1.0);
        let disk: f32 = p[1..].iter().map(|q| q.mass).sum();
        assert!((disk - cfg.disk_mass).abs() < 1e-3);
    }

    #[test]
    fn quantize_rounds_up() {
        assert_eq!(quantize_nbody(1), 256);
        assert_eq!(quantize_nbody(256), 256);
        assert_eq!(quantize_nbody(257), 512);
    }

    #[test]
    fn particle_record_is_32_bytes() {
        assert_eq!(std::mem::size_of::<Particle>(), 32);
    }

    #[test]
    fn substeps_grow_with_stiff_spring() {
        assert_eq!(nbody_substeps(0.001, 0.52, 0.42, 0.12, 9.0), 1);
        assert!(nbody_substeps(0.5, 16.0, 0.42, 0.12, 9.0) > 1);
        assert!(nbody_substeps(0.007, 0.52, 0.42, 0.12, 9.0) >= 1);
    }

    #[test]
    fn diag_two_body_has_angular_momentum() {
        let p = [
            Particle::new(Vec3::ZERO, Vec3::ZERO, 10.0),
            Particle::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 1.0),
        ];
        let d = cosmos_diag(&p, 1.0, 0.0);
        assert!((d.lz - 1.0).abs() < 1e-5);
        assert!(d.kinetic > 0.0);
        assert!(d.star < 0.0);
        assert!(d.pair.is_none());
        let pe = cosmos_diag_pe(&p, 1.0, 0.0, 0.0);
        assert!(pe.pair.is_some());
        assert!((pe.pair.unwrap() + 10.0).abs() < 1e-4);
    }

    #[test]
    fn species_same_mass_and_counts() {
        let cfg = NebulaConfig {
            n: 1024,
            ..NebulaConfig::default()
        };
        let w = [35.0, 19.0, 14.0, 11.0, 10.0, 11.0];
        let ell = [6u8, 5, 4, 3, 2, 1];
        let p = spawn_species_disk(cfg, 3.2, ell, w);
        assert_eq!(p.len(), 1024);
        assert!(p[0].mass > 1.0);
        assert!(p[0].species().is_none());
        let dust: Vec<_> = p.iter().skip(1).collect();
        let m0 = dust[0].mass;
        assert!(dust.iter().all(|q| (q.mass - m0).abs() < 1e-6));
        let mut c = [0usize; 6];
        for q in &dust {
            c[q.species().unwrap() as usize] += 1;
        }
        let sumc: usize = c.iter().sum();
        assert_eq!(sumc, 1023);
        assert!(c[0] > c[4]); // hoarder share > tit_for_tat
        let (lo0, _) = species_band(3.2, ell, 0);
        let (_, hi5) = species_band(3.2, ell, 5);
        let mut in_annulus = 0usize;
        for q in &dust {
            let r = (q.pos.x * q.pos.x + q.pos.y * q.pos.y).sqrt();
            if r >= lo0 * 0.90 && r <= hi5 * 1.08 {
                in_annulus += 1;
            }
        }
        assert!(in_annulus as f32 / dust.len() as f32 > 0.95);
    }
}
