---
status: open
job: classswap-spawners
date: 2026-10-01
commit: 58c8b71
areas: [classes, world]
---

# The run-time class-swap service (G-CLS-031) and the five round-5 spawner units (432/1052, 1066, 899/898, 1885/1886, 1401)

## 1. Summary

* **Class swap (G-CLS-031)**: `moby_update::class_swap::swap(w, id, class)` — the class-dependent part of
  `InitMobyInstance` applied to a live moby, as level04 0x2c6858 writes it inline (there is no engine function). A
  registry `class_swap::PORTS` (one row per swapping class port, matched by code identity through `LevelPorts`)
  feeds `class_swap::targets(ports, class)`, which the renderer uses at level load to build the models, palette room
  and metal entities of the classes a **placed** moby may become; at run time `drive_statics` switches the static
  instance's entities / `MobyAnim` entry to the live class (`MobyOcclusion::set_class`, `MobyAnim::set_class`).
  Plug-and-play: a new swapping class adds a `PORTS` row and calls `swap`.
* **Units ported** (all `units::PORTS` rows, so every level whose class table names the same code runs them):

| unit | classes (level: placed) | module | reference |
|---|---|---|---|
| U154 | 432 ↔ 1052 crank followers (04: 4) | `units::eudora_crank_follower` | level04 0x2c6858 |
| U225 | 1066 creature wakers (06: 6) | `units::blarg_waker` | level06 0x2fda30, wake 0x2e9d10 |
| U140 | 899 path-mover lines (03: 4) + created movers 898 | `units::kerwan_path_spawner` | level03 0x2db280, 0x2db198, 0x2db020 |
| U319 | 1885 pod launchers (09: 4 created, 13: 4) + created pods 1886 + the hatch | `units::pod_launcher` | level09 0x30ab80, 0x30a9e8, 0x30aae0, 0x30b218, 0x30b3b0, 0x30b350, 0x30a778 (level13 copies 0x30d550 / 0x30dd80 / 0x30d148 / 0x30d3b8, identical masked code) |
| U521 | 1401 mine drones (16: 14) + created race mines 933 (creation only) | `units::kalebo_mine_drone` | level16 0x2e37a0, 0x2e3e20, 0x2e3fa0, 0x2e4110, 0x2e4190, 0x2de298, 0x2de3a8 |

* **System-or-not**: no spawner system — each "spawner" creates or wakes its own family's mobys with its own code.
  Shared now: the target search's cuboid-list form `creature::target::acquire_with` (first consumer 1885).

## 2. Where it lives

* `crates/rc-game/src/moby_update/class_swap.rs`: `swap`, `update_anim_state` (`update_moby_animation_state` L01
  0x263718), `PORTS`, `targets`. Module doc = coverage table + the disc-wide caller scan.
* `crates/rc-game/src/moby_update/classes/units/{eudora_crank_follower,blarg_waker,kerwan_path_spawner,pod_launcher,kalebo_mine_drone}.rs`
  — each module doc holds the coverage table (address | what | port).
* `crates/rc-game/src/moby_update/creature/target.rs`: `acquire_with` (0x274df8 in full), `acquire_in` now delegates.
* `crates/rc-engine/src/moby_render.rs`: `spawn_mobys(…, swaps, …)`, `InstanceDraws::other`, `OtherClass`,
  `MobyOcclusion::set_class`, `pending_hide`; `crates/rc-engine/src/moby_anim.rs`: `MobyAnim::set_class`;
  `crates/rc-engine/src/gameplay.rs` `drive_statics` (the per-tick class check).

## 3. Behaviours to verify

### B1. Class swap writes exactly the init's class fields
- **Claim** (level04 0x2c690c..0x2c6964): `swap(w, id, c)` sets `o_class = c`, `class_slot = ClassInfo(c).slot`,
  `b71 = 0xff`, `has_class = !no_header`, `scale = ClassInfo(c).scale` (class scale alone: an instance scale of 2.0
  is lost), `b72 = ClassInfo(c).b0e`, `has_collision = ClassInfo(c).has_collision`, then `react::sphere_lerp`
  (bounding sphere from c's sequences, rows kept, grid re-registered, `uid_hi` low half + 1).
- **Not written**: `update_fn`, pvars, state, mode bits (glow 0x10, 0x400 stay), `glow`, `b73`, `b7f`, `b7d`, anim
  keys / speed / rate, position, rows.
- **Setup**: a `ClassTable` with classes A (slot 3, scale 1, collision false, b0e 0x10, sequences with loop sounds)
  and B (slot 7, scale 0.5, collision true, b0e 0x40, other sphere); a moby of A with instance scale 2 (scale 2),
  mode 0x10|0x400, b7d = 5.
- **Expect**: after `swap(B)` the fields above; `bsphere` = B's sequence sphere · 0.5 placed at the position;
  `rows` unchanged; `update_fn` unchanged (the scheduler still dispatches the same port).
- **Edge cases**: class not in the table → only `o_class` changes; a slot without a header (`no_header`) → `o_class`,
  `class_slot`, `b71`, `has_class = false` only; swapping back and forth twice gives the same fields as one swap.
- **Method**: unit test on `class_swap::swap` with a synthetic `ClassTable`.

### B2. `update_anim_state` (0x263718)
- **Claim**: sequence A (`anim.seq_a`, +0x52) = 0xff → `b7c = 0xff`, `trigger_count = 0`; else `b7c` / `trigger_count`
  = sequence A's loop sound / trigger count of the new class; a sequence the class lacks leaves both [L].
- **Edge**: seq_a ≠ seq_b (mid-blend) uses A, not B.
- **Method**: unit test.

### B3. `class_swap::targets` by code identity
- **Claim**: on level 04, `targets(ports, 432) == [1052]`, `targets(ports, 1052) == [432]`; any class whose level
  table entry is not 0x2c6858's code → `[]` (e.g. 432 on a level where 432 runs other code, or `LevelPorts::by_class_number`).
- **Method**: unit test with `LevelPorts::from_overlays` on the extracted level 04 overlay (the existing level-harness
  way), plus `by_class_number()`.

### B4. Crank followers 432 / 1052 (level04 0x2c6858)
- **Claim**: no pvars → `DeleteMoby`. State 0: `MobyAnimBlend(1, 0, ticks(300))`, state 1, speed 0, pvar+4 = −1;
  a 432 swaps to 1052 (a 1052 placed as such does not). State 1: crank = pvar+0 (moby index; −1 or class ≠ 280 →
  nothing at all, `t` not written); `t` (+0x54) = crank progress (crank pvar+0) **before** comparing; progress = old t
  → voice released (when alive), slot −1; progress ≠ old t and not 0/1 → if the voice slot is not alive, slot =
  `PlayClassSound(0, flags 4)` (the live class's table: 1052's); progress 0 or 1 → voice released, slot −1,
  `PlayClassSound(1, 0)` (return dropped); then only a 1052 with progress exactly 1.0 swaps to 432 and goes to state 2.
  State 2: every tick the voice release only. Other states: nothing.
- **Setup**: level 04 harness (or synthetic table) with a crank 280 moby and a 432 linked to it.
- **Expect**: tick 0: class 1052, state 1, anim seq_b 1, speed 0; while the crank's progress rises 0 → 1: t follows it,
  one loop voice (sound 0, flags 4) started once and kept; when the progress stops changing the voice is released;
  at progress 1.0: sound 1 once, class 432, state 2; progress back to 0.5 afterwards: nothing (state 2).
- **Edge cases**: progress reaching 0.0 (not 1) → sound 1 but no swap and stays in state 1; progress 0.9999 → no
  swap; crank index pointing at a deleted crank of class 280 still reads it (the game only checks the class).
- **Method**: unit test with a fake sound sink (the existing `SoundSink` test sinks) + QA in game.

### B5. Renderer follows the live class of a placed moby
- **Claim**: at load, a placed 432 on level 04 gets palette room for max(432, 1052) slots, 1052's parts built
  (`used`), 1052's metal entities spawned hidden into `InstanceDraws::other`; when its moby's `o_class` becomes 1052,
  `drive_statics` → `MobyOcclusion::set_class(ii, ci(1052))` swaps class / sphere / LOD switch (`lod_trans` = 1052's
  class +0x0e) / groups / metal, queues the 432 entities for hiding, clears `shown[ii]` (the next
  `update_moby_occlusion` shows 1052's group, spawned on first use) and `MobyAnim::set_class` re-points the pose.
  Back to 432: the reverse, reusing the stashed 432 entities.
- **Edge**: `set_class` with the shown class or a class the instance was not prepared for → false, nothing changes;
  RC_PLAY=0 → no swap targets (no extra meshes).
- **Method**: visual check in game (B4's spots) and a renderer unit test of `set_class` if the harness allows ECS.

### B6. Blarg's wakers 1066 (level06 0x2fda30)
- **Claim**: pvars +0 group, +4 cuboid, +8 timer. State 0: cuboid < 0 → nothing; Ratchet's position in the cuboid →
  state 1, and seq B ≠ 1 → blend (1, 0, **5 ticks** raw). State 1: anim flags bit 2 clear → nothing; else state =
  group < 0 ? 3 : 2, seq B ≠ 2 → blend (2, 0, 5). State 2: `FastDecTimer(+8)` running → nothing; else walk the group
  list (all members, dead too) and wake the first in state 0xf; none → state 3; else timer = `ticks(20)`.
- **Wake (0x2e9d10)**: state ≠ 0xf → refused. Else position (4 words) = the waker's, state 6, `has_collision` = class,
  mode `& 0xffbe | 0x1000`, Euler = 0 then z = `rand_angle()`, rows 0..2 from the Euler (row 3 untouched),
  K = pvar+0x120: drag 0, flags 5, +0x3d = 3, up = 2·dt, gravity = 20·dt², speed = 2·dt; `knock::start(rand_angle(),
  seq 2, 1 tick, frame 0)`; K+8 (vz) negated.
- **Draws**: per wake exactly 2 `rand_angle` (yaw, then the throw direction) plus whatever `knock::start` draws (none).
- **Edge cases**: two dormant members → one per 20 ticks; group −1 → state 3 directly from state 1.
- **Method**: unit test with synthetic members forced to state 0xf (827 is unported, so in game none is ever 0xf).

### B7. Kerwan's path-mover lines 899 and movers 898 (level03 0x2db280 / 0x2db198 / 0x2db020)
- **Claim (899)**: path −1 → delete. Init: the units global word 0x161bc0 = `dt·3`; for i < count − 2 point i's w =
  3-D distance to point i+1 **written into `Services::splines`**; timer 0, state 1, slot −1; movers at f = 2.9, 5.8, …
  while f < count·p[0].w; update distance 0x60. Every tick (also the init tick): timer out → timer = `(int)(2.9/speed)`
  (58 or 57 on NTSC: check the f32 result) and a mover at 0; voice not alive → slot = `PlayClassSoundAs(0, 4, m, 898)`.
- **Claim (spawn)**: `CreateMoby(898)`; state 0, update 0x80, draw 0x40, visible 1, mode |= 0x20; Ratchet's light
  word / ambient; position = point 0 (4 words); Euler 0 with z = `atan2(last − p0)`; pvar +0x64 = f, +0x60 = path index,
  +0x08 = 0x20; `MobyBuildMatrix`.
- **Claim (898)**: d += speed word; i = trunc(d / p0.w); i ≥ count − 2 → delete; else position = p[i] + (p[i+1] −
  p[i])·frac (w = p[i+1].w); `CarryRiders(block 0x20, pos − old (w = new pos w), rot, rot)` (rotation part 0).
- **Expect**: a path of 10 points 1 unit apart: 4 movers at load-init time (2.9, 5.8, …) plus one at 0; a mover
  advances 0.05/tick; it is deleted when i reaches 8; `platform_delta` of a mover = its step; Ratchet standing on one
  is carried (hero platform code).
- **Edge cases**: two 899s on the same path both rewrite the w's (same values); path index out of range → nothing.
- **Method**: unit test (synthetic splines) + QA in game.

### B8. Pod launcher hit handler (level09 0x30a9e8)
- **Claim**: `get_hit(mask 0xc30000, keep false)`; `damage::resolve(rec 0x20, flags 0, col 4)`; out5 ≥ 2: health −=
  the resolved record's damage; > 0 → red +0x67 = 90 and `flash::start(0x60)`; ≤ 0 → `SetDeathBits(m, 0, −1)` (its
  bolts), state 2, red 0x78, `flash::start`. Always `hit_slot = 0xff`, `flash::update(0x60)`. In state 1 the handler
  runs **twice** a tick (the second sees no hit but updates the flash again).
- **Method**: unit test with a delivered hit (`World::deliver_hit`), health 1.0 (the data's value: one hit kills).

### B9. Pod launcher init and fire (level09 0x30ab80, 0x30aae0)
- **Claim**: state 0 deletes every live member of its group (`group_first(Alive)` loop), → 1. State 1: fire timer
  (+0x8e, s16) running → nothing; `randi(7)` = 0 → `acquire_with(range +0x80, [+0x84] when ≠ −1 (a list of one:
  excludes nothing), region = path +0x88 when ≠ −1 and non-empty)` written to +0x100 (+0x140 = moby index + 1,
  +0x144 kind); else the stored target moby (gone → kind 2) gives its position. Kind 2 → nothing. Count: +0x91 =
  live group members + live pods (class 1886); slot = the **last** free of 20; max (+0x90) ≤ count or no slot →
  nothing.
- **Aim**: +0x92 = 0: k = `rand_range(0,3)`, first cuboid ≠ −1 of [(j+k)%4]; none → `disc_point(max(range, 3))` (one
  `rand_angle`, one raw `rand`) + position. +0x92 ≠ 0: the cuboid whose centre is nearest Ratchet (< 99840); none →
  (target − position)·0.5, clamped to 3 when range > 3, set to length 3 when shorter. A cuboid: centre + row0·`randf(−1,1)`
  + row1·`randf(−1,1)`. Ground height (0.5 up) ≠ 0 → z. v = (p − pos) scaled to |p − pos|·dt·0.6; start = joint 0 +
  unit(v)·0.05 jittered ±0.025 (3 `randf`); v.z = `lob_up(s, −9.8·dt², start, p)`; `spawn_pod(start, v, +0x93, slot)`;
  pods[slot] = pod; timer = `ticks(+0x8c)`; count + 1.
- **Data** (level 09 inst 903): range 12, cuboid 4, path −1, interval 30, max 4, aim 1, grow 1, group 72 (four 193s),
  target cuboids 2, 3.
- **Edge cases**: all four cuboids −1 in both aim modes; a target that dies between searches (kind 2 then nothing);
  20 live pods (no slot).
- **Method**: unit test (synthetic volumes / groups) + QA.

### B10. Pod spawn and flight (0x30b218, 0x30b3b0 state 0)
- **Claim (spawn)**: `CreateMoby(1886)`; launcher ref, position = start, velocity pvar+0 = v, Euler x then y =
  `randf(−π, π)`, hatch 0, timer `ticks(500)`, update 0xff, draw 0x40, visible, state 0, scale = class·0.3, slot,
  grow, cooldown 0, matrix.
- **Claim (flight)**: cooldown byte −1 a tick; scale += (class − scale)·0.05; position += velocity; collision off for
  pod and launcher during the probes; line old → position + unit(v)·0.2 (flags 4, ignore the pod): hit → position +=
  unit(point − end)·0.215; world (no moby): velocity reflected, and on ground flatter than 40° with no surface id
  (−1): ·0.5, |v| < 0.025 → state 1; a pod: v.z ·= `randf(0.9, 1.1)`; other moby: reflected, ·1.2 on such ground;
  knock sound 0 when the cooldown is 0, then cooldown 12. Sphere r 0.2 at the position: hit → position = pushed
  centre + unit(point − pushed centre)·0.015, then the same bounce. Separation: other live pods of the launcher within
  1.8 (xy) push this one out by d·(1.8 − |d|)·0.5. Life timer out → state 2. Euler x/y = wrap(v.x / v.y); collision
  back on both; v.z −= 9.8·dt²; Euler x += 0.01, y += 0.02.
- **Every state tail**: z < 5 or > 500 → the pod's delete (the launcher's slot cleared); else x, y, z clamped to [5, 1018].
- **Edge cases**: a hit on a sloped wall (> 40°: reflected but not damped); sound at most every 12 ticks; the pod
  touching its launcher (ignored only through collision off).
- **Method**: level-harness test on level 09 (real collision) and QA.

### B11. Pod rest, hatch and shrink (0x30b3b0 states 1/2, 0x30a778, 0x30b350)
- **Claim (state 1)**: z = ground(0.5) + 0.2, w 0; hatch = `hatch(launcher, pos)`; state 2.
- **Claim (hatch)**: first *dead* member of the launcher's group (none → nothing); launcher +0x91 + 1; member:
  position (w 0), state 0, cmd 0, mode = class mode bits (+glow 0x10, +0x400 with class +0x0f), scale = class (·0.1
  with grow), update / draw 0xff, visible, occlusion 0x7f80, b71 = b72 = hit_slot = 0xff; hard cut (0, 0); suck record
  +0x6c / +0x94 / +0x68 = 0; bolts b4 = min(own, launcher's), ≥ 1, launcher's −= it (u16; negative → 1); collision on;
  Ratchet's light; matrix; damage record +0x30 = +0x34 = 0, health = (f32) its s16 +4.
- **Claim (state 2)**: pod scale ·= (1 − 0.05); hatchling (grow) scale += (class − scale)·0.05; pod scale < class·0.05
  → 0.0001 and, with no hatchling / it deleted / no grow / it within 0.01 of its class scale, the pod is deleted and
  its launcher slot cleared.
- **Expect** (level 09): a resting pod shrinks away while a 193 pack biter grows in from a tenth of its size at the
  pod and starts its own state machine; the launcher's bolts are shared out to the biters.
- **Method**: level-harness test on level 09 (group 72 / 71) and QA.

### B12. Pod launcher death (state 2)
- **Claim**: `SpawnBeamExplosion(0, 0, 8, 4, 18, 1, 15, m, …)` at the **launcher's position** (its param_9 = &+0x70
  is never read), streaks 30, spark pairs 8, puffs 20, class sound 1, shake, 1 debris; `BreakFxB` pieces 1733, 1734,
  1735 at the position / rotation with random velocity, spin and the class sphere; `DeleteMoby`.
- **Method**: unit test on the spawn counters (`FxStats`) and created piece classes; QA (one wrench hit kills it).

### B13. Target search cuboid list (`target::acquire_with`)
- **Claim**: with `n ≥ 1` cuboids the region is ignored and Ratchet / decoys inside cuboids[0 .. n−1] are refused
  (the last cuboid never tested; n = 1 refuses nothing); with n = 0 → identical to `acquire_in` (region test).
- **Regression**: every existing caller of `acquire_in` (amoeboid, hop gunner, orxon brawler, …) must behave as before.
- **Method**: unit test (synthetic volumes) + the existing target tests.

### B14. Mine drone 1401 (level16 0x2e37a0 and helpers)
- **Claim (top)**: anim wrapped with A = B = 1 → blend (4, 0, ticks 5); Ratchet's contact moby (`Hero::cap_moby`) =
  this one and neither sequence 1 → blend (1, 0, ticks 5). A hit (mask 0x210000) on it or on its carried mine, state ≠
  6: K gravity 0.008, flags 1, up 12·dt, drag 0.0005, speed 24·dt, +0x3d 0; `knock::start(atan(pos − Ratchet), seq 1,
  1 tick)`; red 120 (no flash start); `SetDeathBits(0, −1)`; state 6. `hit_slot = 0xff`.
- **States**: 0: mode 2 → state 1 (turn vel 0, hard cut 4, mine 0); else missing path A / (mode 0) path B / cuboid →
  delete; else setup (w lengths for i < count − 1 on A, and B unless mode 1; position = A0, yaw to A1; velocities 0),
  state 2, update 0x80, mode |= 0x41, collision off. 1: `SpringTurn2(atan(Ratchet − pos), 0.005, 0.2, 0)` + exhaust.
  2: Ratchet in the cuboid → mode 1: hard cut 0; else a new mine 933 (pvar +0 owner (index + 1), +4 −1.0, +8 +0xc 0,
  +0x10 `randf(π/300, π/150)`, +0x14 `randf(π/450, π/225)`, +0x18 `randf(π/900, π/450)`, the drone's position and
  yaw, update / draw 0xff, visible, matrix), carried, hard cut 2; then shown + animated, state 4, collision on.
  4: carry (mode 0), follow path A (`spline::advance(25·dt)`, three springs 0.01 / 0.2, turn when |dx| and |dy| >
  0.01); end → state 3, blend (3, 0, 5); exhaust. 3: carry, springs to A's last point; anim wrapped with A = B →
  mode 1: state 1, hard cut 4; else the mine released (its state 1) and dropped, state 5, blend (0, 0, 20), cursor 0;
  exhaust. 5: follow path B; end → state 2, hidden, collision off, cursor / velocities 0, position A0, yaw to A1 (no
  exhaust that tick); else exhaust. 6: `knock::update`: bits 1|2 → death explosion (size 1, light 13, no sound) and
  delete; else z < 5 → delete. Flash update for every state that did not delete.
- **Exhaust**: 2 glows (randi 16, randi 2 sign; 0.2 jitter, grow 1..0.9, size 99840, colour 0x7f204080, timer
  `ticks(12)`, phase 2, +0x2a 0x7f) at pos + row0·−0.5 + row2·0.6; 3 glows (0.05 jitter, sizes 80000 / 60000 /
  40000, spins 16 / −16 / 16, colour 0x7fffffff, timers `ticks(2)`·1/2/4, rotation `randi(255)`) at pos + row0·−0.4 +
  row2·0.7.
- **Carry**: mine position = pos + unit(row0)·0.1 + unit(row2)·0.1.
- **Method**: unit test of the state machine with synthetic splines / volumes; QA.

## 4. Shared code touched (regression risk)

* `rc-game/src/moby_update.rs`: `pub mod class_swap` (additive).
* `rc-game/src/moby_update/classes/units/mod.rs`: 8 `PORTS` rows, 5 modules, 5 doc rows (additive). Rows match by
  code identity: any other level whose class table names one of these functions now runs the port.
* `rc-game/src/moby_update/creature/target.rs`: `acquire_in` now calls `acquire_with(…, &[], region)`; the
  behaviour of the region path is unchanged (same closure logic). Existing creature tests cover it.
* `rc-engine/src/moby_render.rs`: `spawn_mobys` takes the swap-target function; the palette range per instance can
  grow only for instances with swap targets (level 04's 432s); metal entity spawning moved into a per-class loop
  (same entities for the placed class); `update_moby_occlusion` hides `pending_hide` first. Risk: entity / palette
  counts on level 04 change slightly; other levels identical (no targets).
* `rc-engine/src/moby_anim.rs`: `MobyAnim::set_class` (additive). `rc-engine/src/gameplay.rs` `drive_statics`: a
  class-index lookup per driven moby per tick (no effect when the class never changes).
* `tools/ghidra/names/census_systems.tsv`: 0x265d78 / 0x263718 now `V` with port citations; the 0x2e9d10 family note.
* Docs: `docs/plan/gaps.md` (G-CLS-031 closed, G-CLS-027 and G-REN-025 rows), `docs/plan/class_census.md` (new section).

## 5. Not ported (don't test as working)

* The pods' blob shadow `0x279ae0` (= L01 0x26eec8): G-REN-025.
* 1066's members (class 827, U211) are not ported (G-SAV-006 / G-HERO-005 blockers): in game they are never dormant,
  so the wakers only play their opening animation and stop (state 3).
* The race mine 933's own update (0x2ddde0, U517): the mines 1401 creates sit where they are dropped (their release,
  state 1, has no effect until 933 is ported).
* 899's debug `printf` and 1401's `printf`s (n/a).
* The crate's unreached class swap toward 0x1fe (G-CLS-009, dead code).

## 6. In-game QA spots (for the user)

* **432 / 1052 (level 04, Eudora)**: `RC_LEVEL=4 RC_HERO_AT=226,181,54,3.14` — bolt crank (inst 148) at (228.8,
  179.1, 53.0) drives the follower pair at (208.3, 188.4, 53.5) / (210.3, 188.3, 53.5); the other pair at (207.4 /
  209.6, 226.1, 53.1) follows the crank at (191.2, 227.6, 53.1) (`RC_HERO_AT=193,226,54,0`). Look: at load the
  followers show 1052's model; winding the crank poses their animation with the progress; a loop sound while it turns
  (1052's sound 0), sound 1 when it reaches the top; at full wind they turn into 432's model for good and stop following.
* **1066 (level 06, Blarg)**: `RC_LEVEL=6 RC_HERO_AT=325,184,136,0` (wakers at (323.3, 184.5, 135.8) and (327.2, 184.0,
  135.8), cuboid 1); also (334 / 337.7, 246, 134.6) and (252.5 / 255.8, 426 / 428, 129.5). Look: entering the cuboid
  plays sequence 1 then 2 (5-tick blends); no creature appears (827 unported).
* **899 / 898 (level 03, Kerwan)**: `RC_LEVEL=3 RC_HERO_AT=242,364,81,0` (also (264.4, 349.6, 74.6), (225.2, 233.8,
  35.6), (230.8, 203.8, 45.6)). Look: a line of platforms 2.9 apart gliding along the path at 3 u/s, a new one every
  ~58 ticks at the start, each vanishing near the end; a loop sound at the spawner; stand on one and ride it.
* **1885 / 1886 (level 09, Gaspar)**: `RC_LEVEL=9 RC_HERO_AT=250,300,27,0` near the launcher (246.3, 306.8, 26.0)
  (group 72: four 193 biters, deleted at the launcher's init); others at (284.8, 350.3, 26.0), (391.5, 322.6, 8.5),
  (395.8, 343.1, 8.0), (331.8, 345.4, 31.0). Look: pods lobbed every 30 ticks from its joint into its target cuboids,
  bouncing with a knock sound, settling, shrinking while a biter grows in at the spot and attacks; at most 4 pods +
  biters; a wrench hit kills it: red flash, beam explosion with sound 1, camera shake, three pieces, bolts.
  Level 13 (Gemlik): `RC_LEVEL=13 RC_HERO_AT=410,505,249,0` (launchers at (402.1, 515.3), (417.2, 532.5), (446.6,
  523.1), (402.5, 480.5), z 248, all group 20: 18 flying biters 63, random cuboid aim, up to 18 alive).
* **1401 (level 16, Kalebo III)**: `RC_LEVEL=16 RC_HERO_AT=104,268,128,0` (drone inst 1303 at (103.8, 270.5, 127.1),
  mode 0); mode 1 at (350.8, 57.8, 125.2) and (359.5, 34.1, 89.6); mode 2 (faces Ratchet at once) at (315.5, 48.7,
  111.5), (311.0, 39.0, 104.1), (312.1, 60.9, 115.3), (283.9–284.9, 58.4–63.6, 129–130.5). Look: hidden until Ratchet
  enters its cuboid, then appears carrying a race mine, flies its path with exhaust glows, drops the mine at the end and
  flies back; touching it plays the flinch; a hit knocks it away and it explodes where it lands (bolts).

## 7. Results (the test expert fills this in)
