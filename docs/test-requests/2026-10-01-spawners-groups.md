---
status: open
job: spawners-groups
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [classes, world]
---

# Moby group services, spawner 815 re-creation, teleporter pads 1135 and Pokitaru's 318

## 1. Summary

- **Group services (G-CLS-023)**: one table of the engine's group functions in `scheduler.rs`: `group_count`
  (0x26e008), `group_cmd` (0x26e090), `group_state` (0x26e0e0), the walk `group_first` / `group_next` / `group_walk`
  (0x26e150 / 0x26e238) with its four filters (`GroupWalk`), and Blarg's park / unpark (level06 0x2f9948 / 0x2f99b0:
  `group_park` / `group_unpark`). A class that issues a group command calls these; nothing else to register.
  `group_count` moved here from `enemy_spawner` (help_blarg's import updated); the crates' private group walk now
  calls `group_walk(…, Alive)`.
- **Spawner 815 (level 1)**: the re-creation `0x302328` and the throw in `EnemySpawnerUpdate` 0x3021b8. No Novalis
  spawner has a group (all six have group −1), so on the disc it only does 0 → 1; the re-creation runs for a spawner
  given a group (synthetic test or modded level).
- **Teleporter pads 1135 (G-CLS-010)**: the whole update 0x308bd8 (cuboid activation, challenge gate, states 0–8,
  prompt, walk-to, sphere hit, arm springs, beam draw callback with the hero hide, HeroTeleport, arrival music, class
  sounds). 24 pads: levels 1 (2), 7 (2), 8 (2), 12 (2), 13 (2), 16 (2), 17 (10), 18 (2).
- **Pokitaru teleporter pads 318 (level 11, 5 pads, census U359)**: their own sibling code 0x2f2518 as a unit port.
- **G-CLS-018** (spawn conditions) was a stale premise: the loader's spawn test was already ported. Closed with the
  measurement; the death reload's re-creation is filed as G-CLS-030. Hoven's "parked helicopters" are class 336
  (unported path flyers), not a pool.
- **New seams**: `HeroCall::WalkTo` (0x249580), `HeroFields::hero_hidden` (0x1413f5 store, applied after the calls),
  `SoundSink::music_request` (MusicRequestTrack 0x27a248; implemented by `ClassSoundSink` → `Music::request`).

## 2. Where it lives

- `crates/rc-game/src/moby_update/scheduler.rs`: section "The moby groups" (table in the comment above `GroupWalk`).
- `crates/rc-game/src/moby_update/classes/enemy_spawner.rs`: `update`, `recreate` (coverage table in module doc).
- `crates/rc-game/src/moby_update/classes/teleporter.rs`: `update`, `teleport`, `activate`, `show`, `hide`, `prompt`,
  `place_arms`, `create_arms`, `spring_arm`, `beam_quads`, `beam_quads_at` (coverage table in module doc).
- `crates/rc-game/src/moby_update/classes/units/pokitaru_teleporter.rs`: `update`, `teleport`, `show`, `prompt`,
  `beam_quads` (coverage table in module doc). Rows "U359 318" and "U359 318 beam", and "U85 1135 beam" in
  `units/mod.rs` `PORTS`; `units::fx_quads` routes both beams.
- `crates/rc-game/src/moby_update/services.rs`: `HeroCall::WalkTo`, `HeroFields::hero_hidden`, `SoundSink::music_request`,
  `HeroFields::run_calls`.
- `crates/rc-game/src/audio/class_sounds.rs`: `ClassSoundSink::music_request`.

## 3. Behaviours to verify

### B1. MobyGroupCount 0x26e008
- **Claim:** `group_count(g, skip)` counts members with state < 0x80 and state ≠ skip (skip −1: every live member);
  g with no list or out of range → 0.
- **Setup:** synthetic `Groups` list [a, b, c, d] with states 0, 0xc, 0x80, 5.
- **Expect:** skip −1 → 3; skip 0xc → 2; skip 5 → 2; g = −1 / 200 → 0.
- **Suggested method:** unit test in scheduler.

### B2. Group state / command 0x26e0e0 / 0x26e090
- **Claim:** `group_state(g, s)` writes s only to members with state < 0x80; `group_cmd(g, c)` writes +0xbc on every
  member including dead ones.
- **Expect:** a dead member (0xfe) keeps its state but gets the command byte.
- **Suggested method:** unit test (existing users: water managers, timed switches, pressure pads, barricade: regression).

### B3. The group walk 0x26e150 / 0x26e238
- **Claim:** filters (a2, a3): (0,0) live only, (1,0) every member, (1,1) dead only, (0,1) nothing; `group_next(cur)`
  uses cur's own +0x21 (read unsigned), continues after cur's list position; cur not in its list or last → None.
- **Setup:** list [a(live), b(dead), c(live)].
- **Expect:** first Alive = a; next(a, Alive) = c; first Dead = b; first Any = a; next(c, Any) = None; Nothing → None;
  `group_walk(Alive)` = [a, c]; a moby with group −1 → next None.
- **Edge cases:** crate groups (crate_.rs `group_members`) must give the same lists as before (live members, list order).
- **Suggested method:** unit test; crates regression (breakables tests).

### B4. Blarg park / unpark (level06 0x2f9948 / 0x2f99b0)
- **Claim:** park: every member (no state test) → state 0x12, has_collision false, mode = (mode & ~0x1000) | 1.
  Unpark: members in 0x12 only → state 0, has_collision = class collision, mode = (mode & ~1) | 0x1000.
- **Edge cases:** a member not in 0x12 is left by unpark; a dead member is still parked (state set to 0x12).
- **Suggested method:** unit test (no ported caller yet: 1051 / 1108 are unported).

### B5. Spawner 815 gate and count (0x3021b8)
- **Claim:** state 0 → 1. State 1: nothing when group (pvar +0) < 0, or `group_count(g, 0xc)` ≠ 0, or spawns (pvar +8)
  ≥ 5; else spawns + 1 and the re-creation. The count rises even when the group has no live member.
- **Setup:** synthetic level-1 table: a spawner 815 with pvar +0 = g, +8 = 0, and a group g of amoeboids (class 572,
  pvars ≥ 0x2b0 bytes) in state 0xc.
- **Expect:** after 1 tick: spawns 1, the first live member re-created (B6); member state 9; the next ticks do nothing
  while that member is alive and not 0xc; after 5 spawns nothing more.
- **Edge cases:** all members dead → count still rises (to 5), nothing written; one member alive in a state ≠ 0xc →
  no spawn.
- **Suggested method:** synthetic unit test (no disc spawner has a group).

### B6. The re-creation 0x302328 and the throw
- **Claim:** member (first live of the group, list order): update distance 0x80, draw distance 0x80, light word and
  ambient = the spawner's, +0x31 visible = 1, collision = class's, mode |= 0x1020, scale = class scale alone, position and
  rotation = the spawner's; pvar +0x250 = 1.0, +0x20 = 3.0, +0x170 = 819 (i32), +0x224 = 30.0; matrix rebuilt. Then K =
  pvar +0x60: +0x84 = 8, +0x80 = (int)(1.0·0.8·1024) = 819, +0xb0 = 7.0, +0x88 = 0.8, +0xb4 = 13.0, +0x78 = +0x7c = 0;
  `knock::start(angle = add_rot(atan2(hero.y − sp.y, hero.x − sp.x), π), seq 5, ticks 8, frame 0)`; state 9.
- **Expect:** the member flies away from Ratchet (yaw target = angle + π, i.e. facing Ratchet), knock timer 300-tick
  rule of `knock::start`; amoeboid state 9 handles the flight.
- **Suggested method:** synthetic unit test.

### B7. Teleporter activation cuboid (0x308b28 / 0x309838)
- **Claim:** state ≠ 0, P[1] ≠ −1, not done (no collected flag, no persistent death bit) and Ratchet in cuboid P[1]:
  death bits (persistent (level, id) and this visit's id) set; group −1 → own state 1, else every live member of its
  group → state 1; the pad shown (+0x31 = 1, mode &= ~1, class collision) and its 3 arms shown; state 5 → 1.
- **Setup:** level 7 (pads 601 / 602 instances, group 66, cuboid 14, P[14] = 0 so both start hidden in state 5), level
  8 pad 818 (cuboid 20, mission 2), level 12 (cuboid 40, group 11), 13 (cuboid 6, group 49), 18 (cuboid 58, group 25).
- **Trigger:** put Ratchet inside the cuboid.
- **Expect:** both pads of the group go to state 1; the one whose own cuboid test passed is shown that tick, the other
  shows when its own update sees Ratchet in the cuboid (same cuboid); death bits recorded → on a reload the pads start
  in state 6 (done).
- **Edge cases:** state 0 never tests the cuboid; done pads skip it.
- **Suggested method:** level harness on level 7 / 12 with Ratchet teleported into the cuboid.

### B8. Challenge gate (P[15])
- **Claim:** P[15] ≠ 0 and times-completed 0x15ee20 = 0 → collision off, mode |= 0x41, return before the state switch
  (stays in state 0, no arms). With the count set: class collision, mode &= ~0x41, normal run.
- **Setup:** level 1 pad instance 961 (P[15] = 1) on a new game: hidden, no arms. `counters.times_completed = 1`: arms
  appear.
- **Suggested method:** level harness level 1.

### B9. State 0 (arms) and state 6
- **Claim:** update distance 0xff; three class-315 arms: draw distance 0x40, visible 1, light/ambient = Ratchet's moby's,
  mode = pad's, position/rotation = pad's, yaw + k·2.0943952 (wrapped); P[14] ≠ 0 or done → 6, else 5 and hidden (pad +
  arms, pad collision off). Then the arm placement (B13). State 6: group ≠ −1 → group state 1; state 1.
- **Setup:** level 17 (10 pads, P[14] = 1, groups 2 and 54): all 10 reach state 1 after the load pass + 1 tick.
- **Suggested method:** level harness level 17 / 1.

### B10. On-pad prompt and △ (state 1, 0x309430)
- **Claim:** mission P[2] ≥ 0 not done → nothing. Partner ≠ −1, body 0x1413f4 ≠ 2, ground moby = pad, air ticks 0 →
  prompt owner 4 with the level's message: L1/7/13 table [13002, 13003, 1036, 1035, 7001, 7002][P[4]], L17
  [17006, 17007, 17008, 17005, 17003][P[4]], L8 [8015, 8013], L12 [12010, 12011], L16 [21487, 21488], L18
  [18004, 18003]; other levels none. △ (pressed bit 0x10, the lease NOT tested) → class sound 0 on the pad, state 2,
  P[12] = −1, P[10] = P[11] = 0, `HeroCall::WalkTo { point: pad pos, yaw: pad yaw, release: 0 }`, sphere hit (r 1.5,
  damage 20, push 1, flags 0x10000, type 0, subtype 1) at the pad.
- **Expect (per level):** Novalis pad 962 (P[4] = 2) shows message 1036; 961 (P[4] = 3) shows 1035; level 13 pads show
  13003 / 13002; level 7 7001 / 7002; level 8: 8013 (P[4] 1), 8015 (P[4] 0); level 8 pad 818 only after mission 2.
- **Edge cases:** jumping on the pad (air ticks ≠ 0) → no prompt; Ratchet as giant Clank (body 2) → no prompt; a
  creature standing on the pad takes 20 damage on △.
- **Suggested method:** level harness (ground moby forced) + HUD prompt check.

### B11. HeroCall::WalkTo (0x249580)
- **Claim:** sets the walk-to point (x, y, z; w kept), yaw, release; SetState(0x65, play) unless the state is already
  0x65..0x67.
- **Expect:** Ratchet walks to the pad centre and turns to the pad's yaw (0x65 → 0x67).
- **Suggested method:** unit test on `run_calls`; QA.

### B12. State 2 (arms out, wait for Ratchet)
- **Claim:** walk = 1 in 0x65 / 0x66, 2 in 0x67, else 0. Spread P[8] < 1 → spring to 1 (velocity P[10]); else tilt
  P[9] < 1 → spring to 1 (P[11]); else, if the prompt owner ≠ 4 or spread ≤ 0.25: walk 2 → beam P[13] = 1.95, state 3;
  walk 0 → state 4 (abort); walk 1 → wait. Spring = `turn::spring(target, 12·dt², 12·dt², 32·dt)`. Arms placed every
  tick.
- **Edge cases:** the prompt lease still held by owner 4 (2-tick expiry) holds the beam back one or two ticks; walking
  away (state leaves 0x65..0x67) aborts to state 4.
- **Suggested method:** unit test with a scripted hero state.

### B13. Arm placement 0x3092d0
- **Claim:** per arm: position = pad + (0, 0, −0.4); rot.y = −20°; position += row 0 of (rot) normalised × (spread·2.4 −
  1.2); rot.y = tilt·(−π/3) − 20°; matrix rebuilt.
- **Expect:** spread 0 → arms 1.2 inward along their row 0; spread 1 → 1.2 outward; tilt 1 → arms tipped −80°.
- **Suggested method:** unit test (pure function of P[8], P[9]).

### B14. State 3 (departure beam, teleport)
- **Claim:** beam P[13] ≠ 0: register `Callback::UnitQuads(row "U85 1135 beam")` on list 1, `Approach(0, 4·dt)`,
  return (no arm placement); after the approach, r < 1.15 → hero_hidden = (r < 0.65). P[13] = 0: `hero_teleport(partner
  pos + 0.2 z, partner Euler, state 0, reset camera)`, hero_hidden = 1 (applied after the SetState that clears it),
  music request (P[16], P[17]) when both ≠ −1, partner state 4 with its P[13] = P[10] = P[11] = 0 and P[8] = P[9] =
  1.0, class sound 1 on the partner, own state 1, P[8..11] = 0, P[13] = 1.95, arm placement.
- **Expect:** ~29 ticks of shrinking beam (1.95 / (4/60)); Ratchet hidden from r < 0.65 (~10 ticks before the
  teleport); camera reset behind him at the partner. Level 7 pads request music (0, 5) and (2, 4).
- **Suggested method:** level harness level 7 / 1 driving the hero; check `HeroFields`, `cinematic` calls, sound log.

### B15. States 4 / 8 (arrival beam, arms back)
- **Claim:** P[13] < 1.95: beam callback, `Approach(1.95, 4·dt)`, hero_hidden = (r < 0.65) while r < 1.15 (state 4
  only, not 8); else tilt P[9] > 0 → spring to 0 with velocity **P[10]**; else spread P[8] ≤ 0 → state 1; else spring
  spread to 0 with **P[11]** (the velocity words swapped relative to state 2, as in the code). Arms placed.
- **Expect:** Ratchet reappears when the arrival beam passes 0.65 (~10 ticks), the beam grows to 1.95 (~29 ticks), the
  arms tilt back then close, state 1.
- **Suggested method:** level harness.

### B16. State 7 (unused on the disc)
- **Claim:** spread < 1 → spring (P[10]) to 1; tilt < 1 → spring (P[11]) to 1; else class sound 1, state 8, P[13] = 0.
- **Suggested method:** synthetic unit test (no disc class stores 7).

### B17. The beam draw (0x3094f0) → fx_draw
- **Claim:** 32 quads, corner u: angle (i + u/2)·6.28318/32 − 3.14159, (cos·r, sin·r, 1.0 ∓ h) in the pad's frame
  (rows + position), h = 0.1 + clamp(2(1.15 − r), 0, 1) when r < 1.15 else 0.1; FX 0x13, additive, RGBA 0x60408080, ST
  (0.5, 0)/(0.5, 1).
- **Suggested method:** unit test on `beam_quads`; visual check in game.

### B18. Pokitaru 318 (level 11, U359)
- **Claim:** see `pokitaru_teleporter.rs` table: state 0 → 1 at once with arms; mission P[2] not done → pad and arms
  hidden with collision off (arms too); state 1: hidden and P[2] done → shown (arms with their class collision); prompt
  owner 0xb: P[4] 0 → 0x2b0d, 1/2/4 → 0x2b0c, 3 → 0x2b0b; △ only when the lease is held (owner 0xb); partner P[1] when
  set and mission P[3] done; music (P[12], P[13]); arrival state 4 sets global flag 0x55 (0x13d3dd) when Ratchet z <
  138; beam radius at +0x3c, the hero hide in every state.
- **Setup:** level 11 pads (instances 47–51): 48 needs mission 4, 50 / 51 mission 3; 47 has the alternate partner 48
  gated on mission 4 (P[3] = 4).
- **Suggested method:** level harness level 11; QA.

### B19. Sound sink music request
- **Claim:** `ClassSoundSink::music_request(t, s)` calls `Music::request(t, s)`; other sinks default to nothing.
- **Suggested method:** unit test on the audio system.

## 4. Shared code touched (regression risk)

- `scheduler.rs`: `group_count` added (moved from `enemy_spawner`), walk / park functions added; `group_ids`,
  `group_state`, `group_cmd` unchanged. help_blarg (U232) now imports `scheduler::group_count` (same code).
- `classes/crate_.rs` `group_members` → `group_walk(…, Alive)`: same members and order (crates' group breaks: test the
  crate-group tests).
- `services.rs`: `HeroCall::WalkTo` (new variant; existing match sites in rc-game only `run_calls`), `HeroFields::hero_hidden`
  (None by default: no change for other classes), `SoundSink::music_request` (default no-op).
- `audio/class_sounds.rs`: one new trait method impl.
- `units/mod.rs`: three new PORTS rows (two draw-only), `fx_quads` arms, one module.
- `teleporter.rs`: rewritten states; the 1135 pads now teleport (previously idle). Levels 1, 7, 8, 12, 13, 16, 17, 18
  smoke: the pads' arms, prompt, the 20-damage sphere hit on △.

## 5. Not ported (don't test as working)

- G-CLS-030: the death reload's re-creation of the level's mobys (LoadLevelCoreData(0, 1)); `spawn_save` still leaves
  this visit's bits empty.
- Class 336 (Hoven helicopters, U392), 1108 / 1051 (Blarg, the park / unpark callers; U230 / U221): unported classes.
- The water raise / lower senders (the Hydrodisplacer's command bits; G-CLS-003 / G-REN-008).
- A teleporter prompt index past the tables (the game reads the next words): no pad has one [L].

## 6. In-game QA spots (for the user)

- Novalis pad 962: `RC_LEVEL=1 RC_HERO_AT=253.86,128.13,55.5,1.84`. Stand on the pad: prompt (message 1036), △:
  Ratchet walks to the centre, the three arms swing out and tip, sound 0 at △; the beam (a translucent ring/cylinder)
  shrinks, Ratchet vanishes, appears at pad 961 (158.97, 139.31, 60.0) with sound 1, the beam grows and the arms fold.
  Note pad 961 itself is challenge-mode only (hidden, no collision on a first playthrough), as the game's data says.
- Umbris (7): `RC_LEVEL=7 RC_HERO_AT=278.15,407.79,74.9` — pads start hidden until Ratchet enters cuboid 14; arrival
  music request (track 0 / stinger 5 or 2 / 4).
- Gemlik (13): `RC_LEVEL=13 RC_HERO_AT=458.69,532.49,317,0.52` (cuboid 6).
- Fleet (17): `RC_LEVEL=17 RC_HERO_AT=478.82,616.07,155.1` — 10 pads, all active; pad 486 / 487 need mission 3.
- Pokitaru (11): `RC_LEVEL=11 RC_HERO_AT=530.91,412.89,231,3.14` (pad 47); prompt texts 0x2b0b..0x2b0d; △ only works
  while the prompt shows.
- Listen for: sound 0 on △, sound 1 at the arrival pad; watch Ratchet hidden during the narrow beam; the camera snaps
  behind him at the arrival.

## 7. Results (the test expert fills this in)
