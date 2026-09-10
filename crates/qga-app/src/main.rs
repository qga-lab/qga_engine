mod app;
mod camera_rig;
mod convert;
mod fibers_json;
mod hud;
mod nbody_gpu;
mod record;
mod scene;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use hud::palette_for_preset;
use nbody_gpu::Integrator;
use qga_math::HopfConvention;
use scene::{ProfileId, SceneKind};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SceneArg {
    Lab,
    Realm,
    Cosmos,
    /// Photonic OAM–flux analog of arXiv:2607.16520
    Oam,
    /// Four-panel Lorenz analogy of reveal A/B/C/D (visualizer, not a road)
    Reveal,
}

impl From<SceneArg> for SceneKind {
    fn from(s: SceneArg) -> Self {
        match s {
            SceneArg::Lab => SceneKind::Lab,
            SceneArg::Realm => SceneKind::Realm,
            SceneArg::Cosmos => SceneKind::Cosmos,
            SceneArg::Oam => SceneKind::Oam,
            SceneArg::Reveal => SceneKind::Reveal,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ProfileArg {
    Tiny,
    Demo,
    #[value(name = "this_box")]
    ThisBox,
}

impl From<ProfileArg> for ProfileId {
    fn from(p: ProfileArg) -> Self {
        match p {
            ProfileArg::Tiny => ProfileId::Tiny,
            ProfileArg::Demo => ProfileId::Demo,
            ProfileArg::ThisBox => ProfileId::ThisBox,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ConventionArg {
    Classical,
    Kingdom,
}

impl From<ConventionArg> for HopfConvention {
    fn from(c: ConventionArg) -> Self {
        match c {
            ConventionArg::Classical => HopfConvention::Classical,
            ConventionArg::Kingdom => HopfConvention::Kingdom,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum IntegratorArg {
    Euler,
    Verlet,
}

impl From<IntegratorArg> for Integrator {
    fn from(i: IntegratorArg) -> Self {
        match i {
            IntegratorArg::Euler => Integrator::Euler,
            IntegratorArg::Verlet => Integrator::Verlet,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "qga-engine",
    about = "QGA native GPU engine (Vulkan / RTX 4090)"
)]
struct Args {
    /// lab / realm / cosmos / oam / reveal (Lorenz A/B/C/D visualizer)
    #[arg(long, value_enum, default_value_t = SceneArg::Realm)]
    scene: SceneArg,

    /// Override fiber count (lab / realm)
    #[arg(long)]
    fibers: Option<u32>,

    /// Override N-body particle count (cosmos)
    #[arg(long)]
    particles: Option<u32>,

    /// OAM index ℓ for `--scene oam` (paper default 3)
    #[arg(long)]
    ell: Option<i32>,

    /// Gauge damping κ for `--scene oam` (documentary window 0.80–0.90)
    #[arg(long)]
    kappa: Option<f32>,

    /// Probe the GPU and step the sim without opening a window
    #[arg(long)]
    headless: bool,

    /// Exit after N frames (0 = run until quit)
    #[arg(long, default_value_t = 0)]
    frames: u32,

    #[arg(long, default_value_t = 1920)]
    width: u32,

    #[arg(long, default_value_t = 1080)]
    height: u32,

    /// Six-species table (labs/global-pointer/state/particles.json)
    #[arg(long)]
    species: Option<std::path::PathBuf>,

    /// Avatar consulting sheet; used with --host to set n_i
    #[arg(long)]
    avatars: Option<std::path::PathBuf>,

    /// Host whose weights fill n_i (bud, bud2, …)
    #[arg(long)]
    host: Option<String>,

    /// After headless steps, write a subsampled species dump for the geodesic lab
    #[arg(long)]
    dump_species: Option<std::path::PathBuf>,

    /// Cosmos color preset: cluster | brown | blue | hazel | amber | green | gray | chromia | dark
    /// (colors only — cluster geometry held). Aliases: grey, heterochromia.
    #[arg(long)]
    preset: Option<String>,

    /// Count policy: tiny (CI) / demo / this_box (4090 defaults). Not adapter discovery.
    #[arg(long, value_enum, default_value_t = ProfileArg::ThisBox)]
    profile: ProfileArg,

    /// Hopf map convention. Default: lab = Kingdom (portal pin); else Classical.
    /// Kingdom is legacy_portal_map and is not Hopf.
    #[arg(long, value_enum)]
    convention: Option<ConventionArg>,

    /// Cosmos integrator. Euler is the v0 default. Verlet is the same symplectic family.
    #[arg(long, value_enum, default_value_t = IntegratorArg::Euler)]
    integrator: IntegratorArg,

    /// Cosmos |L_z| + cheap E-bound (omits pair PE). Software fact, not a paper.
    #[arg(long)]
    diag: bool,

    /// Host all-pairs Plummer PE in the bound, only when n ≤ 8192. Not a GPU force.
    #[arg(long)]
    diag_pe: bool,

    /// Load flux_hopf_lib `export_fiber_curves` JSON into lab / realm (and cosmos sky).
    #[arg(long)]
    fibers_json: Option<PathBuf>,

    /// After headless steps, grab one offscreen PNG (engine-owned still, not UploadStats).
    #[arg(long)]
    dump_png: Option<PathBuf>,

    /// Record every headless frame to MP4 (NVENC / libx264). Engine stills, not UploadStats.
    #[arg(long)]
    dump_mp4: Option<PathBuf>,

    /// Cosmos: start the `5` tour (star → clumps → pull-back).
    #[arg(long)]
    tour: bool,

    /// Initial sim clock in seconds (realm crane offset, fibre phase).
    #[arg(long, default_value_t = 0.0)]
    clock: f32,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    qga_gpu::print_claim_banner("OP1–OP6 / inner_cone mosaic");
    let args = Args::parse();

    // 3900X: 12c/24t. Leave a couple of logical cores for the compositor.
    rayon::ThreadPoolBuilder::new()
        .num_threads(22)
        .build_global()
        .ok();

    if args.headless {
        return app::run_headless(args.into());
    }
    app::run_windowed(args.into())
}

impl From<Args> for app::Launch {
    fn from(a: Args) -> Self {
        let mut launch = app::Launch {
            scene: a.scene.into(),
            fibers: a.fibers,
            particles: a.particles,
            ell: a.ell,
            kappa: a.kappa,
            frames: a.frames,
            width: a.width,
            height: a.height,
            species: a.species,
            avatars: a.avatars,
            host: a.host,
            dump_species: a.dump_species,
            palette: 0,
            profile: ProfileId::from(a.profile).profile(),
            convention: a.convention.map(Into::into),
            integrator: a.integrator.into(),
            diag: a.diag,
            diag_pe: a.diag_pe,
            fibers_json: a.fibers_json,
            dump_png: a.dump_png,
            dump_mp4: a.dump_mp4,
            tour: a.tour,
            clock: a.clock,
        };
        if let Some(id) = a.preset.as_deref() {
            let root = PathBuf::from(
                std::env::var("QGA_PLAYGROUND")
                    .unwrap_or_else(|_| "/home/kinaar/Playground".into()),
            );
            let cluster_species = root.join("labs/global-pointer/state/particles.json");
            let cluster_avatars = root.join("labs/global-pointer/avatars.json");
            if let Some(p) = palette_for_preset(id) {
                launch.species = Some(cluster_species);
                launch.avatars = Some(cluster_avatars);
                launch.host = None;
                launch.palette = p;
            } else {
                log::warn!("unknown --preset {id}; using cluster");
                launch.species = Some(cluster_species);
                launch.avatars = Some(cluster_avatars);
                launch.host = None;
                launch.palette = 0;
            }
        }
        launch
    }
}
