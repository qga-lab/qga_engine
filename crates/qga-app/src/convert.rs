//! CPU records → qga-gpu upload types. Geometry meaning stays in qga-math / qga-sim.
//!
//! `GpuParticle.pad` / `Particle.pad` is overloaded. Do not grow a fourth
//! force term into the same field; that wants a record-layout talk with
//! `qga_gpu` (32-byte contract, pinned at b9c9994):
//! - `(0, 1]` — particle-shader hue (four bins)
//! - `>= 9.5` — species id `10+k` and the WGSL annulus trigger
//! Palette remap (`display_particle`) only rewrites hue for `pad >= 9.5`.

use glam::Vec3;
use qga_gpu::{FaceVert, GpuFiber, GpuHub, GpuParticle};
use qga_math::Fiber;
use qga_sim::{Heightmap, Particle, Sanctuary, TreeSpec};

pub fn gpu_fiber(f: &Fiber) -> GpuFiber {
    GpuFiber::new(f.points.clone(), f.color)
}

pub fn gpu_fibers(src: &[Fiber]) -> Vec<GpuFiber> {
    src.iter().map(gpu_fiber).collect()
}

pub fn gpu_particle(p: Particle) -> GpuParticle {
    GpuParticle {
        pos: p.pos.into(),
        mass: p.mass,
        vel: p.vel.into(),
        pad: p.pad,
    }
}

pub fn gpu_particles(src: &[Particle]) -> Vec<GpuParticle> {
    src.iter().copied().map(gpu_particle).collect()
}

pub fn gpu_hubs_from_tuples(hubs: &[(Vec3, f32, Vec3)]) -> Vec<GpuHub> {
    hubs.iter()
        .map(|(pos, r, col)| GpuHub::new(*pos, *r, *col))
        .collect()
}

pub fn sanctuary_hubs(hubs: &[Sanctuary], y_lift: impl Fn(f32, f32) -> f32) -> Vec<GpuHub> {
    hubs.iter()
        .map(|s| {
            let y = y_lift(s.pos.x, s.pos.z) + 0.55;
            GpuHub::new(Vec3::new(s.pos.x, y, s.pos.z), s.radius.max(0.45), s.color)
        })
        .collect()
}

pub fn tree_hubs(trees: &[TreeSpec]) -> Vec<GpuHub> {
    let mut out = Vec::with_capacity(trees.len() * 2);
    for t in trees {
        let trunk = Vec3::new(0.32, 0.18, 0.08) * (0.65 + 0.35 * t.tint);
        let canopy = Vec3::new(0.14, 0.42, 0.16) * (0.55 + 0.45 * t.tint);
        out.push(GpuHub::new(
            t.pos + Vec3::Y * t.height * 0.35,
            t.trunk_r.max(0.08),
            trunk,
        ));
        out.push(GpuHub::new(
            t.pos + Vec3::Y * t.height * 0.72,
            t.canopy_r.max(0.12),
            canopy,
        ));
    }
    out
}

/// Particle shader is four-bin hue from `pad` in (0, 1]. Species live at `pad = 10+k`.
pub fn display_particle(p: GpuParticle, palette: u32) -> GpuParticle {
    if p.pad < 9.5 {
        return p;
    }
    let k = (p.pad - 10.0).round().clamp(0.0, 5.0) as u32;
    let hue = species_hue(palette, k);
    p.with_hue(hue)
}

pub fn display_particles(src: &[GpuParticle], palette: u32) -> Vec<GpuParticle> {
    src.iter().copied().map(|p| display_particle(p, palette)).collect()
}

fn species_hue(palette: u32, k: u32) -> f32 {
    // Four shader bins: <0.20 gold, <0.40 orange, <0.68 blue, else pink.
    let base = match k {
        0 => 0.10,
        1 => 0.28,
        2 => 0.36,
        3 => 0.52,
        4 => 0.62,
        _ => 0.82,
    };
    let shift = (palette as f32) * 0.07;
    (base + shift).rem_euclid(1.0).max(0.02)
}

pub fn terrain_faces(map: &Heightmap) -> Vec<FaceVert> {
    let n = map.n;
    if n < 2 {
        return Vec::new();
    }
    let extent = map.extent;
    let mut verts = Vec::with_capacity((n * n) as usize);
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 / (n - 1) as f32 - 0.5) * 2.0 * extent;
            let z = (j as f32 / (n - 1) as f32 - 0.5) * 2.0 * extent;
            let h = map.heights[(j * n + i) as usize];
            let hx = if i + 1 < n {
                map.heights[(j * n + i + 1) as usize]
            } else {
                h
            };
            let hz = if j + 1 < n {
                map.heights[((j + 1) * n + i) as usize]
            } else {
                h
            };
            let step = 2.0 * extent / (n - 1) as f32;
            let dx = Vec3::new(step, hx - h, 0.0);
            let dz = Vec3::new(0.0, hz - h, step);
            let nrm = dx.cross(dz).normalize_or_zero();
            let steep = 1.0 - nrm.y.abs();
            let duff = Vec3::new(0.12, 0.16, 0.08);
            let meadow = Vec3::new(0.22, 0.38, 0.16);
            let rock = Vec3::new(0.38, 0.34, 0.32);
            let ash = Vec3::new(0.45, 0.40, 0.38);
            let snow = Vec3::new(0.92, 0.94, 0.97);
            let mut col = duff.lerp(meadow, smoothstep(0.8, 2.2, h));
            col = col.lerp(rock, smoothstep(5.6, 7.4, h));
            col = col.lerp(ash, smoothstep(7.2, 9.0, h) * steep);
            col = col.lerp(snow, smoothstep(8.4, 10.2, h) * (0.35 + 0.65 * (1.0 - steep)));
            let rock_mix = (steep * 1.4).clamp(0.0, 1.0) * (1.0 - smoothstep(9.5, 11.0, h));
            col = col.lerp(rock, rock_mix);
            verts.push(FaceVert {
                pos: [x, h, z],
                alpha: 1.0,
                color: col.into(),
                pad: 0.0,
                nrm: nrm.into(),
                pad2: 0.0,
            });
        }
    }
    let mut faces = Vec::with_capacity(((n - 1) * (n - 1) * 6) as usize);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as usize;
            let b = a + 1;
            let c = a + n as usize;
            let d = c + 1;
            faces.extend_from_slice(&[verts[a], verts[c], verts[b], verts[b], verts[c], verts[d]]);
        }
    }
    faces
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
