# Level generalisation audit (2026-09-26)

A read-only audit of the Rust crates (`rc-formats`, `rc-game`, `rc-engine`, `rc-trace`): every place that
assumes Novalis (level 01), as a checklist for a later generalisation pass. Nothing was changed, built or run.
The evidence comes from grep, from byte comparisons of the level overlays (then the C++ `overlay.elf` copies, now `extracted/levels/NN/overlay.bin`), from the per-level
`lvl.vtbl` class tables, and from `tools/ghidra/names/clusters.tsv`.

## Status after the Gemlik pass (2026-09-28)

The generalisation pass ran level 13 (Gemlik Base) headless and in the engine, fixed what was Novalis-only in
general code, and added guards. Tooling items (T1–T4, the `overlay_diff.py` re-run) were out of scope.

**The two mechanisms** (`crates/rc-formats/src/level_overlay.rs`, X1 + X2):
- `LevelOverlay`: the seven sections (the same order on all 19), `lvl.vtbl` (section 3), code reads, function extents
  (next known start: `jal` targets, class-table updates, `lui`/`%lo` code addresses, a frame set-up after `jr ra`).
- `Relocation::new(level 01, level N)`: **same code** = equal masked words over the reference extent. The mask hides
  `j`/`jal` targets, `lui` of main-RAM addresses (0x0010..0x003f only, so float `lui`s are compared), the `%lo` that
  completes such a `lui` (tracked per register, through `move`, and into forward branch targets: a `lui` in a delay
  slot) and every `$gp`-relative immediate (`$gp` = 0x166c00 on every level). This fixes the cluster hash's `%lo`
  misses (the bomb 0x2c3300, the splash 0x2ff810, the explosion light 0x2f3748, the talking NPC 0x2ff118 now match
  their copies). `func(a)` finds the level's copy; `data(a)` finds a data address through a reference function
  that forms it (`lui`/`%lo` or `$gp`), reading the same instruction in the copy (fallback: the code up to 16 words
  past the reference, found once). Level 01 against itself is the identity. Checked against `clusters.tsv`
  (levels 0/5/13: ~1400–1570 functions agree, 0–1 disagree, the rest are extent misses).

| # | Status | Where / how |
|---|---|---|
| X1, X2 | **fixed (product)** | `rc_formats::level_overlay` (above); `overlay_diff.py` not re-run |
| C1 | **fixed** | `rc_game::moby_update::classes::LevelPorts`: a class runs a port when its `lvl.vtbl` entry is the same code as the port's reference function (level 01; level 03 for the swing target). Class-number lists remain the fallback for classes not in the table. Engine: `gameplay::level_ports()` (class table, joint lists, `moby_spawn`). Novalis: identical for every placed class (only the unplaced 731 gains `MissionNpcUpdate`, as the game) |
| C1 found | fixed by C1 | `FxGroupUpdate` runs other body-piece classes on **every** level (Gemlik 1733–1735, 1801–1804); grass on 00/02/08; `PathPlatformUpdate` 1141 on 10; the ember update on 10 (1806/1807) and 15 (200…210) |
| C2 | open (N/A) | the flyer update exists only on 01; the shared path driver is called from other levels' own updates |
| C3 | **fixed** | the emitter external comes from the table (`LevelPorts::external`; only 01 has it); the ripple manager 751 is now a class port (W3) |
| C4 | open | `moby_spawn::initial_state` is still keyed by class number (rules transfer where the vtbl says so; not rechecked) |
| C5 | open | `MISSION_NPC_CLASSES` [730, 790] (01-only code) |
| C6 | ok | the engine and the smoke test set `Services::level` |
| C7 | **fixed** | the Novalis scheduler test and the debris test read `LevelOverlay::vtbl` |
| P1 | **fixed** | emitter owners = instances whose class runs 0x2bd100 (still 01 only); the `level == 1` cull in `type06` is the game's own branch (kept) |
| M1, M2 | **fixed** | `menus::Overlay::relocated(level, level 01)`: the page roots through `Relocation::data`, then the page tree walked in step with 01's (every page, widget, item list and callback paired). `PageMenu::addrs` holds the level's page / wiring / Port-Options records; widgets dispatch on the callbacks' level-01 labels. All 19 levels load the same 26 pages / 109 widgets and install the Port Options page |
| M3, M7 | **fixed** | `MenuConsts`, the frame light and the corner lists go through `Overlay::at` (0x161fe0 → 0x161e08 on 13; the old read gave joint list 1045220557 and no frame mobys) |
| M4 | **fixed** | names `at(0x1c22c0)`; points `at(0x1c23a8) + 0x10` (the code forms 0x1c23a8, the map reads from level 1) |
| M5, M6 | **fixed** | quick-select gp block `at(0x15f718)` (+0x10 on 05 / 16, from the `$gp` operands), page table at +0x20, d-pad `at(0x17e098)` (0x17df18 on 13) |
| W1–W4 | **fixed** | see "Water on every level" below (the earlier "Gemlik has no water" was wrong: 13 has a ripple module and 22 patches) |
| K1 | open (N/A on 13) | 13 uses the generic star dispatch (120 twinkle + 8 moving) |
| H1 | partly open | the level-13 branches (below) are in unported code; the footstep branch belongs to the hero agent |
| H2 | open | not diffed |
| G1 | open | Gemlik's arrival is scene 0 from its own director class 1353 (`0x30b628`, Gemlik-only: content). `RC_SCENE=+0` forces scene 0 (it plays: 868 ticks, actors 0 / 10 / 1289, speech, subtitles) |
| A1 | ok as is | `entry` pauses the music on 13; the landing sequence (not ported) unpauses it: a direct boot plays unpaused, the post-landing state |
| A2 | open | reverb (another agent) |
| T1–T4 | open | tooling |

### Water on every level (2026-09-28)

**Survey** (by code identity against level 01, `rc-game/tests/water_levels.rs::water_inventory_all_levels`):

| level | ripple module | manager (update, reference code) | patches | other water |
|---|---|---|---|---|
| 01 Novalis | yes | 751, level01 `0x2fd0e8` (7 camera-zone cuboids) | 21 | strips 676 / 678 / 761 / 1225 (the only strip module) |
| 05 Rilgar | yes | 831, level05 `0x30ca80` (one moby per patch; group commands raise / lower; the sewer flood 0x33e button, 6 timed stages) | 48 | 982 `0x318a68`: the flat water plane `0x1612dc..ec` (59.5 ± 0.25) and the dive lock while Ratchet is in its cuboid; canal / sea surface-0 faces at 59.44 |
| 07 Umbris | yes | 902, 941 … 971 (20 classes), level07 `0x30c508` (one moby per patch; group commands; a cuboid trigger then two timed rises to 54) | 36 | sinking liquid (surface 0xb) |
| 11 Pokitaru | yes | 1158, level11 `0x30e088` (patches built from the mobys; the tide 132.36 ↔ 137) | 20 | the sea (surface-0 faces at 223) |
| 12 Hoven | yes | 19, level12 `0x2bf140` (patches built from the mobys; +5 per command; Ratchet's water level near it; a camera cuboid gate) | 17 | deadly liquid (surface 0xd) |
| 13 Gemlik | yes | 1263 / 1393, level13 `0x309e88` (the level swings between two heights) | 22 | 1393's collision is surface 0xb |
| 02, 10, 15, 18 | no | — | — | sinking / deadly liquids (0x3, 0xb, 0xd, 0xe): the hero's surface rules, unchanged |
| 08, 15, 17 | no | — | — | plain surface-0 water faces (the hit z is the level) |

One engine module on every level that has it (`RipplePatchesInit` 0x2b7a48 … `0x2b96c0` and the VU0 kernels, the same
object code relinked); the managers are level code, and all but 751 are copies of one template: each placed moby owns
patch `idx`, writes its z into the patch (+0x08, the water level) and the patch's sub-block mask from a
`FastBSphereCheck` of its bounding sphere (drawn and simulated only in view), and makes the random drops; the moby of
patch 0 runs the module's init, the clock and the draw callback. Their collision (surface-0 faces at the moby's z) is
the water Ratchet swims in, so moving water moves the swimming level. No lava / sludge / quicksand mechanism of its own:
the other liquids are collision surfaces (hero `surface.rs`, per-level rules), already general. **Correction (2026-09-28):** the *visible* seas, lava and sludge are draw-callback surfaces of their own (the liquid grid module on 03 / 05 / 07 / 08 / 09 / 14, the ocean 1111 on 11 / 16, the Hoven liquid 1901 on 12): world_animation.md §3.3, `rc_game::water::sea`.

| # | fix |
|---|---|
| W1 | `rc_formats::water::strip_tables(target, rel)`: the strip classes are the `lvl.vtbl` entries that run the level's copy of a strip update (676 `0x2f6128`, 678 `0x2f6180`, 761 `0x2feb58`, 1225 `0x309c98`); the table and count are read from the `lui`/`%lo` and `addiu a0` before the draw callback's `jal` to the copy of `0x2b96e0`. Only 01 has the module (verified: no other overlay has a copy) |
| W2 | `RippleModuleAddrs::locate` (0x1cad00 consts, 0x1cafe0 UVs, 0x1cb1b0 selectors, the three order tables) and `Ripple751Addrs::locate` (patches, zones, masks, drips, mist) through `Relocation::data`; the patch managers' tables (patches, masks, flood timers / speeds, `$gp` globals) through the same relocation from their own reference levels (`rc_game::water::managers::PORTS`). Found on 05: 0x1d6880 / 0x20b380, 07: 0x1dce40 / 0x204680 (module 0x1ca900), 11: 0x1da880 (module 0x1cb280), 12: 0x1cbf40, 13: 0x1d9bc0 / 0x1f1e20 (module 0x1cab80) |
| W3 | the managers are class ports, `ClassUpdate::Water(i)` (`rc_game::water::managers`, 751 included): `LevelPorts` finds them by code identity; the ripple module lives in the moby system (`Services::water`, `rc_game::water::world::WaterWorld`), registered for drawing with `Callback::RipplePatches`; `water_render` draws whatever module the level has. The engine's 751 external is gone |
| W4 | `WaterWorld::water_height` = `SetWaterLevel` 0x26ed38: the active patch (`RippleHeightQuery`), then the flat plane (982 on 05), else the caller's hit z (0x1742e0 is the collision output buffer, not a table: the port already passed the hit). The hero probe, the underwater test, the bomb's water entry and the hero's ripples all use it; the managers' stores into the hero block (0x13f640 on 05 / 12, the dive lock 0x13f52e on 05) go through `HeroFields` |

**Guards** (`rc-game/tests/water_levels.rs`): the inventory on 19 levels (every table resolves, each port only on its
level); the managers run on 01/05/07/11/12/13 (a step every 9 ticks, the draw registered, every patch's level = its
moby's z); 751's port = the ripple module alone on the same stream; swim and wade on **05** (canal / sea) and **11**
(sea, and a tide pool on the 1158 patches: the level read is the patch's rippled surface). Novalis: `novalis_hero_digest`
and the five water captures byte-identical to before.

**Not ported**: the manager 07 / 13 `.lit` mask overrides are ported but never set by their levels' code; 13's
pause while Ratchet holds a class-0x45 item in state 0x32 (`0x140940`, the hand item moby, not modelled); the classes
that send the raise / lower commands (buttons, 0x33e on 05) are other classes. Drips (787) as before.

**Level-13 branches in engine code** (L01 decomp, `0x15ed84 == 0xd`): `InitLevelRenderGlobals` adopts the placed
ship 533 (`*0x160550`) instead of creating one when ship index 2 and runtime flag `0x160540` (set by the Gemlik
story) — not reachable in a direct boot (ship 1); `PauseAllSounds` sets `0x1ba2a4` (marks grid icons in the
Weapons / Gadgets pages, which are stubs); `GameStateUpdate` creates class 0x509 on the landing (landing not ported);
`FUN_002a2360` sets the landing position. All open, listed for the landing / pause-grid ports.

**Guards.** `rc-formats/tests/level_overlay_disc.rs` (sections, vtbl, relocation on 13), `rc-game/tests/level_ports.rs`
(Novalis registry unchanged; per-level ports and externals), `rc-game/tests/gemlik_generalisation.rs` (menus on 13 and
all levels, body pieces, `gemlik_content_gap` report), `rc-game/tests/all_levels_smoke.rs` (19 levels: load, spawn,
load pass, N ticks of the full tick with a scripted pad; Gemlik 3,000 ticks).

## Common classes pass (2026-09-28)

The classes the Gemlik pass listed as unported on many levels, each read from the decomp (addresses per class below)
and ported once, registered in `classes::LevelPorts` (the level's `lvl.vtbl` entry against the reference function,
`Relocation`); every level listed runs the **same code** (one group per class, masked-word compare), and every `$gp`
constant those functions load has the same value on each level copy (checked per copy). Standard `f32`.

| class | what | update (reference) | levels | port |
|---|---|---|---|---|
| 605 | buried bolt cache: the Metal Detector's target (hidden; offers itself as the nearest cache 0x141390..98; the dug count per cache is a nibble of save chunk 3008 `0x14bf10 + level·16`); the HUD alert frame `HudBoltAlertShow` 0x227d90 | L01 0x2f2eb8 | 1–18 (358) | `buried_bolts` |
| 832 | Visibomb range limiter: out of its 20 areas the missile (class 172, hero state 0x1d) gets static (draw 0x302438) and, after 90 ticks, its flight ends (0x2cb788) | L01 0x302648 | 1–15, 18 (16) | `rc_range` |
| 604 | the Sonic Summoner's "house": an 8-state open / close anim on the mouse's commands (+0xbc), turns to face Ratchet | L01 0x2f2b68 | 1–6, 8, 11, 12, 14 (10) | `mouse` |
| 1818 | the Sonic Summoner's "mouse" (the object's own strings): with head item 5 worn (`0x1404a8` is item slot 2's id, not the hand) and Ratchet in its cuboid with a clear line from the house, it is summoned (Ratchet held in state 0x1f), runs out, flies 1.1 left / 2 up of him shooting creatures (class type +0x46 = 5; search 0x30d308; shot class 1633 0x30c9a8) for 90 s or 100 shots, vanishes in type-53 sparks and goes home | L01 0x30df40 | same 10 (10) | `mouse` |
| 258 | activation zone: when the probe (Ratchet, or the camera with +0x7e) enters / leaves its sphere / path / 6 cuboids, sets update / draw / collision of 12 moby groups and 11 mobys (rules +0x7c / +0x7d / +0x7f) | **L05** 0x2f5200 | 5, 7, 9, 10, 12–15, 17 (50) | `activation_zone` |
| 830 | floor switch: pulsing plate; stood on → green, sound, killed / collected bytes, clears the w of its path's points equal to its key | **L05** 0x30c5b0 | 5, 11, 15, 17, 18 (30) | `floor_switch` |

Other classes unported on 5+ levels: **615** (7 levels, 19 instances, L02 0x2d8ad0, 520 words): the Trespasser lock
(item 26 in hand while standing on it → camera script, hold 0x72, the puzzle); a minigame, not the same pattern.
**341** (5 levels, 17, L05 0x2f8080): the Hydrodisplacer pads (anim driven by a linked moby's +0xbc bits; +0xbc = 1
while Ratchet stands on it holding item 22; glow tween): water machinery, left to the water port.

Seams: `ClassInfo::ty` (class +0x46) filled by `scheduler::class_info`; `Services::{buried, mouse}`;
`TalkGame::metal_detector_bits` (synced from `GameState` in `Interact::sync_game`); `Scheduler::tick` calls the alert
frame after the moby pass. Not ported (counted in `Services::unported`): the HUD element 7 itself, the Metal
Detector's probe point and dig, the static overlay, the mouse's glow sprites (0x30de68), jet particles (type 74),
blob shadow and the Summoner moby's summon state.

Tests: unit tests in each module; `rc-game/tests/common_classes_levels.rs` (every class resolves on exactly its
levels; headless behaviour on two levels each); `all_levels_smoke` prints the drop in unported instances.

## Background: what moves between levels

- **Overlay layout.** Every level overlay loads at 0x15ef00, with the sections `.lit` 0x15ef00 (fixed start,
  size varies), `.bss`, `.data` (0x166000..0x166a00 start), `lvl.vtbl`, `lvl.camvtbl`, `lvl.sndvtbl` and `.text`.
  Section names come from Lombyte `~/Globals/Lombyte/config/overlays/us/level-NN.json`, and the addresses are
  also in `extracted/levels/NN/overlay.txt`. `lvl.vtbl` (the class → update table) is at 0x20bb00 on 01 and at a
  different address on every other level (00 0x1ea300 … 13 0x1f5880 … 18 0x1f2d00). Lombyte ships function
  names only, with no data symbols, so data addresses have to be derived.
- **Engine code is duplicated but relinked.** The ~1375 shared functions sit at a different address in every
  overlay, and their `jal`, `%hi`/`%lo` and `$gp` offsets differ.
- **Layout families** (byte comparison of level-01 tables at the same address):
  - `.lit` slots 0x15f650, 0x160270..0x160328, 0x160280/0x160290 and 0x161200: same bytes on all 19 levels.
  - `.lit` 0x15f718 block: same address on every level except 05 and 16, where it sits +0x10 higher (0x15f728).
  - `.data` tables such as menu pages 0x1b2a08, planet tables 0x1c22c0/0x1c23b8, QS d-pad 0x17e098 and ripple
    constants 0x1cad00: same address on **01, 05, 10, 12 and 16** ("family A"), elsewhere at another address.
  - Item definitions 0x179f40 and the melee chain table 0x17c0a8: same address only on **01, 10 and 12**.
  - Rule: treat every address ≥ 0x15ef00 as per-level unless it has been checked on all 19 overlays.
- **`clusters.tsv` under-matches.** The hash masks `lui`, `jal` and `$gp` fields but not the `%lo` immediates.
  So functions that touch `.data` split by layout family. Example: `ListUpdate` 0x28e600 clusters only with
  {05, 10, 12, 16}, and 0x2370b8 `HeroStatePhysics` is a singleton. Some n = 1 rows are therefore hash misses
  rather than Novalis-only code. The size check below separates the two.
- Boot-ELF addresses (< 0x15ef00, and boot-ELF data copies such as 0x1a04c0 and 0x165500) are valid on every level.

Effort: **S** ≤ half a day, **M** 1–3 days, **L** needs new reversing.
How the address is found on another level:
- **vtbl**: the level's `lvl.vtbl` section plus the cluster hash of the function it points at.
- **code**: a code-pattern search (the `lui`/`addiu` pair in a clustered function), as the glyph and item-table locators do.
- **bytes**: a byte-signature search of a pointer-free table.
- **same**: verified at the same address on all 19 levels.
- **new**: needs new reversing.

## Summary

| # | Subsystem | Finding | Program | How on other levels | Effort |
|---|---|---|---|---|---|
| M1 | menus | Page / widget / item record addresses | L01 `.data` | code (root page pointer in the open fn) + walk | M |
| M2 | menus | Widget kinds recognised by L01 callback addresses | L01 `.text` | improved clusters → role per address | M |
| M3 | menus | `MenuConsts` 0x160270.., `name_dy` 0x15f650 | L01 `.lit` | same (verified); add a check | S |
| M4 | menus | Planet name / point tables 0x1c22c0 / 0x1c23b8 | L01 `.data` | bytes or code (family A only today) | S |
| M5 | menus | Quick-select GP block 0x15f718 / page table 0x15f738 | L01 `.lit` | code (`$gp` offset); +0x10 on 05/16 | S |
| M6 | menus | Quick-select d-pad defaults 0x17e098 | L01 `.data` | bytes / code | S |
| M7 | menus | Frame lights 0x160280/90; corner lists 0x161fe0 (test only) | L01 `.lit` | same / code | S |
| W1 | water | `strip_tables(level)`: level 1 only (classes 676/678/761/1225) | L01 `.data` | new: other levels' water is other classes | L |
| W2 | water | Ripple layout 0x1e34c0, zone tables 0x1fa590..0x1fa6a0, module consts 0x1cad00..0x1cb1b0 | L01 `.data` | code (751-style init call); consts: bytes | M |
| W3 | water | Ripple manager update keyed to 0x2fd0e8 / class 751 | L01 `.text` | vtbl | S |
| W4 | water | Hero water level: `SetWaterLevel(0x1742e0)` table replaced by the hit z | L01 `.data` | code + new | M |
| C1 | moby classes | Port registry keyed by o_class + L01 `UPDATE_FN` | L01 `.text` / `lvl.vtbl` | vtbl | M |
| C2 | moby classes | Flyer: class 660 only; shared driver 0x2f5168 used by other classes | L01 `.text` | vtbl | M |
| C3 | moby classes | Class-27 emitter update `27 if level == 1` | L01 `.text` | vtbl | S |
| C4 | moby classes | Spawn-state rules per o_class (`initial_state`) | L01 `.text` | vtbl check | S |
| C5 | moby classes | Mission NPC classes [730, 790] (ship hide on arrival) | L01 | vtbl (+ new per level) | M |
| C6 | moby classes | `Services::new` level 1; sims set level 1 | — | level index | S |
| C7 | moby classes | Golden reads `lvl.vtbl` at 0x20bb00 | L01 `lvl.vtbl` | section lookup | S |
| P1 | particles | Emitter owners + load-pass cull only on level 1 | L01 | vtbl (data-driven owners) | S |
| K1 | sky | Star overlay tables at level-06 addresses, also read on 17 | L06 `.lit`/`.data` | code / bytes | S |
| H1 | hero / tick | Level-number branches of ported functions dropped | L01 `.text` | transcribe (code is shared) | M |
| H2 | hero / tick | Hero functions differ between overlays | L01 `.text` | diff vs other overlays | M |
| G1 | game state / scene | First-arrival scene trigger: level 1, scene 5 | L01 | new per level | M |
| A1 | audio | Level start: `start_track(0)`, never paused | L01 `.text` | transcribe entry | S |
| A2 | audio | Reverb not modelled (Novalis STUDIO_C 1500 only known) | — | new | M |
| T1 | trace | Runtime EE addresses of the Novalis spawn compare | L01 `.lit`/`.bss`/`.data` | code (`$gp`/`%lo` of clustered fns) | M |
| T2 | trace | Tie/shrub runtime record addresses | L01 `.lit`/`.data` | code | S |
| T3 | trace | `known_addrs(level)`: level 1 only, falls back to L01 | L01 `.data` | code | S |
| T4 | trace | `port_sim` Novalis constants (level 1, 0x2bd100, 0x2fd0e8, ship 531) | L01 | vtbl + level index | S |
| X1 | tooling | Cluster hash misses `%lo` immediates | — | fix `overlay_diff.py` | S |
| X2 | tooling | No per-level overlay symbol resolver | — | new `rc_formats::level_overlay` | M |

## Details

### Menus (`crates/rc-game/src/menus`)
- **M1** `pause.rs:36-52` (`pages::ROOT` 0x1b2a08, MAP, PLANET_SELECT, KIND22/23/2D, FORWARD_ON_SELF, the
  WEAPONS/OPTIONS/GOODIES widgets), `pause/port.rs:41-47` (OPTIONS 0x1b4dd0, QUIT_PAGE 0x1b6060, MODEL_FEW/MANY),
  `pause/tests.rs:5-10`. These are L01 `.data` and are valid only in family A (01/05/10/12/16). Find the root
  page from the `lui`/`addiu` in the page-open code (`PageMenuOpen` / `ROOT_ENTER` 0x2917d8's caller). Then
  walk the records, as `PageMenu::load` already does from the roots. For the other named pages, key on page
  content (title string id) instead of the address.
- **M2** `pause.rs:57-78` (`func::*`: LIST_UPDATE 0x28e600 … MISSIONS_UPDATE 0x28fec8) and the literal arms at
  `pause.rs:527` / `:552` (0x28dd10, 0x28dd18, 0x295200). These are L01 `.text`, so every other level has
  different addresses. Clusters today: LIST_UPDATE, TOGGLE_UPDATE, CAMERA_UPDATE and MISSIONS_UPDATE match
  only {01,05,10,12,16}; LABEL_UPDATE 0x28d788, 0x2917d8, 0x290508, 0x2921d0, 0x295208 and 0x295310 have no
  cluster; the draw callbacks match all 20. After X1, build a per-level `callback → WidgetKind` map.
- **M3** `pause.rs:105-109` (0x160270..0x160328) and `:344` (0x15f650): same bytes at the same address on all
  19 (verified). Keep them, but assert through the resolver.
- **M4** `pause.rs:342-343`: level names 0x1c22c0 and planet points 0x1c23b8. Their bytes are found elsewhere
  on the 14 non-family-A levels, so a byte signature (or the planet-list draw's `lui`/`addiu`) locates them.
- **M5** `quick_select.rs:27-28`: `GP_BASE` 0x15f718 and `PAGE_TABLE` 0x15f738 are 0x10 higher on 05 and 16. The
  rest of the block (a `$gp`-relative constant block) matches byte for byte. Take the `$gp` offset from the
  quick-select draw code. `PAGE_TABLE` is a pointer, so it self-corrects once the block is found.
- **M6** `quick_select.rs:29` `DPAD_DEFAULTS` 0x17e098: family A only; bytes found on every other level.
- **M7** `pause/frame.rs:42-43` light dir/colour 0x160280/0x160290: same on all 19. `CORNER_LISTS_ADDR`
  0x161fe0 (`frame.rs:45`, read only by the disc test at `:260`) is L01-only (other levels hold other bytes
  there, and level 00's `.lit` ends before it).
- Already general: the overlay is loaded per `RC_LEVEL` (`rc-engine/src/menu_render.rs:221`). The frame class
  0x472 = 1138 is in every level's class list. `QsItems` goes through `ItemTables::item_defs_addr` (pattern).

### Water (`rc-formats/src/water.rs`, `rc-game/src/water.rs`, `rc-engine/src/water_render.rs`)
- **W1** `water.rs:149-155`: strip descriptors 0x1e2fc0/0x1e3380/0x1fbc80/0x202d80 for level 1. Classes 676,
  678, 761 and 1225 are placed only on 01, and their draw callbacks (0x2f60f8, 0x2f6150, 0x2feb28, 0x309bf8)
  are singletons. The empty default for other levels degrades cleanly. The Pokitaru/Hoven sea (sweep L1) is
  different code and needs new reversing.
- **W2** `water.rs:262-264` (`ripple_layout`: 0x1e34c0, 21, 7) and `:290-309` (0x1fa5e8/0x1fa608/0x1fa628 zones,
  0x1fa590 masks, 0x1fa650 drips, 0x1fa6a0 mist, 0x1cad00 consts, 0x1cafe0 UVs, 0x1cb1b0 selectors, 0x1cae00 /
  0x1cad40 / 0x1cada0 orders). The patch table and zone tables are level data: take them from the
  `FUN_002b7a48(table, n)` call in the level's ripple class init (code). The module constants are engine data:
  same address in family A (0x1cafe0 also on 02 and 15), bytes found on all others. The module 0x2b7a48
  clusters on 01, 05, 07, 11, 12 and 13, but only Novalis places class 751, so find each level's user through the vtbl.
- **W3** `rc-engine/src/gameplay.rs:116, :351` and `tools/trace/src/port_sim.rs:41, :111`: `RIPPLE_UPDATE`
  0x2fd0e8 external for 751. Gated by `has_ripples()`, so it is safe. Generalise through the vtbl.
- **W4** `rc-game/src/hero/physics.rs:976-978`: the ground probe's `SetWaterLevel(0x1742e0, …)` (L01
  `HeroGroundProbe` 0x232dc0) is replaced by the hit z. On levels with real water tables this will differ.
- Already general: `UnderwaterLook::default` (`fog_zones.rs:52-60`) holds the bytes at 0x161200, which are
  identical in all 19 overlays. Its comment ("other levels keep their own alternate set") is wrong and should be fixed.

### Moby update / classes (`rc-game/src/moby_update`, `rc-formats/src/moby_spawn.rs`)
- **C1** `classes/mod.rs:28-44` + `bolt.rs:38/41`, `crate_.rs:28/31`, `grass.rs:20/23`, `flyer.rs:65/68`,
  `item_offer.rs:24/27`, `teleporter.rs:39/42`; `scheduler.rs:48` `port_update_fn(o_class)`. The registry is
  keyed by class number, and the per-level truth is `lvl.vtbl`. Checked via vtbl + clusters:
  - Bolts 13–16 and crates 500–511: the same function on all 19 levels.
  - Teleporter 1135: 01, 07, 08, 12, 13, 16, 17, 18.
  - Item offers 304/1456–1465: 01 and 13.
  - Grass: 724/725 only on 01. **The same grass function runs classes 1781/1782 (00), 766/767 (02) and 492
    (08)**, which a class-keyed registry misses.
  - Flyer 660, emitter 27 and ripple 751: 01 only.
  Fix: build `o_class → ClassUpdate` per level from the vtbl entry's function hash, and keep the L01 address
  only as a label.
- **C2** `flyer.rs:65-68`: `CLASSES = [660]`. The driver `FlyerPathDriver` 0x2f5168 (clusters 01/03/04/14) also
  runs the classes listed at `flyer.rs:31` (0x33, 0x3c, 0x40, …, 0x46a, 0x473, 0x474), which are placed on 03 (2),
  05 (22), 08 (1) and 16 (27). The level 3/9 combat block (`flyer.rs:489, :591`) is flagged as unported. The
  spline table `0x1b0930[i]` is built from gameplay paths, so it is already general.
- **C3** `rc-engine/src/gameplay.rs:114, :350` (`27 if level == 1`), `port_sim.rs:40, :111`. Correct today
  (class 27 is placed only on 01), but key it on the vtbl function instead.
- **C4** `moby_spawn.rs:221-297` `initial_state`, matching on 459, 572/865/866, 577, 666, 730/790, 750 and 1818.
  In the vtbl, 572/865/866 point at the same function on 05 and 11; 750 on 14 levels (00, 03–08, 10, 12–15,
  17); 1818 on 10 levels. 459, 577, 666 and 730/790 are 01-only. The rules transfer, but the module doc
  (`:18-20`) says "not checked per level", so add the vtbl check.
- **C5** `moby_spawn.rs:457` and `rc-engine/src/scene_render.rs:76`: `MISSION_NPC_CLASSES = [730, 790]`
  (`MissionNpcUpdate` 0x2fad68, 01 only). `ship_hidden_on_arrival` (`moby_spawn.rs:466`) and the scene trigger
  depend on it. Every level needs its own NPC classes, found from the functions the vtbl names.
- **C6** `services.rs:773` (`level: 1` default; the engine overrides it at `gameplay.rs:597`) and
  `port_sim.rs:298` (`svc.level = 1`).
- **C7** `rc-game/tests/moby_update_novalis.rs:218`: `elf_read(…, 0x20bb00, 0x924)`. Use the `lvl.vtbl` section.
- Already general: `SHIP_CLASSES` (`moby_spawn.rs:30`, 0x160548 identical in all overlays),
  `first_visit_ship(level)`. The level branches in `crate_.rs:108/141/427/710` (0x12, 0xf) and `flyer.rs:489`
  (3/9) are transcribed from the code.

### Particles
- **P1** `rc-engine/src/particle_render.rs:161` (owners built only when `level == 1`) and
  `rc-game/src/particles/type06.rs:246` (load-pass cull only on level 1). Faithful to 0x2bd100, which exists
  only on 01. Make the owner list data-driven (the classes whose vtbl entry is the emitter function), so other
  emitter classes can be added. The particle kinds the bolts and crates use are engine code.

### Sky
- **K1** `rc-game/src/sky_stars.rs:106-108` (`FIXED_POS` 0x1bdf30, `FIXED_RGBA` 0x1604f0, `MOVING_RGBA`
  0x160500) and `rc-engine/src/sky_stars.rs:82-88`. These are level-06 addresses, also read from level 17's
  overlay. 17's moving colours happen to be the same bytes at the same address (`.lit`), and 17 reads zeros at
  0x1bdf30 (it has no fixed stars). Tie each table to its level or locate it by code.
- Already general: `level_rotations` (`rc-engine/src/sky_render.rs:121-`) and `level_stars` were surveyed on all 19.

### Hero / camera / tick
- **H1** Level branches in ported L01 functions are absent from the port. This is the same engine code on
  every level, so each branch just needs transcribing:
  - `HeroGroundProbe` 0x232dc0: footstep class = 3 on levels 1 and 0x12. `physics.rs:973-983` does not do it,
    so **this one also affects Novalis**.
  - `HeroStatePhysics` 0x2370b8: level 0xf.
  - `HeroStateTransitions` 0x242930: levels 3/6/0x10 (ledge heights) and 0xf/0x11.
  - Idle 0x241e00: level 0xc.
  - Camera avoidance 0x312ef8: level 0xf (noted at `follow_camera.rs:1007`).
  - `InLevelFrameUpdate` 0x2aba68 pause triggers: levels 0xf/8/0xc (`tick.rs`, `menu_render.rs:450`).
  - `GameStateUpdate` 0x2a4080: levels 10/0xd.
  - `InitLevelRenderGlobals` 0x255958: 0xd, < 9, 10.
  - `PauseAllSounds` 0x28bf50: 0xd.
  Source list: `grep 'Ram0015ed84 [!=]=' work/decomp/level01.elf/*.c`.
- **H2** `HeroStatePhysics` is 21,408 bytes, and no other overlay has a function of that size. `HeroStateTransitions`
  0x242930 (19,120 B) has a same-size function only in 02. 0x23cf98, 0x233de0 and 0x240ed8 are cluster
  singletons. Diff each against its counterparts in two or three overlays (Ghidra) before assuming it is identical.

### Game state / scene / audio
- **G1** `rc-engine/src/scene_render.rs:72` (`NOVALIS_ARRIVAL = 5`) and `:227-232` (`level != 1` → no
  trigger). Each level's arrival scene comes from its own NPC / infobot code (M-reversing per level). The
  scene formats themselves are golden on 19 levels.
- **A1** `rc-game/src/audio.rs:557`: `start_track(0, 1, 0x400)` and always unpaused. `entry` 0x259c40 starts
  track `*0x151708` and unpauses only when (level 1 and planet flag 0x13dd43 clear) or (level 0 and 0x13d390).
  Everything else starts paused.
- **A2** `audio.rs:18`: reverb is not modelled; the per-level preset source is unknown.
- Already general: `GameState::apply_level_start(level)` / `apply_transition` (`game_state.rs:564-601`); the
  boot-ELF chunk tables (`save_game.rs:25-27`), vendor/price copies (`:288-290`) and item definitions by the
  `GiveItem` pattern (`:345-350`); the HUD / fonts / strings (glyph tables by the FontPrint pattern,
  `font.rs:136`); the sound bank / music / emitters and fog zones (gameplay section 0x80).

### Trace harness (`tools/trace`)
- **T1** `novalis_spawn.rs:26-40` (TICK 0x15f5cc, MODE 0x15f5c4, VSYNC 0x15f3f8, IDLE 0x160ff0, CAM_* 0x167240..,
  TAN_HALF_FOV 0x16cf70, FOG 0x15f444, UNDERWATER_LOOK 0x161200) and `:320-342`; `main.rs:287` bails unless
  level 1. These are runtime addresses in `.lit`/`.bss`/`.data`, and `.data` starts at 0x166000..0x166a00
  depending on level, so the 0x167240 camera and 0x16cf70 move. Locate them from the `$gp`/`%lo` operands of
  clustered functions (for example `GameStateUpdate` for the tick).
- **T2** `tie_shrub_cmp.rs:29-36` (0x160fcc, 0x160fc0, 0x160fc8, 0x160490, 0x160494, 0x1604a0, 0x180340, 0x180740).
- **T3** `tfrag_light_cmp.rs:77-81, :376`: `known_addrs` has only level 1 and silently falls back to L01.
- **T4** `port_sim.rs:40-44` (EMITTER/RIPPLE_UPDATE, RIPPLE_CLASS, SHIP_CLASS 531), `:111`, `:298`.

### Tooling
- **X1** `tools/ghidra/scripts/overlay_diff.py`: also mask the 16-bit immediates of loads, stores and `addiu` whose base
  comes from a `lui` (and `$gp` ones, already done). Re-run it. Expect most n = 5 / n = 1 rows (0x28e600, the hero
  functions) to become n = 19.
- **X2** There is no single place to ask "where is X on level N". Add `rc_formats::level_overlay`:
  - sections (with `lvl.vtbl`),
  - a vtbl reader,
  - byte-signature and `lui`/`addiu` code-pattern finders (generalise `save_game::find_item_defs` and
    `font::find_glyph_tables`),
  - a cluster lookup that ships a compact per-function hash table.
  Its output is an `OverlayMap` of named addresses per level, golden-tested on 19 overlays (L01 values as
  anchors). Two overlay readers exist today (`water::Overlay`, `font::read_overlay` / `menus::Overlay`); merge them.

### Already general (for contrast)
- Transcribed engine tables whose bytes exist in all 19 overlays (at other addresses): `GLOVE_JOINTS`
  (`hero/items.rs:262`, L01 0x17aa40) and the melee chain table (L01 0x17c0a8). These constants are valid.
- Every loader/renderer reads per `RC_LEVEL` (`level_load.rs:79-83`; `DEFAULT_LEVEL = 1` at `:14` is only the
  default), and missing Novalis-only data returns empty (water, stars, particles, scenes).
- Tests named `novalis_*` are golden anchors and need no change; they only gain siblings.

## Suggested order
1. **X1, X2**: the resolver and a better cluster hash. Every later step uses them.
2. **C1, C3, C4, C7, P1**: a vtbl-driven class registry. Bolts, crates and teleporters then run correctly on
   most levels, with no new ports.
3. **M1–M7**: menus through the resolver. Pause and quick select then work on all 19 levels.
4. **H1, H2, A1**: transcribe the level branches and diff the hero functions. H1's footstep branch is a
   Novalis fix too.
5. **C5, G1**: each level's mission NPC classes and arrival scene.
6. **T1–T4**: the harness per level (needs a PCSX2 savestate on the validation level).
7. **W1–W4, K1, A2**: water and the rest (new reversing).

## Recommended validation level: 13 (Gemlik Base)
- **Most shared gameplay.** It has the most placed classes in common with Novalis (25; next is 05 with 18),
  and four of the six ported class updates run there on the same function: bolts, 371 crates, 20 gold-weapon
  offers and 2 teleporters, plus 750. It has a placed ship-class instance and music boxes (class 6).
- **Different layout.** `.data` 0x166300 and `lvl.vtbl` 0x1f5880 differ from Novalis. Menu pages, planet
  tables, the QS d-pad, item definitions and the melee table all sit elsewhere, so every hard-coded address fails visibly.
- **Clean fallback test.** It has no water strips, ripples, emitters or flyers, so the Novalis-only paths must switch off cleanly.
- **Clean baseline.** The sweep (level_sweep.md) saw no rendering anomalies there.

Avoid 10 and 12 as the only check: they share Novalis's `.lit`/`.data` layout for most engine tables and would
hide address bugs. Use them afterwards as a cheap smoke. Second choice: **05 (Rilgar)**, with 108 instances of
the 572 family, 22 flyer-driver users and the +0x10 `.lit` shift.
