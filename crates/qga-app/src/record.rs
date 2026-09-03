//! In-engine capture: PNG stills and NVENC MP4 via ffmpeg.
//!
//! GNOME's portal recorder composites the Vulkan window through Mutter and
//! often drops frames. Grabbing BGRA from the GPU and encoding with the 4090's
//! NVENC session gives a real-time, smooth file.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const RECORD_FPS: u32 = 60;

pub fn captures_dir() -> Result<PathBuf> {
    let dir = PathBuf::from("captures");
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    Ok(dir)
}

fn stamp() -> String {
    Command::new("date")
        .args(["+%Y%m%d-%H%M%S"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("{t}")
        })
}

/// Write one tightly packed BGRA frame as a PNG via ffmpeg.
pub fn save_png(width: u32, height: u32, bgra: &[u8]) -> Result<PathBuf> {
    let path = captures_dir()?.join(format!("qga-{}.png", stamp()));
    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "-s",
            &format!("{width}x{height}"),
            "-i",
            "pipe:0",
            "-frames:v",
            "1",
            path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn ffmpeg for PNG (is ffmpeg installed?)")?;
    {
        let mut stdin = child.stdin.take().context("ffmpeg stdin")?;
        stdin.write_all(bgra).context("write PNG frame")?;
    }
    let out = child.wait_with_output().context("ffmpeg png wait")?;
    if !out.status.success() {
        bail!(
            "ffmpeg png failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(path)
}

pub struct VideoRecorder {
    child: Child,
    stdin: Option<ChildStdin>,
    path: PathBuf,
    width: u32,
    height: u32,
    frames: u32,
}

impl VideoRecorder {
    pub fn start(width: u32, height: u32) -> Result<Self> {
        let path = captures_dir()?.join(format!("qga-{}.mp4", stamp()));
        let (child, stdin) = spawn_encoder(width, height, &path, true)
            .or_else(|e| {
                log::warn!("h264_nvenc unavailable ({e:#}); falling back to libx264");
                spawn_encoder(width, height, &path, false)
            })
            .context("start ffmpeg encoder")?;
        log::info!(
            "recording {} ({}x{} @ {} fps)",
            path.display(),
            width,
            height,
            RECORD_FPS
        );
        Ok(Self {
            child,
            stdin: Some(stdin),
            path,
            width,
            height,
            frames: 0,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn push_bgra(&mut self, width: u32, height: u32, bgra: &[u8]) -> Result<()> {
        if width != self.width || height != self.height {
            // Window resized mid-record: skip rather than corrupt the stream.
            return Ok(());
        }
        let stdin = self.stdin.as_mut().context("recorder stdin closed")?;
        stdin.write_all(bgra).context("write mp4 frame")?;
        self.frames += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<PathBuf> {
        drop(self.stdin.take());
        let status = self.child.wait().context("ffmpeg wait")?;
        if !status.success() {
            bail!("ffmpeg exited {status} after {} frames", self.frames);
        }
        log::info!(
            "saved {} ({} frames, {:.1}s)",
            self.path.display(),
            self.frames,
            self.frames as f32 / RECORD_FPS as f32
        );
        Ok(self.path.clone())
    }
}

impl Drop for VideoRecorder {
    fn drop(&mut self) {
        if let Some(stdin) = self.stdin.take() {
            drop(stdin);
            let _ = self.child.wait();
        }
    }
}

fn spawn_encoder(width: u32, height: u32, path: &Path, nvenc: bool) -> Result<(Child, ChildStdin)> {
    let size = format!("{width}x{height}");
    let fps = RECORD_FPS.to_string();
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "bgra",
        "-s",
        &size,
        "-r",
        &fps,
        "-i",
        "pipe:0",
        "-an",
        "-vf",
        "crop=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p",
    ]);
    if nvenc {
        cmd.args([
            "-c:v",
            "h264_nvenc",
            "-preset",
            "p4",
            "-tune",
            "hq",
            "-rc",
            "constqp",
            "-qp",
            "18",
            "-bf",
            "2",
        ]);
    } else {
        cmd.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "18"]);
    }
    cmd.args(["-movflags", "+faststart", path.to_str().unwrap()]);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().context("spawn ffmpeg")?;
    let stdin = child.stdin.take().context("ffmpeg stdin")?;
    Ok((child, stdin))
}
