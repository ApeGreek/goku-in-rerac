---
status: open
job: saves-frontend
date: 2026-10-01
commit: 7bf9dc6
areas: [ui, formats, classes]
---

# The memory card natively (card driver, monitor, saves, loads, previews), the card dialog and save notice (mode 4), the Save / Load / New Game / challenge pages, challenge mode, the ending save, and the front end (card check, logos, title, attract loop, main menu)

## 1. Summary

Ported from the boot ELF and level01 (every function boot-hash-identical on the 19 overlays):

- **`rc_game::memcard`**: the card driver `memcard_Update` (boot 0x2093d8 = L01 0x25f6a8) state by state and substate by
  substate (states 0–4, 7–10, 0xd–0x10, 0x13–0x17), the card monitor `fun_00208840` (25 statuses, handlers L01
  0x25eca0..0x25f3e0, created in Ghidra as `mc_status_00..24`), `memcard_Save(force, pretend)` (L01 0x261448), the
  whole saves of the menus (`fun_002269c0` / `fun_00226a70` / `fun_00226b08`), the slot previews
  (`memcard_RestoreInfo`), the boot card check `fun_00209168`. The card is a folder ([`CardFs`]) holding the files a
  real card holds; libmc calls complete at once and their results reach the machine at the next frame's sync step.
- **`GameState`**: `capture_landmarks` (`fun_00208770`), `challenge_reset` (`fun_00226b08`'s state part).
- **`rc_game::menus::freeze`**: mode 4 kinds 3 (the card dialog, text and buttons per card status) and 5 (the save
  notice), its trigger `save_notice_due` (L00 0x297f78).
- **`rc_game::menus::pause::saves`**: the widgets of the Save page 0x1b5b48 (`SavingDataMenu` 0x296990), the Load pages
  0x1b5bd0 / 0x1b9180 (`LoadingDataMenu` 0x296ce0), the New Game / challenge pages 0x1b9208 / 0x1b9440
  (`SavingDataMenu2` 0x296fc0, flag 0x2000), the confirm pages 0x1b6af8 / 0x1b93b8 / 0x1b9518 (0x2954c0 / 0x295558 /
  0x295450), the slot list 0x2973e8, the slot label 0x2967a0, the image widget's flag 0x100 (0x2937d0 / 0x293d50), the
  front end's Options list enter 0x28dbb8, the time warp's state (`process_global_state_flags` ✕). `PageMenu`: list
  actions 4 / 5 (card ready → page, else flag 2 / 4 + the dialog), action 9 (language), the changed-card hold, the
  page tree now loads the front-end tree (kind 0x2d), the card pages and the confirm pages (also relocated on other
  levels: `page::ROOTS`, `correlate` follows actions 4 / 5).
- **`rc_game::frontend`** + **`rc_formats::frontend`**: the boot flow (`startlevel` 0x1e9658, `transition_do_transition`
  0x1eb798, `fun_001eb0a8`, `transition_default_draw` 0x1eb410): card check with its warning pictures, logos movie
  `mpegs[0]`, the loading still, the title (logo 256×128 from `unknown_14e8.bin`, PRESS START = FX 4 + language − 1),
  the attract loop (1500 vsyncs, `mpegs[80 + i]`), the main menu (kind 0x2d), the hand-over to a level.
- **Unit U570 class 1750** (level 18, 1 instance): the ending buffer (`MakeWholeSave` once) for the time warp.
- **Engine** (`rc-engine` `saves`, `menu_render`, `scene_render`): the card per frame in every mode, `EngineRequest::Save`
  → `memcard_save`, mode 4 in the engine, the front end (`RC_FRONTEND=1`), the end page's choice.

Plug-and-play: a class that saves calls `cinematic::save(w)` (→ `memcard_Save(0, −1)`); the travel lane's
`ShipTravelTo` calls `rc-engine` `saves::memcard_save(play, gs, false, dest)`; any page with the disc's slot widgets
works by its records (no per-page code).

## 2. Where it lives

- `crates/rc-game/src/memcard.rs` — `CardFs`, `MemCard::{update, monitor, memcard_save, save_whole, new_game_save,
  challenge_save, load}`, `Preview::from_section`, `bcd_clock`, `save_root`, `capture`. Coverage tables in the module doc.
- `crates/rc-game/src/game_state.rs` — `capture_landmarks`, `challenge_reset`.
- `crates/rc-game/src/menus/freeze.rs` — `Freeze::{init, update, draw}`, `save_notice_due`.
- `crates/rc-game/src/menus/pause/saves.rs` — `save_update`, `load_update`, `new_game_update`, `confirm_*`, `slots_draw`,
  `slot_info_draw`, `busy_draw`, `front_options_enter`, `time_warp`, `reset_game`.
- `crates/rc-game/src/menus/pause.rs` — dispatch, `Addrs::saves_pages`, `MenuOut::{story, language}`, actions 4 / 5 / 9.
- `crates/rc-game/src/menus/pause/pages.rs` — `image_update` flag 0x100.
- `crates/rc-game/src/frontend.rs` — `FrontEnd::{new, card_frame, frame, menu_closed, exit, draw, draw_fade}`.
- `crates/rc-formats/src/frontend.rs` — `TitleWad::parse` (logo, FX textures), `BootPictures::{parse, picture}`.
- `crates/rc-game/src/moby_update/classes/units/ending_save.rs` — U570.
- `crates/rc-engine/src/saves.rs`, `menu_render.rs` (`saves_in/out`, `open_freeze`, `freeze_frame`, `end_choice`,
  `front_end_frame`), `scene_render.rs` (`R::Save`, `R::EndingSave`).
- Docs: docs/plan/progression.md `## saves`.

## 3. Behaviours to verify

### B1. CRC and section format unchanged; previews read the fixed offsets
- **Claim:** `Preview::from_section(global section bytes)` = level at +0x10, bolts +0x1c, completes +0x28, elapsed +0x34,
  clock +0x40..+0x48, `bad` = the section's CRC check failed (`memcard_RestoreInfo` 0x20ae60).
- **Setup:** the disc template (`save_game.bin` +0x8534); a GameState with level 3, bolts 1234, completes 2, elapsed
  216000·2 + 3600·5, clock [0,0x10,0x20,0x13,0,0x25,0x12,0x26] encoded by `global_section().encode()`.
- **Expect:** template preview level −1; the custom one exactly those values, `bad` false; flip one data byte → `bad` true.
- **Method:** unit test.

### B2. Card driver: creating the save folder (state 9/10)
- **Claim:** request 9 (via the monitor or `MemCard::request(9)`): free < 0x15e → error 7, state 0; else mkdir (0 or
  −4 ok), then icon.sys (0x3c4 bytes = lump), static.ico (lump size), save0..save4 (each `global + 20·level + 8` =
  0xea08 bytes = the template), then `BASCUS-97199RATCHET` (0x3c04 bytes, zeros; PAL 0x3c00); slot −1, errors 0. One
  libmc call per frame: every call frame sets busy, the next frame syncs the result.
- **Setup:** `MemCard::new(CardFs::new(Some(tmpdir)), "/BASCUS-97199RATCHET", Some(lump))`, `card.free` from `update`
  states 0/1 (8000), a zeroed GameState with the boot tables.
- **Trigger:** `request(9)`, then call `update` repeatedly until `state == 0` again (count the frames).
- **Expect:** the 8 files with exact sizes and bytes; `card.slot == -1`; `error == 0`; the frame count is
  deterministic (each sub-state's frame pattern as in the coverage table). Edge: free 100 → error 7, no file written;
  an existing folder → mkdir −4 accepted.
- **Method:** unit test with a temp dir.

### B3. Card driver: the check (state 7/8) and the previews (0x15..0x17)
- **Claim:** chdir ok → slot −1 (if < 0); open save0, read 8, header vs `GetDataSize` (errors +1 per mismatch), close →
  0x15 → for slots 0..4 open / read 8 / read global (header[0] bytes) / `RestoreInfo` → previews; folder missing → slot
  −2, no error (chdir −4 is silent), state 0.
- **Setup:** the folder of B2, then write a whole save of a custom state into `save2.bin`.
- **Expect:** previews 0, 1, 3, 4 level −1; preview 2 = the custom values; `card.errors == 0`; header = [0x1530, 0xaa4].
  Edge: delete `save3.bin` → the preview loop stops with error 0x16 and state 0.
- **Method:** unit test.

### B4. Card driver: load (state 0xd/0xe)
- **Claim:** slot < 0 → error 0x13 (error card unchanged); else open, header, global → `RestoreData`, 20 level sections →
  `RestoreData` (errors summed into `card.errors`); a size > 0x1800 / 0x1000 fails (error 0x1b; the game traps).
- **Setup:** B3's folder; a fresh template GameState.
- **Trigger:** `card.load(2)`; update until idle.
- **Expect:** the GameState equals the saved one chunk for chunk; `card.errors == 0`. Edge: corrupt one level section's
  CRC → that section not restored, errors 1.
- **Method:** unit test.

### B5. memcard_Save (incremental) and its gating
- **Claim:** `memcard_save(gs, force, pretend, cap)`: always the capture (clock into chunk 4, landmarks, the current
  level's map mask into chunk 3002). No card (`save_card == −1`) or slot < 0 → returns `!force`, nothing queued. Else
  `checked |= force`; with checked: force 0 raises flag 0x200; when idle: preview of the slot = (bolts, level after
  pretend, elapsed, clock, completes), `PrepData` of the global (with pretend level and `visited[pretend]` raised to 1)
  and of the **current** level, both put back, request 0xf; returns `req == 0xf`. State 0x10 writes only the global
  section and that level's section into the existing file (seek 8, write, seek level·0xaa4 from there, write).
- **Setup:** B2 folder, slot 1 whole-saved with state A; then state B (bolts changed, level 3).
- **Trigger:** `checked = false` → `memcard_save(force=false)` → expect false, nothing written; `checked = true` →
  `memcard_save(false, -1)` → true, update until idle.
- **Expect:** `save1.bin` = A's bytes except the global section and level 3's section (B's), CRCs valid; flag 0x200 set
  (then the monitor in status 1 → 22 → back to 1). Pretend: `memcard_save(true, 7)` writes level 7 in the global
  chunk 0 and `visited[7] = 1` there, while the GameState keeps level 3 and its `visited[7]`.
- **Edge:** busy card (state ≥ 3) → nothing queued, returns false, flag 0x200 still raised (force 0).
- **Method:** unit test.

### B6. Card monitor statuses
- **Claim:** status 0 → 3 → 4 → (info 0 / −1) 9; 9 + flags & 6 → 10 → (request 7) 11 → slot −2: free < 350 → 19 else 12;
  slot ≥ −1 → 16; 12 + flag 2 → 13; 13 + flag 0x10 → 14 → (request 9) 15 → (no error: request 7, slot 0) 16; 13 + flag
  0x20 → 24 (front end) / 12; 16 → 1 once `checked`; 1/16: flag 0x80 → 21, 0x100 → 20 (with 0x40); 1 + flag 0x200 → 22
  → (idle, no error) 1 / (error) `take_freeze_request()` true and 21; 17/18/20/21 → 3 once flag 0x40 is cleared.
  `status_frames` resets on a change.
- **Method:** unit test driving `monitor()` + `update()` with a temp-dir card and with `MemCard::absent()` (no card:
  stays in status 4).

### B7. The card dialog (mode 4 kind 3) answers
- **Claim (per status, `Freeze::update`):** 2: after the fade (ticks(30)) ✕ clears flag 1 and returns to the previous
  mode; 4 in a level: △ → flag 0x20, flags &= ~6, previous mode; 4 in the front end with a save asked: ✕ → new game
  without a card (`FreezeOut::new_game`), △ → back; 6 / 13: ○ → flag 8 / 0x10, △ → flags &= ~6 | 0x20 (and back in a
  level); 9 in the front end with no request → back; 3 / 5 / 12 (no save asked): △ → flags &= ~6, back; 16 → target =
  the dialog's page, back; 17 / 18 / 20 / 21: ✕ → flag 0x40 toggled, back; 19: front end + save: ✕ new game / △ back;
  23 / 24: ✕ new game, △ flag 0x20 + back. Leaving to a mode other than 3 / 4 resumes the sounds.
- **Draw:** text per status (freeze.rs doc table), window x 0x60..0x1a0 centred vertically around the measured text
  (+0x28), `DrawUIFrame` alpha (1 − fade/30)·80, text colour tween 0xffa888 → 0x80ffa888, buttons at x 0xca / 0x135
  (two) or 0x100 (one), y = bottom − 0x14, colour tween 0x20ffff → 0x8020ffff over the label fade.
- **Method:** unit tests on `Freeze::update` / `draw` with a MemCard set to each status.

### B8. The save notice (mode 4 kind 5)
- **Claim:** due when checked, global flag 0x10 clear, > 7 frames in mode 0, completes 0, not dying, HP ≠ 0, tick >
  ticks(30), level ≠ 0; init logs 20011 into the help log; after ticks(120) ✕ → mode 0, and on level 1
  `memcard_Save(0, −1)`; the button label fades after ticks(90). Draw: frame (0x50, 0x154, 0x60, 0x1a0), the message's
  first part from y 84, the rest bottom-aligned at 0x136, "✕ Continue" at (0x100, 0x140), icon 0x755d frame 0 (64×64 at
  x 0xe0) and frame 1 turning by −2π/55 per vsync.
- **Method:** unit tests; QA in game (see §6).

### B9. Save page (Options → Save)
- **Claim:** the card not ready → the dialog (flag 2); ready → the slot list; Up/Down 0..4 (shared 0x15ee34, sound 1 on a
  change); ✕ on an empty slot saves at once (`fun_002269c0`: capture + whole save, checked on), on a used slot opens the
  confirm page 0x1b6af8 ("A save game already exists … overwrite?" ○ Yes / △ No); ○ returns with page +0x84 = 1 and the
  save runs; the busy text "Saving Data" while the card works; on success the slot's preview updates; on a failure flag
  0x80 and the dialog (status 21 "Save failed!").
- **Setup:** Novalis (`RC_LEVEL=1`) with `RC_SAVE_DIR=<tmp>`.
- **Method:** level-harness test driving `PageMenu::tick` with scripted pads + QA in game.

### B10. Load page and Load Game
- **Claim:** ✕ on a used slot: sound 0, request 0xd, "Loading Data"; done: checked on, `MenuOut::sound_settings`,
  `level_exit = saved level`, `story = false` (0x13e05a = 0). Empty slots ignore ✕. Load uses 0x13cb04 (unmirrored).
- **Method:** harness test + QA.

### B11. New Game page (front end) and "Continue without saving"
- **Claim:** ✕ empty slot → `fun_00226a70` (template reset, clock, whole save, preview level 0, checked) then
  `level_exit = 0`, `story = true`; used slot → confirm 0x1b93b8 (kind 1); ○ (no ✕) → checked off, flags &= ~6, the
  template reset, `level_exit = 0`, `story = true`, nothing written.
- **Method:** harness test.

### B12. Challenge mode
- **Claim (`challenge_reset`):** after the reset the state keeps gold weapons, the owned flags of the list 0x1ba120 only,
  all ammo, all 20 levels' gold bolts, the quick select (unowned → 0), the vendor stock, skill points, bolts; flags
  [4] / [5] and max HP 5 / 8; HelpDesk voice / text 0; completes + 1; level 0. `challenge_save(slot)` whole-saves the
  new state; with slot −1 nothing is written. The end page ○ with the card ready opens 0x1b9440 (the challenge slot
  list, confirm 0x1b9518, kind 2).
- **Method:** unit test on `challenge_reset` with a hand-made end-of-game state; harness for the page.

### B13. Time warp and the ending buffer
- **Claim:** class 1750's first update queues `EngineRequest::EndingSave` every tick; the engine keeps the first whole
  save only. ✕ on the end page: `memcard_RestoreGame(buffer)` keeping level 18's gold bolts and skill point 0x1d, game
  beaten 1, `memcard_Save(0, −1)`, checkpoint cleared, `FadeToBlack(16)`, the death flag (reload).
- **Method:** unit test on `saves::time_warp` (rc-game) with two states; QA on level 18.

### B14. Slot list and slot label draws
- **Claim:** 5 rows from y 4 step 0x4b; the cursor row framed 0x8020ffff (0, y−4, w, y+0x34) then navy (3, y−1, w−3,
  y+0x31), every row 0x80303030; "EMPTY" centred at y + 0x10 (small font); used rows: time "%02d:%02d" (hours =
  elapsed / 216000 ≤ 99), completes (≤ 99) when ≠ 0 at x 0x60 with icon 0xe99e·4 at 0x4e, bolts with commas (≤
  9,999,999) at y + 0x10 after icon 0x754f·15, the date `%02x/%02x/%02x` (month, day, year) at y + 0x20 after icon
  0xe99e·2; the cursor row yellow, others light blue. The label: location / planet names (0x1c22c0 + 12·level, +4) at
  y 4 / 0x14 or "EMPTY" at h/2 − 8. The picture: `planets[level]`.
- **Method:** draw-list unit test (`MenuDraw` output) + visual QA.

### B15. Front end flow
- **Claim:** `FrontEnd::new(rand, check)`: check ≠ 0 → card warning until 0 or (> 10 frames and a button), then
  `FadeToBlack(10)`; logos movie requested once, then `FadeToBlack(ticks(18))`, the still ≥ ticks(180) frames, then the
  title with fade 1.0 −0.0625/frame; logo alpha +1 per frame after ticks(60) to 0x40; PRESS START alpha
  `(int)(cos(((c − 120) % 60)·0.10471976 − π)·32) + 0x60` after ticks(120); Start/✕ → `open_menu` (mode 3); mode 3:
  counter = 60, alphas −0x10/frame; idle ticks(1500) in mode 0 → attract `rand % 4`, cycling, fade 1.0, alphas and
  counter 0; any other mode resets the idle count; the menu closing returns to mode 0.
- **Method:** unit tests of `FrontEnd` (pure); QA with `RC_FRONTEND=1`.

### B16. Front-end data
- **Claim:** `TitleWad::parse(unknown_14e8.bin)`: logo = 256×128 raw RGBA at `hdr[1] + hdr[0x21]` (0x42a340); FX entry
  count 40 (hdr[0x16]) at hdr[0x17]; entries 4..8 are 256×128 sharing palette 101376; `press_start(0)` = FX 4,
  `press_start(2)` = FX 5. `BootPictures`: still / warnings decode to 512×416×4 = 851968 bytes.
- **Method:** golden test on the extracted data.

### B17. Engine: `EngineRequest::Save` now writes
- **Claim:** a class's `memcard_Save(0, −1)` (e.g. the Novalis gold bolt 1134, the infobot 750) runs
  `saves::memcard_save` with the capture: chunk 4 = UTC BCD now, landmarks of the level's talk slots, chunk 3002 = the
  map mask; it writes the card only when auto-save is on (after a save / load / new game into a slot).
- **Method:** QA (log line `saves: memcard_Save(0, -1): queued`), `RC_SAVE_TRACE=1` for the card states.

## 4. Shared code touched (regression risk)

- `menus/pause.rs`: `PageMenu` gains `saves`; `Addrs` gains `saves_pages`; `page::ROOTS` now 20 entries (the card pages)
  — `Overlay::relocated` maps more records on levels ≠ 1; `correlate` follows actions 4 / 5; `PageMenu::load` loads
  kind 0x2d's tree, the card pages, the confirm pages and the front-end Options pages (more pages/widgets than before:
  tests counting pages may change). The tick holds the menu in the card dialog while `card.flags & 1`; list actions 4 /
  5 no longer always emit `freeze` (only when the card is not ready; ready → straight to the page); action 9 sets `lang`.
  `call_draw` routes the image widget with flag 0x100 and no picture to `saves::busy_draw`.
- `menus/pause/pages.rs`: `image_update` flag 0x100 is now the slot picture (was: state −1 always).
- `menus/pause/tests.rs`: the `PageMenu` literal gains `saves: Default::default()`.
- `menus/mod.rs`: `DATA_LABELS` + 3 labels (0x1b8dc0, 0x1b8d90, 0x1ba120).
- `game_state.rs`: two new methods (no change to existing ones).
- `cinematic.rs`: `EngineRequest::EndingSave`, `ending_save()`; `save()` unchanged.
- `rc-engine/menu_render.rs`: `setup` (card + front end), `menu_frame` (front-end branch, card swap around the menu's tick
  and draw, freeze opening, Freeze arm, save notice trigger, status-22 dialog, end choice), `menu_layer` (active under a
  menu-born dialog; the 2D camera's clear colour now set every frame: opaque black in the front end), `build_prims`
  (HUD replaced under a menu-born dialog). `scene_render.rs`: `R::Save` writes, `R::EndingSave`.
- Behaviour change: with a card folder present, saves now write files under the user's config folder (deterministic
  runs: no card unless `RC_SAVE_DIR`).

## 5. Not ported (don't test as working)

- G-SAV-011: native latency (no multi-frame card waits), the reservation file's RAM content, the JST → local clock fix
  (UTC), unrequested states 5/6, 0xb/0xc, 0x11/0x12, PCSX2 `.ps2` import/export.
- G-SAV-012: the title world (the space flight, its mobys, particles, music) behind the front end — black instead.
- G-UI-019: freeze kinds 0, 1, 2, 4, 6.
- G-UI-020: the runtime language switch's text / PRESS START reload.
- G-SAV-003 (left): the checkpoint system's records (`0x29b0a0`, 0x1baa50 / 0x1bb6b0 copies) — not card chunks.
- The level change itself after New Game / Load (travel lane: `EngineRequest::LeaveLevel`).

## 6. In-game QA spots (for the user)

- `RC_FRONTEND=1 RC_SAVE_DIR=/tmp/rcsave cargo dev`: the logos movie, the still, the title (logo fading in to half
  alpha top right, PRESS START pulsing bottom left); wait 25 s → an attract movie, then the title again. Start → the
  main menu (New Game / Load Game / Options; the panel sounds). New Game on an empty folder → "Checking/Reading" →
  "No Ratchet and Clank save data … create?" → ○ Yes → "Creating Save Game File" → the 5 slots "EMPTY" → ✕ → "Saving
  Data", then the level exit (logged; the travel lane loads Veldin). Check `/tmp/rcsave/BASCUS-97199RATCHET/` holds
  icon.sys (964 B), static.ico (33112 B), save0..4.bin (59912 B each), BASCUS-97199RATCHET (15364 B).
- Same, New Game → ○ "Continue without saving" → level exit 0, nothing written.
- `RC_LEVEL=1 RC_SAVE_DIR=/tmp/rcsave cargo dev`, Start → Options → Save: the dialog then the slot list with previews
  (time, bolts, date; the planet picture and names at the side); save into a used slot → the overwrite question.
  After saving, walk to the gold bolt on Novalis: the pickup's `memcard_Save` now writes (log `saves: memcard_Save(0,
  -1): queued`; `save<n>.bin` modification time changes).
- After a first save, leave Novalis to another planet (travel lane): the save notice ("When this icon appears …") with
  the spinning card icon appears once on the next level.
- Options → Load on Novalis: pick the slot → "Loading Data" → level exit to the saved level.
- Level 18 (`RC_LEVEL=18`): the ending buffer is taken on the first tick class 1750 updates (log `saves: the ending
  buffer`); after the credits the end page's ✕ "Timewarp" restores it and reloads, ○ opens the challenge slots.

## 7. Results (the test expert fills this in)
