---
status: open
job: minigames
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [classes, world]
---

# Hoven's turret mini-game (Gemlik's unlock) with its carrier, drones, shells and shots; Kalebo III's race host

## 1. Summary

The story stopped at Hoven: the only unlock of Gemlik (planet 13) is the turret 1267's win. Ported the whole turret game
as per-class code on shared mechanisms (no ride in G-CLS-032 shares code with another; the shells and shots are the
shared pieces, ported once and matched by code identity on every level that has them):

| class (level, instances) | update (reference) | module | shared with |
|---|---|---|---|
| 1267 turret (12, 1) + HUD callback 0x304218 + red screen 0x304d98 | level12 0x303540 | `units/hoven_turret.rs` | — |
| 1274 carrier (12, 1; 17 parts: guns 1278 ×5, plates 1277, block 1280) | level12 0x3069d0 (+0x3054b0, 0x306350, 0x3064e8, 0x306828) | `units/hoven_carrier.rs` | the child records = level18's `triggers::{record,place}_children` |
| 326 gun drones (12, 8) and their shot 409 | level12 0x2e9f68 / 0x2ece00 | `units/hoven_drone.rs` | — |
| 1371 drone rider | level12 0x308918 (+ spawn 0x308670, knock-off 0x308708) | `units/drone_rider.rs` | level15 0x2ea5e0 |
| 458 turret shell (created by code) | level12 0x2ef648 (+ spawn 0x2ef2e8) | `units/turret_shell.rs` | level08 0x2e8210 (Batalia's turret 440, unported) |
| 184 gun shot (created by code) | level12 0x2d4378 (+ spawn 0x2d4150) | `units/gun_shot.rs` | level04 0x2af058 |
| 1455 Kalebo race host (16, 1) | level16 0x2e6808 | `units/kalebo_race.rs` | the talk / teleport code of Rilgar's 918 |

Plug-and-play: a level whose class table names one of these updates runs the port (`LevelPorts` code identity). A new
turret / gun consumer calls `turret_shell::spawn(w, owner, muzzle, vel, target)` or `gun_shot::spawn(w, owner, muzzle,
vel, life)`; a drone-like carrier calls `drone_rider::{spawn, knock_off}`.

## 2. Where it lives

Coverage tables (one row per call / branch) in the module docs of the seven modules above. Shared seams touched:
`moby_update/services.rs` (`HeroFields::{no_vel_clamp, death_z}`), `help.rs` (`Help::cam_pitch_word`), `cinematic.rs`
(`CinematicCall::FocusTicks`, `focus_ticks`), `creature/fx.rs` (`part27`), `tick.rs` (the death-height store),
`units/mod.rs` (rows, `frame_callback` dispatch). Docs: gaps.md G-CLS-032 / 033 / 034 / 035, G-REN-030;
progression.md `## story` rows for Hoven / Kalebo.

## 3. Behaviours to verify

Level words below are `Services::units` words (`hoven_turret::lw`, `hoven_carrier::lw`). dt = 1/60; `ticks(n)` = n (NTSC).

### B1. Turret init (0x303540 state 0)
- **Claim:** 0x161f00 = 0x161f04 = 0; mission (+0xb0) done → state 6 (rests, seq 0); else manipulator on joint list 0
  attached (pvar +0x00), state 3, +0x84 = 200 (s32), rot.y = 0, +0x70 (s16) = 30, +0x72 = 0, +0x78 = +0x7c = 0,
  +0x98 = −1, +0xa0 = z, rot.z = π.
- **Setup:** level 12 harness, the turret (instance 612, pos 326.25, 342.5, 60.99). **Trigger:** first update.
- **Edge:** mission byte done → state 6 and nothing else. **Method:** level-harness unit test.

### B2. Turret waiting (state 3)
- **Claim:** Ratchet ≥ 7 (xy) away and not standing on it (`Hero::ground_moby`): z rises by dt per tick up to +0xa0
  (then = +0xa0). Nearer (or standing on it): z += ((+0xa0 − 1.2) − z)·0.11 (bits 0x3de147ae); spin +0xa4 =
  0.9·add_rot(+0xa4, 0.01·add_rot(sub_rot(atan(Ratchet − pos), rot.z), π/2)); rot.z = add_rot(rot.z, +0xa4) (it turns its
  seat toward him). Mount when Ratchet's moby is < 1.3 (xy) away **and** his grounded tick count 0x13f650 > 15:
  `CameraScript(pos, rot, 1, 0, false)`; seq ≠ 5 → hard cut 5; `SetState(0x32, 1)`; the seat camera targets (B6);
  health +0x8c = 200.0; hit slot 0xff; 0x161f20 + 1; state 4.
- **Edge:** the sound `PlayClassSound(2)` never plays (+0x98 is −1); grounded count ≤ 15 → no mount.
- **Method:** unit test with a synthetic hero position / ground moby / grounded ticks; QA in game.

### B3. Turret ride (state 4): hero writes, HUD registration, hits
- **Claim:** every tick: `HeroFields::hero_hidden = 1`, `no_vel_clamp = 120`, `invulnerable = 120`; 0x161f00 = 1; game
  mode ≠ 2 and 0x1619a0 = 0 → `RegisterDrawCallback2(Callback::UnitFrame(hud row))`. The hit timer +0x90 (s32) out:
  a hit with flag 0x10000 → +0x90 = 4, health −= damage; hit slot 0xff either way.
- **Edge:** a hit while +0x90 runs is ignored (slot still dropped); the shells 458 (0x50001) and the drones' shots
  409 (0x10003) carry 0x10000; the gun shots 184 from the carrier's guns (class 1278, not 0xff / 0x4b1) carry 0x10001.
- **Method:** unit test (deliver a hit record, check health and +0x90).

### B4. Turret death and quit
- **Claim:** 0x161f24 = 0 only: 0x14095f & 1 → health > 100: 0x161f20 − 1; 0x14095f = 0; `HeroCall::Death`. Health ≤ 0
  → 0x161f04 = 1, +0xac = 120, state 7. State 7: `Callback::UnitQuads(tint row)` registered; +0xae out → an explosion
  at camera + polar(randf(0.7, 1.2), cam yaw + randf_sym(0, 30°), cam pitch + randf_sym(0, 30°)) with flash f =
  randf(0.5, 1.3), flash2 0.57f, flash_dist 10, scale 0.7, 7 streaks, 15 sparks, 25 puffs, class sound 3, no shake, no
  light; +0xae = rand_range(5, 13). +0xac out → `HeroCall::Death`. `tint()` = 0xaa red at alpha (80 − t)·80/80 while
  +0xac < 80, None otherwise.
- **Edge:** with 0x161f24 = 1 (the carrier's last stand) health can go ≤ 0 without dying; the rand draw order
  (randf, randf_sym ×2, randf, rand_range) matters for the stream.
- **Method:** unit test of the branches; `tint` pure function test.

### B5. Turret controls and fire (0x1619a0 = 0)
- **Claim:** stick (`Hero::stick`; y negated when `Help::cam_pitch_word` = 0): yaw speed +0x7c approaches
  −150°·dt·x at 240°·dt² while speeding up the same way, 360°·dt² otherwise; rot.z += it; pitch speed +0x78
  approaches 70°·dt·y (180° / 720°·dt²); pitch +0x74 += it, clamped [−0.7853982, 0.6108652]; the barrel manipulator's
  quaternion about axis 1 by +pitch (`set_axis(−pitch)`). Anim flags & 2 → blend seq 5 over 1. Fire on □ or ○ held
  (0xa0) with +0x70 = 0: blend seq 4 (barrel +0x80 = 1, then 0) / 3 (then 1); vel = polar(50·dt, rot.z + π/2, pitch);
  muzzle = joint point 2 / 1 + 3·vel; a shell; class sound 0; shakes Up 0.03 ×10, Forward 0.03 ×7; +0x70 = 20; five
  type-27 sparks (size 50000, colour 0x5f2f4f6f, life rand_range(5, 10)) and one flash (500000, 0x2f4f7f7f, life
  rand_range(4, 7), velocity 0.5·dt along the shot).
- **Edge:** held fire = one shot every 20 ticks alternating barrels; pitch clamp on both sides.
- **Method:** unit test with a scripted stick / buttons; QA in game (feel).

### B6. Seat camera and aim assist
- **Claim:** camera targets: position = rows·(0, 0.45, 3) + pos, Euler (0, −pitch, rot.z + π/2), every ride tick. Aim
  assist: the ten pvar targets +0x40..; a target's point (+1.5 z for classes 326 / 1278) within 7° yaw and 7° pitch of
  the camera (1278: 2° / 3°) → +0x88 + 2 up to 30; else `FastDecTimer(+0x88)`.
- **Method:** unit test (synthetic camera yaw / pitch and a target moby).

### B7. Turret HUD (0x304218 → `hoven_turret::hud_frame`, `DrawCallbacks::rings`)
- **Claim:** 4 backdrop quads (FX 0x2e, 0x60808080) centred (64, 356) ±55 px; 32 ring slices radius 40 (FX 6): colour
  tween((h − 100)/100, 0x6000ffff → 0x6000ff00) above 100 else tween(h/100, 0x600000ff → 0x6000ffff); the first
  trunc((1 − h/200)·32) slices alpha 0x20; two reticle squares of radius 40 about (256, 208), FX 0x11 turned by
  4·rot.z, FX 0x12 by 8·pitch, colour from +0x88 (green → yellow up to 15, yellow → red above).
- **Edge:** h = 200 → no dim slice; h = 0 → all 32 dim; +0x88 = 0 → pure green.
- **Method:** unit test of the primitive list; visual QA.

### B8. Turret win (carrier deleted)
- **Claim:** the carrier (+0x94) in state ≥ 0x80 and game mode ≠ 2: `SetMissionDone(+0xb0)`, movie 13, `UnlockPlanet(13)`,
  0x161f00 = 0, `HeroTeleport((328.4, 338.7, 60.98), (0, 0, −1.7), 0, reset cam)`, state 5; state 5 (mode ≠ 2) → scene 1,
  state 8; state 8 (mode ≠ 2) → save, state 6. In mode 2 instead: manipulator detached, seq 0, z = rest, rot.z = π.
- **Method:** unit test (delete the carrier moby, check the cinematic requests in order).

### B9. Shell 458 (turret_shell)
- **Claim:** spawn: ambient a0a0a0, distances 0xff, rot from vel, +0x1c = ±1 by tick parity, ten muzzle bursts (type 21
  size 20000 + type 02, def −1), life 120 on level 12 else trunc(scale(240)). Update: move; outside 10..500 on any axis
  → deleted; two trail blobs; spin 320°/s·sign; life out → deleted (no blast); `CollLine_Fix(old, new, 0, self, tmpl)`
  (flags 0x50001, damage 2, +0x20 = 1, +0x18 = the stale stack float bits) hit → position = hit point; all > 15 →
  `SpawnBeamExplosion(3, 3, 5, 3, 9, 1, min(24, x−2, y−2, z−2), …, 0 streaks, 10 sparks, 20 puffs, debris 1, shake)`
  (damage sphere 3 / 3); deleted.
- **Edge:** homing branch only for class 0x1b3 targets (none passed by 1267). **Method:** unit test; level 08 harness
  that 458's update resolves to this unit (code identity).

### B10. Carrier 1274
- **Claim:** init: records recorded (17), on path point 0 (+3 z when Ratchet below 52), guns' health 10, falling list
  cleared, health 100. State 1: hidden while the camera is in cuboid +0x624; height spring (3 / 10 / 19 above rest);
  bob 0.7 at 70°·dt, wobble 2° (35° / 30°·dt); → 2 on Ratchet in cuboid +0x5ec or the turret game on (0x1619b4 = 1).
  State 2: path flight (8·dt speed, slows from 11 points before the end), yaw spring; → 3 at speed 0. State 3: 0x161f34
  + 1, yaw to 3.0545. The engine loop sound (class sound 0, flags 4) in states 1..3, released otherwise.
- **Battle (`battle`):** per gun: pitch target to the camera, look-at manipulator, yaw sweep (aim within 5°, then ±32°
  sweeps); firing when the turret game is on or engaged (0x161f34 > 1200, within 80 / 4): voice timers, shots every
  7 + clamp(mounts, 0, 11) ticks at 80·dt toward the camera (z + randf(−1.5, 0.3) ± 0.3·clamp(mounts − 3, 0, 5)), nine
  type-04 puffs; hits (mask 0x50000) on the gun (1.0) or its part (0.2) with the turret game on: 20-tick cooldown,
  gun at 0 → carrier −20, class sound 1, the gun's part falls (gun 4: every 1277 plate too), 130 type-22 puffs, gun
  hidden and no collision; with the game off → 0x161f34 = 4000. Smoke while 0 < health < 10. Carrier health < 1 with
  the turret alive → 0x161f24 = 1, state 4.
- **Death (state 4):** camera targets cuboid +0x608 → blend to +0x60c (`turn::spring(1, 0.666·dt², 0.666·dt², 0.4·dt)`,
  Euler x stays cuboid A's: the game's slip), letterbox on, 0x1619a0 = 1; at 1: letterbox off, `CameraScript2(3)`; then
  every part deleted, 0x1619b4 = 0, scene 7, carrier deleted. First death tick: all parts + the carrier fall and are
  flung (gravity / spins by class), the fall below z 20 deletes a part.
- **Edge:** the falling carrier keeps its flown pose apart (0x161f40 / 0x161f60); glow pulse on every non-gun part.
- **Method:** level-12 harness test of the state sequence with synthetic hits; trace compare if a PCSX2 recording exists.

### B11. Gun shot 184
- **Claim:** spawn: scale ·0.5 (owner not class 0xff), two type-26 glows (100000 / 40000, colours 0x2f4f7f7f /
  0x4f6f7f7f, life ·5/4). Update: far limit 100 on level 12; whizz sound 0 once within 3 when receding; outside / x,y,z < 0
  → deleted; template flags 0x10001 dmg 1 (0x50001 dmg 3 for owner classes 0xff / 0x4b1), dir xy-normalised, z 1, w
  5627.98; timer out → quarter life without template, then deleted; a line hit (owner ignored) → five type-27 sparks
  off the reflected velocity (only before the timer phase), deleted.
- **Method:** unit test.

### B12. Drone 326 / shot 409 / rider 1371
- **Claim:** ordinary drone: wakes on cuboid +0x80, flies path +0xa0, then bursts of 4 shots (15·dt, 25 puffs each),
  shuttles along path +0xa4, hint 12002 (record 0x5d) while item 11 not owned; death: knocks the rider off, explosion
  (4 / 2 / 100000 / scale 3 / light 15 / 20 streaks / 3 sparks / 4 puffs / sound 2 / shake / debris 1), `SetDeathBits`
  (bolts), deleted. Turret mode (+0xc0 ≠ 0): starts after +0xc4 (240..400 + clamp(mounts − 1, 0, 10)·35 ticks), aims
  at the camera at 22·dt, bursts of 3 past 6 mounts, respawns (state 0) after death; deleted (rider knocked off) when
  0x1619a0 is set. Shell 458 hits do 10. Focus moby 0x16735c taken / dropped (unit word `hoven_drone::FOCUS`, plus
  `CinematicCall::FocusMoby` / `FocusTicks(0)`).
- 409: flies, two type-04 puffs a tick, line + 0.2 sphere test; a moby hit other than the owner gets damage 1 (3 in the
  turret, 2 past 15 mounts), flags 0x10003; state 2 explodes (small beam + shake 0.025 ×20 in the turret, else the death
  explosion 0.25 / 13).
- 1371: posed on the drone (mode |= 6) until knocked off: knock flight (gravity 40·dt², speed from the formula, angle
  yaw + π), spins 250°·dt, blows up on landing (2 / 1 / 9 / 1 / 15, 5 streaks, 2 sparks, 4 puffs, sound 0, shake,
  debris 1) or vanishes below z 5.
- **Method:** unit tests per branch; QA.

### B13. Race host 1455 (Kalebo III)
- **Claim:** talk block at +0x20 (radius 3.7); node 3 → teleport to (142, 90.35, 83) yaw 2.35, entry pose = (120.45,
  286.95, 119.5) yaw −3.14, state 3, race groups +0x60 on / +0xa0 off, death height 74 (`HeroFields::death_z` →
  `Game::death_z`), unit word 0x161c74 = 76, music (2, 6); Ratchet's group 0 → state 1, groups back, height (state 1:
  100, 115 off group 0xf), word 100, music (0, 8). +0xbc 1 / 2 / 3 as in the coverage table; node 4 played → item
  movie 0 → state 4 → Hologuise (item 31), save. Score word 0x13fbf8 > 4499 → skill point 0x13d422 (sound + banner).
  Hint 16003 (record 0x87) near her without the Hoverboard.
- **Edge:** the race itself is not ported (G-LVL-007): in game the race starts and ends at once (group 0).
- **Method:** unit tests; QA of the talk and teleport.

## 4. Shared code touched (regression risk)
- `HeroFields` gained `no_vel_clamp`, `death_z` (None by default: no change for other writers). `tick.rs` applies
  `death_z` after the moby loop — only 1455 writes it.
- `Help::cam_pitch_word` (copied in `sync_in`; default 1).
- `CinematicCall::FocusTicks` (new variant; only the drones push it).
- `creature::fx::part27` (new helper; existing callers unchanged).
- `units::frame_callback` now a match (veldin countdown unchanged).
- `DrawCallbacks::rings` now also receives the turret HUD's primitives (the Trespasser minigame extends the same list;
  no level has both).

## 5. Not ported (don't test as working)
- Full-screen red tint draw (G-REN-030; colour computed by `hoven_turret::tint`).
- 0x14095f vehicle quit and `TriggerIn::riding_class` engine wiring (G-CLS-034).
- The occlusion line's ignored moby 0x13e5bc (n/a: world-only occlusion in the port).
- Writes into moby[−1] by 0x306828 / 0x306350 for −1 records (skipped).
- 1109, 69 (+ the creature 388 it fights; planet 14 waits on both), 1242, 1379 (G-CLS-032); 336, 1269, 1345 (G-CLS-033).
- 1455's race (G-LVL-007), the sea 1111 reading 0x161c74, the landmark word 0x13dc1c (read as 0).
- Mobys are not re-created on the death reload (G-CLS-030): after a turret death the turret stays in state 7 → the
  game cannot be retried without reloading the level.

## 6. In-game QA spots (for the user)
- `RC_LEVEL=12 RC_HERO_AT=326.25,336,61.5,1.57 cargo dev` — walk onto the turret (it sinks 1.2 and turns its seat to
  you as you approach). After standing on it ¼ s: Ratchet vanishes, the camera snaps into the seat, the HUD shows the
  backdrop bottom-left with a full green health ring and a centre reticle. Left stick turns / pitches (pitch limits
  −45° / +35°), □ or ○ fire alternate barrels: muzzle sparks + flash, class sound, camera kick, smoke-trailed shells.
  The carrier (by 345, 389, 62) flies in on its path, its five guns sweep and fire glowing shots at you; the drones 326
  rise and fire bursts. Hits drop the ring (yellow under 100, red low). Shoot each gun (10 shells on the gun, or 50 on
  its plate): smoke when damaged, a big puff and the plate falls when it dies. Last gun: camera glides between two
  views (letterbox), the carrier breaks apart and falls, scene 7, then movie 13 and Gemlik unlocked, Ratchet set
  down by the turret, scene 1, save.
- Lose: let health reach 0 → red screen (not drawn yet: G-REN-030), explosions in front of the camera for 2 s, death.
- `RC_LEVEL=12 RC_HERO_AT=337,436,30,0 cargo dev` — an ordinary drone near Hoven's start area (wakes on its cuboid).
- `RC_LEVEL=16 RC_HERO_AT=118,287,120,3.14 RC_GIVE_ITEMS=29,30 cargo dev` — the race host: talk, node 3 teleports to the
  race start and music changes (then back at once: the race is G-LVL-007).

## 7. Results (the test expert fills this in)
