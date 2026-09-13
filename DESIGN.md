# QGA Engine — design

A native graphics engine whose **geometry is the QGA book** and whose **look is
the Hopf explorer**. Two end states share one runtime: a fantasy realm you can
walk, and a solar nebula that can collapse into a system.

This is not a Gradio portal and not a Three.js page. Those stay in `kingdom/`
and `flux_hopf_explorer/`. This repo is the real-time GPU track.

The extracted renderer is [`qga_gpu`](https://github.com/qga-lab/qga_gpu).
**This repo owns scenes, math, and simulation.** Renderer claims are
**Software fact**. QGA maps used as world language are **Model**.

## Crate map

```
qga_engine/
├── crates/qga-math     # quaternions, Hopf, Hurwitz, gauge, topographs
├── crates/qga-sim      # realm worldgen, nebula ICs, OAM PDE, Lorenz analog
└── crates/qga-app      # winit window, clap, GPU N-body, scene graph
                        # binary: qga-engine
```

WGSL for N-body lives in `crates/qga-app/src/shaders/nbody.wgsl`. Fiber /
particle / HUD / post shaders live in `qga_gpu`. No runtime Python.

## Sources of truth

| Concern | Upstream | Engine |
|---------|----------|--------|
| Quaternions, classical Hopf, Hurwitz 24 | `flux_hopf_lib` + fixtures `qga-math/tests/fixtures/*_v1.json` + `qga-math` parity tests | `qga-math` (checked Rust port). Pedagogical `qga/lib` is not the runtime SoT. |
| Gauge L/R, flux topographs, Magic Islands | QGA book labs | `qga-math` |
| Kingdom `legacy_portal_map`, stereographic pole, κ / θ_crit / W_g / λ_t / 350/π | `flux_hopf_lib` (Model) | **`qga-sim`** owns the Model numbers (`model.rs`). `qga-math` keeps copies only so path-dep consumers still compile. |
| Tube aesthetic, LOD, flux motes, bloom, void | `flux_hopf_explorer` | [`qga_gpu`](https://github.com/qga-lab/qga_gpu) WGSL |
| World *meaning* (Z-map, flywheels, class group) | QGA book Ch. 3–8 | realm biomes, ley lines, sanctuaries |
| Photonic OAM–flux analog (LG packets, λt=2 survival, golden ℓ) | `oam_flux` v0.5-preprint / arXiv:2607.16520 | `--scene oam` |
| Frame, upload ring, glow, capture | `qga_gpu` | git dep, not in-tree |

Python is an **authoring** language in this ecosystem. The frame loop is Rust +
Vulkan. Do not `sys.path` into sibling repos from the engine. Fiber conversion
is `qga-app::convert` — do **not** enable `qga-gpu`'s optional `qga-math`
feature on this consumer.

## Why Vulkan / wgpu (not CUDA-GL, not WebGPU-in-browser)

This box is Wayland + NVIDIA 580 + Vulkan 1.4 ICD (`libGLX_nvidia.so.0`).
`wgpu` selects the Vulkan backend, talks Wayland WSI through `winit`, and keeps
compute + raster on the same 4090 queue. CUDA 12.6 is installed and is the
right home for later ports of `toe` / `hfb` research kernels; it is the wrong
home for the swapchain.

Count policy is a three-row table (`tiny` / `demo` / `this_box`) plus CLI
overrides. Not adapter discovery. `this_box` is the 4090 row (24 GiB, Ada):

| Resource | Lab | Realm | Cosmos | OAM |
|----------|-----|-------|--------|-----|
| Hopf fibers × points | 256 × 192 | 128 × 128 | 48 × 96 (sky ley) | 128 × 96 |
| Tube radius | 0.045 | 0.038 | 0.07 (sky) | ribbon |
| Terrain | — | 256 × 256 | — | — |
| N-body particles | — | trees as hubs | **262 144** (cap 524 288) | 65 536 LG helix + flux motes |
| CPU jobs | 22-wide rayon | same | IC + download | 16³ twist PDE |

Mailbox is offered by the driver; FIFO is the safe present. `F11` toggles the
hint. Software fact: `qga_gpu` measured empty frames with mailbox on driver 580
in the demo; the engine still exposes the toggle.

## QGA → fantasy realm

The playable map is **stereographic ℝ³ of S³**. Linked Hopf fibers are not
decoration — they are the portal graph (every distinct pair of fibers links
once).

| QGA object | Game object |
|------------|-------------|
| 24 Hurwitz units | Sanctuaries / fast-travel hubs |
| Along-fiber adjacency | Ley lines, rivers, sky-bridges |
| Inter-fiber adjacency | Roads, mountain passes |
| Left multiplication | World rotation / “greater magic” (SO(3) double cover) |
| Right multiplication | Fiber phase / local time / spell clock |
| Flux topograph values | Terrain height + mana density |
| Separator components | Borders, ridges, ward-walls |
| Magic Islands (periodic reduced configs) | Stable biomes / dungeons |
| Class-group composition | Alchemy / faction combining |
| Z → flywheel map | Elemental affinity of materials |
| θ_crit burst | Overchannel / storm events |
| W_g ≈ 111.408 | Long-cycle calendar lock |

v0 draws sanctuaries, ley tubes, Shasta heightmap, and sequoia hubs. Gameplay
systems (inventory, combat, dialogue) wait until the world *looks and moves*
like QGA.

## QGA → solar-system genesis

A Keplerian dusty disk is a **flux flywheel in disguise**: angular momentum is
the fiber phase, flattening is the κ restoring torque toward the equatorial
chart, and bound clumps that survive are Magic-Island analogues.

| Sim ingredient | Role |
|----------------|------|
| Tiled all-pairs N-body (WGSL, workgroup 256) | Gravity on the 4090 |
| Softening ε | Avoids 1/r² blow-ups at merger |
| −κ ẑ torque | Flywheel flattening into a disk |
| Central seed mass | Protostar; bloom scale from mass |
| Hopf-modulated IC swirl | Non-Keplerian seed from S³ |
| Six-species disk (`--species` / `--preset`) | Iris-stroma rings; colors remapped, geometry held |
| θ_crit on local density | “Ignition” marker (visual, v0) |

Default is a 262 144-body nebula (`quantize_nbody` to 256). `--particles`
overrides; `[` / `]` halves or doubles up to 524 288. Planets as labeled
bodies, SPH gas, and radiative transfer are later phases. The `5` tour camera
dwells on the star, then on detected clumps, then pulls back. Clump detection
is a polar-bin overdensity of the snapshot — Software fact, not a Magic-Island
theorem.

## Photonic OAM–flux analog

`--scene oam` is the paper demo: CPU twist-lattice PDE + LG p=0 radial weights
+ flywheel kicks, uploaded each frame as fibers / hubs / particles. Physics
constants (R, e⁻², κ_doc, κ⋆, B(κ), λt = 2, golden-quantized ℓ) match
`oam_flux` v0.5-preprint.

Pump λt ∈ [0, 1], then pure PDE relaxation to λt = 2, then a short hold.
Survival S is plotted against R ≈ 0.1375 and e⁻² ≈ 0.1353. Operational analog,
not a Dirac spinor solver.

## Reveal Lorenz visualizer

`--scene reveal` is a four-panel Lorenz (1963) **visualizer** of reveal
A/B/C/D. It is not a measured road and does not reverse those experiments.
Lorenz σ is a fluid parameter, not alignment σ(Z).

- A — extra rotor on one of two twins (not a quieter header)
- B — chain paint (step index) vs geometry (lobes)
- C — many sites; leftover is the pack mean, not a lab axis
- D — idle: no off-opening heading, obtained stays false

## Runtime architecture

```
qga-app  (winit Wayland window, clap, input)
    │
    ├── qga-sim   realm worldgen / nebula ICs / CPU flux step / OAM–flux / Lorenz
    │       └── qga-math
    └── qga-gpu  (https://github.com/qga-lab/qga_gpu)
            ├── Vulkan device (high-performance adapter, native limits)
            ├── raster: fiber ribbons, particles, hubs, faces, HUD
            └── post: HDR threshold + 9-tap glow + tonemap
    cosmos n-body compute stays in qga-app (ping-pong storage + MAP_READ staging)
```

Call-site conversion (`qga-app::convert`): `Fiber` → `GpuFiber`, `Particle` →
`GpuParticle`. Display palettes rewrite particle hue for cosmos presets
without changing mass or rings.

Frame uniforms are 256-byte aligned. Particle and fiber records are 32 bytes.
N-body uses workgroup tiles of 256 (matches GLSL/CUDA textbook layout, maps
cleanly onto Ada). Software fact: those sizes are `qga_gpu` contract; do not
drift them here.

`qga-app --headless` prints an `engine-proof` line (scene stepped, 32-byte
records, workgroup 256, frame count, profile, convention). It does **not**
print `qga_gpu::UploadStats`. That counter belongs to `qga_gpu`.

## Capture

Software fact. `F12` writes a PNG; `V` pipes 60 fps BGRA to `ffmpeg`
`h264_nvenc` (software fallback if NVENC is missing). Files land in
`captures/`, which is gitignored. GNOME's portal recorder is the wrong tool
for this swapchain.

## Phases

**v0 (this tree)** — window, five scenes, QGA math tests, GPU n-body, fiber
ribbons, realm terrain, photonic OAM–flux analog, Lorenz visualizer, PNG/MP4
capture, bitmap HUD (OAM survival plot, cosmos preset/view tabs), six-species
iris palettes. Optimized defaults for *this* 3900X + 4090.

**v1 (this drop, engine-owned)** — `--headless` prints engine step proof
(scene, 32-byte records, workgroup 256, frame count). `--dump-png` is the
engine still (grab after N headless steps). `UploadStats` remains `qga_gpu`.
`--fibers-json` loads `export_fiber_curves` schema v1. `--profile
tiny|demo|this_box` plus glow scalars (0.55 / 0.85 / 1.15). Cosmos
`--integrator verlet` + `--diag`. HUD names left-multiply (world rotor) vs
right-multiply (ξ₂ / day clock). Clump detection is a polar-bin overdensity of
the snapshot and feeds the `5` tour. Shot list: [docs/VISUALS.md](docs/VISUALS.md).

Leave in `qga_gpu` or later: `UploadStats`, glow/bloom *shader*, tube
extrusion, mesh-shader tubes, explorer LOD, CUDA interop (v3), RPG (v4),
DLSS/FSR, ray-traced glow.

**v2** — character controller, sanctuary fast-travel, separator-as-collision,
Magic-Island biome HUD. Day-cycle and clump-tour are v1; do not list them here.

**v3** — CUDA interop for `flux_hopf_lib.simulation` kernels; labeled
planets; optional DLSS/FSR; ray-traced fiber glow.

**v4** — RPG loop (inventory, elemental Z-map crafting) *or* a publishable
solar-system documentary mode. Same engine.

## Non-goals (v0)

- Replacing `flux_hopf_lib` or the QGA manuscript.
- Shipping a combat system.
- Browser/WebGPU (that is still `flux_hopf_explorer`).
- Owning the Vulkan frame (that is `qga_gpu`).
- Claiming the Z-map or 350/π observations as theorems inside the renderer.
- Printing or asserting `UploadStats` (the demo in `qga_gpu` does that).
