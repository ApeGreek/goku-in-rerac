---
status: open
job: hero-consumers
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [hero, classes, world, particles]
---

# The body system's consumers: Giant Clank's missiles and beam, Quartu's Giant Clank NPC, the switch's item pass, the riding floats, the surface-jump underwater store

## 1. Summary

- **Giant Clank's missile, class 0x100** (`units::giant_missile`, levels 15 / 18, created by the hero code): spawn
  L15 `0x29d408` / L18 `0x2aa0f8` with its own auto-target search `0x2658d8`, update `0x29d6c0` / `0x2aa3b0`. A compiled
  copy of the Devastator missile with its own constants (no gold, no frame-load throttle, top speed 100 u/s from rest,
  life `ticks(150)`, range 260 (2D) from Ratchet, the platform motion only, unclamped streaks, blast flags 0x810000,
  scale × 3 × 2). Per-class code: the shared helpers (intercept, `SpringTurn`, `area_hit`, fireball, flashes, light,
  colour tables) are the port's.
- **Giant Clank's head beam, class 0x5f3** (`units::giant_beam`): create `0x29ec00`, point `0x29ece0`, end `0x29ed40`,
  update `0x29edb0` (L18 `0x2ab8f0` / `0x2ab9d0` / `0x2aba30` / `0x2abaa0`). Held at the point Giant Clank's state
  0x61 gives every tick (charging: type-31 sparks into it, a point light growing to radius 7, a type-5 flash, the
  screen record), then flies along the hero's yaw at 40 u/s for `ticks(60)` hitting everything in a growing sphere
  (20 damage, flags 0x8b0000) with glows (26), streaks (15) and twinkles (69). The beam moby is a level word
  (`giant_beam::GLOBAL`). `hero::bodies::giant::after_update` now makes the missiles / shockwave / beam point / beam end
  in the hero code's order (`Spawn::BeamEnd` added for the hit intake's `0x2a9be0`).
- **Particle type 69's modes 1..4** (`particles::type69`): mode 1 / 2 / 3 copy a point out of the moby's pvar block
  (+0xd0 / +0x1f0 / +0xe0, `Particles::pvar_points` filled by `World::refresh_particle_mobys` through
  `services::pvar_block_point`, which reads past a 0x80-byte block into the next slot's as the game's contiguous blocks
  do [L]); mode 4 follows the moby's x / y with z + 0.2 a tick.
- **Quartu's Giant Clank mission NPC, class 1446** (`units::quartu_giant_mission`, level15 `0x2ec760`, 1 instance):
  talk in his cuboid → mission done → kill the two groups as Giant Clank → after `ticks(120)` Clank leaves the body
  (`bodies::queue_leave`), Ratchet teleported, planet 16 unlocked, scene 1, movie 16, planet banner, scene 2, save.
  His head look-at is the talking NPCs' code with his own offsets: `talking_npc::look_at_layout` / `LookLayout` (the
  774 layout is now a data row, behaviour unchanged).
- **Finding (no code): the Clank-section starters on levels 0, 4, 7, 9, 13, 17 do not exist.** On those levels
  `SwitchCharacter` is called only by the hero code for the disguise (body 3, `0x53`), the checkpoint reload and the
  mode freeze's leave. Every class starter is ported (22, 1061, 1451 / 1899) but the boss 1422 (other lane).
- **The gold bolt's leave of the disguise** (`classes::gold_bolt`): with Ratchet in body 3 it now queues the leave
  (`FUN_00231450`, every level's copy) before its teleport (was an "unported" count).
- **The switch's item pass `0x231088`** (G-HERO-035): `items::slot_pass` (slots 0..3) made at the switch's +2 with the
  tick's item environment (`bodies::switch_character_with`, `HeroFields::run_calls_with`, the tick's closure).
- **The ride seam** (G-HERO-034): `HeroFields::ride` + `services::turn_about`; consumers 664 / 1293 / 1320 (level 09)
  and 1069 (level 07) in `units::riding_floats`.
- **The surface jump's underwater store** (G-REN-032): `SwimEvent::UnderwaterOff` → the engine stores
  `UnderwaterStore::Off` with the tick's counter.

Plug-and-play: a class that carries Ratchet itself calls `w.hero_fields_mut().ride(hero_pos, turn_about(...), dz)`;
another NPC with the look-at code adds a `LookLayout` row; a body-switching class keeps calling
`bodies::queue_switch` (the item pass now comes with it).

## 2. Where it lives

| what | file::fn | decomp |
|---|---|---|
| missile | `units/giant_missile.rs::spawn`, `search`, `update`, `trail`, `explode` | L18 0x2aa0f8, 0x2658d8, 0x2aa3b0 |
| beam | `units/giant_beam.rs::create`, `point`, `end`, `update` (`charge`, `fly`), `screen_flash` | L15 0x29ec00, 0x29ece0, 0x29ed40, 0x29edb0 |
| Giant Clank's spawns | `hero/bodies/giant.rs::after_update`, `beam_end`, `Spawn::BeamEnd` | L00 0x2a96f8 / 0x2a99b0 / 0x2a9b80 / 0x2a9be0 |
| type 69 modes | `particles/type69.rs::update`, `moby_of`; `particles.rs` `pvar_points`, `moby_refs`; `services.rs::pvar_block_point`, `refresh_particle_mobys` | L01 0x289df0 |
| 1446 | `units/quartu_giant_mission.rs::update` | L15 0x2ec760 |
| NPC look-at layout | `classes/talking_npc.rs::LookLayout`, `LAYOUT`, `look_at_layout` | L01 0x2ff3d0..0x2ff734 |
| gold bolt | `classes/gold_bolt.rs::start_cutaway` | L01 0x307ca0 |
| item pass | `hero/items.rs::slot_pass`; `hero/worn.rs::worn_slot_loop`; `hero/idle.rs::back_slot_loop`; `hero/bodies.rs::switch_character_with`, `SlotPass`; `services.rs::HeroFields::run_calls_with`; `tick.rs` | L01 0x231088, 0x231348 |
| ride seam | `services.rs::HeroFields::ride`, `turn_about` | L09 0x27fe88, L07 0x288968 |
| floats | `units/riding_floats.rs::float_664`, `float_1293`, `float_1069` | L09 0x2f86a0, 0x3091b0, L07 0x3112c8 |
| underwater store | `hero/swim.rs::underwater_check` (`SwimEvent::UnderwaterOff`); `rc-engine/src/gameplay.rs` | L01 0x2406b0 |
| Devastator's list | `hero/devastator.rs::already_targeted` (class 0x100 counted) | L01 0x274738 |

Coverage tables: the module docs of `giant_missile.rs`, `giant_beam.rs`, `quartu_giant_mission.rs`,
`riding_floats.rs`, `particles/type69.rs`, and rows +2 / HUD of `hero/bodies.rs`, rows 0x5f / 0x61 of
`hero/bodies/giant.rs`.

## 3. Behaviours to verify

### B1. Missile spawn: moby fields and the level scale
- **Claim:** `spawn(yaw, 0, 0.5, yaw, 0, owner, muzzle, None)` (L18 0x2aa0f8): `CreateMoby(0x100)`; update / draw
  distance 0xff, visible 1, state 0; scale = class scale × 3.0 (gp−0x57c8 / L15 gp−0x5780, image value 3.0) then
  doubled (× 6 total); rotation z = moby yaw, y = moby pitch; rows rebuilt; position = muzzle; pvars: owner = owner+1,
  lead 1, turn rates 0, speed 0, top 100·dt, yaw / pitch from the search, life `ticks(150)`, motion timer and start
  `ticks(60)` when |Ratchet's applied platform motion| > 0.1·dt else `ticks(1)`.
- **Setup:** a World on level 15 data (or synthetic), a hero (Giant Clank body or Ratchet) with `plat_applied` 0 / 1·dt.
- **Trigger:** call `giant_missile::spawn`.
- **Expect:** the values above; `cmd` (+0xbc) 0; no RNG draws when there are no targets.
- **Edge cases:** platform moving → motion 60; a world face between (owner.x, owner.y, muzzle.z) and the muzzle →
  `cmd` = 1 and position = the hit point (line flags 0, ignore = the hero moby).
- **Suggested method:** unit test on `spawn` with a synthetic table.

### B2. Missile target search 0x2658d8
- **Claim:** over the target list: skip dead (state ≥ 0x80), non-targetable, class type ≠ 5. Aim point = position +
  record height (0.5 without). 3D distance d < 7.5 and |diff(hero moby yaw, atan(target pos − hero pos))| < 60° and
  |elevation| < 60° → taken at once (yaw / pitch to it, aim height stays 0.5, search ends). Else if d ≤ best:
  `cone_miss(d, aim yaw, aim pitch + 0x13f634·0.5, …, radius byte, 20°)` < 20° and a clear camera line (flags 6,
  ignore owner) → aim turns to it, best = d, aim height = record +0x10 (0.5 without one). Blocked line → skipped.
  The target's record +0x1e |= 0x80; record +0x0b ≠ 0 → lead 0; +0x0c ≠ 0 → top = +0x0c·dt.
- **Setup:** synthetic table with 2–3 creature mobys (class type 5, targetable, a record), hero at the origin.
- **Trigger:** `spawn` with target None.
- **Expect:** the chosen target in `pv::TARGET`, aim point = its position + aim height, yaw / pitch toward it.
- **Edge cases:** the 7.5 hard lock beats a closer cone target listed later; two in the cone → the nearest (the aim
  changes after each accepted one); a blocked camera line; a target without a record (radius not subtracted).
- **Suggested method:** unit test.

### B3. Missile flight
- **Claim:** speed += (top − speed)/10 a tick; motion = platform motion × timer / start (FastDecTimer first); a live
  target after the first tick → toward its aim point (leading by the intercept when lead = 1; no lead past
  `ticks(150)`), else the fired yaw / pitch after the first tick, else its own rotation; `SpringTurn` (2π·dt², π·dt²,
  max 270°/s with no target word, 360°/s with one); step = polar(speed, yaw, −pitch); the trail; position += step +
  motion; any negative coordinate → deleted.
- **Setup:** missile spawned with / without a target; tick the update.
- **Expect:** speed after n ticks = 100·dt·(1 − 0.9ⁿ); position steps; the turn limits.
- **Edge cases:** the target dies (state ≥ 0x80) → fired yaw; the target word non-zero but dead still uses 360°/s.
- **Suggested method:** unit test on `update` (no collision).

### B4. Missile trail (every tick)
- **Claim:** draws in order: 3 × `randf(−1, 1)`, `randf(0.1, 0.2)`, `randf(0.1·dt, dt)`, `randf(0, 1)`; type 44 puff
  (40000, growth 1000, fall 0xb951b717, alpha 0x7f, 0x606060, `ticks(60)`, byte3 0x44, texture def[23]); type 44 at
  the missile (fall 0xb9d1b717, alpha 0x7f, 0xb0b0b0, `ticks(6)`); `rand_angle`; type 21 spark (20000, 0.02 along row 2
  turned about row 0, 0x4f007fff → 0x1fffffff, `ticks(5)`, split 1). No throttle (every tick, unlike the Devastator).
- **Suggested method:** unit test with a particle system; check record fields and the RNG sequence.

### B5. Missile collision and expiry
- **Claim:** `CollLine_Fix(old, new, 0, hero moby, tmpl)` with tmpl push = unit step (z 1, w 5627.97), flags 0x830000,
  damage 3, type 3 / 1, class 0x100, w20 1. Nothing hit: life −1; life out or xy distance to Ratchet > 260 → deleted, no
  blast. A world face of kind ≥ 1 → blast kind 1 at the hit; a face of kind < 1 → nothing; the hero moby → passes;
  the owner → ignored; another moby → blast kind 2 (it gets the line's hit record).
- **Edge cases:** spawn-time `cmd` = 1 with no hit this tick → blast with edge (1, 0, 0), normal (0, 0, 1), drift
  (0, 0, 8·dt) [L: the game's edge is uninitialised stack].
- **Suggested method:** unit test with a synthetic collision mesh / moby.

### B6. Missile blast
- **Claim:** `area_hit(2, pos, m, damage 3, push 1, z 1, ignore (itself on a moby hit), 0x810000, 3, 1)`; 10 type-15
  streaks (40000, 0x4f007fff → 0x1f00007f, life `rand_range(t60, t120)`, split; kind 1 in the face's plane:
  `rand_angle` then `randf(8.5, 16.5)`; else `randf`, `rand_angle` ×2; z += 5·dt; + drift; **no** 8·dt clamp); the
  fireball (3 × `randf(−1, 1)`, to the camera with z += d/2, (d/5)·dt and 2d·dt, clamp 10·dt, `rand_range(t60, t90)`,
  `bomb::fireball` type 0); rings (d < 6: trunc(d)/2 pairs else 3; slower by (7 − d)·dt within 7; 400000; colours
  SPARK_A / SPARK_B; the white half-speed partner); 10 type-8 puffs (200000, `randf(0, 3)·dt`, `rand_range(t20, t35)`);
  flashes (frame load < 0.95 and d > 9: two white 4.0 at 15 / 24 ticks; yellow 4.0 `ticks(20)`; white 2.0 `ticks(19)`
  always); `cmd` 0; shake up (0.4 − 0.0175·d, 0.050000012 from 20) `ticks(25)`; class sound 0; light `LIGHT_BOMB`;
  deleted.
- **Suggested method:** unit test (RNG order, particle counts by type, flash mobys, shake request, sound event).

### B7. Beam create / point / end
- **Claim:** `point(p)`: no beam → `create(p)`: `CreateMoby(0x5f3)`, the beam word = it, draw / update distance 0xff,
  visible 1, state 0, at p; screen words 0x15f324 = 0xff, 0x15f334 = 0xffffff, 0x15f338 = 0x1f00000042, 0x15f328 =
  0x15f320 = 0x15f330 = 0; charge 0; light slot = `alloc(radius 0, intensity 0, p, colour 0)` (−1 above frame load 0.8
  or 8 taken). Then (class 0x5f3) position = p, held = 1. `end()`: a beam of class 0x5f3 in state 0 → 0x15f320 =
  0x15f330 = 0, light freed, deleted, word 0; a flying beam (state 1) is left alone; another class → word 0.
- **Suggested method:** unit test on a World.

### B8. Beam charge (state 0, held, Giant Clank)
- **Claim:** charge 0 → 0x15f330 = 1, 0x15f334 = 0x00ffff00; charge += 1; k = min(charge / 30, 1); trunc(10k²) sparks
  (x, y, z `randf(−1, 1)`, green `rand_range(0x4f, 0xaf)`, blue `rand_range(0x4f, 0xff)` (both ≤ 0xff), length
  `randf(20, 40)`): type-31 line from the point into the beam, RGBA 0x7f·b·g·0x4f. Charge < 50: alpha of 0x15f334
  steps to 0x25 by `ticks(1)` (0xff with a zero packet → 0x25 first), packet = 0x1f00000042; else alpha 0xff. No
  light: every 4th tick (counter & 3 = 0) one is allocated; else the light at the beam, radius clamp(charge, 0, 50)·7/50,
  a type-5 flash (grow k·840000, size 0, rgb `rand_range` (0xf..0x2f, 0x2f..0x3f, 0x3f..0x7f), `ticks(20)`), r =
  `randf(0.8·r, 2·radius/7)`, g = `randf(min(0.8·g, r), r)`. Held cleared.
- **Edge cases:** held but body ≠ 2 → launches (as not held); 50-tick boundary.
- **Suggested method:** unit test (words, light slot, RNG order).

### B9. Beam launch and flight
- **Claim:** not held → 0x15f320 = 0x15f330 = 0, charge 0, velocity = (cos, sin, 0)(hero yaw)·40·dt, state 1, flight 0.
  Flight: f = 40·dt·t/2 + 3 (t before +1); the light radius ×0.4 a tick, freed below 0.0001; t > `ticks(60)` or body ≠
  2 or outside [2, 1021]³ after the step → the beam word's moby deleted, word 0 (light left). Else
  `area_hit(f, pos, beam, 20, 1, 1, None, 0x8b0000, 2, 1)`; type 26 glow (f·0.5·210000, colour at alpha 0x4f, life
  min(t60 − t, t5)); type 15 streak (210000, polar `randf(f·dt, 2f·dt)`, colour at 0x7f → 0x4f7f4f2f, `ticks(45)`,
  split, texture def[11][0]); 8 twinkles (polar `randf(f·dt, 2f·dt)` velocity, position within `randf(0, f/2)`, size
  `randf(6000, 32000)·f·0.4`); 12 mode-3 twinkles at the beam (size `randf(80000, 120000)·f`, the third 180000·f when
  `randi(8)` = 0, timer `ticks(2)`, +0x30 = 1/t, colour 0x7f7f7f).
- **Suggested method:** unit test (RNG order, hit records on a synthetic moby inside the sphere).

### B10. Giant Clank's spawn order and the hit intake
- **Claim:** `after_update` makes `Spawn::Missile` (`spawn(yaw, 0, 0.5, yaw, 0, body, pos, None)`), `Shockwave`,
  `Beam` (`point`), `BeamEnd` (`end`) in queue order; a hit taking Giant Clank to 0x5d queues `BeamEnd` (the charging
  beam is deleted; a flying beam continues).
- **Suggested method:** hero-level test with a Giant Clank body (existing `hero_bodies` harness), state 0x5f key 4 →
  two class-0x100 mobys; state 0x61 → one 0x5f3 held, then launched after key 22.

### B11. Type 69 modes
- **Claim:** mode 0 unchanged; mode 1 / 2 / 3 → position = the moby's pvar point +0xd0 / +0x1f0 / +0xe0 (a 0x80-byte
  block reads the next slot's block at offset − 0x80·k); mode 4 → (moby x, moby y, old z + 0.2); then the box test
  (outside [2, 1021]³ → killed, no draw); a moby missing from the frames → unchanged + `unported_branch`.
- **Suggested method:** unit test on `type69::update` with `moby_frames` / `pvar_points` filled; one on
  `services::pvar_block_point`.

### B12. 1446 prologue and states 0..3
- **Claim:** (see the module table) no Giant Clank link (+0x14c = −1) → nothing; body ≠ 2 → Giant Clank's moby hidden
  (0x41) and collision off; state 9 → update distance 0, hidden, return; game mode 2 → hidden; else shown, class
  collision, shadow slab z ± 0.2. State 0: update distance 0xff, talk radius 20, talk registered; talk mission done →
  planet 16 locked: state 3 and the other live mobys with mission byte = +0x280 deleted; unlocked: state 9. State 1:
  Ratchet in cuboid +0x27c, group < 2 or 9, `talk_update` true → `SetMissionDone(+0x280)`, anim speed 0, state 2,
  mission mobys deleted. State 2 (no scene running / pending): anim speed 1; talk LAST ≠ 0 → 9; else checkpoint at
  cuboid +0x150 → 3. State 3 (body 2): the +0x158 moby deleted (not 0xfe / 0xfd), +0x158 = −1; both groups empty
  (`group_count(g, 0x40)`) → 4, timer `ticks(120)`.
- **Suggested method:** unit tests with a synthetic table / talk tables; or level-15 harness.

### B13. 1446 finale (states 4..8)
- **Claim:** timer out → `HeroCall::LeaveBody`; Giant Clank's anim hard cut to (0, 0); talk node 1, auto 1; +0x140 = 1;
  `UnlockPlanet(16)` (banner unless on 16); `SetMissionDone(+0x284)`; music request (0, 5); cuboid +0x154 → hero
  teleport (centre, Euler, state 0, camera reset) and checkpoint there; → 5. 5: scene 1 → 6. 6 (mode ≠ 2): movie 16 →
  7. 7: mode ≠ 2 → planet banner 16, scene 2; then save, → 9 (mode 2: save and → 9 only). 8: save → 9.
- **Suggested method:** unit test on the request lists (`svc.cinematic.requests`, hero fields calls).

### B14. Head look-at layout
- **Claim:** 774 unchanged (same records, targets, order; regression); 1446 uses pitch record +0x160 (list 1), yaw
  record +0x1e0 (list 0), glance +0x260, seen +0x274, glance timer +0x278, no sequence gate.
- **Suggested method:** the existing `talking_npc` test must still pass; a new one for the 1446 layout.

### B15. Gold bolt in the disguise
- **Claim:** `Hero::mode == 3` at the cutaway start → `HeroCall::LeaveBody` queued before the teleport's calls.
- **Suggested method:** unit test (hero mode set to 3 synthetically).

### B16. The switch's item pass
- **Claim:** a `SwitchCharacter` (class call or the checkpoint restore) runs `items::slot_pass` right after
  `put_away` and f13f6 = 1: holding a non-wrench item → target = wrench, restore = it, slot state 3, the item blends
  to sequence 2 over 2 ticks, the swap's fidget `rand_range(50, 90)`; then slots 1 / 2 / 3's loop parts. Without item
  data nothing happens.
- **Edge cases:** holding the wrench (no swap); empty hand; the back slot without its classes (skipped).
- **Suggested method:** tick-level test with item data (Blarg 1061 or Orxon 22 switch) comparing the hand slot state
  and the RNG sequence; unit test on `slot_pass`.

### B17. Riding floats 664 / 1293 / 1320
- **Claim:** see `riding_floats.rs` tables: init (664: z − 0.3, rest = z), ride timer `ticks(30)` with class sound 0 on
  the first touch, free: tilt springs to 0 (10°·dt², 0.01, 5°·dt) and z rises 0.3·dt (1293: 1.0·dt; more than 2 below
  rest: one more step, springs ×3, max 90°·dt) to rest, sound 2 crossing rest − 0.25 (1293: − 0.3) upward; ridden:
  springs to (−0.1·d·row1, 0.1·d·row0) (40°·dt², 0.01, 10°·dt), z −= 0.35·dt, sound 1 crossing downward, Ratchet's
  platform delta = turn_about(hero, pos, old tilt, new tilt) − hero, z + the drop; bob 0.14 at 30°/s, wobble 0.087 at
  18 / 28°/s (1293: 25 / 31°/s); rotation += tilt; `CarryRiders(+0x20, …)`. 1293 state 0 with a link: z − 5, random
  x / y tilt `randf(±90°)`, waits (state 1) until the link is deleted.
- **Suggested method:** unit tests on synthetic mobys and a hero standing on them (ground moby, air ticks 0); check
  `hero_writes.platform`.

### B18. Riding float 1069
- **Claim:** see the table: told to drop (+0xbc = 2) → state 2 with the delay +0xbd; rock (1° sin / cos of wrapped
  phases); the linked moby gets +0x19d and state 0xc; the fall (rot.x → 80°, z −= 3.1·sin(step), gravity +0xb4);
  below water − 15: deleted unless kept, else state 4 → 5; the float on the water (buoyancy 0.2, ±0.9, gravity 9.8,
  ≤ 5·dt up; ridden −0.25·dt within 2.6 of the water); the splash sound every `rand() % t20 + t20` on a crossing;
  ridden: platform delta 0 (the game's rotation pair is the same), tilt toward Ratchet ±0.03; tilt clamped ±30°.
- **Edge cases:** mission done at load → z − 5 and state 5 (or deleted); state 4 with no link [L: the game reads a
  null pointer].
- **Suggested method:** unit tests.

### B19. Surface jump underwater store
- **Claim:** under water (group 0x11) and more than 0.4 above the water level → `SwimEvent::UnderwaterOff` then
  `SetState(6)`; the engine stores `(counter − 1, Off)` in `WaterWorld::underwater_store`.
- **Suggested method:** hero unit test on `underwater_check` (the event); engine side: QA in game.

## 4. Shared code touched (regression risk)

- `moby_update/services.rs`: `HeroFields::ride`, `turn_about`, `run_calls_with` (and `run_calls` now delegates),
  `pvar_block_point`, `refresh_particle_mobys` fills `pvar_points` (more work per tick, no behaviour change for
  existing types), the `platform` doc row.
- `particles.rs` (`pvar_points` field, `moby_refs` lists type 69's moby) and `particles/type69.rs` (modes): existing
  type-69 callers use mode 0 only — the existing test must still pass.
- `classes/talking_npc.rs`: look-at generalised (`LookLayout`); 774's behaviour must be identical (existing test).
- `hero/worn.rs` (slot loop split out, same order), `hero/idle.rs` (`back_slot_loop` visibility), `hero/items.rs`
  (`slot_pass`), `hero/bodies.rs` (`switch_character_with`), `tick.rs` (the switch now runs the item pass: a switch
  with a non-wrench item now starts its put-away at once — intended, faithful).
- `hero/devastator.rs::already_targeted` counts class 0x100 too (only levels 15 / 18 have it).
- `classes/gold_bolt.rs`: mode 3 path (unreachable until the disguise exists).
- `hero/swim.rs` (new event), `rc-engine/src/gameplay.rs` (the store).
- `units/mod.rs`: six new `PORTS` rows (0x100, 0x5f3, 1446, 664, 1293 / 1320, 1069): `all_levels_smoke`'s unported
  counts drop on 07 / 09 / 15 / 18.

## 5. Not ported (don't test as working)

- Missile: the missile view's target bracket (`0x1f6c48`, table 0x169a40) — G-HERO-037 (unreachable in play).
- Beam: the screen flash's draw (0x15f320..0x15f33c are kept as words only) — G-REN-030.
- 1446: the big-head cheat's scene manipulator `0x2ec678` and its +0x1d0 scale — G-SAV-006.
- Item pass: slots 4..6 (nothing modelled).
- The disguise body 3 itself (G-WPN-006): the gold bolt's leave is never reached.

## 6. In-game QA spots (for the user)

- **Giant Clank's weapons:** `RC_LEVEL=15 RC_HERO_AT=130.5,219.5,27,1.57` — step onto the pad (1451), △ → Giant Clank.
  ○: two missiles from the arms per volley (big, 6× class scale), a long grey smoke trail and sparks, they home on
  creatures in a 20° cone (or anything within 7.5 in front), explode with streaks, a fireball, rings, puffs, flashes, a
  camera shake and sound 0; they vanish without a blast 2.5 s after firing or 260 from Ratchet. △: the head beam
  charges (blue lines drawn into a point 3 ahead / 5 up, a light growing, a flash), then flies forward for one second
  growing, smashing creatures and crates (20 damage) with glows and streaks; next △ only after ~5 s (lockout). Get hit
  while charging: the beam disappears. Same on `RC_LEVEL=18 RC_HERO_AT=604.1,724.5,97.5,1.57` (pad 1899).
- **Quartu's mission NPC 1446:** `RC_LEVEL=15 RC_HERO_AT=131.8,212.3,25.5,1.57` — talk (△) to the NPC next to the
  Giant Clank pad (his head follows you); after the talk become Giant Clank and kill the two enemy groups; 2 s later
  Clank climbs out, Ratchet is placed, scene 1, movie 16, the "Kalebo III" banner, scene 2, save log; the NPC is gone.
- **Gaspar's floats:** `RC_LEVEL=9 RC_HERO_AT=372.1,190.9,27,0` (664 row) and `RC_LEVEL=9 RC_HERO_AT=394.9,230.2,27,0`
  (1320, rise from the lava once their link is gone) — stand on one: it sinks slowly and tilts toward you, carrying
  you along its tilt; class sounds 0 (first touch), 1 (sinking past its line), 2 (back up). Step off: it rises back.
- **Umbris' floats:** `RC_LEVEL=7 RC_HERO_AT=239.6,425.4,37,1.57` (1069 ×5 in a row) — rock when told, drop into the
  water, settle bobbing; standing on one pushes it down, sounds on crossing the water.
- **Surface jump:** any water level (e.g. `RC_LEVEL=5`): dive, swim up and jump out (state 6): the underwater fog /
  tint switches off on the jump tick even if no water face is near the camera.

## 7. Results (the test expert fills this in)
