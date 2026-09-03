# Scenes

Engine-owned scene kinds. `qga-gpu` does not know about lab / realm / cosmos /
oam / reveal. Defaults are `HardwareProfile::THIS_BOX` (RTX 4090 + 3900X).

## lab

Kingdom-convention Hopf fiber family. Default 256 fibers × 192 points, tube
radius 0.045. `--fibers` / `[` `]` retune (16…2048). No terrain, no N-body.

## realm

Mt. Shasta analogue on stereographic ℝ³. `generate_realm`:

- 24 Hurwitz-unit sanctuaries (named hubs)
- ley-line fibers (default 128 × 128)
- 256² topograph heightmap (`shasta_height` + Hopf)
- sequoia grove as hubs

Cinematic `C`: grove among sequoias → reveal the cone (~38 s). RMB+WASD flies.

## cosmos

Tiled all-pairs N-body on the 4090 (`nbody.wgsl`, workgroup 256) plus a sky
ley of 48 × 96 Kingdom fibers.

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
in cells; it is not nine viewports. `5` starts a tour camera (star dwell then
system pull-back). Clump detection is not wired — `tick_tour` is called with
an empty marker list.

`--dump-species` after headless cosmos writes a subsampled snapshot for the
geodesic lab.

## oam

Photonic OAM–flux analog of [arXiv:2607.16520](https://arxiv.org/abs/2607.16520)
(`oam_flux` v0.5-preprint). CPU 16³ twist PDE, LG packet, four flywheel
kicks, 65 536 visual motes, 128 × 96 fibers.

Phases: pump (λt 0→1) → relax (to λt = 2) → hold. HUD plots survival S vs
λt with R and e⁻² guides. Title reports S and golden-quantized ℓ.

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
