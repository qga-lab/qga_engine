//! Scene cameras on top of qga-gpu's generic orbit/fly camera.

use crate::scene::PlanetMarker;
use glam::Vec3;
use qga_gpu::{Camera, CameraMode};

pub const OAM_OVERVIEW_DISTANCE: f32 = 13.8;
pub const OAM_OVERVIEW_PITCH: f32 = 0.32;

pub fn start_tour(cam: &mut Camera, tour: &mut bool, tour_t: &mut f32) {
    *tour = true;
    *tour_t = 0.0;
    cam.cinematic = false;
    cam.mode = CameraMode::Orbit;
}

/// Visit the star, then clumps by orbital radius, then a system pull-back.
pub fn tick_tour(cam: &mut Camera, dt: f32, tour_t: &mut f32, planets: &[PlanetMarker]) {
    if cam.mode != CameraMode::Orbit {
        return;
    }
    *tour_t += dt;
    cam.yaw += 0.22 * dt;

    let mut stops: Vec<(Vec3, f32, f32)> = Vec::with_capacity(12);
    stops.push((Vec3::ZERO, 7.5, 0.52));
    let mut worlds: Vec<PlanetMarker> = planets.to_vec();
    worlds.sort_by(|a, b| {
        a.pos
            .truncate()
            .length()
            .partial_cmp(&b.pos.truncate().length())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for p in worlds.iter().take(8) {
        let dist = 2.4 + p.radius * 7.5 + p.pos.length() * 0.08;
        stops.push((p.pos, dist.clamp(2.2, 18.0), 0.38));
    }
    stops.push((Vec3::ZERO, 30.0, 0.82));

    const DWELL: f32 = 5.2;
    let n = stops.len();
    let total = n as f32 * DWELL;
    let t = (*tour_t).min(total - 0.001);
    let u = t / DWELL;
    let i = (u.floor() as usize).min(n.saturating_sub(1));
    let j = (i + 1).min(n - 1);
    let mut f = u.fract();
    f = f * f * (3.0 - 2.0 * f);
    let (ta, da, pa) = stops[i];
    let (tb, db, pb) = stops[j];
    cam.target = ta.lerp(tb, f);
    cam.distance = da + (db - da) * f;
    cam.pitch = pa + (pb - pa) * f;
}

pub fn frame_oam_overview(cam: &mut Camera) {
    cam.mode = CameraMode::Orbit;
    cam.cinematic = true;
    cam.target = Vec3::new(0.0, 0.55, 0.0);
    cam.distance = OAM_OVERVIEW_DISTANCE;
    cam.pitch = OAM_OVERVIEW_PITCH;
    cam.yaw = 1.05;
    cam.far = 400.0;
}

pub fn tick_cinematic_oam(cam: &mut Camera, dt: f32, elapsed: f32) {
    if !cam.cinematic || cam.mode != CameraMode::Orbit {
        return;
    }
    cam.yaw += 0.042 * dt;
    let t = elapsed;
    if t < 4.0 {
        cam.distance = OAM_OVERVIEW_DISTANCE;
        cam.pitch = OAM_OVERVIEW_PITCH;
        cam.target = Vec3::new(0.0, 0.55, 0.0);
    } else if t < 12.0 {
        let u = ((t - 4.0) / 8.0).clamp(0.0, 1.0);
        cam.distance = OAM_OVERVIEW_DISTANCE - 1.4 * u;
        cam.pitch = OAM_OVERVIEW_PITCH + 0.10 * u;
        cam.target = Vec3::new(0.0, 0.55 + 1.1 * u, 0.0);
    } else if t < 18.0 {
        let u = ((t - 12.0) / 6.0).clamp(0.0, 1.0);
        let s = u * u * (3.0 - 2.0 * u);
        cam.distance = 12.4 - 3.6 * s;
        cam.pitch = 0.42 - 0.14 * s;
        cam.target = Vec3::new(0.0, 1.65 + 0.7 * s, 0.0);
    } else {
        cam.distance = 8.8;
        cam.pitch = 0.28;
        cam.target = Vec3::new(0.0, 1.8, 0.0);
    }
}

/// Hurricane-time-lapse crane: orbit, pull back, and drop pitch as the disk grows.
pub fn tick_cinematic_cosmos(cam: &mut Camera, dt: f32, elapsed: f32) {
    if !cam.cinematic || cam.mode != CameraMode::Orbit {
        return;
    }
    cam.yaw += 0.072 * dt;
    let t = (elapsed / 48.0).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    cam.distance = 12.5 + 24.0 * s;
    cam.pitch = 0.95 - 0.40 * s;
}

/// Grove among sequoias → reveal the Shasta cone.
pub fn tick_cinematic_shasta(cam: &mut Camera, dt: f32, elapsed: f32, peak: Vec3) {
    if !cam.cinematic || cam.mode != CameraMode::Orbit {
        return;
    }
    cam.yaw += 0.045 * dt;
    let t = (elapsed / 38.0).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    let grove = Vec3::new(-5.8, 2.4, 6.4);
    cam.target = grove.lerp(peak + Vec3::Y * 0.4, s);
    cam.distance = 6.5 + 14.0 * s;
    cam.pitch = 0.18 + 0.22 * s;
}
