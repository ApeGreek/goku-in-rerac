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
  prices, camera), `hud::tests::prompt_slot_12_shows_while_requested`, `crates/rc-game/tests/ui/interaction_vendor.rs`
  (Novalis headless: prompt → △ → mode 5 → ✕ ✕ buys the Pyrocitor → △ → back, bolts 5000 → 2500, owned, quick
  select [10, 16], stock byte 0x50; determinism).

**Port choices / not ported**: superseded by the vendor rebuild (§10); the 2D-panel vendor described here before
2026-09-28 is gone.
* The scene-end `HeroTeleport` of a talker (`FUN_002783a8`) is applied through the hero-write channel by
  crate::scene_render; with `RC_SCENE` set its scene requests are dropped (a talker then waits in state 2).
* The prompt's race best-time lines (levels 5 / 16), the ship's and teleporter's prompts (their classes are not
  ported), `PromptRelease` users.

## 7. Engine check (Novalis)

(2026-09-28, the rebuild of §10: frame 100 △, frame 104 substate 0, frame 144 the screens (sound 4), frame 180 ✕
(sound 0), frame 200 ✕, frame 215 the purchase (sound 7), frame 246 the salesman's greeting line (vendor_audio 000),
△ at 330 → substate 2 at 337 → `VendorExit` at 377 (sound 6), the camera blending back behind Ratchet 3.5 in front
of the vendor. The older numbers below are the 2D-panel port's.)

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
dropped. Headless: `crates/rc-game/tests/ui/interaction_vendor.rs`. `novalis_hero_digest` byte-identical (2800 lines).

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

## 9. Vendor, as built (the complete experience, level01) [H unless marked]

Everything the player sees and hears from walking up to a Gadgetron vendor to walking away, read from the code
(level01.elf addresses; the vendor code is the same in every overlay). Constants at gp = 0x166c00.

### 9.1 Approach (class 11 update 0x2bb128, §4)

* **Idle** (state 1, far): the machine sits closed on **seq 0** (1 frame) and pulses its glow word +0x90 (phase
  0x1613a4 += 0.05). The hologram child **class 1143** (the Gadgetron logo: 3 joints, 9 normal + 7 chrome packets,
  class scale 0.0136) was created at init 2.95 above the vendor, draw distance 64, +0x73 = 32; its scale V+0x90
  is 0 and it is hidden (mode |= 1).
* **Near** (every 8th tick: XY ≤ 16 and |Δz| ≤ 8 → state 2): **seq 1** (1 frame) blended over 10 ticks, the
  hologram shown and grown by 0.1 per tick to scale 1 (10 ticks), drawn at `V+0x90 · class scale · 2.5`. Its two
  manipulators (`AttachManipulator(child, 0/1, V+0x10/+0x50)`) spin its joints about z at +0.01 / −0.01 rad per tick
  (0x16139c / 0x1613a0, `FUN_00221e38(rec, 2, a)`). The draw callback **0x2ba9c0** (`RegisterDrawCallback2` every tick
  of states 1 and 2) draws the projector beam under the logo, the scan plane, and the **four glow points**: the shared
  glow quad `0x2781d0` (size 0.1333, FX 0xb, additive) at `rows·(1.1·cos a, 1.1·sin a, 0.59) + position`, a = k·π/2 − π,
  coloured `+0x90 & 0xffff0000` (the blue byte of the pulsing glow word; 0x2bb058..0x2bb0d4): the halos on the four
  antenna tips. Port: the callback is registered (`Callback::VendorBeam`, list 2); `crate::fx_draw` draws the glow
  points, crate::vendor_render the beam (2026-09-28). Leaving (XY > 18 or |Δz| > 10) → state 1, seq 0 (blend
  10), the logo shrinks 0.1 per tick and hides at 0.
* **Prompt** (state 2 only): §4's rule → "△ Activate Gadgetron Vendor" (21475, owner 1).
* **Sounds**: none in states 1 / 2 (the class sound table has 8 entries; all are played by mode 5).

### 9.2 Opening (`OpenVendorMenu` 0x2ae1a0, then `VendorModeUpdate` 0x2b03b8 substate 0)

On the tick △ is pressed with the lease, the class calls `OpenVendorMenu(vendor)` inside the moby loop:
1. Clears the vendor globals 0x1ca940 (0x220 bytes) and 0x1cab60; the salesman voice index 0x1ca978 = language
   0x15ed88 − 1 (≥ 0); `sound group pause 0x1d` (`func_0x0012e3e8`), **`music_Pause(0)`**.
2. The list (`VendorBuildItemList` 0x2adef0), the selection in the middle, the carousel offset; the HUD's **bolt
   counter** (`queue_animation_update(0x12, …, 9999999)`: slot 2 pinned) and, for an ammo entry, the **ammo slot**
   (0x30: icon 60000 + item, ammo 0x13d428[item], max records +0xe).
3. `DAT_0015f5d8 = 1` (the next render is skipped), substate 0, the snapshot / salesman buffers at 0x174284 /
   0x174288 + 0x60000, **class sound 3** (open), then **`FadeToBlack(4)`**: 4 blocking frames that darken the last
   image with black at alpha 0x20, 0x40, 0x60, 0x80 (0x21b438); then the full-screen fade 0x15f3fc = 1.0.
4. Mode 5, hero `SetState(100, 1)` with 0x1413f5 = 1 and `FUN_002486c0` (**Ratchet and his items hidden**),
   `hard_cut(vendor, 2, 0)`: the vendor jumps to **seq 2** (12 frames: the unfold), speed = 0.5 · 0x15ed60 (1.0 on
   NTSC) → 24 ticks.
5. The camera: **`CameraScript(eye, euler, 1, 0, 0)`**: mode 1 = snap, i.e. a **cut** (under the black) to eye =
   vendor rows · (3.8, 0, 1.5) + vendor position, Euler (0, 0, vendor yaw + π) (level, facing the machine); every
   mode-5 frame re-sends the same targets (0x316dd0 / 0x316e28).
6. The vendor light set 14 (0x1806c0 / 0x1806d0) = colour (0.9, 0.9, 0.6), direction vendor rows · (−0.42, −0.7,
   −0.577), second light off; 4 manipulators on the vendor's joints 0x14..0x17 (0x166300: the arm rotations that
   lay out the monitors for ≤ 7 entries: ±(7 − n) · 0x681 on two of them).

**Substate 0** (40 frames, `ticks(0x28)`), per frame: **the world runs** — `FUN_002ab920`, `MobyUpdateLoop`,
`RunLevelCallbacks`, `PatchShrubGifs`, `UpdateParts`, `UpdateAllPointLights`, `IncrementTickCounter` (gated by the
debug flags 0x16c4e0, all set in play) — but **not the hero and not the camera update**; the fade 0x15f3fc −= 0.34
(1.0 → 0.66 → 0.32 → 0: black for 3 frames after the 4 fade frames); the vendor's own update is state 3 (hologram
scale 0) while the moby loop advances its seq 2 unfold; the screens' glass quads (0x2b3700) are registered while the
timer ≤ 36. At 40: **seq 3** (1 frame: open) blended over 8 at speed 1.0, `0x1ca94c = 1` (snapshot request),
0x1622a0 = 1 / 0x16229c = 8 (the screens' **power-on**, 8 frames), **class sound 4** (screens on), substate 1.
`sound_update` runs at the end of every mode-5 frame.

### 9.3 The menu screen (substate 1; render `DrawWorld_Mode5` 0x2b4020)

**Update** (substate 1: no world update, no tick): the vendor's `MobyAnimAdvance`; the first frame after the salesman
data arrived creates the salesman (below); the spinning mobys (below); the pad (§9.4); `DrawSpriteHelper_C`
0x2af7e8 (the buy flow); `HudUpdate(1)`; `sound_update`.

**Background.** The first substate-1 render draws the world once with the vendor hidden (+0x34 |= 1) and **grabs the
frame** (`GrabFrameSnapshot`); every later frame uploads that still image and draws on it, in order: the item
hologram, the vendor (`DrawMobyList(vendor, 1)`), the hologram cone, the six screens, the popup (while buying), the
glass quads, the HUD. So the world behind is frozen; the vendor, the hologram, the screens and the HUD animate.
The same render creates the screen mobys (0x1ca954 + 0x100 = 0x1ca960: +0 and +0x500 class 13, +0x100 / +0x200 /
+0x300 by `FUN_002af3f0` 0x2af3f0) and starts reading the **salesman data** `vendor.bin` (toc 0x198) from disc.

**The screens** (`VendorDrawScreens` 0x2b3130, read from the disassembly: it does not decompile). For screen s =
0..5 (0 ticker 0x2b1a48, 1 item panel 0x2b1f08, 2 salesman 0x2b1b58, 3 button window 0x2b2430, 4 prompt 0x2b2688,
5 icon strip 0x2b1c10; jump table 0x20a570):
1. `MobyGetBoneMatrix(vendor, 3, {4s, 4s+1, 4s+2})` 0x264630: the world positions p0, p1, p2 of three vendor joints
   (the monitor's corner and its two edges). With the margins m = 0x1ca798[s] (4 floats: s0 (0.01, 0.01, 0.009,
   0.01), s1 (0.01, 0.03, 0.03, 0.03), s2 (0.03, 0, 0, 0), s3 (0.02, 0.06, 0.04, 0.035), s4 (0.07, 0.05, 0.03,
   0.03), s5 (0.03, 0.06, 0.02, 0.03)): a = p1 − p0, b = p2 − p0; corner c = p0 + â·m0 + b̂·m1; a = â·(|a| − 2·m2),
   b = b̂·(|b| − 2·m3).
2. `FUN_002adc38(c, c + a + b)` projects the corner and the opposite corner with the current camera: the screen
   rectangle (x, y, W, H) in game pixels (an axis-aligned rectangle: **the screens are drawn flat, facing the
   camera**, over the monitor's projected extent).
3. Power-on (0x1622a0, frames 8 − 0x16229c) and power-off (0x162298, frames 0x162294): f = frames / 8; the
   rectangle shrinks to f of its width and height about its centre (the CRT turning on / off); drawn again with
   `FUN_002adc38` → (x', y', w', h').
4. The render target: `fun_00239690(9, 7)` = a 512×128 target in VRAM with **`SetRenderToTextureView(1.0, …, 512,
   128)`**: the same camera, projection zoom 1.0 (instead of 0.63), centre (256, 64); cleared black
   (`FUN_00223470` 0..512 × 0..128, colour 0); the screen's draw function; `FUN_002b2cd8(w', h', s)` the static; then
   the view restored (0x2b3100).
5. The target's texels (0, 0)..(W − 1, H − 1) are drawn as a sprite on (x', y', w', h') (`DrawBoneQuads` 0x21c018 =
   the textured-rectangle packet), RGBA 0x80808080, ALPHA 0x64. At rest the copy is 1:1 (the screen content is laid
   out in screen pixels).

**The static** (`FUN_002b2cd8(w, h, s)`, into the target after the content; it sets TEST_1 0x32003 and, before the
noise and again before the bar, **CLAMP_1 = 0** (`VU1_addGSregister(8, 0)`: REPEAT), so the texel ranges below, far
past the 32×32 noise and the 16×16 bar, tile): per screen a counter 0x1caba0[s]:
when running (+2 per frame, the ticker's never below 0x18) FX texture **0x1a** (noise) is drawn over (0, 0, w, h)
at a random texel offset (`randi(200)`, `randi(200)`) with alpha `min(2·(0x80 − |c − 0x80|), 0x80)`
(`subtract_integer_with_clamp` 0x221110 is `abs`: fades in over 32 frames, holds 64, fades out over 32; the ticker's
resting 0x18 gives 0x30), ALPHA 0x68 with FIX = that alpha (added: `Cd + Cs·FIX/128`), until c passes 0xff; an idle
screen restarts it with probability 1/700 per frame. Screens 1..5 also roll a **scan bar**: FX texture **0x1c** over
(0, −(0x200 − c)/32, w, 1.5·(h + 16)), colour 0x505050, alpha `min(0x100 − |c − 0xfe|, 0x50)` with c before its +2
(fades in over 38 frames, holds, fades out; 255 frames), ALPHA 0x44, started with probability 1/360. The menu
panels' effect (`fun_00223e28`, menus.md §3 step 7) uses the same noise curve. The popup (screen 6) adds FX **0x19** (64×64 glass) over it.

**Glass quads** (0x2b3700, after the screens): screens 1..5 get a quad in 3D on the same inset corners, FX
texture **0x19**, RGBA 0x80808080, TEST 0x32003, ALPHA 0x44, UV (0, 0)..(1, 0.984).

**Screen contents** (target pixels, small font unless noted):
* **Ticker** (0): the LED text 0x2b1838 (`fun_00238310(2.0, text, −scroll, 8)`): HUD icon 0xe935 glyph cells
  0x1ca598 / advances 0x1ca698, scale 2 (`DrawTexturedQuad(x, 8, 18, 18, u, v, 9, 9, 0x80404040, font)`: half
  brightness, from the disassembly at 0x2b19b4), 'b' blinks the next glyph. The ticker is drawn right after the
  hologram cone, whose `FastDrawQuadReal` leaves TEX1_1 = 1 (point sampling: its quad's +0x80), so its glyphs and
  static are point-sampled; screens 1..5 follow the item's / salesman's moby draws (bilinear TEX1) [H: order in
  `DrawWorld_Mode5`; moby TEX1 per moby_untextured.md]; +2 px per frame; when scrolled out, a random line
  of 0x1ca538[24] (`randi(24)`), 18 leading spaces; black bars at x 0..4 and 226..230.
* **Item panel** (1): `DrawMobyList(+0x100)` **the item's 3D model** (class = item definition +0x10: the weapon's
  own model, e.g. Pyrocitor 176, Bomb Glove 192, `0x1df` for item 24) and `DrawMobyList(+0)` (class 13), both lit
  by light set 14 with ambient 0x202020, drawn with the RTT view; then the name at (6, 8), for ammo "Ammo" (20317)
  at (24, 24) and the unit price right-aligned at (118, 101); a weapon's price at (118, 101) (grey 0x80808080 and the
  discounted price at (118, 85) under the discount 0x13d4e3; the PDA strikes the old price with 7 lines).
  **Placement** (`FUN_002af3f0`, on every selection change): position = vendor rows · ((0, −1.6, 1.5) +
  0x1c8da0[item]) + vendor position, with +0x1c8dac[item] added to z again for a weapon (twice with the discount);
  rotation 0x1c8d90[item] (x, y) and z = table + vendor yaw; `fun_00212ed8(m, 0 or 1, 0)` its first frames; +0x194
  = 0 (no spin). E.g. Pyrocitor (16): offset (−3.4, −2.15, −0.15) + 0.12, rotation (0, −1.57, −3.0).
* **Salesman** (2): `DrawMobyList(salesman)`: the **class 12 moby** (92 joints, class scale 1/6), a 3D character,
  **not a video**. Its sequences are not in the level: `vendor.bin` (toc 0x198, WAD-compressed; 16 offsets then the
  sequences) is read at the menu's first render and relocated into class 12's sequence slots (`FUN_002ade20`); the
  moby is created when the read is done (`InitMobyInstance(0x1ca954, 12)`), placed at vendor rows · (−0.5, −1.8,
  0.2) + position, yaw = the vendor's, lit like the item. Its behaviour (`FUN_002aee20`, `FUN_002af248`): a greeting
  on creation (a random set k = `randi(2)`, voice stream 10000 + 18k + v, v = language − 1, seq 1 then seq 3k+4 when
  the stream is ready, the voice started at a set frame of the talk sequence), idle seq 2 / 3 / 0 cycling every 600
  frames with an idle line (10000 + 6(3k+1) + v, seq 3k+5), a line on a cursor move (1 in 4, at most every 360
  frames: 10000 + 6(3k+2) + v, seq 3k+6); the voices are the 36 VAGs of `vendor_audio` (toc 0x1a0). [M: the stream
  id → file mapping is id − 10000.]
* **Button window** (3): "✕ Buy" 21044 / "△ Back" 21043 / "✕ Yes" per the buy flow, centred in the rectangle
  (`font_print_window_small`, [M] exact layout).
* **Prompt** (4): "△ Exit" 20192 / "△ No" 21067 centred at (40, 20).
* **Icon strip** (5): the 48×48 icons at x = 12 + 56·i, y 6 (ammo icon frame 2, weapon frame 0), the selection box
  pulsing `((t & 15)·4 − 32 clamped) + 0x40`; ≥ 8 entries: the carousel.
* **Price and bolt icon**: the price is the item panel's text; the bolt icon beside it is the HUD's bolt counter
  (slot 2, pinned) [M: the screenshot's small bolt next to "1" on the panel is part of the panel art or the HUD].

**The item hologram** (above the pad, drawn before the vendor): an ammo entry with a class in 0x1c94a0[item]
(e.g. Pyrocitor ammo → class **214**, the canister; Bomb Glove ammo → 226) draws **+0x200** at vendor rows ·
((−1.0, 0.05, 1.3) + (0, 0, sin(φ)/20) + 0x1c9490[item]) + position, rotation (0x1c9480[item].xy, φ); a weapon
draws **+0x300** (the item's class) at the same base + 0x1c9dd0[item] with rotation (0x1c9dc0[item].xy, φ), then
shifted by 0x1c9db0[item] rotated by its own matrix. φ = 0x1622a4 += 0.05 per frame (the spin), sin(φ)/20 the bob.
+0x300 gets +0x334 |= 0x100 [M: the hologram draw style]. Light set 14, ambient 0x202020.
**The cone** (`VendorDrawHologramCone` 0x2b3cc8): 4 quads over 9 vertices 0x1ca800 (vendor space: x 0; y ±0.6 / ±0.2
/ ±0.05 at z 1.6 / 0.9..1.0 / 0.67), UVs 0x1ca890 (a 3×3 grid) with V + 0x161184 (+0.01 per frame, wrapped at 1), vertex
alpha 0x10 at the top row and 0x60 below (RGB 0x80), FX texture 0x18, ALPHA 0x4000000044 (additive) [M: texture and
blend from menus.md §4].
**The spinning class-13 mobys** +0 / +0x500 at (−10.2, −4.25, 0) / (−9.5, −5.85, 0.85) in vendor space, x rotation
−0.05 per frame, y −0.36, z +0.05: +0 is drawn in the item panel's target (the backdrop behind the item).

### 9.4 Input and sounds (class 11 sound table, `PlayClassSound(n, 0, vendor)`)

| sound | when |
|---|---|
| 3 open | `OpenVendorMenu` (the △ tick) |
| 4 screens on | substate 0 → 1 (frame 40) |
| 1 cursor | Left / Right edge that moves the selection (`0x13cb04` against the last frame's `gp−0x4950`); in the quantity popup each step (auto-repeat: held > 15 frames every 8th, > 47 every frame; the sound on every 4th repeat) |
| 0 select | ✕ on an entry (buy flow 1), a cancel (△ in the popup), a failed purchase |
| 2 denied | the popup opens on an entry that cannot be bought (maxed out / can't afford) |
| 7 purchase | a purchase (ammo or weapon), with ticker 20319 "THANK YOU" |
| 5 exit | △ in the menu (not while a voice line is being requested, `gp−0x5a74`) |
| 6 closed | `VendorExit` after the leave |

Denied texts in the popup: "You're maxed out!" 21046, "You can't afford it!" 21048; the ticker 20320 on a purchase
short of bolts. The popup is **class 0x471** (1137: 5 joints, seq 0 = 4 frames open, seq 1 idle) at the vendor's
position and rotation, lit with ambient 0x101010 and light set 14, its text drawn into a target (screen 6:
`FUN_002b2848`, the static with FX 0x19) mapped on its joints; the buy flow waits for its animation: open seq 0
(4 frames), ✕ / △ → seq 0 from the last frame backwards (blend 8 / 1), then the purchase when it has closed.

### 9.5 Closing (substate 2, `VendorExit` 0x2ae660)

△ → 0x162298 = 1, 0x162294 = 8, sound 5; the screens power off over those 8 frames. At 0: the vendor **seq 4**
(10 frames: the fold) from frame 9 blended over 8 at speed −0.5 · 0x15ed60 (backwards at half speed), the camera
view restored from its copy (0x166400), substate 2. **Substate 2** (40 frames): the world runs again (as substate 0),
the glass quads while ≤ 36; then seq 1 (blend 8), the manipulators detached, and (no weapon demo) `VendorExit(0)`:
the HUD slots released, `SetState(0, 1)`, Ratchet shown, **`HeroTeleport`** to vendor rows · (3.5, 0, 0) +
position facing the vendor (yaw + π), **`CameraScript2(2)`**: the follow camera **blends** back from the vendor view
(rates 0.018), the vendor's state 1, **class sound 6**, sound group 0x1d resumed, **`music_Unpause`**. A weapon
bought with a demo scene (0x1ca4a0[item] ≥ 0: Pyrocitor 5, …) plays that scene first (substate 3,
`VendorStartWeaponDemo` 0x2ae7f8) and returns through `FUN_002aecf0`.

Timing (NTSC frames): open = 1 (△ tick) + 4 (FadeToBlack) + 40 = 45 frames to a usable menu, of which the last
37 show the unfolding vendor; close = 8 (power-off) + 40 (fold, world running) = 48 frames, then the camera blend.

### 9.6 Why the port was slow and silent (2026-09-28, before this rebuild)

* **Silent**: mode 5 did not run the EE audio frame. Class sounds are queued in the sound slots by
  `PlayClassSound` and only become 989snd plays in `sound_update` (`AudioSystem::game_frame_with`), which ran in the
  gameplay tick's sound step; mode 5 skips the tick, so `run_audio` filled those frames with IOP-only frames
  (`render`) and no vendor sound ever started (the queued slots were issued, late, after the exit).
* **Sluggish**: the frame counts matched (4 + 40 open, 8 + 40 close), but the port froze the world and did not
  play the vendor's seq 2 / 4, so the 40-frame fly-in and the 40-frame leave were a still picture (and the leave
  a still picture with the screens already gone) followed by a hard camera cut: 80 frames with nothing moving read
  as lag. No frame hitch or load was involved (the tables load at the level's first tick, not at the open).
* **Measured after the rebuild** (realtime run, `RC_INTERACT_TRACE=1`, wall clock from the △ tick): substate 0 from
  63 ms (the 4 fade frames), the menu at 747 ms (frame 144 = △ + 44; the game: 45 frames incl. the △ frame = 733 ms);
  △ in the menu → mode 0 in 48 frames (≈ 800 ms; substate 2 at +7, the exit at +47). Headless
  (`tests/ui/interaction_vendor.rs`): open → menu 44 frames, △ → mode 0 48 frames, with the world ticking on exactly
  the 40 + 40 frames of substates 0 / 2.

## 10. The vendor in the port (rebuild, 2026-09-28)

**Game logic** (`crates/rc-game/src/menus/vendor.rs` + `vendor/{layout,salesman,screens}.rs`):
* `Vendor::open` / `frame` follow §9.2–§9.5 frame for frame: `FadeToBlack(4)` (`pre_fade`), substate 0's 40 world
  frames with the fade and the unfold, the screens' power-on (`power()`), the menu, the buy flow timed by the
  popup moby's own animation (class 0x471), the 8-frame power-off, the fold and substate 2's 40 world frames, the
  exit. The vendor's animation is asked for as `VendorAnim` requests (hard cut seq 2 at speed 0.5, blend seq 3,
  blend seq 4 from frame 9 at −0.5, blend seq 1); `world_runs()` / `screens_shown()` / `glass_only()` say what the
  frame does.
* `layout::VendorLayout` reads every placement constant by its level-01 label (mapped on other levels by
  `Overlay::relocated`: `DATA_LABELS`).
* `screens`: the screen quads from three joint points and the margins (`Quad`), the shrink, the projection
  (`View`, `FUN_002adc38`), `place`, the glass quads, the static (`Statics::step`, `FUN_002b2cd8`). The content per
  screen is `Vendor::screen_content` (target pixels); `Vendor::render_screens` is the render's per-frame part on the
  game's stream (the ticker's line choice, the static's draws; it also starts the salesman's data read).
* `salesman::Salesman`: `FUN_002aee20` / `FUN_002af248` (the greeting, idle cycle, remarks on a cursor move, the
  purchase reset, the voice started at the talk sequence's key time `MobyAnimKeyTime`).
* `Vendor::scene`: the placements of the item model (+0x100), the class-13 backdrop (+0), the salesman, the item
  hologram (+0x200 / +0x300) and the popup.
* The class (`classes/vendor.rs`): △ also calls `SetState(100)` through the hero-call channel (as `OpenVendorMenu`
  does inside the moby loop); `hologram()` exposes the logo's size and visibility for the renderer.

**Engine** (`crates/rc-engine/src/interact_render.rs`, `vendor_render.rs`, `screen_canvas.rs`):
* Mode 5 runs `Vendor::frame` once per frame (crate::menu_render), applies the animation requests to the vendor moby
  and, in the menu, advances it; the world frames run the gameplay tick in its scene form (`Game::camera_paused`,
  Ratchet in state 100) through `MenuMode::world_tick`; the frames without a tick run the audio's EE frame
  (`sound_update`), so every class sound plays when the game plays it; the music is paused from the open to the exit;
  the salesman's lines play on the dialogue voice (`vendor_audio/NNN.bin`, NNN = id − 10000); the HUD slots are
  emptied at the open (`FUN_0024fb00`, `HudFeed::reset`); at the exit Ratchet is teleported 3.5 in front of the
  vendor (state 0) and the follow camera blends back from the vendor view (`CameraScript` mode 1 then
  `CameraScript2(2)` on the next tick).
* crate::vendor_render draws the approach logo (class 1143 with its chrome pass and its two spinning joints) and
  beam, the menu's cone, the item hologram, the popup, the screens as HUD-pass primitives (black target, content,
  static; squeezed while powering on / off) and the glass quads; the static (and after it the glass quads) goes to
  the HUD's static layer (`Hud2dHook::statics`, over the canvases): its primitives wrap their texels (`Prim::repeat`,
  the 2D pass' flag 2) as the game's CLAMP_1 = 0 does, and the noise is added (ALPHA_1 0x68, FIX = its alpha).
  **Fixed 2026-09-28**: the static's alpha used `max(c − 0x80, 0)` where the game takes `abs` (0x221110), so bursts
  and scan bars started at full strength instead of fading in (and the ticker's resting noise was 0x80, not 0x30);
  and the static was drawn with the pass' inherited CLAMP, so every texel past the textures' last row / column repeated it: the ticker's
  always-running noise showed as long lines from the top-left corner (a 32×32 noise patch when both random offsets
  fell under 32, else rows or columns stretched across the screen) or nothing, and a scan bar (FX 0x1c's last row,
  opaque grey) covered its screen with a flat grey for its 255 frames; now the noise speckles and the bar's scan
  lines roll, as in the game; the item panel's and the salesman's 3D parts are
  crate::screen_canvas canvases.
* **crate::screen_canvas** (general render-to-texture): `Canvases::create(commands, images, name) → CanvasId`,
  `layer(id)` (the `RenderLayers` for the entities it shows), `show(id, Some(CanvasView { rect, focal, centre, clear
  }))` per frame (None hides it), `CanvasView::from_target(size, tan, texels, rect, clear)` for a game render target.
  A window-sized image rendered by a `Camera3d` carrying the main transform, a `GameProjection` with the canvas'
  focal lengths and a `SubCameraView` for its principal point; a UI node shows the rectangle, **under** the HUD
  composite (`create`: the vendor's screens, drawn in the 3D pass before the HUD) or **over** it
  (`create_over_hud`: a page menu's 3D widget, whose render target `PageMenuDraw` 0x28d080 copies after the panel's
  navy rect; docs/plan/gadgets.md §3). `show_exact(id, view, (tan_x, tan_y))` gives the projection tangents exactly
  (the game camera's own, bit-identical; `half / (half / tan)` does not always round-trip in `f32`). [M: the vendor's two 3D screens use the frame's own projection: the item and the salesman are placed in
  the world where that view shows them over their monitors; the render target's zoom argument 1.0 would put them at
  the targets' edges at a quarter of the size the game shows.]

**Still not ported**: the weapon demo scenes (substate 3), the PDA's remote vendor presentation, the popup's
ammo-quantity backdrop (+0x500 drawn in the popup), the approach beam's scan plane (FX 0x1b, the callback's second
quad), the sound group 0x1d pause, the world snapshot (the port draws the frozen world live).
