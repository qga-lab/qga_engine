//! Engine-owned scene kinds and box defaults. The extracted qga-gpu crate
//! does not know about lab / realm / cosmos / oam / reveal.
//!
//! Counts are scene policy, not adapter discovery. Three named rows plus
//! CLI overrides; no capability query for v1.

use qga_math::HopfConvention;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Lab,
    Realm,
    Cosmos,
    /// Photonic OAM–flux analog (arXiv:2607.16520).
    Oam,
    /// Four-panel Lorenz analogy of reveal A/B/C/D. Visualizer, not a road.
    Reveal,
}

impl SceneKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lab => "lab",
            Self::Realm => "realm",
            Self::Cosmos => "cosmos",
            Self::Oam => "oam",
            Self::Reveal => "reveal",
        }
    }
}

/// Lab is the Kingdom portal pin. Everything else is Classical unless
/// `--convention` overrides. Kingdom is `legacy_portal_map` and is not Hopf.
pub fn scene_convention(scene: SceneKind, override_c: Option<HopfConvention>) -> HopfConvention {
    if let Some(c) = override_c {
        return c;
    }
    match scene {
        SceneKind::Lab => HopfConvention::Kingdom,
        _ => HopfConvention::Classical,
    }
}

pub fn convention_label(c: HopfConvention) -> &'static str {
    match c {
        HopfConvention::Classical => "Classical",
        HopfConvention::Kingdom => "Kingdom (portal pin)",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileId {
    Tiny,
    Demo,
    ThisBox,
}

impl ProfileId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Demo => "demo",
            Self::ThisBox => "this_box",
        }
    }

    pub fn profile(self) -> HardwareProfile {
        match self {
            Self::Tiny => HardwareProfile::TINY,
            Self::Demo => HardwareProfile::DEMO,
            Self::ThisBox => HardwareProfile::THIS_BOX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HardwareProfile {
    pub name: &'static str,
    pub id: ProfileId,
    #[allow(dead_code)]
    pub cpu_threads: u32,
    #[allow(dead_code)]
    pub vram_mib: u32,
    pub lab_fibers: u32,
    pub lab_points: u32,
    pub realm_fibers: u32,
    pub realm_points: u32,
    pub realm_terrain: u32,
    pub cosmos_particles: u32,
    pub cosmos_particles_max: u32,
    pub cosmos_sky_fibers: u32,
    pub cosmos_sky_points: u32,
    pub oam_fibers: u32,
    pub oam_points: u32,
    pub oam_motes: u32,
    pub oam_nx: u32,
    pub tube_radius_lab: f32,
    pub tube_radius_realm: f32,
    pub glow: f32,
}

impl HardwareProfile {
    /// Headless CI / laptop smoke. Not a 4090 policy document.
    pub const TINY: Self = Self {
        name: "tiny",
        id: ProfileId::Tiny,
        cpu_threads: 4,
        vram_mib: 1024,
        lab_fibers: 32,
        lab_points: 24,
        realm_fibers: 32,
        realm_points: 32,
        realm_terrain: 32,
        cosmos_particles: 256,
        cosmos_particles_max: 1024,
        cosmos_sky_fibers: 16,
        cosmos_sky_points: 24,
        oam_fibers: 32,
        oam_points: 24,
        oam_motes: 256,
        oam_nx: 8,
        tube_radius_lab: 0.05,
        tube_radius_realm: 0.04,
        glow: 0.55,
    };

    /// Short demo. Still a multiple of workgroup 256 for cosmos.
    pub const DEMO: Self = Self {
        name: "demo",
        id: ProfileId::Demo,
        cpu_threads: 8,
        vram_mib: 4096,
        lab_fibers: 64,
        lab_points: 48,
        realm_fibers: 64,
        realm_points: 64,
        realm_terrain: 64,
        cosmos_particles: 4096,
        cosmos_particles_max: 16384,
        cosmos_sky_fibers: 24,
        cosmos_sky_points: 48,
        oam_fibers: 64,
        oam_points: 48,
        oam_motes: 4096,
        oam_nx: 12,
        tube_radius_lab: 0.048,
        tube_radius_realm: 0.04,
        glow: 0.85,
    };

    /// This machine: Ryzen 9 3900X (24 threads) + RTX 4090 24 GiB.
    pub const THIS_BOX: Self = Self {
        name: "this_box (RTX 4090 + Ryzen 9 3900X)",
        id: ProfileId::ThisBox,
        cpu_threads: 24,
        vram_mib: 24564,
        lab_fibers: 256,
        lab_points: 192,
        realm_fibers: 128,
        realm_points: 128,
        realm_terrain: 256,
        cosmos_particles: 262_144,
        cosmos_particles_max: 524_288,
        cosmos_sky_fibers: 48,
        cosmos_sky_points: 96,
        oam_fibers: 128,
        oam_points: 96,
        oam_motes: 65_536,
        oam_nx: 16,
        tube_radius_lab: 0.045,
        tube_radius_realm: 0.038,
        glow: 1.15,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct PlanetMarker {
    pub pos: glam::Vec3,
    #[allow(dead_code)]
    pub count: u32,
    pub radius: f32,
    #[allow(dead_code)]
    pub color: glam::Vec3,
}
