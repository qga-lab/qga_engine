//! Engine-owned scene kinds and box defaults. The extracted qga-gpu crate
//! does not know about lab / realm / cosmos / oam / reveal.

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

#[derive(Clone, Copy, Debug)]
pub struct HardwareProfile {
    pub name: &'static str,
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
    pub tube_radius_lab: f32,
    pub tube_radius_realm: f32,
}

impl HardwareProfile {
    /// This machine: Ryzen 9 3900X (24 threads) + RTX 4090 24 GiB.
    pub const THIS_BOX: Self = Self {
        name: "RTX 4090 + Ryzen 9 3900X",
        cpu_threads: 24,
        vram_mib: 24564,
        lab_fibers: 256,
        lab_points: 192,
        realm_fibers: 128,
        realm_points: 128,
        realm_terrain: 256,
        cosmos_particles: 262_144,
        cosmos_particles_max: 524_288,
        tube_radius_lab: 0.045,
        tube_radius_realm: 0.038,
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
