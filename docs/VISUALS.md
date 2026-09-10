# UI demo visuals — roadmap

Engine-owned look. Renderer claims stay **Software fact**. QGA maps used as
world language stay **Model**. This is the shot list and the future cut, not
a paper.

Phase 7 is still the gate. Do not open Phase 8, OP1, a third Vulkan front-end,
or a PIC depositor from this document.

## What a stranger should see in five minutes

One binary, five scenes, one 4090 (or `--profile tiny` on CI). The HUD always
names three things: Hopf convention, fibre clock vs world gauge, and Model
vs Software fact.

| Scene | First impression | Proof on screen | Do not imply |
|-------|------------------|-----------------|--------------|
| **lab** | Linked Hopf tubes, cyan→gold, void | HUD: `Kingdom (portal pin)` unless `--convention classical`. `xi2` right-phase clocks the tubes. | Kingdom is not Hopf. It is `legacy_portal_map`. |
| **realm** | Shasta cone, sequoias, 24 sanctuary hubs | Ley ribbons = along-fibre of the 24. Roads = inter-fibre. Names/biomes tagged Model. | OP1 is not closed. The split is a draw of `candidate_adjacency`. |
| **cosmos** | Dusty disk flattening, star bloom | Workgroup-256 N-body. `--diag` K/U*/Uspring/\|Lz\|. Tour `5` visits clumps when found. Clump detection is a polar-bin overdensity of the snapshot. | Not a flux-flywheel theorem. Integrator is Euler or Verlet, Software fact. Do not title the disk a flywheel proof. |
| **oam** | LG doughnut, lattice carpet, S vs λt plot | Analog of arXiv:2607.16520. R and e⁻² guides on the HUD. λt=2 is Model. | Not a Dirac solver. Survival clustering is this port. |
| **reveal** | Four Lorenz panels | Keys 7/8/9/D. Title: visualizer, not a road. | Lorenz σ is not alignment σ(Z). |

## Camera language (already in `camera_rig.rs`)

| Scene | Default crane | Tour / takeover |
|-------|---------------|-----------------|
| lab | Slow orbit; right-phase restamp is the motion | LMB/wheel kills cinematic |
| realm | Grove → cone (~38 s) | RMB+WASD fly. Sanctuaries are hubs, not a fast-travel system yet (v2). |
| cosmos | Hurricane pull-back (~48 s) | `5` = star dwell → clump stops (orbital radius) → system pull-back. Empty clump list used to skip straight to pull-back; now `detect_clumps` fills dwells. |
| oam | Nested Hopf family zoom | `C` frames the carpet |
| reveal | Slow azimuth, no crane | Panel keys only |

Record with `V` (NVENC) or `F12` stills. GNOME portal is the wrong tool.
Headless `--dump-png` grabs the offscreen target. That is an engine still,
not `qga_gpu::UploadStats`.

## Shot list for a demo reel (v1)

Hold `--profile demo` unless the box is the 4090 (`this_box`). 1920×1080,
glow on, mailbox off (FIFO). Claim line stays on the first card of each shot.

1. **Lab, Kingdom pin (8 s).** `--scene lab`. Title card: “Kingdom is legacy_portal_map — not Hopf.” Tubes clock on `xi2`.
2. **Lab, Classical (8 s).** `--convention classical`. Same camera. The family must *look* different. If it does not, the fork is broken.
3. **JSON overlay (6 s).** `--fibers-json` explorer export. Same lab camera. Proves the engine consumes `export_fiber_curves` without Python at runtime.
4. **Realm establishing (20 s).** Crane grove → cone. HUD: ley/road counts, Model tag, fibre clock.
5. **Realm fly (12 s).** RMB through sequoias toward Crownhold. Roads as brown hairlines, ley as live tubes (right-phase). Terrain does not spin — left-multiply is the fibre overlay, not a heightmap rotate.
6. **Cosmos collapse (30 s).** `--scene cosmos --integrator verlet --diag`. Crane pull-back. HUD energy should *oscillate*, not march. If it marches, dt / 1.01 ε² self-cut is wrong.
7. **Cosmos tour (25 s).** Press `5` after clumps appear. Star → 2–8 overdensities → pull-back. Clump detection is a polar-bin overdensity of the snapshot.
8. **Iris palette (10 s).** `--preset hazel` then `P` through brown/blue. Geometry held, hues only.
9. **OAM pump→relax (25 s).** `--scene oam --ell 3 --kappa 0.85`. S curve vs R / e⁻². Title reports golden ℓ.
10. **Reveal A then D (12 s).** Keys `7` then `D`. On-screen: “NOT A ROAD”.

Total ~2.5 minutes. One MP4 per shot in `captures/`, gitignored. Do not
cut a “flux flywheel proof” title over the Verlet disk.

`make stills` (`--profile tiny`) is CI proof, not this reel. Tiny realm
sequoias at 32² blow out. After this branch is on main, shoot:

| Shot | Profile | Why |
|------|---------|-----|
| lab + `--fibers-json` | tiny is fine | fixture path |
| realm ley/roads | demo or this_box | only scale where planting holds |
| cosmos Verlet + `--diag` | demo | energy / L_z readable |
| oam Classical HUD | tiny/demo | convention must be on-screen |
| reveal | tiny | not Hopf; keep it last |

## What moves, and which multiply

Left multiply = SO(3) double cover = world gauge. Right multiply = fibre
clock. The HUD line is the data-flow proof.

| Object | Left (world) | Right (clock) | v1 |
|--------|--------------|---------------|----|
| Lab / cosmos sky / realm Hopf family | Slow Y-rotor restamp from stored S³ | `phase_unit(ξ₂(t))` restamp | yes |
| Realm ley ribbons of the 24 | static (graph on sanctuaries) | static | yes — do not clock the OP1-model graph |
| Realm roads | static line list | static | yes |
| Terrain heightmap | not rotated | — | yes — rotating the mesh would look like a planet spin and steal the “greater magic” read |
| Sanctuary hubs | stay on the stereographic 24 | — | yes |
| Cosmos particles | Newtonian N-body, not a group action | — | yes |
| OAM fibers | analog demo owns its own restamp | — | leave |

Rates (Software fact of this binary): right-phase τ ≈ 40 s, left-spin τ ≈ 180 s.

## Color and light (do not restyle in qga_gpu from here)

Keep the explorer palette. Engine maps:

- Fibers: `color_from_eta` cyan→gold
- Sanctuaries: same, from Hurwitz η
- Ley ribbons: inherit sanctuary color
- Roads: rock brown `(0.42, 0.34, 0.28)` hairline
- Cosmos dust: four-bin hue from `pad∈(0,1]`; species at `pad≥9.5` remapped by `--preset`
- OAM: Fig. 1 cyan/orange/gold
- Reveal: C+ cyan, C− orange, pack gold

Glow is `qga_gpu`. Engine only sets `VisualState.glow` from the profile row
(`tiny` 0.55, `demo` 0.85, `this_box` 1.15) so a 32² sequoia grove does not
white-out. `G` still toggles. Tube radius is the same policy table.

## Future visuals (owned here, after Phase 7)

Priority is “looks and moves like QGA”, not RPG.

**Next (still v1 / early v2), engine-owned**

- Day-cycle *light*: a dim directional from right-phase so Shasta snow reads sunrise. Shader lives in `qga_gpu`; the *angle* is computed here from `right_rotor`. Coordinate a gpu rev before touching uniforms.
- Clump markers as hubs (gold, radius from `Clump.radius`) so the tour has something to look at besides empty space.
- Sanctuary nameplates: bitmap HUD world-projected, Model names.
- `--dump-png` contact sheet of all five scenes in `make stills`.
- Load `export_fiber_curves` *and* keep generated family as a dim underlay (JSON = hero, generated = ghost).

**v2 (playable map), still this repo**

- Character controller on the heightmap. Collision = separator edges (Model).
- Fast-travel: walk into a Hurwitz hub, fade, land on another of the 24.
- Magic-Island biome paint already exists as scores; show it as a HUD heat, not a second terrain.
- Cosmos labeled clumps (“bound 1…n”) after a density threshold that is documented as Software fact.

**Not this repo**

- Mesh-shader tubes, explorer LOD, ray-traced glow, DLSS/FSR → `qga_gpu`
- CUDA interop of `flux_hopf_lib.simulation` → v3
- Inventory / Z-map crafting → v4 RPG, and only if the world already moves
- Esirkepov / Yee / Boris → different scene, parked in `notes/PARKED.md`
- Unifying Kingdom and Classical
- Painting 350/π or W_g as a lattice theorem on the HUD

## Profile matrix for captures

| Profile | Why | Typical command |
|---------|-----|-----------------|
| `tiny` | CI / stills / JSON load proof | `make headless`; `--dump-png` |
| `demo` | Talk reel, laptop | `--profile demo --scene realm` |
| `this_box` | 4090 documentary | defaults, cosmos 262 144 |

Never let CI inherit `this_box`. Headless already prints `engine-proof` with
`profile=`. Tiny realm stills still white-out: `plant_sequoias` density is
tuned for 256², not 32². Reel shots use `demo` or `this_box`.

## Acceptance for a visual change

A still or 8-frame headless dump is not enough. For anything that *moves*:

1. Lab Kingdom vs Classical must not match (fixture already forbids Kingdom from satisfying `hopf_hurwitz_v1`).
2. Right-phase restamp must move lab tubes; left-only restamp must move them *differently* (unit tests in `qga-sim::gauge`).
3. Realm stills must show both ley ribbons and road hairlines, with HUD Model tag.
4. Cosmos `--diag` energy after 8 tiny Verlet steps is finite; tour with clumps nonempty after a disk has run.
5. `--fibers-json` on lab uploads without calling Python.

Then shoot the reel. Then stop. Phase 8 is a spine note, not another pass of bloom.
