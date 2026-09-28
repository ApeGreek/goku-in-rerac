# Class-port units, levels 09–18 (census "cheap wins", G-CLS-027)

The census units (class_census.md, `work/census/units.tsv`) whose first copy is on levels 09–18, ported one unit per
port under `crates/rc-game/src/moby_update/classes/units/` and registered in `units::PORTS` (`ClassUpdate::Unit(i)`,
found on every level by code identity through `LevelPorts`). The levels 00–08 half is a sibling batch in the same
registry. Tests: `crates/rc-game/tests/cheap_classes_b.rs` (registration and instance counts, then each unit headless
on one of its levels).

Unit ids are the census run of 2026-09-28 20:05 (they are renumbered by every run; the reference update is the stable
key). Addresses are the reference level's unless marked `L01:` (the level-01 copy of a shared function).

**Shared seams added for these units** (each with two or more consumers, or replacing a copy):
- `units::PORTS` / `UnitPort` (with `joints`: the classes whose joint points a port reads) and
  `LevelPorts::needs_joint_lists` (the loader fills the joint lists of classes another port reads; the engine's
  `gameplay.rs` loader uses it).
- `World::joint_local` (the first half of `joint_point`: `fun_00210850` + `VecScale(scale/1024)`).
- `scheduler::{group_ids, group_state, group_cmd}` (`0x26e0e0` / `0x26e090`; were private in `water::managers`, which
  now calls them).
- `interact::set_global_flag` + `GameWrite::Flag` (a class's store into the global flags `0x13d388`: the mirror the
  classes read this tick and the saved-game write; part of G-SAV-010).
- `units::{hero_pos, class_scale, class_collision, take_hero_light (L01 0x272078), bob (L01 0x277a00), wobble
  (L01 0x277a80)}`.

---

## U408 — Gemlik asteroids 212 / 1412 (level 13, 130 created) — `units::asteroid`

| address | what it does | ported / not |
|---|---|---|
| 0x2e1638 prologue | `VecDistance(pos, 0x13f3d0)` | `asteroid::update` |
| state 0 | home = pos (w 20), row = rows[0] (w scale), splits = `rand_range(2, 3)`; +0x72 = 0 below the class scale; → 3 | `update` |
| state 3 | pos = home; spin 0; `randf(−30, 30)` then `randf(−180, 180)` °·dt on axis `rand() % 3` (twice); 3 × `random_angle_radians`; velocity 0; scale = row w; +0x72 = 100 above the class scale; → 1 | `random_spin` |
| state 1 | Ratchet state 0x32 **and** the moby at 0x140940 of class 0x45 **and** (drawn or d < 40): velocity `randf(1, 10)·dt` along row, → 2 | ported; the 0x140940 class is **not modelled** (always false: G-HERO-002) |
| state 2 | `FastDecTimer__FRs(+0x4c)`, pos += velocity | `drift` |
| state 2 far | undrawn and d ≥ 40: `FastDecTimer__FRi(+0x44)`: a piece `DeleteMoby`, a rock → 4 (collision off, mode 1, undrawn, scale) | `drift`, `hide` |
| state 2 near | timer = `ticks(360 / 120)`; cool-down ≠ 0: done; collision on; index parity = tick parity: `coll_sphere(pos, 6.2f, 0, self)` (L01 0x212960) | `drift` |
| contact | hit moby ≠ maker: `FUN_00221570` reflect, pos = pushed centre (0x174170); the hit moby not a 212 (or the world): split | `drift` (`services::reflect`) |
| hit | else `MobyGetHitMessage(0x330000)`: attacker not a 212: push `ClampVecLength(20·dt)` added, split; +0xa4 = 0xff | `drift` |
| state 4 | `FastBSphereCheck(draw dist, home, r 20)` out of view: back home, collision on, drawn, mode &= ~1, → 3 | `update` (`fx::in_view`) |
| epilogue | clamp pos to [20, 1003]³; rotation += spin (`fast_add_rotations` ×3) | `update` |
| 0x2e1348 split | `PlayClassSound(0, 0)` (sound) | `split` |
| | `SpawnBeamExplosion(7f, 10f, 4f, 2f, 9, 1, 15f, moby, 0x160680 (unused), pos 0, 10f, 3f, 16f, −1, 0, debris 1, −1, 0)` (damage 0 beyond 20; effects on other mobys: its damage sphere, flags 0x810001) | `split` (`fx::beam_explosion`); its debris burst `param_16` is counted, not spawned (fx.rs, G-CLS-026) |
| | +0x72 = 0; no splits: rock → 4, piece deleted | `split` |
| | loop `rand_range(1, 2)`: `randf(0.25, 0.75)`, `rand_vec(5dt, 10dt)`, `VecScale(0.75)`, `VecAdd`, `FastVecNormalize(0.5)`, `VecAdd` ×2, **piece**, scale −= s, splits −= 1, break rules | `split` |
| | collision off, cool-down `ticks(30)` | `split` |
| 0x2e1140 piece | `CreateMoby(1412)`, fields, `ticks(30)`, spin draws as state 3, `MobyBuildMatrix`, `FUN_00272078` (Ratchet's light) | `spawn_piece` (`take_hero_light`) |
| other mobys | the explosion's damage sphere (flags 0x810001) on nearby mobys | `fx::beam_explosion` |

Tests: `asteroid::tests::init_reset_and_idle`; `cheap_classes_b::asteroids_idle_then_split_on_a_hit` (130 rocks idle
and spin in the box; a hit: class sound 0, explosion particles, 1–2 pieces with maker / velocity / cool-down, the rock
smaller, collision off; a piece drifts and is deleted undrawn far away).

## U303 — Gaspar chain links 1181 (level 09, 124 created) — `units::chain_link`

| address | what it does | ported / not |
|---|---|---|
| state 0 | `0x13d3c1[+0x69]` set (global flag read): `DeleteMoby` | `update` (the flag mirror `TalkGame::flags`) |
| | previous = 0x3041d0(`FUN_002645a8(self, 0)`, −1); next = 0x3041d0(pos, 0); none: `STUB_printf` | `search`; the print is counted (`unported`) |
| 0x3041d0 | walks the moby list 0x15ffe4 (**[L]** the table in index order), classes 1181 / 1172 / 1184, self excluded; the candidate's position or its joint-0 point; nearest within 0.5, or a 1184 within 10 | `search` |
| state 1 gate | undrawn or > 32 from the camera 0x166f40 (**[L]** `World::camera`): only when `counter % 8 == spawn id % 8` | `hang` |
| neighbours | deleted next: settled = 0; next unsettled: settled = 0 | `hang` |
| sag | unsettled, previous and next 1181: next unsettled → `(|d|−1.3)·0.7` pulls toward both, `dt²·9.8` fall; then pitch / yaw toward the previous (`FastArcTan`, `fast_subtract_rotations`, 10 %) | `hang`, `pull`, `turn_toward` |
| break | +0xbc = 1: +0x6c moby in state 1 gets +0xbc = 1 (effect on another moby); timer `ticks(10)`, update distance 0xff, → 2 | `hang` |
| state 2 | toward the previous `(|d| − 1.3)`, fall `2·dt²·9.8·(ticks(10) − (t − 3))`, turn 50 %; timer end: previous / next (1172, 1181, 1184) +0xbc = 1, update distance 0xff; → 3 | `breaking`, `tell` |
| state 3 | `SpawnBeamExplosion(0, 0, 2, 1, 10, 1, 10, link, camera − pos (unused), pos 0, 10, 5, 8, **class sound 0**, 0, debris 1, −1, 0)`; `DeleteMoby` | `update` (debris burst counted, as above) |

Tests: `chain_link::tests::{break_runs_down_the_chain, search_takes_the_nearest_within_half_a_unit}`;
`cheap_classes_b::chain_links_find_their_neighbours_and_break_in_turn` (all 124 link up; a break explodes the link and
runs down the chain; class sound 0, particles).

## U553 — Veldin grouped pieces 885, 888, 891, 892, 894, 900, 901, 936 (level 18, 119 created) — `units::barricade`

| address | what it does | ported / not |
|---|---|---|
| state 0 | mission (+0xb0) set and its save byte `0x14c050[L·16 + m]` = 0xff: `DeleteMoby`; else → 1 and `*P[0] = 3` when P[0] ≠ 0 (a self-relative pointer: 52 point at their own +0x08) | `update` |
| state 1 | `MobyGetHitMessage(0x80000)`: `0x26e0e0(group, 2)` (the whole group's state: effect on other mobys), `PlayClassSoundByClass(0, 0, moby, 885)`; +0xa4 = 0xff | `update` (`scheduler::group_state`) |
| state 2 (≠ 892) | `randf(10, 15)·dt`; collision off; → 3; `randf_sym(0, 90)`·°·dt → +0x08, +0x0c; `VecSub(pos, Ratchet)`, z 0, `FastVecNormalize`; `rand_vec(s, s)`; `VecAdd`; `fabs` z; +0x1c = z | `launch` |
| state 2 (892) | 10 × (`randf(8, 10)·dt`, `randf(0, 6)`, `randi(6)` ×2, `rand_range(ticks(15), ticks(20))`, `rand_range(ticks(30), ticks(45))`, `PartType11Spawn(1.5e6, …, 0x15f580 = 0)`; `rand_range(ticks(5), ticks(10))`, `rand_range(ticks(15), ticks(20))`, white `PartType11Spawn(·0.5)`); 10 × (`randf(0, 6)`, 3 × `randf(−1, 1)`, `randf(0, 3)`, `FastVecNormalize`, `randi(6)` ×2, `rand_range(ticks(30), ticks(45))`, `PartType08Spawn(5e5, …)`) with the colour tables 0x1f29a0 / 0x1f29b8 (= `fx::SPARK_A` / `SPARK_B`, checked); `DeleteMoby` | `burst` |
| state 3 | x / y rotation += +0x08; z velocity −= 20·dt²; pos += velocity; below start − 16, or `coll_sphere(pos, 1, 2, self)` hit: `0x273f50(1.5, 0, moby, pos, −1)` (death explosion) + `DeleteMoby` | `fly` (`fx::death_explosion`) |
| other states | `DeleteMoby` | `update` |

Who sends hit flag 0x80000 is not established here (the port's weapons do not set it) [L]. Tests:
`cheap_classes_b::barricade_group_flies_apart_on_a_hit` (no pieces with missions not started; with them: a hit sets the
whole group of 67 to state 2 and plays 885's sound 0; the pieces fly away from Ratchet upward and all explode).

## U294 — Gaspar tethered platforms 1182–1189 (level 09, 80 created) — `units::tethered_platform`

| address | what it does | ported / not |
|---|---|---|
| prologue | `$gp−0x5838++` (a call counter; nothing reads it) | n/a (write-only counter) |
| state 0 | `FUN_002645a8(self, 0)`; global flag `0x13d3c1[+0x68]` set: `DeleteMoby` | `init` |
| | draw distance 0x3ff; core with +0x28: 0xff, core none, 0x2c2500 for 1182..1189 but 1184 | `init`, `make_piece` |
| 0x2c2500 | `CreateMoby`, update / draw distance 0xff, state 1, drawn, spawn point, core, `MobyBuildMatrix`, own joint 0, `VecSub`, `MatrixMulVec3` (core rows), `VecAdd`, the core's rotation and rows, `MobyBuildMatrix` | `make_piece` |
| | a placed piece: 0x2c2610 (nearest 1184, 0x15ffe4 walk [L] table order) within 20: core, and the core's slot `class − 1182` = it (effect on the core); else `STUB_printf` | `init`, `nearest` (print counted) |
| | +0x30 = `fun_00210850(list 0)` translation · scale/1024 | `init` (`World::joint_local`) |
| states 1 / 2 | core: `0x277a00` bob, `0x277a80` wobble | `hold` (`units::bob`, `units::wobble`) |
| | piece: core not in state 3: rotation = core's, `EulerToMatrix`, `0x221608`, `FUN_002645a8(core, 0)`, `VecSub` | `hold` (`script::euler_rows`) |
| | piece with a core: mode bit 1 and +0xbc from the core | `hold` |
| | state 2: core x += 1°·dt; `FastDecTimer` → 3 | `hold` |
| | state 1, +0xbc = 1: update distance 0xff, `rand_range(ticks(10), ticks(50))`, the 9 slots' update distance 0xff (effect on the pieces), → 2 | `hold` (slots within the block only) |
| state 3 | `FastDecTimer`; **0x13d3c0 = 1**, `0x13d3c1[+0x68] = 1` (save flags) | `shake_loose` (`interact::set_global_flag`) |
| | `SetDeathBits(0x200, −1)` (bolts / death bits) | `crate_::set_death_bits` |
| | `0x273f50(4, 13, moby, pos, −1)` (death explosion: sparks, flashes, shake, light) | `fx::death_explosion` |
| | core: `PlayClassSound(2, 0)` | `shake_loose` |
| | velocity, spins `randf(∓18°·dt)`, timer `randf(ticks(90), ticks(150))`, gravity `randf(4.2, 6.2)·dt²`, 1185/1186 variants; collision off; → 4 | `shake_loose` |
| state 4 | fall, spins; out of [12, 1011]³: `DeleteMoby` | `fall` |
| | `randi(3) = 0`: `FUN_0021ef40` (zero), `randi(2)`, 2 × `rand_range`, `randf(8, 16)`, `PartType11Spawn(4e5, …, 0x2f3f3f7f, 0x1613d0[k])` | `fall` |
| | z < 25: 50 × (`random_angle_radians`, 2 × `randf(−3, 3)`, `fast_cos` / `fast_sin` accumulations, 3 × `randf(n, 2n)` · `multiply_global_scale` · `truncate`, `PartType02Spawn`); `VecScale(0.45)`; spins ·0.9; `PlayClassSound(0, 0)`; → 5 | `fall` (the stack leftovers: module doc [L]) |
| state 5 | sink `10.8·dt²/3`, spins; out of the box: `DeleteMoby` | `update` |

Tests: `cheap_classes_b::tethered_platform_breaks_and_sinks_into_the_lava` (80 placed, 70 pieces find their 10 cores;
the core bobs; told to break: states 2 → 5, the flag writes 0x38 / the core's byte, class sound 2, explosions, the
pieces follow, the lava splash and sound 0, deletion).

## U417 — Gemlik explosive tanks 1261 (level 13, 55 created) and their fireball 1634 — `units::explosive_tank`

| address | what it does | ported / not |
|---|---|---|
| state 0 | D+9 = 0; rotation x / y ·0.7 | `idle` |
| | `MobyGetHitMessage(0x330001)`; the debug `STUB_printf` of the damage | `idle`; the print n/a (debug) |
| | `0x26f378(moby, hit, D = +0x20, 0, &out5, 0, 0, col 4)`; reaction 1 / 2 → health 0 | `creature::damage::resolve` |
| | out5 > 1, health ≤ the record's damage: health 0, mode &= ~0x1000 (off the target list), `SetDeathBits(0, −1)` (bolts / death bits), F+7 = 0x78, `0x272318` (flash), +0x70 = 0, burst, scorch, `PlayClassSound(0, 0)`, → 1 | `idle` (`crate_::set_death_bits`, `creature::flash::start`) |
| | else health −= damage, F+7 = 0xfa, D+6 = `ticks(60)`, `0x272318`, rotation x / y = `randf(±2°)` | `idle` |
| | +0xa4 = 0xff; `0x2723f8` | `idle` (`creature::flash::update`) |
| state 1 | centre pos + (0, 0, 1); `0x2705a8(0.5, 4.5, t / ticks(20))`; `coll_sphere_mobys(r, centre, 0x10, self)` + `0x26f8f8(4, 1, 1, self, centre, list, n, 0, 0x830001, 2, 1)` (hits every listed moby) | `exploding` (`creature::attack::area_hit`) |
| | `multiply_global_scale(10) < t`: mode \|= 0x41, scorch; else alpha ·7 >> 3; t += 1; past `ticks(20)`: scorch, `DeleteMoby` | `exploding` |
| 0x3073c8 burst | base (0, 0, 2dt)·1.5 (`VecScale`), normal up, pos + (0, 0, 1), camera 0x1670c0 distance; 10 low fireballs (2 × `randf(−1, 1)`, `FastVecDot`, `FastVecNormalize`, `VecSub`, `randf(0, 1)`, `randf(3.5, 6.5)`, `rand_range(ticks(60), ticks(120))`); 4 high (`randf(6.5, 10)`, `(ticks(60), ticks(90))`); one toward the camera (3 × `randf(−1, 1)`, `ClampVecLength(10dt)`); n rings (`randf(8, 10)`, `randi(6)` ×2 from 0x1f5560 / 0x1f5578 (green; differ from the bomb's), 2 × `rand_range`, `PartType11Spawn(6e5, …)`); flashes 0x309a68 (6, 6 beyond 9; 6, 5.25, 4.5) | `burst` (`bomb::flash`; the fireball spawn is its own, below). No sound, shake or light in this copy |
| 0x30c138 fireball spawn | `CreateMoby(1634)`, drawn, distances 0xff, alpha `rand_range(0x40, 0x80)`, ambient (0x20, 0x60, 0x10) (`0x2650d0`), pos, velocity, 2 × `randf(2π, 4π)·dt`, life ×2, low: scale ·`randf(0.5, 0.75)`, +0x1c = scale, +0xbc = type, scale ·`randf(2, 5)`, `MobyBuildMatrix` | `fireball` |
| 0x30c3b8 (1634's update) | high ones: 2 passes × 2 `PartType02Spawn` (jitter ±0.065 / ±0.1, one extra `randf(±30)` draw in the second pass, v1 z, heading ± `randf(±90°)`, v2 xy / z, 3 phase lengths `trunc(scale(randf))`, `$gp` 0x161f08..0x161f68 colours) | `fireball_update` |
| | spins, pos += velocity, z −= 14.6·dt², out of the positive octant: delete; last quarter: scale = +0x1c·t/q; `FastDecTimer` done: delete; high: `CollLine_Fix(old, new, 2, Ratchet)` hit → scorch 0x30c2b0 + delete | `fireball_update` (`World::coll_line`) |
| 0x3072c0 / 0x30c2b0 scorch | `random_angle_radians`, `randf(0, 0.8)`, `randf(0, 1.5)`, `randf(1.5, 5.7)`, `rand_range(100, 180)`, `PartType52Spawn` (colours 0x38002028 / 0x68002028, 0x2020) | `scorch` (one code shape, two word sets) |

**Cross-unit:** 0x3073c8 is the Bomb Glove's dry explosion (`bomb::explode`) at k = 1.5 with this level's colours and
its own fireball class; `creature::react::burst` (0x304798) is a third copy. The three are separate functions in the
game; one parameterised burst would serve all three (follow-up; `bomb.rs` / `react.rs` belong to running agents).
Tests: `cheap_classes_b::explosive_tank_takes_hits_then_explodes`.

## U533 — Drek's Fleet sliding doors 1359–1364, 1367, 1369, 1372, 1373 (level 17, 52 created) — `units::fleet_door`

| address | what it does | ported / not |
|---|---|---|
| prologue | a cuboid −1: `DeleteMoby`; `0x15f5c4 == 2` (cutscene): nothing | `update` |
| state 0 | 1360, 1361, 1363, 1367, 1373: +0xbc = 1; home = pos; → 1 | `update` |
| state 1 | offset 0; the sound slot: `release_voice_slot` when owned and live (0x13e5d8 / 0x13e5c4), −1 | `release` |
| | `FUN_001fc2c0` (fabs) of rot x ≥ π/4 and 0x13f658 = 0 (no magnetic floor): nothing | `update` (`Hero::f658`) |
| | `PointInCuboid(Ratchet, P6)` or the linked moby (P4) in P6: class +0x28 (sound defs) and not `SoundIsAlive`: `PlayClassSound(0, 4)`; → 2 | `keep_sound` (`ClassInfo::has_sounds`, new) |
| state 2 | sound kept; offset ±3dt; pos = home + offset·(cos, sin) of rot z (+π/2 for 1367, 1369, 1372, 1373); |offset| > 2: release, → 4 | `update`, `place` |
| state 4 | release; Ratchet or the camera 0x1676c0 in P7, or the linked moby in P6: wait; else the sound, → 5 | `update` |
| state 5 | sound kept; Ratchet / camera / linked in P6 → 2; offset steps back (0 within a step); at 0: release, → 1 | `update` |

Tests: `cheap_classes_b::fleet_door_opens_for_ratchet_and_shuts_after` (all 52 have both cuboids; a door slides 2
units with class sound 0 flags 4 while Ratchet is in its trigger, and back home when he has left).

## U477 cogs 937 (15, 48), U479 belts 1250 (15, 18), U500 floats 650 (16, 29), U499 pieces 647 (16, 16), U563 carriers 1584 (18, 38)

Their coverage tables are in the module docs (`units::{linked_cog, quartu_belt, rising_float, extending_piece,
veldin_carrier}`): every call and branch of each update is listed there. Side effects: the belt's and the carrier's
platform block (`triggers::carry_riders`; the hero rides them through `hero::platform`), the float's class sound 1
with the level-wide 4-tick limit (`units::Globals`, the level word 0x161a88), the carrier's attachment 1892 (created
and moved: an effect on another moby). U479 resembles the conveyor belts of 02 / 12 (`units::conveyor`, U95) but is a
separate function (no reversal, heading + π/2, anim speed 1.3889); no merge. Test:
`cheap_classes_b::small_units_on_their_levels`.

## U559 props 1432 (18, 13), U493 482 (16, 8), U484 bubble vents 1425 (15, 6), U456 1397 (14, 6)

Tiny updates, their tables in the module docs (`units::{hidden_prop, bubble_vent, oltanis_switchboard}`): 1432 hides
itself for good at its first update; 482's update (level16 0x2cb600) is `DeleteMoby(self)`, registered on the
markers' Rust update (`units::marker`, U139: another function with the same effect); 1425 blows a type-34 bubble one
tick in two up to the water level in its pvar (`bomb_water::part34`, shared with the Bomb Glove's water branch);
1397 only initialises / advances fields another moby reads. Test: `cheap_classes_b::tiny_units_on_their_levels`.

## Units found not cheap (the census missed a system)

| unit | classes (level, created) | why | gap |
|---|---|---|---|
| U543 | 1843 fighters (17, 60) | Ratchet's ship-combat mode: hero state 0x32 with the ship moby (0x140940, class 0x45) gates the attack states 5 / 6 and the target-list bit; the census sees no call for it (a direct read of 0x1413d4). Also a trail draw callback (0x2f3b68) and the per-level skill-point counters 0x157150..0x157154 | G-LVL-009 (new) |
| U370 | 1319 fighters (11, 52) | the same fighter source as U543, compiled for Pokitaru (differences: path table 0x1b0eb0, drop classes 0x4c2 / 0x4c4, 0x141c18 = 0xffff on its death): one port with a data row once the ship mode exists | G-LVL-009 |
| U408 (ported) | 212 / 1412 (13) | its wake needs the same ship mode; everything else is ported | G-LVL-009 |
| U474 | 408 (15, 17; 48) | an enemy dispenser with a shared hum loop and a draw callback of its own (0x2cbd88, a new draw-callback kind for the engine) and the hero state 0x65 case; not attempted in this batch | G-CLS-027 (left) |

## Frames (2026-09-28)

`RC_PLAY_FLY=1 RC_CAM=… RC_LEVEL=… RC_SCENE=0 RC_OCCL=0 RC_DUMP_FRAMES=120..120`, two runs each (scratchpad
`cheapB/frames/`): U408 (13: an asteroid 212 in front of the station, others in the sky), U303 (09: the links hanging
from their anchor over the lava), U553 (18: a row of the grouped pieces), U294 (09: a platform core with its pieces
over the lava), U533 (17: a door in its frame), U477 (15: the cogs inside their machine), U500 (16: the floats at
rest), U479 (15: the green belt), U563 (18: the hexagonal plates at the carrier's position [L]); U417's camera found no
clear view of a tank (inconclusive). Every pair byte-identical, except the first U408 pair (the asteroid's spin phase
differed once; a second pair was identical: an intermittent engine-side difference in which ticks the asteroids are
active [L], not reproduced headless).

## Native, not emulated

Standard `f32` throughout; no PS2 float model. Values from the decomp (the level's `$gp` tuning words are cited by
address). [L] items: the moby list 0x15ffe4 walked as the table in index order (chain links, platform pieces), the
camera globals of each level (0x166f40 / 0x1670c0 / 0x1676c0) read as `World::camera`, the tethered platforms' splash
stack leftovers, the names of the classes.
