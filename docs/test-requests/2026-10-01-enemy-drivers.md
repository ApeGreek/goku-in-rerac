---
status: open
job: enemy-drivers
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [classes, world]
---

# The drivers of the boats and troopers: Pokitaru's commando 114 (+ gate 65), Kerwan's train 822 (+ lead 845), Kerwan's 573

## 1. Summary

Three level classes that drive the ports of the previous job (G-ENM-011), each ported in full from the decomp and the
disassembly, registered by code identity in `units::PORTS`:

- **U353 114, Pokitaru's commando** (level 11, 1 placed, instance #45) and **U350 65, the gate** he opens (level 11, 1
  placed + 1 created twin). The commando talks (talk block at pvar +0x20, talk slot 1), follows Ratchet through four
  phases, **starts and rides the boats 1075** (`pokitaru_boat::try_start` / `arrived` / `skip_to_end`), and hands over the
  O2 Mask (item 6).
- **U123 822, Kerwan's train** (level 3, 2 placed: #4 the locomotive / controller, #5 passive) and **U124 845, its lead**
  (1 placed; its update is the shared flyer driver). The train calls the trooper port's `waiting` / `send_away` and its
  `cmd` gates the troopers' turn; it releases the infobot (cmd 6).
- **U129 573, Kerwan's charging creatures** (level 3, 87 placed). Its state 5 is what `kerwan_trooper::group_alerted`
  reads; it waits by a trooper (+0x248).

Shared layer extended (plug-and-play for the next consumer):
- `interact::talk_register_at` / `talk_update_at` / `talk_refresh_at` / `poll_scene_end_at`: the talk system on a block
  at any pvar offset (the old functions are the offset-0 wrappers).
- `interact::set_talked(w, npc, v)` (`FUN_0027b438`), `interact::place_after_scene(w, npc, d)` (`FUN_002783a8`),
  `interact::give_item(w, item, equip)` (`GiveItem` 0x275760 from a class: banner from the overlay's banner table, the
  acquired mirror, `GameWrite::GiveItem` applied by the engine with the item tables). `TalkTables::give_banners`.
- `region::graph_init` (level11 0x2873b0) and `region::push_out` (level11 0x287548 / level00 0x261d78).
- `HeroCall::Reload` (0x141401 = 1 without the death count).
- A new consumer: call these; e.g. any class that gives an item calls `interact::give_item`.

## 2. Where it lives

- `crates/rc-game/src/moby_update/classes/units/pokitaru_commando.rs` (`update` 0x2d0fa8, `motion` 0x2d1030, `moving`
  0x2d1340, `brain` 0x2d14b0, `set_state` 0x2d2088, `enemies` 0x2d2460, `reach` 0x2d2340, `ledge` 0x2d25e8, `look`
  0x2d27b0, `gate_update` 0x2cb810, `open_gate` 0x2cb990). Coverage table in the module doc.
- `crates/rc-game/src/moby_update/classes/units/kerwan_train.rs` (`update` 0x292e98, `reset` 0x292578, `drive` 0x292890,
  `stand` 0x292d10, `arrive`, `lead_update` = level03 0x293c78 → `classes::flyer::driver`). Coverage table in the module doc.
- `crates/rc-game/src/moby_update/classes/units/kerwan_hound.rs` (`update` 0x2c5bb8, `hits` 0x2c6a20, `trooper_ready`
  0x2c6ca0). Coverage table in the module doc.
- `crates/rc-game/src/moby_update/interact.rs` (`*_at`, `set_talked`, `place_after_scene`, `give_item`,
  `give_banner_table`, `GameWrite::GiveItem`), `crates/rc-game/src/moby_update/creature/region.rs` (`graph_init`,
  `push_out`), `crates/rc-game/src/moby_update/services.rs` (`HeroCall::Reload`), `crates/rc-engine/src/gameplay.rs`
  (applies `GameWrite::GiveItem`).
- Registration: `crates/rc-game/src/moby_update/classes/units/mod.rs` (rows "U129 573", "U353 114", "U350 65", "U123 822",
  "U124 845").

## 3. Behaviours to verify

Level data used below (extracted level 11 / 3): commando #45 at (500.24, 440.74, 227.0) yaw −1.587, pvars: areas
+0x60 = [0x1f, 0x21, 0x26, 0x26], graphs +0x70 = [0x20, 0x22, 0x27, 0x27], walls +0x80/+0x84/+0x88 = 0x29/0x2a/0x28,
groups by phase [5, 6, 7], [8, 9], [10], [11], cuboids +0xd0.. = 7, 2, 4, 8 / +0xe0.. = 1, 3, 8, 9, boats +0xf0/+0xf4 =
#440 / #441, hazard cuboids 5 / 6, machine 1157 = #445, gate = #43, missions +0x108 = 2, +0x10c = 3, +0x118 = 1, cuboids
+0x110 = 0x1b, +0x114 = 0x1c, talk radius (+0x2c) 1.2 in the data. Train #4 at (285.93, 86.61, 57.97), pvars: cars from
#14 (1210 chain #14..#18), paths A 106 (500 pts) / B 16 / C 102 (stop point index 7: the only w = 1.0), cuboids restart 2,
start 3, top speed 10, lead #925 (845, path 14), arrival cutaway #906 (737), ride cutaway #907 (737), troopers #872..#875.

### B1. 573 init (0x2c5bb8 state 0)
- **Claim:** mode |= 0x1000; home (+0x200) = position; +0x7f = 0x18; state 1; health +0x20 = 1.0; +0x29 = 0; +0x240 = 0;
  meter +0x24 = 1; walker seeded (`SeedJumpPattern`) then J(+0x180): +0x1c0 = 4π·dt², +0x1c4 = π·dt, +0x188 = 2, +0x180
  = 0x38d (int), +0x1a4 = speed(+0x234 = 8)·dt, +0x1b8 |= 0x20, +0x1a8 = 0x40061bba, +0x18c = 2, +0x1bc = 2π·dt², +0x1ac =
  +0x1a4·scale(6); +0x264 = 1.0, +0x244 / +0x26c / +0x260 = 0; bytes +0x58 = 8, +0x5a = 7; one `rand()`: odd → mode |=
  0x8000. Then: trooper link and no path → seq 0 (0 ticks, unless already) and state 2; else seq 1 (0 ticks, unless on it).
- **Setup:** a 573 instance (e.g. #775: no link, no path; #777: trooper #863, no path; #773: trooper and path 72).
- **Trigger:** one update.
- **Expect:** the fields above; #777 → state 2, #775 / #773 → state 1; the rand stream advanced by exactly one `rand()`.
- **Edge cases:** the mirror bit on an odd draw; a trooper link with a path → state 1 (not 2).
- **Suggested method:** unit test on `kerwan_hound::update` with a synthetic world.

### B2. 573 notice / charge / bite / return (states 1, 3, 4, 6, 7)
- **Claim:** state 1 (no path): `randi(14)` drawn each tick; on 0 and Ratchet within the range +0x230 of home (3-D) and
  < 3 in height → state 3, seq 2 over `ticks(10)`, anim speed 0x3f60c7ce (0.878). State 3: `SpringTurn2(atan(t − pos),
  0.02, 0.3, 0.1)`; anim done → +0x23c = `ticks(trunc(randf(30, 90)))`, state 4, seq 3 (`ticks(10)`). State 4:
  `SpringTurn2(…, 0.05, 0.3, 0.2)`, walker step toward 2·(cos, sin) of the yaw; +0x23c out → re-rolled; xy distance <
  r·0.5·40 + r·10 + 1.5 (r = 8·dt: 2.667 + 1.333 + 1.5 = **5.5**) → state 6, seq 5 over `trunc(scale(10))` = 10 ticks; else
  Ratchet farther than the range from home (3-D) or > 3 in height → state 7 (no blend). State 6: the same turn and step
  (out → +0x40); in seq_a 5 the walker speed J+0x24 approaches 0 by 8·dt/40 per tick; at key time 25 (±0.25), < 1 in height,
  facing within 15°, target moby set and xy distance < 2: a hit record to the target (`attack::hit_moby`): damage 1,
  flags 1, pos = target + 0.75 z, dir (0.2 cos yaw, 0.2 sin yaw, 0); then nothing else that tick. Anim done → J+0x24 =
  8·dt, state 7, seq 3 over `ticks(3)`, +0x244 = `ticks(120)`. State 7: turn home (0.02, 0.3, 0.1), step; within 1 (xy) of
  home → state 1, seq 1 (`ticks(3)`); else when +0x244 is 0 and within the bite distance → state 6, seq 5.
- **Setup:** level 3, #775 (285.1, 191.4, 46.3), range 20; Ratchet at 11 units, same height.
- **Trigger:** ticks until it notices; walk away beyond 20 of home.
- **Expect:** the state / seq sequence; Ratchet hurt once per bite (1 damage, small push); no bite in the 120 ticks after
  a bite while it walks home; back home it idles (seq 1).
- **Edge cases:** Ratchet 3+ above → no notice; the alert doubles the range (B4); a decoy target (target layer) is bitten
  only if it has a moby (`t.moby`).
- **Suggested method:** level-harness test (L03, #775) + unit tests of the thresholds.

### B3. 573 run path / wait by a trooper (states 1-with-path, 2, 5)
- **Claim:** with a path (+0x270 ≠ −1) state 1 puts it at the path's point 0 facing point 1 each tick; `randi(14)` = 0 and
  its group alerted (`kerwan_trooper::group_alerted`) → node 0, state 5; otherwise Ratchet in cuboid +0x27c, or within
  +0x274 (3-D) and < 4 in height → node 0, state 5; state 5 gets seq 3 (`ticks(10)`) unless on it. State 5: the nearest
  segment (1000 / 5) raises the node; walk_to(node) (out +0x40); arrived (bits 0x14): next node, or at the last: path −1,
  state 1 (seq 1, ticks(10)), with a trooper link: seq 0, state 2. Then (every tick of 5) `randi(14)`; ≠ 0 → nothing; a live
  trooper link not ready → nothing; else path −1, re-roll timer, state 4, seq 3. State 2: `randi(14)` ≠ 0 → nothing;
  live trooper not ready → nothing; else → state 4 (re-roll, seq 3). Ready (`trooper_ready`, 0x2c6ca0): the 574's seq_a is
  10 and its key time ≥ 10, or its state is 10 / 13, or its +0x284 = −1.
- **Setup:** level 3: #773 (path 72, range 30, cuboid 31, group 56, trooper #860); #777..#779 by trooper #863.
- **Trigger:** Ratchet into cuboid 31 (272.88, 143.22, 50); for the waiters, Ratchet into #863's sight so it plays seq 10.
- **Expect:** #773 runs path 72 to its end (or charges early, 1 in 14 per tick once its trooper is ready / it has none);
  while #773 is in state 5, the 574s of group 56 see `group_alerted` true (their jump-down); #777..#779 charge once #863's
  seq 10 passes key 10.
- **Edge cases:** a dead trooper (state 0xfe/0xfd) releases the 573; a trooper link to a non-574 is never "ready" (keeps
  waiting unless dead); path end with a trooper link → back to 2.
- **Suggested method:** level-harness test on L03 (the group interplay), unit test of `trooper_ready`.

### B4. 573 hits and death (0x2c6a20, state 0x15)
- **Claim:** every tick: lure (+0x38 ≠ 0) or a ready trooper → +0x240 = `ticks(240)` (the lure is not cleared); range
  +0x230 = 2·+0x22c while +0x240 runs, else +0x22c. A hit (mask 0x330000, resolver column 4) with out5 ≠ 1 and not in 0x15:
  its group's `cmd` (+0xbc) = 1 for every member (dead ones too), K flags 9, +0x15d = 0, health −= damage, K gravity
  50·dt², untargetable, the arc `0x250a78(4.5, 2)`: up = √(4·g), t = 2·up/g, speed = 9/t, drag = 9/t²; keys 5 / 12; the
  aim along the hit's push (`knock::aim`); `knock::start(seq 4, 1 tick, frame 0)`; state 0x15; flash colour 0xf0 and
  `flash::start`. Always +0xa4 = 0xff. Any such hit kills (no health test). State 0x15: untargetable; `knock::update`;
  landed / wrapped (0x60) → `death_explosion(0.5, 13, sound −1)`, `BreakFxB` 0x6ce and 0x6cf at pos / rot, `burst_pieces(0x77d,
  1, 0x77d, 1, 3, flag 2)` (one 0x77d at the moby, up to 3 more at volume points with three `rand_angle` each, quiet flag),
  `SetDeathBits(m, 0, −1)` (bolts), `DeleteMoby`; else z < 2 → `SetDeathBits`, `DeleteMoby`.
- **Setup:** any 573; a wrench / blaster hit.
- **Expect:** the flight, the explosion + pieces + bolts, the group cmd 1 on its group's members (e.g. #826..#828's group
  57), the cmd reset to 0 at the tail of each 573's own tick.
- **Edge cases:** a hit while already in 0x15 does nothing; a hit the resolver suppresses (out5 = 1) does nothing; falling
  below z 2 without landing → no explosion, but the bolts.
- **Suggested method:** unit test with a hit record; level-harness for the pieces.

### B5. 573 tail and unreachable states
- **Claim:** drawn and within 32 (3-D) of the camera → shadow probe; flash update; `cmd` = 0. States 8 (knock flight
  → 4 / delete below 2) and 9 (`react::carried`, done → state 0, suck record +0xc8 = 0) exist but have no setter on level 3
  (default reaction table, 573's slot w8 = 0x1e40fc).
- **Suggested method:** unit test (force state 8 / 9).

### B6. Commando init / resume (0x2d14b0 state 0)
- **Claim:** `NpcTalkRegister(m, +0x20)`; update distance 0xff; `region::graph_init` on graphs 0x20 / 0x22 / 0x27 with walls
  0x1f / 0x21 / 0x26 (masks in the points' w words); +0x130 / +0x140 zeroed. O2 Mask not acquired: mission 3 done → phase 3,
  at cuboid 9 (616.93, 568.4, 233.03, yaw −2.402), gate opened (B11), state 0xe (talk radius 10, node 2), both boats
  `skip_to_end`. Acquired and mission 3 done → both boats skipped, gate opened, deleted. Mission 2 done and Ratchet within
  12 (3-D) of cuboid 3 (435.15, 642.38, 225.27) → phase 1 at cuboid 3, state 6 (seq 3, speed 5·dt), cuboid 0x1c moved
  500 down, boat #440 skipped. Else: cuboids 0x1b and 0x1c 500 down, phase 0, talk radius 5, state 1 (seq 0).
- **Setup:** level 11 fresh / with missions 2 or 3 done / with item 6 acquired.
- **Expect:** the branch's writes; boats in state 5 at their path's second-to-last point when skipped.
- **Suggested method:** unit test with a synthetic mission table; level-harness for the boats.

### B7. Commando talk and follow (states 1..5)
- **Claim:** state 1 at node 0: Ratchet on the ground (air ticks 0) and `talk_update_at(+0x20)` true (△ in range 2·5 = 10,
  facing rules) → scene placement 2.5 in front facing back, state 2; other node: Ratchet within 6 (xy) → 3. State 2: game mode
  ≠ 2 → 3. 3: Ratchet beyond 6 (xy), reachable (inside the area polygon and `LineOfSightTest(0.2, walls, graph)` writes the
  way point to +0x120) and enemies = 1 → 4 (seq 2, speed 1.5·dt). 4: within 3 / unreachable / enemy within 10 (3-D) → 3;
  beyond 8 → 5 (seq 3, 5·dt). 5: same, within 4 → 4. 3..5 with no live enemy in the phase's groups (1246 boarders held /
  dying / waiting don't count) → 6.
- **Motion:** in 4, 5, 6, 8, 10 the yaw turns to +0x120 (`turn_toward` 4π·dt², 8π·dt², 4π·dt) and the velocity is
  (cos, sin)·speed with vz = last move z − 20·dt² (≤ 0); every tick the walker move with collision (up 0.5, r 0.3, flags
  0x10); in 4..6 (and 7 with +0x15c = 3) pushed 0.3 out of the area's walls (and +0x80..+0x88 in phase 2, +0x80 / +0x84 in
  phase 3).
- **Setup:** RC_LEVEL=11, Ratchet at (500.24, 437, 227.2) facing +y.
- **Expect:** the prompt / scene; then he walks / runs behind Ratchet inside the beach polygon, never through its walls.
- **Edge cases:** Ratchet airborne → no talk; the ledge jump (B9).
- **Suggested method:** level-harness test + QA.

### B8. Commando boats (states 6..10)
- **Claim:** 6: walk to the phase's point (cuboid +0xe0[phase]: 1, 3, 8, 9) via `reach`; within 1 (xy): phase 0 → +0x120 =
  boat #440 pos + 2·unit(row 1) + 1·unit(row 2), +0x15c = 8, state 7; phase 1 → boat #441 with −2·row 1, cuboid 0x1b back up
  500, 8, 7; phase 2 → 0xb; phase 3 → cuboid 0x1c back up, 0xc. 7 (jump, seq 4): at key 9 velocity = (target − pos) /
  ticks(60), vz = `lob_up(xy speed, −10.8·dt², pos, target)`, anim speed 2/3; moving: vz −= 10.8·dt² per tick; key 29 →
  velocity 0, anim speed 1; anim done → +0x15c. 8 (seq 3): +0x120 = boat + rows·(3.2, 0, 0.95); within 0.5 → 9 (seq 0). 9:
  placed at the helm every tick (`from_local`, local (3.2, 0, 0.95), rot 0); `try_start(boat)` → seq 5 (`ticks(20)`) the
  tick it starts (Ratchet must stand on the boat, air ticks 0); boat state 5 → 10 (seq 3, +0x120 = boat + rows·(−3, 0, 1)).
  10: within 0.5 → phase + 1, +0x120 = cuboid +0xd0[phase] centre (phase 1: cuboid 2 (457.61, 529.88, 233)), +0x15c 4, 7.
- **Expect:** boat #440 (then #441) sails with the commando at its helm; the 1246 boarders show up and board (boat
  state > 1); at the end he jumps off to the next area.
- **Edge cases:** Ratchet not on the boat → the commando waits at the helm; skipped boats (B6) are already in state 5.
- **Suggested method:** level-harness test on L11 (hero placed on the boat), QA.

### B9. Commando ledge jump (0x2d25e8)
- **Claim:** in 4..6, the commando inside hazard cuboid 5 or 6: probe = pos + 3·unit(row 0), clamped to 4 from him; inside
  the area polygon; ground below (probe + 5 up, `GroundHeight(0.5, …, 0x20)`) more than 0.5 off his z; a collision line
  from 0.15 above his feet to 0.15 above that ground hits (flags 2) → +0x120 = (probe xy, ground z), +0x128 = ground below
  it, +0x15c = 3, state 7 (then the jump and back to 3).
- **Suggested method:** unit test of `ledge` on synthetic collision; QA at cuboids 5 / 6 (454.81, 547.91, 233), (465.73,
  565.54, 232.05).

### B10. Commando end (0xb..0xf) and reward
- **Claim:** 0xb: 1157 (#445) in state 5 → phase + 1, 3. 0xc: talk radius 255 each tick; `talk_update_at` → `SetMissionDone(3)`,
  global flag 84 = 1, talked word of slot = 2, at cuboid 9 (pos + Euler), the scene end place 2 then 2.5 in front facing back,
  state 0xd. 0xd: (motion) scene id 1 at its tick ticks(2000) → gate opened; game mode ≠ 2 → gate opened (if still closed),
  0xe. 0xe: mission 1 done, Ratchet not in state 0x32, `talk_update_at` → place 2.5, 0xf. 0xf: game mode ≠ 2 →
  `give_item(6, equip)`: banner = give-banner table entry 6 (level 11: 0x2b05 = 11013) shown `ticks(300)`, acquired[6] = 1,
  `GameWrite::GiveItem{6, true}` (the engine: owned, ammo, quick select / hand request), `GameWrite::Save`-equivalent
  (`cinematic::save`), `help.request(11000, 0x3a)`, deleted.
- **Edge cases:** item 6 already owned → banner entry 6 + 0x25 (−1 on level 11: no banner).
- **Suggested method:** unit test (force phase 3 / state 0xe with a talk table), engine test for `GiveItem`.

### B11. Gate 65 (0x2cb810 / 0x2cb990)
- **Claim:** state 0: → 1, +0x04 = yaw; unmirrored: `CreateMoby(65)` twin with draw / update distance copied, mode = this |
  0x8000, same pos / rot, its +0x04 = yaw; +0x08 = twin index + 1. Open (commando): only in state 1 → global flag 90 = 1,
  state 2, the twin state 2. State 2: yaw turns to +0x04 + 0.2077 (mirrored −) with `turn_toward(20°·dt², 20°·dt², 45°·dt)`,
  velocity +0x00; returned 0 → 3.
- **Setup:** level 11 #43 (592.27, 579.69, 233.06, yaw −0.847).
- **Suggested method:** unit test.

### B12. Commando look-at (0x2d27b0)
- **Claim:** states 0..5, 0xb, 0xc, 0xe, 0x10: d = Ratchet − (pos + 1 z); head target pitch = clamp(−atan(|d.xy|, d.z),
  −30°, 15°), head yaw = 0.7·clamp(yaw diff, ±70°), torso yaw = 0.3·that; `manip::look(0.02, 0.3)` on +0x160 (list 0) and
  +0x1e0 (list 1) every tick (targets reset to 0 after each call).
- **Suggested method:** unit test on the record fields.

### B13. Train init and parking (822 cmd 0..3)
- **Claim:** #5 (path A −1): every tick +0x3e = 0xd, +0x28 = 4, +0x20 = 0, +0x24 = 0; nothing else. #4 cmd 0: same header;
  +0xbc = xy distance to its joint 0; +0xdc = 20; +0xc0 = 10·dt, +0xc8 = same; +0xd4 = 7 (path 102's w = 1 point); cmd 1;
  block flags +0x9c = 3; lead #925 mode |= 6 (never runs its own update). cmd 1: `reset(0, A[0])`, lead path = 106, lead
  speed = +0xc0, one driver tick, cmd 2. cmd 2: `reset(heading B0→B1, B[0])`, cmd 3, lead path 106, lead index 1, +0xc8
  = 0. cmd 3: `stand` each tick (+0x72 = 0x40, cars' carries); Ratchet in cuboid 3 (198.97, 362.34, 81), movement group
  0/1/9/0xc → cmd 4: map predicate 1, `music_request(4, 7)`, talked word (slot 3) = 1, #907 cmd 1 aimed at Ratchet
  (P[6] yaw / P[5] −pitch from the camera), #907 hold `ticks(860)`, horn timer `ticks(30)`, voice −1.
- **Reset claim:** the lead at p; the locomotive 20 behind (−heading), rot.y = 0, rot.z = heading + π, its riders carried;
  spring velocities 0; each car: old pose recorded, at 10 + its +0xc4 behind the previous joint (first: 0xbc behind the
  locomotive), at the locomotive's z, yaw toward the joint point, matrix rebuilt, mode |= 6, its riders carried; the lead's
  state 0, mode |= 6.
- **Suggested method:** level-harness (L03) checking positions after 3 ticks; unit tests on `reset`.

### B14. Train ride (cmd 4) and fall-off
- **Claim:** +0xc8 += (+0xc0/5)·dt up to +0xc0 (5 s to top speed); map predicate 1; `drive`: lead speed = max(0, 20 −
  xy distance), driver tick, the locomotive moves +0xc8 along (yaw + π, pitch), springs (k = +0xc8·60/+0xc0); each car pulled
  to 10 (xy) behind the previous joint 1, less its +0xc4; `CarryRiders` on all (Ratchet / troopers on the cars ride).
  Sounds: class sound 1 loop (flags 4) kept alive; horn class sound 0 when the timer runs out, then `rand_range(ticks(900),
  ticks(400))` (900..1398). Ratchet airborne and not in group 3: +0x10c counts; > 60 and 10 below the nearest point of the
  lead's path → loop released, voice −1, `FadeToBlack(16)`, `HeroCall::Reload` (fell_out = 1, no death counted). On the
  ground: +0x10c = 0. #907 re-aimed while its cmd ≠ 0. Ratchet's ground moby a 822 / 1210 → that moby's talked word = 3.
- **Suggested method:** level-harness (Ratchet placed on car #14 after starting the ride); QA.

### B15. Train arrival (HeroOnMoby → cmd 5 → 6) and restart
- **Claim:** Ratchet on the locomotive (#4) while path C and #906 are set and #907's cmd is 0: the first live trooper link
  (#872..#875) not in state 0xe → state 0xe (one only); #906 cmd 1 aimed at C[0]; its hero cuboid (P[10] = cuboid 5)
  centre = C[0] − 24·dir(C0→C1) at C[0].z + 2.7, Euler (0, 0, heading); #906 hold `ticks(10000)`; `reset(heading, C[0])`;
  lead path 102, index 1, t 1.0; +0xc0 doubled → lead speed; driver tick; cmd 5; `music_request(0, 8)`. cmd 5: drive,
  predicate, sounds; once the lead's index ≥ 7: +0xdc = 0, +0xc8 −= (+0xc0/1.9)·dt to 0; at 0: #906 cmd 0, cmd 6, +0xc0
  halved, loop released; #906 re-aimed at Ratchet every tick of 5. cmd 6: Ratchet in cuboid 2 (302, 244, 34.5) → 574s in
  state 0x10 → 0x11 (none exist), +0xc8 = +0xc0, +0xdc = 20, cmd 1; else `stand`. The infobot #908 riding #4 leaves on
  cmd 6 (`classes::infobot`).
- **Suggested method:** level-harness + QA.

### B16. 845 lead
- **Claim:** registered as level03 0x293c78 → `flyer::driver`; it never self-updates (mode bit 2 set by the train's cmd 0
  before its slot is reached); driven only by the train.
- **Suggested method:** level-harness: 845 runs exactly one driver tick per train tick in cmds 1, 4, 5 (+1 in the arrival tick).

### B17. Shared: talk at an offset, set_talked, place_after_scene, give_item, banner table
- **Claim:** `*_at(base)` read / write every talk field at `base + talk::X`; the offset-0 wrappers behave exactly as before
  (talking NPC 774, Novalis). `set_talked` writes the mirror and `GameWrite::Talked(slot, v)`; no slot → nothing.
  `place_after_scene(d)` = pos + d·(cos yaw, sin yaw), yaw + π. `give_banner_table` finds level 11's table at 0x1b0d40 (entry
  6 = 11013); other overlays likewise (74 entries); `give_item` banner `ticks(300)` unless −1.
- **Suggested method:** unit tests on synthetic worlds; a loader test that level 11's table entry 6 is 0x2b05.

### B18. Shared: region::graph_init / push_out
- **Claim:** graph_init: every w = 0 then mutual bits for each pair no wall separates (mod 32). push_out (r): per edge with
  either w ≠ 0, cross-distance ≤ r: over the segment → r from the line; beyond an end → r from the edge's first point (only
  if closer than r); z and w of the input kept; None when no edge moved it.
- **Suggested method:** unit tests on synthetic paths (a square wall, points inside / near corners).

## 4. Shared code touched (regression risk)

- `interact.rs`: `talk_register` / `talk_refresh` / `talk_update` / `poll_scene_end` now wrap `_at(…, 0)`: the talking
  NPCs (774, item offers, mission NPC, infobot talk flags) must behave identically. `TalkTables` gained `give_banners`
  (loaded from the overlay; a missing pattern gives an empty table: no banner). `GameWrite::GiveItem` added (apply_writes
  ignores it; the engine applies it).
- `region.rs`: two new functions only (no change to the existing ones).
- `services.rs`: `HeroCall::Reload` (new variant; existing calls unchanged).
- `rc-engine/src/gameplay.rs`: the write loop applies `GiveItem` with the item tables read from the disc.
- `units/mod.rs`: five new port rows; class 845 on level 3 now runs `flyer::driver` (first tick only), class 822 / 573 / 114
  / 65 now run.
- Docs only: `pokitaru_boat.rs`, `kerwan_trooper.rs`, `pokitaru_biter.rs` module docs (caller notes).

## 5. Not ported (don't test as working)

- The big-head cheats: 573's `0x251d00(2.1, m, 0, +0x280)` (0x15edb7) and the commando's `0x2d0ec0` / head scale 2.75
  (0x15edb0): G-SAV-006.
- The flyer driver's level-3 / 9 combat block for the lead 845 (hit 0x210000, the hide timer): G-CLS-015.
- Pokitaru's cutaway machine 1157 (#445): the commando waits in phase 2 (state 0xb) forever without it: G-ENM-011.
- 573's suck record sequence table store (+0xd0): n/a (default reaction table).

## 6. In-game QA spots (for the user)

- `RC_LEVEL=11 RC_HERO_AT=500.2,437,227.2,1.57`: the commando in front; △ "talk" → scene; then clear the beach enemies
  (groups 5–7); watch him walk to the boat at (495, 480), jump on (seq 4 jump), walk to the helm; stand on the boat: it sets
  off (engine loop, wake, the boarders jumping aboard), he holds the helm (seq 5); at the end he steps off and jumps ashore.
- `RC_LEVEL=11 RC_HERO_AT=592,575,233.5,0` with the O2 Mask path: the gate at (592.27, 579.69) swinging ±12° open during
  the last scene (needs 1157: not reachable yet).
- `RC_LEVEL=3 RC_HERO_AT=285,180,46.5,1.57`: the 573s #775 / #776 / #780 / #829 notice (anim speed change), turn, charge,
  bite at key 25 (one hurt per bite), walk home; a hit: the flight, small explosion, two pieces + up to four 0x77d bits, bolts.
- `RC_LEVEL=3 RC_HERO_AT=226,190,46.5,1.57`: the trooper #863 with three 573s (#777–#779) beside it: they wait until the
  trooper waves (its seq 10) then charge.
- `RC_LEVEL=3 RC_HERO_AT=199,362,82,0` (inside the train's start cuboid 3): the train (parked from (266, 441) toward
  (348, 448)) departs: music track 4, horn, engine loop, the ride cutaway camera on Ratchet for ~14 s; ride the cars; step
  onto the locomotive: one trooper vanishes, the arrival cutaway, the train jumps to path C and brakes at its 8th point,
  the infobot leaves. Fall off the train: fade and reload.
