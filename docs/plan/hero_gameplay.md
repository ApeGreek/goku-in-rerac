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
| Gold bolt 1134 (bob / spin, pickup cutscene) | 0x307ca0 | – | Needs the pickup cutaway (hero anim 0x82, camera script). Queue. |
| Infobot / item pickup 750 | 0x2fbf80 | – | Hidden until activated, flies a path to Ratchet, unlocks an item. Queue. |
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
| Persistent-arm weapons (item +0x30 ≠ 0: the Blaster-type weapons keep the arm up): layer re-creation, standing pose, idle rules | `0x22eca0`, `0x242858`, idle transitions 0x242930, 0x22e660 | – | No ported item has +0x30 ≠ 0. Port with the first such weapon. |
| Comet-Strike (0x15) | `0x2be1c0`, 0x236da0 | P | A full-body state: the wrench code never calls 0x22ee08 (xrefs: HeroPdaGadget and the weapon updates 0x2c7d68, 0x2cd458, 0x2ce448, 0x2d2450, 0x2e4e60, 0x303000). No layer to wire. |

### 1.6 Other hero-facing gaps (listed, not done)

| gap | game | plan |
|---|---|---|
| The other weapons: Pyrocitor 16 (class 176, 0x2cd458; sold on Novalis), Suck Cannon 11 (157, 0x2c7d68), Devastator 19 (177, 0x2ce448), Blaster 15 (168), Glove of Doom 20, Mine Glove 17, Drone 24, Decoy 25, Visibomb 23 …: their `HAND_ITEMS` rows, fire cases, projectiles and arm sequences | weapon updates listed above, `HeroPdaGadget` 0x240ed8 | One weapon at a time on the arm layer built here; the Pyrocitor first (Novalis vendor). |
| Auto-aim target list 0x1abe80 (mode 0x20 records) and the melee aim assist 0x22e238 | | With the first enemy class that registers a target record. |
| 0x20 (Walloper lunge, item 0x12), 0x21 (wrench rebound off flag-2 targets) | hero_states.md §1.2 | |
| Gold bolt 1134, Infobot 750 | 0x307ca0, 0x2fbf80 | Pickup cutscenes. |
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
