---
status: open
job: space-visuals
date: 2026-10-01
commit: 7bf9dc6
areas: [world, cinematic, render, ui, classes]
---

# Space visuals: the flight's scenery, the title world, the mode-6 actor extras, the level-change leftovers and the weapon demo playback

## 1. Summary

Batch 6, lane `space` (G-LVL-011, G-SAV-012, G-LVL-010, G-LVL-012; plus G-UI-006's playback at the coordinator's request).

* **The flight between planets' scenery** (game mode 6 sub 4, `DrawWorldPaused` 0x2a3b90): the `transition` lump parser
  (`rc_formats::transition`), the per-tick scenery values (`rc_game::travel::space::FlightDraw`), the engine draws
  (`rc-engine` `flight_render`): the lump's 6-shell sky turned / scaled / translated per shell and drawn with tan ≥ 0.63,
  the destination's planet picture (variant 4), the area caption (variant 4 past tick 60), the trail from the actor's
  engine joints (joint lists 1 / 2) with the lump's FX 0, the canopy glass with FX 1, the memory-card icon while the card
  works, the flight's own sound bank and its sound v at tick 1. Fixed on the way: the planet offset started at
  (0, 0, 280) instead of 0x1605a0 = (−370, −280, 280).
* **The title world** behind the front end (`RC_FRONTEND=1`): the title lump read through the level parsers
  (`rc_formats::frontend::TitleWorld` rebuilds a level core index over its tables), drawn by the level renderers
  (tfrags, ties, shrubs, billboards, sky shells, the sky's stars) on render layers 32 / 33 while the title runs, its
  5 scene actors, its looping camera (tan 0.63), its scene sounds (the boot's bank and defs), its fog and clear colour
  (`rc-engine` `title_world`, `rc_game::travel::title`, `rc_game::sky_stars::Variant::Title`).
* **Mode-6 actor extras** (G-LVL-010): the actors' shadows, the streamed slot and bounding sphere, Ratchet's head
  manipulator, the helmet 0x509 on Orxon (10, with the O2 mask) and level 13, Clank's glow pulse, the mirror cheat on
  the scene camera, the ship shadow's exact view-depth fade, sub 3's camera update after its body.
* **Level-change leftovers** (G-LVL-012): `entry`'s music handling (no forced resume on the no-landing levels),
  `RC_LANDING=1` (the boot's level enters with its landing scene).
* **The weapon demo playback** (G-UI-006): a weapon bought at the vendor with a demo scene plays `unknown_1530[scene]`
  in the vendor's frame (the same space-scene player, `SUB_DEMO`), then `VendorExit(1)`.

Instance counts: the title world has 156 tfrags, 296 ties, 569 shrubs, 6 moby classes (0, 530, 1138, 1564, 1635,
1905), a 4-shell sky, a 15-chunk scene (1398 ticks, actors [0, 530, 1635, 1905, 1564]); the flight lump has 5
variants (238 / 278 / 218 / 238 / 148 ticks, one actor of class 533 each), 133 pictures, 4 FX textures, a 5-sound bank;
the weapon demos are `unknown_1530[0..13]` (actors [Ratchet 0, the weapon's gadget class]).

**Plug and play.** The space-scene player (`ShipMode`) now takes any frame (`DemoFrame::of_moby(w, moby, offset,
yaw_add)` + `demo_start`): the item offer's gold-weapon demo (`FUN_002aea70`, offset (0, 6, 0), yaw −π/2) only needs its
engine entry and `FUN_002aecf0` exit. A second world on its own layers: build a `LoadedLevel` and call
`tfrag_render::spawn_tfrags_on`, `tie_render::spawn_ties_layered`, `shrub_render::spawn_shrubs_layered`,
`shrub_billboard::spawn_billboards`, `sky_render::spawn_shells`, `sky_stars::spawn_stars` with a `RenderLayers`.

## 2. Where it lives

| file | what |
|---|---|
| `crates/rc-formats/src/transition.rs` (new) | `TransitionLump` (module doc: the header map) |
| `crates/rc-formats/src/frontend.rs` | `TitleWorld::parse` (module doc "The title world as level data") |
| `crates/rc-game/src/travel/space.rs` | coverage table rows `EnterSpaceLoadingLoop`, `SpaceLoadingLoop`, `DrawWorldPaused`, subs 0 / 8 actor rows, `VendorStartWeaponDemo`, `VendorModeUpdate` substate 3; `FlightDraw`, `flight_shell`, `FLIGHT_*`, `PLANET_*`, `CAPTION_*`, `CARD_*`, `DemoFrame`, `SUB_DEMO`, `ShipMode::{flight_start, flight_tick, demo_start, demo_tick, ratchet_actor, helmet, actor_anim_for, actor_joint}`, `clank_actor`, `build_with` |
| `crates/rc-game/src/travel/ship.rs` | `depth_alpha`, `shadow_quads` (the `fun_001fa820` row) |
| `crates/rc-game/src/travel/title.rs` (new) | `TitleScene` (`fun_001eb0a8`, `transition_update_movie_camera`, the scene sounds 0x1862b0) |
| `crates/rc-game/src/sky_stars.rs` | `Variant::Title`, `TITLE_LEVEL`, `gen_title_twinkle` (boot `update_sky_effects` 0x22ae70) |
| `crates/rc-engine/src/flight_render.rs` (new) | the flight's sky, planet, trail, glass, caption, card icon, audio |
| `crates/rc-engine/src/title_world.rs` (new) | `load`, `title_audio`, `spawn`, `tick`, `cameras`, `upload` |
| `crates/rc-engine/src/travel_render.rs` | the flight's hooks, `request_weapon_demo` / `take_weapon_demo_done`, `fly_away_camera`, `RC_LANDING`, `upload_gfx` / `spawn_gfx` (shared actor drawing), `with_world_view` |
| `crates/rc-engine/src/interact_render.rs` | the vendor's demo request and `VendorExit(1)` |
| `crates/rc-engine/src/shadow_render.rs`, `scene_render.rs` | `ActiveScene::space_actors` / `space_hero_hidden` |

## 3. Behaviours to verify

### B1. The transition lump
- **Claim:** `TransitionLump::parse(global/transition.bin)`: base = hdr[1] = 0x9800; `variant(v)` = data `hdr[0x14 + v]`
  (lump offsets 0x1cb400, 0x1d4c00, 0x1de400, 0x1e7c00, 0x1f1400) to the next variant (variant 4 to the sound bank);
  `sky_block()` = data 0x41000..0x6f540 (6 shells); `picture(k)`: table at data 0x6f540, count 133, PIFs; `planet_picture
  (d)` = picture d (128×128 PSMT8); `caption(lang, d)` = picture 19 + 19·max(lang − 1, 0) + d (256×32); `fx_textures()` = 4
  entries (32×32, 64×64, 64×64, 64×64; FX 2 and 3 share the palette at bank 0x1c00); `sound_bank()` = data 0x1ee400 to the
  end (989snd type 3, "SBlk", 5 sounds); `classes()` = [(531, 0x22ba80), (532, 0x25de80), (533, 0x288b80)]; `gs_image()`
  = lump 0x800..0x9800.
- **Setup:** the extracted `global/transition.bin`.
- **Expect:** the values above; `planet_picture(19)` and `caption(1, 19)` are errors; `caption(0, d) == caption(1, d)`.
- **Method:** unit test (rc-formats, extracted data).

### B2. The flight's start
- **Claim:** `flight_start(w, variants, first_arrival, dest, rigs, out)`: variant v = `rand() >> 16 & 3` (4 for the
  first-arrival flights, count 2), `planet` = (−370, −280, 280, 0), `turn` = `FLIGHT_SKY_TURN[v]`, the trail reset,
  `out.audio` contains `SpaceAudio::FlightBank`, the hold `ticks(12)` before the first frame.
- **Setup:** a `World` with a level class table; any rigs map.
- **Expect:** as above; after the hold the first `FlightDraw` has `variant == v`.
- **Edge cases:** first-arrival (dest 0, or dest 1 with planet 3 locked): v 4, no `FlightSound`, `trail == false`,
  `glass == None`, the actor hidden in variant 4.
- **Method:** unit test on `ShipMode`.

### B3. The flight's sound
- **Claim:** `SpaceLoadingLoop` tick 1 of every variant (not the first-arrival flights): `SpaceAudio::FlightSound(v)`
  with v the current variant (`snd_play_sound_vol_pan_pmpb(bank, v, 0x400, 0, 0, 0)`); the engine plays bank sound v of
  the transition bank after `FlightAudio::Bank` swapped the audio system's data to it (master volumes from
  `master_volumes(sfx, music, false, false)`).
- **Setup:** a flight with the load done after the first variant.
- **Expect:** one `FlightSound` per variant, at its tick 1; the variant numbers match `Flight::variant`; none on a first
  arrival.
- **Method:** unit test (the audio list per frame); QA (a whoosh at each variant start).

### B4. The flight's scenery per tick
- **Claim:** each flight frame's `FlightDraw`: `sky_turn = [0, 0, 0, 0, 3.0][v]`; `shell = FLIGHT_SHELL[v]·(tick −
  ticks(120))·20·f` (f = (end − tick)/end in v 4, else 1); `sky_tan = max(camera tan, 0.63)` (the camera's own tan is
  ≥ 0.3 in v 0..3, ≥ 0.63 in v 4); v 4: `planet` corners = `PLANET_CORNERS·PLANET_SCALE[dest] + planet`, `planet +=
  f·(0.3, 0.5, 0, 0)` every tick before; `caption_alpha = min((tick − 60)·2, 128)` for v 4 and tick > 60, else 0;
  `dest` = the destination; `trail`/`glass` only off the first-arrival flights; `glass` = (actor 0's class, its joint
  list 0 matrix).
- **Setup:** synthetic scenes (1 actor) for v 0 and v 4, dest 3 (scale 1.4).
- **Expect:** values per tick as above; at the variant change the planet resets to (−370, −280, 280); the frame whose
  count reaches 2 gives `FlightDone` and no `FlightDraw`.
- **Method:** unit test.

### B5. The flight sky's shells
- **Claim:** `flight_shell(k, turn)`: k 0, 1 → ((0, 0, turn), 1.0); k 2 → ((0, −0.075, turn − 0.15), 1.25); k 3 →
  ((0, 0.05, turn + 0.125), 1.5); k 4 → ((0, 0.1, turn − 0.05), 1.75); k 5 → ((0, −0.15, turn + 0.1), 2.0) (z wrapped to
  ±π by `fast_add_rotations`); k ≥ 6 → None. The engine's `sky_render::shell_transform(euler, s, t)`: rotation
  `Rx·Ry·Rz` (Z applied first), uniform scale s, translation `game_to_bevy(t)`; `sky.wgsl` applies the whole model
  matrix to (x, y, z, 1) (it used w = 0).
- **Setup:** the table; a level sky (no translation, no scale).
- **Expect:** values; a level sky renders unchanged (its entities have only a rotation).
- **Method:** unit test; regression screenshot of a level sky (e.g. Novalis) before / after.

### B6. The trail from the engine joints
- **Claim:** each flight half-tick pushes a trail sample from `MobyAttachToJoint(actor, 1)` and `(actor, 2)` (the joint
  lists of the ship class from the `spaceships` file, evaluated with the streamed sequence of the chunk at the actor's
  frames / t, its rows, position and scale); without a skeleton for the class the old stand-in (the ship's trail points
  `TRAIL_A/B` in the actor's frame).
- **Setup:** `flight_render::rigs()` (classes 531..533).
- **Expect:** the sample points lie on the ship's engine joints (compare with the actor's evaluated joint matrices); two
  samples per tick; count ≤ 32.
- **Method:** unit test with a rig; QA (the trail leaves the engines in every variant).

### B7. The flight's draws in the engine
- **Claim:** while the flight runs (`FlightRender::active`), the flight sky's shells are spawned on layer 31 and the sky
  camera shows that layer (clear black), the main camera stops clearing; the shells' transforms follow each frame's
  `FlightDraw`; the sky camera's tan is raised to `sky_tan`; the planet quad (ST (0,0)(1,0)(0,1)(1,1), RGBA 0x80808080,
  ALPHA 0x44, the planet picture as FX index 4), the trail (FX 0 of the lump), the glass (FX 1 of the lump) are effect
  draws on layer 26; the caption (256×32 at (0x20, 416 − 0x58)) and, while the card is busy (state > 2 or a request
  pending), the card icon (FX 2, 64×64 at (44, 320)) and the spinner (FX 3, 17×17 centred at (76, 352), angle −2π·(vsync
  % 55)/55) are HUD prims. At the flight's end the shells are despawned, the sky camera's layers emptied, the main camera
  clears black again.
- **Setup:** a level change by ship (QA spot 1).
- **Expect:** the starry sky drifting / turning; in the approach (last variant) the destination planet's picture growing
  toward the camera and the area caption fading in; the trail behind the ship; the canopy glass.
- **Method:** QA (visual); engine smoke run of a flight (no panic, entities on layers 26 / 31).

### B8. The title world's data
- **Claim:** `TitleWorld::parse(global/unknown_14e8.bin)`: the core index = a rebuilt `LevelCoreHeader` (gs_ram (512,
  0x1fe0 + 0x100), tfrags 0, sky 0x64640, moby / tie / shrub classes (6 / 48 / 33), texture tables, part / FX tables,
  textures base 0xb4c40, part bank 0x32f240, FX bank 0x330240, part defs 0x3fe0 + 0x100, chrome map 0x400 / 0) then
  the lump's header region at 0x100; data = lump[0x5c000..]; gs = lump[0x4800..0x5c000]; gameplay = data[0x637c00..];
  scene = data[0x392b40..0x3ce340]. `title_world::load`: 156 tfrags (every strip list valid), 296 tie / 569 shrub
  instances, sky 4 shells, scene 15 chunks / 1398 ticks / 5 actors, occlusion all `ALWAYS`, the fog and background of
  its level settings.
- **Setup:** extracted data.
- **Expect:** the counts; `parse_textures` and `tfrag_lod::load` succeed on the gs image; no panic.
- **Method:** unit test (rc-formats) + engine-level load test (`title_world::load`).

### B9. The title scene loop
- **Claim:** `TitleScene::tick`: chunk tick and tick + 1; tick < 1398: chunk + 1 when the chunk tick passes 0x5f (chunk
  tick 0); tick ≥ 1398: tick 0, chunk 0, chunk tick 0. The camera = the record at the chunk tick with tan 0.63 (not the
  record's); actors: frames chunk tick >> 1 and + 1, t = (chunk tick & 1)·0.5 (no cut rule), position = the lerp
  (absolute: no ship frame). `restart()` = tick 0, chunk 0, chunk tick 0 (after an attract movie). `actors_drawn(mode)`
  is false in boot mode 3 (the main menu).
- **Setup:** the title scene.
- **Expect:** tick 1398 wraps to 0; tick 96 is chunk 1 record 0; the camera tan is always 0.63.
- **Method:** unit test.

### B10. The title's scene sounds
- **Claim:** `TitleScene::sounds` (before each tick, with the tick it left): entries (0, 99999999, def 2), (0, …,
  def 3), (0, …, def 0), (0x2e4, 0x4d8, def 4), (0x114, 0x12c, def 1): inside the window a def is started when its slot
  does not play it (`class_index == def` and state 1 / 2), else it keeps playing; outside the window a slot that plays it
  is released and the slot cleared. The plays are `SoundSlots::play(def, 0, None, None, None, 0x400)` on the title audio
  (`title_audio`: the global `sound_bank.bin`, level defs = the 7 boot defs at 0x186100 with bank ids 0, 0, 8, 9, 2, 3,
  4; class 0x472's 5 menu defs at 0x1861e0 with bank ids 0, 1, 5, 6, 7).
- **Setup:** an `AudioSystem` built from `title_audio`.
- **Expect:** three looping ambiences from tick 0, a sound at ticks 276..300 and repeated plays between 740 and 1240,
  released outside; the boot defs' ids as listed (read from the boot ELF).
- **Edge cases:** the attract movie's `movie_stop` rebuilds the audio system: the stale slots re-trigger.
- **Method:** unit test (slot states per tick); QA (listen on the title).

### B11. The title's stars
- **Claim:** `SkyStars::new(TITLE_LEVEL, …)`: the first frame `srand(12345)` then 256 records: 246 twinklers (kind 1,
  texture 1, ALPHA 0x48, +0xc = `randi(256)` then the base colour, rotation `rand_angle`, size (`randi(24)` + 32)/256,
  position (cos a·sin b, sin a·sin b, cos b)·50 with a = `add_rot(−3, rand_angle·0.2)`, b = `rand_angle·0.09 + 1.2`, g
  = `randi(24)`, A = `randi(32)`, tint by `r16 & 1`), 10 moving (kind 0, two r16 counters, texture 1, size 0.16); every
  frame the generic update (moving: the counters, the blink 0x702020f0 / 0x202020f0; any other kind: the twinkle
  jitter). Drawn by `sky_stars::spawn_stars` on layer 33 between shells 1 and 2.
- **Setup:** `rc_game::sky_stars`, a seeded `Rng`.
- **Expect:** a fixed record set (golden hash) independent of the incoming rng state; all twinklers' azimuth within
  −3 ± 0.63 rad and polar 0.92..1.48 rad.
- **Method:** unit test.

### B12. The title world in the engine
- **Claim:** with `RC_FRONTEND=1` the title lump loads on a thread when the front end starts; in the title phase the
  world is spawned on layers 32 (tfrags, ties, shrubs, billboards, actors) and 33 (sky, stars), the main camera shows
  layer 32 without clearing, the sky camera shows layer 33 cleared to the title's background colour, the menu layer is
  clear (`saves::title_world_shown`), `ActiveScene::camera` = the title camera each vsync, the audio system's data =
  the title audio, the fog state and `ClearColor` = the title's. The actors (no hero: light word 0, ambient 0x38) are
  hidden while the main menu (boot mode 3) is open. `sound_update` + one audio frame per vsync in boot modes 0 and 4 (mode
  3: the page menu's). The cameras stay on the title until the transition's black screen (then `view_layers`), and go
  back to the level view at a level change.
- **Setup:** `RC_FRONTEND=1 cargo dev`.
- **Expect:** see QA spot 4; no host-level geometry visible behind the title; New Game: the title stays the last image
  under the fade, then the cards.
- **Edge cases:** the attract movie (after 25 s idle): the scene restarts at tick 0 when it ends; Quit Game from a
  level: the title world again; a load failure prints "title world: not loaded" and leaves the front end over black.
- **Method:** QA; engine smoke run `RC_FRONTEND=1` (no panic, title entities spawned).

### B13. The logo and PRESS START placement (coordinator's question)
- **Claim:** unchanged and faithful: `draw_textured_quad(x, y, w, h, u, v, tw, th)` draws from (x, y) with the origin at
  the frame's left / top edge (`InitViewContext`: 0x13e510 = (0x800 − 256)·16); the logo (0xec, 0x10, 0x100, 0x80) covers
  x 236..492, y 16..144 of the 512×416 frame (its texture is opaque across the whole 256×128); PRESS START (0xa0, H − 0x50,
  0xc0, 0x60) covers x 160..352 from y 336, its text in rows 60..70 of the 128-row texture (screen y ≈ 381..388, centred
  at x ≈ 260).
- **Method:** unit test on `FrontEnd::draw` positions; QA with the title world behind (the logo top right over the space
  scene).

### B14. The ship shadow's depth fade
- **Claim:** `ship::depth_alpha(view, 32, sphere)` (`fun_001fa820` 0x2222e8): None when `BSphereView::culled(32,
  sphere)`; else clamp(trunc(32·1024 − (c.z − r)·1024) >> 7, 0, 0x80) with c.z the view depth of the sphere's centre and r
  its radius (×1024 units as the VU0 code): 0x80 while the near side is within 16 units, 0 at 32. `shadow_quads` uses it
  with the moby loop's `w.view` (and mode 6's: the last drawn frame's view from `travel_render`); without a view the
  linear fallback. The fly-away fade (−4 per tick past `ticks(150)`) and RGBA `(a >> 1) << 24 | 0x808080` unchanged.
- **Setup:** a `BSphereView::from_camera` and spheres at depths 5, 16, 24, 31, 33 and behind the camera.
- **Expect:** 0x80, 0x80, 0x40, 8, None, None (±1).
- **Method:** unit test.

### B15. The mode-6 actors' streamed slot and shadows
- **Claim:** subs 0 / 8 (and the demo): each actor's table moby gets seq A / B = its class's streamed slot (the class's
  sequence count, the chunk's sequence put there), `MobyBuildMatrix` with that class (the streamed sequence's bounding
  sphere); `SpaceFrame::scene_actors` lists (moby, class with the slot) and the engine publishes them as
  `ActiveScene::space_actors`: `shadow_tick` probes those with +0x7f ≠ 0 (`ShadowProbeAlongDir`, direction 0) and
  `collect` casts their shadows with the streamed pose. `space_hero_hidden` hides Ratchet's own shadow while mode 6
  hides him; both reset on the landing's end and at a level change.
- **Setup:** QA spot 1 (take-off on Kerwan); a level-harness run of a take-off.
- **Expect:** Ratchet's actor casts a shadow until `ticks(62)` of the take-off (not on Orxon without the O2 mask), Clank's
  until `ticks(350)`; on the landing Ratchet's from `ticks(360)`, Clank's from `ticks(524)`; no shadow of the hidden hero.
- **Method:** level-harness test (shadow casters per tick) + QA.

### B16. Ratchet's actor (`FUN_0024a1d0`)
- **Claim:** once per scene (`head_attached`, cleared by `enter` / `demo_start`) a joint modifier on the actor's joint list
  0x18 with scale = Ratchet's head scale 0x15ee14 (`hero.idle.rec17_scale`, ≈ 0.92; capped at 1.33 in game mode 6), key
  `ACTOR_HEAD_KEY`; on levels 2, 9, 11, 13 every actor of class 0x1b1 / 0x50a / 0x509 gets scale = class scale × that
  scale. The engine evaluates the actor with its joint modifiers.
- **Setup:** a scene with a class-0 actor; the big-head cheat (slot 1) on and off.
- **Expect:** the node once; scale 0.92 normally, 1.33 with the cheat in mode 6 (1.57 in the vendor demo, game mode 5).
- **Method:** unit test.

### B17. The helmet 0x509
- **Claim:** on Orxon (level 10) with the O2 mask (item 6) and on level 13, the Ratchet actor's update creates the
  helmet once per scene (`CreateMoby(0x509)`: +0x32 = 0x40, mode |= 0x806, the actor's light words, +0x73 = 0x18 when the
  class's +6 byte is set), places it each tick on the actor's joint list 4 (point and rows of `MobyAttachToJoint`), its
  pose = `hero::worn::pose_from_host` of the actor's streamed pose on joints [8, 10] (key A the snapshot, t 0), its
  bounding sphere from sequence 0; deleted at the scene's end.
- **Setup:** `RC_LEVEL=10 RC_GIVE_ITEMS=6` take-off; level 13 take-off.
- **Expect:** the helmet on Ratchet's head through the scene, gone afterwards; no helmet on other levels or on Orxon
  without the mask.
- **Method:** unit test + QA.

### B18. Clank's actor (`FUN_00228bc0`)
- **Claim:** every tick: mode |= 0x10; glow = s = sin(2π·(counter % ticks(100))/ticks(100) − π), `(0x4c + trunc(24s))
  << 16 | 0x80000000 | (0xa2 + trunc(52s)) << 8 | (0x44 + trunc(24s))`; with the cheat "Clank has a large noggin"
  (slot 3) a modifier on joint list 2 scaled by 0x15ee18 once. The engine writes the glow word into the actor's slot
  (`ExtraMobys::set_glow`) when it changes.
- **Setup:** a scene with a class-10 actor, counters 0, 25, 50.
- **Expect:** the glow words of the formula; Clank's eye glow pulsing every 100 ticks in the take-off.
- **Method:** unit test + QA.

### B19. The mirror cheat on the scene camera
- **Claim:** with cheat slot 4 the take-off / landing camera's left row = up × forward (`FastVecCross(0x167460, 0x167470,
  0x167450)`).
- **Method:** unit test.

### B20. The fly-away camera in the same tick
- **Claim:** mode-6 world frames run the gameplay tick in its scene form (no `CameraUpdate`) for subs 0 / 8 / 3; in sub 3
  `travel_render::fly_away_camera` runs after the body: the queued camera calls (`CameraScript`, the targets) applied,
  then `Camera::update` with the moby scene, the camera moby deleted if one was live. The camera therefore reaches each
  tick's target in that tick (it lagged one tick).
- **Setup:** a fly-away (QA spot 1, pick another planet).
- **Expect:** the camera position after tick n equals the script camera's response to tick n's target (compare with a
  one-tick-delayed run); no double camera update per tick.
- **Method:** engine harness (camera per tick) + QA (the glide between the two cuboids is smooth).

### B21. The hero update in mode 6
- **Claim:** no change: the game runs the hero update 0x228870 in subs 0 / 8 / 3 (`GameStateUpdate`), Ratchet in state 100.
- **Method:** none (documentation correction).

### B22. `entry`'s music
- **Claim:** after a level change the level's music starts (first EE frame of the new audio system) and is then paused
  (scene inbox) unless the first Novalis arrival or Veldin with global flag 8; the landing's end unpauses; a level without
  landing (level 0, Novalis' first arrival, level 14 without 0x13d3f0) no longer posts a resume: the music stays as
  `entry` left it until the following story scene's end resumes it.
- **Setup:** a level change to Kerwan (landing) and to Veldin with a new game.
- **Expect:** Kerwan: music silent during the landing, back after; new game on Veldin: paused until the opening scene ends.
- **Edge cases:** Novalis first arrival: music plays at once.
- **Method:** engine harness (audio scene state per frame) + QA.

### B23. `RC_LANDING=1`
- **Claim:** the boot's level runs `level_entry` once (not in the front end): the music rule above and `ShipLandingStart`
  (mode 6 sub 8 at the level-start landing, the walk-back), or mode 0 for the levels without one.
- **Setup:** `RC_LEVEL=3 RC_LANDING=1`.
- **Expect:** the Kerwan landing scene at the boot, Ratchet steps back onto the pad.
- **Method:** QA / smoke run.

### B24. The weapon demo
- **Claim:** substate 2's end with a demo (`VendorOut::weapon_demo`): the engine pushes `CameraRelease{kind 2}`, sets the
  hand request (session `temp_hand`) to the item (0 for the Drone Device 0x18), and starts `unknown_1530[scene]` with
  `DemoFrame::of_moby(vendor, (4, 0, 0), π)` (origin = vendor rows·(4, 0, 0) + position, z + 2, then
  `GroundHeight(0.5)`; yaw = vendor yaw + π; rows from (0, 0, yaw)): fade 0, `FadeToBlack(4)` hold, the stream
  `vendor_audio[36]`, then per frame (the world ticking in scene form): fade + 0.1 (≤ 1) past end − 12 else − 0.125
  (≥ 0); the camera and the actors in the demo's frame (record tan); Ratchet's head manipulator; at the end the actors
  deleted, fade 0, `take_weapon_demo_done()` true once, and the vendor's `VendorExit(1)` (Ratchet 3.5 in front of the
  vendor, state 0, `CameraResetBehindHero`, music unpaused). The weapon actor's class comes from the level's gadget table.
- **Setup:** a vendor with bolts; item → scene: 9→7, 10→0, 11→10, 13→13, 14→9, 15→8, 16→5, 17→1, 18→11, 19→12,
  20→2, 21→6, 24→3, 25→4 (table 0x1ca4a0).
- **Expect:** after the purchase and the vendor's fold: a short scene of Ratchet using the weapon in front of the vendor
  (the lump's end tick), its sound, a fade out, then Ratchet in front of the vendor with the camera behind him and the
  new weapon in hand.
- **Edge cases:** a weapon without a demo exits as before (`VendorExit(0)`); the PDA's remote vendor never demos; no vendor
  moby: exit at once.
- **Method:** unit test on `demo_tick` (fade per tick, DemoDone) + QA.

## 4. Shared code touched (regression risk)

* `sky.wgsl`: the model matrix applied to (x, y, z, 1) (was w = 0). Level skies have only a rotation: identical.
* `sky_render`: the shell spawning factored into `spawn_shells` (`spawn_sky` calls it: same entities, order, star
  slot); `follow_main_camera` pub(crate); `shell_transform` new.
* `tfrag_render::spawn_tfrags_on`, `tie_render::spawn_ties_layered`, `shrub_render::spawn_shrubs_layered`,
  `shrub_billboard::spawn_billboards`: the old entry points are wrappers with no layer (unchanged behaviour).
* `sky_stars` (engine): `spawn_stars` factored out of `setup` (same draws); `build_meshes` skips draws past the current
  simulation's groups (was an index). (rc-game) `Variant::Title`, `level_stars(TITLE_LEVEL)`, `SkyStars::frame` reseeds for
  the title only; the other variants unchanged.
* `fx_draw::FxSlots::layer` (None by default).
* `moby_render::load_mobys_with_gs` (`load_mobys` wraps it).
* `shadow_render`: casters / probes include `ActiveScene::space_actors`; Ratchet's shadow hidden with `space_hero_hidden`.
* `scene_render::ActiveScene`: two new fields (reset with the resource).
* `travel_render`: mode 6 runs the tick without its camera update (sub 3 too); `upload_gfx` evaluates with joint
  modifiers and writes glow words; `spawn_gfx` falls back to gadget classes; `level_entry` no longer resumes music on the
  no-landing path; the vendor demo.
* `interact_render` (vendor): the demo request and `VendorExit(1)`; `menu_render`: the front end's layer clear colour
  when the title world is shown; `menus/vendor.rs`: docs only.
* `rc_game::travel::space`: `ActorPose::glow`, `SpaceAudio::{FlightBank, FlightSound, VendorStream}`,
  `SpaceEvent::DemoDone`, `flight_start`'s new parameters; the take-off / landing actors' matrices now use the streamed
  sequence's bounding sphere (the flames' and shadow's sphere moves with the scene pose).

Tests that may change: any golden of the ship shadow alpha (now the depth fade), the flight's planet offset, sky renders
of levels with translations (none), the mode-6 actor table mobys' seq bytes.

## 5. Not ported (don't test as working)

* G-CUT-009: `FUN_0024a0e8` (the hero's worn items on the scene's Ratchet actor); also the mode-2 cutscenes' use of
  `FUN_0024a1d0` / `FUN_00228bc0`.
* G-REN-005: Clank actor's antenna glow sprite (`FUN_00229738` → the hero glow list).
* G-UI-006: the item offer's demo (`FUN_002aea70` / `FUN_002aecf0`); the instant hand clear `FUN_002305e8`.
* [L] the planet quad depth-tested behind the actors (the game draws it first without Z); the flight's particle textures
  (no particle pass); the per-actor 0x20-byte frame header copy (the engine evaluates the streamed frames directly).
* G-LVL-012: PAL lumps / movies.

## 6. In-game QA spots (for the user)

Ships / hatches as in `2026-10-01-planet-travel.md` §6 (Kerwan `RC_LEVEL=3 RC_HERO_AT=249.2,96.5,56.5`).

1. **The flight:** `RC_LEVEL=3 RC_HERO_AT=249.2,96.5,56.5 RC_UNLOCK_PLANETS=2,4` → △ at the hatch → the planet page →
   Aridia. After the fly-away: the flight over a starry sky that drifts and turns (no longer black), the trail from the
   ship's engines, the canopy glass; a whoosh at the start of each variant; in the last variant (the approach) Aridia's
   picture grows toward the camera and "Aridia"'s caption fades in at the bottom left; the ship shrinks and the trail
   fades in the last second. If the card was still saving: the card icon with its spinner at the bottom left.
2. **The take-off / landing extras:** same spot: Ratchet's and Clank's actors cast shadows (Ratchet's until about 1 s,
   Clank's until about 6 s of the take-off); Clank's eye glow pulses; the ship's shadow fades with distance from the
   camera (full within 16 units). `RC_LEVEL=10 RC_GIVE_ITEMS=6` at Orxon's hatch (231.30, 191.92, 49.75): Ratchet wears
   the helmet in the take-off (and the landing on return).
3. **The fly-away camera:** the camera glides between the two cuboids without a one-frame stutter at the start.
4. **The title:** `RC_FRONTEND=1 cargo dev`: after the still, the title shows the space scene (planet geometry, stars,
   the ship flying past with Ratchet) behind the logo (top right) and PRESS START; listen for the space ambience and the
   whooshes (ticks 276–300, 740–1240 of the 23-s loop); the loop restarts after the attract movie; in the main menu the
   world stays but its actors hide; New Game: the title fades to black, then the cards.
5. **Quit Game:** in a level, Options → Quit Game → the title world again behind the front end.
6. **`RC_LANDING=1`:** `RC_LEVEL=3 RC_LANDING=1`: the Kerwan landing scene at the boot.
7. **The weapon demo:** `RC_LEVEL=1 RC_GIVE_BOLTS=30000`, buy a weapon at the Gadgetron vendor (e.g. the Pyrocitor, item 16 →
   scene 5): after the vendor folds, a short demo scene of Ratchet with the weapon in front of the vendor, then Ratchet
   standing 3.5 in front of the vendor with the camera behind him.
8. `RC_TRAVEL_TRACE=1` prints the demo and flight events; `RC_STAR_STATS=1` the title stars' hash.

## 7. Results (the test expert fills this in)
