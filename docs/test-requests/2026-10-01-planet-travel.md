---
status: open
job: planet-travel
date: 2026-10-01
commit: 7bf9dc6
areas: [classes, world, cinematic, render, ui]
---

# Planet travel: the ship class, game mode 6, `DoSpaceTransition` and the engine's one runtime level change

## 1. Summary

- **The runtime level change** (`rc-engine` `level_switch.rs`): one path for every level change. The boot and every
  change load a level through `level_switch::load_bundle` (level data, the loader's moby spawn pass with the ship of
  0x13e056, the fog zones) and the level start's schedules `LevelStartup` / `LevelPostStartup` (the old `Startup` /
  `PostStartup` systems of the per-level renderers moved there); a change happens in an exclusive system at the end of
  a frame: `LevelUnload` (each module drops its per-level state: `level_switch::remove` / `reset`), the old level's
  meshes / cameras / UI roots despawned (not `KeepAcrossLevels`: the main camera, the movie UI), the new `Level`,
  `MobySpawn`, `FogState`, `GameFog`, `ClearColor`, the audio system's level data (`AudioOut::swap_level`), the current
  level index (`level_load::set_level_index`), the old level's cached lumps dropped (`rc_data::Lumps::forget_level`),
  `LevelGeneration` + 1, the level start's schedules again; the lazy `PreUpdate` set-ups (gameplay, menus, attachments,
  vendor, menu models) and the `Local` caches (world lights, after-images, the Swingshot rope, the sky's first clear)
  re-run when the generation changes. Per-level static caches (`gameplay::level_ports`, `camera_ports`,
  `level_reactions`, `thruster_render::tables`) are per level (`level_load::PerLevel`), `interact_render::class_blob`'s
  core is keyed by level.
- **The ship class** (`rc_game::travel::ship`, `ClassUpdate::Ship` = 0x2a1c40 for classes 531 / 532 / 533, code-identical
  on all 19 overlays): glow pulse, glass and shadow callbacks, hatch prompt, △ → take-off, the level-10 Clank boarding.
  The ship is now driven from the moby table (`Play::driven`): position, rows, anim (sequence 1 by hard cut), hide.
- **Mode 6** (`rc_game::travel::space::ShipMode`, driven by `rc-engine` `travel_render.rs`): take-off (sub 0), landing
  (sub 8), revisit landing, fly-away (sub 3), the flight (sub 4).
- **`DoSpaceTransition`** (`rc_game::travel::transition::plan`): every branch as steps; the story cards
  (`rc_game::travel::cards`, `space_plates`), the transition movies (`mpegs[40 + n]` through crate::movie_render), the
  saved-game writes (`GameState::apply_transition`), the background load, the flight, the level start's rules
  (`GameState::apply_level_start` with the destination's item tables) and the swap; the new level's `entry`
  (`ShipLandingStart`).
- **Requests in**: `rc_game::cinematic::EngineRequest::LeaveLevel { dest }` (`cinematic::leave_level`, the classes'
  `0x2a29a0`; `media_render::request_level_exit` for Quit Game / New Game / Load Game / the end page — the saves and
  media lanes' callers) → `travel_render::request_leave(dest)` (crate::scene_render's request pass); the planet page's
  post-action 2 → `travel_render::request_ship_travel(dest)` (crate::menu_render).
- Plug-and-play for a new consumer: a class that leaves the level calls `cinematic::leave_level(w, dest)`; engine code
  calls `travel_render::request_leave(dest)`; a new per-level resource registers `level_switch::remove::<T>` /
  `reset::<T>` on `LevelUnload` (or builds in `LevelStartup`); a lazy set-up adds `generation:
  Res<LevelGeneration>` and resets its guard on `generation.is_changed()`.

## 2. Where it lives

- `crates/rc-game/src/travel/mod.rs` — `ShipGlobals` (0x13e030..), `ship_for` (0x13e056 rule), `FlyAwaySetup` (level
  settings +0x3c..+0x44), the tables (hatch 0x1bdd00, riders 0x1be200 / 0x1be230, trail 0x1bdf30 / 0x1bdf40 / 0x1bdfb0
  / 0x160590, skip 0x1605d0, landing 0x160570).
- `travel/ship.rs` — `update` (0x2a1c40), `boarding` (state 0x2a), `register_shadow` / `shadow_quads` (0x2a2130),
  `flames_frame` / `flame_quads` (0x2a2ab8), `Trail` / `trail_quads` (0x2a2d28), `exhaust` (0x2a3eb8 =
  `fun_0022f5b0`), `set_hidden` (0x2a2450 / 0x2a2480).
- `travel/space.rs` — `ShipMode::{take_off (0x2a27c8), enter (0x2a24b8), travel_to (0x2a2848), landing_start
  (0x2a29c0), frame, scene_tick / actor_fx / scene_end (0x2a4080 subs 0 / 8), fly_away_tick / fly_away_setup /
  fly_away_body (sub 3), flight_start (0x2a5868), flight_tick (0x2a33b0)}`, `ship_fx`, `walk_back` (0x24a3d8),
  `lump_and_stream`.
- `travel/transition.rs` — `plan` (0x2a68f8). `travel/cards.rs` — `Plates::parse` (0x2a6180), `CardPlayer::frame`
  (0x2a64e0).
- `crates/rc-engine/src/level_switch.rs`, `travel_render.rs` (module docs: API and frames).
- Shared edits: `rc-game` `cinematic.rs` (`EngineRequest::LeaveLevel`, `leave_level`), `moby_update/services.rs`
  (`Services::travel`), `moby_update/classes/mod.rs` (`ClassUpdate::Ship`), `draw_callbacks.rs` (`ShipShadow`,
  `ShipFlames`, `ShipTrail`, `DrawCallbacks::ship`), `lib.rs`; `rc-engine` `main.rs`, `gameplay.rs`, `scene_render.rs`,
  `menu_render.rs`, `fx_draw.rs`, `moby_spawn.rs`, `level_load.rs`, `audio_out.rs`, `movie_render.rs` (movie UI kept),
  every per-level renderer's plugin (`LevelStartup` / `LevelUnload` registrations); `rc-data` (`forget_level`).

## 3. Behaviours to verify

### B1. Ship index for the destination (0x2a68f8)
- **Claim:** `ship_for(planet_unlocked, dest)`: 0; 1 if `planet_unlocked[8]` or dest > 7; 2 if `planet_unlocked[14]`
  or dest > 13. Set before the load (`level_load::set_ship`), read by `moby_spawn::ship_index` unless `RC_SHIP`.
- **Setup/Trigger:** unit test on `ship_for`; engine: a transition to 8 loads class 532, to 15 class 533.
- **Expect:** (dest 1, none) 0; (8, none) 1; (3, planet 8 unlocked) 1; (3, planet 14 unlocked) 2; (14, none) 2.
- **Method:** unit test (rc-game).

### B2. ShipUpdate glow (0x2a1c6c)
- **Claim:** only while +0x31 (drawn last frame) ≠ 0: +0xbc += 2 (u8 wrap); f = cos((+0xbc − 0x80)·0.024543693);
  v = trunc(f·50) + 0x96; +0x90 = `v | v<<8 | v<<16`, class 533: `v>>1 | v<<8 | v<<16`; `register2(ShipGlass)` with
  the joint-list-0 matrix in `DrawCallbacks::matrices`; xy distance (ship → camera 0x167240) < 32 → `ShipShadow`
  registered on list 1 with its quad.
- **Setup:** a table with a ship moby (class 531 / 533), visible = 1 / 0; camera at 10 / 40 units.
- **Expect:** cmd 0x7e → 0x80: v = 200 (f = 1); class 533 → R = 100. visible 0: no change, nothing registered.
  Distance 31.9 → shadow registered; 32 → not.
- **Method:** unit test on `ship::update` with a World.

### B3. Hatch flag and the take-off prompt (0x2a1d44..0x2a1ee0)
- **Claim:** pvar s16 +0xc: 0 → 1 when xy(ship, hero) < 6 and |hatch − hero| (3-D) < 4 (hatch = rows·(1,0,0) +
  pos); 1 → 0 when |hatch − hero| > 4.1 (so 4.0..4.1 keeps it); mode & 1 → 0. With the flag: `try_prompt(2, 0x53e4)`
  every tick ("△ Enter ship", HUD slot 12); △ (pressed & 0x10) with the lease: `Prompt::release(2, prompt_hud)`,
  `travel.take_off = true`, `autoskip = false`, `Handoff::ShipMenu` queued.
- **Edge cases:** hero 5.9 xy but 4.05 from the hatch: stays 0; after on, moving to 4.05: stays on; the ship hidden
  (mode & 1, the mission NPC's first-arrival hide): flag 0, no prompt; △ without the lease (another owner holds slot
  12): nothing; level 10 with `hero.mode == 1` (Clank): state 0x2a, mode |= 0x41, `StartScene(9)`, no take-off.
- **Method:** unit test (World, table, `Interact` prompt).

### B4. Level-10 boarding chain (state 0x2a)
- **Claim:** while game mode 2: nothing. Else, in order, one per update: flags[0x48] = 0 and item 28 owned → scene 4,
  flags[0x48] = 1; flags[0x49] = 0 and planet 11 unlocked → scene 5; flags[0x4a] = 0 and planet 11 → movie 0xb;
  flags[0x4b] = 0 and planet 11 → scene 6; else state 0, mode &= ~0x41, `autoskip = true`, `take_off = true`.
  Flag writes go through `GameWrite::Flag`.
- **Edge cases:** item 28 not owned but planet 11 unlocked → scene 5 first; all flags set → straight to the take-off.
- **Method:** unit test.

### B5. Take-off trigger (InLevelFrameUpdate 0x15f630)
- **Claim:** after a mode-0 tick that set `take_off`: if HP ≠ 0 → `ShipTakeOff` (HP < 1 → 1; music paused; stream
  stopped; help box killed; `EnterShipMode(0)`; 0x13e05c = 0); HP == 0 → flag dropped, nothing.
- **Expect (engine):** that frame renders normally; from the next frame mode 6 (`MenuMode` = Ship), 6 frames of
  `FadeToBlack(6)` black ramp (alphas 22, 43, 64, 86, 107, 128 accumulated), 1 black frame (the stream starts:
  `space_audio[ship + 6]`), then scene tick 1 with fade 0.875, … Hero and items hidden; HUD not drawn.
- **Method:** engine harness (frame-exact run) / QA.

### B6. EnterShipMode(sub) choices (0x2a24b8)
- **Claim:** `lump_and_stream`: take-off lump ship+1, stream ship+6; level 10 / ship 1 / no item 6 → (0, 0xb);
  level 14 / ship 2 → (8, 0xe). Landing lump ship+5, stream ship+3 (level start: ship); level 10 / ship 1 / no item 6 →
  (4, 10 / 9); level 14 / ship 2 → (9, 0xd / 0xc). Fade ticks(6) except the level-start landing (0). Writes: sub,
  tick −1, game mode 6, fade 1.0, white 0, `SetState(100, 2)` queued, ship mode |= 1, every live (state < 0x80) class
  74 / 203 moby mode |= 0x80. Actor 0 of class 533 → `SHIP_CLASSES[ship]` (only actor 0: `FUN_00259288`).
- **Method:** unit test on `lump_and_stream` (all branches) and `enter` (World).

### B7. Take-off / landing scene tick (0x2a4080 subs 0 / 8)
- **Claim:** per frame: sub tick +1; fade −0.125 (clamp 0); scene and chunk tick +1; chunk roll at 96; the camera
  record of the chunk tick made ship-local (eye = rows·e + pos; Z angle + ship yaw; tan from the record) through
  `scene_camera`; actor k: f = chunk tick >> 1, t = (tick & 1)·0.5, 1.0 on an odd tick of a cut record; position =
  lerp(track f, f + 1) ship-local; +0x48 = ship yaw; +0x71 = 0xff; `MobyBuildMatrix`.
- **Skip:** (pressed & 0x50) and tick > 30 and fade == 0: take-off with tick < 528 → tick = 558, chunk 5, chunk tick
  78, the stream stopped, `FadeToBlack(4)` (4 frames), fade 1.0; with `autoskip` no fade and autoskip cleared; landing
  with tick < end − 30 → tick = end (ends this frame). Autoskip also skips with no button.
- **Edge cases:** pressing ✕ at tick 30 (not > 30): no skip; fade > 0: no skip; take-off at tick 528: no skip.
- **Method:** unit test on `ShipMode::frame` with a loaded lump (`unknown_12e8/001.bin`, 598 ticks, cuts 103, 243,
  363, 503).

### B8. Per-class actor writes in the scenes
- **Claim:** class 0: +0x7f: take-off 0 (level 10 without item 6, or tick > 62) else 0x18; landing 0x18 past tick 360
  (level 10: only with item 6), else 0. Class 10: take-off 0x18 while tick ≤ 350; landing 0x18 past 524.
  Ship classes: glass (list 2) and shadow (list 1) every tick; take-off: tick < 360 pulse `cos(((t & 0x3f) − 0x20)·π/32)·80
  + 0x78`; 360..503: `min(0xff, (t − 360)·1.25)` (533 no red), past 420 the real ship (0x13e030) +0xbc = (t − 420)·1.2,
  +0xb2 = 0, flames registered on it; past 464 white += 0.025 (≤ 1); from 504: white −= 0.025 (≥ 0), glow 0xa0a0a0.
  Landing: t < 240: exhaust twice (z −4·dt, −3·dt) while t < 200, pulse, actor +0xbc = 0x32, +0xb2 = 10, flames on the
  actor; 240..360: the real ship's flames dying (+0xbc = (300 − t)·1.5 while t < 300), glow `(360 − t)·1.25`; past 360
  the pulse.
- **Method:** unit test (values per tick); visual QA (white flash at the end of the take-off, the flames under the
  ship on landing, dust puffs).

### B9. Scene end (0x2a4080)
- **Claim:** stream stopped; actors deleted (and the helmet); ship mode &= ~1 (shown); classes 74 / 203 mode &= ~0x80.
  Take-off → `SpaceEvent::PlanetPage`: the engine sets mode 0 and asks crate::menu_render for `EnterMenuMode` kind 0xe
  (the planet select page 0x1b6508) in the same frame. Landing → `Landed`: game mode 0, music unpaused; the level-start
  landing (0x13e05c) → `walk_back` (point = pos − 0.75·forward, `GroundHeight`; if |hero z − ground| ≤ 0.3: hero
  placed there + `HeroCall::WalkTo { point, yaw, release 1 }`), else `HeroTeleport(landing spot, rot, state 0,
  reset cam)` (the spot = the hero after `HeroInit`, `ShipGlobals::landing_spot`).
- **Method:** unit test; QA (the planet page opens right after the take-off; after a revisit landing Ratchet stands
  at his level spawn).

### B10. ShipTravelTo (0x2a2848)
- **Claim:** dest = current level → `EnterShipMode(8)` then tick = 240, chunk 2, chunk tick 48 (the landing joined in
  the middle), no save. Else `saves::memcard_save(play, gs, false, dest)` (memcard_Save(0, dest): the pretend save with
  level = dest), sub 3, sub tick −1, dest kept, fade 1.0, white 0, mode 6.
- **Method:** unit test on `travel_to`; check that the card file (crate::saves, `RC_SAVE_DIR`) got the global section
  with level = dest after a fly-away.

### B11. Fly-away first tick and setup (sub 3)
- **Claim:** sub tick 0: music paused, stream 40015 + ship pending; hold `ticks(12)` frames (black ramp); then the
  stream starts and the setup: ship seq 1 → 2 at t 0, +0x72 = 0xff, no collision; with a path (settings +0x3c ≥ 0):
  riders `CreateMoby(0)` and `CreateMoby(10)` (draw 0x1ff, +0x72 0xff, mode |= 6, the hero's light); `CameraScript(cam
  pos, cam euler, 1, 0, 0)`; cam ease 0 / 0; the path's points' w = turn rates (point i: wrapped yaw(i → i+1) −
  yaw(i−1 → i), the last: yaw(n−1 → 0) − yaw(n−2 → n−1), point 0 = 0) scaled so the sharpest (over 1..n−2) is 0.349 rad;
  segment = |p1 − p0|; t = speed = 0; trail emptied; from = ship pos; to = p0; pitch_to = atan(xy|p1 − p0|, p0.z −
  p1.z); yaw_to = atan(p1 − p0).
- **Method:** unit test on `ShipMode` with a synthetic path (3 / 4 points; check the w values and the scale).

### B12. Fly-away body (sub 3)
- **Claim:** camera: `spring(1, 0.666·dt², 0.666·dt², 0.5·dt)` on cam_t; target pos = lerp(cuboid a centre, cuboid b
  centre), Euler (0, lerp_rot(a.y, b.y), lerp_rot(a.z, b.z)) → `camera_targets`. Ship: tick < 150: f = (1 − cos(π·tick/120))/2
  (1 from 120), +0x54 = f, pos = lerp(from, to), rot (0, lerp_rot(pitch), lerp_rot(yaw)), fade −0.125, exhaust
  (−3.75·dt) while f < 1. From 150: speed += 0.8 (≤ 100); ✕/△ or t > n − 6 start the fade (0.0625); ships 0 / 1:
  +0xbc 0x32, +0xb2 10, flames; t += speed·dt / segment; t > n − 1 or fade ≥ 1 → `Leave { dest }`; else
  `path::pose(path, closed, t)` with rot.x = rot.w (the roll), `MobyBuildMatrix`, a trail sample (rows·trail A/B +
  pos), `ShipTrail` registered, fade += 0.0625 once started. Riders: ship rotation, pos = rows·seat + pos.
- **Edge cases:** no path (−1): no riders, the ship never moves (the leave never comes; game identical); no cuboids:
  the camera stays where the script put it.
- **Method:** unit test (positions over ticks on a synthetic path); QA (the ship lifts off its pad along the level's
  path, Ratchet and Clank in the cockpit, the camera glides between two points, the trail behind, fade to black).

### B13. Trail and flame quads
- **Claim:** `trail_quads`: per sample i < count − 1 (u = (head − i − 1) & 31) two rings × two quads (FX 0x13, FX 0 in
  the flight; additive); corner k: m = k >> 1, f = (i − m + 1)/32, colour `tween_color(f, head, tail)`, offset length
  (1 − f²)·0.8. `flame_quads`: 8 / 2 / 2 flames, intensity +0xbc (+ randi(+0xb2)), colour `intensity << 24 |
  0x2058b0` (533: 0x308000), corners rows·(c·intensity·size/40 + p) + bsphere/1024, FX 5 additive.
- **Method:** unit tests on the pure functions (count, colours at i = 0 and 31, sizes).

### B14. DoSpaceTransition plan (0x2a68f8)
- **Claim:** exact step lists (progression.md `## travel` table): dest < 0; new game (dest 0, visited[0] = 0); first
  Novalis (from 0, dest 1, visited[1] = 0); the generic branch with each of dest 4 / leaving 7 / dest 13 / leaving 14 /
  dest 16 conditions (cards 9+10 / 11 / 12+13 / 14 / 15+16, movies 6..10, fade 12 each), then SetLevel, Flight, Enter.
  `first_arrival` = dest 0 or (dest 1 and planet 3 locked). Fades are `ticks(n)`, card ticks 240 / 180.
- **Edge cases:** leaving 7 with visited[7] = 2 → no card; leaving 7 with planet 8 locked → no card and (B15) 7 stays
  visited 1; dest 4 already visited → no card; a load on level L (from = dest = L) → generic, no card unless the
  conditions hold.
- **Method:** unit test on `plan` with synthetic `GameState`s.

### B15. Saved-game writes of the transition
- **Claim:** `Step::SetLevel` = `GameState::apply_transition(dest)` (existing rules): first-Novalis branch visited[0] = 2
  (not visited[1]: the disassembly 0x2a6c88 loads the old level before the store), level = 1; generic visited[from] = 2
  except 7 with planet 8 locked / 14 with planet 15 locked; level = dest. `Step::Enter` = `apply_level_start(dest,
  items(dest))` (vendor, bomb glove, unlock dest, elapsed + 180, hero init, visited ≥ 1, record 3007) before the swap.
- **Method:** unit test of the sequence on a `GameState`; engine check after a fly-away (save state).

### B16. Story cards (0x2a64e0)
- **Claim:** `Plates::parse` of `space_plates/000.bin`: 17 cards 512×64 + the 64×64 band. `CardPlayer`: alpha 4·i for
  i < 32, 0x80, then (ticks − i)·8 in the last 16 frames; one line (a = b) at y 176; two lines at 162 and 208, the
  second from frame 0x41 with alpha (i − 0x40)·4 until 0x60; band t from (i % 600)/600 over 0.4; `load`: while not
  loaded ticks ≥ i + 20; then `FadeToBlack(2)` (2 frames). Language index = 0x15ed88 − 1 (≥ 0).
- **Method:** unit tests (alphas per frame, the load extension); visual QA of a new game (cards "Kyzil Plateau, Planet
  Veldin" / "(11:13am local time)").

### B17. Transition movies
- **Claim:** `Step::Movie(n)` plays `mpegs[40 + n]` through crate::movie_render (`MovieRequest::movie_b(n)` with exit
  `FrontEnd`: back to mode 6, no music restart); the next step waits until `MovieState::busy()` is false.
- **Method:** QA (new game: three movies; first Novalis: movies 43, 44, 45).

### B18. The flight (0x2a5868 / 0x2a33b0)
- **Claim:** start: sub 4, fade 1, variant `rand() >> 16 & 3` (first arrival: 4, count 2), hold ticks(12); per frame:
  draw lists cleared, fade −0.25, ticks +1, chunk roll at > 0x5f; at the end of a variant: actors deleted; not loaded
  → count 0; count > 1 → done; count 0 → v = (v + (rand() >> 16) % 3 + 1) & 3, else v = 4; count + 1. Camera
  `scene_camera`, tan ≥ 0x1be0c8[v] (0.3 / 0.63); v 4: planet offset += f·(0.3, 0.5); shell = dir[v]·(tick − 120)·20·f.
  Actor 0 class 533 → the ship's class; two half-ticks per frame (t + 0, + 0.25); v 4 last 56 ticks: scale ·(end −
  tick)/56, trail head alpha (end − tick) << 24, else 0x38.
- **Expect (engine):** after the load is done: one random variant, then the approach (148 ticks), then the swap.
- **Method:** unit test on `ShipMode` with the decompressed `transition.bin` variants (238 / 278 / 218 / 238 / 148
  ticks); QA (the ship flies across black space, then the new level).

### B19. The runtime level change (engine)
- **Claim:** after `Enter`: the old level's entities gone (count printed: "level switch: N entities of the old level
  despawned"), the new level drawn, Ratchet at the new level's spawn, the HUD (new level text), the menus (new level's
  page tree), the vendor, the help, the map, the water / sea / sky / particles of the new level; no panic; memory of the
  old level's lumps dropped; `level_load::level_index()` = dest; the per-level caches rebuilt (class ports, camera ports,
  reactions); the boot path unchanged (`RC_LEVEL`).
- **Edge cases:** two changes in a row (A → B → A); a change while a vendor / menu was opened before (closed by then);
  `RC_AUDIO=0`; `RC_PLAY=0` (no travel); `RC_HUD=0`.
- **Method:** engine smoke test (a scripted run: `RC_LEVEL=2 RC_UNLOCK_PLANETS=3` → ship → planet 3), plus an
  `all_levels`-style loop calling the switch for each level in-process (frame-exact) and checking the entity / resource
  counts against a fresh boot.

### B20. New level entry (`ShipLandingStart` 0x2a29c0)
- **Claim:** 0x13e05c = 1, 0x13e05a = 0; level 1 with planet 3 locked, level 0, level 14 with flag 0x13d3f0 (flags[0x68])
  = 0, or level > 0x13 → mode 0 at once; else `EnterShipMode(8)` without fade, stream `space_audio[ship]` (level 10 /
  ship 1 / no item 6: 9; level 14 / ship 2: 0xc), the landing; music paused at the entry unless (level 1, planet 3
  locked) or (level 0, flag 8).
- **Method:** unit test on `landing_start`; QA (arriving at Kerwan: the ship lands, Ratchet steps back 0.75 onto the pad).

### B21. Level exits routed to the one path
- **Claim:** `EngineRequest::LeaveLevel { dest }` (classes, `media_render::request_level_exit`: Quit Game −1, New Game
  0 with 0x13e05a = 1, Load Game = the restored level with 0x13e05a = 0, the end page's challenge restart 0) →
  `travel_render::request_leave(dest)`; the planet page post-action 2 → `request_ship_travel(dest)`. There is no other
  level-change code. Quit (−1): fade 6, level −1 in the save, the front end over the reloaded current level
  (`saves::set_front_end_active(true)`, crate::menu_render's set-up starts it).
- **Method:** QA with `RC_FRONTEND=1` (title → New Game: the opening cards and movies, then Veldin; Load Game: the
  saved level, its landing if any), Options → Quit Game (back to the title); code review: grep for other callers of
  `level_switch::LevelChange::swap`.

### B22. ShipGlobals in the gameplay set-up
- **Claim:** `Services::travel` = { moby: the loader's ship, ship: 0x13e056 of this load, setup: level settings
  +0x3c / +0x40 / +0x44 (e.g. Novalis path 75, cuboids 70 / 69), landing_spot: the hero after `HeroInit` }; the ship's
  table moby: draw 0xff, update distance 0x10, mode &= ~2, seq 1 frame 0, the hero's light words; its joint lists in
  `Services::joint_lists` (from the level core or the spaceships file).
- **Method:** engine test reading `Play::svc.travel` after the set-up for levels 1, 3, 13.

## 4. Shared code touched (regression risk)

- **Every per-level renderer's start** moved from `Startup` / `PostStartup` to `LevelStartup` / `LevelPostStartup`
  (run by `level_switch` in `PostStartup` at the boot): tfrags (main.rs `setup`; the camera split into `setup_camera`
  in `Startup`), mobys, ties, shrubs, billboards, sky (+ `main_camera_loads`), stars, water, particles, HUD, shadows,
  occlusion, `moby_spawn::init`. Risk: boot-order changes (the first frame's state). All frame-exact boots should match
  the previous captures exactly; please re-run the existing frame-exact screenshot / trace tests.
- `gameplay.rs` set-up: the ship joins `driven` (drawn from the table, its `SpawnHidden` removed, anim from the table),
  its table moby gets the loader's fields (draw 0xff, update 0x10, seq 1). Risk: the Novalis first-arrival hide (mode
  |= 3 by `ship_hidden_on_arrival`) must still hide it (`occl.drive` with mode & 0x81); `RC_SPAWN_RULES=0`.
- `ClassUpdate::Ship`: the ship now runs an update in the moby loop (and in the load pass: harmless, the hero far).
- Lazy set-ups' guards (`gameplay::setup`, `menu_render::setup`, `moby_attach::setup` / `rope_draw`,
  `menu_models::setup`, `vendor_render::setup`, `world_lights::update`, `afterimage_render::draw`,
  `sky_render::stop_clearing`): `generation.is_changed()` resets them. Risk: none at the boot (the generation's first
  run is "changed" and the guards start false).
- `menu_render` set-up: the front end's `RC_FRONTEND` / `setup_card` now only at generation 0.
- `scene_render`: `ActiveScene::white` (the white quad when no black one is drawn), `R::LeaveLevel`, `SceneFrameSet`,
  `level_reset`. `menu_render`: `MenuFrameSet`, the ShipTravel post-action call. `fx_draw`: three new arms.
  `movie_render`: the movie UI root has `KeepAcrossLevels`.
- `level_load::level_index` now reads the switch's index (the boot's `RC_LEVEL` until a change).
- `interact_render::class_blob`: the level core cached per level (was once per process).

## 5. Not ported (don't test as working)

- G-LVL-010: the mode-6 per-actor calls `FUN_0024a1d0`, `FUN_00228bc0`, `FUN_0026f0e0`, the helmet `CreateMoby(0x509)`
  (`FUN_0024a310`); the ship shadow's view-depth alpha (linear stand-in); the hero update running in mode 6's world
  frames; the fly-away camera targets one tick late; no actor shadows in the space scenes.
- G-LVL-011: the flight's scenery: the `transition` lump's sky, classes, planet picture, caption, sound bank and sound;
  the shell translation and the planet offset are computed but not drawn; the trail during the flight (drawn on the
  world layer the flight view hides); the actors' joints 1 / 2 for the trail (the ship's trail points stand in).
- G-LVL-012: no lump streaming after `entry`; the entry's music pause vs the new music's start [L]; PAL; the boot
  (`RC_LEVEL`) has no landing scene; Quit's front end reloads the current level as its backdrop.
- G-LVL-002: level 13's adopted ship (0x2a2360, the fixed landing spot).
- G-CLS-030: the death reload's moby re-creation (not a level change).
- The mirror cheat's left-row cross product in the mode-6 cameras (G-SAV-006 / the mirror pass of the media lane).

## 6. In-game QA spots (for the user)

Ships / hatches (hatch = where to stand; the prompt shows within 4 of it): Aridia `RC_LEVEL=2` (204.85, 154.84, 26.61);
Kerwan `RC_LEVEL=3` (249.23, 98.28, 55.50); Eudora `RC_LEVEL=4` (212.99, 169.11, 56.41); Rilgar 5 (357.10, 105.38, 62.12);
Batalia 8 (150.20, 188.42, 38.83, ship 1); Orxon 10 (231.30, 191.92, 49.75); Quartu 15 (316.59, 202.58, 31.94, ship 2).
Novalis 1 (167.23, 126.85, 61.15): hidden until the mission NPC's mission is done.

1. `RC_LEVEL=3 RC_HERO_AT=249.2,96.5,56.5 RC_UNLOCK_PLANETS=2,4`: "△ Enter ship" at the hatch; △: fade to black (6
   frames), the take-off scene (Ratchet and Clank board, canopy, engines; ~10 s; the engine glow pulsing, the flames
   igniting late, a white flash at the end); ✕ after 0.5 s skips (4-frame fade). Then the planet page: Kerwan,
   Aridia, Eudora listed. Pick Aridia: black, fade in, the ship rises with dust puffs, flies off along its path with
   the trail, Ratchet and Clank in the seats, the camera gliding; fade to black; then the flight (the ship over black
   space for ~10 s; no sky yet: G-LVL-011); then Aridia loads and the landing scene plays (flames, dust), Ratchet steps
   back onto the pad, mode 0, the HUD back. Listen: the take-off / fly-away / landing voice streams (space_audio).
2. Same, but pick Kerwan (the current planet) on the page: the landing scene from its middle (tick 240), Ratchet back at
   his spawn; no save.
3. Pick Eudora (4, first visit): cards "…Eudora…" and movie `mpegs[46]` before the flight.
4. `RC_FRONTEND=1` (saves lane): New Game → black, the Veldin opening (cards 0+1, movie 40, card 2, movie 41, cards 3+4
   while loading, movie 42), then Veldin in mode 0. Options → Quit Game → back to the title.
5. On Veldin (level 0) the end of the level (Clank's story trip, class 834: the story lane) → the first Novalis
   arrival: cards 5+6, movies 43, 44, card 7, movie 45, card 8 (loading), Novalis in mode 0, then scene 5.
6. `RC_TRAVEL_TRACE=1` prints each transition step and mode-6 event; `RC_SHIP=k` forces the ship class at the boot.

## 7. Results (the test expert fills this in)
