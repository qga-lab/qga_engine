//! Fantasy realm generated from the gauged Hopf lattice.

use glam::{Vec2, Vec3};
use qga_math::{
    color_from_eta, hurwitz_units, magic_island_score, sample_fiber_family, shasta_height,
    stereographic, Fiber, Functional, HopfConvention, Q, SHASTA_XZ,
};

#[derive(Clone, Copy, Debug)]
pub enum Biome {
    Sanctuary,
    LeyMarsh,
    Ridge,
    Island,
    VoidShore,
}

#[derive(Clone, Debug)]
pub struct Sanctuary {
    pub name: &'static str,
    pub q: Q,
    pub pos: Vec3,
    pub radius: f32,
    pub color: Vec3,
}

#[derive(Clone, Debug)]
pub struct Island {
    pub center: Vec3,
    pub radius: f32,
    pub biome: Biome,
    pub score: f32,
}

#[derive(Clone, Debug)]
pub struct Heightmap {
    pub n: u32,
    pub extent: f32,
    pub heights: Vec<f32>,
}

impl Heightmap {
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let n = self.n as i32;
        let u = ((x / self.extent) * 0.5 + 0.5) * (n - 1) as f32;
        let v = ((z / self.extent) * 0.5 + 0.5) * (n - 1) as f32;
        let i = u.floor() as i32;
        let j = v.floor() as i32;
        let fu = u - i as f32;
        let fv = v - j as f32;
        let at = |ii: i32, jj: i32| {
            let ii = ii.clamp(0, n - 1) as u32;
            let jj = jj.clamp(0, n - 1) as u32;
            self.heights[(jj * self.n + ii) as usize]
        };
        let a = at(i, j);
        let b = at(i + 1, j);
        let c = at(i, j + 1);
        let d = at(i + 1, j + 1);
        a * (1.0 - fu) * (1.0 - fv) + b * fu * (1.0 - fv) + c * (1.0 - fu) * fv + d * fu * fv
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeSpec {
    pub pos: Vec3,
    pub height: f32,
    pub trunk_r: f32,
    pub canopy_r: f32,
    pub tint: f32,
}

#[derive(Clone, Debug)]
pub struct RealmWorld {
    pub sanctuaries: Vec<Sanctuary>,
    pub fibers: Vec<Fiber>,
    pub heightmap: Heightmap,
    pub islands: Vec<Island>,
    pub trees: Vec<TreeSpec>,
    pub peak: Vec3,
}

const AXIS_NAMES: [(&'static str, [f32; 4]); 8] = [
    ("Umbral Gate", [-1.0, 0.0, 0.0, 0.0]),
    ("Crownhold", [1.0, 0.0, 0.0, 0.0]),
    ("Westreach", [0.0, -1.0, 0.0, 0.0]),
    ("Eastspire", [0.0, 1.0, 0.0, 0.0]),
    ("Ashfen", [0.0, 0.0, -1.0, 0.0]),
    ("Greensward", [0.0, 0.0, 1.0, 0.0]),
    ("Dawnmere", [0.0, 0.0, 0.0, -1.0]),
    ("Nightwell", [0.0, 0.0, 0.0, 1.0]),
];

const ISLE_NAMES: [&str; 16] = [
    "Isle of Four Winds",
    "Hollow of Amber Knot",
    "Cairn of Linked Fire",
    "Well of Quiet Gold",
    "Thorn of the Left Gauge",
    "Saffron Fold",
    "Blue Meridian",
    "Sepulchre of Two Squares",
    "Garden of Right Phase",
    "Hurwitz Meadow",
    "Lattice of Twelve Bells",
    "Mirror of Antipodes",
    "Salt of the Fiber Sea",
    "Keep of the Mediant",
    "Oracle of the Flywheel",
    "Crown of the Class Group",
];

fn sanctuary_name(q: Q, half_idx: &mut usize) -> &'static str {
    for &(name, c) in &AXIS_NAMES {
        if (q.w() - c[0]).abs() < 1e-5
            && (q.x() - c[1]).abs() < 1e-5
            && (q.y() - c[2]).abs() < 1e-5
            && (q.z() - c[3]).abs() < 1e-5
        {
            return name;
        }
    }
    let name = ISLE_NAMES[*half_idx % ISLE_NAMES.len()];
    *half_idx += 1;
    name
}

#[derive(Clone, Copy, Debug)]
pub struct RealmConfig {
    pub n_fibers: u32,
    pub n_points: u32,
    pub terrain: u32,
    pub extent: f32,
    pub scale: f32,
}

impl Default for RealmConfig {
    fn default() -> Self {
        Self {
            n_fibers: 160,
            n_points: 128,
            terrain: 256,
            extent: 22.0,
            scale: 2.0,
        }
    }
}

pub fn generate_realm(cfg: RealmConfig) -> RealmWorld {
    let mut half_idx = 0usize;
    let sanctuaries: Vec<Sanctuary> = hurwitz_units()
        .into_iter()
        .map(|q| {
            let pos = stereographic(q, cfg.scale);
            let name = sanctuary_name(q, &mut half_idx);
            let (eta, _, _) = qga_math::angles_from_q(q);
            Sanctuary {
                name,
                q,
                pos,
                radius: 0.55,
                color: color_from_eta(eta.max(0.2)),
            }
        })
        .collect();

    let fibers = sample_fiber_family(
        cfg.n_fibers as usize,
        cfg.n_points as usize,
        (0.18, 1.28),
        cfg.scale,
        HopfConvention::Classical,
    );

    let n = cfg.terrain;
    let mut heights = vec![0.0f32; (n * n) as usize];
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 / (n - 1) as f32 - 0.5) * 2.0 * cfg.extent;
            let z = (j as f32 / (n - 1) as f32 - 0.5) * 2.0 * cfg.extent;
            heights[(j * n + i) as usize] = shasta_height(x, z);
        }
    }
    let heightmap = Heightmap {
        n,
        extent: cfg.extent,
        heights,
    };
    let peak = Vec3::new(
        SHASTA_XZ.0,
        heightmap.sample(SHASTA_XZ.0, SHASTA_XZ.1),
        SHASTA_XZ.1,
    );
    let trees = plant_sequoias(&heightmap);

    // Islands: high-score bumps around projected Hurwitz points that sit inland.
    let mut islands = Vec::new();
    for s in &sanctuaries {
        let xz = Vec2::new(s.pos.x, s.pos.z);
        if xz.length() > cfg.extent * 0.92 {
            continue;
        }
        let pts = vec![s.q];
        let topo = qga_math::build_flux_topograph(pts, vec![], Functional::HopfHeight);
        let (score, kind) = magic_island_score(&topo);
        let biome = match kind {
            qga_math::IslandType::Elliptic => Biome::Sanctuary,
            qga_math::IslandType::ZeroHyperbolic => Biome::Island,
            qga_math::IslandType::Hyperbolic => Biome::Ridge,
            qga_math::IslandType::Parabolic => Biome::LeyMarsh,
        };
        islands.push(Island {
            center: Vec3::new(s.pos.x, heightmap.sample(s.pos.x, s.pos.z) + 0.4, s.pos.z),
            radius: 1.1 + score,
            biome,
            score,
        });
    }

    RealmWorld {
        sanctuaries,
        fibers,
        heightmap,
        islands,
        trees,
        peak,
    }
}

fn hash21(i: u32, j: u32) -> f32 {
    let mut n = i.wrapping_mul(1597334677) ^ j.wrapping_mul(3812015801);
    n = n.wrapping_mul(747796405).wrapping_add(2891336453);
    (n >> 8) as f32 / 16777216.0
}

/// Giant-sequoia / mixed-conifer belt: below snow line, off the steep cone.
fn plant_sequoias(map: &Heightmap) -> Vec<TreeSpec> {
    let mut trees = Vec::new();
    let cell = 0.48_f32;
    let n = ((map.extent * 2.0) / cell) as i32;
    let peak = Vec2::new(SHASTA_XZ.0, SHASTA_XZ.1);
    for j in 0..n {
        for i in 0..n {
            let jx = hash21(i as u32, j as u32);
            let jy = hash21(j as u32 + 17, i as u32 + 91);
            if jx > 0.62 {
                continue;
            }
            let x = -map.extent + (i as f32 + jy) * cell;
            let z = -map.extent + (j as f32 + jx) * cell;
            let h = map.sample(x, z);
            let hx = map.sample(x + 0.35, z);
            let hz = map.sample(x, z + 0.35);
            let slope = ((hx - h).abs() + (hz - h).abs()) * 1.6;
            if h < 1.05 || h > 6.4 || slope > 0.85 {
                continue;
            }
            let dpeak = (Vec2::new(x, z) - peak).length();
            if dpeak < 3.2 {
                continue;
            }
            let grove = (-((x + 5.5).powi(2) + (z - 6.2).powi(2)) / 28.0).exp();
            if jx > 0.22 + grove * 0.45 {
                continue;
            }
            let giant = hash21(i as u32 + 3, j as u32 + 7) > 0.955;
            let scale = if giant { 1.85 } else { 0.75 + jy * 0.7 };
            let height = (1.15 + grove * 0.9) * scale;
            let trunk_r = if giant { 0.22 } else { 0.055 + 0.04 * scale };
            trees.push(TreeSpec {
                pos: Vec3::new(x, h, z),
                height,
                trunk_r,
                canopy_r: trunk_r * 3.4 + 0.18,
                tint: jy,
            });
        }
    }
    trees
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realm_has_hurwitz_sanctuaries() {
        let w = generate_realm(RealmConfig {
            n_fibers: 16,
            n_points: 32,
            terrain: 32,
            ..RealmConfig::default()
        });
        assert_eq!(w.sanctuaries.len(), 24);
        assert_eq!(w.fibers.len(), 16);
        assert_eq!(w.heightmap.heights.len(), 32 * 32);
        assert!(w.sanctuaries.iter().any(|s| s.name == "Crownhold"));
        assert!(w.peak.y > 1.0);
    }
}

#[allow(dead_code)]
pub fn biome_color(b: Biome) -> Vec3 {
    match b {
        Biome::Sanctuary => Vec3::new(0.95, 0.82, 0.40),
        Biome::LeyMarsh => Vec3::new(0.18, 0.62, 0.55),
        Biome::Ridge => Vec3::new(0.45, 0.38, 0.42),
        Biome::Island => Vec3::new(0.28, 0.72, 0.38),
        Biome::VoidShore => Vec3::new(0.10, 0.16, 0.28),
    }
}
