mod app;
mod camera_rig;
mod convert;
mod hud;
mod nbody_gpu;
mod record;
mod scene;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use hud::palette_for_preset;
use scene::SceneKind;
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
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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
