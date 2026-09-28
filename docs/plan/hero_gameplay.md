# Hero-facing gameplay: crates, pickups, health, ammo, the weapon arm

Audit of the hero-facing gameplay systems (what the player picks up, spends and sees), with the game addresses
(level01 unless marked), the port status and the plan; then what was built on 2026-09-28 (§2–§5). Companion of
`hero_states.md` (the states), `moby_update_catalogue.md` (the crates and bolts), `hud_text.md` (the HUD slots),
`interaction.md` / `menus.md` (the vendor), `particles.md` and `moby_animation.md` §6.4 (pose layers).

Confidence: **H** read from the decompiler C / disassembly of the function itself; **M** from the function and its
callers; **L** inferred.

## 1. Audit

Status: **P** ported, **P\*** ported in this pass, **part** partly, **–** not ported.

### 1.1 Crates

| system | game | status | notes / plan |
|---|---|---|---|
| Bolt crate 500 (and 502 reinforced, 505 TNT): break → `SetDeathBits` → `BoltBurst` → `BoltSpawn` | `CrateUpdate` 0x2ea178, `CrateDropBolts` 0x2eb498, 0x26c250, 0x275988, 0x2bcdb8 | P | `classes/crate_.rs`. A spawner-counted dropper (+0xb1 ≥ 0) pays at least 1 bolt even with +0xb4 = 0 (H). |
| **Nanotech crate 501** | same update; init `CrateSpawnIconMoby` 0x300528 → class 806 | P (crate) / **P\*** (806) | The catalogue's names were swapped (H, from `CrateDropBolts` and 0x300de0): **501 is the nanotech crate**, its drop only clears +0xb4 and sets the death bits; the health comes from the cluster 806. The "ammo-crate death gate" (`MissionState::ammo_crate_gate`, pvar +0xf8) is therefore the *nanotech* crates appearing after N deaths to a mission's enemies. |
| **Ammo crate 511** (init: pvar +0xc6 = 100) | `CrateDropBolts` ammo branch, `0x26bff8`, `0x2daf10` | P (drop) / **P\*** (pickup) | 1 (4 in 5) or 2 pickups of pvar +0xcb's item, re-picked among the owned vendor-list items weighted to those below max. |
| Multi-hit crates 0x1fb–0x1fd, crates 503/504/506–510, crates on moving platforms | `FUN_0026f378`, `FUN_00275290` | – | None on Novalis (level01 maps only 500/501/502/505/511 to 0x2ea178). Port when a level needs them. |
| Crate respawn (planet 0x12, pvar +0xc8) | crate state 6 | P | Reads the ammo tables (now filled, §2.3). |

### 1.2 Pickups

| system | game | status | notes / plan |
|---|---|---|---|
| Bolts 13–16: idle spin / glint, fall, settle, fly-to-hero, collect, despawn | `BoltUpdate` 0x2bb758, `CollectBolt` 0x2bc4f0 | P | Complete for Novalis. The bolt grabber (item 34, `0x13d4e2`) widens the pickup volume to 12 / 4.5: `Services::bolt_grabber` is now synced from the owned table (§2.3). |
| **Ammo pickups** (classes 204, 213, 214, 222, 223, 225, 226, 1006, 1438, 1447, 1449) | `AmmoPickupUpdate` 0x2db028 (Lombyte `FxDebrisGroupUpdateB`), states via the jump table 0x20ace0 | **P\*** | §2.1. |
| **Nanotech orbs** (cluster 806 + particle type 62, healing ring type 59, glints type 60) | `NanotechUpdate` 0x300de0, 0x300c40, 0x300cf8, 0x300d70, 0x3005a8, 0x300900, 0x300808; `PartType62Spawn` 0x288bb0 / update 0x288ca8; `PartType59Spawn` 0x288410 / 0x288530 | **P\*** | §2.2. The cluster's draw callback 0x301c00 (a glowing mesh drawn straight to the GS: 3 × `FUN_0021fda8` fans + 32 quads over effect textures) is **not ported** (counted `nanotech draw callback 0x301c00`). |
| The pickup banner (`ShowBannerf(text, n, −1)`: "+n bombs") | 0x278a50 | **P\*** | `HudState::show_bannerf`; texts from the item records +0x34 / +0x36. |
| Gold bolt 1134 (bob / spin, glow, glints, pickup cutaway) | 0x307ca0 | **P\*** | §6 (2026-09-28). |
| Infobot 750 (paths, pickup, scene → movie → scene, planet unlock) | 0x2fbf80 | **P\*** | §6. On Novalis the Infobot is the Water Pump Worker's talk-table sale. |
| Gold-weapon offers 304 | 0x2e1ac0 | P | (dialog not ported). |

### 1.3 Health

| system | game | status | notes / plan |
|---|---|---|---|
| HP 0x1415f8, max HP 0x15eda0 (4; 5 / 8 with Premium / Ultra Nanotech), HP = max at the hero init / death reload | hero init 0x226b70 | P | `Hero::health`, `GameState::global.max_hp`; the moby loop reads max HP from `GameCounters::max_hp` (synced each tick). |
| **Heal**: each nanotech cluster heals **1 HP** when its first orb reaches Ratchet (HP + clusters already on their way < max HP; 4 as Clank) | 0x300de0 state 3 | **P\*** | §2.2. Written through `HeroFields::health`. |
| Damage / knockback / death | `damage.rs` (`HeroTakeDamage` 0x226fa8) | P | |
| HUD health: shown by damage (0x226fa8), the select button (0x2aba68), menu close (0x28c6c8) and state transitions; the slot's update 0x24e238 re-arms 120 ticks on any HP change, always shown at 1 HP; init 210 ticks | `HudShowHealth` 0x24a498 | P | The port also queues the slot on a gain [L]: the game does not (the slot stays assigned, and a heal re-arms it only while it is up); kept, it is the result the player sees when the slot was shown at the level start. Verified on screen (§6). |
| Nanotech upgrades (Premium / Ultra, the Orxon seller 1326, records 40 / 41) | talk system | – | Level 10; the purchase must set flags [4]/[5] and max HP 5 / 8. Queue with Orxon. |
| Clank's own health 0x1415fc | | – | With the Clank levels. |

### 1.4 Ammo

| system | game | status | notes / plan |
|---|---|---|---|
| Per-item ammo 0x13d428, max ammo (record +0xe), pickup amount (+0xc) | item records 0x1c4530 | P | Mirrored in `Weapons::ammo` before the tick, copied back after. |
| Use on fire (`0x249450`, used stat 0x13dea0) | | P (Bomb Glove) | The other weapons' fire code is not ported (§1.6). |
| **`AddAmmo` 0x2494d8** from pickups, picked-up stat 0x13de08 | | **P\*** | `pickup::add_ammo`, `HeroFields::ammo` / `ammo_picked`, `GameState::global.ammo_picked_up`. |
| Vendor ammo purchase (AddAmmo, bought stat 0x13dd70) | vendor menu | P | `menus/vendor.rs`. |
| HUD weapon / ammo slot (`%d/%d`, red at 0) | 0x24f9c0, 0x2519c0 | P | Follows the game state, so pickups and purchases show. |
| Ammo in the crates' pick (owned list, max) | 0x26bff8 | **P\*** | The engine's moby loop had an empty inventory (every ammo crate would pick item 10 blind); now `ItemTables` from the price records, the vendor list and Ratchet's mirrors. |

### 1.5 The weapon arm (upper-body layer)

| system | game | status | notes / plan |
|---|---|---|---|
| Pose-layer nodes (6 × 0x40 at 0x18efc0), alloc / free, start, advance | `FUN_00263e08`, `FUN_00263ec8`, `FUN_002641c0`, `FUN_00263f70` | **P\*** | `hero::anim::AnimLayer`. |
| Layer decode and blend in the evaluator (`MobyProc` +0x60 list) and in the chain evaluator | `FUN_00267770`, `MobyAnimEvalChain` 0x268ee8 | **P\*** | `rc_formats::moby_anim::{evaluate_layered, evaluate_chains_layered}`. |
| The weapon draw (arm layer on joint list 12, + list 13 when 0x1413fb = 2), upkeep, put away, holster | `0x22ee08`, `0x22f068`, `0x22efd8`, `0x2405f8` | **P\*** | `hero::weapons`. §2.4. |
| Persistent-arm weapons (item +0x30 ≠ 0: the Blaster-type weapons keep the arm up): layer re-creation, standing pose, idle rules | `0x22eca0`, `0x242858`, idle transitions 0x242930 | **P\*** (§7) | `weapons::{arm_on_state_change, gun_stance, stance_kept, idle_stance}`. The glove-holding layers 0x140050 of `0x22e660` (0x1413fb) are not ported. |
| Comet-Strike (0x15) | `0x2be1c0`, 0x236da0 | P | A full-body state: the wrench code never calls 0x22ee08 (xrefs: HeroPdaGadget and the weapon updates 0x2c7d68, 0x2cd458, 0x2ce448, 0x2d2450, 0x2e4e60, 0x303000). No layer to wire. |

### 1.6 Other hero-facing gaps (listed, not done)

| gap | game | plan |
|---|---|---|
| **Pyrocitor 16** (class 176, 0x2cd458; sold on Novalis): **ported (§7)**. The other weapons: Suck Cannon 11 (157, 0x2c7d68), Devastator 19 (177, 0x2ce448), Blaster 15 (168), Glove of Doom 20, Mine Glove 17, Drone 24, Decoy 25, Visibomb 23 …: their `HAND_ITEMS` rows, fire cases, projectiles and arm sequences | weapon updates listed above, `HeroPdaGadget` 0x240ed8 | One weapon at a time on the arm layer built here; the Pyrocitor first (Novalis vendor). |
| Auto-aim target list 0x1abe80 (mode 0x20 records) and the melee aim assist 0x22e238 | | With the first enemy class that registers a target record. |
| 0x20 (Walloper lunge, item 0x12), 0x21 (wrench rebound off flag-2 targets) | hero_states.md §1.2 | |
| ~~Gold bolt 1134, Infobot 750~~ | 0x307ca0, 0x2fbf80 | Done: §6. |
| The nanotech cluster's glowing mesh (draw callback 0x301c00) | 0x301c00, `FUN_0021fda8` | A small custom renderer (effect textures 0xb and gp−0x4dbc, 290-vertex fans). |
| Cheat 6 (0x15edb6) nanotech extras (0x13f510 timer, pickups at full health) | 0x300de0 | Only with the cheats menu. |
| Nanotech upgrades, Clank's health | §1.3 | With their levels. |

## 2. In the port (2026-09-28)

Code: `crates/rc-game/src/moby_update/classes/pickup.rs` (registry `classes/mod.rs`), `particles/type59.rs`,
`particles/type62.rs`, `hero/anim.rs` (`AnimLayer`, `AnimCtl::{key_rate, eval_chains_with}`), `hero/weapons.rs`,
`rc_formats::moby_anim::{PoseLayer, evaluate_layered, evaluate_chains_layered}`; engine wiring in `gameplay.rs`
(inventory, max HP, bolt grabber, stats, the layered palette) and `hud_render.rs` (the banner). Written on standard
`f32` (the pickups, the layer bookkeeping); the evaluator's layer blend uses the evaluator's existing PS2 float helpers
(consistent with the code around it). RNG draws are the game's, in its order.

### 2.1 Ammo pickups 0x2db028

Pvars +0x00 amount, +0x04 (dropper's +0xc4), +0x06 dim timer, +0x08 item, +0x0c, +0x10 velocity, +0x20 pickup delay
(`ticks(16)`), +0x24 rise speed, +0x28 spawn z. **H** (disassembly + jump table 0x20ace0: 0 → 0x2db798, 1 → 0x2db1ec,
2 → 0x2db4d0, 3 → 0x2db65c, 4 → 0x2db7a4).
* Physics (states 1, 3, 4): moving → `coll_sphere(pos + 0.5 up, 0.4, flags 2)` pushes it out (`CollOutput +0x30`,
  then −0.5); gravity `4.9·dt²` before and after `pos + vel` (the midpoint rule); `CollLine_Fix(pos + 0.1 up → next,
  flags 2)`: a hit lands it (velocity 0, position = hit point).
* State 1: alpha → 0x80 by `128/ticks(30)`; after the delay, at rest: collected when Ratchet's pickup volume (the
  bolts' `0x1415d8` / `0x1415dc`, [`World::bolt_radii`]) holds it and the ammo is below max, or when it fell 2 below its
  spawn height; never with 0 HP. Then `AddAmmo`, the banner (`+0x36` text for 1, else `+0x34`), the picked-up stat,
  state 2, scale ×1.2, rise `8·dt`, sound 0 of class 213 (at most every `ticks(10)`, gp−0x5398). Otherwise alpha
  +1 (≤ 0xff) and, when Ratchet's bounding sphere overlaps its own by 0.2 (`FUN_00265210`: the gap of the ×1024
  spheres), state 3 with `ticks(60)` at +0x06.
* State 2: rise speed → 0 by `24·dt²`; the flight speed `|vel|` → `64·dt` by `24·dt²` toward the body point
  0x13f420; deleted at scale 0 or within one step; scale → class scale × `clamp(d / (ticks(30)·speed + 0.01), 0.3,
  1)` by 10 % of the class scale a tick.
* State 3: alpha pulses to `10 + trunc(5 + 5·sin(2π·(counter % 60)/60 − π))`; back to 1 when the spheres no longer
  overlap or the ammo drops below max. State 4: scale grows 3 % a tick to the class scale, then state 1 with its
  collision.

### 2.2 The nanotech cluster 806 (0x300de0)

**H** for the states and constants (decompiler C + .lit 0x161d60..0x161e5c: turn 4°, bob 0.06 at 2°/tick, radius
0.25 ± 0.15 with the steps 0x1fbe00, orb 60000 / trail 10000, colour 0x7f7f4040, heal ring hues / sizes / heights,
glint colours); **M** for the rotation direction of `FUN_00274ac8` (right-handed axis-angle, a mirror would only
mirror the orbit), see the module doc of `pickup.rs` for the state machine. Particle types: 62 (orbs kind 1 with no
update, trails kind 0 / kind 2 attached through `Particles::anchors`), 59 (the ring, attached to the hero), 60.

### 2.3 The engine's item tables

The moby loop's `World::inventory` was the empty default, so the ammo crates picked blind and the level-18 crate
respawn rule saw no items. Now `pickup::ItemTables` (price records `0x1c4530` from the talk tables, the vendor list
0x15edd0, Ratchet's owned / ammo mirrors); max HP 0x15eda0 into `GameCounters::max_hp`; the bolt grabber flag
(owned item 34 = 0x13d4e2) into `Services::bolt_grabber`; the pickups' ammo and picked-up stat back through
`HeroFields` → `Weapons::{ammo, picked}` → `GameState`.

### 2.4 The weapon arm layer

`0x22ee08` (moving, not standing): for list 12 (and 13 with 0x1413fb = 2) a node with weight 1, key A = Ratchet's
current key B, key B = (`def +0x28`, or `+0x2c` crouched, frame 0) over `ticks(10)` / `ticks(11)`, advanced once. The
upkeep `0x22f068` (first thing in `HeroItemsUpdate`): re-blend over 8 when the item's layer sequence changed, advance
(t += speed · rate; key steps take the new key A's rate; the wrap sets flag 2), weight → 1 by 0.2; on the wrap with
0x1413fa clear: 0x1413f8 = 0, speed 0, fade 0.07 a tick (0.25 after the holster's 0x140064 = 2) and free at 0. The Bomb
Glove: moving sequence 66, crouched 44 (standing throws use the state 0x23 with the full-body 0x2c). The evaluator
blends each listed joint `q = nlerp_flip(q, q_layer, 1 − w, w)`, the translation (the layer's rest where its keys have
none) and a layer inherited scale by `w` (§6.4), in `MobyProc` and in `MobyAnimEvalChain` (the hand item follows the
arm: `AnimCtl::eval_chains_with` in the hand attach and the wrench's hand point). The main animation is untouched.

## 3. Tests

* Unit: `particles::type62::tests`, `particles::type59::tests`, `pickup::tests` (the cubic, the rotation sense, the
  item tables), the existing `weapons::tests` (the arm while running, now on the layer).
* `crates/rc-game/tests/hero_gameplay_novalis.rs` (full ticks: the scheduler, particles, moby collision, the hero with
  the wrench and the glove, the hit path): the bolt crate 376 pays bolts; the nanotech crate 533 (on 377) shows 8 orbs,
  heals 2 → 3 and 3 → 4, and does nothing at 4 / 4 (the free cluster waits); the ammo crate 552 fills the glove 38 → 40
  (capped) and leaves its pickup at 40 / 40; a glove throw while running at the spawn plays sequence 66 on the arm
  layer at full weight while the hero stays in the run state on the walk / run animation, uses one bomb, fades and
  frees the layer; the layered palette differs from the plain one only in the arm list's joints and their children.
  Every scenario runs twice with identical rows (RNG included).
* Guards: `novalis_hero_digest` unchanged (the digest cuts the hero dump before `owned`; no hero state or RNG changes
  on those scripts); `cargo test --workspace` green.

## 4. Verified vs inferred

* Verified (H): the crate identities (501 nanotech, 511 ammo), every state and constant of 0x2db028 and 0x300de0,
  `AddAmmo`, the pickup volume shared with the bolts, the heal of 1 per cluster up to max HP (4 as Clank), the layer
  node layout and its start / advance / upkeep / fade rules, the layer joints = the second byte list of Ratchet's
  joint list 12 (count byte `pb[2]`), both evaluators applying the +0x60 list.
* Inferred: the quaternion rotation sense of `FUN_00274ac8` (M); the layer's scale channel default (L: the decode
  fills 0x7f7f; a layer without an inherited scale leaves the main one); the HUD health re-show on a gain (the port's
  extra queue, [L] as before).

## 6. Gold bolts and Infobots (2026-09-28)

Code: `crates/rc-game/src/moby_update/classes/gold_bolt.rs` (1134, `GoldBoltUpdate` 0x307ca0 with 0x308380 / 0x308470 /
0x308550 / 0x3087e0 / 0x3089f0), `classes/infobot.rs` (750, `InfobotUpdate` 0x2fbf80 with 0x2fbb20 / 0x2fcd88 /
0x2fce68); the module docs carry the pvar layouts and state machines. Both are registered by class number and run on
every level whose class table names the same function (`LevelPorts`): the gold bolt on levels 1–18 (17 overlays byte
for byte, Gemlik's up to relocations), the infobot on 0, 1, 3–8, 10, 12–15, 17 (checked by
`tests/gold_bolt_infobot_novalis.rs::registered_on_the_levels_with_identical_code`). Placement comes from each
level's instances: 40 gold bolts on the disc (none on 00), pvar +0x00 = the index in the level's 4 bytes.

**Reused**: the script camera and the cinematic call layer (`CameraScript` mode 1 snap + targets, `CameraScript2(0)`,
`HeroTeleport(…, 0x72)`, the letterbox 0x15f404), the hero hold state 0x72 and `HeroCall::SetAnim`, particle types 59
(glow) and 60 (glint) with the nanotech ring's glow pattern, `creature::{turn::spring_turn, ground::key_time}`,
`spline::nearest` (the infobot's path aim), the checkpoint record, the talk system (the Novalis Infobot sale), the
scene / movie hand-offs, the HUD banner (`HudState::show_banner`), `GameState::unlock_planet`, the saved-game write
channel `interact::GameWrite`. **New**: the two classes; `cinematic::{fade_to_black, show_banner, show_planet_banner,
unlock_planet, save}` with `Cinematic::banner` (`ShowBanner` / `ShowPlanetBanner` 0x277c38, message table level01
0x20a0c0 = `PLANET_BANNERS`, `ticks(1180)`); `EngineRequest::FadeToBlack` and the engine's `scene_render::FadeHold`
(the blocking `FadeToBlack(n)` 0x21b438 of a gameplay class: the n frames after the asking tick fade the last view to
black, tick suspended); `HudState::show_banner_msg`; the hand-item hide 0x1413ff (`Hero::f13ff`, set through
`HeroFields::hide_hand` after the hero calls, cleared by `SetState` on foot, read by `moby_attach`).

**Game state (in memory, laid out as the save).** There is no save/load flow, menu or travel yet; everything goes into
`GameState` (Persistent), in the save chunks' own layout, so a future save writer only serialises it:

| field | game address | save chunk | written by |
|---|---|---|---|
| `levels[l].gold_bolts[i]` (u8, 1 = collected) | 0x14bec0 + l·4 + i | 3003 (level slot) | gold bolt pickup: `GameWrite::GoldBolt { level, index }` |
| gold bolt count | Σ of those bytes | – | derived (`TalkGame::gold_bolts`; the gold-weapon offers' condition 6 spends it: `spendable_gold_bolts`) |
| `global.planet_unlocked[p]` | 0x13dd40 | 14 | `cinematic::unlock_planet` → `GameWrite::UnlockPlanet` → `GameState::unlock_planet` |
| `global.map_order[n]` | 0x13d510 | 20 | same (appended) |
| `levels[l].missions[m]` (0xff done) | 0x14c050 | 3004 | `SetMissionDone` (infobot +0xb0, talkers, mission NPC) |
| `global.landmarks[slot].flags` (talked) | 0x13d5bc | 15 | talk system; the infobot's `FUN_0027b438(1)` on its path start |
| `global.bolts` | 0x15ed98 | 7 | the purchase (500 for the Novalis Infobot) |

The moby loop reads the mirror `TalkGame` (`gold_bolt_bits`, `planet_unlocked`, …, synced before every tick and
before the load pass), so "stays collected" holds for the session: a revisit or reload runs the bolt's init, which
deletes a collected one. `memcard_Save` is logged (`EngineRequest::Save`). Rewards: the gold-weapon offers (class 304,
kind 6 nodes) already read the count; spending is the offer's existing `GoldWeapon` write.

**The flows (level01).**
* Gold bolt: idle (bob 1 ± 0.3, spins 120 / 40 / 20 °/s, 4 soft glows, glints 1 in 40) → Ratchet within 3 (xy) / 2
  (z) → `FadeToBlack(ticks(10))`, Ratchet 2.5 in front facing it (0x72), sound 0, the camera snapped beside it, the
  bolt on Ratchet (sequence 1), his animation 0x82, hand item hidden, letterbox → the camera orbits 40° about him
  (or moves between the two cuboids of pvar +4 / +8) while sequence 1 plays → 60 ticks → banner 21427 "Gold Bolt
  acquired" (180 ticks), the byte, control back, `CameraScript2(0)`, save, deleted. pvar +0x0c ≠ 0 (one bolt on
  level 16): banner, byte, save at once.
* Infobot: see `infobot.rs`; state 8 = `UnlockPlanet(+4)` + `SetMissionDone` + checkpoint + scene +0xc → movie +0xd →
  scene +0xe (each after the last ends) → save + `ShowPlanetBanner(+4)`. Novalis' instance 874 is inert (hidden, no
  path, no scenes). **Novalis' Infobot** is the Water Pump Worker 774's: "△ Buy Infobot for 500 bolts" (talk kind 1,
  item 37, purchase + chain) → scene 1 → movie 2 (`mpegs[4]`) → scene 2 → `UnlockPlanet(2)` + `ShowPlanetBanner(2)`
  ("Infobot for Planet Aridia acquired", 1180 ticks), the worker leaves. The mission NPC's Kerwan reward
  (`UnlockPlanet(3)` after scene 3 → movie 3 → scene 4) now goes the same way.

**Tests** (`crates/rc-game/tests/gold_bolt_infobot_novalis.rs`, headless, game state of a first arrival): both
Novalis camera variants (959 orbit, 958 cuboids 0x51 → 0x50): picked up on the tick Ratchet is in reach, idle bob
within spawn + 1 ± 0.3, FadeToBlack(10), 0x72 + animation 0x82 + hand hidden, bolt on him, letterbox and script
camera for the whole 398-tick cutaway, then control back, banner 21427 / 180, save request, count +1, chunk-3003 byte
set, glows and glints spawned; a revisit with that state has no bolt 0 (the others idle); determinism; the Infobot
purchase: hand-offs scene 0, scene 1, movie 2, scene 2 in order, bolts 5000 → 4500, planet 2 unlocked and in the map
order, the planet banner; the Novalis 750 inert; the per-level registry. Updated: `hero_gameplay_novalis`'s ammo-crate
check no longer assumes one pickup (the crate's `randi(5)` moves with the RNG stream now that the gold bolts draw).

**Engine** (frame-exact, `RC_SCENE=0`, scratchpad `goldbolt_infobot/`): `RC_HERO_AT=244.05,107.90,87.5,-2.101
"RC_PLAY_SCRIPT=40-80:stick 0 -1"`: idle glow (frame 50), pickup on frame 89, fade frames 89–98 (94: the last view
at ~60 % black), cutaway (150: the bolt over Ratchet, bars, no HUD; 300: he holds it, no wrench), banner "Gold Bolt
acquired" with control back (520); two runs identical (frames 150, 520). Infobot: `RC_HERO_AT=253.13,186.67,95.57,
2.5066 RC_GIVE_BOLTS=5000 "RC_PLAY_SCRIPT=2470-2472:press TRIANGLE" RC_MOVIE_SKIP=120`: scene 0 → △ at 2470 →
scene 1 (2473) → movie 2 (3336, holofilm) → scene 2 (3462) → UnlockPlanet(2) at tick 4207, the banner "Infobot for
Planet Aridia acquired" with 4500 bolts (4420).

**Verified vs inferred.** [H] every state, constant (.lit 0x161f10..0x161f74, 0x161c80) and call order of the two
updates and their helpers; the planet banner table and `ticks(1180)`; `ShowBanner(−1)` = `ticks(180)`; the Novalis
data (surveyed). [M] level 13's gold bolt = the port (relocation match, not byte identical); `DialogStreamStart` /
`DialogStreamUpdate` taking the pvar bytes +0xc / +0xd / +0xe (argument registers lost in the decompile, menus.md
§6). Port choices: the fade hold shows the last view with the asking tick's world under it (Ratchet already
teleported, the bolt on him) and holds its HUD / bars; the infobot ride's platform-matrix carry (0x2752c0 with a
mode-0x20 ride of class 822) is counted, not ported; the gold bolt's cheat mirror (0x15edb5) and control mode 3's
`FUN_00231450` are not ported. Not engine-checked: other levels' gold bolts / infobots (headless smoke and registry
only), the infobot's path / ride states (no Novalis instance uses them).

## 7. Joint modifiers, the Pyrocitor, the scene body (2026-09-28)

**The runtime joint-modifier list (moby +0x64).** [H: `MobyProc` 0x267fc0 at 0x268b00, `MobyAnimEvalChain` 0x268ee8,
`AttachManipulator` 0x264370, `DetachManipulator` 0x2643e8, `FUN_00227050`, `FUN_00227590`; RAM at the Novalis idle
savestate] A node (`u8 mode +3, target record +4, next +8, f32 weight +0xc, quat +0x10, scale +0x20, trans +0x30`) is
linked in front of the list for one of the class's joint lists; it acts on that list's **second byte list's first
joint** (`pb[pb[0] + 4]`). After the pose layers: mode 0 `q ← q ⊗ q_m`, scale xyz `∘ s_m` (its presence word kept;
the chain evaluator marks it present), translation xyz `+ t_m`; mode ≠ 0 a weighted nlerp (and lerps) toward the node.
MobyProc skips nodes at or past the class's joint count. Ratchet's users on foot:
* **the joint records 0x17ab00** (31 × 0xb0; `0x2273d0` → `0x227050` every idle update): a record whose targets / scale
  are set or whose angles have not settled is (re)attached, mode 0, quaternion `FUN_0026ee30` from its Euler angles
  (`FUN_00221e38` = rotation by −a about one axis, composed x ⊗ y ⊗ z), scale = the tick's +0xac; otherwise detached.
  Their writers: the head look (records 1 / 3 / 12), the idle secondaries (13..16), record 17's head scale (0.92),
  and **`HeroLean` 0x235638** (records 0..3, 13..16 from the turn residual 0x13f4d8: walk / run, fall and jumps, the
  pack glide 8 and the Thruster hover 0x81; its spring constants for records 0..3);
* **the blink's eyelid nodes 0x140080** (7 × 0x40, `0x227590`): lists 15..21 (0x17c640), mode 1, fixed poses
  (0x17c4c0 / 0x17c530 / 0x17c5a0), weight `BLINK[frame]`, all attached during a blink (node 0 first, so node 6 heads the
  list), all detached at its end.
In the port: `rc_formats::moby_anim::{JointModifier, evaluate_posed, evaluate_chains_posed, list_target}` (native `f32`
on the record values), `Moby::joint_mods` (the list, written by the owner; every pose evaluation reads it: the palette,
the dynamic mobys, the shadows, the hand / back attachments), `hero::idle::{Manip, Idle::manips, Idle::modifiers,
BLINK_NODES, axis_quat, euler_quat, Hero::lean}`, `Hero::joint_targets` (the loader's list targets) and the
write-back. **RAM match** (`shadow_volume_novalis.rs`, the fixture now carries the savestate's 11 nodes): the port's
nodes equal RAM's (quaternions to 2e-6), and **all 21 posed shadow records** match to **3.05e-5** units (the 10 under
the list 1.53e-5; 0.178 without the list), radii 1.5e-8. Not ported: the swim lean (0x2370b8 cases 0x31, 0x33..0x35),
the Magneboots aim lean `0x2352e0`, the pack record 18 `0x235e60`, `HeroScanTargets`' look target, the cheats' big
head `0x24a1d0`, Clank's eyelids `0x2278c0` (Clank's pose has no modifier input yet), and the NPCs' own manipulators
(vendor 11, talking NPC 774: their classes compute the angles; `Moby::joint_mods` is where they would go).

**The Pyrocitor** (item 16; `hero::pyrocitor`, the `HAND_ITEMS` row; module doc for the whole update) [H: disassembly
of 0x2cd458, 0x2cde98, 0x2ce0a0, `PartType12Spawn` 0x280138 / `PartType12Update` 0x280408, 0x2d0fc8, 0x2d1068]: no case
in `HeroPdaGadget`; its update fires on ○ held: `0x22ee08` (the stance 51, full body; def +0x28 = −1, so no moving arm
layer: running, Ratchet keeps his run and the flame leaves the hand along his facing), flames (particle type 12,
`particles::type12`: 2 glow puffs + 2 flames a tick, 12 units, cut 0.75 before walls), the kept flames' hit spheres
and embers (type 2), two nozzle spheres, one ammo per 10 ticks, the flickering point light (7.5), the loop sound, the
pilot flame (class 179, `classes::pyro_glow`), first person from below the eye along the view. The persistent-arm rules
(def +0x30): `0x242858` (a walk / stop SetState or the walk's stop turns into the stance), `0x22eca0` (arm layers when
leaving idle), the idle keep / stance, `0x22efd8`'s return to idle. The hand item is not a table moby in the port:
hits are Ratchet's, the pilot flame reads its owner's state from pvars; an item update's standing `SetAnim` is made
right after the slot loop (`Weapons::pending_*`). Gold weapons (0x13e520) are not mirrored (0). Not ported: the hold
statistics / "hold ○" help (`0x2cde98`), the moby texture scroll of class 179, the grind aim rows (0x22f068 group 0xf).

**The scene body 99 / 100** (`hero::scripted`) [H: SetState case 99/100, `0x2370b8` case 99/100, no `0x242930` case]:
group 0x18, 0x1413fc = 1, the weapon put away, with `play` the idle sequence on eased curve 1; the physics zeroes the
velocity; nothing leaves it but `SetState(0, 1)`. The gameplay Ratchet is never visible in it (the scene hides him and
his items, `FUN_002486c0`, and draws its own actor; the vendor hides him too), so this is all of it.

**Tests**: `moby_anim::modifier_tests` (3), `idle::tests::{modifier_list_order_and_blink, euler_node_quaternions,
running_lean_targets}`, `type12::tests` (3), `pyrocitor::tests`, `pyro_glow::tests`, `scripted::tests`,
`shadow_volume_novalis` (the RAM match), `hero_pyrocitor_novalis.rs` (the purchase's `GiveItem`, standing / running
at a crate (broken by the flames; not without them) / first person, each twice identical). `novalis_hero_digest`: with
`RC_HERO_DIGEST_NO_IDLE=1` (the idle block cut) byte-identical to before; the full digest differs only in the idle
block (the new records 0 / 2, the list, the lean's targets and angles while moving).
