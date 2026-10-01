---
status: open
job: menus-flow
date: 2026-10-01
commit: <filled in by the coordinator at commit time>
areas: [ui, world]
---

# Menus and flow leftovers: pause pages, language switch, freeze kinds, the Helpdesk girl, vendor leftovers

## 1. Summary

Batch 6, lane `menus` (G-UI-002, G-UI-020, G-UI-019, G-CUT-008, G-UI-006). All addresses are level01.elf.

- **Pause pages (G-UI-002).** Every widget callback of the page tree now dispatches (41 pages / 145 widgets reached
  from the roots on level 01; `PageMenu::stub_calls` stays empty on the disc's levels). New: the missions page (the
  list 0x293090, the missions widget's per-destination cursor + enter 0x28fe28 + the Up/Down keys of 0x28fec8), the
  streamed picture widget 0x293398 / 0x293670 (the missions page's `mission_ss` picture and the ship's planet-select
  picture), the confirm page's gold-bolt panel 0x292980, the kind-0x23 scroller page 0x1b6fb8 (0x295de0 / 0x295df8 /
  0x296170; no caller on the disc), the 3D Ratchet's Persuader / Map-o-Matic / Bolt Grabber (menu_models).
- **Language switch (G-UI-020).** Action 9 → `hud_render::set_language` (every later load reads it), the front end's
  message table = `all_text[lang]` every frame, PRESS START per language at once.
- **Freeze kinds 0, 1, 2, 4, 6 (G-UI-019)** on `menus::freeze`, plus the pause-test decision function
  `menus::mode::in_level_trigger` (0x2aba68) replacing the engine's simplified test.
- **The Helpdesk girl (G-CUT-008)** on the end page: state machine, voice lines via the dialogue player, 3D render.
- **Vendor (G-UI-006):** the approach beam's scan plane (FX 0x1b) and the beam's camera-yaw turn, the popup's
  ammo-quantity backdrop (+0x500), the four arm manipulators 0x166300, `VendorExit`'s line stop, the weapon-demo
  decision and request (playback not ported: the engine runs the plain exit).

Plug and play: a race class fills `Freeze::ctx` each frame and calls `Freeze::init(KIND_RACE, 0, 0)`; a vehicle class
sets `TriggerIn::riding_class` (engine) and applies `FreezeOut::vehicle_quit`; a new dialogue range only needs a row
in `menu_render::dialogue_stream`; another streamed-picture widget is data (+0x30 TOC field, +0x34 flags).

## 2. Where it lives

- `crates/rc-game/src/menus/freeze.rs` — `Freeze::{init, update, draw}`, `update_race`, `update_video`, `draw_race`,
  `draw_video`, `pulse`, `pulse_frame`, `FreezeCtx`, `RaceQuit`, `gp` constants (coverage table in the module doc).
- `crates/rc-game/src/menus/mode.rs` — `TriggerIn`, `Trigger`, `in_level_trigger`, `VEHICLE_CLASSES`.
- `crates/rc-game/src/menus/pause/media.rs` — the girl (`girl_enter/update/tick/draw/leave`, `Girl`, `GirlClass`),
  the scroller (`scroller_enter/update/draw`, `Scroller`, `ScrollSrc`); coverage table in the module doc.
- `crates/rc-game/src/menus/pause/map_page.rs` — `mission_list`, `Missions`, `missions_enter`, `missions_keys`,
  `missions_unfocused`, `wrap_small`, `missions_draw`, `Picture`, `picture_enter/leave/update/draw`, `gold_panel_draw`,
  `MapTables::gold_totals`, `Mission::{picture, descs}`; coverage table in the module doc.
- `crates/rc-game/src/menus/pause/planet_select.rs::missions_update` (0x28fec8).
- `crates/rc-game/src/menus/pause.rs` — dispatch, `Data::{Missions, Picture, Scroller}`, `Addrs::{missions_label,
  missions_image}`, `MenuOut::{voice, voice_continue, voice_stop}`, `enter` sets `media.kind23`.
- `crates/rc-game/src/menus/pause/gadgets.rs` — `GirlView` (kept in `MediaMenu::girl_view`; the engine copies it into `menu_models::GadgetsPreview::girl`).
- `crates/rc-game/src/frontend.rs` — `FrontEnd::press_texs`, `set_language`, `LANGUAGES`.
- `crates/rc-game/src/menus/vendor.rs` — `WeaponDemo`, `DEMO_STREAM`, `arm_manipulators`, `VendorOut::{detach_arms,
  weapon_demo, stop_voice}`, `VendorScene::popup_panel`, substates 2 / 3.
- `crates/rc-engine/src/menu_render.rs` — trigger wiring, `freeze_effects`, `menu_voice_frame`, `dialogue_stream`,
  `apply_language`, the front end's `all_text` table, the girl class load.
- `crates/rc-engine/src/menu_models.rs` — `girl_anim_class`, `Role::Girl` (5th canvas), `Role::Extra`.
- `crates/rc-engine/src/hud_render.rs` — `language()` / `set_language` (runtime 0x15ed88).
- `crates/rc-engine/src/vendor_render.rs` — the scan plane, the camera-yaw beam, the popup canvas + `SLOT_POPUP_PANEL`.
- `crates/rc-engine/src/interact_render.rs` — `arms`, `joint_points_posed`, `remote_vendor`, the demo fallback, the exit's voice stop and the remote exit.
- `crates/rc-engine/src/saves.rs` — `front_end` fills `press_texs`.

## 3. Behaviours to verify

### B1. Freeze kind 0, small frame open / close timing (0x2249b0 case 0)
- **Claim:** after `init(0,0,0)`: texts [0x4f6e, 0x5248, 0x5249], fade 0, step 0, t (0x1734e0) 0, label (0x1734e4) 0.
  Step 0: t +1 per frame up to 8, then label +1 up to 8, then step 1 (17th frame after the 16 counts). Step 1: ✕
  (0x40) → step 2; else ○ (0x20) or Start (0x800) → step 3; nothing else changes. Steps 2/3: label −1 to 0, then t −1
  to 0, then the action on the frame both are 0.
- **Setup:** `Freeze::init(KIND_RACE, 0, 0)`, `MemCard::absent()`, level 5.
- **Trigger:** 17 empty frames, ✕, then 17 frames.
- **Expect:** (t, label, step) sequence (1,0,0)…(8,0,0),(8,1,0)…(8,8,0),(8,8,1); after ✕ step 2, label 7..0, t 7..0,
  then `out.mode == Some(0)`, `out.race_quit == Some(RaceQuit{stop_sound: None, count: None})` (ctx.sound −1, ticks 0),
  `out.resume_sounds == true`.
- **Edge cases:** ○ path → `race_rewind` only when `ctx.race_stage > 2` (and stage −1); `hero_state_ticks > 4200` on
  level 5 → `count == Some(0)`, level 16 → `Some(1)`, level 8 → None; `ctx.sound = 7` → `stop_sound == Some(7)` and
  ctx.sound becomes −1; ✕ and ○ the same frame → ✕ wins.
- **Suggested method:** unit test on `Freeze::update`.

### B2. Freeze kind 0 draw (0x2237d8 case 0)
- **Claim:** stage < 3: `pulse_frame(185 − ⌊50·s⌋, 185 + ⌊50·s⌋, 256 − ⌊100·s⌋, 256 + ⌊100·s⌋)` with s = clamp(t/8,
  0.1, 1); three centred texts at y 150 / 174 / 198 (0x4f6e, 0x5249, 0x5248), colour 0x00ffc0c0 while label = 0 else
  `tween(label/8, 0x00ffc0c0, 0x80ffc0c0)`. Stage ≥ 3 (results): frame 205 ± 120·s, 256 ± 100·s; line 1 = "1st " +
  msg 0x5240 (place 1 green tween 0x0060ef60→0x8060ef60, else "2nd/3rd/Nth " red 0x006060ef→0x806060ef) at y 110;
  best time == time: 0x50a3 at y 148 and "m:ss:hh" (3600 ticks/min) at y 168 green, else 0x50a2 at y 158 red; best score
  set and == score: 0x50a5 at y 196 and the score at y 216 green, else 0x50a4 at y 206 red; 0x5249 at y 254, 0x5248 at
  y 278. The frame = one backing `Draw::Rect16` (alpha of the pulse | 0x04) + 8 bevel bars (offsets in
  `pulse_frame`); pulse = `tween(sin(((vsync%180)/180)·6.28318 − 3.14159)/2 + ½, 0x80e08060, 0x80d06050)`.
- **Setup:** kind 0 with t = label = 8; ctx {race_stage 3, place 2, time 7385, best_time [7385, 0], score 12,
  best_score [12, 0], level 5}.
- **Expect:** the draw list (9 Rect16 + texts) with the values above; "2nd " prefix; time text "2:03:08".
- **Edge cases:** place 1 / 3 / 4 suffixes; level 16 uses index 1; PAL (3000 ticks/min); best_score 0 → red branch.
- **Suggested method:** unit test on `Freeze::draw` with synthetic `MenuAssets` messages.

### B3. Freeze kinds 1 / 4 / 2 (0x2249b0 cases 1, 2, 4; 0x2237d8 default / case 2)
- **Claim:** kind 1: every frame `all_visible`; △ → mode 0 + `vehicle_quit`; ✕ → mode 0. Kind 4: △ → mode 0 +
  `leave_body` (no all_visible); ✕ → mode 0. Kind 2: `all_visible`; ✕ → mode 0; △ nothing. Draws: kinds 1/4 the pulse
  frame (0x50, 0x9c, 0xb0, 0x150), 0x5229 at (256, 0x5a) 0x8000c0c0, 0x4ee0 at y 115 and 0x524a at y 130 0x80ffa888;
  kind 2: the frame (100, 0xa0, 0xb0, 0x150), 0x524a at (256, 0x7a) 0x8000c0c0.
- **Setup/Trigger:** `init(k, 0, 0)` and presses.
- **Expect:** as claimed; `resume_sounds` on each close (mode 0).
- **Suggested method:** unit tests.

### B4. Freeze kind 6 (the PAL 60 Hz test)
- **Claim:** init fade = 30, step 0. The fade decrements twice per frame in step 0 (entry + step) → step 1 after 15
  frames. Step 1: △ → previous mode; ✕ → `fade_to_black = Some(4)`, `video_mode = Some(0)`, step 2, t = 600.
  Step 2: t −1 per frame, at 0 → `video_mode = Some(1)`, step 3; △ → fade 4 + video 1 + previous mode; ✕ → video 0 +
  previous mode. Step 3: ✕ → previous mode. Draw: `UiFrame(100, 300, 0x60, 0x1a0, (1 − fade/30)·80)`, buttons 0x524e at
  (0xca, 0x118) and 0x524b at (0x135, 0x118) in steps 0..2, 0x524a at (256, 0x118) in step 3, text window
  (100..300, 96..416, anchor 256, y 104, line 16, centre) with 0x522a / 0x522b (step 2) / 0x522c (step 3).
- **Suggested method:** unit tests.

### B5. The pause tests (0x2aba68, `in_level_trigger`)
- **Claim:** the order and gates listed in the function doc: level 15 + body 2 + Start → `Freeze(4)` (None with camera
  lock 0x13); group 9 + state 0x32 + riding class 0x45 / 0x563 / 0x4da + Start → `Freeze(1)`; mode 0 + level 8 / 12 +
  state 0x32 + Start → `Freeze(0)` (no 8-frame wait); Start → `Menu(0)` with ≥ 8 frames, state ≠ 0x72, and group 22 or
  (swap state ≠ 2, state ∉ {0x32, 0x1d}, not fallen, HP ≠ 0, mode 0); Select / R3 → `Menu(10)` (≥ 8 frames, group ≠ 22,
  state ∉ {0x72, 0x32, 0x1d}, not fallen, HP ≠ 0, not riding, not the debug step).
- **Edge cases:** pad disconnected = Start; group 22 with state 0x32 + Start → `Menu(0)`; fell_out → no menu, no map;
  frames 7 → nothing (except kind 0's rider case).
- **Regression:** previously the engine opened the pause in fewer cases (no fell / group-22 rules): check the existing
  pause/map behaviour on Novalis still opens with Start at rest and Select.
- **Suggested method:** unit tests on `in_level_trigger`; QA in game.

### B6. Freeze effects in the engine (`menu_render::freeze_effects`)
- **Claim:** `all_visible` → `svc.visibomb.all_visible_at = counter − 1` (the occlusion shows everything);
  `leave_body` → the body moby `visible = 0`, `has_collision = false`, `mode |= 1`, `HeroCall::LeaveBody{game_mode: 4}`,
  then (with `bodies.entry_pose`) the pose teleport + `SetState(0)` + camera reset behind the hero; `race_quit` → the
  entry-pose teleport. The dialogs run and draw without a memory card set up (`MemCard::absent`).
- **Suggested method:** QA in game (Giant Clank on Quartu), or an engine-level test with a staged `Play`.

### B7. The Helpdesk girl (media.rs, G-CUT-008)
- **Claim:** enter: with `girl_class` set → `Girl{stream 0, spawned, state 0, count 0, anim on seq 0}`; without →
  stream 3, not spawned (no draw). Update: stream 0 → 1 → 2 (state 0, or 4 when `completes ≠ 0`). State 0/2/4: count 0
  → `out.voice = 60000 + 6·(state/2) + max(lang − 1, 0)`; count +1 clamped to 120; proceed at count 120 only with
  `media.voice_state == 3`: count 0, state +1, `set_sequence(seq state/2 + 1, frame 0, blend 24)`. State 1/3/5: once
  seq_a == seq_b and count 0 → `voice_continue`, count 1; when the anim wrapped (flags & 2) → state 2 (from 1) or 6,
  count 0, blend to seq 0 over 24. State 6: count +1 clamped 240, at 240 → state 0 (or 4). `girl_tick` advances the
  anim after the widget updates. Leave → `girl = None`, `voice_stop`. Draw → `MediaMenu::girl_view` with the widget rect (cleared by every draw),
  return 4.
- **Setup:** a `PageMenu` (synthetic) with a widget whose callbacks are 0x2992f0/0x299670/0x299190/0x299268 and a
  `GirlClass` built from any class with ≥ 4 sequences; lang 1 → v = 0; lang 3 → v = 2.
- **Expect:** the voice ids 60000 / 60006 (first ending) or 60012 (completes ≠ 0); the 120-frame wait; no progress while
  `voice_state ≠ 3`; the 240-frame wait in state 6; state cycle 0,1,2,3,6,0…
- **Edge cases:** lang 0 → v 0; voice not ready for a long time (count stays 120); leaving mid-line sets voice_stop.
- **Suggested method:** unit tests; engine: `menu_voice_frame` loads `post_credits_audio/{n:03}.bin` for 60000 + n
  and plays it on `voice_continue` (audible regardless of the HelpDesk voice option); stop on leave → `StopSpeech`.

### B8. The girl's data (`menu_models::girl_anim_class`)
- **Claim:** `post_credits_helpdesk_girl_seq.bin` WAD-decompresses; words at 0, 8, 16 are the offsets of sequences 1..3,
  each parsed with pointers relative to itself; the class's sequence vector grows to ≥ 4.
- **Suggested method:** golden test on the extracted lump + level 18's class 0x7a5 (check it exists on level 18; if
  not, the girl is not drawn: record which levels have class 1957).

### B9. Missions list (`mission_list`, 0x20bc00)
- **Claim:** a mission is listed iff `flags & 2 == 0`, status ≠ 0, and not (flags & 1 and status 2); its text = name, or
  with descs: 0x523e when done else `descs[cb_value]`; picture = +0x12 + cb_value; done = status 2; `all` true iff every
  mission without flag 2 has status 2.
- **Suggested method:** unit test with synthetic `Mission`s; disc test on Novalis' list (level 1 table 0x1870f0).

### B10. Missions widget (0x28fe28 / 0x28fec8) and its draw (0x293090)
- **Claim:** enter: status recomputed for `dest`, count = listed count, cursor of dest ≠ −1 → the label 0x1b3dc0's id and
  the picture widget 0x1b3c80's explicit index set. Unfocused: cursor[dest] = −1 each tick. Focused: Up → (c + n − 1) %
  n, Down → (c + 1) % n, sound Cursor on change, label/picture refreshed on Up/Down or a planet change (R1/L1, sound
  Cursor + `dest_changed`). Draw: "Missions" 0x4f59 large at (4,4) 0x80ffa888; per listed item: wrapped small text at x
  0x10 (width w − 0x11), yellow 0x8020ffff without colour codes when it is the cursor; a check box at (9, y + 10) or
  (9, y + 16) for two lines, ticked when done; y += 16 per line; all done → 0x523d small, window (0..h, 0..w, anchor h/2,
  y_start y + 8, centred), yellow; returns 2.
- **Edge cases:** cursor −1 then Up → n − 2 (game quirk); count 0 → no key effect; the confirm page's widget 0x1b6950
  writes the same label/picture.
- **Suggested method:** disc test (pause_pages-style harness on Novalis: open the map page, ✕ to missions, press Down).

### B11. Streamed picture widget (0x293398 / 0x293670)
- **Claim:** enter: state 0, index −1. Index source: flag 1 explicit (−1 → nothing), flag 2 dest, flag 4 focused grid
  cursor, else focused list cursor ≥ 0. State 0/2 with a new index (and < the TOC field's entries) → state +1, index
  set; state 1/3 → shown = index, state 2. Draw: state ≥ 2 → `MenuDraw::Image{Lump{field, shown}, 0, 0, w, h, 0, 0,
  tw, th, 0x80808080}`, return 0x10; else 0. Missions page widget: field 0x500 (`mission_ss`), 0xdb × 0xaf; planet
  select: field 0x788, 0x132 × 0x6c, index = dest.
- **Suggested method:** unit + disc test (planet select page: change the cursor, see the index step after 2 frames).

### B12. Confirm page gold-bolt panel (0x292980)
- **Claim:** the gold bolt (spin from `gold_enter`) at camera + (8, 1.3, −0.1), pitch −1.9; text "%s %d %s %d" =
  msg 0x4f4f, gold bolts found on dest (`levels[dest].gold_bolts` non-zero count), msg 0x4f53, `gold_totals[dest]`
  (Novalis table 0x1c4e08 = [0,3,4,3,1,2,2,2,2,2,2,1,2,1,4,2,2,2,3,0]); right-aligned at (w − 16, 200) black and
  (w − 17, 199) 0x80ffa888, plus the centre-crop offset; return 8.
- **Suggested method:** unit test on the draw; QA (ship → planet select → ✕ a planet).

### B13. Kind-0x23 scroller (0x295de0 / 0x295df8 / 0x296170)
- **Claim:** the state script in `scroller_update`'s doc (labels 0x50a9 / 0x50d4 / 0x50d6 / 0x5106 / 0x5136 / 0x5138 /
  0 / 0x5144 / 0, lists 0x1b9618 / 0x1b96c8 / 0x1b9788 / 0x1b9818 / 0x1b9830, single ids 0x50d5 / 0x5137 / 0x5143 /
  0x5175; waits 180 / 240 / 300; scroll +10 (+20 with L1+R1+L2+R2 held)); state 19 at timer 0 closes the menu only when
  entered with kind 0x23. Draw (direct, screen coords): the list's messages from y + 4 − scroll/16, 10 px apart;
  `fits` when the end + 0x18 < bottom.
- **Suggested method:** unit test with `PageMenu::enter(0x23, …)` (page 0x1b6fb8 loads from the disc tree).

### B14. Language switch (G-UI-020)
- **Claim:** front end Options → Language (action 9, arg = language): `PageMenu::lang` = arg; `FrontEnd::set_language`
  swaps PRESS START to `press_texs[arg]` at once; the front end's message table becomes `all_text` block arg on the
  next frame; `hud_render::language()` returns arg from then on (the next level load's text, the help voice bank, movies,
  scenes, space plates); `svc.help.text.lang` = arg.
- **Suggested method:** QA in game (RC_FRONTEND=1); unit test on `FrontEnd::set_language`.

### B15. Vendor: scan plane and beam turn (0x2ba9c0)
- **Claim:** per drawn approach beam: the cone's V phase +0.01 (wrap −1) and the scan phase t +0.01 (past 1 → 0); the
  beam and the plane turn by the camera's yaw 0x167258 (not toward the camera); the plane: corners (±1.2·t·size,
  ±1.2·t·size), z = 1.1 + 1.4t on −x, 0.2 lower on +x, UV (0,0)(1,0)(0,1)(1,1), RGBA `((1 − t)·128) << 24 | 0x808080`,
  FX 0x1b, normal blend.
- **Suggested method:** visual check (Novalis vendor approach); unit test of the corner math if extracted.

### B16. Vendor: popup quantity backdrop (+0x500)
- **Claim:** while the popup shows "How many?" (ammo entry, not full, bolts ≥ unit price) `VendorScene::popup_panel` =
  class 13 at vendor-space `spin[1]` with `backdrop[1]` rotation; the engine shows the popup canvas (its black clear,
  the 3D class 13 under the popup's text). Not in the other popup states.
- **Suggested method:** unit test on `Vendor::scene`; visual check.

### B17. Vendor: arm manipulators 0x166300
- **Claim:** at `OpenVendorMenu` four nodes join the vendor's joint-mod list on lists 0x14..0x17 (identity quat, scale 1,
  translation 0 except record 2 y = +1665·(7 − n) and record 3 y = −1665·(7 − n) when n < 8 entries); removed at
  substate 2's end. The screens are placed from the posed joints.
- **Suggested method:** unit test on `arm_manipulators(3)` = [(0x14,0),(0x15,0),(0x16,(0,6660,0)),(0x17,(0,−6660,0))];
  visual check on a vendor with few items (monitor layout).

### B18. Vendor: exit and demo decision
- **Claim:** substate 2 at t 40 (or remote): blend seq 1; `detach_arms`; a weapon bought this visit with 0x1ca4a0[item]
  ≥ 0 and not remote → `weapon_demo = {item, scene, 0x2734}`, `stop_voice`, substate 3; else exit + stop_voice + sound 6.
  The engine turns a demo request into the plain exit (sound 6) and logs it; on exit it sends `StopSpeech`.
  Table (Novalis): items 9..25 → scenes [7,0,10,−1,13,9,8,5,1,11,12,2,6,−1,−1,3,4].
- **Suggested method:** unit test on `Vendor::frame` substate 2 with `bought` set; QA (buy a weapon).

### B19. The PDA's remote vendor presentation (`OpenVendorMenu(0)`, 0x2ae1a0 / `DrawWorld_Mode5` / `VendorExit`)
- **Claim:** the hand-off `OpenVendor { vendor: None }` creates a class-11 moby at the camera position + (3.8, 0, −1.5)
  (world axes), +0x32 = 0x40, rotation (0, 0, π), its matrix built, state 3; the vendor's animation hard-cut to seq 3,
  speed 0.5; the arm manipulators attached as for a vendor; the view during the menu: the camera's position with rows
  identity (forward +x, left +y, up +z, Euler 0); no item hologram (`VendorScene::hologram` None) and no cone; substate 0
  ends at once (remote). `VendorExit`: `SetState(0)` where Ratchet stands (no teleport, no camera script), the moby
  deleted (state 0xfe), the view released.
- **Setup:** Novalis with the PDA (`RC_GIVE_ITEMS=32`), fire it (○ within 8 ticks) in movement group 0.
- **Expect:** the vendor appears in front of the camera's +x direction; screens on its monitors; on △ the vendor
  closes and the moby is gone; Ratchet keeps his position.
- **Edge cases:** the level has no class 11 → no moby, the menu still opens (2D only); the table full → same.
- **Suggested method:** QA in game; an engine-level check of the moby table after open/exit.

## 4. Shared code touched (regression risk)

- `rc-engine/src/menu_render.rs` (own lane): the pause tests now go through `in_level_trigger` (adds the group-22,
  fell-out and swap-lock rules and kinds 0/1/4); freeze frames now run without a card; every menu frame runs the
  dialogue player's step (`Help::voice_frame`) — a help line playing when the pause opens keeps counting down (the VAG
  keeps playing on the PS2 too); the front end's message table is now `all_text`.
- `rc-engine/src/hud_render.rs`: `language()` reads a runtime value first (RC_LANG unchanged otherwise).
- `rc-engine/src/menu_models.rs`: 5th canvas, `GadgetsPreview::girl`; extras on the 3D Ratchet (Persuader /
  Map-o-Matic / Bolt Grabber appear when owned: new visuals on the Gadgets/Weapons page).
- `rc-engine/src/vendor_render.rs`: the beam now turns by the camera yaw (was: facing the camera) and steps per drawn
  vendor (was: per tick); new scan quad; popup canvas.
- `rc-engine/src/interact_render.rs`: vendor screens placed from posed joints (`evaluate_chains_posed` with the moby's
  joint mods) — identical when no mods; exit stops speech.
- `rc-engine/src/saves.rs` (saves lane, additive): `press_texs`.
- `rc-game/src/menus/pause/planet_select.rs::missions_update`: now calls `dest_changed` on a planet change and resets the
  cursor when unfocused (the planet-select tests' expectations around 0x28fec8 may change: the stub counter
  "missions list 0x28fec8" is gone).
- `rc-game/src/menus/pause/map_page.rs`: `Mission` / `MapTables` gained fields (`..Default` literals unaffected).
- `rc-game/src/menus/mod.rs` `DATA_LABELS`: + 0x1c4e08 and the five scroller lists.
- `rc-game/src/frontend.rs` (saves lane, additive): `press_texs`, `set_language`, `LANGUAGES`.
- `docs/plan/gaps.md`: rows G-UI-001/002/006/019/020, G-CUT-008, new G-AUD-013.

## 5. Not ported (don't test as working)

- The weapon demo's playback (space-scene player in the vendor frame, `unknown_1530`, substate 3's world run, the
  `VendorExit(1)` / `FUN_002aecf0` ends, `FUN_002aea70`) — G-UI-006.
- The world snapshot behind the vendor; the vendor's own glow packets in mode 5 — G-UI-006.
- The 3D Ratchet's streamed per-item animations (0x1b9870 / stream queue 0x1d5ec0), the item callbacks, the head
  manipulators, the drones — G-UI-002.
- The Sound page's mixer (group 2 = music on the page; options volumes into the mixer) — G-AUD-013.
- `EnterMenuMode`'s 0x16c4ec deferral, 0x141660 = 0x24 → 0, 0x14161b, 0x1b4918 / 0x1ba2ac / 0x1ba2b0 — G-UI-002.
- Freeze consumers: the race classes' `ctx` and `race_quit` / `race_rewind`, the vehicles' riding class and
  `vehicle_quit` (0x14095f), the swap state 0x1403fc and camera lock +0x86 into the trigger, PAL video mode — G-UI-019
  notes / G-LVL-007 / G-CUT-005.
- Draw deviations (widgets in place instead of render target + blit, 1-px lines, level fog on frame mobys) — G-UI-002.
- `fun_001f6cb8`'s per-glyph accent quads (0x80..0xa7) in the missions list: drawn through `FontPrint` [L].

## 6. In-game QA spots (for the user)

- Missions page: `RC_LEVEL=1`, Select (map) → ✕ → Up/Down: the list, check boxes, the description label and the
  `mission_ss` picture change; L1/R1 change planet (cursor sound, list recomputed).
- Ship planet select: `RC_LEVEL=1`, take off from the ship → the planet list: the planet picture follows the cursor;
  ✕ on a planet → the confirm page: the gold bolt spinning and "Found N of M" right-aligned.
- Gadgets page: `RC_GIVE_ITEMS=33,34,35` (Map-o-Matic, Bolt Grabber, Persuader) → Start → Gadgets: the three on
  Ratchet's back/joints.
- Giant Clank quit: `RC_LEVEL=15`, as Giant Clank press Start → "Quit?" dialog (pulsing frame); △ → leaves the body,
  back at the pad; ✕ → continues.
- Language: `RC_FRONTEND=1`, Options → Language → French: menu text and PRESS START switch at once; New Game: the level
  text in French.
- End page: `RC_LEVEL=18` beat the boss (or `EnterMenuMode(0x21)`): the Helpdesk girl in the left panel, a voice line
  after ~2 s, her talk animation, 4 s pause, the second line (first ending) / the third line (completes ≠ 0).
- Vendor: `RC_LEVEL=1 RC_HERO_AT=<Novalis vendor>`: approach → the beam turns with the camera, a rising scan square
  fading out; open, choose ammo with room → "How many?" shows the spinning backdrop; a vendor with ≤ 7 entries lays
  its monitor arms out; buy a weapon → log "VendorStartWeaponDemo …" then the normal exit.
- PDA: `RC_LEVEL=1 RC_GIVE_ITEMS=32`, select the PDA and press ○: the vendor appears ahead (seq 3, no hologram cone);
  △ closes it and it disappears.

## 7. Results (the test expert fills this in)
