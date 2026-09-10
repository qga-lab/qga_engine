# Scenes

Engine-owned scene kinds. `qga-gpu` does not know about lab / realm / cosmos /
oam / reveal. Count policy is `--profile tiny|demo|this_box` (default
`this_box` = RTX 4090 + 3900X). Hopf convention: lab = Kingdom (portal pin);
everything else Classical unless `--convention`.

## lab

Default convention **Kingdom (portal pin)** — `legacy_portal_map`, not Hopf.
The fork is the feature. `this_box` 256 fibers × 192 points, tube radius 0.045.
`--fibers` / `[` `]` retune (16…2048). No terrain, no N-body. HUD names the
convention.

## realm

Mt. Shasta analogue on stereographic ℝ³. `generate_realm`:

- 24 Hurwitz-unit sanctuaries (named hubs)
- ley-line fibers (Hopf family) plus **along-fibre** edges of the 24 as ley ribbons
- **inter-fibre** edges of the same 24 as roads/ridges (line segments). Model, not OP1-closed.
- 256² topograph heightmap (`shasta_height` + Hopf)
- sequoia grove as hubs
- HUD: names + biomes are Model; `xi2` fibre clock (right-multiply) vs left-multiply world gauge

Cinematic `C`: grove among sequoias → reveal the cone (~38 s). RMB+WASD flies.
Hopf-family tubes restamp each frame: right-multiply = fibre clock, left-multiply
= world gauge (HUD). Ley/road graph of the 24 stays static. `--fibers-json`
replaces the Hopf family only.

## cosmos

Tiled all-pairs N-body on the 4090 (`nbody.wgsl`, workgroup 256) plus a sky
ley of 48 × 96 fibers (Classical unless `--convention`). Semi-implicit Euler
is the default integrator; `--integrator verlet` is velocity Verlet with
cached *a* (same symplectic family, one force eval per step). `--diag` reads
back K + U_star + U_spring and |L_z| — a Software-fact diagnostic, not a
flywheel theorem. Host refuses huge √κ Δt by auto-substepping.

Default 262 144 bodies, cap 524 288, `quantize_nbody` to 256. ICs:
`spawn_nebula` (Kepler dusty disk + Hopf swirl + central seed) or
`spawn_species_disk` when `--species` / `--preset` is set.

Six-species iris disk (collarette → limbus) holds mass and Kepler ℓ rings.
`--preset` remaps display hues only:

| id | palette |
|----|---------|
| cluster | kernel colors (0) |
| brown, blue, hazel, amber, green, gray, chromia, dark | 1…8 |

Aliases: `grey`, `heterochromia`. `--preset` also loads
`$QGA_PLAYGROUND/labs/global-pointer/state/particles.json` (and avatars if
`--host` is passed). Those JSON files are not in this repo.

HUD: **PRESETS** and **VIEW** tabs (`P`, or click). Space hides the tabs
(does **not** pause cosmos). VIEW “3×3 GRID” currently draws palette *labels*
in cells; it is not nine viewports. `5` starts a tour camera (star dwell → detected clumps by radius → system
pull-back). Clump detection is a polar-bin overdensity of the snapshot.

`--dump-species` after headless cosmos writes a subsampled snapshot for the
geodesic lab.

## oam

Photonic OAM–flux analog of [arXiv:2607.16520](https://arxiv.org/abs/2607.16520)
(`oam_flux` v0.5-preprint). CPU 16³ twist PDE, LG packet, four flywheel
kicks, 65 536 visual motes, 128 × 96 fibers.

Default Hopf convention is **Classical** unless `--convention`. HUD names it.
Kingdom is `legacy_portal_map` and is not Hopf — lab is the portal pin, not this
scene.

Phases: pump (λt 0→1) → relax (to λt = 2) → hold. HUD plots survival S vs
λt with R and e⁻² guides. Title reports S and golden-quantized ℓ. λt and the
survival guides are Model.

`--ell` (default 3), `--kappa` (documentary window 0.80–0.90; `,` `.` nudge).
`[` `]` change ℓ. `C` frames the nested Hopf family.

This is an operational analog, not a Dirac spinor solver.

## reveal

Four-panel Lorenz (1963) visualizer of reveal A/B/C/D. **Not a measured
road.** Lorenz σ = 10, ρ = 28, β = 8/3; fifty ICs, 720-point trails.

| Key | Panel |
|-----|-------|
| `7` | A — extra rotor on one twin (not a quieter header) |
| `8` | B — chain paint vs lobe geometry |
| `9` | C — pack mean is leftover, not a lab axis |
| `D` | D — idle; obtained stays false |

Slow azimuth while orbiting. No crane.
