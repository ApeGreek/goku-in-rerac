---
status: open
job: media-cheats
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [ui, classes, hero, cinematic, render]
---

# Front-end media (movies outside gameplay, the credits slideshow, the end of the game, Goodies) and the cheats

## 1. Summary

Batch 6, lane `media` (docs/plan/progression.md `## media`). Built:

- **Movies outside gameplay** on the existing player: `PlayMovieB(n)` = `mpegs[40 + n]`, `PlayMovieC(n)` = `mpegs[70 + n]`
  (language 0), the title's attract movie `mpegs[80 + i]` (entry fade 12, replay mode 2, back to the interrupted mode, no
  music restart), the page menu's replays with the replay mode 0x15eed8 (3 → 2, 4 / 5 / 6 → 1) and its `FadeToBlack(16)`
  over the menu image, the return page reopening the menu after the movie / scene / slideshow.
- **The credits slideshow** (game mode 7): pictures, timing, dissolve, US jumps, text entries with their fades, Start
  skip, entry / exit fades. Callers: Goodies → Credits, and the boss 1422.
- **The end of the game**: the boss's `PlayMovieB(11)` (the ending, `mpegs[51]`) and its `PauseAllSounds(0x21)` — which is
  really `EnterMenuMode(0x21)` — now open the end-of-game page 0x1b7670 (stats, check boxes, hints; ○ challenge / save page,
  ✕ timewarp, both handed to the `saves` lane). The Helpdesk girl widget is not ported (G-CUT-008).
- **Goodies**: the unlock function `0x29aa60` (Sketchbook ≥ 15 skill points, Epilogue = 30, Making Of / Commercials = 10
  gold weapons, the descriptions), the Sketchbook and Epilogue pagers and arrows, the image widget's flags 1 / 2 / 0x400 /
  0x1000 (the Epilogue's per-language pictures across the following TOC fields), Credits / Cinematics / In-Level Movies /
  Making Of / Commercials post-actions, Quit Game (`EngineRequest::LeaveLevel { dest: −1 }`).
- **Cheats** (`rc_game::cheats`): the bytes 0x15edb0 / 0x15edc0 as one view for the hero (`Hero::cheats`, from
  `GameOptions::cheats`) and the classes (`Services::cheats`); the move-sequence entry `0x2285a0`; the Cheats page (list
  built by `0x28dbe8`, toggled by `0x294830`); the debug code entry `MenuInput` 0x298f80; every effect: the enemies' big
  head (`manip::big_head`, 21 class modules), the actors' (`manip::scene_big_head`, `actor_big_head`, the NPC head
  record), Ratchet's / Clank's heads, the mirror (render pass, gold bolt, crank), the health cheat in the nanotech
  clusters.
- **The options are live**: `GameOptions` (mirror, camera options, cheats) is rebuilt from the saved game every tick
  (it was set once at level start).

Plug-and-play: a new class with the enemies' big head calls `manip::big_head(w, scale, moby, list, owner, ofs)` (the
`0x278720` signature); one with the scene actors' calls `manip::scene_big_head(w, &[classes], list, scale)`; a talker on
`talking_npc::look_at_layout` gets the NPC head scale automatically; a class requests the credits / an ending movie / the
page menu with `cinematic::enter_slideshow` / `play_movie_b` / `enter_menu_mode`; the title plays attract movies with
`movie_render::play_attract(i)` and polls `MovieState::busy`.

## 2. Where it lives

- `crates/rc-game/src/cheats.rs`: `classify`, `MoveEntry::step` (0x2285a0), `CheatTables::read` (0x20a228 / 0x20a248,
  0x179b80, 0x1ba020), `entries` (0x28dbe8), `CodeEntry::step` / `apply_code` (0x298f80), `Cheats`.
- `crates/rc-game/src/slideshow.rs`: `Slideshow::{new, tick, draw}` (0x2ad558, 0x2ad738, 0x2ad6f0, 0x21ab78),
  `draw_entry` (0x21e8a0), `draw_line` (0x21e728), `dissolve`, `Tables::read` (0x174070, 0x173500).
- `crates/rc-game/src/movie_player.rs`: `MoviePlayer::with_entry`, `MOVIE_B_BASE_NTSC`, `MOVIE_C_BASE_NTSC`,
  `MOVIE_ATTRACT_BASE_NTSC`, `ATTRACT_FADE`, `MENU_FADE`.
- `crates/rc-game/src/menus/pause/media.rs`: `goodies_unlocks` (0x29aa60), `cheats_enter` (0x28dbe8), `stats_update`
  (0x295c98), `stats_draw` (0x2964a0), `check_box` (0x2932f0), `sketch_pager` (0x295770), `epilogue_pager` (0x295858),
  `epilogue_arrows` (0x295960), counts 0x279118 / 0x279188 / 0x2791d0.
- `crates/rc-game/src/menus/pause.rs`: `PageMenu::{load, enter, tick}` (the new roots, `goodies_unlocks`, the code entry),
  dispatch, `correlate` (follows action −1 items), `MenuOut::{code, end_choice, level_exit}`.
- `crates/rc-game/src/menus/pause/options.rs`: `opt_byte` / `set_opt_byte` (0x15edb0..0x15edbb), `quit_update` → level exit.
- `crates/rc-game/src/menus/pause/pages.rs`: `image_update` flags 1 / 2 / 0x400 / 0x1000, `field_entries` (contiguous fields).
- `crates/rc-game/src/cinematic.rs`: `enter_slideshow`, `play_movie_b`, `enter_menu_mode` (`EngineRequest::EnterMenu`,
  replaces `PauseSounds`), `banner_call`.
- `crates/rc-game/src/moby_update/manip.rs`: `big_head`, `actor_big_head`, `scene_big_head`, `big_head_scale`.
- Hero: `hero/idle.rs` (`head_look`, `clank_glow_blink`), `hero/bodies.rs` (`approach_head_scale`, record 28),
  `hero/bodies/clank.rs` (antenna glow 3.1), `hero/pose.rs` (`clank_sway`), `follow_camera.rs` (`eye_height`),
  `tick.rs` (the cheat entry after the footsteps).
- Engine: `rc-engine/src/media_render.rs` (`Slides`, `take_class_requests`, `request_slideshow`, `request_menu`,
  `request_level_exit`, `request_end_choice` / `take_end_choice`, `menu_fade_draws`), `movie_render.rs` (`MovieRequest`,
  `request`, `play_attract`, `MovieExit`, `MovieState::busy`), `menu_render.rs` (`open_menu`, `return_page_reopen`,
  `post_fade_step`, `start_slideshow`, the `Mode::Slideshow` arm, `MenuMode::replay`), `mirror_render.rs` +
  `assets/shaders/mirror.wgsl`, `scene_render.rs` (actors posed with their joint modifiers, the scene head record cleared
  at `enter_mode2`, the replay mode in the skip rule), `hud_images.rs` (`read_lump_picture` across fields),
  `gameplay.rs` (the cheat bytes and live options in, the toggle out, `cheat_patterns`).

## 3. Behaviours to verify

### B1. Move classification (`0x2285a0`)
- **Claim:** code by (state ticks 0x13f4e8, group 0x1413dc, state 0x1413d4): ticks = 15: group 4 / state 0xb by sector
  0x13f7a0: 0 → 1 (mirror on → 2), 1 → 2 (mirror → 1), 3 → 3, 2 → 0; group 4: 0x11 → 9, 10 → 10, 0xe → 5, else 0; group 6:
  0x14 → 6, 0x13 → 0x13fdb0 + 12, 0x15 → 4, else 0; any other group: 0x22 → 11, else 0. Ticks = 60: state 4 → 7, 8 → 8
  (any group), else 0. Any other tick count → 0.
- **Setup / Trigger:** `cheats::classify` with synthetic `MoveInput`s.
- **Expect:** exactly the table; ticks 15 with group 4 state 4 → 0 (not 7); ticks 60 state 0xb → 0.
- **Edge cases:** mirror only changes sectors 0 / 1; combo 2 → 14.
- **Suggested method:** unit test.

### B2. Move ring and pattern match (`0x2285a0`)
- **Claim:** nothing unless `open` (0x15eea0 ‖ 0x15ee20); a code goes into ring[idx], idx + 1; each pattern i (0..11,
  NUL-terminated, empty skipped) is compared with the last n codes (start idx − n, +16 when negative, wrapping at 16); the
  first match toggles active[i] (0 → 1 → 0) and returns `Toggled { slot: i, on }`; idx resets to 0 after 15.
- **Setup:** `CheatTables::read` of level 01's overlay (patterns: 0 [1 2 3 3 4 5 4 6], 1 [3 3 3 7 10 8], 2 [9×10 5 6],
  3 [3 6 4 5 6 1 2 7], 4 [1 1 1 1 12 13 14 6 2 2 2 2 5 7], 6 [4 4 4 4 3 7 3 7 4 4 4 4], 7 [10 3 3 3 10 3 3 3 10 3 3 3 7], 5 / 8..11
  empty).
- **Trigger:** feed the codes of pattern 1 through `step` (each a classify-producing input), then again.
- **Expect:** first full sequence → slot 1 on; repeating → off; the banner id 0x4fbe / 0x4fbf; codes across the ring wrap
  (start the sequence at idx 13) still match; a sequence with a wrong code inside does not; `open = false` → no ring write.
- **Edge cases:** pattern 2 (12 codes) after 4 junk codes (wrap); two patterns sharing a suffix (only the first index
  matches).
- **Suggested method:** unit test.

### B3. Toggle applied by the engine
- **Claim:** after the tick, 0x15edc0[slot] = 1, 0x15edb0[slot] = on, `ShowBanner(0x4fbe "Cheat Enabled" / 0x4fbf "Cheat
  Disabled", ticks(180))`; the toggled byte is already visible to the rest of the same tick (`GameOptions::cheats`,
  `Hero::cheats`).
- **Setup:** a game state with `game_beaten = 1`; level 01; Ratchet on foot.
- **Trigger:** perform pattern 1 (3 3 3 7 10 8: three back flips (state 0xb, sector 3), the crouch (state 4) held 60
  ticks, a Heli-Pack long jump (state 0xa), the Heli-Pack glide (state 8) held 60 ticks), or inject via a tick harness.
- **Expect:** chunk 7 / chunk 37 bytes, HUD banner text, the log line `cheats: tick …`.
- **Edge cases:** not beaten / completed → never; in a body (mode ≠ 0) → never (the entry runs only in 0x228870).
- **Suggested method:** tick harness test (rc-game) + QA.

### B4. Cheats page list (`0x28dbe8`) and toggles (`0x294830`)
- **Claim:** entries in table order slots [1, 3, 0, 7, 4, 6, 2] with labels [20506, 20507, 20508, 20509, 20510, 20512,
  20513], only those with 0x15edc0[slot] ≠ 0, values "on" 0x4f5a / "off" 0x4f5b; ✕ flips 0x15edb0[slot]
  (`*flag = *flag == 0`), sound 0 (confirm); Up / Down no wrap, sound 1.
- **Setup:** `PageMenu::load` of level 01; `cheats_ever` = [1, 0, 1, 0, 1, …].
- **Trigger:** open Goodies → Cheats (page 0x1b7fb0), ✕ on row 0.
- **Expect:** rows: "Actors have oversized craniums" (slot 0), "Levels are mirrored" (4), … only the set ones, in table
  order (slot 0's label comes 3rd in the table, so with slots 0, 2, 4 set the order is 0, 4, 2); ✕ flips the byte; the draw
  shows on / off from the byte.
- **Edge cases:** no cheat ever → an empty list (Down does nothing); a cheat set but never activated (only a save can
  do that) is not listed.
- **Suggested method:** unit test on the page menu (rc-game tests/ui).

### B5. Debug code entry (`MenuInput` 0x298f80)
- **Claim:** every page-menu tick; while held & 0xf == 6 (R2 + L1, the un-mirrored held 0x13cb00): each newly pressed Up /
  Down / Left / Right / □ / ○ (0x13cb04 & 0xf0a0) records 0 / 1 / 2 / 3 / 4 / 5 (priority in that order) up to 20; else
  the count resets. At the 20th: the first k in 2..0x92 with key i = table[(k·(i + 1)) & 0xff] for all i gives id = k − 2:
  < 0x25 item id owned + acquired; 0x25..0x36 `UnlockPlanet(id − 0x24)` (+ banner unless current level); 0x37..0x3c flag
  id − 0x37 = 1; 0x3d..0x5a skill point id − 0x3d (if 0: set, jingle level sound 1, banner 0x53d6 ticks(180)).
- **Setup:** the table 0x1ba020 of level 01; a pause menu open.
- **Trigger:** build the 20-key sequence for k = 2 + 0x3d (skill point 0) from the table and feed it with R2 + L1 held.
- **Expect:** `MenuOut::code = Some((SkillPoint(0), {banner: (0x53d6, 180), jingle: true}))`, 0x13d408[0] = 1; a second
  time nothing (already set: no banner, no jingle); releasing L1 mid-way resets.
- **Edge cases:** a 21st press is ignored (count stays 20 until release); Cross / Triangle are not recorded (mask).
- **Suggested method:** unit test (`CodeEntry::step` + `apply_code`), plus a page-menu tick test.

### B6. Enemy big head (`0x278720` family) — `manip::big_head`
- **Claim:** cheat 0x15edb7 off: an attached node is detached (`DetachManipulator`, node cleared), nothing else; on:
  attached once on list `list` of `moby`, scale xyz = `scale` every call (the node synced into the target's
  `joint_mods`). Callers and arguments (pvar offsets): pack_biter 193 (2.1, m, 0, 0x230), hover_zapper 252 (3.6, m, 2,
  0x230), flying_biter 63 (2.5, m, 0, 0x230), buzz_bomb 52 (2.7, m, 0, 0x280), area_stalker 1445 (2.1, m, 1, 0x220),
  orxon_flyers 1196 (2.5, m, 0, 0x270) and 1199 (2.5, m, 1, 0x270), orxon_brawler 1202 (2.5, m, 2, 0x2b0), gemlik_turret
  29 (2.7, rider (class 0x24, state < 0x80), 1, 0x70 of the turret), pokitaru_biter 1246 (2.5, m, 0, 0x280),
  aridia_sandshark 580 (2.1, m, 0, 0x2a0), aridia_flamer 612 (2.5, m, 1, 0x1a0), pokitaru_thrower 1231 (2.5, m, 0,
  0x1e0), batalia_runner 452 (2.5, m, 1, 0x260), kerwan_hound 573 (2.1, m, 0, 0x280), hop_gunner (2.5, m, 6, 0x170),
  horny_toad (2.1, m, 1, 0x200), path_enemy 459 (2.5, m, 2, 0x200), amoeboid 572 / 865 / 866 (2.9, m, 0, 0x270), critter
  (1.5, m, 0, 0x230), chicken 270 (5.0 gold / 7.7, m, 0, 0x20), boss 1422's pilot (2.5, pilot, 0, 0x1c0 of the boss, the
  pilot's state < 0x80).
- **Setup:** any level with the class; `Services::cheats` slot 7 set (or `cheats_active[7] = 1` in the game state).
- **Trigger:** one moby update.
- **Expect:** the moby's `joint_mods` holds a node on the list's target joint with scale (s, s, s); toggling the cheat
  off detaches it on the next update; with the cheat on, the head is drawn big in game.
- **Edge cases:** the target class without that joint list loaded (attached flag set, no node, `unported` counted);
  the turret's rider dead (state ≥ 0x80) → no call.
- **Suggested method:** class harness tests per class family (one representative each), visual QA.

### B7. Look-record scale (`0x251d70`) — `manip::big_head_scale`
- **Claim:** record +0x70 = scale with 0x15edb7, else 1.0. Kerwan trooper 574: (2.5, P+0x150) → P+0x1c0.
- **Expect:** with the cheat the trooper's head node scale 2.5 after `manip::look`; without, 1.0 (unchanged behaviour).
- **Suggested method:** unit / class test.

### B8. Scene actors' big head — `manip::scene_big_head`
- **Claim:** only in game mode 2 with 0x15edb0: the first scene actor whose class matches gets a node (list `list`,
  scale) in its `joint_mods`; once set (`Services::scene_head`) no other actor gets one until the next scene start
  (`enter_mode2` clears it). Callers: talking NPC 774 (own class or 0x32b, 2.1), mission NPC 730 / 790 (0x32b, 2.75),
  camera trigger (0x398 / 0x1bf, 2.1), commando 114 (own class, 2.75), Quartu's 1446 (own class, list 1, 2.75), boss 1422
  (0x4e1, 2.5). The scene renderer poses actors with their table moby's `joint_mods`.
- **Setup:** Novalis, the arrival scene 5 or the plumber scene 0 (actors 774 …), cheat 0 on.
- **Expect:** the actor's head drawn scaled in the scene; the second matching actor of the same scene not scaled; the
  next scene again scales the first match.
- **Edge cases:** mode 0 → nothing; cheat off → nothing (no detach either: the node stays until the actor is deleted at
  the scene end).
- **Suggested method:** scene harness / visual QA.

### B9. NPC head record (`TalkingNpcUpdate` 0x2ff6d8)
- **Claim:** with 0x15edb0, the pitch (head) record's +0x70 = 2.75 before the springs (`look_springs`), every
  `look_at_layout` user (774, 1446, the story lane's talkers); k / d unchanged (× 0x15ed64 = 1).
- **Expect:** the head node's scale 2.75 after `manip::look`; without the cheat 1.0.
- **Suggested method:** class test on 774.

### B10. Mouse 0x1b1 (`0x30df40`)
- **Claim:** states ≥ 3: with 0x15edb0 attach (list 5, record P+0x90) and scale 2.1; without: detach if attached; then
  the joint points 3 / 4.
- **Suggested method:** class test.

### B11. Ratchet / Clank heads (hero)
- **Claim:** `0x22b928`: record 17's scale source 0x15ee14 approaches 1.57 (0x3fc8f5c3) by ≤ 0.05 a tick with 0x15edb1,
  else 0.92; with 0x15edb3 and Clank on the back: record 18's scale = 0x15ee18. `0x2278c0` (every tick, every body):
  0x15ee18 approaches 1.8 (0x3fe66666) with 0x15edb3, else 1.0. `0x235e60`: with Clank and 0x15edb3 record 18 = 0x15ee18.
  Body 1: record 25 = 0x15ee18 (existing), the antenna glow approaches 3.1 × its class scale (×1.4 flashing) instead of
  1.7. Body 2 with 0x15edb3: record 28's scale = 0x15ee18 · 1.4. Follow camera eye in body 1: 1.2 (0x3f99999a) with
  0x15edb3, else 0.9.
- **Setup:** hero harness (Novalis), `GameOptions::cheats`.
- **Expect:** after N ticks the values converge (1.57 after 13 ticks from 0.92; 1.8 after 16 from 1.0); turning the
  cheat off converges back.
- **Suggested method:** hero unit tests.

### B12. Mirror cheat on screen (`mirror_render`)
- **Claim:** while 0x15edb4 = 1, the main camera carries `MirrorFlip`: the world image is mirrored left / right before
  the UI pass; the HUD, menus, movies and subtitles are not mirrored. The pad's left / right swap and the camera's rows
  already followed the byte; they now refresh every tick (live options).
- **Setup:** set "Levels are mirrored" via the Cheats page or a save.
- **Expect:** world mirrored, HUD normal; the stick's left turns Ratchet to screen-left.
- **Suggested method:** screenshot QA (frame-exact capture with and without).

### B13. Mirrored animations 0x15edb5 (gold bolt, crank)
- **Claim:** gold bolt pickup: mode |= 0x8000 on the bolt when 0x15edb5; bolt crank: the tangent −90° instead of +90°.
- **Setup:** a save with chunk 7 byte 5 = 1 (no in-game way sets it).
- **Suggested method:** class tests.

### B14. Health cheat (`0x300de0`)
- **Claim:** a nanotech cluster is taken also when `cheat 0x15edb6 && 0x13f510 == 0` (HP may be full); on take with the
  cheat: 0x13f510 = ticks(600); in state 3 with HP == max, 0x13f510 ≠ 0 and the cheat, the cluster's life +0x54 is held at
  ≥ ticks(250) and the four ring timers +0x58.. each +1 that tick.
- **Setup:** Novalis, a nanotech crate, Ratchet at max health, cheat 6 on.
- **Expect:** breaking the crate at full health → the orbs fly to him, he is invulnerable 600 ticks (`HeroFields::invulnerable`);
  without the cheat at full health they stay.
- **Edge cases:** f510 already running → not taken at full health; with HP < max it heals as before.
- **Suggested method:** class harness + QA.

### B15. Slideshow state machine (`rc_game::slideshow`)
- **Claim:** `new(us)`: state 0, index 1, time 0, fade 1.0; each `tick`: fade −0.05 (floored 0); state 0: when
  times[index] − 36 ≤ time: (non-US and index 0x27) or index > 0x2a → exit; else counter −1, state 1; state 1: counter < 32
  → dissolve(counter); counter > 35 → the US jumps (17 → 27 with time 8760; 37 → 39 with time 12980; non-US 8 → 17 with
  time 5520), state 0, index + 1, load file index % 20; then counter + 1, time + 1; Start → exit. The exit sets fade 0.
- **Setup:** `Tables::read` level 01 (times [0, 420, 720, …, 14600]).
- **Trigger:** run ticks until exit with no Start.
- **Expect:** first dissolve at time 384 (420 − 36), 32 dissolve steps, load of file 2 at counter 36; loads sequence
  2..17, then 8 (index 28) … 17, 0, 1, 2 (indices 40..42); exit when time reaches 14564 (index 43); total frames ≈ 14565 +
  the jumps' skipped time. With Start on tick k: exit at k.
- **Edge cases:** non-US path (index 8 → 17, exit at index 0x27) only with `us = false`.
- **Suggested method:** unit test (pure).

### B16. Slideshow draw
- **Claim:** the picture (512×416) first, the fade as black `trunc(min(f,1)·128)` when f > 0, then the text entries at
  time t: entry visible for start ≤ t ≤ start + duration; bar alpha `trunc(a·112)`, text alpha `trunc(b·128)` with a / b
  as in the module doc; join = 1 → one line with " - " for each run of 0x01; else one row per 0x01 run, 0x1a apart;
  align 0 / 1 / −1; width scaled by a; bar at (x0 − 0x20, y − 0xe, x1 − x0 + 0x40, 0x1c); shadow (x0 + 1, y − 7) black,
  text (x0, y − 8) 0xe0e0e0, regular font, colour codes off.
- **Setup:** entry 0 (msg 20649 "Created and Developed by\x01\x01Insomniac Games", x 256, y 44, start 30, dur 2220, align 0,
  join 1) at t = 30, 38, 46, 2246, 2250.
- **Expect:** t = 30: a = 0 (no width), b = 0; t = 38: a = 0.5; t = 46: a = 1, b = 0; t = 2246: b = (2250 − 2254)/8 < 0 → 0;
  t = 2250: a = 0.
- **Suggested method:** unit test on the draw list.

### B17. Slideshow in the engine (`media_render::Slides`, menu_render's mode 7)
- **Claim:** request → mode 7 (`Services::game_mode` 7 at once from a class), 8 entry fade frames (black over the frozen
  frame; all black after the menu's fade), then the slideshow frames, `sound_update` every frame, Start skips, the exit's
  12 fade frames over the last frame, mode 0, `game_mode` 7 → 0; the return page (Credits) reopens Goodies.
- **Setup:** Goodies → Credits (needs `game_beaten`); or the boss path.
- **Expect:** pictures dissolve in columns, credits text fades in / out; log lines `media:` / `menus: … slideshow`.
- **Suggested method:** engine QA (frame captures at frames 400, 420 for the first dissolve).

### B18. Page menu post-actions with fade and replay mode
- **Claim:** after the close's 2 ticks: 16 frames `FadeToBlack(16)` over the menu image (snapshot + 0x30 black + black
  coverage), then: game mode 0; replay 0x15eed8 (old saved): 3 → 2 + in-level movie arg; 4 → 1 + `PlayMovieB(arg)`; 6 → 1
  + `PlayMovieC(arg)`; 5 → 1 + `StartScene(arg)` (help killed, fade 0); 7 → unchanged + slideshow; the movie's entry fade
  black; sounds: for movies `movie_stop` then the close's unpause; mode Gameplay (the movie / scene take over) or 7.
- **Then:** back in mode 0 with the return page set: the menu reopens before the tick on that page, replay restored.
- **Setup:** Goodies unlocked; Cinematics → item 0 (action 8, arg 0: `mpegs[40]`), item 2 (action 7, arg 1: `mpegs[3]`);
  Commercials (action 10); In-Level Movies (action 6, scene).
- **Expect:** files `mpegs[40]` / `[3]` / `[70..72]`; Start alone skips 4 / 6 replays; any button skips 3's; after the movie
  the Cinematics page is open again with the cursor kept.
- **Edge cases:** the Making Of (`PlayMovieC(3)`, 353 MB, interlaced); a scene replay's skip with Start alone.
- **Suggested method:** engine QA with `RC_MENU_TRACE=1`; movie_player unit test for `with_entry(true, 4)` (FadeIn black).

### B19. Goodies unlocks (`0x29aa60`)
- **Claim:** on every `EnterMenuMode`: skill points (≤ 30) > 14 → Sketchbook action 3 else 2; = 30 → Epilogue 3 else 2;
  gold weapons (≤ 10) = 10 → Making Of 10 else 2, Commercials 3 else 2; descriptions 0x4fd3 / 0x4fd9, 0x4fd4 / 0x4fda,
  0x4fd7 / 0x4fdb, 0x4fd8 / 0x4fdb. Goodies visible when beaten ‖ completed.
- **Setup:** `PageMenu::load` level 01; game states with 14 / 15 / 30 skill points, 9 / 10 gold weapons.
- **Expect:** the item actions and the label table; ✕ on a locked entry plays the denied sound; on an unlocked one
  opens the page / starts the movie.
- **Suggested method:** unit test.

### B20. Sketchbook / Epilogue pagers and pictures
- **Claim:** Sketchbook: ✕ next / ○ previous of 30 (the hints list's cursor), the picture = `sketchbook[cursor]`, sound 1
  on change; △ back to Goodies; Start / Select close. Epilogue: → / ✕ next, ← / ○ previous of 12, picture = page +
  [0, 0, 12, 36, 48, 24][language] entries from `epilogue_english` (French → `epilogue_french`, …), arrows: W0 "○" at
  (w − 24, h/2 − 8) with the arrow turned π at (12, h/2); W2 "✕" at (4, h/2 − 8) with the arrow at (40, h/2), 8 × 16.
- **Suggested method:** unit test on the pagers and `field_entries(0xbc8) == 60`; visual QA.

### B21. End-of-game page (kind 0x21)
- **Claim:** the boss's `enter_menu_mode(0x21)` opens page 0x1b7670 with no close keys; W1 draw: "You've got:" centred
  at (w/2, h/5 − 8); rows at 2h/5 − 8, 3h/5 − 8, 4h/5 − 8: "%d of 40 Gold Bolts" (0x14bec0[20·4] ≤ 40), "%d of 10 Gold
  Weapons" (0x13e520[37] ≤ 10), "%d of 30 Skill Points" (0x13d408[32] ≤ 30) at x 0x14, the small font when the widest + 0x18
  > w; check box at (0xb, y + 9), ticked when full. Keys (0x13cae4): ○ → with card status 1 / 0x10 the save page 0x1b9440,
  else `EndChoice::Challenge` + `LeaveLevel { dest: 0 }`; ✕ → `EndChoice::Timewarp`, menu closes.
- **Suggested method:** unit test on the widget functions; engine QA at the end of level 18.

### B22. Class requests (`cinematic`)
- **Claim:** `enter_slideshow` / `play_movie_b(n)` (n < 0 ignored; help killed) / `enter_menu_mode(kind)` set
  `Services::game_mode` 7 / 1 / 3 and push their request; the engine takes them right after the tick
  (`media_render::take_class_requests`); `LeaveLevel` etc. stay in the list.
- **Suggested method:** unit test on the request list.

### B23. Movie requests (`movie_render`)
- **Claim:** `MovieRequest::movie_b(11).file == 51`, `movie_c(3).file == 73` with language 0, `attract(2).file == 82` with
  replay 2, fade 12, exit FrontEnd; the exit restores the interrupted mode for FrontEnd and sets replay 0; Gameplay exits
  clear replay 2 → 0, restart the level music, `game_mode` 1 → 0.
- **Suggested method:** unit tests (engine crate tests).

## 4. Shared code touched (regression risk)

- `rc-engine/src/gameplay.rs` tick: `p.game.options` and `p.game.camera.opts` are now rebuilt from the saved game every
  tick (were set once at level start): the Options page's camera settings now take effect at once (intended: the game
  reads the globals live). Risk: tests that set `game.options` by hand and then run the engine tick.
- `rc-game/src/tick.rs`: `GameOptions` gained `cheats` / `cheat_entry`; `Game` gained `cheat_patterns` / `cheat_toggled`;
  the hero gets `cheats` copied every tick; the cheat entry runs after the footsteps (on foot only; no rand draws).
- `rc-game/src/hero.rs`: `Hero::cheats`, `Hero::cheat_moves` (new fields; `Hero::new` zeroes them).
- `rc-game/src/hero/idle.rs`: `clank_glow_blink` now approaches 0x15ee18 every mode-0 tick (no change without the cheat:
  it stays 1.0).
- `rc-game/src/hero/bodies.rs`: `glow` uses `approach_head_scale` (same result without the cheat).
- `rc-game/src/moby_update/services.rs`: `Services::cheats`, `Services::scene_head`.
- `rc-game/src/moby_update/manip.rs`: new functions only.
- 21 class modules: one call each (no change with the cheats off; detach only when attached).
- `rc-game/src/moby_update/classes/talking_npc.rs`: `look_springs` writes the head record's scale request only with
  cheat 0; the `scene_big_head` call at the top of the update.
- `rc-game/src/menus/pause.rs`: the loader now also loads the kind 0x21 / 0x23 / 0x2d roots, the save page and the
  Goodies' locked pages (more pages / widgets: tests counting pages may change); `correlate` follows action −1 items (the
  relocation map of other levels gains those pages); `enter` calls `goodies_unlocks` (the Goodies list's items 5..8 change
  from −1 to 2 / 3 / 10, the label table is patched); the stub "cheat entry 0x298f80" is gone (the code entry runs).
- `rc-game/src/menus/pause/options.rs`: `quit_update` also sets `level_exit`; `opt_byte` / `set_opt_byte` know the cheat
  bytes.
- `rc-game/src/menus/pause/pages.rs`: `image_update` flags 1 / 2 / 0x400 / 0x1000 (none of the in-level pages but the
  Epilogue use them), `field_entries` counts contiguous fields (a picture index past a field's count is now available
  when the next field follows: the game reads the TOC that way).
- `rc-game/src/cinematic.rs`: `EngineRequest::PauseSounds` removed (replaced by `EnterMenu`), `pause_sounds` removed;
  `enter_slideshow` / `play_movie_b` set `game_mode`.
- `rc-game/src/movie_player.rs`: `MoviePlayer::with_entry`; default behaviour unchanged.
- `rc-engine/src/movie_render.rs`: `MovieRequest` gained fields (constructors); the replay mode comes from
  `MenuMode::replay` (was always 0); the exit clears `game_mode` 1.
- `rc-engine/src/menu_render.rs`: the open logic moved to `open_menu` (same steps); `return_page_reopen` runs before the
  tick; the close's audio for movie / scene / slideshow post-actions is delayed by the 16-frame fade; `hook.replace_hud`
  also in mode 7.
- `rc-engine/src/scene_render.rs`: actors posed through `evaluate_posed` with their table moby's `joint_mods` (empty
  without the cheat: same pose); the scene skip rule reads `MenuMode::replay`; the requests' fallback forwards
  Slideshow / MovieB / EnterMenu.
- `rc-engine/src/hud_images.rs`: `read_lump_picture` resolves an index past its field into the following field.
- `rc-game/src/moby_update/classes/units/veldin_boss.rs`: `enter_menu_mode(0x21)` instead of `pause_sounds`.

## 5. Not ported (don't test as working)

- The end page's Helpdesk girl (W0 0x2992f0 / 0x299670 / 0x299190 / 0x299268): G-CUT-008.
- The end page's timewarp restore / challenge mode / memcard save: the `saves` lane (`media_render::take_end_choice`).
- The trippy contrails (0x15edb2): their classes (level05 0x2d7920, level11 0x3192c8, level17 0x2f3b68) are not ported
  (G-SAV-006 leftovers, G-CLS-001).
- The disguise's (body 3) Ratchet head record 1.9: G-WPN-006. The red-dot glow quad size in body 1: G-REN-005.
- The story lane's new talkers' scene manipulators (aridia 0x2e0cd8 / 0x2e1868, eudora 0x2e2f90): G-SAV-006 leftovers.
- The 4 extra iterations of `0x28dbe8`'s loop (reading labels as slots, bss bytes expected 0): n/a.
- 0x1ba268 (the third Goodies unlock): no writer found [L].
- The scene FOV / offset cheat: identity in the game (0x15f59c = 1.0, 0x15f5a0 = 0 reset by `DialogStreamStart`).
- The IOP stream's waits in the slideshow; PAL pictures and timings (`us = false` only in tests).
- After a menu-launched movie, the reopened menu's snapshot is of the world (the game's frame buffer then holds the
  black of the exit fade) [L].

## 6. In-game QA spots (for the user)

- Goodies need `game_beaten` (chunk 31) or `completes` (chunk 2) ≠ 0: use a save with the game beaten (or the saves
  lane's dev override). `RC_LEVEL=1`, Start → Goodies.
  - Credits: the credits pictures dissolve column by column every ~5 s, the names fade in over a bar; Start skips; you
    return to Goodies. Listen: the level music continues.
  - Cinematics: "Ratchet works on his ship" (`mpegs[40]`, Start alone skips), "Infobot - Drek's plan" (any button skips).
    After each: back on the Cinematics page.
  - In-Level Movies on Novalis: "Sorry about that." = scene 5 replay.
  - Sketchbook (≥ 15 skill points), Epilogue (30), Making Of and Commercials (all gold weapons): ✕ / ○ page; locked ones
    play the denied sound and show "You have to complete any 15 Skill Points to unlock this feature".
  - Cheats: only the cheats ever entered; ✕ toggles on / off.
- Cheat entry (beaten save, `RC_LEVEL=1`, on foot, Heli-Pack owned): "Ratchet has a big head": three back flips, a
  crouch held 1 s, a Heli-Pack long jump, a Heli-Pack glide held 1 s → banner "Cheat Enabled", his head grows over ~13
  ticks. (Codes: progression.md `## media`.) "Levels are mirrored": world mirrored, HUD not; left / right on the stick swapped.
- Big heads: `RC_LEVEL=3` (Kerwan troopers / hounds) with "Enemies have massive domes" on; `RC_LEVEL=1` the plumber
  scene with "Actors have oversized craniums" (the plumber's head 2.1 in the scene, 2.75 when he talks).
- Health cheat: Novalis, full health, break a nanotech crate: the orbs come anyway, Ratchet blinks invulnerable 10 s.
- Pause menu code entry: hold R2 + L1 in the pause menu and press 20 d-pad / □ / ○ keys of a code (codes in
  0x1ba020's scheme; e.g. a skill-point code gives the jingle and the banner).
- The end: `RC_LEVEL=18`, beat the boss (or force phase 9): credits → the ending movie `mpegs[51]` → the end page with
  the totals and check boxes; ○ / ✕ log their choice.
- Quit Game (Options → Quit Game → ○): `LeaveLevel { dest: −1 }` logged / handled by the travel lane.

## 7. Results (the test expert fills this in)
