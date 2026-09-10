# QGA Engine

Spine: [`qga`](https://github.com/kinaar8340/qga) — manuscript + pedagogical Python  
Shared math: [`flux_hopf_lib`](https://github.com/kinaar8340/flux_hopf_lib)  
Engine: this repo (scenes, Rust math) · [`qga_gpu`](https://github.com/kinaar8340/qga_gpu) (frame)  
This repo: scenes, Rust math port, sims. Not the swapchain / upload path.

Native GPU graphics engine for **Kingdom Come / QGA** worlds, Hopf/flux
solar-system simulation, and the photonic OAM–flux analog of
[arXiv:2607.16520](https://arxiv.org/abs/2607.16520).

**This repo owns geometry meaning and the scenes.** The Vulkan frame lives in
[`qga_gpu`](https://github.com/kinaar8340/qga_gpu). Math is a Rust port of
[`qga`](https://github.com/kinaar8340/qga) / [`flux_hopf_lib`](https://github.com/kinaar8340/flux_hopf_lib),
not a Python call at runtime. The look — bioluminescent fibers, flux motes,
void backdrop — comes from
[`flux_hopf_explorer`](https://github.com/kinaar8340/flux_hopf_explorer).

Renderer claims are **Software fact**. QGA maps (Z-map, Magic Islands) used as
worldgen are **Model** in the HUD, not theorems of this binary. \(350/\pi\) is
a Hypothesis; do not paint it as a lattice theorem. Attack it in [`op5`](https://github.com/kinaar8340/op5).

```
crates/qga-math    quaternions, Hopf, Hurwitz lattice, flux topographs (fixtures are SoT)
crates/qga-sim     realm worldgen, nebula ICs, OAM–flux PDE, Lorenz analog, Model numbers
crates/qga-app     window, input, scenes, GPU N-body  → binary `qga-engine`
```

`qga-gpu` is pinned at `rev = "b9c9994"` (`features = ["winit", "headless", "capture", "glow"]`). Realm / cosmos / oam / reveal stay here. No `v0.1.0` tag yet.

See [DESIGN.md](DESIGN.md) for the QGA → game/sim mapping and [docs/SCENES.md](docs/SCENES.md)
for per-scene notes.

## Hardware targets

| Target | What to run |
|--------|-------------|
| Headless CI | `make headless` (`--profile tiny`, skip if no adapter) |
| Laptop demo | `--profile demo --scene lab` |
| 4090 lab | `--profile this_box` (default) |

### Lab (4090)

| | |
|---|---|
| GPU | NVIDIA GeForce RTX 4090, 24 GiB, driver 580, Vulkan 1.4, SM 8.9 |
| CPU | AMD Ryzen 9 3900X, 12 cores / 24 threads, `-C target-cpu=native` (Zen 2) |
| RAM | 64 GiB |
| OS | Ubuntu, GNOME, **Wayland** |

Vulkan through `wgpu` only. No OpenGL. N-body is a WGSL compute pass in
`qga-app`; fiber generation is rayon on 22 of 24 threads. CUDA 12.6 is present
and reserved for a later research-kernel backend. v0 keeps graphics and gravity
on one Vulkan queue.

Verified on this machine: adapter `NVIDIA GeForce RTX 4090`
(`vendor 0x10de device 0x2684`), Wayland present. Defaults:

| Resource | Lab | Realm | Cosmos | OAM |
|----------|-----|-------|--------|-----|
| Hopf fibers × points | 256 × 192 | 128 × 128 | 48 × 96 (sky ley) | 128 × 96 |
| Terrain | — | 256 × 256 | — | — |
| Particles | — | motes / trees as hubs | **262 144** (cap 524 288) | 65 536 LG motes |
| Extra | — | sanctuaries + sequoias | tiled all-pairs WGSL | 16³ twist PDE |

Particle counts are quantized to a multiple of the n-body workgroup (256).
Profile glow: tiny 0.55 / demo 0.85 / this_box 1.15.

## Build

```bash
make check          # cargo check --workspace && cargo test --workspace
make test           # same as check
make headless       # 8 offscreen cosmos frames, --profile tiny; prints engine-proof
make stills         # five tiny --dump-png into captures/ (gitignored; CI proof, not the reel)
make realm          # windowed default scene
```

```bash
cargo check --workspace
cargo test --workspace
cargo run -p qga-app --release -- --scene realm
cargo run -p qga-app --release -- --scene cosmos --particles 65536
cargo run -p qga-app --release -- --scene cosmos --preset hazel
cargo run -p qga-app --release -- --scene lab --fibers 512
cargo run -p qga-app --release -- --scene oam --ell 3 --kappa 0.85
cargo run -p qga-app --release -- --scene reveal
```

Headless adapter probe (no window):

```bash
cargo run -p qga-app --release -- --headless --frames 8 --scene cosmos --profile tiny
cargo run -p qga-app --release -- --headless --frames 8 --scene cosmos --profile tiny --integrator verlet --diag
cargo run -p qga-app --release -- --headless --frames 32 --scene oam
cargo run -p qga-app --release -- --headless --frames 16 --scene reveal
```

`--preset` / `--species` look under `$QGA_PLAYGROUND` (default
`/home/kinaar/Playground`) for `labs/global-pointer/state/particles.json` and
`avatars.json`. Those files are not in this repo.

```bash
# optional geodesic-lab dump after headless cosmos
cargo run -p qga-app --release -- --headless --frames 32 --scene cosmos \
  --dump-species /tmp/species-dump.json
```

`--dump-species` requires `--scene cosmos`.

## Scenes

| Flag | What you see |
|------|----------------|
| `--scene lab` | Native Hopf-fiber explorer. Default convention: **Kingdom (portal pin)** — `legacy_portal_map`, not Hopf. |
| `--scene realm` | Mt. Shasta analogue: volcanic cone, snow, sequoia grove, ley-line fibers, 24 Hurwitz sanctuaries |
| `--scene cosmos` | Solar-nebula collapse: GPU N-body + midplane spring (Model flattening). Optional six-species iris disk |
| `--scene oam` | Photonic OAM–flux analog of arXiv:2607.16520: LG packet, flywheel kicks, pump→relax at λt=2 |
| `--scene reveal` | Full-page Lorenz analogy of reveal A/B/C/D. Keys `7` `8` `9` `D`. Visualizer, **not** a measured road |

Default scene is `realm`. Window default 1920×1080 (`--width` / `--height`).
`--profile tiny|demo|this_box` selects counts. `--convention classical|kingdom`
overrides the scene default (lab = Kingdom portal pin; else Classical).
`--integrator verlet` and `--diag` are cosmos Software-fact options
(`E-BOUND OMIT PAIR PE`; kernel gravity is still tiled all-pairs).
`--diag-pe` adds host pair PE when n≤8192. `--dump-mp4`, `--tour`, `--clock`
record the reel. `--fibers-json` loads `export_fiber_curves` into lab/realm/cosmos sky.
`--dump-png PATH` grabs one offscreen still after headless steps.
See [docs/VISUALS.md](docs/VISUALS.md) for the demo shot list.

`--scene oam` ports the `oam_flux` pump–relax trial. The title reports mean
survival S against the analog band {R ≈ 0.1375, e⁻² ≈ 0.1353} plus
golden-quantized ℓ. It is an operational analog, not a Dirac spinor solver.

## Controls

| Key | Action |
|-----|--------|
| LMB drag | Orbit |
| RMB + WASD | Fly (realm). Space/Ctrl while RMB: up/down |
| Wheel | Zoom |
| Space | Lab / realm / oam / reveal: pause. Cosmos: hide/show PRESETS and VIEW tabs |
| `1` `2` `3` `6` | Lab / Realm / Cosmos / OAM |
| `7` `8` `9` `D` | Reveal full-page A / B / C / D |
| `4` | Toggle ring layer (cosmos sky fibers / realm ley lines) |
| `P` | Cosmos: open/close **PRESETS** (cluster / brown / blue / hazel / amber / green / grey / chromia / dark). Arrows + Enter, or click |
| VIEW tab | Cosmos: default vs 3×3 grid *labels* (click the tab). Colors only — cluster geometry held |
| `5` | Cosmos tour camera (star dwell → system pull-back) |
| `C` | Toggle cinematic crane (cosmos: orbit + pull-back; realm: grove → Shasta; OAM: zoom to nested Hopf family). LMB/wheel takes over |
| `[` `]` | Fewer / more fibers or particles (OAM: ℓ −/+) |
| `,` `.` | OAM: κ −/+ in the documentary window [0.80, 0.90] |
| `G` | Toggle glow |
| `R` | Reset scene |
| `F11` | Toggle vsync hint (FIFO ↔ mailbox) |
| `F12` | Screenshot PNG → `captures/` |
| `V` | Start/stop 60 fps MP4 (NVENC on the 4090) → `captures/` |
| Esc | Quit |

GNOME's built-in screen recorder composites the Vulkan window through Mutter and
often drops frames. `V` grabs BGRA off the GPU and encodes with `h264_nvenc`,
so playback matches the sim instead of the compositor. `captures/` is gitignored.

`--preset` aliases: `grey` → gray, `heterochromia` → chromia.

## Related repos

| Repo | Role |
|------|------|
| [`qga_gpu`](https://github.com/kinaar8340/qga_gpu) | wgpu/Vulkan renderer. Owns the frame. Pin by git tag. |
| [`qga`](https://github.com/kinaar8340/qga) | Manuscript + pedagogical Python |
| [`flux_hopf_lib`](https://github.com/kinaar8340/flux_hopf_lib) | Shared math SoT |
| [`flux_hopf_explorer`](https://github.com/kinaar8340/flux_hopf_explorer) | Three.js companion (browser) |
| `inner_cone` | Sculpture viewer (Model). Pin `qga_gpu` / `qga-math` by git tag. |

Do not `sys.path` into sibling repos from the engine. Do not default `$QGA_PLAYGROUND` to a personal path.

Geometry libraries are MIT. Several VQC repos are PolyForm Noncommercial plus patent notice US 63/913,110. This repo is MIT.
