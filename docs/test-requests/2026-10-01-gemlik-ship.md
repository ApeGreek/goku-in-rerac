---
status: open
job: gemlik-ship
date: 2026-10-01
commit: 9257c3f
areas: [classes, world, hud, render]
---

# Gemlik's ship battle: the ship 69 and its HUD, the shots 1009 / 295, Qwark's ship 388 and its pieces

## 1. Summary

Planet 14 (Oltanis) was blocked: Gemlik's story director 1353 waits for Qwark's ship 388 to die, and the only way to
fight it is Gemlik's flown ship 69. Ported both, with everything they make, as per-class code on shared mechanisms;
the shared pieces (the shots, the vehicle record) are written once and bind by code identity on every level that has
them.

| class (level, instances) | update (reference) | module | shared with |
|---|---|---|---|
| 69 Gemlik's ship (13, 1) + HUD callback 0x2b97f8 | level13 0x2bb068 | `units/gemlik_ship.rs`, `units/gemlik_ship_hud.rs` | — |
| 1009 laser shot (created by code) | level13 0x302780 (+ spawn 0x3025f8, search 0x302420) | `units/ship_laser.rs` | levels 11, 17 |
| 295 homing missile (created by code) | level13 0x2e6c08 (+ spawn 0x2e6a58) | `units/ship_missile.rs` | level 17 |
| 388 Qwark's ship (13, 1) + beam draw 0x2ea8c8 | level13 0x2eb098 | `units/qwark_ship.rs` | — |
| 389..400 parts, 401 hit shield | level13 0x2ece00 (+ spawn 0x2ecd10) | `units/qwark_ship_parts.rs` | — |
| 352 shield, 82 missile, 83 mine | level13 0x2e84d8 / 0x2c1528 / 0x2c2048 | `units/qwark_ship_parts.rs` | — |

New shared systems:
- `rc-game/src/vehicle.rs`: the ridden-vehicle record 0x140940..0x14095f (`Services::vehicle`). The freeze menu's kind-1
  "Quit?" now sets its quit bit (`rc-engine` menu_render), the pause trigger reads `riding_class` from it, Hoven's
  turret reads the bit from it (G-CLS-034 closed).
- `Services::occlusion_fallback` (0x15f608 for a frame: 1 all visible, 2 the octant fallback) replaces the Visibomb's
  `all_visible_at`; `UpdateModeFreeze` now asks for 2 (was treated as 1).
- `Services::view_tan_x` (the view's half-angle tangent): the ship's speed FOV; the main camera projection follows it.
- `DrawCallbacks::{screen, screen_texts}` (`ScreenPrim` / `ScreenTex` / `ScreenText`): a frame callback's 2-D layer,
  drawn by `rc-engine` hud_render before the HUD; `ScreenTex::FxCut` = an FX texture with its CLUT cut (the gauge),
  decoded into the HUD atlas by `hud_render::apply_fx_cuts`.
- `story::{group_set, class_list_set, class_state_set}` (moved `group_set` from `kalebo_race`).
- `help::dialogue_stream` (the 50000 range: `global/qwark_boss_audio/{(id − 50000)·6 + lang}`), used by gameplay and the
  menus; `Help::continue_stream` for a class.
- `units::Globals::word_or` (a level word with a non-zero initial value).

## 2. Where it lives

Coverage tables (one row per call / branch) in the module docs above. Docs: gaps.md G-CLS-032 (69 / 388 struck),
G-CLS-034 (closed), G-LVL-009, G-HERO-027 (camera class 19's arming), G-REN-034 (new: the other view-tangent readers).

## 3. Behaviours to verify

dt = 1/60; `ticks(n)` = n (NTSC). Gemlik = level 13; the ship 69 sits at (482.3, 545.3, 317.0), its mount area is cuboid 8,
the director's ambush cuboid also sits there (`RC_HERO_AT=485,545.3,317.5` lands in it). The arrival scene runs
about 854 ticks first.

### B1. The mount
- **Claim:** standing in the mount area (or △ with the "Enter" prompt within 2 of the ship) the screen fades to black
  in 15 ticks (0.067 a tick), then Ratchet is hidden, the ship placed at cuboid +0x130 three units up, the chase
  camera on, the engine sound (class sound 0, flags 4) loops, the music track 2 (stinger 4) requested, the help box
  suspended; the record: health 255, missiles 10 of 20. The screen fades back in.
- **Edge:** the director's ambush (scene 1) and the area mount happen together on a first visit.
- **Method:** QA in game; unit test of `vehicle::Record::take`.

### B2. Flight
- **Claim:** the first 2 seconds the speed creeps up (+0.003 a tick), then cruises at 24·dt; ✕ held boosts to 30·dt
  (the boost sound plays while held); the stick turns (sprung, ±80° pitch), the ship rolls into turns; flying away
  from the level's centre (512, 512, 320) past 50 units (xy) or above / below it the heading is pulled back; the
  field of view widens with the boost (65° → 85°); the exhaust plumes from joints 1 and 2 grow with the boost.
- **Edge:** touching level geometry destroys the ship at once (big explosion, state 6 → the death sequence after 4 s
  unless a line plays); other mobys scrape it (−5 every 10 ticks).
- **Method:** QA in game (feel); unit test of `gemlik_ship::steer`.

### B3. Weapons
- **Claim:** □ / L1 fire lasers from alternating wing joints every 4 ticks (class sound 3), 400 u/s, 30-tick life; a
  laser homes gently (15° cone, 50°/s) on creatures within 60. ○ / R1 fire a missile every 30 ticks while missiles
  remain (class sound 2), from joints 7..10 in turn, at the HUD's lock (+0xec); it homes hard and blasts 2 / 10.
- **Method:** QA; unit tests of `ship_laser::search` (score, cones) and the missile's homing branch.

### B4. HUD
- **Claim:** missile pips top-left (two columns), the crosshair 23 units ahead, the lock marker (green, spinning, growing
  in over 75 ticks, then red / blinking), radar blips bottom-right relative to the camera yaw (fading at 38..46 px),
  red rings on Qwark / 1003 / 1284 when on screen, a green pulsing arrow toward them when off screen (beyond 45°),
  the low-health text (msg 0x5269) blinking every 90 ticks below a tenth, the gauge (FX 0x28 + set) with the part
  above the eased health black.
- **Edge:** the gauge fills up from empty at every mount (+0xf0 starts at 0).
- **Method:** visual QA against PCSX2; unit test of `gemlik_ship_hud::hud_frame`'s primitive list and +0xe8 count.

### B5. Qwark's ship
- **Claim:** hidden until Ratchet flies (state 1); then flies path 0 into the arena and fights in phases: 1 missile
  volleys, 2 missiles / mines, 3 + the tractor beam (grabs and holds the ship within 60..100), 4 the shield (352, slow
  to damage), 5–7 the rest; a part breaks off at each sixth of health lost; the boss meter (slot 1, 6 segments) tracks
  his health; taunts play from `qwark_boss_audio`. At 0 he spins down path 4, explodes, the ship lands (mission done,
  Ratchet at the exit cuboid, checkpoint), the director plays scenes 2 / 3 and movie 14 and unlocks planet 14.
- **Edge:** quitting (Start → Quit?) resets him (state 9 → 0) and puts Ratchet back; dying makes him gloat (state 6)
  and the level reloads; the longer since the first attempt (26 / 41 × 6 s of play time) the weaker he starts (120 / 102).
- **Method:** QA in game (the whole battle); unit tests of `qwark_ship::taunt` (line choice, bits) and the phase
  thresholds in `hits`.

### B6. Census binding
- **Claim:** `rc-trace class-census` shows 69, 295, 1009 (11, 13, 17), 388, 82, 83, 352, 389..401 as ported.
- **Method:** run the census.

## 4. Known not done (gaps)

- The blob shadow under the parked ship (G-REN-025), the crash fly-by camera (class 19, G-HERO-027), the shield's
  texture scroll (G-REN-018), the view tangent outside the main projection (G-REN-034), the HUD handle 0x14095c
  (never written on Gemlik).
