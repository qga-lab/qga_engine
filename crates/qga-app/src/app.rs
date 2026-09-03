use crate::camera_rig;
use crate::convert;
use crate::hud::{
    build_cosmos_hud, build_oam_hud, build_reveal_hud, palette_for_preset, preset_hit, view_hit,
    PRESET_IDS, VIEW_IDS,
};
use crate::nbody_gpu::{NbodyGpu, SimParams};
use crate::record::{self, VideoRecorder, RECORD_FPS};
use crate::scene::{HardwareProfile, SceneKind};
use anyhow::{Context, Result};
use glam::Vec3;
use qga_gpu::{
    Camera, CameraMode, GpuContext, GpuOrbInstance, LineStyle, Renderer, VisualState,
};
use qga_math::{sample_fiber_family, HopfConvention};
use qga_sim::{
    analog_legend, generate_realm, quantize_nbody, ring_radius, spawn_nebula, spawn_species_disk,
    NebulaConfig, OamConfig, OamDemo, RealmConfig, RevealPanel, RevealQuad, SPECIES_PAD_BASE,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

#[derive(Clone, Debug)]
pub struct Launch {
    pub scene: SceneKind,
    pub fibers: Option<u32>,
    pub particles: Option<u32>,
    pub ell: Option<i32>,
    pub kappa: Option<f32>,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub species: Option<PathBuf>,
    pub avatars: Option<PathBuf>,
    pub host: Option<String>,
    pub dump_species: Option<PathBuf>,
    pub palette: u32,
}

pub fn run_headless(launch: Launch) -> Result<()> {
    log::info!("QGA Engine headless — {}", HardwareProfile::THIS_BOX.name);
    let gpu = GpuContext::init_headless()?;
    log::info!("{}", gpu.report());
    let mut renderer = Renderer::new(&gpu)?;
    let mut nbody = None;
    let mut oam = if launch.scene == SceneKind::Oam {
        Some(make_oam_demo(&launch))
    } else {
        None
    };
    let mut reveal = if launch.scene == SceneKind::Reveal {
        Some(RevealQuad::new())
    } else {
        None
    };
    load_scene(
        &gpu,
        &mut renderer,
        &mut nbody,
        launch.scene,
        &launch,
        oam.as_mut(),
        launch.palette,
        false,
    )?;
    if let Some(demo) = reveal.as_mut() {
        demo.step(0.0);
        sync_reveal(&gpu, &mut renderer, demo)?;
    }
    let n = launch.frames.max(1);
    let t0 = Instant::now();
    for i in 0..n {
        if launch.scene == SceneKind::Cosmos {
            if let Some(nb) = nbody.as_mut() {
                nb.step(&gpu, 1);
            }
        }
        if let Some(demo) = oam.as_mut() {
            demo.step(1.0 / 60.0);
            sync_oam(&gpu, &mut renderer, demo)?;
        }
        if let Some(demo) = reveal.as_mut() {
            demo.step(1.0 / 60.0);
            sync_reveal(&gpu, &mut renderer, demo)?;
        }
        gpu.device.poll(wgpu::Maintain::Wait);
        if i == 0 || i + 1 == n {
            log::info!("frame {} / {}", i + 1, n);
        }
    }
    if let Some(demo) = oam.as_ref() {
        log::info!("{}", demo.title_suffix());
    }
    let n_part = nbody.as_ref().map(|nb| nb.len()).unwrap_or_else(|| renderer.particle_count());
    log::info!(
        "headless ok: {} frames in {:.3}s | fibers={} particles={}",
        n,
        t0.elapsed().as_secs_f32(),
        renderer.fiber_count(),
        n_part
    );
    if let Some(path) = launch.dump_species.as_ref() {
        let Some(nb) = nbody.as_mut() else {
            anyhow::bail!("--dump-species requires --scene cosmos");
        };
        dump_species_snapshot(&gpu, nb, &launch, path)?;
    }
    Ok(())
}

pub fn run_windowed(launch: Launch) -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let scene = launch.scene;
    let palette = launch.palette;
    let preset_ix = PRESET_IDS
        .iter()
        .position(|&id| palette_for_preset(id) == Some(palette))
        .unwrap_or(0);
    let mut app = EngineApp {
        launch,
        window: None,
        gpu: None,
        renderer: None,
        camera: Camera::orbit(Vec3::ZERO, 8.5),
        vis: VisualState::default(),
        palette,
        grid: false,
        tour: false,
        tour_t: 0.0,
        nbody: None,
        scene,
        keys: Keys::default(),
        lmb: false,
        rmb: false,
        last: Instant::now(),
        time: 0.0,
        frames: 0,
        title_acc: 0.0,
        profile: HardwareProfile::THIS_BOX,
        recorder: None,
        want_shot: false,
        last_rec: Instant::now(),
        oam: None,
        reveal: None,
        preset_open: false,
        preset_ix,
        view_open: false,
        view_ix: 0,
        tabs_hidden: false,
        cursor: [0.0, 0.0],
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct Keys {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    space: bool,
    ctrl: bool,
}

struct EngineApp {
    launch: Launch,
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    camera: Camera,
    vis: VisualState,
    palette: u32,
    grid: bool,
    tour: bool,
    tour_t: f32,
    nbody: Option<NbodyGpu>,
    scene: SceneKind,
    keys: Keys,
    lmb: bool,
    rmb: bool,
    last: Instant,
    time: f32,
    frames: u32,
    title_acc: f32,
    profile: HardwareProfile,
    recorder: Option<VideoRecorder>,
    want_shot: bool,
    last_rec: Instant,
    oam: Option<OamDemo>,
    reveal: Option<RevealQuad>,
    preset_open: bool,
    preset_ix: usize,
    view_open: bool,
    view_ix: usize,
    tabs_hidden: bool,
    cursor: [f32; 2],
}

impl EngineApp {
    fn boot(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let attrs = Window::default_attributes()
            .with_title("QGA Engine")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.launch.width,
                self.launch.height,
            ));
        let window = Arc::new(event_loop.create_window(attrs)?);
        let gpu = GpuContext::init_windowed(window.clone())?;
        log::info!("{}", gpu.report());
        let size = window.inner_size();
        self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
        let mut renderer = Renderer::new(&gpu)?;
        self.scene = self.launch.scene;
        if self.scene == SceneKind::Oam {
            self.oam = Some(make_oam_demo(&self.launch));
        }
        if self.scene == SceneKind::Reveal {
            self.reveal = Some(RevealQuad::new());
        }
        load_scene(
            &gpu,
            &mut renderer,
            &mut self.nbody,
            self.scene,
            &self.launch,
            self.oam.as_mut(),
            self.palette,
            self.grid,
        )?;
        if let Some(demo) = self.reveal.as_mut() {
            demo.step(0.0);
            sync_reveal(&gpu, &mut renderer, demo)?;
        }
        apply_camera_for_scene(&mut self.camera, self.scene);
        if self.scene == SceneKind::Oam {
            self.vis.glow = 0.88;
        }
        if self.scene == SceneKind::Reveal {
            self.vis.glow = 0.72;
            self.vis.show_rings = true;
        }
        self.window = Some(window);
        self.gpu = Some(gpu);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn tick(&mut self) -> Result<()> {
        let dt = self.last.elapsed().as_secs_f32().clamp(0.0, 0.05);
        self.last = Instant::now();
        if !self.vis.paused {
            self.time += dt;
        }
        self.frames += 1;
        self.title_acc += dt;

        let wish = Vec3::new(
            (self.keys.d as i32 - self.keys.a as i32) as f32,
            (self.keys.space as i32 - self.keys.ctrl as i32) as f32,
            (self.keys.w as i32 - self.keys.s as i32) as f32,
        );
        if self.rmb || self.camera.mode == CameraMode::Fly {
            self.camera.fly_move(wish, dt);
        }
        if !self.vis.paused {
            match self.scene {
                SceneKind::Cosmos if self.tour => {
                    camera_rig::tick_tour(&mut self.camera, dt, &mut self.tour_t, &[]);
                }
                SceneKind::Cosmos => {
                    camera_rig::tick_cinematic_cosmos(&mut self.camera, dt, self.time)
                }
                SceneKind::Realm => {
                    let peak = Vec3::new(qga_math::SHASTA_XZ.0, 11.2, qga_math::SHASTA_XZ.1);
                    camera_rig::tick_cinematic_shasta(&mut self.camera, dt, self.time, peak);
                }
                SceneKind::Oam => {
                    let t = self
                        .oam
                        .as_ref()
                        .map(|d| d.visual_time())
                        .unwrap_or(self.time);
                    camera_rig::tick_cinematic_oam(&mut self.camera, dt, t);
                }
                SceneKind::Lab => {}
                // Slow azimuth like the Morris / Vanderplas butterfly clip. No crane.
                SceneKind::Reveal => {
                    if self.camera.mode == CameraMode::Orbit {
                        self.camera.yaw += 0.15 * dt;
                    }
                }
            }
        }

        let rec_due = self.recorder.is_some()
            && self.last_rec.elapsed() >= Duration::from_secs_f32(1.0 / RECORD_FPS as f32);
        let grab = rec_due || self.want_shot;
        let want_title = self.title_acc > 0.4;
        let rec_label = if self.recorder.is_some() {
            "REC"
        } else {
            "live"
        };
        let cam_label = if self.tour {
            "tour"
        } else if self.camera.cinematic {
            "crane"
        } else {
            "free"
        };

        let (title, captured) = {
            let Some(gpu) = self.gpu.as_mut() else {
                return Ok(());
            };
            let Some(renderer) = self.renderer.as_mut() else {
                return Ok(());
            };
            if self.scene == SceneKind::Cosmos && !self.vis.paused {
                if let Some(nb) = self.nbody.as_mut() {
                    nb.step(gpu, 1);
                    let parts = nb.download(gpu)?;
                    let display = convert::display_particles(parts, self.palette);
                    renderer.write_particles(gpu, &display)?;
                }
            }
            if self.scene == SceneKind::Oam && !self.vis.paused {
                if let Some(demo) = self.oam.as_mut() {
                    demo.step(dt);
                    sync_oam(gpu, renderer, demo)?;
                }
            }
            if self.scene == SceneKind::Reveal && !self.vis.paused {
                if let Some(demo) = self.reveal.as_mut() {
                    demo.step(dt);
                    sync_reveal(gpu, renderer, demo)?;
                }
            }
            if self.scene == SceneKind::Cosmos {
                renderer.write_hud(
                    gpu,
                    &build_cosmos_hud(
                        self.preset_open,
                        self.preset_ix,
                        self.view_open,
                        self.view_ix,
                        self.grid,
                        self.tabs_hidden,
                    ),
                )?;
            }
            let captured = renderer.render(gpu, &self.camera, &self.vis, self.time, grab)?;
            let n_bodies = self
                .nbody
                .as_ref()
                .map(|nb| nb.len())
                .unwrap_or_else(|| renderer.particle_count());
            let title = want_title.then(|| {
                if self.scene == SceneKind::Oam {
                    let suffix = self
                        .oam
                        .as_ref()
                        .map(|d| d.title_suffix())
                        .unwrap_or_default();
                    format!(
                        "QGA Engine — oam — {:.0} fps — {} — {} — {}",
                        1.0 / dt.max(1e-4),
                        suffix,
                        rec_label,
                        cam_label
                    )
                } else {
                    format!(
                        "QGA Engine — {} — {} — {:.0} fps — bodies {} — rings {} — {} — {}",
                        self.scene.name(),
                        gpu.adapter_info.name,
                        1.0 / dt.max(1e-4),
                        n_bodies,
                        if self.vis.show_rings { "on" } else { "off" },
                        rec_label,
                        cam_label
                    )
                }
            });
            (title, captured)
        };

        if let Some(title) = title {
            self.title_acc = 0.0;
            if let Some(window) = self.window.as_ref() {
                window.set_title(&title);
            }
        }

        if let Some(frame) = captured {
            if self.want_shot {
                self.want_shot = false;
                match record::save_png(frame.width, frame.height, &frame.bgra) {
                    Ok(p) => log::info!("screenshot {}", p.display()),
                    Err(e) => log::error!("screenshot: {e:#}"),
                }
            }
            if rec_due {
                self.last_rec = Instant::now();
                if let Some(rec) = self.recorder.as_mut() {
                    rec.push_bgra(frame.width, frame.height, &frame.bgra)?;
                }
            }
        }

        if self.launch.frames > 0 && self.frames >= self.launch.frames {
            std::process::exit(0);
        }
        Ok(())
    }

    fn handle_key(&mut self, event: KeyEvent) {
        let down = event.state == ElementState::Pressed;
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };
        match code {
            KeyCode::KeyW => self.keys.w = down,
            KeyCode::KeyA => self.keys.a = down,
            KeyCode::KeyS => self.keys.s = down,
            KeyCode::KeyD => {
                self.keys.d = down;
                if down && !event.repeat && !self.rmb {
                    self.focus_reveal(RevealPanel::D);
                }
            }
            KeyCode::Space => {
                self.keys.space = down;
                if down && !event.repeat && !self.rmb {
                    if self.scene == SceneKind::Cosmos {
                        self.tabs_hidden = !self.tabs_hidden;
                        if self.tabs_hidden {
                            self.preset_open = false;
                            self.view_open = false;
                        }
                        log::info!(
                            "nav tabs {}",
                            if self.tabs_hidden { "hidden" } else { "shown" }
                        );
                    } else {
                        self.vis.paused = !self.vis.paused;
                    }
                }
            }
            KeyCode::ControlLeft | KeyCode::ControlRight => self.keys.ctrl = down,
            KeyCode::Escape if down => std::process::exit(0),
            KeyCode::Digit1 if down => self.switch(SceneKind::Lab),
            KeyCode::Digit2 if down => self.switch(SceneKind::Realm),
            KeyCode::Digit3 if down => self.switch(SceneKind::Cosmos),
            KeyCode::KeyP if down && !event.repeat => {
                if self.scene == SceneKind::Cosmos && self.tabs_hidden {
                    self.tabs_hidden = false;
                }
                self.preset_open = !self.preset_open;
                if self.preset_open {
                    self.view_open = false;
                }
                log::info!(
                    "presets {}",
                    if self.preset_open { "open" } else { "closed" }
                );
            }
            KeyCode::ArrowDown if down && self.view_open => {
                self.view_ix = (self.view_ix + 1) % VIEW_IDS.len();
            }
            KeyCode::ArrowUp if down && self.view_open => {
                self.view_ix = (self.view_ix + VIEW_IDS.len() - 1) % VIEW_IDS.len();
            }
            KeyCode::ArrowDown if down && self.preset_open => {
                self.preset_ix = (self.preset_ix + 1) % PRESET_IDS.len();
            }
            KeyCode::ArrowUp if down && self.preset_open => {
                self.preset_ix = (self.preset_ix + PRESET_IDS.len() - 1) % PRESET_IDS.len();
            }
            KeyCode::Enter | KeyCode::NumpadEnter if down && self.view_open && !event.repeat => {
                self.apply_view();
            }
            KeyCode::Enter | KeyCode::NumpadEnter if down && self.preset_open && !event.repeat => {
                self.apply_eye_preset(PRESET_IDS[self.preset_ix]);
            }
            KeyCode::Digit6 | KeyCode::Numpad6 if down => self.switch(SceneKind::Oam),
            KeyCode::Digit7 | KeyCode::Numpad7 if down => self.focus_reveal(RevealPanel::A),
            KeyCode::Digit8 | KeyCode::Numpad8 if down => self.focus_reveal(RevealPanel::B),
            KeyCode::Digit9 | KeyCode::Numpad9 if down => self.focus_reveal(RevealPanel::C),
            KeyCode::Comma if down && !event.repeat && self.scene == SceneKind::Oam => {
                if let Some(demo) = self.oam.as_mut() {
                    demo.set_kappa(demo.cfg.kappa - 0.01);
                    log::info!("κ = {:.3}", demo.cfg.kappa);
                }
                self.resync_oam();
            }
            KeyCode::Period if down && !event.repeat && self.scene == SceneKind::Oam => {
                if let Some(demo) = self.oam.as_mut() {
                    demo.set_kappa(demo.cfg.kappa + 0.01);
                    log::info!("κ = {:.3}", demo.cfg.kappa);
                }
                self.resync_oam();
            }
            KeyCode::Digit4 | KeyCode::Numpad4 if down && !event.repeat => {
                self.vis.show_rings = !self.vis.show_rings;
                log::info!(
                    "ring layer {}",
                    if self.vis.show_rings { "on" } else { "off" }
                );
            }
            KeyCode::Digit5 | KeyCode::Numpad5 if down && !event.repeat => {
                if self.scene != SceneKind::Cosmos {
                    self.switch(SceneKind::Cosmos);
                }
                self.vis.show_rings = false;
                camera_rig::start_tour(&mut self.camera, &mut self.tour, &mut self.tour_t);
                log::info!("planet tour");
            }
            KeyCode::KeyC if down && !event.repeat => {
                self.camera.cinematic = !self.camera.cinematic;
                if self.camera.cinematic {
                    self.tour = false;
                    if self.scene == SceneKind::Oam {
                        camera_rig::frame_oam_overview(&mut self.camera);
                    }
                }
                log::info!(
                    "cinematic camera {}",
                    if self.camera.cinematic { "on" } else { "off" }
                );
            }
            KeyCode::KeyG if down => self.vis.glow = if self.vis.glow > 0.2 { 0.15 } else { 1.15 },
            KeyCode::KeyR if down => self.switch(self.scene),
            KeyCode::KeyV if down && !event.repeat => {
                if let Err(e) = self.toggle_record() {
                    log::error!("record: {e:#}");
                }
            }
            KeyCode::F12 if down && !event.repeat => {
                self.want_shot = true;
            }
            KeyCode::F11 if down => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.toggle_present_mode();
                }
            }
            KeyCode::BracketLeft if down => self.retune(-1),
            KeyCode::BracketRight if down => self.retune(1),
            _ => {}
        }
    }

    fn toggle_record(&mut self) -> Result<()> {
        if let Some(rec) = self.recorder.take() {
            let n = rec.frames();
            let path = rec.finish()?;
            log::info!("stopped recording after {n} frames → {}", path.display());
            return Ok(());
        }
        let (w, h) = self
            .gpu
            .as_ref()
            .and_then(|g| g.config.as_ref())
            .map(|c| (c.width, c.height))
            .unwrap_or((1920, 1080));
        self.recorder = Some(VideoRecorder::start(w, h)?);
        self.last_rec = Instant::now()
            .checked_sub(Duration::from_secs_f32(1.0 / RECORD_FPS as f32))
            .unwrap_or_else(Instant::now);
        Ok(())
    }

    fn focus_reveal(&mut self, panel: RevealPanel) {
        if self.scene != SceneKind::Reveal {
            self.switch(SceneKind::Reveal);
        }
        if let Some(demo) = self.reveal.as_mut() {
            demo.set_panel(panel);
        }
        if let (Some(gpu), Some(renderer), Some(demo)) = (
            self.gpu.as_mut(),
            self.renderer.as_mut(),
            self.reveal.as_ref(),
        ) {
            if let Err(e) = sync_reveal(gpu, renderer, demo) {
                log::error!("reveal sync: {e:#}");
            }
        }
        log::info!("reveal panel {}", panel.name());
    }

    fn switch(&mut self, scene: SceneKind) {
        self.scene = scene;
        self.time = 0.0;
        apply_camera_for_scene(&mut self.camera, scene);
        self.oam = if scene == SceneKind::Oam {
            Some(make_oam_demo(&self.launch))
        } else {
            None
        };
        let keep_panel = self.reveal.as_ref().map(|d| d.panel);
        self.reveal = if scene == SceneKind::Reveal {
            let mut q = RevealQuad::new();
            if let Some(p) = keep_panel {
                q.set_panel(p);
            }
            Some(q)
        } else {
            None
        };
        self.tour = false;
        self.tour_t = 0.0;
        if let (Some(gpu), Some(renderer)) = (self.gpu.as_mut(), self.renderer.as_mut()) {
            if let Err(e) = load_scene(
                gpu,
                renderer,
                &mut self.nbody,
                scene,
                &self.launch,
                self.oam.as_mut(),
                self.palette,
                self.grid,
            ) {
                log::error!("reload scene: {e:#}");
            }
            if let Some(demo) = self.reveal.as_mut() {
                demo.step(0.0);
                if let Err(e) = sync_reveal(gpu, renderer, demo) {
                    log::error!("reveal sync: {e:#}");
                }
            }
        }
        log::info!("scene {}", scene.name());
        if scene == SceneKind::Oam {
            self.vis.glow = 0.88;
            log::info!("{}", analog_legend());
        } else if scene == SceneKind::Reveal {
            self.vis.glow = 0.72;
            self.vis.show_rings = true;
        } else if self.vis.glow < 0.5 {
            self.vis.glow = 1.15;
        }
    }

    fn ndc_cursor(&self) -> Option<(f32, f32)> {
        let (w, h) = self
            .gpu
            .as_ref()?
            .config
            .as_ref()
            .map(|c| (c.width as f32, c.height as f32))?;
        if w < 1.0 || h < 1.0 {
            return None;
        }
        Some((
            self.cursor[0] / w * 2.0 - 1.0,
            1.0 - self.cursor[1] / h * 2.0,
        ))
    }

    fn hit_presets(&mut self) -> bool {
        if self.scene != SceneKind::Cosmos || self.tabs_hidden {
            return false;
        }
        let Some((x, y)) = self.ndc_cursor() else {
            return false;
        };
        if let Some(h) = view_hit(x, y, self.view_open) {
            match h {
                0 => {
                    self.view_open = !self.view_open;
                    if self.view_open {
                        self.preset_open = false;
                    }
                    log::info!("view {}", if self.view_open { "open" } else { "closed" });
                }
                i if i > 0 => {
                    self.view_ix = (i as usize) - 1;
                    self.apply_view();
                }
                _ => {}
            }
            return true;
        }
        match preset_hit(x, y, self.preset_open) {
            Some(0) => {
                self.preset_open = !self.preset_open;
                if self.preset_open {
                    self.view_open = false;
                }
                true
            }
            Some(i) if i > 0 => {
                let ix = (i as usize) - 1;
                self.preset_ix = ix;
                self.apply_eye_preset(PRESET_IDS[ix]);
                true
            }
            _ => false,
        }
    }

    fn apply_view(&mut self) {
        self.grid = self.view_ix == 1;
        log::info!(
            "view {}",
            if self.grid { "3x3 grid" } else { "default" }
        );
    }

    fn apply_eye_preset(&mut self, id: &str) {
        match palette_for_preset(id) {
            Some(0) => {
                self.palette = 0;
                log::info!("preset cluster (kernel colors)");
            }
            Some(p) => {
                // Colors only — same mass, same rings, iris stroma remap.
                self.palette = p;
                log::info!("preset {id} (eye colors, geometry held)");
            }
            None => log::warn!("unknown preset {id}"),
        }
        if let (Some(gpu), Some(renderer), Some(nb)) =
            (self.gpu.as_mut(), self.renderer.as_mut(), self.nbody.as_mut())
        {
            if let Ok(parts) = nb.download(gpu) {
                let display = convert::display_particles(parts, self.palette);
                if let Err(e) = renderer.write_particles(gpu, &display) {
                    log::error!("palette particles: {e:#}");
                }
            }
        }
    }

    fn resync_oam(&mut self) {
        if let (Some(gpu), Some(renderer), Some(demo)) =
            (self.gpu.as_mut(), self.renderer.as_mut(), self.oam.as_mut())
        {
            if let Err(e) = sync_oam(gpu, renderer, demo) {
                log::error!("oam sync: {e:#}");
            }
        }
    }

    fn retune(&mut self, dir: i32) {
        match self.scene {
            SceneKind::Lab | SceneKind::Realm => {
                let cur = self.launch.fibers.unwrap_or(match self.scene {
                    SceneKind::Lab => self.profile.lab_fibers,
                    _ => self.profile.realm_fibers,
                });
                let next = if dir > 0 {
                    (cur * 2).min(2048)
                } else {
                    (cur / 2).max(16)
                };
                self.launch.fibers = Some(next);
                self.switch(self.scene);
            }
            SceneKind::Cosmos => {
                let cur = self
                    .launch
                    .particles
                    .unwrap_or(self.profile.cosmos_particles);
                let next = if dir > 0 {
                    (cur * 2).min(self.profile.cosmos_particles_max)
                } else {
                    (cur / 2).max(1024)
                };
                self.launch.particles = Some(next);
                self.switch(self.scene);
            }
            SceneKind::Oam => {
                if let Some(demo) = self.oam.as_mut() {
                    let next = demo.cfg.ell + dir;
                    demo.set_ell(next);
                    log::info!("ℓ = {:+}", demo.cfg.ell);
                }
                self.resync_oam();
            }
            SceneKind::Reveal => {}
        }
    }
}

impl ApplicationHandler for EngineApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.boot(event_loop) {
                log::error!("boot failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(rec) = self.recorder.take() {
                    if let Err(e) = rec.finish() {
                        log::error!("finalize recording: {e:#}");
                    }
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size.width, size.height);
                    self.camera.aspect = size.width as f32 / size.height.max(1) as f32;
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.tick() {
                    log::warn!("frame: {e:#}");
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        if down && self.hit_presets() {
                            // HUD ate the click.
                        } else {
                            self.lmb = down;
                            if down {
                                self.camera.cinematic = false;
                                self.tour = false;
                            }
                        }
                    }
                    MouseButton::Right => {
                        self.rmb = down;
                        if down {
                            self.camera.cinematic = false;
                            self.tour = false;
                            self.camera.mode = CameraMode::Fly;
                            self.camera.eye = self.camera.eye();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = [position.x as f32, position.y as f32];
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.05,
                };
                self.camera.cinematic = false;
                self.tour = false;
                self.camera.zoom(y);
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if self.lmb || self.rmb {
                self.camera.orbit_delta(delta.0 as f32, -delta.1 as f32);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}

fn apply_camera_for_scene(cam: &mut Camera, scene: SceneKind) {
    match scene {
        SceneKind::Lab => {
            *cam = Camera::orbit(Vec3::ZERO, 7.5);
        }
        SceneKind::Realm => {
            *cam = Camera::orbit(Vec3::new(-5.8, 2.4, 6.4), 6.5);
            cam.pitch = 0.18;
            cam.yaw = 2.4;
            cam.far = 400.0;
            cam.mode = CameraMode::Orbit;
            cam.cinematic = true;
        }
        SceneKind::Cosmos => {
            *cam = Camera::orbit(Vec3::ZERO, 12.5);
            cam.pitch = 0.95;
            cam.far = 800.0;
            cam.cinematic = true;
        }
        SceneKind::Oam => {
            *cam = Camera::orbit(Vec3::ZERO, camera_rig::OAM_OVERVIEW_DISTANCE);
            cam.yaw = 0.65;
            camera_rig::frame_oam_overview(cam);
        }
        SceneKind::Reveal => {
            *cam = Camera::orbit(Vec3::ZERO, 18.0);
            cam.pitch = 30.0_f32.to_radians();
            cam.yaw = 0.0;
            cam.far = 120.0;
            cam.cinematic = false;
        }
    }
}

fn make_oam_demo(launch: &Launch) -> OamDemo {
    let mut cfg = OamConfig::default();
    if let Some(ell) = launch.ell {
        cfg.ell = ell;
    }
    if let Some(kappa) = launch.kappa {
        cfg.kappa = kappa.clamp(0.80, 0.90);
    }
    if let Some(n) = launch.fibers {
        cfg.n_fibers = n.clamp(16, 512);
    }
    if let Some(n) = launch.particles {
        cfg.n_motes = n.clamp(4096, 262_144);
    }
    OamDemo::new(cfg)
}

fn sync_oam(gpu: &GpuContext, renderer: &mut Renderer, demo: &mut OamDemo) -> Result<()> {
    renderer.write_live_fibers(gpu, &convert::gpu_fibers(demo.fibers()), 0.036)?;
    renderer.upload_hubs(gpu, &[])?;
    renderer.write_particles(gpu, &convert::gpu_particles(&demo.particles()))?;
    renderer.write_hud(gpu, &build_oam_hud(&demo.hud()))?;
    Ok(())
}

fn sync_reveal(gpu: &GpuContext, renderer: &mut Renderer, demo: &RevealQuad) -> Result<()> {
    renderer.write_particles(gpu, &convert::gpu_particles(&demo.particles()))?;
    renderer.write_live_fibers(gpu, &convert::gpu_fibers(&demo.fibers()), 0.022)?;
    renderer.upload_hubs(gpu, &convert::gpu_hubs_from_tuples(&demo.hubs()))?;
    renderer.write_hud(gpu, &build_reveal_hud(demo.panel))?;
    Ok(())
}

fn clear_uploads(gpu: &GpuContext, renderer: &mut Renderer) -> Result<()> {
    renderer.write_live_fibers(gpu, &[], 0.04)?;
    renderer.retain_static_fibers(gpu, &[], 0.04)?;
    renderer.upload_hubs(gpu, &[])?;
    renderer.write_particles(gpu, &[])?;
    renderer.update_faces(gpu, &[]);
    renderer.update_line_segments(gpu, &[], LineStyle::black_hairline());
    renderer.update_orb_instances(gpu, &[GpuOrbInstance::identity()]);
    renderer.write_hud(gpu, &[])?;
    Ok(())
}

fn load_scene(
    gpu: &GpuContext,
    renderer: &mut Renderer,
    nbody: &mut Option<NbodyGpu>,
    scene: SceneKind,
    launch: &Launch,
    oam: Option<&mut OamDemo>,
    palette: u32,
    _grid: bool,
) -> Result<()> {
    let hw = HardwareProfile::THIS_BOX;
    *nbody = None;
    clear_uploads(gpu, renderer)?;
    match scene {
        SceneKind::Lab => {
            let n_fibers = launch.fibers.unwrap_or(hw.lab_fibers);
            let fibers = sample_fiber_family(
                n_fibers as usize,
                hw.lab_points as usize,
                (0.15, 1.35),
                2.0,
                HopfConvention::Kingdom,
            );
            renderer.write_live_fibers(gpu, &convert::gpu_fibers(&fibers), hw.tube_radius_lab)?;
            log::info!(
                "lab: {n_fibers} fibers × {} pts (Kingdom Hopf)",
                hw.lab_points
            );
        }
        SceneKind::Realm => {
            let cfg = RealmConfig {
                n_fibers: launch.fibers.unwrap_or(hw.realm_fibers),
                n_points: hw.realm_points,
                terrain: hw.realm_terrain,
                ..RealmConfig::default()
            };
            let world = generate_realm(cfg);
            renderer.update_faces(gpu, &convert::terrain_faces(&world.heightmap));
            renderer.write_live_fibers(
                gpu,
                &convert::gpu_fibers(&world.fibers),
                hw.tube_radius_realm,
            )?;
            let hm = &world.heightmap;
            let mut hubs = convert::sanctuary_hubs(&world.sanctuaries, |x, z| hm.sample(x, z));
            hubs.extend(convert::tree_hubs(&world.trees));
            renderer.upload_hubs(gpu, &hubs)?;
            log::info!(
                "realm: Shasta peak {:.1}, {} sequoias, {} sanctuaries, {} fibers, {}² terrain",
                world.peak.y,
                world.trees.len(),
                world.sanctuaries.len(),
                world.fibers.len(),
                world.heightmap.n
            );
        }
        SceneKind::Cosmos => {
            let n = quantize_nbody(launch.particles.unwrap_or(hw.cosmos_particles));
            let mut neb = NebulaConfig {
                n,
                ..NebulaConfig::default()
            };
            let (particles, sim) = if let Some(path) = launch.species.as_ref() {
                let spec = load_species_file(path)?;
                if let Some(k) = launch.kappa {
                    neb.kappa = k;
                } else {
                    neb.kappa = spec.kappa;
                }
                let mut w = spec.weights;
                if let (Some(avatars), Some(host)) = (launch.avatars.as_ref(), launch.host.as_ref())
                {
                    w = load_host_weights(avatars, host)?;
                    log::info!("species weights from {host} in {}", avatars.display());
                }
                let world_r0 = neb.outer * 0.78 / 6.0_f32.powf(2.0 / 3.0) * spec.r0;
                let particles = spawn_species_disk(neb, world_r0, spec.ell, w);
                let sim = SimParams {
                    n,
                    dt: 0.007,
                    g: neb.g,
                    eps2: neb.softening * neb.softening,
                    kappa: neb.kappa,
                    pad: [world_r0, 0.10, 1.0],
                };
                log::info!(
                    "cosmos species: r0={world_r0:.2} kappa={} w={w:?}",
                    neb.kappa
                );
                (particles, sim)
            } else {
                let particles = spawn_nebula(neb);
                let sim = SimParams {
                    n,
                    dt: 0.007,
                    g: neb.g,
                    eps2: neb.softening * neb.softening,
                    kappa: launch.kappa.unwrap_or(neb.kappa),
                    pad: [0.0; 3],
                };
                (particles, sim)
            };
            let gpu_parts = convert::gpu_particles(&particles);
            *nbody = Some(NbodyGpu::new(gpu, &gpu_parts, sim)?);
            renderer.write_particles(gpu, &convert::display_particles(&gpu_parts, palette))?;
            let sky = sample_fiber_family(48, 96, (0.2, 1.1), 18.0, HopfConvention::Kingdom);
            renderer.write_live_fibers(gpu, &convert::gpu_fibers(&sky), 0.07)?;
            log::info!("cosmos: {n} bodies + sky fibers");
        }
        SceneKind::Oam => {
            if let Some(demo) = oam {
                sync_oam(gpu, renderer, demo)?;
                log::info!("{}", analog_legend());
                log::info!(
                    "oam: {} motes={} fibers={}",
                    demo.title_suffix(),
                    demo.mote_count(),
                    renderer.fiber_count()
                );
            }
        }
        SceneKind::Reveal => {
            log::info!("reveal: full-page Lorenz A/B/C/D (keys 7 8 9 D) — visualizer, not a road");
        }
    }
    Ok(())
}

struct SpeciesFile {
    r0: f32,
    kappa: f32,
    ell: [u8; 6],
    weights: [f32; 6],
    ids: [String; 6],
    colors: [String; 6],
    sizes: [f32; 6],
}

fn load_species_file(path: &std::path::Path) -> Result<SpeciesFile> {
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| path.display().to_string())?,
    )?;
    let types = v["types"]
        .as_array()
        .context("species file missing types[]")?;
    anyhow::ensure!(types.len() == 6, "need six species types");
    let mut ell = [0u8; 6];
    let mut weights = [0f32; 6];
    let mut ids: [String; 6] = std::array::from_fn(|_| String::new());
    let mut colors: [String; 6] = std::array::from_fn(|_| String::new());
    let mut sizes = [1.4f32; 6];
    for k in 0..6 {
        let t = &types[k];
        ids[k] = t["id"].as_str().unwrap_or("x").to_string();
        colors[k] = t["color"].as_str().unwrap_or("#ffffff").to_string();
        ell[k] = t["ell"].as_u64().unwrap_or((6 - k) as u64) as u8;
        weights[k] = t["w"].as_f64().unwrap_or(0.0) as f32;
        sizes[k] = t["size"].as_f64().unwrap_or((2.0 + k as f64 * 0.4).sqrt()) as f32;
    }
    Ok(SpeciesFile {
        r0: v["r0"].as_f64().unwrap_or(1.0) as f32,
        kappa: v["kappa"].as_f64().unwrap_or(0.85) as f32,
        ell,
        weights,
        ids,
        colors,
        sizes,
    })
}

fn load_host_weights(path: &std::path::Path, host: &str) -> Result<[f32; 6]> {
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| path.display().to_string())?,
    )?;
    let avatars = v["avatars"]
        .as_array()
        .context("avatars.json missing avatars[]")?;
    let ids = [
        "hoarder",
        "corrector",
        "flower",
        "adapter",
        "tit_for_tat",
        "sloth",
    ];
    for a in avatars {
        if a["host"].as_str() == Some(host) {
            let wts = &a["weights"];
            let mut w = [0f32; 6];
            for k in 0..6 {
                w[k] = wts[ids[k]].as_f64().unwrap_or(0.0) as f32;
            }
            return Ok(w);
        }
    }
    anyhow::bail!("host {host} not in {}", path.display())
}

fn dump_species_snapshot(
    gpu: &qga_gpu::GpuContext,
    nbody: &mut NbodyGpu,
    launch: &Launch,
    path: &std::path::Path,
) -> Result<()> {
    let gpu_parts = nbody.download(gpu)?.to_vec();
    let spec = match launch.species.as_ref() {
        Some(p) => load_species_file(p)?,
        None => anyhow::bail!("--dump-species requires --species"),
    };
    let neb_outer = 13.5_f32;
    let world_r0 = neb_outer * 0.78 / 6.0_f32.powf(2.0 / 3.0) * spec.r0;
    let r_max = ring_radius(world_r0, 1) * 1.15;
    const KEEP: usize = 120;
    let mut buckets: [Vec<serde_json::Value>; 6] = std::array::from_fn(|_| Vec::new());
    let mut hist = [0u32; 6];
    for p in &gpu_parts {
        if p.mass > 1.5 || p.pad < 9.5 {
            continue;
        }
        let k = (p.pad - SPECIES_PAD_BASE).round().clamp(0.0, 5.0) as usize;
        hist[k] += 1;
        let rho = (p.pos[0] * p.pos[0] + p.pos[1] * p.pos[1]).sqrt();
        let ri = ring_radius(world_r0, spec.ell[k]);
        if buckets[k].len() < KEEP * 4 && ((rho - ri).abs() < 0.55 * ri || buckets[k].len() < KEEP)
        {
            buckets[k].push(serde_json::json!({
                "x": p.pos[0],
                "y": p.pos[1],
                "z": p.pos[2],
                "k": k,
                "size": spec.sizes[k],
                "rgb": spec.colors[k],
            }));
        }
    }
    let mut parts = Vec::new();
    for k in 0..6 {
        let step = (buckets[k].len() / KEEP).max(1);
        let mut kept = 0usize;
        for (i, p) in buckets[k].iter().enumerate() {
            if i % step == 0 && kept < KEEP {
                parts.push(p.clone());
                kept += 1;
            }
        }
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let types: Vec<serde_json::Value> = spec
        .ids
        .iter()
        .zip(spec.colors.iter())
        .enumerate()
        .map(|(k, (id, col))| {
            serde_json::json!({"id": id, "color": col, "ell": spec.ell[k], "size": spec.sizes[k]})
        })
        .collect();
    let out = serde_json::json!({
        "host": launch.host,
        "engine": "qga-engine cosmos",
        "n_sim": gpu_parts.len(),
        "n": parts.len(),
        "rMax": r_max,
        "world_r0": world_r0,
        "hist": hist,
        "types": types,
        "parts": parts,
    });
    std::fs::write(path, serde_json::to_string_pretty(&out)? + "\n")?;
    log::info!(
        "species dump {}  hist={hist:?}  kept={}",
        path.display(),
        parts.len()
    );
    Ok(())
}
