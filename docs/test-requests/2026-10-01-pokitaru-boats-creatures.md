---
status: open
job: pokitaru-boats-creatures
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [classes]
---

# Pokitaru's boats 1075, Kerwan's gun troopers 574 with their rocket 833, Batalia's runners 452

## 1. Summary

- **Pokitaru boats, class 1075 (level 11, 2 instances #440 / #441; unit U363)**: the whole update `0x309ac0` and its
  propeller / wake helper `0x30a500`, plus the three entry points other classes call (`0x30a480` start, `0x30a840`
  skip, `0x30a830` arrived). The hand-off with the boat biters 1246 (`units::pokitaru_biter`, already ported) is the
  boat's state (> 1 shows the 30 hidden boarders, 5 deletes the ones still hidden), its platform block (pvar +0x60:
  the biters, Ratchet and bolts ride it through `CarryRiders`) and the boarders' area path (the boat's +0xa8 = the
  biters' +0x230), which the boat moves with it every sailing tick. **Only the commando 114 (unported, G-ENM-011)
  starts a boat**: in the game as in the port, without it the boats wait at their path's start forever.
- **Kerwan gun troopers, class 574 (level 3, 19 placed / 17 created; unit U130) and their rocket 833** (created by
  code): the whole update `0x2c6fd0` (16 states), its hit handler `0x2c95a8`, node picker `0x2c9ca0`, turn check
  `0x2c6e18`, group test `0x2c6d98` (+ `0x2c6d58`, `0x2c5b78`), the 822 interface `0x2c6c60` / `0x2c6c90`, the rocket
  spawner `0x2d4288` and update `0x2d43c8`.
- **Batalia runners, class 452 (level 8, 3 instances #341–#343; unit U280)**: the whole update `0x2e2df0` and its
  member refill `0x2e2da0`. The member (collecting) mechanics are dead on the disc (class 633 exists on no level; see
  B-R7) but ported.
- **Shared layer extended (plug-and-play)**: `creature::fx::muzzle_smoke` (level03 `0x250ae8`, also the code of L10
  0x255ab0 / L15 0x251d00 / L18 0x264c10), `fx::part21`, `fx::part44`, `fx::turn_about`, `creature::hard_cut`
  (`fun_00212ed8`), `bolt::start_fly_to` (the bolt fly-off with a target), and the moby → follow-camera channel for
  the distance / pivot-height setters (`cinematic::follow_distance` / `follow_pivot_height` → `CinematicCall::
  FollowDistance` / `FollowPivotHeight` → `Camera::set_distance` / `set_pivot_height`). A future class that fires a
  gun with the smoke ring calls `fx::muzzle_smoke(w, id, muzzle, None)`; a class that pulls the follow camera out calls
  the two `cinematic` functions.

## 2. Where it lives

- `crates/rc-game/src/moby_update/classes/units/pokitaru_boat.rs`: `update` (0x309ac0), `init`, `sail`,
  `boarders_done` (0x30a850), `propellers` (0x30a500), `try_start` (0x30a480), `skip_to_end` (0x30a840), `arrived`
  (0x30a830). Coverage table in the module doc.
- `crates/rc-game/src/moby_update/classes/units/kerwan_trooper.rs`: `update`, `init`, `stand`, `aim`, `fire` /
  `shoot`, `punch`, `patrol`, `follow`, `knocked`, `fall`, `turn`, `dying`, `tail`, `explode`, `hits` (0x2c95a8),
  `look`, `turn_check` (0x2c6e18), `pick_node` (0x2c9ca0), `group_alerted` (0x2c6d98), `waiting` / `send_away`
  (0x2c6c60 / 0x2c6c90), `spawn_rocket` (0x2d4288), `rocket_update` (0x2d43c8). Coverage table in the module doc.
- `crates/rc-game/src/moby_update/classes/units/batalia_runner.rs`: `update` (0x2e2df0), `refill` (0x2e2da0),
  `run_to`, `next`. Coverage table in the module doc.
- `crates/rc-game/src/moby_update/classes/units/mod.rs`: `PORTS` rows "U363 1075", "U130 574", "U130 833", "U280 452"
  (joint lists loaded for 1075, 574, 452).
- `crates/rc-game/src/moby_update/creature/fx.rs`: `part21`, `part44`, `turn_about`, `muzzle_smoke`.
- `crates/rc-game/src/moby_update/creature.rs`: `hard_cut`.
- `crates/rc-game/src/moby_update/classes/bolt.rs`: `start_fly_to` (and `start_fly` now calls it).
- `crates/rc-game/src/cinematic.rs`: `CinematicCall::FollowDistance` / `FollowPivotHeight`, `follow_distance`,
  `follow_pivot_height`, their arms in `apply_camera_calls`.
- Docs: `docs/plan/creatures.md` §14, `docs/plan/gaps.md` G-ENM-011 (new), G-SAV-006 / G-HERO-026 (consumers added).

Level data facts used below (from the disc, verified by dumping the gameplay files):
- Level 11: boat #440 at (494.98, 479.55, 222.58), pvars +0xa0 path 12 (600 points, z 223, point 3 has w = 37), +0xa8
  area path 13 / +0xac local copy 14 (11 points each, identical), +0xb0 = 1 (lift), +0xb4 = group 57; boat #441 at
  (440, 647.01, 223), path 30 (500 points, the node with w = 37 is index 239), area 28 / 29, +0xb0 = 0, group 58. The
  boarders: 1246 #551–#565… in groups 57 (17 members) / 58 (12 members), +0x254 = 440 / 441, +0x230 = 13 / 28.
- Level 3: 574 #860–#878 (pvar data per instance in the module doc's pvar list); #872–#875 stand on the 1210
  platforms #14–#17 (+0x280), #876 has +0x262 = 1, #860 / #862 / #863 wait with a 573 (+0x284 = #773 / #774 / #778).
- Level 8: 452 #341 (path 18, trigger 18), #342 (19 / 19), #343 (20 / 20, second path 52); their links +0x130 name
  class-13 bolts (#3–#10).

## 3. Behaviours to verify

### Pokitaru boat 1075

### B-B1. Init (state 0)
- **Claim:** `0x309ac0` case 0: draw distance (moby +0x32) = 0xff; seq 1 blended over 1 tick (unless seq B is 1); the
  local copy +0xac[k] = (+0xa8[k] − pos)·rowsᵀ for every point of +0xa8 (written into the shared spline table); +0xc4
  = +0xa0 (the path index); position = the path's point 0 (all 4 lanes); node +0xd4 = 1; state 1; the four
  propeller angles +0xf0..+0xfc = four `rand_angle` draws (in order); voices +0xc8 / +0xcc = −1; the platform block
  flags (pvar +0x9c) |= 1.
- **Setup:** level 11, boat #440 (or a synthetic boat with pvars 0x200, +0x08 = 0x60, mode 0x20).
- **Trigger:** one update.
- **Expect:** the values above; the boat sits at (496, 480, 223) (+ the bob, B-B6); spline 14's points equal spline
  13's points in the boat's local frame.
- **Edge cases:** the bob is applied at the end of every update including init (z = 223 + sin(phase)·0.13 with phase
  = dt·π/2 after the first tick).
- **Suggested method:** level-harness test on level 11.

### B-B2. Waiting (state 1), the skip
- **Claim:** state 1 does nothing until +0xb8 ≠ 0 (`skip_to_end`, 0x30a840); then the position = the path's point
  count−2, yaw = atan to point count−1, +0xc4 = +0xa0, state 5. Node stays 1.
- **Setup:** boat in state 1.
- **Trigger:** `pokitaru_boat::skip_to_end(w, boat)`, one update.
- **Expect:** boat #440 at path 12's point 598, facing point 599; `arrived(w, boat)` true; the hidden boarders of the
  group get deleted by their own tick (1246's tick: boat state 5 and not drawn → `DeleteMoby`).
- **Edge cases:** skip while in state 2–4 does nothing (only case 1 reads +0xb8).
- **Suggested method:** unit / level-harness.

### B-B3. Start (0x30a480) and the boarders' hand-off
- **Claim:** `try_start(w, boat)`: only in state 1 and with Ratchet's ground moby (0x13f64c) = the boat and air ticks
  (0x13f65e) = 0: state 2, seq 0 blended over `ticks(20)` (unless seq B is 0), returns true; else false and nothing.
  Once the boat's state is > 1 every 1246 boarder of it in state 0xc becomes drawn (+0x31 = 1), shown (mode &= ~1)
  and gets its class collision; boarders then jump aboard (≤ 5 boarding at a time per group) when the boat's local x
  of the biter is below its +0x258 and above −3.1 (`pokitaru_biter::board`).
- **Setup:** level 11, boat #440 in state 1; set `hero.ground_moby = Some(440)`, `hero.air_ticks = 0`.
- **Trigger:** `try_start`, then run updates.
- **Expect:** state 2; the 17 group-57 boarders become visible on the next tick.
- **Edge cases:** air ticks ≠ 0 → false; ground moby another moby → false; state 2 → false.
- **Suggested method:** level-harness (the caller 114 is not ported, G-ENM-011).

### B-B4. Sailing (state 2): engine sounds, speed, path, camera, area path
- **Claim (0x309ac0 case 2):** target = path point [node]. Ratchet's ground moby = the boat → keep class sound 1
  (flags 4) alive in +0xcc (`SoundIsAlive` else `PlayClassSound(1, 4)`), release +0xc8 if it still plays a sound of
  the boat (owner-guarded) and set it −1; speed +0xd0 `Approach(6·dt, 6·dt²)`. Otherwise sound 0 in +0xc8, release
  +0xcc, `Approach(0, 6·dt²)`. Move: d = target − pos clamped to length speed (3-D), pos += d. Node < count−1: within
  6·dt (3-D) of the target → node + 1; the new node's w = 37.0 → +0xc0 = 1. Last node: |d|xy < 0.001 → both voices
  released, state 3. Turn: `0x270cc0(atan(target − pos), 30°·dt², 30°·dt², 180°·dt, &yaw, &+0xd8)`. +0xc0 set → each
  tick `cinematic::follow_distance(7, 0.003 (0x3b449ba6), false)` and `follow_pivot_height(5, 0.003)`. Area path:
  +0xa8[k] = rows·+0xac[k] + pos (rows = the moby's current rows, last MobyBuildMatrix).
- **Setup:** boat #440 started (B-B3), Ratchet standing on it.
- **Trigger:** run ticks.
- **Expect:** speed ramps to 0.1/tick (6 u/s) at 0.001667/tick²; the boat follows path 12 heading along it; class
  sound 1 playing, 0 released; after node 3 is passed (w = 37) the camera calls are queued every tick (check
  `svc.cinematic.calls`); spline 13's points move with the boat (point_in_polygon tests of the boarders follow).
  Ratchet off the boat → sound 0, speed decays to 0 and the boat stops (stays in state 2).
- **Edge cases:** the camera calls only change the camera when the follow camera is current (`set_distance` /
  `set_pivot_height` are no-ops otherwise); sound slots released only when they still play a sound of the boat.
- **Suggested method:** level-harness; sound sink mock for the voice checks.

### B-B5. End of path (states 3, 4, 5)
- **Claim:** state 3: when every live (state < 0x80) member of group +0xb4 (s16) with class 1246 is in state 0x14,
  0xb, 0x12 or 0xc (`0x30a850` / `0x316128`): +0xb0 ≠ 0 → speed 0, state 4; else state 5. A group index out of the
  table or an empty list counts as done. State 4: `0x270830(node z + 6, dt², dt², dt, &z, &+0xd0)` (spring); z ≥ node
  z + 5.9 → 5. State 5 with +0xb0: Ratchet's z (0x13f3d8) below the boat → spring to node z + 1, else node z + 8.
- **Setup:** boat at the last node, group members in various states.
- **Expect:** #440 (lift) rises ~6 above its last point, then rides between +1 and +8 as Ratchet is below / above it;
  #441 just stops (state 5).
- **Edge cases:** a member in state 0xd/0xe/0xf/0x10 (boarding / aboard) keeps the boat in 3; members of other classes
  are ignored; dead members (state ≥ 0x80) are ignored.
- **Suggested method:** unit test with a synthetic group.

### B-B6. Bob, wobble, CarryRiders (every tick)
- **Claim:** at entry (state ≠ 0) z = +0xe4 (the un-bobbed z). Tail: +0xe4 = z; +0xe0 = add_rot(+0xe0, dt·π/2); z += 
  sin(+0xe0)·0.13; `0x277a80(0.0419 (0x3d2b92a6), 25°·dt (dt·0.43633232), 35°·dt (dt·0.61086524), m, +0xe8, +0xec)`
  (rot.x = amp·sin a·sin b, rot.y = amp·sin a·cos b); `CarryRiders(+0x60, pos − entry pos, entry rot, rot)` (block +0x00
  the Euler of the rotation change, +0x10 the displacement).
- **Expect:** a boat that does not sail still writes a non-zero block every tick (the bob and the wobble), so Ratchet
  standing on it bobs and tilts with it; the biters aboard (`pokitaru_biter::carry`) move by the same block.
- **Suggested method:** unit test on the block values.

### B-B7. Propellers and wake (0x30a500, every tick before the states)
- **Claim:** for i = 0..3: manipulator record at +0x100 + 0x40·i attached to the boat's own joint list i
  (`AttachManipulator`) when not attached; when attached its angle +0xf0[i] += dt·12.217304 (700°/s; 2.6179938 = 150°/s
  in state 1) and its quaternion is the rotation about axis 0 (`FUN_00221e38`). Wake: xy distance to the camera ≤ 38;
  state 1 on odd ticks (tick counter & 1) → none; z (un-bobbed) < 224.5: for each joint list 0..3, 2 bubbles (1 in state
  1): pos = joint point + (randf_sym(0, 0.3) ×3), heading = yaw + π + randf_sym(0, 0.1222), speed randf(2, 5)·dt
  (×0.4 in state 1), vz randf(−2, 2)·dt, size randf(6300, 14700), `PartType34Spawn(size, 222.5, pos, vel)`.
- **Setup:** boat #440 with the camera within 38 (xy).
- **Expect:** the four propellers turn (joint modifiers on the boat's mobys' joint list targets); type-34 bubbles 4 per
  even tick in state 1, 8 per tick when sailing; none with the camera beyond 38 or the boat z ≥ 224.5 (lifted #440).
- **Edge cases:** the rand draw order per bubble: 3 × randf_sym(0.3), randf_sym(0.1222), randf(2, 5), randf(−2, 2),
  randf(6300, 14700).
- **Suggested method:** level-harness (counts in `svc.fx.part_spawns[34]`), visual QA.

### Kerwan trooper 574 and rocket 833

### B-T1. Init (state 0)
- **Claim:** see the module doc row "0": +0x7f = 0x1c; light word + ambient copied from Ratchet's moby; walker J at
  +0xd0 seeded then J+0xc 3, J+0x18 0, J+0x44 π·dt, J+0x24 6·dt, J+8 2, J+0x28 π, J+0x40 4π·dt², J+0x3c 2π·dt², J+0x38
  flags 9, J+0x2c = 6·dt·multiply_global_scale(15); health 3.0, meter (s16 +0x24) 3, +0x29 = 1, +0x28 = 2, +0x30 =
  1.75, K z offset (+0x98) 0.7, K air (+0xbc) 0, K radius (+0x90) 716; +0x58 = 10, +0x5a = 0xc; +0x40..+0x4f zero; mode
  |= 0x1000; cooldown (+0x250) 0; node 0; +0x130 = +0x134; position = path point 0; state 1; +0x288 = draw distance;
  +0x28a = level word 0x1618f4 & 7, the word + 1; +0x263 = 0; the path's points 0..count−3 get w = the 3-D distance to
  the next point (written into the shared spline); +0x284 ≠ −1 and +0x290 = −1 → seq 0 (`ticks(10)`, unguarded), state 2.
- **Setup:** level 3, every 574.
- **Expect:** troopers #861 etc. at their paths' first points; #860 / #862 / #863 (pod links) stay in 1 because they
  have a jump path (+0x290 ≠ −1); slots 0..7 assigned in update order.
- **Suggested method:** level-harness.

### B-T2. Visibility, deletion, the mission-hidden one
- **Claim:** +0x262 = 1 (#876): mission (+0xb0) not done → nothing at all (return); done → no collision, mode |= 0x41,
  return. Game mode (0x15f5c4) 2 → mode |= 0x41 (still updates); otherwise (state ≠ 0xe) mode &= ~0x41. Position out of
  [2, 1021]³, or path id −1 outside state 0xd → `DeleteMoby`.
- **Suggested method:** unit test.

### B-T3. Look-at (head list 2, torso list 3)
- **Claim:** drawn or d < 24 (d = xy distance to the target): state 0xc: head yaw target (+0x1b8) = 0.25·Δ, torso
  (+0x238) = 0.75·Δ with Δ = sub_rot(+0x28c, yaw); state < 7 and d < +0x270: torso = sub_rot(atan(target − pos), yaw)
  (head unchanged); both clamped to ±π/4; then `manip::look` on +0x150 (list 2) and +0x1d0 (list 3) with k 0.03, d 0.3.
  The head record's scale (+0x1c0) is written 1.0 every tick (the non-cheat branch of 0x251d70).
- **Expect:** the trooper's head and torso joints turn toward Ratchet within ±45°.
- **Suggested method:** unit test on the records; visual QA.

### B-T4. Standing (state 1): jump-down path
- **Claim:** a done sequence other than 0 → seq 0 (`ticks(10)`, unguarded). With a jump path (+0x290): at its point 0
  facing point 1 every tick; goes (node +0x298 = 0, state 9, seq 1 guarded) when the group is alerted (a live member is
  a 574 in state 9 or a 573 in state 5), or the target's position is inside cuboid +0x29c, or the mission is done, or d
  < +0x294 and |dz| < 4.
- **Edge cases:** group byte 0xff → not alerted; out-of-range d or dz ≥ 4 → waits.
- **Suggested method:** level-harness (#860: jump path 73, range 20).

### B-T5. Standing (state 1): pod, turn, jump range, punch, aim
- **Claim:** see the module doc rows "1". Order: pod link → seq 0, state 2; in sight (d < +0x270) or jump range (d <
  +0x27c): the turn check (B-T10) first; jump range and |dz| < 4 → +0x27c = 0, +0x256 = 1, state 8, seq 1; then d < 3.5
  and |dz| < 3 → state 6, seq 2; else in sight, |dz| < 10, cooldown (`FastDecTimer` on +0x250) not running, facing
  within 45° → drawn: state 4, seq 3.
- **Edge cases:** the cooldown is only decremented when the earlier tests pass (the game's evaluation order).
- **Suggested method:** level-harness with Ratchet placed.

### B-T6. Aim (4) and fire (5), the rocket
- **Claim:** 4: anim done → 5, +0x254 = 0, seq 4 over 0 ticks; else punch range → 6; else within the leash +0x278 and
  |dz| < 10 → the next node (`pick_node` when +0x274 = 0, else node + 1 if not at the end) and 7 (seq 1). 5: key 8
  passed (`passed_frame(8)`) → +0x254 + 1, v = aim point (target rec +0x20) − joint 0, set to length 6·dt, z × 0.5;
  heading atan(v); more than 30° off the yaw → heading = yaw ± 30° and v = (cos, sin, 0)·6·dt; muzzle = joint 0 +
  v·multiply_global_scale(10); `fx::muzzle_smoke(m, muzzle)`; `spawn_rocket(heading, 6·dt, 0.75, v, joint 0, m,
  ticks(130))`. Then the not-done / done branches of the module doc row "5".
- **Expect:** a rocket 833 per firing key; 24 type-44 smoke puffs + 6 type-21 sparks at the muzzle (`part_spawns`
  44 += 24, 21 += 6 per shot); the trooper keeps firing while Ratchet stays in sight within 5° of its facing.
- **Suggested method:** level-harness; count rockets and particles.

### B-T7. The rocket 833 (spawn + update)
- **Claim:** spawn: `CreateMoby(833)`: update dist 0xff, draw 0x7e, state 1, drawn; pos = joint 0; pvars +0x00 v,
  +0x10 shooter, +0x14 life = ticks(ticks(130)), +0x18 0.75, +0x1c heading, +0x24 speed, +0x28 0; yaw atan(v); a line
  from (shooter.x, shooter.y, joint z) to joint 0 (flags 2, ignore shooter) that hits → pos = hit point, life 0, speed
  0; `MobyBuildMatrix`. Update state 1: target acquired with range float(life << 1)·speed; pos += v + Ratchet's applied
  platform motion (0x13f490 with z × 0.35); yaw = heading, rot.x += 2π·dt; two type-4 puffs at pos − 0.2·v̂ with vel =
  (cos, sin)·speed·(−0.5) + v + platform (A: 0x6f00afff / 0xff, life ticks(rand_range(15, 22)), base 0x28, growth
  rand_range(20, 35), additive; B: 0x1fffffff / 0x4f4f4f, ticks(rand_range(30, 60)), 0x28, rand_range(50, 75), not
  additive); a target moby: pitch (rot.y) = SpringTurn(rot.y, −atan(xy dist to aim, aim.z − z), π/4·dt², π·dt², π/6·dt,
  &+0x20), z += sin(−pitch)·speed; collision `CollLine_Fix(pos, old pos, 0, shooter)` else `coll_sphere(0.2, pos, 0,
  shooter)`: a moby hit → hit record (flags 0x10001, damage 1, b18 1, b19 1, class 833, push (cos h, sin h, 1, exact
  marker), w20 1) delivered to it; any hit → state 2. Life `FastDecTimer` ≠ 0 → 2. State 2: death explosion (0.25,
  light 13, class sound 0) and delete.
- **Expect:** rockets fly at 6 u/s, home vertically on Ratchet, hit him (damage 1) or blow up on walls / after
  their life; smoke trail.
- **Edge cases:** life 0 at spawn (muzzle inside a wall) → explodes on its first update.
- **Suggested method:** unit test of `rocket_update` on a synthetic world; level-harness for the hit on Ratchet.

### B-T8. Punch (state 6)
- **Claim:** key time read; `SpringTurn2(atan(target), 0.03, 0.3, 0.3, +0x258)`; keys 25..31, |dz| < 2, d < 3.5,
  facing within 15° → `deliver_hit(target moby)` with flags 0x10001, damage 1, b18 0, b19 1, class 574, push (cos yaw,
  sin yaw, 1, exact marker), then nothing else this tick; anim done → the next node and 7 (or 1 + turn check at the
  path's end when +0x274 ≠ 0).
- **Expect:** Ratchet loses one health orb per punch hitting him in that key window (the hit record re-delivers each
  tick of the window: the resolver's cooldown on the hero side decides).
- **Suggested method:** level-harness.

### B-T9. Patrol (7), follow down (8), jump down (9)
- **Claim:** module doc rows 7 / 8 / 9: walk_to (J at +0xd0) to the node point; arrival bits 0x14; state 8 / 9 take
  the nearest segment (`spline::nearest(…, 1000, 5, band 0, open)`) as a lower bound of the node; the jump path's end
  → +0x290 = −1, state 1 (seq 0 guarded), then a pod link → seq 0 (unguarded), state 2, else the turn check.
- **Suggested method:** level-harness with a trooper on a short path.

### B-T10. The turn check (0x2c6e18)
- **Claim:** no target moby → false. 822 link (+0x264) of class 822 with `cmd` ≠ 4 → false. a = |sub_rot(h, yaw)|:
  (a < π/4 or +0x263 ≠ 0 or seq B = 1) and a ≤ π/2 → false. d(xy) ≥ 20 → false. Else: a > 1.1780972 → seq 1 (ticks 10,
  unguarded) and anim speed 1; else seq 9 (ticks 10, unguarded); +0x28c = h; `cmd` (moby +0xbc) = the state; state 0xc;
  true.
- **Then state 0xc (B-T11).**
- **Suggested method:** unit test.

### B-T11. Turning (0xc)
- **Claim:** seq B 1: `SpringTurn2(+0x28c, 0.09, 0.27, 0.15)`; seq B 9: `(0.01, 0.3, 0.2)`; other seqs: nothing. Within
  2° (0.034906585): +0x263 = ticks(60); `cmd` 7 / 8 → state `cmd` (7 picks the node first), seq 1 guarded, and (seq 1
  branch only) anim speed 1; other `cmd` → state 1, seq 0 guarded, cooldown ticks(10).
- **Suggested method:** unit test.

### B-T12. Hits (0x2c95a8)
- **Claim:** module doc "The hits": the lure (+0x38) → alert ticks(240) (s16 +0x252); sight +0x270 = +0x26c + 5 while
  the alert runs. Resolver column 4, mask 0x330000. Reaction 1 (or health ≤ 0): K gravity 40·dt², drag 100·dt², speed
  3/t + 50·dt²·t, up s with s = sqrt(160·dt²), t = 2s/g; aim from the hit's push; start seq 7, 5 ticks, frame 2; keys
  10 / 20; air 2·dt; +0x255 0; untargetable; flash 0xfa; state 0xd; class sound 3 (flags 0). 3 / 7 / 8: flash 0x78,
  gravity 80·dt², drag 100·dt², speed 2.5/t + 50·dt²·t, up sqrt(160·dt²), seq 5 frame 0, keys 6 / 13, state 10, sound
  1. 4 / 5: seq 6 frame 2, keys 5 / 10. 6: the drag is not written, speed halved, facing within 60° of the push heading
  → yaw = heading + π, seq 5 then hard cut (5, frame 5), keys 14 / 28. 0xc / 0xd: flash 0x78 only. Every passed hit:
  `flash::start`. Always: hit slot 0xff, `flash::update`.
- **Expect:** the Blaster / wrench knock a trooper back (state 10) with class sound 1; the third point of damage kills
  (health 3 at init): death flight, class sound 3.
- **Edge cases:** hits in state 0xd are ignored (flash update only); out5 = 1 hits ignored.
- **Suggested method:** level-harness with a synthetic hit record.

### B-T13. Knocked (10), falling (0xb), dying (0xd), explosion and respawn
- **Claim:** module doc rows 10 / 0xb / 0xd and **explode**: beam explosion (no damage, flash 2 / 1 / 9, scale 1,
  light 15, 5 streaks, 2 spark pairs, 4 puffs, class sound 6, shake, 1 fireball) at pos + 1 z, pieces 0x6d0 / 0x6d1 /
  0x6d2 (`BreakFxB`, zero vectors), `SetDeathBits(m, 0 or 0x200, −1)` (bolts), then a 822-linked trooper (+0x264 ≠ −1)
  goes to its path start hidden without collision (state 0xe), any other is deleted. Knocked: phase 2 → anim speed 1;
  landing off a ledge (ground more than 5 below, probed from z + 2) → class sound 10 flags 0x21, seq 0xb, state 0xb;
  fall: z < 5 → explode(0x200), else continue as 0xc. Dying: landed (& 0x40) → explode(0); otherwise below 5 →
  `SetDeathBits(0)` and respawn / delete (no explosion); falling from 5+ above the ground → class sound 10 (flags 0)
  once (`cmd` = 0xb).
- **Suggested method:** level-harness.

### B-T14. The tail: shadow, riding platforms, gravity, falling off
- **Claim:** module doc "The tail". Riding: +0x280's moby gone → −1; a 1210 / 821: all path points carried; position
  carried and yaw = the carried rotation's z. Gravity (not 0xb..0xe): +0xe8 += 9.8·dt² (0 in 10); z −= +0xe8; ground
  (0.5 above, flags 0): below → z = ground, +0xe8 = 0, the hit moby with a platform block becomes +0x280; more than 5
  above → seq 0xb, state 0xb, K zeroed, K vz = −+0xe8, gravity 19.6·dt². On its tick slot (counter & 7 = +0x28a) within
  70 (xy) of the camera: z < 5 or 10 below the path's nearest point → explode(0x200).
- **Expect:** troopers #872–#875 move with the 1210 platforms when those move; a trooper pushed off a ledge falls,
  and explodes below 5.
- **Suggested method:** level-harness.

### B-T15. 822 interface and group test
- **Claim:** `waiting(m)` = class 574 in state 0xe; `send_away(m)` sets 0xe; `group_alerted(g)`: a live member that is
  a 574 in 9 or a 573 in 5.
- **Suggested method:** unit test.

### Batalia runner 452

### B-R1. Init (0) and wait (1)
- **Claim:** 0: +0xe0 = +0xe4, at its point 0, node 1; +0x114 ≥ 0 → +0x100 = 1, +0x110 = +0x114; mode |= 0x1000; +0x29
  = 0; Ratchet's light / ambient; +0x40 zeroed; state 1; seq 1 over 0 ticks (guarded). 1: Ratchet within 24 (3-D) of the
  trigger path (+0x220)'s point 0 → `refill` → state 5, seq 3 over 6 raw ticks (guarded).
- **Setup:** level 8, #341.
- **Suggested method:** level-harness.

### B-R2. Run (5)
- **Claim:** next = (node + count + step) % count (step = s8 +0xd4); `SpringTurn2(atan(next − pos), 0.01, 0.3, 0.1,
  +0x224)`; members within 6 (B-R7); no live member → seq 3 over ticks(20) (guarded); next = count−1 → +0x114 < 0 →
  `DeleteMoby` (no flash update), else state 6; otherwise move (d = next − pos set to 10·dt, pos += d) and within 0.5 →
  node = next.
- **Expect:** #341 / #342 run their paths at 10 u/s and vanish at the end; #343 switches to path 52.
- **Suggested method:** level-harness.

### B-R3. Flee on the second path (6 / 7)
- **Claim:** 6: the same stepping on +0x110 / +0x100 / +0x104; Ratchet farther than 19 (3-D) → 7. 7: turn to Ratchet;
  within 17 or `devastator::already_targeted` → state 6, and when Ratchet is nearer the next node than the runner, the
  step +0x104 is negated.
- **Suggested method:** level-harness with #343.

### B-R4. The hit and the flight (8)
- **Claim:** any hit record with mask 0x210000 outside state 8: members in state 3 → 1; K gravity 0.008, flags 1, up
  10·dt, drag 0.0005, speed 8·dt, +0x3d 0; `knock::start(atan(pos − Ratchet), K, seq 4, 1 tick, frame 0)`; flash value
  0x78 (no flash start); `SetDeathBits(m, 0, −1)`; state 8. Always hit slot 0xff. 8: `knock::update`; & 3 → death
  explosion (size 1, light 13, no sound), delete; else z < 5 → delete; else flash update.
- **Expect:** its bolts drop when hit; it tumbles and blows up when the flight lands.
- **Suggested method:** level-harness with a synthetic hit.

### B-R5. Shadow probe
- **Claim:** drawn and within 29 (3-D) of the camera → `shadows::probe_down`, +0x7f = 0x17.
- **Suggested method:** unit test.

### B-R6. States 2 / 3 / 4 (no setter on the disc)
- **Claim:** 2: the next node's w ≠ 0 → run toward it; w = 0 → state 3, seq 2 over 6. 3: anim speed 3; done → refill,
  live members → state 3, state 4, seq 6 over 6, anim speed 1. 4: each live member pulled toward joint 0 by ≤ 4·dt;
  within 2·dt → member state 4, slot freed, +0x22c + 1; none → state 5 (seq 3).
- **Suggested method:** unit test with a synthetic class-633 member (set the runner's state to 2).

### B-R7. Members (refill 0x2e2da0, the eat in state 5)
- **Claim:** `refill`: the ten links +0x130 (moby indices) whose class is 633 are written into the member slots from
  slot 0 (the rest untouched). State 5: a live member within 6 (xy) → seq 6 over ticks(20) (guarded),
  `bolt::start_fly_to(member, 0, 0, Some(runner))`, slot freed. On the disc no class 633 exists: the list stays empty.
- **Suggested method:** unit test with synthetic class 633.

### Shared layer

### B-S1. `fx::muzzle_smoke` (level03 0x250ae8)
- **Claim:** 24 type-44 puffs (60000, 3000, 0.85, −0.001, w 0.85, alpha 0x1e, rgb 0xffffff) with directions the moby's
  rows[2] turned about rows[0] by i·15° + randf_sym(0, 7.5°), speed randf(0.05, 0.1), spin rand_range(0, 3) negated on
  randi(2) ≠ 0, life rand_range(ticks(30), ticks(90)); then 6 type-21 sparks (10000, 0x4f007fff / 0x1fffffff, split
  1) at i·60° + randf(0, π/4), speed randf(0.05, 0.1), life rand_range(ticks(20), ticks(60)). Draw order as listed (+
  each spawner's own rotation draw).
- **Suggested method:** unit test of the draw count and the spawned records.

### B-S2. `cinematic::follow_distance` / `follow_pivot_height`
- **Claim:** queued as `CinematicCall::FollowDistance` / `FollowPivotHeight` and applied with the tick's camera calls
  to `Camera::set_distance(d, rate, base)` / `set_pivot_height(h, rate)` (no-ops unless the follow camera is current).
- **Suggested method:** unit test on `apply_camera_calls`.

### B-S3. `bolt::start_fly_to`, `creature::hard_cut`
- **Claim:** `start_fly` is now `start_fly_to(…, hero moby)` (unchanged behaviour); `start_fly_to` stores the given
  target at +0x60 and takes the spin axis toward it. `hard_cut` = `moby_anim::hard_cut` + the loop sound refresh.
- **Suggested method:** the existing bolt tests must still pass (regression).

## 4. Shared code touched (regression risk)

- `bolt.rs`: `start_fly` refactored into `start_fly_to` (same behaviour for every existing caller: the hero moby
  target). Existing bolt / Suck Cannon vacuum tests should be unchanged.
- `cinematic.rs`: two new `CinematicCall` variants and their apply arms (additive; no existing call changes).
- `creature/fx.rs`: new functions only (`part21`, `part44`, `turn_about`, `muzzle_smoke`).
- `creature.rs`: new `hard_cut` only.
- `units/mod.rs`: four new `PORTS` rows (joint lists now loaded for 1075, 574, 452; levels 3, 8, 11 load them).
- `pokitaru_biter.rs`: doc only. Its behaviour now changes on level 11 because the boats update: the hidden boarders
  appear when a boat's state passes 1 (never without 114), and are deleted when it reaches 5 (`skip_to_end`); the
  biters' area path 13 / 28 now moves with a sailing boat. Tests that assumed the boats static (state 0 forever) may
  see the boats in state 1 at their path starts and their propeller manipulators attached.
- Level 3 / 8 / 11 smoke counts (`all_levels_smoke` unported lists) drop 574, 833, 452, 1075.

## 5. Not ported (don't test as working)

- 574's big-head cheat branch (`0x251d70` writing 2.5) and 452's cheat manipulator `0x26dae0` (G-SAV-006).
- The callers of the ported entry points (G-ENM-011): the commando 114 (boats' start / skip / arrived), the 822
  controller (574 `waiting` / `send_away`, its `cmd` 4), the 573 (state 5 group alert, the pod it waits with).
- 574's `0x24fba8` (writes a stack copy: no effect in the game, nothing to port).
- 574 state 0xf has no setter on the disc (ported, unreachable).
- 452's member logic: reachable only with a class 633 (none on the disc) and states 2–4 have no setter (ported).

## 6. In-game QA spots (for the user)

- `RC_LEVEL=11 RC_HERO_AT=495,479.5,226,1.6`: Ratchet drops onto boat #440's deck at its path start. Look: the boat
  bobs gently (0.13 up and down, one cycle every 4 s) and tilts a little; Ratchet bobs with it; the four propellers turn
  slowly; small bubbles behind the propellers on every other tick when the camera is close. Listen: no engine sound
  until it sails (it never sails without the commando 114). The 17 boat biters (near 480, 535) stay hidden.
- `RC_LEVEL=3 RC_HERO_AT=262,196,49,0`: trooper #861 about 15 to the east (path 13). Look: its head and torso turn
  toward Ratchet (±45°), it raises its gun (seq 3), then fires rockets from its gun (a white smoke ring of 24 puffs and
  6 sparks at the muzzle, the rocket with a smoky orange trail homing up / down on Ratchet); the rocket hit costs one
  orb; a rocket blows up (small explosion with sound) on walls. Walk up close: it punches (one orb). Hit it with the
  wrench twice: knocked back with a sound, red flash; the third hit: death tumble with its death sound, then the
  explosion (camera shake, fire, sparks, three body pieces flying, class sound 6) and bolts. Turn behind it within 20:
  it turns around (seq 1 / 9) before acting.
- `RC_LEVEL=3 RC_HERO_AT=250,96,61,1.57`: the troopers standing on the four train-car platforms (#872–#875 on the
  1210s at y≈86): they fire from the cars.
- `RC_LEVEL=8 RC_HERO_AT=190,225,40,-0.9`: runner #341 (near 213, 199) waits; walk toward it: within 24 of its path's
  start it runs off along its path at 10 u/s (seq 3) and disappears at the end. Hit it before: its bolts drop, it
  tumbles and explodes on landing (death explosion, no sound).
- `RC_LEVEL=8 RC_HERO_AT=205,300,40,0` near #343 (path 20, then 52): after its first path it keeps fleeing on path 52;
  stop more than 19 away and it turns to face Ratchet; come within 17 (or lock a Devastator missile on it,
  `RC_GIVE_ITEMS=11`, the Devastator) and it runs again, reversing when Ratchet is ahead of it.

## 7. Results (the test expert fills this in)
