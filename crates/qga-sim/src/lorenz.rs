//! Four-panel Lorenz visualizer of reveal A/B/C/D.
//!
//! This is Machine 1 (1963 convection) used as a **captioned analogy**.
//! It does not reverse reveal's numbers, does not discover a pole, and
//! does not share σ with alignment's wisdom gate.
//!
//! Motion matches the classic butterfly of many initial conditions
//! (Stephen Morris / Jake Vanderplas): particles follow the Lorenz field
//! around C+ and C−. The attractor stays put. No extra flywheel.
//!
//! A — extra rotor on one of two twins (not a quieter header)
//! B — chain paint (step index) vs geometry (lobes)
//! C — many sites; leftover is the pack mean, not a lab axis
//! D — idle: no off-opening heading, obtained stays false

use crate::Particle;
use glam::Vec3;
use qga_math::Fiber;

pub const LORENZ_SIGMA: f32 = 10.0;
pub const LORENZ_RHO: f32 = 28.0;
pub const LORENZ_BETA: f32 = 8.0 / 3.0;
const SCALE: f32 = 0.42;
/// Fifty ICs, as in the Morris / Vanderplas butterfly clip.
const N_IC: usize = 50;
const TRAIL: usize = 720;
const DT: f32 = 0.008;
const SUBSTEPS: u32 = 2;
/// Extra xy rotor on the headed twin. Kick, not a header.
const HEADER_KICK: f32 = 0.085;
/// Shader bins: <0.20 gold, <0.40 orange, <0.68 cyan, else magenta.
const HUE_PLUS: f32 = 0.55;
const HUE_MINUS: f32 = 0.28;
const COL_PLUS: Vec3 = Vec3::new(0.20, 0.62, 1.0);
const COL_MINUS: Vec3 = Vec3::new(1.00, 0.40, 0.08);
const COL_BOX: Vec3 = Vec3::new(0.42, 0.44, 0.52);
/// Vanderplas / Morris coordinate box (Lorenz x, y, z).
const BOX_X: (f32, f32) = (-25.0, 25.0);
const BOX_Y: (f32, f32) = (-35.0, 35.0);
const BOX_Z: (f32, f32) = (5.0, 55.0);

#[derive(Clone, Debug)]
struct Trace {
    p: Vec3,
    trail: Vec<Vec3>,
    cap: usize,
}

impl Trace {
    fn new(p: Vec3, cap: usize) -> Self {
        Self {
            p,
            trail: Vec::with_capacity(cap),
            cap,
        }
    }

    fn push_state(&mut self) {
        if self.trail.len() == self.cap {
            self.trail.remove(0);
        }
        self.trail.push(self.p);
    }
}

fn lorenz_rhs(p: Vec3) -> Vec3 {
    Vec3::new(
        LORENZ_SIGMA * (p.y - p.x),
        p.x * (LORENZ_RHO - p.z) - p.y,
        p.x * p.y - LORENZ_BETA * p.z,
    )
}

fn rk4(p: Vec3, dt: f32) -> Vec3 {
    let k1 = lorenz_rhs(p);
    let k2 = lorenz_rhs(p + k1 * (dt * 0.5));
    let k3 = lorenz_rhs(p + k2 * (dt * 0.5));
    let k4 = lorenz_rhs(p + k3 * dt);
    p + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0)
}

fn extra_rotor(p: Vec3, ang: f32) -> Vec3 {
    let (s, c) = ang.sin_cos();
    Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z)
}

/// Lorenz (x,y,z) → world (x, z-up, y), centered on the attractor.
fn map_pt(p: Vec3) -> Vec3 {
    Vec3::new(p.x * SCALE, (p.z - (LORENZ_RHO - 1.0)) * SCALE, p.y * SCALE)
}

/// Uniform ICs in [-15, 15]³ (Vanderplas), seed 1. Index 1 twins index 0.
fn packed_ics(n: usize) -> Vec<Vec3> {
    let mut s = 1u32;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(Vec3::new(
            -15.0 + 30.0 * unit(&mut s),
            -15.0 + 30.0 * unit(&mut s),
            -15.0 + 30.0 * unit(&mut s),
        ));
    }
    if n >= 2 {
        v[1] = v[0];
    }
    v
}

fn unit(s: &mut u32) -> f32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s >> 8) as f32 / ((1u32 << 24) as f32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealPanel {
    A,
    B,
    C,
    D,
}

impl RevealPanel {
    pub fn name(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

pub fn lorenz_equilibria() -> (Vec3, Vec3, Vec3) {
    let r = (LORENZ_BETA * (LORENZ_RHO - 1.0)).sqrt();
    (
        Vec3::ZERO,
        Vec3::new(r, r, LORENZ_RHO - 1.0),
        Vec3::new(-r, -r, LORENZ_RHO - 1.0),
    )
}

/// Live analogy of reveal A/B/C/D on the Lorenz butterfly.
pub struct RevealQuad {
    traces: Vec<Trace>,
    b_step: u32,
    pub t: f32,
    pub panel: RevealPanel,
    /// D never obtains a heading from this opening.
    pub no_mirror: bool,
}

impl Default for RevealQuad {
    fn default() -> Self {
        Self::new()
    }
}

impl RevealQuad {
    pub fn new() -> Self {
        let ics = packed_ics(N_IC);
        let traces = ics.into_iter().map(|p| Trace::new(p, TRAIL)).collect();
        Self {
            traces,
            b_step: 0,
            t: 0.0,
            panel: RevealPanel::A,
            no_mirror: true,
        }
    }

    pub fn set_panel(&mut self, panel: RevealPanel) {
        self.panel = panel;
    }

    pub fn step(&mut self, dt: f32) {
        let n = ((dt / DT).ceil() as u32).clamp(1, 8) * SUBSTEPS;
        for _ in 0..n {
            for (i, tr) in self.traces.iter_mut().enumerate() {
                let nxt = rk4(tr.p, DT);
                tr.p = if i == 1 {
                    extra_rotor(nxt, HEADER_KICK)
                } else {
                    nxt
                };
            }
            self.b_step = self.b_step.wrapping_add(1);
            self.t += DT;
        }
        for tr in &mut self.traces {
            tr.push_state();
        }
    }

    fn peloton(&self) -> Vec3 {
        if self.traces.is_empty() {
            return Vec3::ZERO;
        }
        let mut acc = Vec3::ZERO;
        for tr in &self.traces {
            acc += map_pt(tr.p);
        }
        acc / self.traces.len() as f32
    }

    pub fn particles(&self) -> Vec<Particle> {
        let mut out = Vec::with_capacity(self.traces.len() * (TRAIL + 1));
        match self.panel {
            RevealPanel::A => {
                for (i, tr) in self.traces.iter().enumerate() {
                    let mass = if i == 1 { 0.95 } else { 0.55 };
                    push_trail(&mut out, &tr.trail, mass);
                    push_head(&mut out, tr.p, if i == 1 { 2.4 } else { 1.6 });
                }
            }
            RevealPanel::B => {
                for tr in &self.traces {
                    push_trail_step(&mut out, &tr.trail, self.b_step);
                    push_head(&mut out, tr.p, 1.7);
                }
            }
            RevealPanel::C | RevealPanel::D => {
                for tr in &self.traces {
                    push_trail(&mut out, &tr.trail, 0.55);
                    push_head(&mut out, tr.p, 1.6);
                }
            }
        }
        out
    }

    pub fn hubs(&self) -> Vec<(Vec3, f32, Vec3)> {
        let (_, cp, cm) = lorenz_equilibria();
        let mut hubs = vec![(map_pt(cp), 0.26, COL_PLUS), (map_pt(cm), 0.26, COL_MINUS)];
        if self.panel == RevealPanel::C {
            hubs.push((self.peloton(), 0.50, Vec3::new(1.0, 0.82, 0.25)));
        }
        hubs
    }

    /// Coordinate box from the reference clip, plus nothing that spins the butterfly.
    pub fn fibers(&self) -> Vec<Fiber> {
        coordinate_box()
    }
}

fn lobe_hue(p: Vec3) -> f32 {
    if p.x >= 0.0 {
        HUE_PLUS
    } else {
        HUE_MINUS
    }
}

fn push_trail(out: &mut Vec<Particle>, trail: &[Vec3], mass: f32) {
    let n = trail.len().max(1) as f32;
    for (i, p) in trail.iter().enumerate() {
        let age = i as f32 / n;
        out.push(
            Particle::new(map_pt(*p), Vec3::ZERO, mass * (0.18 + 0.82 * age))
                .with_hue(lobe_hue(*p)),
        );
    }
}

fn push_trail_step(out: &mut Vec<Particle>, trail: &[Vec3], step: u32) {
    let n = trail.len().max(1) as f32;
    for (i, p) in trail.iter().enumerate() {
        let k = ((step as usize + i) % 9) as f32;
        let hue = if p.x >= 0.0 {
            0.48 + k * 0.016
        } else {
            0.22 + k * 0.016
        };
        let age = i as f32 / n;
        out.push(Particle::new(map_pt(*p), Vec3::ZERO, 0.20 + 0.70 * age).with_hue(hue));
    }
}

fn push_head(out: &mut Vec<Particle>, p: Vec3, mass: f32) {
    out.push(Particle::new(map_pt(p), Vec3::ZERO, mass).with_hue(lobe_hue(p)));
}

fn coordinate_box() -> Vec<Fiber> {
    let (x0, x1) = BOX_X;
    let (y0, y1) = BOX_Y;
    let (z0, z1) = BOX_Z;
    let c = [
        Vec3::new(x0, y0, z0),
        Vec3::new(x1, y0, z0),
        Vec3::new(x1, y1, z0),
        Vec3::new(x0, y1, z0),
        Vec3::new(x0, y0, z1),
        Vec3::new(x1, y0, z1),
        Vec3::new(x1, y1, z1),
        Vec3::new(x0, y1, z1),
    ];
    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    edges
        .iter()
        .map(|&(a, b)| {
            let pa = map_pt(c[a]);
            let pb = map_pt(c[b]);
            Fiber {
                eta: 0.2,
                xi1: 0.0,
                points: vec![pa, pb],
                s3: Vec::new(),
                base: pa,
                color: COL_BOX,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equilibria_are_convection_cusps_not_theology() {
        let (o, plus, minus) = lorenz_equilibria();
        assert!(o.length() < 1e-6);
        assert!(plus.x > 0.0 && minus.x < 0.0);
        assert!((plus.z - minus.z).abs() < 1e-5);
        assert!((plus.z - (LORENZ_RHO - 1.0)).abs() < 1e-4);
    }

    #[test]
    fn extra_rotor_diverges_it_does_not_quiet() {
        let seed = Vec3::new(0.0, 1.0, 1.05);
        let mut u = seed;
        let mut h = seed;
        for _ in 0..400 {
            u = rk4(u, DT);
            h = extra_rotor(rk4(h, DT), HEADER_KICK);
        }
        assert!(
            (u - h).length() > 5.0,
            "headed extra rotor should kick off the unheaded twin, not damp it"
        );
    }

    #[test]
    fn chain_paint_is_not_lobe_orientation() {
        let mut p = Vec3::new(0.0, 1.0, 1.05);
        let mut agree = 0u32;
        let mut n = 0u32;
        for i in 0..2000 {
            p = rk4(p, DT);
            if i < 400 {
                continue;
            }
            let paint = i % 9;
            let lobe = if p.x >= 0.0 { 0 } else { 1 };
            if paint % 2 == lobe {
                agree += 1;
            }
            n += 1;
        }
        let frac = agree as f32 / n as f32;
        assert!(
            (frac - 0.5).abs() < 0.2,
            "step-index paint must not lock to Lorenz lobe (frac={frac})"
        );
    }

    #[test]
    fn d_stays_idle_no_mirror() {
        let mut q = RevealQuad::new();
        for _ in 0..30 {
            q.step(1.0 / 60.0);
        }
        assert!(q.no_mirror);
        q.set_panel(RevealPanel::D);
        assert!(!q.particles().is_empty());
        assert!(q.hubs().len() >= 2);
        q.set_panel(RevealPanel::C);
        assert!(q.hubs().len() >= 3);
    }

    #[test]
    fn fifty_ics_and_cusps_stay_put() {
        let mut q = RevealQuad::new();
        assert_eq!(q.traces.len(), N_IC);
        for _ in 0..80 {
            q.step(1.0 / 60.0);
        }
        let (_, cp, cm) = lorenz_equilibria();
        let plus = map_pt(cp);
        let minus = map_pt(cm);
        q.set_panel(RevealPanel::B);
        let hubs = q.hubs();
        assert!((hubs[0].0 - plus).length() < 1e-5, "C+ origin stays put");
        assert!((hubs[1].0 - minus).length() < 1e-5, "C- origin stays put");
        let parts = q.particles();
        assert!(parts.len() >= N_IC);
    }

    #[test]
    fn world_is_mapped_lorenz_not_a_flywheel() {
        let q0 = RevealQuad::new();
        assert!(
            (q0.traces[0].p - q0.traces[1].p).length() < 1e-5,
            "A twins share an opening IC"
        );
        let mut q = q0;
        q.step(1.0 / 60.0);
        q.set_panel(RevealPanel::A);
        for p in q.particles() {
            assert!(
                p.pos.x.abs() < 16.0 && p.pos.y.abs() < 22.0 && p.pos.z.abs() < 18.0,
                "mapped Lorenz stays inside the coordinate box, got {}",
                p.pos
            );
        }
        assert!(
            (q.traces[0].p - q.traces[1].p).length() > 1e-4,
            "headed twin should already have been kicked"
        );
    }

    #[test]
    fn panel_switch_does_not_reset_time() {
        let mut q = RevealQuad::new();
        q.step(0.5);
        let t = q.t;
        q.set_panel(RevealPanel::B);
        assert!((q.t - t).abs() < 1e-6);
        assert_eq!(q.panel, RevealPanel::B);
    }

    #[test]
    fn particles_split_hue_by_cusp() {
        let mut q = RevealQuad::new();
        for _ in 0..120 {
            q.step(1.0 / 60.0);
        }
        for panel in [
            RevealPanel::A,
            RevealPanel::B,
            RevealPanel::C,
            RevealPanel::D,
        ] {
            q.set_panel(panel);
            let parts = q.particles();
            let mut plus = 0u32;
            let mut minus = 0u32;
            for p in &parts {
                let h = p.pad;
                if (0.20..0.40).contains(&h) {
                    minus += 1;
                } else if (0.40..0.68).contains(&h) {
                    plus += 1;
                }
            }
            assert!(
                plus > 80 && minus > 80,
                "panel {} needs both C+ cyan and C- orange (plus={plus} minus={minus})",
                panel.name()
            );
        }
    }

    #[test]
    fn coordinate_box_has_twelve_edges() {
        let q = RevealQuad::new();
        let fibers = q.fibers();
        assert_eq!(fibers.len(), 12);
        for f in &fibers {
            assert_eq!(f.points.len(), 2);
            assert!((f.color - COL_BOX).length() < 0.05);
        }
    }
}
