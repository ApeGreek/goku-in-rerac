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
| 459 | 25 | 0x2e6bf0 | enemy | path enemy (grabs Ratchet, hit reactions) | not ported |
| 666 | 2 | 0x2f4960 | enemy (spawner) | dropship: flies a path, drops ≤ 8 children (0x1ab / 0x154 / 0x1cb) | not ported |
| 688 | 2 | 0x2f7728 | enemy (scripted) | gunship volleys (686 projectiles, 700 smoke) | not ported |
| 815 | 6 | 0x3021b8 | enemy spawner | spawns its pvar class while its group is empty | not ported |
| 1818 | 1 | 0x30df40 | critter (controllable) | the "mouse" that takes control of Ratchet | not ported |
| 11 | 1 | 0x2bb128 | other | Gadgetron vendor | not ported |
| 604 | 1 | 0x2f2b68 | other | commanded NPC | not ported |
| 605 | 20 | 0x2f2eb8 | other | nearest-area marker (hidden) | not ported |
| 613 | 4 | 0x2f3120 | other | water current | not ported |
| 730 / 790 | 1 / 1 | 0x2fad68 | other | mission NPCs | not ported |
| 737 | 5 | 0x2fb5b0 | other | camera trigger | not ported |
| 750 | 1 | 0x2fbf80 | other | infobot | not ported |
| 774 | 1 | 0x2ff118 | other | talking NPC | not ported |
| 832 | 1 | 0x302648 | other | RC-vehicle range limiter | not ported |
| 1134 | 3 | 0x307ca0 | other (pickup) | gold bolt | not ported |
| 1341 | 1 | 0x30acb8 | other | help-hint director | not ported |
| 280 | 2 | 0x2e0c68 | prop | bolt crank | not ported |
| 641 / 665 | 1 / 4 | 0x2f4348 / 0x2f4710 | prop | crank-driven rotator / slider | not ported |
| 695 | 9 | 0x2f8268 | prop | floating pushable | not ported |
| 701 | 3 | 0x2f9080 | prop | collapsing platform | not ported |
| 703 / 715 | 2 / 5 | 0x2f95c0 | prop | elevators | not ported |
| 704 | 3 | 0x2f9810 | prop | breakable rock | not ported |
| 705 | 5 | 0x2f9bf0 | prop | oscillating spinner | not ported |
| 709 / 710 / 711 | 1 / 1 / 1 | 0x2f9d80 | prop | breakable by 686 | not ported |
| 729 | 1 | 0x2fa800 | prop | big breakable wall | not ported |
| 746 | 2 | 0x2fb8a8 | prop | hinged bridge | not ported |
| 754 | 12 | 0x2fd9a0 | prop | breakable pot | not ported |
| 768 / 769 | 4 / 4 | 0x2fed68 | prop | sliding door halves | not ported |
| 778 | 1 | 0x2ff860 | prop | breakable prop | not ported |
| 1042 | 12 | 0x307c48 | prop | shootable | not ported |
| 1813 | 4 | 0x30d0f0 | prop | breakable crate variant | not ported |
| 66, 310, 687, 742, 747, 780 | 1, 1, 1, 5, 3, 2 | none | prop | static, no update (mode 2 as in the game) | — |
| 676, 678 | 1, 1 | 0x2f6128 (undef), 0x2f6180 | effect | custom-draw water meshes | not ported (drawn by the water renderer) |
| 700 | 6 | 0x2f8c58 | effect | impact smoke emitter | not ported |
| 760 | 4 | 0x2fdbc0 | effect | waterfall foam / mist | not ported |
| 761, 1225 | 1, 1 | 0x2feb58, 0x309c98 (undef) | effect | strip water | not ported (drawn by the water renderer) |
| 809, 1848 | 1, 1 | 0x2ba658, 0x30f208 | effect | texture / UV scrollers | not ported |
| 1504 | 1 | 0x30b618 | effect | wandering point light | not ported |
| 1546 | 1 | 0x30c190 | effect | cutscene FX driver | not ported |

Spawn-only classes the ported creatures create or wake: the body pieces 1747–1749 (577's death, `FxGroupUpdate`
0x30cd18), the explosion light 639 (0x2f3748), the explosion flashes 0x70 (ported before, `classes::debris`), the bolts
13–16 (ported before).

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
| `fx::beam_explosion` | `SpawnBeamExplosion` 0x273310 | body pieces | damage sphere, type-15 streaks, type-11 pairs by camera distance, type-8 puffs, flashes, shake, sound (debris burst `0x2c4c20` and the light template 0x1b06d0 not ported) |
| `fx::break_piece` / `piece_update` | `BreakFxB` 0x278ad8 / `FxGroupUpdate` 0x30cd18 | 577 death | 8 draws per piece; flies, spins about its sphere, bounces, explodes after `randf(60, 120)` ticks |
| `fx::light_spawn` / `light_update` | `0x2f3570` / `0x2f3748` (class 639, all levels) | death explosion | timer, view test, position / radius / colour ramps and their draws; the light itself is not rendered |
| `fx::rate_slot` | `0x2efbf8` (n = 2) | 577 | busy-until table 0x161a80 (3 slots, 60 ticks) |
| `ground::ground` | `GroundHeight` 0x26e618 + `CollType` 0x2151d8 | 577, knockback | a miss reads CollType's stale output in the game; −1 here |
| `ground::key_time` / `passed_frame` | `MobyAnimKeyTime` 0x263920 / `0x2765b0` | 577 bite, 572 strike, knockback | |
| `region::*` | `0x26e6c0`, `0x276640`, `ClampToPath` 0x276820, `0x276a48`, `0x276c40`, `LineOfSightTest` 0x276fe8 | 572 family | arena walls (edges with a non-zero w), waypoint graph (point w = visibility bits) |
| bolt drop | `SetDeathBits` 0x26c250 → `BoltBurst` 0x275988 | 577, 572 family | reused: `classes::crate_::set_death_bits` |
| flashes | `FlashSpawn` 0x2c20e0 | explosions | reused: `classes::debris::flash_spawn` |
| sparks | `PartType11Spawn` 0x27f8f8 | explosions | reused: `World::part11` |
| class sounds | `PlayClassSound` 0x2a1618 | 577 (death), 572 (hit 0, landing 4) | reused: `World::play_sound` → the audio layer's `ClassSoundSink` |

Particle types the creatures spawn that have no port yet: 2 (amoeboid goo), 8 (explosion puffs), 15 (explosion streaks),
52 (amoeboid drips). Their pool records are taken and the spawns' own draws made at the game's point
(`fx::part_unported`); the particle system kills them on their first update.

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

## 4. Registration (`classes/mod.rs`, class-keyed)

| update | classes | levels whose table runs this function for them |
|---|---|---|
| `ClassUpdate::Critter` (0x2efc60) | 577 | 01 |
| `ClassUpdate::Amoeboid` (0x2edca0) | 572, 865, 866 | 01, 05, 11 |
| `ClassUpdate::FxPiece` (0x30cd18) | 1736–1738, 1747–1749, 1761–1763, 1770, 1814, 1815, 1817 | 01 (other levels use other class numbers for this function: not registered) |
| `ClassUpdate::ExplosionLight` (0x2f3748) | 639 | all 19 |

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

## 6. Not done / open

* Classes: 459 (path enemy), 666 (dropship), 688 / 686 / 700 (gunship volley), 815 (enemy spawner), 1818.
* Suck cannon capture (577 state 7, 572 state 0xe, `0x305260`); the +0x258 fall-out of the amoeboid; the big-head
  manipulator (`0x278720`, a cheat flag); the moby shadow probe (`0x26f020`: moby +0x84 / +0x88).
* Particle types 2, 8, 15, 52 (records taken, draws made, not simulated or drawn); the knockback's burn sparks
  (`0x271258`, type 4) and water splash (`0x2ff768`); the beam explosion's debris burst (`0x2c4c20`) and light template.
* The explosion light is not rendered (no dynamic point lights in the renderer).
* The explosion flash (class 0x70) draws opaque in the engine; the game very likely draws it translucent (inferred; renderer, not this pass).
* Decoy targets (0xcb / 0x76c / 0x10e) are searched in the moby table, not the game's gadget lists (none exist yet).
