# Creatures and enemies: survey, shared layer, ported classes (2026-09-27)

Addresses are level01 (`work/decomp/level01.elf`) unless noted. The shared layer lives in
`crates/rc-game/src/moby_update/creature.rs` + `creature/*`; the classes in `moby_update/classes/{critter,amoeboid}.rs`.
Standard `f32` everywhere (no PS2 float model); formulas, constants, operation order and the rand draws are the game's.

## 1. Survey: Novalis classes that are created but had no update in the port

Counts are the instances the loader creates on a first visit (`rc_formats::moby_spawn::loader_spawns` with a new game's
save: 929 of 983). "fn" is the level01 class-table entry (`lvl.vtbl` 0x20bb00). Status after this pass in the last
column.

| class | created | fn | kind | what | status |
|---|---|---|---|---|---|
| **577** | 14 (68 placed; 54 need a done mission) | 0x2efc60 | ambient critter | hovering bugs that land, bite and flee | **ported** |
| **572** | 5 | 0x2edca0 | enemy | amoeboid, big; splits in two when hit | **ported** |
| **865 / 866** | 10 / 20 | 0x2edca0 | enemy | amoeboid medium / small, hidden until a split wakes them | **ported** |
| **459** | 25 | 0x2e6bf0 | enemy | robot trooper: jetpack arrival, patrol path hops, fire-glob sprayer (722) | **ported** (2026-09-28) |
| **666** | 2 | 0x2f4960 | enemy carrier | dropship: flies a path, drops ≤ 8 children (0x1ab / 0x154 / 0x1cb) | **ported** |
| **688** | 2 | 0x2f7728 | enemy (scripted) | gunship: town bombardment (686 shells, 700 fires) and the bridge fly-by | **ported** |
| **815** | 6 | 0x3021b8 | enemy spawner | re-creates a member of its group while the group is empty; group −1 on Novalis (inert) | **ported** (gate only) |
| 1818 | 1 | 0x30df40 | helper (summoned) | the Sonic Summoner's "mouse": head item 5 (`0x1404a8` is slot 2's item, not the hand) worn and owned (`0x13d4c5`); flies beside Ratchet shooting enemies (1633) | **ported** (`classes::mouse`, level_generalisation.md "Common classes") |
| 11 | 1 | 0x2bb128 | other | Gadgetron vendor | not ported |
| 604 | 1 | 0x2f2b68 | prop | the mouse's house (opens / closes on the mouse's commands) | **ported** (`classes::mouse`) |
| 605 | 20 | 0x2f2eb8 | other | buried bolt cache (the Metal Detector's target; hidden) | **ported** (`classes::buried_bolts`) |
| 613 | 4 | 0x2f3120 | other | water current | not ported |
| 730 / 790 | 1 / 1 | 0x2fad68 | other | mission NPCs | not ported |
| 737 | 5 | 0x2fb5b0 | other | camera trigger | not ported |
| 750 | 1 | 0x2fbf80 | other | infobot | **ported** (hero_gameplay.md §6; inert on Novalis) |
| 774 | 1 | 0x2ff118 | other | talking NPC | not ported |
| 832 | 1 | 0x302648 | other | Visibomb range limiter | **ported** (`classes::rc_range`; inert without the Visibomb) |
| 1134 | 3 | 0x307ca0 | other (pickup) | gold bolt | **ported** (hero_gameplay.md §6) |
| 1341 | 1 | 0x30acb8 | other | help-hint director | not ported |
| 280 | 2 | 0x2e0c68 | prop | bolt crank | not ported |
| 641 / 665 | 1 / 4 | 0x2f4348 / 0x2f4710 | prop | crank-driven rotator / slider | not ported |
| 695 | 9 | 0x2f8268 | prop | floating pushable | not ported |
| 701 | 3 | 0x2f9080 | prop | collapsing platform | **ported** (§7; the camera look `FUN_002f9000` is not) |
| 703 / 715 | 2 / 5 | 0x2f95c0 | prop | elevators | **ported** (§7) |
| 704 | 3 | 0x2f9810 | prop | breakable rock | **ported** (§7; its type-22 puffs are records only) |
| 705 | 5 | 0x2f9bf0 | prop | oscillating spinner | **ported** (§7) |
| 709 / 710 / 711 | 1 / 1 / 1 | 0x2f9d80 | prop | breakable by 686 | **ported** (§7) |
| 729 | 1 | 0x2fa800 | prop | big breakable wall | **ported** (§7) |
| 746 | 2 | 0x2fb8a8 | prop | hinged bridge | not ported |
| 754 | 12 | 0x2fd9a0 | prop | breakable pot | **ported** (§7) |
| 768 / 769 | 4 / 4 | 0x2fed68 | prop | sliding door halves | **ported** (§7) |
| 778 | 1 | 0x2ff860 | prop | breakable pipe (leaves the spray 779) | **ported** (§7, with 779) |
| 1042 | 12 | 0x307c48 | prop | shootable | **ported** (§7) |
| 1813 | 4 | 0x30d0f0 | prop | breakable crate variant | **ported** (§7, with the remains 1816) |
| 66, 310, 687, 742, 747, 780 | 1, 1, 1, 5, 3, 2 | none | prop | static, no update (mode 2 as in the game) | — |
| 676, 678 | 1, 1 | 0x2f6128 (undef), 0x2f6180 | effect | custom-draw water meshes | not ported (drawn by the water renderer) |
| **700** | 6 | 0x2f8c58 | effect | fire / smoke where a gunship shell landed; drops embers 696–698 | **ported** |
| 760 | 4 | 0x2fdbc0 | effect | **fire and smoke fields** on the bombed buildings (not waterfall foam: §7) | **ported** (levels 00, 01, 14) |
| 761, 1225 | 1, 1 | 0x2feb58, 0x309c98 (undef) | effect | strip water | not ported (drawn by the water renderer) |
| 809, 1848 | 1, 1 | 0x2ba658, 0x30f208 | effect | 809: the fire fields' smoke scroll and timer set-up; 1848: UV scroller | 809 **ported** (§7); 1848 not ported |
| 1504 | 1 | 0x30b618 | effect | wandering point light | not ported |
| 1546 | 1 | 0x30c190 | effect | cutscene FX driver | not ported |

Spawn-only classes the ported creatures create or wake: the body pieces 1747–1749 (577's death, `FxGroupUpdate`
0x30cd18), the explosion light 639 (0x2f3748), the explosion flashes 0x70 (ported before, `classes::debris`), the bolts
13–16 (ported before); since 2026-09-28 the trooper's fire glob 722 (0x2fa4c0), its body pieces 1736–1738 / 1770
(`FxGroupUpdate`), the gunship's shell 686 (0x2f6a30) and the embers 696–698 (0x2f8718).

### What the player meets on Novalis (2026-09-28)

| what the user sees | class | how it arrives |
|---|---|---|
| hovering bugs that land and bite | 577 | placed (14; 54 more once a mission is done) |
| blobs that split | 572 → 865 → 866 | placed; the small ones wait hidden until a split |
| **the "bigger robots"** (yellow jetpack troopers with a fire-glob sprayer) | **459** (25) | all placed, but 21 wait *hidden 20 units above* their patrol path's first point until Ratchet comes within 30 (xy) and then fly down on their jetpacks (10 u/s); none stood on the ground in the game. Unported they stood at their raw placement, doing nothing |
| **the ones that fly down from a ship** | 459 × 3 (309, 311, 312) + dropship **666** (instance 687) | Ratchet's landing spot is inside cuboid 27: the dropship leaves at once, flies path 41 over the pad, opens at node 6 and releases them one every 20–30 ticks; they glide to cuboids 38 / 40 / 41 (by the town gate) |
| three troopers at the bridge | 459 (320, 321, 333; wake range 0) + gunship **688** (instance 695) | Ratchet enters cuboid 22 → the gunship's scripted fly-by shells the bridge; with 60 ticks left it wakes them and they drop in |
| three troopers descending a slope | 459 (325–327; wake range 0) | trigger cuboid 42 → they fly down their arrival paths 45–47 (from z 70 to 40) |
| four troopers already standing | 459 (322, 328–330: +0x1b4 set) | not lifted: they stand until Ratchet is within 30 |
| the gunship strafing the town, burning houses | 688 (instance 694) + 686 shells + 700 fires + 696–698 embers | always active (update distance 0xff): fires at path 27's nodes 11 / 27 / 198 / 210 on cuboids 12–15; the fires start when Ratchet and the camera are in their cuboid |
| (nothing) | 666 instance 688 | no trigger cuboid: the parked dropship (sequence 5) |
| (nothing) | 815 × 6 | group −1: inert on Novalis |

**Spawn logic** (verified from the pvars and the code): no enemy is *created* after the load on Novalis — every trooper is
a static instance; "appearing" is the dormant state 2 (hidden, no collision) left by proximity (+0x1a8), a trigger
cuboid (+0x1c0), or a carrier writing +0x1e8 (dropship 666 on release, gunship 688 at its fly-by's end). The only
spawners, 815, are inert (group −1). The loader's spawn test (mission-gated flags 1 / 3 on the troopers) creates all
25 on a first visit.

**The enemy code on other levels** (same method): 459, 666, 688, 700, 722, 815 and 696–698 run their function only on
**01**; 686's function also runs class 686 on **14** (registered: the registry is class-keyed); 1818's on 01–06, 08, 11,
12, 14 (not ported).

**The creature code on other levels** (class-table entries whose function has the same cluster hash, `tools/ghidra/names/clusters.tsv`,
checked with `lvl.vtbl` of all 19 overlays): `AmoeboidUpdate` runs 572 / 865 / 866 on **01, 05 (Rilgar, 108 instances) and 11**;
`GroundCritterUpdate` only 577 on 01; the explosion light 639 on **all 19 levels**; `FxGroupUpdate` runs 13 classes on 01
(1736–1738, 1747–1749, 1761–1763, 1770, 1814, 1815, 1817; other levels use other class numbers for it — the registry is
class-keyed, so they are not registered, see level_generalisation.md C1). The cluster hash under-matches (`%lo` operands),
so a 577-like function elsewhere could be missed.

## 2. The shared layer (`moby_update/creature`)

Every function below is engine code (cluster present in ≥ 2 overlays unless noted); each is ported once and the classes
call it.

| service | game | used by | notes |
|---|---|---|---|
| `target::acquire` | `0x274b78` | 577 | Ratchet, none while his group is 0x18 or state 0x72; decoys (0xcb / 0x76c state 3, 0x10e with +0xbc) nearer than the range win. Record: position, Euler, aim 0x13f410, body 0x13f420 |
| `target::acquire_in` | `0x274df8` | 572 family | the same with a polygon region (Ratchet's feet inside) |
| `turn::spring_turn2` | `SpringTurn2` 0x26d058 | 577 (+ knockback yaw) | `vel += acc·clamp(d/(π/20)) − damp·vel`, capped, never past the target |
| `turn::turn_toward` | `0x270cc0` (+ 0x270ac0, 0x2709f8) | 572 family | accelerate / brake to stop on the heading (`v²/2a`), vmax |
| `turn::approach` | `Approach` 0x270728 | all | |
| `walker::seed` / `step` / `move_collide` | `SeedJumpPattern` 0x26d930, `0x26d9a8`, `0x26d610`, `0x26d1d0` | 577, 572 family | speed ramp, ledge probe (up / down), line guard, up to 6 sphere push-outs, ground line with step-up, undo on too big a height change |
| `knock::start` / `update` / `aim` / `lob_up` | `0x271418`, `0x271558`, `0x26fa48`, `0x26faf0` | 577 (hit, death), 572 family (split children) | ballistic flight with drag, landing (≥ 45°), wall bounce, water entry count, anim speed timed to the apex / landing keys, crates the flight touches get a hit |
| `damage::resolve` | `0x26f378` | 577, 572 family | attack kind from the record's type bytes, per-kind cooldown (restart ticks 37/15/30/…/60), damage tables 0x1b0648 / 0x1b0600 / gp−0x6b80, the wrench's push redirect (0.3·push + 0.7·away from Ratchet) |
| `attack::hit_moby` | `0x26eaa8` | 577 bite, 572 strike, knockback crate hits | a hit record for the target; Ratchet's moby → the tick → P2's `hit_intake` (hurt state, knockback, health) |
| `attack::group_command` | `0x26e090` | 577 | +0xbc on every group member (the alert) |
| `flash::start` / `update` | `0x272318` / `0x2723f8` | 577 | ambient-colour hit flash (red 0x80 on Novalis, 4 ticks in, 15 out) |
| `fx::death_explosion` | `0x273f50` | 577 | 3 type-11 spark pairs, 2 flashes, camera shake in view, class sound 4, explosion light |
| `fx::beam_explosion` | `SpawnBeamExplosion` 0x273310 | body pieces | damage sphere, type-15 streaks, type-11 pairs by camera distance, type-8 puffs, flashes, shake, sound, the light (template 0x1b06d0) (debris burst `0x2c4c20` not ported) |
| `fx::break_piece` / `piece_update` | `BreakFxB` 0x278ad8 / `FxGroupUpdate` 0x30cd18 | 577 death | 8 draws per piece; flies, spins about its sphere, bounces, explodes after `randf(60, 120)` ticks |
| `fx::light_spawn` / `light_update` | `0x2f3570` / `0x2f3748` (class 639, all levels) | death / beam explosions, the bomb | timer, view test, position / radius / colour ramps and their draws; its point-light slot (`rc_game::point_lights`), which the moby renderer merges into nearby mobys' third light |
| `fx::rate_slot` | `0x2efbf8` (n = 2) | 577 | busy-until table 0x161a80 (3 slots, 60 ticks) |
| `ground::ground` | `GroundHeight` 0x26e618 + `CollType` 0x2151d8 | 577, knockback | a miss reads CollType's stale output in the game; −1 here |
| `ground::key_time` / `passed_frame` | `MobyAnimKeyTime` 0x263920 / `0x2765b0` | 577 bite, 572 strike, knockback | |
| `region::*` | `0x26e6c0`, `0x276640`, `ClampToPath` 0x276820, `0x276a48`, `0x276c40`, `LineOfSightTest` 0x276fe8 | 572 family | arena walls (edges with a non-zero w), waypoint graph (point w = visibility bits) |
| bolt drop | `SetDeathBits` 0x26c250 → `BoltBurst` 0x275988 | 577, 572 family | reused: `classes::crate_::set_death_bits` |
| flashes | `FlashSpawn` 0x2c20e0 | explosions | reused: `classes::debris::flash_spawn` |
| sparks | `PartType11Spawn` 0x27f8f8 | explosions | reused: `World::part11` |
| class sounds | `PlayClassSound` 0x2a1618 | 577 (death), 572 (hit 0, landing 4) | reused: `World::play_sound` → the audio layer's `ClassSoundSink` |
| `turn::spring_turn` | `SpringTurn` 0x26cef0 | 459 (tilt, hop turn), `spring_turn2` | `SpringTurn2` 0x26d058 is now this on the yaw (`classes::flyer` keeps its own private copy) |
| `projectile::template` | the enemy shots' stack template | 722, 686 | push, attacker = the projectile, flags, type bytes, class, damage |
| `projectile::sweep` | `CollLine_Fix(a, b, fl, ignore, tmpl)` 0x211870 → `coll_sphere_mobys(r, b, 0x10, ignore, tmpl)` 0x214468 | 722 | the step's line with the template, the sphere on a miss; the hit point / the end, or none |
| `projectile::part` | `PartType16Spawn` 0x280f30, `PartType22Spawn` 0x281f30, `PartType26Spawn` 0x282b00 | 700 smoke, 459 jet exhaust, 696–698 burst, 722 / 686 glows | record + the spawner's draws (16: the 0.9 / 0.95 / 1.0 throttle, then `rand()`; 22 / 26: `rand()`); the types are not simulated. Types 4 / 8 / 15 go through `fx::part04` / `part08` / `part15` (simulated) |
| `projectile::in_world` | the [2, 1021]³ test | 459, 696–698 | |
| `Globals::cutscene` / `scripts` | `0x15f404`; `SetState` + `CameraScript` / `CameraScript2(0)` | 688 fly-by, 459 camera cuboid | `ScriptRequest::{Start, End}`: the tick's cinematic hand-off (`rc_game::cinematic::from_creature`) turns them into Ratchet's hold state and the script camera |
| `react::*` | the class reaction table (`lvl.vtbl` word 3), `0x304168`, `0x304390`, `0x3044a0`, `0x304690`, `0x305260`, `0x3051a8`, `0x304798`; the damage record's lure +0x18 | 577 (state 7), 572 / 866 (state 0xe) | §8; the Suck Cannon's and the Taunter's side is hero_gameplay.md §10 |
| `Globals::ripple_z` | `*(0x1612d0 + i·0x1190 + 8)`: ripple patch heights | 572 fall-out rule | empty on Novalis (no amoeboid uses it); the water set-up of levels 5 / 11 has to fill it |

Particle types the creatures spawn: 2 (amoeboid goo, `fx::goo_burst` = 0x2ef770), 8 (explosion puffs), 15 (explosion
streaks, with their child streaks), 52 (amoeboid drips, `fx::goo_drips` = 0x2ef560, flat quads) — all ported
(2026-09-28, particles.md "The Bomb Glove explosion and the effect particles"): the spawners fill the game's records and
make their draws at the game's point; the updates run and draw. `fx::part_unported` remains for the projectiles' types.

The pvar header of mode-0x20 creatures (`creature::header`): +0x00 damage record, +0x0c hit flash, +0x10 knockback,
+0x14 extra, +0x18 walker (self-relative offsets as the loader's pointer fixups leave them).

## 3. The classes

### Critter 577 (`GroundCritterUpdate` 0x2efc60)

See the module doc of `classes/critter.rs` (states 0–0x11 and 99, pvar map). The first visit creates 14 of the 68:
two groups near the landing pad area (instances 579 / 591 in group 1 with path enemy 309, 590 / 592 in group 2), six on the
ledge at (142, 215, 42) and four walkers (+0x20a = 1) at (108, 99, 57.6) / (117, 94.6, 57.6) / (234, 108, 75.5) /
(235, 109, 75.5). Hover draws: 3 every `ticks(120)` per hoverer (the "wander pick" of trace_results_novalis.md).

### Amoeboids 572 / 865 / 866 (`AmoeboidUpdate` 0x2edca0)

See the module doc of `classes/amoeboid.rs`. Novalis: five big ones in groups 22 / 28 / 41 / 46 / 47 at (32–70, 108–142,
35.5–40), each group with two 865 and four 866 waiting; arena paths 69–77, target polygons 70 / 73 / 79.

### Robot trooper 459 (`PathEnemyUpdate` 0x2e6bf0) and its fire glob 722 (`0x2fa068` / `0x2fa4c0` / `0x2fa1b0`)

See the module doc of `classes/path_enemy.rs` (states 0–3, 6, 7, 9–0xc, 0xe; pvar map). Shared: the resolver (column
4, mask 0x210000), the knockback flights (death 0xe: `40·dt²` gravity, sequence 9; stagger / knock-down 0xc: sequences
0xc / 0xe), the hit flash (+0x60), `SpawnBeamExplosion(0, 0, 2, 1, 9, 1, 15, …, 5, 2, 4, sound 4, shake, debris 1)`
(stack arguments read from the disassembly at 0x2e87a8), `BreakFxB` 1736–1738 / 1770, `SetDeathBits`, `target::acquire`,
`joint_point` (list 0 the muzzle, 1 the jetpack). Class-own: the patrol pick `0x2e6790` (`path_enemy::pick`), the jet
exhaust `0x2e6518` (3 type-22 records, 8 draws each a tick while jetting), the fire glob: `24·dt` per tick towards a
point `+0x198` = 4 ahead, gravity `2/ticks(10)²`, life `ticks(10)` — a 4-unit flame, so a trooper hurts Ratchet only
close up (its range +0x1a0 of 13–17 is the *aiming* range); hit template flags 0x10001, type 1 / 1, damage 1, push
(vel.xy, 1, exact). A trooper that Ratchet comes within 4 of and that cannot fire hops away along its path.

### Dropship 666 (`DropshipUpdate` 0x2f4960)

See `classes/dropship.rs`. Uses the flyer driver `FlyerPathDriver` 0x2f5168 (`classes::flyer::driver`, made public).
Seats `0x20b360` = (−1 + k/2, 0, 1.75). The carried children run no update (mode 6); the ship advances their animation
and builds their matrix.

### Gunship 688 (`GunshipUpdate` 0x2f7728), shell 686, fire 700, embers 696–698

See `classes/gunship.rs`. **The scripted fly-by (for the cutscene system):** trigger cuboid P+0x170 (Ratchet's feet),
camera cuboid P+0x174 (the script camera's place and Euler), timer P+0x186 = `ticks(510)`; the first 120 ticks run in
the trigger tick (the update loops while the timer is above `ticks(510) − ticks(120)`); `ScriptRequest::Start {
hero_state: 0x72, cuboid, centre, euler, ticks }` on the trigger, `ScriptRequest::End` when the timer runs out (then the
spawn id's killed / death bits, so a second visit wakes the troopers at once and deletes the ship); while it runs
`Globals::cutscene` (0x15f404) is set: the shells then make half the trail, 2 spark pairs, 10 streaks and hit mobys in
radius 1 (flags 9, Ratchet ignored; template flags 0x810001, damage 0: the bridge pieces 709–711, cuboids 19–21) with
sound 2 on a bridge piece, else 1. The gunship's side needs from the camera only the cuboid's centre and Euler.

### Spawner 815 (`EnemySpawnerUpdate` 0x3021b8)

See `classes/enemy_spawner.rs`: the state 0 → 1 step and the gate (`MobyGroupCount` 0x26e008, limit 5); the group
re-creation `0x302328` (an amoeboid respawn) is not ported — no Novalis spawner has a group.

### Amoeboid fall-out (`0x2eec68`)

+0x258 is a ripple-patch index (not a spline): an awake amoeboid below that patch's height bursts (`0x2ef770`) and is
deleted without bolts; at init +0x258 ≠ 0 also means health 1 and awake. Ported in `classes/amoeboid.rs` over
`Globals::ripple_z`. Only reader besides the init (verified by a scan of the function and its callees for `lw …,
600(…)`).

## 4. Registration (`classes/mod.rs`, class-keyed)

| update | classes | levels whose table runs this function for them |
|---|---|---|
| `ClassUpdate::Critter` (0x2efc60) | 577 | 01 |
| `ClassUpdate::Amoeboid` (0x2edca0) | 572, 865, 866 | 01, 05, 11 |
| `ClassUpdate::FxPiece` (0x30cd18) | 1736–1738, 1747–1749, 1761–1763, 1770, 1814, 1815, 1817 | 01 (other levels use other class numbers for this function: not registered) |
| `ClassUpdate::ExplosionLight` (0x2f3748) | 639 | all 19 |
| `ClassUpdate::PathEnemy` (0x2e6bf0) / `PathEnemyShot` (0x2fa4c0) | 459 / 722 | 01 |
| `ClassUpdate::Dropship` (0x2f4960) | 666 | 01 |
| `ClassUpdate::Gunship` (0x2f7728) / `GunshipShell` (0x2f6a30) | 688 / 686 | 01 / 01, 14 |
| `ClassUpdate::GunshipFire` (0x2f8c58) / `GunshipEmber` (0x2f8718) | 700 / 696, 697, 698 | 01 |
| `ClassUpdate::EnemySpawner` (0x3021b8) | 815 | 01 |

`needs_joint_lists` now also covers 459 (muzzle, jetpack) and 688 (the two guns).

Engine seam: `crates/rc-engine/src/moby_spawn.rs` keeps the spawn state of a class with a ported update plain while the
game tick runs (`RC_PLAY` ≠ 0); otherwise `rc_formats::moby_spawn::initial_state`'s 577 lift / 865–866 drop and hide were
applied on top of the classes' own init (hoverers 12 up and never noticing Ratchet, split children never shown).

## 5. Verification (2026-09-27)

**Unit tests** (`moby_update::creature::tests`, 9): rotation wrap; `turn_toward` brakes onto the heading (at most the
game's one-step residual); `rate_slot`; the resolver (wrench kind 1, `ticks(15)` cooldown, the push redirected away from
Ratchet at unit xy length, the second hit inside the cooldown loses its damage, the one after keeps it); the red flash ramp
(full at 4 ticks, back to the ambient after 15); arena walls / clamp / the way round a wall through the waypoint graph; a
critter on a hand-built floor: 1 load draw (the mirror), 3 draws per hover pick, lands, walks, bites Ratchet (record for his
moby: attacker, flags 1, damage 1); a wrench-like hit: death flight (not targetable), three pieces 1747–1749, the explosion
light, bolts.

**Novalis headless** (`tests/creatures_novalis.rs`: the loader's spawn test, `load_level_mobys`, the scheduler's load pass
and moby loop, the moby collision, the hero with the wrench through the engine's hit sink): 14 critters and 5 big
amoeboids awake at the start; next to critter group 1 the hoverers land, walk and bite, Ratchet goes hurt (0x16) and loses
all 4 HP (death 0x3d) in 600 ticks; with □ every 24 ticks two critters die by the wrench (death flight 99), 6 pieces, 2
explosion lights, 14 bolts dropped; next to amoeboid group 22 the big one splits into two 865 and those into 866 (states 2,
3, 4, 6, 7, 8, 9 seen, 7 mobys involved), Ratchet is hit 4 times, 9 bolts. Every run twice: identical per-tick rows.

**Rand stream** (`compare-novalis-spawn` on both savestate dumps, before = HEAD, after = this pass; the window is the
second savestate minus the first, 1072 ticks):

| | load pass | first savestate (2438 ticks) | second (3510) | window per tick |
|---|---|---|---|---|
| before | 1408 | 80 054 | 115 881 | 33.42 |
| after | 1457 (+49: 14 critter mirrors, 35 amoeboid glow phases) | 79 849 | 115 901 | 33.63 |
| game | | 254 936 | 311 008 | 52.31 |

Measured inside the classes (a scratch-only draw counter around the dispatch): critters 86 draws in the first run (14 at
load, 72 in ticks), 167 in the second: **81 in the window (0.076 per tick)**, against the game's ≈ 100 (0.1 per tick, 4
critters re-picking hover spots, trace_results_novalis.md "Second savestate"). Amoeboids: only their 35 load draws (none
awake near the camera in either state, as in the game). The rest of the change (+144 in the window, −254 in the first
run's ticks) is the other RNG-dependent consumers (drip timing, glints) moving with the shifted stream.

**Hero digest** (`hero_novalis.rs::novalis_hero_digest`, `RC_HERO_DIGEST`): byte-identical before / after (2800 lines).

**Engine** (`RC_SCENE=0`, `RC_AUDIO=0`, `RC_HERO_AT=146.5,160.5,40.5,3.14159`, □ every 24 ticks): critters walking up
(frame 140), a critter in its death flight with the bolts flying to Ratchet (185; two runs identical PNGs), the explosion
flash and the bolt counter (245); `RC_HERO_AT=55,145,40.6,-2.3`: a medium amoeboid after the split (100) and a strike
(265). A final run with the arrival scene and audio (frame 1800) ran through the scene as before.

### Enemies (2026-09-28)

**Headless** (`tests/creatures_enemies_novalis.rs`, the same level harness plus the trigger volumes, the joint lists and a
MobyProc +0x31 stand-in; every run twice, identical rows): the dropship brings 309 / 311 / 312 from the spawn point and
each lands within 1.5 of its drop cuboid (the RAM comparison below: within 0.015); trooper 310 wakes, flies down,
fires 4 globs, Ratchet hurt 3 times (4 → 1 HP, state 0x16); the wrench kills 310 (stagger 0xc, death 0xe, four pieces,
bolts); a bomb kills it (on the ground the death flight ends in its first tick); the bridge fly-by holds Ratchet in 0x72
for ticks 0–388 (510 − 120 run at once), fires 5 shells, wakes 320 / 321 / 333 and deletes the gunship; gunship 694
fires on the town (type 4 / 15 records). Unit tests (`creature::tests`): `spring_turn` = `SpringTurn2` bit for bit, the
particle records' draws, the sweep and the template.

**Against the game's RAM** (`compare-novalis-spawn`, the spawn savestate, after 2438 ticks): class 459 state 25/25 (was
0/25), mode 25/25 (0/25), position 20/25 (4/25; the three dropped troopers within 0.015 of RAM, yaw within 2e-4, the
other two differ by one float ulp of the +20 lift); 666 state / sequence equal, the flying dropship's position within
1e-4; 688 state and mode equal (0/2 before), gunship 695 hidden and waiting, 694 3.4 units along its path from RAM; 700
mode 6/6 (0/6).

**Rand stream** (`compare-novalis-spawn` on both savestate dumps; before = this tree with the enemy classes switched
off for the measurement, after = with them; window = second savestate minus first, 1072 ticks):

| | first savestate (2438 ticks) | second (3510) | window per tick |
|---|---|---|---|
| before | 83 356 | 120 478 | 34.63 |
| after | 131 520 | 173 466 | **39.13** |
| game | 254 936 | 311 008 | 52.31 |

The first run's +48 000 are the dropship arrival (the jet exhaust of three troopers), the town bombardment (686 trails,
impact bursts) and the fires' smoke and embers (the ember burst: 40 × 11 draws). Still missing in the window: the sound
layer's 3 per tick (wiring), the foam, the idle fidgets, the particle *updates* of types 16 / 22 / 26 (killed on their
first update) — see trace_results_novalis.md.

**Fixture / digest:** `tools/trace/tests/novalis_spawn.rs` passes unchanged (the ported classes change no slot fact at
load: class, spawn id and index are the loader's); `novalis_hero_digest` identical (md5 4d404f3d…, the same as the
other agents' runs today: the digest runs without mobys).

**Engine** (`RC_SCENE=0 RC_AUDIO=0`, scratch `novalis_enemies/shots/`): `RC_HERO_AT=160,140,60.5,2.27` frame 240 the
dropship over the pad with a trooper leaving it, frame 300 three troopers gliding down; `130.94,174.93,40.5,2.85`
frame 200 a trooper spraying a glob at Ratchet (two runs: identical PNGs); `125.5,175.8,40.5,0` with □ every 20 ticks,
frame 250: the trooper's death explosion, pieces and bolts; `72.26,189.26,47,1.09` frames 150 / 300: the bridge fly-by
through the cutscene camera with the shell bursts. A final run with the arrival scene and audio (frame 2000) ran
through.

## 6. Not done / open

* Classes: 1818 (gadget-gated); 815's group re-creation `0x302328`; (the bridge pieces 709–711 now break under the
  shells: §7); the dropship's camera moby P+0x180 (−1 on Novalis); the gunship's kill effects
  (skill point, level sound 1, banner 0x53d6, debris 1510 `0x30be70`: no Novalis weapon has mask 0x800000).
* Particle types 16 (smoke), 22 (jet exhaust / ember smoke) and 26 (the globs' and shells' glow): records taken and the
  spawners' draws made, not simulated or drawn (the updates' draws are missing).
* `Globals::ripple_z` must be filled by the water set-up of levels 5 / 11 for the amoeboid fall-out rule.
* ~~Suck cannon capture (577 state 7, 572 state 0xe, `0x305260`)~~ (done 2026-09-28: §8); the big-head manipulator (`0x278720`, a cheat flag);
  the moby shadow probe (`0x26f020` / `0x26eff8`: moby +0x84 / +0x88).
* The knockback's burn sparks (`0x271258`) and water splash (`0x2ff768`); the beam explosion's debris burst (`0x2c4c20`).
* The explosion light lights mobys (the point-light merge) but not the world (tfrag / tie / shrub point-light relight).
* (Fixed 2026-09-28) The explosion flashes (classes 0x70, 1192) now draw translucent: +0x23 below 0x80 is drawn as an
  alpha-blended, non-depth-writing moby (moby_render_notes.md §8); verified from MobyProc's vertex alpha and the
  flashes' spawners, which set +0x23 = 0x20..0x40.
* Decoy targets (0xcb / 0x76c / 0x10e) are searched in the moby table, not the game's gadget lists (none exist yet).

## 7. World props and effects (the non-enemy survey classes, 2026-09-27)

Every class below was read from the level01 disassembly (Ghidra MCP, read-only) and checked against the level tables
(`lvl.vtbl` of all 19 overlays × `tools/ghidra/names/clusters.tsv`): the listed levels are every level whose class table
runs the same function for the same class, so the class-keyed registry is right on all of them. Standard `f32`; the
formulas, constants, sounds, order and `rand` draws are the game's. Tests: `crates/rc-game/tests/novalis_world.rs`
(7 tests: init, cycles, draws and determinism of each).

**What each thing is in the game, and which port system it uses** (no new system where one existed):

| item | what it is in the original | port system | new / reused |
|---|---|---|---|
| 760 "waterfall foam" | **fire and smoke fields** on the bombed city roofs (and Veldin 00, level 14): a moby with no class blob that owns elements in global arrays and draws them in a list-2 draw callback (`0x2fe080`) as camera-facing `FastDrawQuadReal` quads (orange additive flames FX 46, grey smoke FX 47, a six-quad smoke curtain); element timers and respawns (`rand`) run in the callback | `classes/fire_field.rs` (state) + `classes/draw_callbacks.rs` (the game's two per-frame callback lists, run in `Scheduler::tick`) + `rc-engine` `water_render.rs` "Fire fields" (`fx_prim.wgsl`, the shared draw-callback material of `fx_draw.rs`) | **new**, general (the draw-callback lists serve any class; the renderer reads the per-level quad tables from the overlay) |
| 809 | the fire fields' global smoke scroll and their timer constants | `classes/fire_field.rs` | new (same module) |
| the waterfall foam | the ripple manager 751's zone-5 spray at the foot of the fall: one flat foam ring (particle type 57, kind 1) a tick on a row cycling every 20 ticks, and up to 20 mist puffs (type 56) at odds 1/32 | the particle system (`particles/type56.rs`, `type57.rs`, registered in the type table) spawned from `rc_game::water` (751's tick) | new types; existing 751 port and particle renderer (kind 1 is drawn) |
| the waterfall itself | 676 strips 4 / 6: static strip meshes with a two-layer UV scroll (world_animation.md §1) | `water_render.rs` strips (already ported) | reused, unchanged |
| 705 spinners | a moby: blades turning at a speed that ramps ±1, holds 90 ticks, reverses; its sphere hits what touches it (flags 0x10001) | `classes/props.rs` | new class |
| 703 / 715 elevators | a moby: vertical ping-pong with waits and sounds, carrying riders (`CarryRiders`) | `classes/props.rs` + `triggers::carry_riders` / `hero::platform` | new class, reused carry |
| 768 / 769 doors | a moby: the halves slide ±2 along the yaw when Ratchet is within the radius | `classes/props.rs` | new class |
| 701 collapsing platforms | a moby: Ratchet on the trigger → rumble (camera shake), fall 20, rock bits | `classes/props.rs` + `gunship::spawn_ember` + `World::shake_camera` + `cinematic` | new class, reused debris / shake / cinematic |
| 1042 shootables | deleted by a wrench-kind hit | `classes/props.rs` | new class |
| 704 / 709–711 / 729 / 778 | breakables bursting into rock bits (the ember classes 696–698) and type-22 smoke puffs | `classes/breakables.rs` + `gunship::spawn_ember` + `projectile::part(T22)` + `crate_::set_death_bits` | new classes, reused FX |
| 779 | the spray a broken pipe 778 leaves: a loop sound and two type-22 puffs a tick | `classes/breakables.rs` | new class |
| 754 / 1813 / 1816 | pots / boxes: `BreakFxA` (bolts), `BreakFxB` (shards 1817 / 1815: `fx::break_piece`, ported), `BreakFxC` (the remains 1816, whose update is empty) | `classes/breakables.rs` + `crate_::bolt_burst` + `creature::fx` | new classes, reused FX |
| 660 exhaust "rate" | the class-27 emitters riding the flyers, one type-6 puff a tick while in view | `particles/type06.rs` + `classes/flyer.rs` (unchanged) | reused: already exact (trace_results_novalis.md §"World props") |

| update | classes | levels | file |
|---|---|---|---|
| `ParticleFieldUpdate` 0x2fdbc0 (+ draw callback 0x2fe080) | 760 | 00, 01, 14 | `fire_field.rs` |
| `TextureScrollUpdate` 0x2ba658 | 809 | 00, 01, 14 | `fire_field.rs` |
| 0x2f9bf0, 0x2f95c0, 0x2fed68, 0x307c48, 0x2f9080 | 705, 703 / 715, 768 / 769, 1042, 701 | 01 | `props.rs` |
| 0x2f9810, 0x2f9d80, 0x2fa800, 0x2ff860, 0x2ffb28, 0x2fd9a0, 0x30d0f0, 0x30d200 | 704, 709–711, 729, 778, 779, 754, 1813, 1816 | 01 | `breakables.rs` |

**Fire fields in detail.** Pvar layout, init, callback and 809: module doc of `classes/fire_field.rs`. The init
allocates the element ranges from `0x161390` in instance order (809, instance 2, resets it first); each flame element
starts in its wait of (j + 1)·60 ticks and then cycles every (j + 3)·60 ticks (fade in 60, wait, fade out 60, respawn:
6 + 1 draws). The update registers the callback only while the sphere over the cuboid passes `FastBSphereCheck(255)`,
so off-screen fields freeze. The load pass draws 7 per element: 952 on Novalis. The engine draws the fields with the
state of the last callback run (one frame behind the PS2's draw-time update; the drain runs once per tick, which keeps
the effect frame-rate independent). Additive in linear light (the native `AdditiveNoZ` rule, not the particles'
display-byte layer). Levels 00 / 14: 6 / 46 fields, 83 / 1024 elements; level 00's instances 163 / 164 (+0x42 = 3)
spawn type-23 smoke particles (records and draws; type 23 is not simulated).

**Not ported (this survey):** 604 (commanded NPC, driven by other code's commands), 605 (nearest-area marker: writes
0x141390..98, whose reader is not ported), 613 (water current: hero push fields 0x13f4e4 / 0x13f528 / 0x141608 the hero
block lacks), 695 (floating pushables: `SetWaterLevel` and the `0x26d270` move-collide), 750 (infobot) / 1341 (help
hints) / 832 (RC range) / 1546 (cutscene FX), 1134 (gold bolt: the idle glows need particle type 59 and per-level
constants; the pickup is a cinematic), 1504 (wandering point light: the bank exists, the colour walk is not traced),
1848 (env-map overlay renderer), the 701 camera look (`FUN_002f9000`). Other agents own 11, 774, 280 / 641 / 665,
737, 730 / 790, 746 and the enemies.

## 8. Weapon reactions (2026-09-28)

How a weapon reaches a creature besides a hit (hero_gameplay.md §10 for the weapons): the **class reaction table** (the
third word of the class's `lvl.vtbl` entry; only the Suck Cannon uses it) and the damage record's **lure** +0x18 (the
Taunter). `creature::react` ports the table's shared handlers once; a class supports the Suck Cannon by having a table
(its three-line wrappers are `react::Wrappers` rows: held state, the amoeboids' 866-only start, the bounce sound, the
sequence table, the suck record's offset = the header's +0x14 record) and by calling `react::carried` from its held
state: 577 (state 7; a landed let-go → 0xe with the hover wait `ticks(180)`), 572 / 866 (state 0xe → 1). The level's
tables are matched against level 01's by `react::tables_from_overlay` (Rilgar: {270, 572, 866}; 865 keeps the default
and is never taken; 459 too). The lure: 577 (+0x38 → +0x214 = `ticks(240)`), 572 family (+0x38 alert), 459 (+0x38 →
+0x1e2 = `ticks(600)`) already read and clear it; the Taunter now writes it. Test: `tests/hero_reactive_novalis.rs`.

