---
status: open
job: water-liquids
date: 2026-10-01
commit: 58c8b71
areas: [world, classes, render]
---

# Water and liquids leftovers: drips, Gemlik's pause, underwater stores and moby pass, the liquid mesh draws, 613, 1848

## 1. Summary

- **G-REN-008 (water-manager leftovers), closed.**
  - **Drips 787 (level 01)**: 751's tick split into its game-order phases (`RippleSim::tick_zones` → the clock →
    the draw registration → `drip_due` → `tick_mist`); the drip moby is spawned by `units::drip::spawn` (the level01
    spawner `0x2ffcd0`) and runs its own update (`0x2ffdc0`, registered as the unit row "787 drip"). The fallback
    path without a moby system (`RippleSim::tick_with`, used by rc-engine `water_render` with `RC_PLAY=0`) keeps the
    old draws (point + two offsets + timer; `CreateMoby` fails there).
  - **Gemlik's ship pause (level 13 managers 1263 / 1393)**: reads the new `Services::vehicle` (0x140940). Nothing
    writes it yet (the flown ships are G-LVL-009), so on the disc today it never pauses.
  - **The amoeboid fall-out rule (05 / 11)** reads `WaterWorld::patch_level(i)` (patch +0x08); the dead
    `creature::Globals::ripple_z` was removed.
  - **Underwater flag**: the test is cleared while the game mode 0x15f5c4 ≠ 0 (cutscenes) as well as for the type-6
    camera; `HeroTeleport` (`cinematic::hero_teleport`) stores `0x167494 = (group == 0x11)` through the new seam
    `WaterWorld::underwater_store` (`UnderwaterStore`), applied once by rc-engine `fog_state` before its test.
  - **Level 08's 327 splash** when Ratchet falls into the liquid (`water::sea::batalia_splash`, second half of
    level08 `0x2da0f0`).
  - `bomb_water::entry` split: its second half is `bomb_water::splash_and_drops(w, size, at)`, shared by the bomb /
    mine (size 2), the drip (size 2) and 327 (size 3).
- **G-REN-026 (liquid mesh draws), closed** as one general path: `water::sea::MeshSet` (meshes + colour rule + 1–2
  passes of texture / ST rule / FIX) loaded per port, drawn by `rc-engine` `sea_render::mesh_prims`. Users: level
  09's 108 lava flows (two passes, FX 0x2c.. 16-frame blend, FIX 0x80 / 0x60) and its 10 grid-textured meshes (extras
  of the existing grid port 2 on level 9 only); new sea ports 8 (854, level 02: seven gated grids with one shared image
  and fog), 9 (293, level 12, 47 two-texture strips), 10 (1418, level 14, 19 two-texture strips), 11 (1848, level 01,
  the reflective overlay). Level 15's `0x29642c` was identified as class 28's glow quad (not a liquid).
- **G-REN-027 (underwater for the draw-callback liquids), closed**: no level has its own underwater code
  (`UnderwaterTest` / `SetWaterLevel` / `UpdateFog` identical on all 19 levels by `rc-trace overlay-diff`); the
  port's test gained `CollLine_Fix`'s moby pass (flags 0x12 test moby triangle meshes: the patch managers' water).
- **G-CLS-005 (Novalis world classes), closed**: 613 water currents (`units::water_current`, unit row "U50 613";
  HeroFields gained `speed`, `jump_lock`, `current_dist`), 1848 (sea port 11). 695 / 1504 were ported by the
  cheap-wins agent (not this job: `units/floating_pushable.rs`, `units/wandering_light.rs`).
- **Plug-and-play**: a new liquid mesh user is a `SeaPort` row plus a table loader (`fs::parse_flow_meshes` 0x40-byte
  records, `fs::parse_mesh_records` 0x20-byte records, `fs::parse_pointer_meshes` pointer tables, `fs::strip_state_fix`
  for a GS packet's FIX); a new store into the underwater flag sets `WaterWorld::underwater_store`; a class that
  needs the vehicle reads `Services::vehicle`.

## 2. Where it lives

- `crates/rc-game/src/moby_update/classes/units/drip.rs`: `spawn` (0x2ffcd0), `update` (0x2ffdc0); coverage table.
- `crates/rc-game/src/moby_update/classes/units/water_current.rs`: `update` (0x2f3120); coverage table.
- `crates/rc-game/src/moby_update/classes/units/mod.rs`: rows "787 drip", "U50 613" (and the doc table rows).
- `crates/rc-game/src/water.rs`: `RippleSim::tick_with` / `tick_zones` / `drip_due` / `tick_mist`, `drip_points`.
- `crates/rc-game/src/water/managers.rs`: `update_751` (phases + drip), `update_gemlik` (pause); module doc
  "Additions 2026-10-01".
- `crates/rc-game/src/water/world.rs`: `WaterWorld::patch_level`, `UnderwaterStore`, `WaterWorld::underwater_store`,
  level 9 loaded as a reference overlay.
- `crates/rc-game/src/water/sea.rs`: ports 8–11, `TwoTexPort`, `gaspar_ref`, `aridia_ref`, `env_overlay_ref`,
  `GridAnim`, `MeshTex`, `MeshSt`, `MeshColour`, `MeshPass`, `MeshSet`, `GridSetData`, `flow_colour`, `flow_offset`,
  `sphere_map_st`, `env_map_st`, `grid_anim`, `gaspar_extras`, `aridia_grids`, `two_tex_set`, `env_overlay_set`,
  `grid_set_update`, `two_tex_update`, `env_overlay_update`, `grid_set_drawn`, `batalia_splash`; module doc "The
  liquid meshes" coverage table.
- `crates/rc-formats/src/sea.rs`: `LiquidMesh`, `parse_flow_meshes`, `parse_mesh_records`, `parse_pointer_meshes`,
  `strip_state_fix`.
- `crates/rc-engine/src/sea_render.rs`: `LevelSea::anim_frames`, `anims`, `grid_prims` / `grid_prims_into`,
  `mesh_prims`, `mesh_set_groups`, `draw` (module doc "The liquid meshes").
- `crates/rc-game/src/fog_zones.rs`: `UnderwaterState::update_with_scene`.
- `crates/rc-engine/src/fog_state.rs`: the store application, the game-mode clear, the moby pass.
- `crates/rc-game/src/moby_update/services.rs`: `Services::vehicle`; `HeroFields::fall_voice_clear`, `speed`,
  `jump_lock`, `current_dist` (and `of` / `apply`).
- `crates/rc-game/src/moby_update/classes/bomb_water.rs`: `splash_and_drops`.
- `crates/rc-game/src/cinematic.rs`: `hero_teleport` (the store).
- `crates/rc-game/src/moby_update/classes/amoeboid.rs`: the fall-out rule's read.

## 3. Behaviours to verify

### B1. 751's tick order and draws (level01 0x2fd0e8)
- **Claim:** per tick: zone loop (masks, `randi(odds)` per active patch, drop = 2 `randf(±4)` draws), the clock, the
  registration, then (zone 0 or 6 only) `drip_timer -= 1`; at < 1: `randi(5)`, `randf(0xbe19999a, 0x3e19999a)` x,
  then y, the drip spawn (with a moby: `rand_angle`, then `randf(0x3b4de32f, 0x3d80adfd)`), then
  `rand_range(300, 0x4b0)` into the timer; then zone 5's foam draws. Without a 787 slot (class not loaded / table
  full) the spawn makes no draws.
- **Setup:** Novalis water data (`LevelWaterData` level 1), camera inside zone 0's cuboid (751 pvar word 0 = cuboid
  2) or zone 6's (cuboid 8); `drip_timer` = 1.
- **Trigger:** one tick of 751 through the scheduler (with a class table holding 787).
- **Expect:** exactly one 787 moby created at one of the five points of 0x1fa650 ((152, 94.5, 71.3), (153, 93.5,
  69.55), (162, 86.5, 68.55), (175, 96.5, 67.8), (175, 100.5, 72.05)) ± 0.15 in x and y, z as stored;
  `info.drips == 1`; the timer in 300..1199; the rng stream advanced by zone draws + 3 + 2 + 1. With the camera in
  zone 1: no drip draws, timer unchanged.
- **Edge cases:** timer starts at 0 (first drip on the first zone-0/6 tick); without a moby system (`tick_with`)
  only 4 draws per drip (point, x, y, timer).
- **Suggested method:** unit test on `RippleSim::tick_zones` / `drip_due` + a scheduler bench (like
  `bomb_water::tests::Bench`) counting draws.

### B2. The drip spawn 0x2ffcd0
- **Claim:** `CreateMoby(787)`; light word = Ratchet's (+0x38); rotation x 0; draw distance 0x40; update distance
  0xff; scale × 0.2; rotation z = `rand_angle`; position = the point; pvar velocity (0, 0, −dt, 0); gravity 15·dt²;
  life 0x78; spin `randf(0x3b4de32f, 0x3d80adfd)` (0.00314..0.0628); `MobyBuildMatrix`.
- **Setup:** bench with class 787 in the table and Ratchet's moby.
- **Expect:** every field above; draws: exactly 2.
- **Suggested method:** unit test.

### B3. The drip update 0x2ffdc0
- **Claim:** every tick vz −= gravity; rotation x += spin; to = position + velocity; `CollLine_Fix(position, to, 2)`:
  no hit → position = to; hit on a non-water face (`surface_id` ≠ 0) → `DeleteMoby`, position not moved; hit on water:
  first time (+0x18 = 0) `PlayClassSound(0, 0, self)`; `RippleDisturb(x, y, 0.5, −0.35, all patches, additive)`;
  life = 0x78, hit = 1, spin = 0, vz = −1.5·dt, gravity = 0.8·dt²; splash 775 size 2 at (x, y, hit z) with alpha 0x70
  and 16 type-35 drops (each `rand_angle`, `randf(0, 3dt)`, `randf(3dt, 6.5dt)`, `rand_range(0x5a, 0x78)`,
  `randi(2)`, then `PartType35Spawn`'s 2 draws); then position = to; while hit: scale × 0.99 (0x3f7d70a4); life
  `FastDecTimer` → `DeleteMoby` at 0.
- **Setup:** `bomb_water::tests::pool` (floor surface 0x21, water surface 0) with a drip spawned above the water.
- **Trigger:** ticks until deleted.
- **Expect:** falls with growing speed (−dt then −dt − 15dt²·k); on crossing the water one sound event index 0 class
  787, one ripple disturbance, one 775 (alpha 0x70, scale 2× class scale), 16 type-35 records; then sinks
  (vz starts −1.5·dt, gravity 0.8·dt²), shrinks 1 % a tick, deleted on hitting the floor or after 120 ticks.
- **Edge cases:** a second water hit (should not happen from below; if the test forces it: the sound is not played
  again but the ripple / splash / drops are); a drip spawned over dry ground: deleted on the first ground hit, no
  splash; life 0 untouched → `FastDecTimer` returns 1 → delete.
- **Suggested method:** bench test with a draw ledger (as the bomb-in-water ledger).

### B4. Gemlik's ship pause (level13 0x309e88 head)
- **Claim:** with `Services::vehicle` = Some(v), v's class 0x45, v's state not 0xfe / 0xfd, and Ratchet's state 0x32,
  the manager does nothing (no state change, no patch write, no mask, no clock, no registration, no draws).
- **Setup:** level 13 water data, a manager moby in state 1, a vehicle moby of class 0x45.
- **Expect:** pause only when all three hold; vehicle None, class ≠ 0x45, state 0xfe / 0xfd, or Ratchet's state ≠
  0x32 → the normal update (z swings by cos, rotation advances unless the group / state exceptions, a drop draw).
- **Suggested method:** unit test on `water::managers::update` with port 6.

### B5. The amoeboid fall-out rule reads the patch level
- **Claim:** with pvar +0x258 = i ≠ 0 and the amoeboid's z below patch i's +0x08 (`WaterWorld::patch_level(i)`: the
  module patch's record z after the init, else the stored record), the amoeboid bursts into goo and is deleted (no
  bolts). No water data → counted as unported ("amoeboid 572: fall-out rule without the level's ripple patch").
- **Setup:** level 05 or 11 water data with the managers' init run; an amoeboid with +0x258 set.
- **Expect:** below → goo burst + delete; above → nothing; index past the patches → counted, no burst.
- **Suggested method:** unit test (classes area).

### B6. Underwater flag: game mode clear and the HeroTeleport store
- **Claim:** `fog_state`: a new `WaterWorld::underwater_store` (tick counter changed) is applied once before the test:
  `HeroGroup` → flag = Ratchet's group == 0x11; `Off` → false. Then the test: forced (`RC_UNDERWATER`), else cleared
  while the type-6 camera is active **or `Services::game_mode` ≠ 0**, else the collision test.
- **Setup:** any level with a `Play`; call `cinematic::hero_teleport` from a class.
- **Expect:** the store recorded with the tick counter; after the frame, the flag equals (group == 0x11) when the camera
  test finds no water face within ±0.75 (it keeps the stored value), else the test's result. During a cutscene (game
  mode 2) the flag is false.
- **Suggested method:** unit test of the store; engine-level check of `fog_state` logic (or QA in game: teleport out of
  water while underwater).

### B7. The underwater test's moby pass (G-REN-027)
- **Claim:** `UnderwaterState::update_with_scene` casts `coll_line_m(mesh, Some(scene), …, flags 0x12)`: world faces and
  moby triangle meshes (two-sided, primitives skipped); a moby water face (surface 0) sets flag = cam.z < h + 0.04
  with h = `SetWaterLevel(hit)`; non-water moby hits are re-cast below (up to 6 casts).
- **Setup:** level 05 (Rilgar) or 11 (Pokitaru) with the managers' mobys in place; camera 0.3 below a manager moby's z
  inside its patch.
- **Expect:** flag on (it was off before this change when the world mesh has no water face there); camera 0.3 above:
  off.
- **Edge cases:** a solid moby (platform) between the camera and the water within ±0.75: re-cast from its hit point.
- **Suggested method:** world test with a level harness (world area).

### B8. Level 08's 327 splash (second half of level08 0x2da0f0)
- **Claim:** on level 8 only, every tick 327 updates: Ratchet z < the liquid z (record +0x08 = 15) and liquid z ≤
  z − disp.z (he crossed it downward this tick): his death-fall voice slot (0x141602, `Damage::voice_slot`) released if
  alive and set to −1 (`HeroFields::fall_voice_clear`); the splash 775 of size 3 alpha 0x70 at (his x, y, 15) and 16
  type-35 drops (same draws as B3); then 16 type-46 rings: `rand_vec(1, 1)` (3 draws) offset from that point, z = 15.05,
  size `randf(0.7, 1)`, spin 2.0 for the first and −2.0 for the others, velocity 0, level 15; when made, the ring's
  timer +0x0a = trunc(scale(`randf(30, 60)`)).
- **Setup:** level 8 water data, 327's moby, Ratchet's block with pos z 14.9 and disp z −0.2 (previous z 15.1).
- **Expect:** one 775, 16 type-35, 16 type-46 (timers 30..59), the voice release call when the slot is alive.
- **Edge cases:** z exactly 15 with previous 15.2 (z < 15 false → nothing); previous z 15.0 (15 ≤ 15 → yes when z <
  15); level ≠ 8 → nothing; no particle system: the draws as with records (2 + 1 per ring).
- **Suggested method:** bench test with draws counted.

### B9. The liquid mesh loaders
- **Claim:** level 9: port 2's `GridData::extras` = [flows: 108 meshes, max 4 strips each, 6600 vertices in all, colour
  `Flow`, passes (Anim{0x2c, 16, 20}, Flow{1, 0}, FIX 0x80) and (Anim, Flow{2, 0.5}, FIX 0x60), before_grid; grid
  meshes: 10 meshes, colour Const(0x00757c8e), pass (Grid, Stored{0.5})]. Level 7 (the same port, other callback):
  no extras. Level 2: port 8 with 7 grids in the order 0x1f3c00, 0x1f3cd0, 0x1f3d30, 0x1f3d90, 0x1f3dd0, 0x1f3e10,
  0x1f3c60 and gates 0, 1, 1, 1, 1, 1, 2; anim {0x28, 0x40, 30}; fog [0, 260080, 255, 0]; RGB (0x0f, 0x0f, 0x19); FIX
  0x80. Level 12: port 9, 47 meshes with normals, FX 0x2c / 0x2d, FIX 0x80 / 0x40, Const(0x80808080). Level 14: port
  10, 19 meshes, FX 0x2e / 0x2f, FIX 0x80 / 0x40. Level 1: port 11, 5 meshes of 148, 49, 148, 148, 89 vertices with
  normals and stored colours, FX 0x28, FIX 0x20.
- **Setup:** `rc_game::water::world::LevelWaterData::load` per level (the existing `tests/world/sea_levels.rs`
  harness).
- **Expect:** the values above; the sea inventory test's `EXPECTED` must gain port 11 on level 1, port 8 on level 2,
  port 9 on level 12, port 10 on level 14 (it fails until updated: intended).
- **Suggested method:** world test (update `sea_levels.rs`'s EXPECTED and add asserts on the new data).

### B10. The new ports' updates
- **Claim:** 854 (0x2ea198): state 0 → run.inited, run.fix 0x80, update distance 0xff, state 1; state 1 registers on
  the after-ties list every tick. 293 (0x2e7208): state 0 → update distance 0xff, state 1; state 1 registers on the
  after-ties list only when pvar +0xc = −1 or the camera is in that cuboid (85 on the disc). 1418 (0x307a80): the same
  without a gate. 1848 (0x30f208): state 0 zeroes the scroll, state 1; state 1 adds dt·0.025 to both lanes (wrap: above 1 → minus 1, below −1 → plus
  1; a lane passes 1 after about 2400 ticks and drops by 1) and registers on list 1.
- **Setup:** `sea_levels.rs`'s `run(level, port, cam, n)` harness.
- **Expect:** the registrations (list and callback), states, scrolls.
- **Edge cases:** 293 with the camera outside cuboid 85 → no registration.
- **Suggested method:** world test.

### B11. The mesh draw rules (rc-engine `sea_render::mesh_prims`, pure functions in `water::sea`)
- **Claim:** `flow_colour(i) = 0x80787070 + ((i·0x89) & 15)·0x20200` (i = 0 → 0x80787070; i = 1: (0x89 & 15) = 9 → 0x808a8270);
  `flow_offset(c) = 1 − (c & 0x7ff)/2048` (c = 0 → 1, c = 2048 → 1, c = 1024 → 0.5); flow ST pass 1 = (s, t + f), pass
  2 = (s, t + 2f + 0.5); `sphere_map_st`: e = 0.45·unit(cam − p), v = −e, d = v·n; d > 0 (positive bits) → e, else v +
  2(n·d − v); ST = (x + 0.5, y + 0.5); `env_map_st`: e = unit(p − cam), n' = unit(−n), r = unit(e − 2(n'·e)n'), r.z
  += 1, l = |r|, ST = 2(r.x/(2l) + 0.5) + scroll.s, 2(r.y/(2l) + 0.5) + scroll.t. A mesh with a sphere is skipped when
  `BSphereView::culled(256, sphere)`; FIX 0x80 passes (and a Grid pass with FIX ≥ 0x61) are opaque with alpha 0x80,
  lower FIX passes are blend effects with vertex alpha = FIX and the FX texture's alpha forced to 0x80.
- **Expect:** the formulas on hand-picked vectors (e.g. sphere map with n = (0, 0, 1), cam straight above p: d < 0 →
  reflected; cam below: d > 0 → e).
- **Suggested method:** unit tests (pure functions).

### B12. 613 water current (level01 0x2f3120)
- **Claim:** first tick: pvar +0x2e = 1, every point's w of path +0x20 = distance to the next (the last to the first) in
  `Services::splines`, update distance 0xff. Then only when Ratchet's group is 0x11 / 0x12, or prev group 0x11 / 0x12,
  or state 0x12, or prev state 0x12: p = nearest path point (`spline::nearest(open, 999, 5, 0)`), q = 2 units further
  (`spline::advance`, end flag); d = xy distance(Ratchet, p); d > radius (+0x28) → nothing. Swimming (group 0x11 /
  0x12): `current_dist` < d → nothing, else `current_dist` = d; not at the end: v += unit(q − p)·7·dt², v.z = 0, |v| >
  top (+0x24) → |v| = top; platform = v (+ the pull when +0x2c ≠ 0: (p − Ratchet) with z 0, at most 0.4·top); at the
  end: |v| decays by 0.5 %; group 0x12 and prev state 0x12 and timer < ticks(3): speed = min(speed, 1.5·dt), momentum
  0. Not swimming but prev group or the group before 0x12, and group 4 / 5: |v| decays 1.5 %, platform = v, jump_lock
  = max(jump_lock, ticks(35)).
- **Setup:** level 1, currents 668 / 669 (paths 26 / 25, top 0.0667, radius 10) or 670 / 671 (paths 32 / 33, top 0.025,
  radius 8); Ratchet placed within the radius in group 0x11.
- **Expect:** v grows by 7/3600 a tick along the path up to the top speed; `HeroFields::platform` = v each tick; the
  nearer current wins when two overlap (`current_dist`); after a jump out (group 4 / 5 with prev group 0x12) the push
  decays and `jump_lock` = 35.
- **Edge cases:** radius exactly d (pushes); path −1 (nothing); first-tick w recomputation (the disc's w are −1);
  Ratchet not in water and no previous-water groups → nothing (and the init still runs).
- **Suggested method:** bench test (classes) with a synthetic path; QA in game.

### B13. HeroFields additions apply exactly
- **Claim:** `HeroFields::of` reads `speed` (0x13f4e4) and `jump_lock` (Swim::jump_lock, 0x13f528), sets `current_dist`
  9999; `apply` writes speed and jump_lock back (bit-exact when untouched), clears `damage.voice_slot` to −1 when
  `fall_voice_clear`.
- **Expect:** a class that writes nothing leaves the hero bit-identical; 613's writes land.
- **Suggested method:** unit test on services.

## 4. Shared code touched (regression risk)

- `services.rs`: `Services::vehicle` (new), `HeroFields` (4 new fields; `of` / `apply` now also copy `speed` and
  `jump_lock` and maybe clear the voice slot) — every class hero write passes through `apply`: check the hero tests
  that use hero writes (bolt crank, flow 679, pickups, teleporter) stay green.
- `units/mod.rs`: two rows (787 drip, U50 613): the census / `LevelPorts` registration tests count unit rows.
- `water.rs`: `RippleSim::tick_with` now calls the phases; the Novalis ripple guards (draw counts per tick, the step
  period) must be unchanged except where a drip moby is spawned (the moby path now draws 2 more per drip).
- `water/managers.rs` `update_751`: the drip now creates a moby: Novalis rand-ledger tests (`trace_results_novalis.md`
  style) may change at drip ticks (intended, closer to the game: the trace shows "787 creation 2").
- `bomb_water.rs`: `entry` split (same calls and order): the bomb / mine water ledgers must stay green.
- `creature.rs`: `Globals::ripple_z` removed; `amoeboid.rs` reads `patch_level`.
- `cinematic.rs` `hero_teleport`: also records the underwater store (every class that teleports Ratchet).
- `fog_zones.rs` / rc-engine `fog_state.rs`: the moby pass and game-mode clear change the underwater flag on levels
  with moby water (05, 07, 11, 12, 13) and in cutscenes (intended).
- `water/sea.rs`: `PORTS` grew to 12 (ports 8–11); `SeaData` gained `GridSet` / `Meshes`; `GridData::extras`;
  `sea_levels.rs` got a catch-all arm (compile fix only) and its EXPECTED inventory needs the new ports.
- `rc-formats/src/sea.rs`: new parsers only.
- `sea_render.rs`: `LevelSea::grid_frames` → `anim_frames` keyed by `GridAnim`; `grid_prims` takes the record and
  module; the grid seas' images are now cached per animation (shared if two ports had the same animation).

## 5. Not ported (don't test as working)

- The Visibomb view's fog keep in the grid draws (0x15f458 = 1): G-REN-031.
- The surface jump's store `0x167494 = 0` (hero `0x2406b0`): G-REN-032 (the seam exists).
- The Hydrodisplacer's raise / lower command (its update `0x309cd0`: +0xbc = 8 / 4 into the pad's linked manager):
  G-WPN-006. The managers receive commands already.
- The vehicle word 0x140940 writers (the flown ships): G-LVL-009 (Gemlik's pause never fires until then).
- 613: the w lane (0x13f44c, the push's yaw change) of the path functions' outputs taken as 0 [L] (G-CLS-005 note).
- 854: the module init's `sb 0x80, 0x3f(moby)` side store into the 854 moby (unread): n/a.

## 6. In-game QA spots (for the user)

- **Drips (787)**: `RC_LEVEL=1 RC_HERO_AT=165,95,62` (the cave pool with the floats 695, water at 61.1; the drips
  fall from (152–175, 86–100, 68–72)). Stand with the camera in the pool's zone: a small drop every 5–20 s falls,
  splashes (spray shell + drops, a drip sound), ripples the pool, then sinks and shrinks.
- **Water currents (613)**: `RC_LEVEL=1 RC_HERO_AT=66,145,34` (path 25, water z 33) or `RC_HERO_AT=270,166,79` (path 32,
  z ≈ 78.5): swim in: Ratchet drifts along the channel, faster up to the top speed; jumping out keeps the drift for a
  moment and a second surface jump is refused for ~0.6 s.
- **Reflective overlay (1848)**: `RC_LEVEL=1 RC_HERO_AT=252,193,96` (within 32 units of (252.8, 193.2, 95.6)): a faint
  (25 %) sliding reflection (FX 40) over the structure's five meshes, sliding with the camera and slowly scrolling.
- **Gaspar lava (09)**: `RC_LEVEL=9 RC_HERO_AT=450,540,55` (flows near (462, 557, 52)) and the grid meshes near
  (377, 422, 25): animated lava rivers (16-frame blend, two scrolling layers at different speeds) and the extra lava
  pools in the grid's texture.
- **Aridia liquids (02)**: `RC_LEVEL=2 RC_HERO_AT=140,140,26` (record 0x1f3c00 at (132.6, 133.5, 24.8)) and the others
  near (44, 205), (54, 267), (140, 215), (178, 258, 4.8), (112, 284), (162.6, 60.5): animated murky liquid that shows
  only while the camera is inside cuboids 46 / 48 / 49 (854's pvar), fogged to (15, 15, 25).
- **Hoven strips (12)**: `RC_LEVEL=12 RC_HERO_AT=250,280,53` (near (249.9, 278.4, 52)): two-layer liquid (FX 0x2c +
  a 50 % sphere-mapped FX 0x2d), only while the camera is in cuboid 85.
- **Oltanis strips (14)**: `RC_LEVEL=14 RC_HERO_AT=231,75,71` (near (231, 75.2, 70)): the same two-texture look with FX
  0x2e / 0x2f.
- **Batalia splash (08)**: `RC_LEVEL=8` walk off an edge into the liquid at z 15 (the 327 grid, e.g. near (212, 233)):
  a big splash (size 3), drops, 16 spreading rings on the surface, the falling scream cut.
- **Underwater with moby water (05)**: `RC_LEVEL=5 RC_HERO_AT=247,171,38` (manager patches at z 37.5): dive; the camera
  under the surface should now tint and fog (it did not before when only the mobys carry the water faces).
