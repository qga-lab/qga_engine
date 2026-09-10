# qga_engine

Native GPU scenes, QGA math, and simulation. The Vulkan frame lives in
https://github.com/kinaar8340/qga_gpu. Geometry meaning lives here
(`qga-math` / `qga-sim` / `qga-app`).

## Hard rules
- Do not edit ~/Projects/qga_gpu or ~/Projects/inner_cone in this repo session.
- Do not sys.path or subprocess Python at runtime.
- Do not vendor qga-gpu sources back into crates/. The renderer is a git dep.
- Do not enable qga-gpu's optional `qga-math` feature. Fiber conversion is
  `qga-app::convert`.
- Cosmos N-body compute stays in `qga-app` (`nbody_gpu.rs` + `nbody.wgsl`).
  qga-gpu only draws the particles.
- Claim labels: Theorem / Model / Software fact / Open. Renderer and buffer
  sizes are Software fact. QGA maps used as world language are Model.
- Frame uniforms 256-byte aligned. Particle and fiber records 32 bytes.
  Keep `qga_sim::Particle` in lockstep with `qga_gpu::GpuParticle`.
- Buffer copy/map: 4 / 8. Texture copy rows: 256. Capture uses padded_bpr.
- Vulkan via wgpu only. No OpenGL. CUDA is out of scope for v0.
- `--scene reveal` is a Lorenz visualizer, not a measured road. Do not claim
  it reverses reveal A/B/C/D. Lorenz σ is not alignment σ(Z).
- `--scene oam` is an operational analog of arXiv:2607.16520, not a Dirac
  spinor solver. Survival S clustering with R / e⁻² is Model + Software fact
  of this port, not a theorem.

## Target machine
RTX 4090 24 GiB, driver 580, Vulkan 1.4, Wayland + winit, Ryzen 9 3900X.
`.cargo/config.toml` sets `-C target-cpu=native`. Rayon pool is 22 threads.

## Public surface (v0)
Binary `qga-engine` (`crates/qga-app`).
Scenes: lab, realm, cosmos, oam, reveal.
CLI: `--scene`, `--fibers`, `--particles`, `--ell`, `--kappa`, `--headless`,
`--frames`, `--width`, `--height`, `--species`, `--avatars`, `--host`,
`--dump-species`, `--preset`, `--profile tiny|demo|this_box`, `--convention
classical|kingdom`, `--integrator euler|verlet`, `--diag`, `--fibers-json`,
`--dump-png`, `--dump-mp4`, `--tour`, `--clock`, `--diag-pe`.
`$QGA_PLAYGROUND` (default `/home/kinaar/Playground`) for species/avatar JSON.

## Defaults (HardwareProfile rows)
`this_box`: Lab 256×192, realm 128×128 / 256² terrain, cosmos 262144 (cap
524288, quantized to 256), OAM 128×96 / 65536 motes / 16³ PDE.
`tiny`: CI smoke (cosmos 256, glow 0.55). `demo`: short talk (glow 0.85).
`--profile` selects the row. `this_box` glow 1.15.

## Consumers
inner_cone @ 89a890c and shellscan pin `qga_engine@7e7866b` and
`qga_gpu@b9c9994` (git rev). Do not change record layout without coordinating
with qga_gpu.

qga-gpu git dep is pinned `rev = "b9c9994"` in Cargo.toml. Cargo.lock is
not the pin. Do not float main. No v0.1.0 tag until we choose to cut one.
