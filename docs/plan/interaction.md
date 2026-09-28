# The "use" system: prompts, talking, buying (RAC1 NTSC, level01.elf)

Addresses are **level01.elf** unless marked (every function named here is engine code present in all 19 level
overlays; `tools/ghidra/names/clusters.tsv` byte-clusters some of them, the others were matched per level by their
call sites, §5). gp = 0x166c00. Pad: pressed 0x13cae4 (after the lock), 0x13cb04 / 0x13cb00 pressed / held before the
lock. Confidence: **[H]** read from code and data, **[M]** code with lost arguments or one inference, **[L]** unverified.

## 1. Answer

**Yes, there is a general use system — two shared layers — but no generic "interactable" object.** Every "press △
to …" in the game goes through one of them; what is per class is only the proximity rule and what △ does.

1. **The context prompt** (HUD slot 12) [H]. One owner-keyed lease: a class that qualifies calls
   `try_set_help_message(owner, msg)` 0x278f58 every tick; the lease lasts 2 ticks (0x15f590 = 2) and is counted down
   by `PromptTick` 0x278eb8 (Lombyte `RaceTimerShow`, wrong) before the moby loop; the class then acts on △ itself:
   `if ((pad & △) && acquired) { … }`. The text goes to the 0x50-byte buffer 0x17e9b0 (`PromptSetText` 0x24cdb0) that
   the slot-12 draw 0x24c898 (Lombyte `HudRaceTimerDraw`, wrong) prints in a bar frame. The vendor, the ship, the
   teleporters, the transport pads, the vehicles and the Nanotech seller all use it (§2).
2. **The NPC talk system** [H]. A data-driven dialogue graph per NPC: `NpcTalkUpdate` 0x27b028, `NpcTalkRegister`
   0x27b480, `NpcTalkRefresh` 0x27b550. The NPC's slot comes from its instance record (+0x74), its node list from a
   per-level table; nodes carry the prompt text, the scene / movie △ plays, a condition (bolts ≥ price, item owned, …)
   with true / false successors and flags (auto, chain, **purchase**). The R.Y.N.O. salesman (Rilgar), the Infobot /
   Heli-Pack / Swingshot / Grindboots / Thruster-Pack / Hydro-Pack / PDA sellers, the gold-weapon offers, Blackwater
   City's "△ Enter race", the bouncer's bribe and the plain "△ Talk" NPCs are all rows of these tables (§3).

The **vendor** (class 11) is a prompt user whose △ switches to **game mode 5** (`OpenVendorMenu` 0x2ae1a0, §4); the
**ship** is a prompt user whose △ sets 0x15f630 (take-off, mode 6, menus.md §5); a **talker** hands over to an
in-engine scene (`DialogStreamStart` 0x2ac330, mode 2) or a PSS movie (0x2acf50, mode 1) and its dialogue advances
when that ends (0x2ac608 / `MovieExitToGameplay` 0x2ad2b8 → `NpcTalkRefresh(npc, talk, 1)`).

## 2. The context prompt [H]

| fn | name | what |
|---|---|---|
| 0x278f58 | `try_set_help_message(owner, msg)` | owner = 0x15f594 → text (if msg ≠ 0), timer 2, msg, return 2; free → take it, return 1; else 0 |
| 0x279000 | `force_help_message(owner, msg)` | `try_set`, else take it anyway (3) |
| 0x279070 | `PromptRelease(owner)` (Lombyte `OpenShipMenu`) | owner holds it and slot handle 0x162284 ≠ −1 → free lease and slot |
| 0x278eb8 | `PromptTick` (Lombyte `RaceTimerShow`) | `if timer && --timer == 0: owner = 0`; owner and msg (0x162288) → `HudShow(12, 0, 0x24c828, 0x24c878, 0x24c898)` or keep it up 10 ticks (`FUN_0024b4b0(h, 10)`); called by `InLevelFrameUpdate` 0x2aba68 before the moby loop unless the hero state is 0x1d / 0x32 |
| 0x24cdb0 | `PromptSetText(text)` | copy into 0x17e9b0 (0x50 bytes; longer → "Message too long") |
| 0x24c828 / 0x24c878 / 0x24c898 | slot-12 element | init: timer ScaleTicks(10) + 30, size 32×32; update: the generic ramp 0x24b538; draw: below |

**Draw 0x24c898** (Lombyte `HudRaceTimerDraw`): y = 0x15f770 (32) + 18 NTSC (10 PAL) = 50; `s = slide/8`,
A = trunc(128·s); text colour `A << 24 | 0x40f040` (green), regular font, `font_print_center(256, …)`. One line: cap
30080:1 at x = 224 − w/2 (32×32), middle 30080:0 at 256 − w/2 width w, cap rotated at x + w + 32. A byte 0x01 splits
the text into two lines: frame 54 tall, x from the wider line, line 1 at y + 8, line 2 at y + 27. On Rilgar (5) and
Kalebo III (16), within 5 of the race start ((294.5, 225.4) / (115.7, 287)), it adds the best-time banners (msg
21275 "%s: %d:%02d:%02d" of 0x15ee58.., 21276) below [H code, not ported].

The quick-select ring is blocked while an owner holds the lease (`0x15f594 ≠ 0`, menus.md §2), so △ near a vendor
opens the vendor, not the ring. `TeleporterPadUpdate` reads the owner (4) to fade its beam.

**Owners** (first argument at every call site of the 19 overlays; `interact::owner`):

| owner | who | levels | text | rule / △ |
|---|---|---|---|---|
| 1 | vendor 11 | all | 21475 "△ Activate Gadgetron Vendor" | §4 → mode 5 |
| 1 | Nanotech seller 1326 | 10 Orxon | 10014 / 10015 "△ Buy Premium / Ultra Nanotech for 4,000 / 30,000 bolts", 10016 / 10017 "You need …" | buys (records 40 / 41) |
| 1 | class 22 | 10 | 21489 "WARNING: O2 Mask required …" | warning only |
| 2 | ship (`ShipUpdate` 0x2a1c40 and copies) | all | 21476 "△ Enter ship" | within 4 of the hatch → 0x15f630 = 1 (take-off) |
| 4 | teleporter 1135 (`FUN_00309430`) | 1, 7, 8, 12, 13, 16, 17, 18 | per level table | standing on the pad → teleport |
| 4 | big red button 1118 | 6 | 6019 "△ Press the big red shiny button?" | |
| 5 | `Help_Update` (force, msg 0) | all | — | clears the prompt while a help box opens |
| 6 | gadget classes 168 / 170 / 172 / 615, vehicle 69 (force, msg 0) | all | — | no prompt while they run |
| 7 | vehicles: jet plane 1242, Gemlik 69 | 11, 13 | 11017 "△ Fly the Jet Plane", 21476 | board |
| 8 | transport pads 806 / 997 (Kerwan), 998 (Rilgar) | 3, 5 | 3017..3023 "△ Go to …", 5016..5029 | travel |
| 9 | 1061 | 6 | 6025 "△ Explore outside?" | |
| 10 | 1109 (6), 424 (8) | 6, 8 | 6018..6022, 8013..8016 | travel |
| 11 | 318 | 11 | 11019..11021 | travel |

## 3. The NPC talk system [H unless noted]

**Slots.** The loader's `MobyUnknown74Hook` 0x25e7b0 stores each moby whose instance +0x74 ≠ −1 at
`0x179638[base + k]`, base = `0x1c4938[level]` (`{0, 2, 15, 19, 34, 37, 46, 50, 54, 58, 60, 65, 69, 74, 86, 93, 100,
107, 114, 121}`, the same in every overlay); `FUN_0027b3b0(npc)` finds the global slot again. The node list is
`0x1b1af8[slot]` (per-level address; 45 lists on the disc). The "talked" flag of slot i is `0x13d5b0 + 16·i + 0xc`, the
flags word of the landmark record i (save chunk 15).

**Talk block** = the first 0x40 bytes of the NPC's pvars: +0x04 s16 the node that played last, +0x08 u8 auto (node
flag 1; 0xff disabled), +0x09 registered, +0x0c f32 **r**, +0x36 s16 current node, +0x38 tick of the change, +0x3c
the node list.

**Node** (0x1c bytes): +0 msg (prompt text), +4 s16 scene (−1 none, bit 0x4000 = movie), +6 next, +8 condition kind,
+0xa item / argument, +0xc node if true, +0xe node if false, +0x10 flags (1 auto: no △ and no facing; 4 chain: the
next node's scene plays as soon as this one ends; 8 purchase). Conditions: 1 bolts ≥ price (records +0), 2 item owned
(0x13d4c0), 3 owned and 0x13d4e8 = 0, 4 flag 0x13d388, 5 spendable gold bolts ≥ arg, 6 bolts ≥ gold price (records
+0x14) and spendable gold bolts > 3 (`compute_clamped_count_difference` 0x2790d0: gold bolts collected ≤ 40 − 4 ·
gold weapons ≤ 10).

**`NpcTalkUpdate(npc, talk)` 0x27b028** (returns 1 when a scene started / a node advanced): hero state 0x1d or HP 0 →
0; register if needed; game mode ≠ 0 → 0; node −1 or auto 0xff → 0; `VecDistance(npc, hero feet) > 2r` → 0;
`NpcTalkRefresh(…, 0)`; for a non-auto node: `FastDiffRots(atan(hero − npc), npc yaw) > r` → 0 and
`FastDiffRots(atan(npc − hero), hero yaw) > 1.57` → 0 [H, disassembly: the same float r is the range half and the
facing tolerance]; tick < cooldown 0x179590 (only ever 0) → 0; non-auto and △ (0x13cae4 & 0x10) not pressed → keep
slot 12 up through handle 0x160130 (and for kinds 1 / 6 the bolt counter for 60 ticks), 0. Else: talked flag := 1;
0x179588 = npc, 0x17958c = talk; flag 8: kind 1 → bolts −= price, kind 4 → 0x13d388[item] = 0; scene −1 →
`NpcTalkRefresh(…, 1)`; else slot 12 freed, `DialogStreamStart(scene)` (the NPC hidden while it runs) or the movie.

**`NpcTalkRefresh(npc, talk, advance)` 0x27b550**: with advance: +0x04 = node, node = next (flag 4 on the old node:
the new node's scene starts at once); then the node's text into 0x17e9b0 (kind 6: `%d` becomes the gold price as
`"%d%s%03d"`, separator from 0x20a180 by language % 6: `,` En/Fr/Es, `.` De/It; msg 0 frees the slot); the condition
picks yes / no (recursing when the node changes).

**Users** (callers of each level's `NpcTalkUpdate`, found through the register function's cluster, §5):

| level | classes | tables (slot: nodes, in words) |
|---|---|---|
| 0 Veldin | 834 (Clank) | chained scenes 4 → 1 → 2 → movie 1 → 3 (auto) |
| 1 Novalis | 774 Water Pump Worker; 304 gold-weapon offers | 774: scene 0 (auto) → "You need 500 bolts to buy the Infobot" / "△ Buy Infobot for 500 bolts" (kind 1, item 37, purchase + chain) → scene 1 → movie 2 → scene 2; offers: "You must own the …" (kind 2) → "You need 60,000 bolts and 4 Gold Bolts" / "△ Buy the Gold Tesla Claw for 60,000 bolts and 4 Gold Bolts" (kind 6) |
| 2 Aridia | 786, 788 | "Destroy the sand sharks", "Bring the prize from the hoverboard races" (kind 4) |
| 3 Kerwan | 890, 909, 914 | "△ Buy Heli-Pack / Swingshot for 1,000 bolts" (kind 1, purchase), "△ Talk" (21477) |
| 4 Eudora | 1190 | chained scenes / movie 6 |
| 5 Rilgar | 918, 919, 925 | **"△ Enter race"** (kind 2: owns the Hoverboard 30, else "You need a Hoverboard to race") → node 3 (no scene: the class starts the race); "△ Bribe Bouncer with 4,000 bolts" (item 38); **"△ Buy R.Y.N.O. for 150,000 bolts"** (kind 1 item 23, purchase; kind 3 → "△ To get R.Y.N.O. again") |
| 6 Nebula G34 | 1105 | "△ Buy Grindboots for 2,000 bolts" |
| 8 Batalia | 774 Big Turret Guy, 1130, 1144, 1283 | Infobot 2,000 (item 39), "△ Enter turret and blast ships" |
| 11 Pokitaru | 90, 114, 298 | "△ Pay 2,000 Bolts for the Thruster-Pack", "△ Trade Raritanium for Persuader" (kind 4) |
| 12 Hoven | 282, 328 | "△ Pay 2,000 Bolts for the Hydro-Pack" |
| 13 Gemlik | 304 gold-weapon offers | as Novalis |
| 14 Oltanis | 851, 924 | "△ Buy Gadgetron PDA for 1,000 bolts", Infobot 2,000 (item 42) |
| 15, 16 | 1446; 1377, 1455 | Kalebo III "△ Enter race" (16007) |

Levels 7, 9, 10, 17, 18: no talk user [M: their register function did not cluster; checked by the absence of
talk tables in their range].

**Price records** 0x1c4530[43] (0x18 bytes; the range table follows them): +0 price, +4 discounted price (flag
0x13d4e3 [L]), +8 u16 vendor ammo unit price, +0xa PDA ammo unit price, +0xc pickup amount, +0xe max ammo, +0x12 ammo
granted with the item, +0x14 the gold version's price. Items 37..42 are the talk system's pseudo items: Infobot
Novalis 500, bribe 4,000, Infobot Batalia 2,000, Premium Nanotech 4,000, Ultra Nanotech 30,000, Infobot Oltanis
2,000. **Where the item is granted:** the purchase node only takes the bolts; the class gives the item after the
scene (774: talk +0x04 = 4 → `UnlockPlanet(2)`; 304: +0x04 = 2 → gold weapon owned and bolts −= gold price).

## 4. The Gadgetron vendor [H unless noted]

**Class 11 update** 0x2bb128 (not a Ghidra function; read from the disassembly): glow phase 0x1613a4 += 0.05 (±3.14
wrap), `+0x90 = ((int)(sin φ · 48) + 96) · 0x010101 | 0x80000000`. State 0: `CreateMoby(1143)` (the hologram,
+(0, 0, 2.95), two manipulators), state 1. State 1: hologram scale −0.1 (hidden at 0); every 8th tick XY distance
(`VecDistance2`) ≤ 16 and |Δz| ≤ 8 → state 2, seq 1 (blend 10). State 2: scale +0.1; every 8th tick XY > 18 or
|Δz| > 10 → state 1, seq 0. Prompt rule: hero group 0 / 1 or state 3, not 0x1d / 0x32, 0x1413f4 = 0, XY ≤ 4 and
|Δz| ≤ 2, `FastDiffRots(hero yaw, atan(vendor − hero)) ≤ π/2`; opening also needs key A (+0x52) = 1 → owner 1, text
21475; △ with the lease → `OpenVendorMenu(vendor)`, state 3 (hologram scale 0). `VendorExit` sets state 1.

**Mode 5** (menus.md §4 has the full flow; the port's module docs list the details): `OpenVendorMenu` 0x2ae1a0
(list, selection n/2, sound 3, `FadeToBlack(4)`, mode 5, hero SetState(100) with 0x1413f5 = 1 (hidden), camera
`CameraScript` to the vendor's rows · (3.8, 0, 1.5) + position facing yaw + π, HUD bolt counter (slot 2 | 0x10) and ammo
slot (0x30)), `VendorModeUpdate` 0x2b03b8 (substates 0 fly-in 40, 1 menu, 2 leave 40, 3 demo), the buy flow 0x2af7e8
(Lombyte `DrawSpriteHelper_C`), `AddAmmo` 0x2494d8 (returns the overflow), `GiveItem` 0x275760, `VendorExit` 0x2ae660.
Class sounds: 0 select / cancel, 1 cursor, 2 denied, 3 open, 4 screens on, 5 △, 6 closed, 7 purchase.
Screens: several 512×128 targets in one 512×512 texture (0x2b3130) mapped onto the vendor's monitor joints
(`MobyGetBoneMatrix(vendor, 3, {4s, 4s+1, 4s+2})` per screen, `DrawBoneQuads`): ticker 0x2b1a48 (the LED font of HUD
icon 0xe935, cells 0x1ca598, advances 0x1ca698, characters 0x20..0x5a only, 'b' blinks the next glyph, scale 2,
+2 px per frame, a random line of 0x1ca538[24] via `randi(24)` when it has scrolled out, the text prefixed with 18
spaces 0x20a510), icon strip 0x2b1c10, item panel 0x2b1f08, salesman video 0x2b1b58, prompt 0x2b2688, button window
0x2b2430 [M: text place lost], popup text 0x2b2848 ("Purchase?" / "✕ Yes" / "△ No", "How many?" with `<` qty `>` and
the total at (64, 76) right-aligned, "You're maxed out!", "You can't afford it!").

## 5. Method

Call sites of `try_set_help_message` / `force_help_message` were found in all 19 overlays by scanning for `jal` to
each level's copy (clusters.tsv), with the owner and message from the `addiu a0/a1` before the call; the class of
each site from the level's class table (`lvl.vtbl`). Each level's `NpcTalkUpdate` was found as the caller of its
`NpcTalkRegister` (cluster of 14), its users the same way; table and range addresses from the register / lookup
code; the node lists dumped and printed with the level's English text. The vendor update was disassembled from the
overlay bytes (it is not a function in the Ghidra project).

## 6. In the port (2026-09-27)

* `crates/rc-game/src/moby_update/interact.rs` (new): `Prompt` (try_set / force / release / tick / shown), the owner
  ids, `vendor_rule` / `talk_rule` / `diff_rots` (native `f32`: `atan2` and an exact wrap for `FastArcTan` /
  `FastDiffRots`), `TalkTables::load` (node lists, ranges and the 43 price records `ShopTable`, found in any overlay by
  the register code's pattern and the range bytes), the talk block, `talk_register` / `talk_update` / `talk_refresh` /
  `poll_scene_end`, `Handoff` (OpenVendor, Scene, Movie, ShipMenu, GoldUpgrade) and `GameWrite` (the saved-game
  writes, applied by the engine after the tick), the state `Interact` in `Services::interact`.
* Classes: `vendor.rs` (11, new), `talking_npc.rs` (774, new: Novalis' Water Pump Worker; the Batalia states and the
  head look-at counted), `item_offer.rs` (304: its "not ported" help registration and buy prompt are now the talk
  system; the offer buys the gold weapon on talk +0x04 = 2).
* HUD (`crate::hud`): `Element::Prompt` (slot 12, init / ramp / draw 0x24c898), `HudState::set_prompt`, `keep_up`
  (`FUN_0024b4b0`), `bolts_pinned` (the vendor's slot 2 | 0x10).
* `crates/rc-game/src/menus/vendor.rs` (new): mode 5 — `VendorTables::load` (item names / icons from the item
  definitions, descriptions from the hologram table, ticker lines, LED font: all located in the overlay by their
  bytes), `build_list`, `Vendor::open` / `frame` / `draw`, the buy flow, `vendor_camera`.
* Engine: `crates/rc-engine/src/interact_render.rs` (new): `install` (at load, before the load pass), `after_tick`
  (the OpenVendor hand-off → mode 5; the talkers' Scene / Movie hand-offs stay for crate::scene_render, which plays
  or skips them and signals the dialogue), `vendor_frame` (mode 5 per main-loop frame: pad, frame, draws through the
  menu 2D layer, class sounds into the audio system on the game's stream, HUD feed, camera cut, exit), `hide_hero`.
  `crate::hud_render::HudFeed` carries the prompt and the vendor's HUD requests. `RC_GIVE_BOLTS=<n>` (debug) sets the
  bolts at the start; `RC_INTERACT_TRACE=1` logs prompt owners, hand-offs, purchases and sounds.
* Tests: `interact::tests` (lease, vendor and talk rules, wrap, gold bolts, node parse), `menus::vendor::tests` (list,
  prices, camera), `hud::tests::prompt_slot_12_shows_while_requested`, `crates/rc-game/tests/interaction_vendor.rs`
  (Novalis headless: prompt → △ → mode 5 → ✕ ✕ buys the Pyrocitor → △ → back, bolts 5000 → 2500, owned, quick
  select [10, 16], stock byte 0x50; determinism).

**Port choices / not ported** (each is a visible difference):
* The vendor's screens are 2D panels at fixed places (`vendor::layout`), black at alpha 0x60, instead of textures on
  the monitor joints; the item model is replaced by its HUD icon; no salesman video, hologram (created but kept hidden:
  its glass draw and cone 0x2ba9c0 are not ported), frame mobys, weapon demo scenes or PDA remote vendor.
* The world does not update during the 40-frame fly-in and leave (the game runs the moby loop and particles there);
  the vendor's own animation (seq 3 open, seq 4 close) is not advanced in mode 5, so the box stays closed.
* The camera cuts to the vendor's front after the open's fade (the game's `CameraScript` / `CameraScript2(2)` blend
  back is a cut here).
* The scene-end `HeroTeleport` of a talker (`FUN_002783a8`) is applied through the hero-write channel by
  crate::scene_render; with `RC_SCENE` set its scene requests are dropped (a talker then waits in state 2).
* The prompt's race best-time lines (levels 5 / 16), the ship's and teleporter's prompts (their classes are not
  ported), `PromptRelease` users.

## 7. Engine check (Novalis)

`RC_SCENE=0 RC_HERO_AT=168.16,138.03,60,0.9327` (3 units in front of the vendor, facing it), `RC_GIVE_BOLTS=5000`,
`RC_PLAY_SCRIPT="100-100:press TRIANGLE,180-180:press X,200-200:press X"`: the prompt from frame 8; frame 100 △ →
OpenVendorMenu (2 entries: Bomb Glove ammo, Pyrocitor), sound 3; frame 144 the screens (sound 4); frame 180 ✕
(sound 0), popup "Purchase?"; frame 200 ✕; frame 216 the purchase (2500 bolts, sound 7), bolts 5000 → 2500, quick
select [10, 16]. With `RC_GIVE_BOLTS=1000` the ✕ gives sound 2 and "You can't afford it!". Two runs to frame 230
give identical PNGs (also the not-enough-bolts shot at frame 196).

**Talker (scenes on)**: `RC_HERO_AT=253.13,186.67,95.57,2.5066` (in front of the Water Pump Worker 774), `RC_GIVE_BOLTS=5000`,
no `RC_SCENE`: tick 1 the worker's auto node 0 hands off scene 0 (queued behind the arrival scene 5); the talked flag
(landmark 3) is written; scene 5 plays (1532 frames), then scene 0 (2318 ticks, actors 0 / 774 / 10 / 750, 2342
frames); at its end crate::scene_render places Ratchet and signals the dialogue, which advances to node 2 (bolts ≥ 500)
and shows "△ Buy Infobot for 500 bolts" in slot 12 (frame 3960). With `RC_SCENE=0` the same hand-off is logged and
dropped. Headless: `crates/rc-game/tests/interaction_vendor.rs`. `novalis_hero_digest` byte-identical (2800 lines).

## 8. Racing and other hand-offs (what each needs)

* **Blackwater City race (Rilgar 918/919/925)**: the node "△ Enter race" advances to a node with no scene; the class
  then starts the race (hero on the hoverboard, `0x15ed84`-specific race code, HUD slot 12's best-time lines,
  `mode_freezeInit(0)` "Quit Race?"). Needs: the class, the hoverboard hero states, the race HUD — out of scope here.
* **Ship** (`ShipUpdate`): owner 2, within 4 of the hatch, △ → 0x15f630 → take-off (mode 6, menus.md §5). The
  `Handoff::ShipMenu` exists; the class is not ported.
* **Teleporter** (1135): owner 4 standing on the pad; teleport states not ported.
* **Movies** (node scene bit 0x4000): crate::scene_render's `play_movie` → crate::movie_render (native player); the
  dialogue continues at `MovieExitToGameplay`.
* **The Novalis Infobot** (2026-09-28, hero_gameplay.md §6): after the sale's scene 1 → movie 2 → scene 2 the worker's
  `UnlockPlanet(2)` / `ShowPlanetBanner(2)` reach the game state and the HUD banner through `rc_game::cinematic`
  (engine: the whole chain from △ to the banner, frames 2470 → 4420).
