---
status: open
job: camera-classes
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [hero, world, classes]
---

# The level camera classes 3 / 1 / 14, the switch's blends, the whole follow-camera tweaks and target modes, the camera moby, 701's look

## 1. Summary

- **Level-class cameras that become current** (`follow_camera/class_cam.rs`): the plumbing for a level camera class
  that takes over the view (its UpdateCam words, the pre hook → choice → switch → update order of
  `UpdateAllCameras` 0x20d620, the switch back), the switch `FUN_0020d110`'s blend choice (`Camera::switch_blend`) and
  the shared D block's words a class reads before writing (`d_word_00`, `d_word_64`). Plug-and-play: a new class adds
  its state to `ClassCam`, its four functions, a row in `CLASS_CAMS` and its `CameraPorts` check.
- **Class 3, the rail / slide camera** (`follow_camera/rail.rs`): 13 records on 01 (mode 3), 05 (mode 1), 08 (modes 0,
  2), 16 (modes 2, 0, 1), 17 (no path). Grinding on 08 / 16, the sinking state 0x31 on 01 / 05.
- **Classes 1 / 14, the fixed and side views** (`follow_camera/cuboid.rs`): Kerwan (03: class 1 #22, class 14 #7, #8,
  #13, #14) and Eudora (04: class 1 #0).
- **The choice** now checks every slot but the current camera's own (two class-3 records hand over at a rail change)
  and reports the winning slot; the follow camera has a +0x7e (class 1's hook sets it).
- **`0x3111d8` whole**: Clank / giant Clank tweaks, the weapon-up leash, state 0x81 with L2/R2, the scripted focus
  moby 0x16735c's auto-yaw (`Camera::set_focus_moby`; writers not ported). Class 18's region test reads the focus moby.
- **Target modes `0x30fb08` / `0x3101c0` whole**: modes 3 (state 0xf), 5 (now also state 0xd), 8 (state 0xc), 9 / 10
  (state 0x14), 11 (state 0x77), 7 → 6 / 1, the inverse look-blend length on 2 → 1 and raise → 4.
- **The camera moby** (class 1007, `follow_camera/camera_moby.rs`): created while the follow camera is current, deleted
  otherwise; its update makes it a collision blocker at the camera when Ratchet runs toward it.
- **701's camera look** (`classes/props.rs` `collapse_look`) through new moby → camera calls
  (`cinematic::follow_turn_toward`, `follow_look_height`, `focus_moby`).
- **`CameraResetBehindHero`** now drops the level-class and Swingshot cameras and stops the blend (0x167370 = 0).

## 2. Where it lives

| what | file::fn | decomp |
|---|---|---|
| ports per level | `follow_camera/level.rs::CameraPorts::from_overlays` (fields `rail`, `fixed`, `side`), `runs_class_cam` | camvtbl rows; L01 0x315dd8.., L03 0x2e8870.., 0x2ebd70.. |
| the choice | `level.rs::Camera::activation_loop`, `current_slot`, `current_release`, `set_current_release` | 0x20d620, 0x20d410 |
| switch | `class_cam.rs::switch_blend`, `class_switch_in`, `follow_switch_in`, `class_frame` | 0x20d110 |
| class 3 | `rail.rs::rail_init`, `rail_update`, `rail_pre`, `map_onto` | 0x315de0, 0x314e98, 0x315358, 0x316030 |
| classes 1 / 14 | `cuboid.rs::fixed_hook`, `fixed_init`, `fixed_update`, `fixed_pre`, `side_hook`, `side_init`, `side_update`, `side_pre` | L03 0x2e8870, 0x2e8628, 0x2e8730, 0x2e8980, 0x2ebd70, 0x2eb978 + 0x2eb7e0, 0x2ebb00 + 0x2ebc58, 0x2ebde8 |
| records | `rc-formats/src/cameras.rs::RailCamera`, `SideView` | gameplay 0x08 pvar blocks |
| tweaks | `follow_camera.rs::update_type0` | 0x3111d8 |
| target modes | `follow_camera.rs::target_mode`, `vertical_target` | 0x30fb08, 0x3101c0 |
| focus moby | `follow_camera.rs::set_focus_moby`, `focus.rs::focus_test` / `focus_moby_id`, `tick.rs` (feeds the moby) | 0x3111d8, L02 0x2fb648 |
| camera moby | `camera_moby.rs::handle_coll_with_hero`, `update`; `tick.rs` (create / delete); `MobySystem::create_moby` / `delete_moby`; `classes/mod.rs` `ClassUpdate::CameraMoby` | 0x20d068, 0x2ba760, 0x2ba7c8 |
| 701 look | `classes/props.rs::collapse_look` | L01 0x2f9000 |

Coverage tables: module docs of `class_cam.rs`, `rail.rs`, `cuboid.rs`, `camera_moby.rs`; docs/plan/player_controller.md
§15 ("Class 3", "Classes 1 and 14", "The hero-state tweaks", "Target modes", "The camera moby", "The classes the port
does not run").

## 3. Behaviours to verify

### B1. `CameraPorts` per level
- **Claim:** `rail` true on 01, 05, 08, 16, 17 (and false elsewhere: no class-3 row); `fixed` true on 03, 04; `side`
  true on 03; the old `region / swing / placed / focus` unchanged.
- **Setup / method:** extend `cameras::tests::every_level_disc`-style or the existing `ported_camera_classes_every_level`
  with the overlays of all 19 levels.

### B2. The choice: slot exclusion and the winning slot
- **Claim:** every slot but the current camera's own slot is checked (0x20d620 compares UpdateCam pointers, not
  classes); `LevelCameras::won_slot` = the winning slot; a later slot needs a higher priority than the best unless the
  best is the current camera releasing (+0x7e ≠ 0).
- **Setup:** synthetic `LevelCameras` (as `level.rs` tests do) with two class-3 records (rails 0 and 1, priority 6,
  kind 7) plus the system records; the hero in camera mode 3 on rail 0, then on rail 1.
- **Expect:** first the rail-0 record becomes current (`class_cam.slot` = its slot); on rail 1 the pre hook sets +0x7e =
  3, the follow camera (prio 5) and then the rail-1 record (prio 6 > 5) win: a direct switch to the rail-1 record
  (no follow camera in between), `blend` unchanged (blend 3: pose copied).
- **Edge:** the existing `unported_class_is_wanted` behaviour for class 3 changes (class 3 is now run): adjust it to a
  class the port does not run (8 or 19).

### B3. `switch_blend` (0x20d110)
- **Claim:** (old +0x7e, new blend) → result: (4, any) → nothing, false; (2, 1) → rates 0.018 both, kind 0, mode 1 (2
  if a blend runs); (2, 5) → kind 2, orbit_len 40, mode 1/2; (2, 3) → mode 1/2 only; (0, 1) → rates 0.018, kind 0, mode
  1/2, false; (0, 5) → orbit 40, mode 1/2, false; (3, 0) / (0, 3) / (0, 6) → true (pose copied), no blend; (5, x≠1,5) →
  true + rates 0.018 (0.01 on level 1) kind 0 mode 1/2; (0, 6) → true + rates; (0, 0) → false.
- **Method:** unit test on `Camera::switch_blend` with `level_cams.level` = 1 and 3.

### B4. Class 3 init (0x315de0 + 0x314e98)
- **Claim:** pose = the previous camera's (rows, position, saved forward = its forward); the camera path's w = the
  chord to the next point (last → first); `RailState`: look_h 1.5 (0.5 in mode 3), rate 0.01, ang_rate 0.01, vel 0,
  cursor (0, 0), mapped_at 1; ang_vel = the follow camera's `raise_a[1]` (D+0x64; class 3's own when coming from
  another class-3 record); +0x36 = 1 only in mode 3 when (camera − feet) · unit(path[seg+1] − path[seg]) > 0 for the
  path segment nearest the feet (range 20, step 1, open).
- **Mode 2 mapping (once per level, +0x34):** each point of path +0x28 → nearest segment on the camera path (20,
  step 5, open; a miss keeps the previous point's segment, 0 at first), each point of +0x2c → on the record's grind
  path; per point: old_w = its w; len = k × w[(prev_seg + i) mod count] with k = seg(i) − seg(prev) (+count if < 0)
  (the game's loop index never moves: this exact quirk).
- **Setup:** level 08 record #2 (mode 2: path 119, rail 6, maps 44 / 43) and #0 (mode 0), level 01 record #0
  (mode 3); or synthetic splines.
- **Method:** unit tests on `map_onto` (synthetic) + a level test on 08 asserting the mapping's segments are monotone
  along rail 6.

### B5. Class 3 update (0x315358), per mode
- **Mode 0 claim:** target = path[seg] + setlen(path[seg+1] − path[seg], path[seg].w · t / rail[seg].w) with the hero's
  rail cursor (seg, t); position springs per axis (`Cam_InterpValues` k 0.01, d 0.175, limit 0); look = the point
  `+0x00` (0.5 on 08) along the hero's rail (closed) + up_s · look_h; D+0x80 = 1 along the camera path from the hero's
  cursor (open).
- **Mode 2 claim:** the mapped interval [seg(prev), seg(i)) on +0x2c holding the hero's rail segment (wrapping when
  seg(i) < seg(prev)); look-ahead = +0x2c[prev].old_w (0 → pvar +0x00); arc = Σ rail w from +0x2c[prev].seg to the hero's
  segment (+ t); target = +0x28[i].len · arc / +0x2c[i].len along the camera path from +0x28[prev].seg (closed); the
  rate D+0x10 steps by 0.00015 toward +0x28[prev].old_w · 0.01 (≤ 0 → 0.01) and is the position spring's k; the
  angle rate's target is the same.
- **Modes 1 / 3 claim:** rail < 0: nearest path point to the feet (20, 1, open), then +0x38 (×−4 when +0x36 = 1) along
  the path (closed); rail ≥ 0: +0x38 along the hero's rail (closed), then the nearest path point to that; look: rail
  < 0 → feet + (point `+0x00` ahead along the path (closed; open on level 14 in mode 0) − the nearest point).
- **Rows claim:** dir = unit(look − position); angle = 90° − asin(dir · forward); D+0x50 → target by 0.00015; turn =
  angle spring (0, angle, D+0x50, 0.175, 0, D+0x64); forward = rot(saved, turn, saved × dir) normalised; left = unit(up_s
  × forward); up = forward × left.
- **Expect on 08:** grinding rail 5 from its start, the camera stays ≈ beside the rail on path 118, looking 0.5 ahead +
  1.5 up; no jitter; at the rail end (off_rail ≠ 0) the pre hook releases (+0x7e = 3) and the follow camera comes back
  with a cut and its row blend (D+0x20 = 90).
- **Edge:** no mapped interval holds the hero (mode 2) → no position step this tick [L]; nearest fails (> 21 from the
  path) → no position step, look = feet [L]; level 17's record (path −1) inert.
- **Method:** level-harness test on 08 (`RC_HERO_AT` on rail 5 with Grindboots, a grind for 300 ticks) asserting the
  current class 3 and the camera-to-rail distance; unit tests on `rail_update` with synthetic splines per mode.

### B6. Class 3 pre hook (0x316030)
- **Claim:** camera mode 3 and rail < 0 → stays; on the record's rail with off_rail 0 → stays; otherwise +0x7e = 3, or 5
  when the record's mode is 3.
- **Expect:** leaving the slide on 01 (mode 3) → the follow camera with the pose copied and the rates blend at **0.01**
  (level 1) — a smooth glide back; on 08 / 16 → a cut.

### B7. Class 3 switch in
- **Claim:** blend 5 (01): orbit blend 40 ticks (0x167373 = 2, 0x1673f4 = 40) from the follow camera; blend 3 (05, 08,
  16, 17): no blend (cut to the rail camera's first update from the copied pose — visually the first frame equals the
  old pose turned by one spring step).

### B8. Class 1 hook (0x2e8870)
- **Claim:** only for kind 4 (+0x74); a non-releasing best with priority ≥ 6 → 0; feet out of the cuboid → 0; the
  published camera (0x167240) in the cuboid too → 1 (then L03 blend 1: rates blend 0.018; L04 blend 3: a cut); else the
  current camera's +0x7e = 2, kind 0, rates 0.018 → 1 (L04: the rates blend because +0x7e = 2).
- **Setup:** 03, cuboid 45 centre (300.94, 133.86, 52.21); 04 cuboid 39 centre (223.72, 150.70, 63.33).
- **Expect:** walking in: the view moves to the record's spot ((299.54, 141.29, 49.89) on 03, rotation (0, 0, −1.339))
  blended over ~56 ticks; it then turns to follow Ratchet (+1.5 up) with a spring (k 0.02, d 0.1) acting as a
  fraction of the angle; walking out: +0x7e = 3 → the follow camera from that pose, no blend.

### B9. Class 1 update (0x2e8730)
- **Claim:** look = feet + (0, 0, D+0x5c = pvar +0x18 = 1.5); target = 90° − asin(dir · forward); turn = angle spring
  (0, target, 0.02, 0.1, 0, D+0x00) with D+0x00 starting as the follow camera's desired.x (the shared block's word);
  forward = saved turned toward dir by (90° − asin(saved · dir)) × turn about saved × dir.
- **Edge:** the first tick's turn is clamped to |target| (the leftover velocity is large): check that the first update
  turns by angle × target, not more.

### B10. Class 14 (0x2ebd70 / 0x2eb978 / 0x2ebdb0 / 0x2ebde8)
- **Claim:** hook −1 in a jump (camera mode 0x50) unless under water (group 0x11), and in group 6; init: the camera at
  feet + up·4 − 8·row0(rot), forward = unit(feet + up·1.5 − position), left = unit(forward × gravity), orbit blend 120
  ticks (the switch's 40 overwritten); update: (x, y) spring to Ratchet and z to the held height (k 0.01, d 0.2), the
  held height not following a jump out of the water; position = D + up·4 − 8·row0; forward toward D + up2·1.5; pre
  hook: out of the cuboid → +0x7e = 3.
- **Setup:** 03, cuboid 19 centre (206.60, 141.73, 59.29) (record #7 facing yaw π), cuboid 24 (198.84, 142.54, 68.58),
  38 (135.65, 185.79, 82.24), 39 (135.77, 196.28, 65.31).
- **Expect:** a side-on view at a fixed facing, smooth lateral follow; jumping inside does not raise the camera; jumping
  into the cuboid does not activate it (hook −1) until landing.

### B11. Follow camera tweaks (0x3111d8)
- **Claim (body 1):** no owner lock → `set_distance(3, 0.003, base)`, not gliding → look_h = 1 and pivot 1 (0.003);
  D+0x20c = 0.6. **(body 2):** D+0x230 < 2 → look 9 (0.005), distance 12 (base), pivot 15; `lock_toward`; D+0x20c =
  2.0, D+0x214 = 0.14. **(weapon up, 0x1413fa ≠ 0, group ≠ 0xf):** leash 0, `look_from_smoothed`, h spring 0.04 / 0.2.
  **(state 0x81, held & 3):** the same three. **(focus moby):** see B12.
- **Method:** unit tests on `update_type0` with a synthetic hero (`hero.mode`, `f13fa`, `state`, pad bits).
- **Expect in game:** holding a gun (Blaster) up: the camera's horizontal spring stiffer (0.04) and no leash: it lags
  less when strafing.

### B12. Scripted focus moby (0x16735c)
- **Claim:** with `focus_moby = Some(id)` and D+0x230 < 2: state ≥ 0x80 → cleared; the right stick |x| or |y| ≥ 0.3 →
  counter 0; counter += 1 (≤ 400); `turn_toward_point((n/400)·0.2094, 0, moby pos)` each tick; D+0x230 = 0 then the
  scan. Class 18's test: the region's moby = the focus moby → inside regardless of shapes.
- **Method:** unit test on `Camera` with `world.mobys` filled; `cinematic::focus_moby` applies it.

### B13. Target modes (0x30fb08 / 0x3101c0)
- **Claim:** the transitions in player_controller.md §15 "Target modes" table: entries (mode, prev, mode_t, mode_inv,
  raise_t, raise_h, raise_inv, raise_frozen, raise_a = vtarget, raise_b) for states 0xf (3: 30 / 85 / 3.0), 0xc (8:
  30), 0xb (6: 30 / 35 / 1.0), 0xd and 0xe (5: 30 / 45 / 3.5), group 4 (2: 30), group 2 (1: 10), state 0x14 (9: mode_t
  0, inv 1, raise 30 / 1.5); exits with 20 ticks; 9/10 → 1 with row blend 90 and saved forward; 10's raise timer;
  vertical target of 9 / 10 = lerp(plain, raise_a, 0.5); 11 for state 0x77.
- **Method:** unit tests on `target_mode` / `vertical_target` with synthetic hero states, one per row.
- **Regression:** the hero digests that pass through states 0xc, 0xd, 0xf, 0x14 change (new camera modes): intended.

### B14. Camera moby (0x20d068 / 0x2ba7c8)
- **Claim:** with the follow camera current and no camera moby: a `Create` request at the last published position;
  the tick creates class 1007 (+0x30 = 0xff, mode |= 0x41, position, `build_matrix`), `cam_moby` = id; another camera
  current → `Delete(id)`; the update: position = the current camera's own position (`LoopGlobals::cam_pos`); |d·up| <
  2.5 and (flat ≥ 0.1, |yaw diff| ≥ 135°, flat speed ≥ 0.1·dt) → the camera's spot on Ratchet's plane (stays at the
  camera under water); |d·up| < 2.5 otherwise → (15, 15, 15); |d·up| ≥ 2.5 → at the camera.
- **Setup:** any level (class 1007 is loaded on all 19); Ratchet running toward the camera on flat ground.
- **Expect:** Ratchet stops against an invisible wall at the camera's spot; switching to first person deletes it (a
  free slot appears), back → recreated.
- **Regression:** one dynamic moby slot taken from the first tick: ids of later-created mobys shift by one in the
  harnesses that have a moby world (intended, as the game).

### B15. 701's camera look (L01 0x2f9000)
- **Claim:** in states 2 and 3 each tick: `turn_toward(0.13963, 0, cuboid +0x80's row 0)`, `set_distance(7, 0.02,
  false)`, `set_pivot_height(2.5, 0.02)`, `set_look_height(3.5, 0.02, false)` through the cinematic queue.
- **Setup:** Novalis, a collapsing platform 701 (Ratchet above its trigger cuboid).
- **Expect:** while the platform rumbles and falls the camera swings round along the cuboid's x axis, pulls out to 7
  and up; the queue has the four calls per tick (test `take_calls`).

### B16. `Camera::reset` (0x20ee80)
- **Claim:** after reset the follow camera is current (class / Swingshot cameras inactive, their +0x7e cleared) and the
  blend mode 0.

## 4. Shared code touched (regression risk)

- `follow_camera/level.rs::activation_loop`: slot-based exclusion (was class-based), live +0x7e of the current camera,
  `won_slot`; class 1's / 14's hooks. Existing level tests: `unported_class_is_wanted` uses class 3 as "unported": now
  run → its assertion for class 3 changes.
- `follow_camera.rs`: `update` (class cameras, `handle_coll_with_hero`), `reset` (drops the class / Swingshot cameras,
  blend 0), `update_type0` (the whole 0x3111d8), `target_mode` and `vertical_target` (all modes), `switch_cameras`
  (class cameras; the follow +0x7e cleared on switches), `current_pos`, `set_focus_moby`, new fields.
- `focus.rs::focus_test` signature (+ the focus moby).
- `rc-formats/src/cameras.rs`: `RailCamera`, `SideView`.
- `cinematic.rs`: `FollowTurnToward`, `FollowLookHeight`, `FocusMoby` (additive).
- `tick.rs`: `LoopGlobals::cam_pos`, the focus moby feed, the camera moby create / delete after the camera update;
  `MobySystem::create_moby` / `delete_moby` (default None / no-op), implemented in `services.rs::SharedServices` and
  delegated in `rc-engine/src/gameplay.rs::HeroWorld`.
- `classes/mod.rs`: `ClassUpdate::CameraMoby` (ALL 74).
- `classes/props.rs`: 701's look (the `unported("701: camera look")` counts are gone).
- Digest / harness effects (intended): the camera moby's slot; the hero digests through states 0xc / 0xd / 0xf / 0x14
  / 0x77 (target modes) and with a weapon up (0x1413fa leash / spring); grind runs on 08 / 16 and Novalis's sinking
  (class 3 takes the view).

## 5. Not ported (don't test as working)

- Class 19, the armed fly-by (10, 13, 14, 15): no ported class arms it (units 343 / 344 / 415 / 451 / 458 / 490);
  needs a gameplay fade / FOV on the camera view — G-HERO-027 (spec: player_controller.md §15).
- Classes 20 / 21, the level-14 grind race cameras — G-LVL-007.
- Class 8, the hoverboard camera (05, 16) — G-HERO-008.
- Class 22, giant Clank's focus (15) — G-HERO-005.
- The writers of the focus moby 0x16735c (units 1422, 1470, 1051) — G-HERO-026 / G-CLS-001.
- n/a rows: UpdateCam +0x8e (no reader), the D words written and never read by the classes, the yaw stabiliser (gp
  0x16220c: no writer), the camera moby's "no camera" (5, 5, 5) branch.

## 6. In-game QA spots (for the user)

- **Batalia rails (class 3, mode 0 / 2):** `RC_LEVEL=8 RC_GIVE_ITEMS=29 RC_HERO_AT=176.98,151.81,42.5,-1.6` (rail 5's
  start; jump onto it): the camera rides beside the rail and looks 0.5 ahead; at the end it cuts back to the follow
  camera. Rail 6 (mode 2): `RC_HERO_AT=165.64,128.24,40.5,2.47`; rail 0 (mode 2): `RC_HERO_AT=244.65,68.39,43.5,-1.8`.
  Switching rails mid-ride hands over from one rail camera to the next without the follow camera.
- **Kalebo III rails (16):** rail 1 `RC_LEVEL=16 RC_GIVE_ITEMS=29 RC_HERO_AT=113.99,264.07,134.5,3.14` (mode 2, the long
  one), rail 3 `RC_HERO_AT=170.27,108.76,121,0.81` (mode 0), rail 8 `RC_HERO_AT=299.27,76.35,136,0.03` (mode 1: the
  camera 4 behind along path 158).
- **Novalis slide / sinking (class 3, mode 3):** `RC_LEVEL=1 RC_HERO_AT=265.83,201.48,96,-0.6` (path 42's start): in
  the sinking state the view glides in (orbit blend, 40 ticks), stays 3 behind along the path, and glides back out
  (0.01 rates) — the same spot as the old wanted-class-3 log.
- **Kerwan fixed / side views (classes 1 / 14):** `RC_LEVEL=3 RC_HERO_AT=300.94,133.86,52.5,0` (class 1: the view at
  (299.5, 141.3, 49.9) turning after Ratchet, blended in); `RC_LEVEL=3 RC_HERO_AT=206.6,141.73,59.5,3.14` (class 14,
  cuboid 19: a side view 8 behind, 4 up, Ratchet's jumps not followed; enter by walking, not by jumping in).
- **Eudora fixed view:** `RC_LEVEL=4 RC_HERO_AT=223.72,150.7,63.5,0`.
- **Camera moby:** anywhere: run straight at the camera (pull the stick toward the screen) at the camera's height: Ratchet
  is stopped where the camera stands.
- **Weapon up:** Blaster held up and strafing: the camera follows tighter.
- **Novalis collapsing platforms (701):** step on one: the camera turns along the platform, pulls back to 7 and up
  while it rumbles and falls.

## 7. Results (the test expert fills this in)
