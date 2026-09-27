# First PCSX2 comparison: Novalis spawn savestate (2026-09-26)

Input: `~/Library/Application Support/PCSX2/sstates/SCUS-97199 (CE4933D0).01.p2s` (PCSX2 v2.8.2, save version
0x9a590000, EE RAM 32 MiB, zstd). Fresh game, Veldin played, first arrival on Novalis, after the arrival scene,
standing at the spawn. Kept as `~/PS2/ratchet1/savestates/novalis_spawn.p2s` with `novalis_spawn.png` (PCSX2's
screenshot: Ratchet at the spawn, the landing pad on the right is **empty**); its dumps are in `work/trace/`
(git-ignored): `novalis_spawn_ee.bin` (EE RAM), `novalis_spawn_spr.bin` (scratchpad). The facts the regression test
needs are distilled into `tools/trace/tests/fixtures/novalis_spawn.tsv` (numbers only; `distill-spawn`).

Commands (docs/plan/trace_harness.md §5):
```
cargo run -p rc-trace -- compare-tfrag-light   --state latest --level 01
cargo run -p rc-trace -- compare-novalis-spawn --state latest            # a–g below, exit 2 on any mismatch
cargo run -p rc-trace -- compare-novalis-spawn --ee work/trace/novalis_spawn_ee.bin --load-pre-draws 1 --load-emitters-culled
```
Reports: `work/trace/novalis_spawn_report.txt`, moby mismatches `novalis_spawn_mobys.csv`, tfrag
`level01_tfrag_light_mismatches.csv` (not written: no mismatch).

**Classification policy.** The port must *behave* like the game with native systems; bit-exactness is a diagnostic.
Every mismatch below is one of:
* **M — misunderstanding**: wrong formula, constant, field, order of events, timing, or missing behaviour. Fix it
  natively in our code.
* **H — hardware arithmetic**: last-bit float rounding and similar. Accept it with a tolerance (stated), unless it
  would be noticeable in play; then reproduce the *result* natively. No new hardware modelling is proposed.
* **S — session/timeline**: not a port rule; the savestate's history (Veldin play, the arrival scene the port does
  not run yet) makes the values differ. Listed so they are not mistaken for bugs.

## 0. What the savestate says about the session

| global | value | meaning |
|---|---|---|
| level 0x15ed84 | 1 | Novalis |
| tick counter 0x15f5cc | 2439 | 1 (the load's `++` after `MobyLoadTimeUpdatePass`) + 2438 gameplay ticks |
| mode 0x15f5c4 / frames in mode 0x15f5c8 | 0 / 783 | gameplay for the last 783 frames; the 1655 ticks before were the arrival (scene 5 is mode 2, confirmed by the bolts' mode-2 hide bit, §f) |
| idle counter 0x160ff0 (`IncrementTickCounter`, gp−0x5c10) | 780 | last pad input 780 ticks ago |
| hero state / prev | 0 / 7 (timer 730) | a jump, landed, idle since |
| rand 0x12f4d8 | 0x0705d91a | **254 936 draws after `srand(1234)`** (so the level seed is confirmed) |
| VSync 0x15f3f8 / play time 0x15eea4 | 3445 / 6610 | |

The port run (`port_sim.rs`) mirrors this: load pass, 2438 ticks, the first 1655 in game mode 2, a ✕ press held
**2 ticks** ending 780 ticks before the end. With that press the port's landing timers equal the game's exactly
(timer 730, grounded 743); a 1-tick press gives 729/742 and ≥ 4 ticks 731/744, so the press was 2–3 ticks.

## Summary

| check | result | class of the mismatches |
|---|---|---|
| tfrag lit RGBA | **51331/51331 vertices, 1004/1004 tfrags** (single-set 51202, blended 129), bit-exact with both the disc and the RAM light bank | — (locator fix: `occl_index` 0x3a is load-time data) |
| shrub palettes | **1208/1208 instances, 28992/28992 entries** bit-exact | — |
| tie palettes | 1507/1508 instances, 96511/96512 entries; one entry r and b −1 | H |
| game state (254 chunks at their RAM addresses) | 250/254 equal, + 14 play-dependent | M (scene-5 flags), S |
| hero block (87 fields) | 76/87 bit-equal; state, prev state, timers, position, velocity, capsule, ground point exact | M ×3, H ×8 |
| follow camera (16 fields) | 3/16 bit-equal; position within 2.5e-4 u, yaw within 5.4e-5 rad | S/H (tolerance) |
| fog globals | equal to the level settings (no zone at the camera) | — ; underwater tint alpha differs: M (minor) |
| rand stream | not equal (254 936 vs 118 640 draws); **load pass pinned draw by draw** | M ×2 found, rest S |
| tick counter | 2439 vs 2438 | M (off by one) |
| moby table | 54 instances the game never spawns; on the 929 paired: class 929, group 929, crate rotation 199/199 after the two load-pass fixes | M (spawn test, ship, emitters), S (scene, RNG phase), H (crate z) |

## a. Game state — 250/254 chunks equal

Every chunk of the 47 global and 20 × 11 per-level save descriptors was compared at its RAM address with the port's
`GameState` after new game → Veldin start → Clank init → transition → Novalis start. Equal: level, max HP 4, HP 4,
item owned/acquired (bomb glove), ammo (bomb glove 10: **no ammo was picked up on Veldin**), vendor
`[4a, 10, ff…]`, quick-select `[10]`, equipped hand 10, planet unlocked `[1]`, galaxy order `[1]`, visited `[2, 1]`,
wrench held, options, all per-level chunks of levels 2–19 except one.

| chunk | RAM | port | class | diagnosis |
|---|---|---|---|---|
| 5 global flags | [8] [15] [16] = 1 | [8] = 1 | **M** | flag 15 (0x13d397) is set by class 730/790 when it starts scene 5 on the first gameplay tick (cutscenes_transitions.md §4.3); flag 16 (0x13d398) is set in the same sequence (writer not yet read). Neither class nor scene 5 runs in the port, so a port booted here would replay the arrival scene logic incorrectly. |
| L00 / L01 3007 entry record | time field 6 / 11 | 0 / 0 | S | `time = max(time, ScaleTicks(play time)/600)` at entry: 6 = Veldin entered at ≥ 3600 play ticks, 11 at Novalis. The port's play time is 0 on this path. The rule matches. |
| L08 3007 entry record | {count 2, time 3, mask 0x80000001} | zero | open | a level-8 entry would set mask bit 8, not bit 0; no entry of this session writes slot 8. Writer unknown (boot/title path?); needs a read of 0x1416c0 writers. |

Play-dependent (listed, not counted): bolts **74** (all from Veldin: L00 bolts collected 74, L01 0), elapsed 7331,
play time 6610, clock, landmarks, help records/log (pos 6), hits taken **2** (L00 hits 2), Veldin killed bits (3),
L01 packed moby state (214 bytes), **L01 mission byte [3] = 0xff** (set during the arrival: the scene/infobot
sequence completes mission 3).

## b. Hero — 76/87 fields bit-equal

Exact: position (162.53032, 136.39348, **60.0**: the port's ground snap), yaw 1.9872903, velocity (0, 0, −0.015),
displacement, effective velocity, target yaw/speed, capsule 0.8/0.7/0.45, ground z/point/normal, state 0, prev
state 7 / group 4, timer 730, air ticks 0, grounded 743, histories (idx 6, count 32), health 4, Ratchet's moby +0x10.

| field | RAM | port | class | diagnosis / fix |
|---|---|---|---|---|
| fidget timer 0x140360 | 39 | 70 | **M** | (1) `HeroInit` 0x226b70 draws `rand_range(ticks(180), ticks(300))` into it (after `srand(1234)`, before the load pass); `Hero::init_from_moby` draws nothing. (2) The idle fidgets (0x241e00) are not ported, so the timer never counts down and re-arms. Fix: `hero.rs` `init_from_moby` takes the game rng and draws; port the fidget (`hero/states.rs`). |
| seq_timer 0x13f4f4 | 14 | 710 | **M** | the game changed Ratchet's sequence 14 ticks ago: an idle fidget animation. Same fix as above (`hero/anim.rs` fidget seqs). Visible in play: the port's Ratchet never fidgets. |
| ground surface 0x13f640 | 0 | −1 | **M** | the ground probe's hit surface/footstep class is not stored (port keeps −1 = none). Fix in `hero/physics.rs` ground probe (0x232dc0): store the hit's surface byte. Affects footstep sounds / surface reactions. |
| rot.x, rot.y | −0.0 | +0.0 | H | sign of zero only; compare with `==` on floats. Tolerance: ±0 equal. |
| rows[0].x, rows[1].y | −0.40455654 | −0.40455657 | H | 1 ulp of cos(yaw) (trig polynomial last bit). Tolerance 2 ulp / 1e-6. |
| height 0x13f62c | −0.0150070 | −0.0150032 | H | 3.8e-6 (a few ulp at the probe hit distance). Tolerance 1e-5. |
| slope / pitch / roll | 2.38e-7 | 1.79e-7 | H | residuals of atan2/asin on a flat normal (0.9999999). Tolerance 1e-6 rad. |

## c. Follow camera — within 2.5e-4 u

RAM (164.40727, 132.15005, 62.004063), port (164.40750, 132.15016, 62.004063); both 4.640 u behind and 2.00406 above
the feet, FOV tangent 0.63 equal, z equal to the bit. Euler yaw 1.9872458 vs 1.9873002 (5.4e-5 rad), rows within
5e-5. Class **S/H**: the game's camera entered gameplay from the end of scene 5 (mode 2 → 0 handoff), the port's
from the load snap; both springs converged to rest points that differ in the last digits. Not noticeable; tolerance
1e-3 u / 1e-4 rad. Re-check once the scene-end camera handoff is ported (`follow_camera.rs`).

## d. Fog — equal

0x15f444.. = (105, 127, 180), Dn 0, Df 245760, In 255, If 102 = the level settings bit for bit; no fog zone at the
RAM or the port camera. Underwater flag 0x167494 = 0.

| field | RAM | port | class | diagnosis |
|---|---|---|---|---|
| underwater tint 0x161203 | 0x30 | 0x40 | **M** (minor) | 0x30 is what class 751 writes for an active ripple zone other than 1–3 (`UnderwaterLook::set_from_ripple_zone`); the port keeps the overlay's static 0x40 until a zone activates, and the sim/engine have not activated one here. Check whether 751's init (0x2fd0e8 state 0) writes the tint, and wire `set_from_ripple_zone` in the 751 external update (`rc-engine water_render.rs`, `rc_game::water`). Only visible underwater. |

## e. rand stream and tick counter

**Tick counter: M.** RAM 2439 after 2438 ticks; `LoadLevelCoreData` does `0x15f5cc++` right after
`MobyLoadTimeUpdatePass`, so the first gameplay tick sees 1. `Game::new` starts at 0 and the load pass runs at 0.
Every counter-driven rule is one tick early in the port (delete→reuse 2-tick window, bolt glint timers, sky rotation,
menu blink phase). Fix: `rc_game::tick::Game` (or the engine's setup after `load_pass`) sets `counter = 1`.

**Stream.** Game 254 936 draws (104.6 per tick on average, arrival scene included), port 118 640 (load pass 1487,
then 48 per tick at idle). The whole-run total cannot match without the arrival scene, but the **load pass is
pinned draw by draw** from data that load-time draws leave in RAM:
* every bolt's spin-direction bit (pvar +0x55, `randi(2)` first of its 4 draws): all 281 match the stream at
  **offset 1** in the game; the port starts at offset 0;
* every class-500 crate's π/2 turn (`randi(2)`, group-list order): all 175 decidable crates match the stream at
  **offset 1125 = 1 + 281·4** in the game (nothing draws between the last bolt and the first crate); the port has
  them at 1204 = 1124 + **80**.

Two **M** findings follow, both fixed in the diagnostic run (`--load-pre-draws 1 --load-emitters-culled`, which
brings crate rotation to 199/199):
1. **HeroInit's draw** (see b): the one draw before the first bolt. Fix: `hero.rs` `init_from_moby(…, rng)`, with
   `Game::new` seeding before the hero init (`tick.rs`).
2. **Class-27 emitters draw 80 in the port's load pass** (10 emitters × 1 particle × 8 draws); the game's draw 0.
   `type06::emitter_update` treats `view = None` as "visible", while the game's load-time `FastBSphereCheck` view
   culls them. Fix: in the load pass, pass a culling view / skip the spawn (`rc_game::particles::type06`,
   `rc-engine/src/gameplay.rs` load pass `view: None`).

After the load pass the stream cannot be followed from one savestate (scene 5 ran 1655 ticks with actors and a
camera the port does not have). Next step: a second savestate a few seconds later with the pad untouched gives the
per-tick draw count in mode 0 to compare with the port's 48.

## f. Moby table

RAM array at 0x1e9a480 (from Ratchet's moby pointer 0x1413d0), 1022 slots before the end marker.

**Spawn test, M.** The game's loader created **929** of the 983 instances: the 54 class-577 instances with spawn
flags 2 (instances 593–646) are not created on this save, so every later moby sits 54 slots lower. The port creates
all 983 (`scheduler::load_static_mobys`; moby_update_catalogue.md "Not modelled: spawn conditions"). Consequences:
wrong enemies present, wrong moby indices for everything after 592 (group lists and pvar moby links in the game use
the remap 0x1acc00). Fix: port the spawn test (spawn flags bits 0–3 on `spawn_id` against the save bits, 0x15fc88
table) in `rc_game::moby_update::scheduler::load_static_mobys` + the index remap (`Groups::parse` map).

The comparison pairs RAM and port mobys by (class, spawn id). On the 929 pairs:

| field | equal | notes |
|---|---|---|
| class, group | 929, 929 | |
| rotation | 817 (898 with the load-pass fixes) | the rest are unported classes' own turns + 577's random mirror |
| state | 709 | 191 are unported classes that ran in the game (state 1, 2, 0xc, 0xe…; port 0) — S until ported |
| mode | 443 | xor 0x1 ×249 (bolts, below); 0x2 ×139 (unported classes: the port sets no-update, the game runs them); 0x1001 ×30 (865/866); 0x1041/0x1000 ×25 (459); 0x42 ×20 (605); 0x1002 ×11 (660, 688); 0x8000 ×7 (577: the random mirror, not modelled); 0x43/0x41/0x3 ×5 (688, 730, 790, 750, 1135) — all unported classes' own updates |
| position | 519 (546 within 1e-3) | bolts (phase), unported movers, crates ≤ 1 ulp |
| seq/frame/t/speed | 926 / 919 / 920 / 899 | unported classes' anims |
| visible +0x31 | 772 | the port's +0x31 is a draw-distance stand-in (no frustum/occlusion) |

Per ported class:
* **Bolts 13 (281)**: state 3 all equal, spin bits all equal (after the offset); positions differ only by the idle
  orbit **phase** (spin angle = init + k·updates: RNG offset and number of updates). **249 are hidden** (mode bit 1):
  `BoltUpdate` hides bolts while game mode 2 runs (the port has this rule), and those beyond the update distance of
  the gameplay camera have not run since. In the game **all 281** ran during scene 5; with the port's camera at the
  hero only 32 did. S (scene camera / actors), plus a question for the scene port: does the mode-2 moby loop update
  everything, or did the scene camera pass them?
* **Crates 500/501/502/505/511 (240)**: rotation 199/199 after the load-pass fixes; positions differ by ≤ 1 ulp of z
  (75.51146 vs 75.51147; 23 crates): **H**, tolerance 2 ulp / 2e-5. **3 class-501 crates are deleted in RAM**
  (instances 535, 539, 544, all at z ≈ 40, state 0xfd), alive in the port: **M, open** (find the delete path in
  `CrateUpdate`: water / kill volume? the killed bitset is empty, so not the hero).
* **Grass 724/725**: every field but +0x31 equal.
* **Class 27 (emitters)**: RAM positions are far from the instance positions (e.g. (213.3, 380.3, 79.9) →
  (346.5, 120.5, 117.5)); the port never moves the moby. **M**: 0x2bd100 (or something it calls) moves the emitter
  moby; the port's emitter keeps its own copy (`particles::Owner`). Read 0x2bd100's moby writes.

**The ship (531), M, visible in play.** RAM: mode 0x13 (hidden, no update), on the pad; the screenshot shows an empty
pad. The port creates it visible and updating (moby_update_catalogue.md "Ship"). On this first arrival the ship
crashed in scene 5 and stays hidden. Fix: the loader/level-start rule that hides the ship on the crash arrival
(read `DoSpaceTransition`'s first-arrival branch and 531's update) in `rc-engine/src/gameplay.rs` / `moby_spawn.rs`.

Also after the statics in RAM: 1007, 1143, 806 ×9 (501 icons, 3 gone with their crates), 315 ×3, 1204, **71 (wrench),
607, 601 (Clank)**, debris 122/112/686/639 (deleted). The port's run has no hand items (the engine's item loader is
Bevy-side) and no scene actors.

## g. Tie and shrub palettes

Light bank in RAM: sets 0–4 = the disc; **sets 13 and 14 are non-zero** (13 written every frame by
`UpdateAllPointLights` 0x2528a8, 14 by the vendor/menu light `0x28c128`/`OpenVendorMenu`). The tfrags light
identically with the disc bank and the RAM bank, so none of them selects 13/14; ties and shrubs were lit with
the RAM bank; the moby lighting will need set 13
(rc-engine moby lighting: check it is written). No point lights active.

* Shrubs: 1208/1208, 28992/28992 (average colour in the matrix block col1.w also equal).
* Ties: 1508 located by content (ambient copy +0x140 and matrix = disc), 1507 fully equal. Tie 435, slot 14:
  ours (130, 120, 88) / RAM (131, 120, 89), single-set selector 0. **H**: r and b move together and g does not, so
  one shared scalar (the dot product d_A or f_A = max(d, d·w)) is 1 ulp different and two channels sit on a byte
  boundary. Tolerance: ±1 per channel. Not noticeable.

## Prioritised fix list

**M — native fixes (behaviour):**
1. Spawn conditions in the moby loader + the index remap: `rc_game::moby_update::scheduler::load_static_mobys`,
   `Groups::parse` map, pvar moby links (54 enemies present that should not be).
2. First-arrival ship hidden (mode 0x13 after the crash): `rc-engine/src/gameplay.rs` ship creation /
   `rc_formats::moby_spawn::ship_state` (visible in play).
3. Tick counter starts at 1 after the load pass: `rc_game::tick::Game::new` / engine setup.
4. HeroInit's `rand_range(ticks(180), ticks(300))` fidget draw: `rc_game::hero::Hero::init_from_moby` (+ `Game::new`
   order: srand before hero init).
5. Class-27 emitters must not spawn in the load pass (culled load-time view): `rc_game::particles::type06::emitter_update`
   `None` semantics, `rc-engine/src/gameplay.rs` load pass.
6. Idle fidgets (0x241e00: timer countdown, seq change): `rc_game::hero::states` / `hero::anim`.
7. Ground-probe surface 0x13f640: `rc_game::hero::physics` probe 0x232dc0.
8. Scene-5 bookkeeping: classes 730/790 and flags 15/16, mission byte 3 (`rc_game::moby_update::classes`,
   `scene_player.rs`).
9. Open reads: 3 deleted 501 crates (`CrateUpdate` delete path), class-27 moby moves (0x2bd100), 751's tint write
   (0x2fd0e8), the L08 3007 record writer.

**H — tolerances to accept (no hardware modelling):** hero rows/angles 2 ulp or 1e-6; hero height 1e-5; ±0 equal;
camera 1e-3 u / 1e-4 rad (re-check after the scene handoff); crate z 2 ulp; tie colour ±1 per channel.

**S — needs more traces, not fixes:** the post-load stream and per-tick draw count (second savestate in mode 0),
bolt orbit phases and hide bits (depend on scene 5), all unported classes' states.

## Tool changes made for this run

* `tfrag_light_cmp.rs`: header bytes 0x3a/0x3b (`occl_index`) moved from the load-invariant set to the volatile
  set: the loader renumbers them (disc 0 for every tfrag, RAM a per-tfrag value). Without this the table was not found.
* New: `compare-novalis-spawn`, `compare-tie-shrub-light`, `port-load-pass` (trace_harness.md §5).

## Open reads resolved (2026-09-27)

Read-only: Ghidra (`/levels/level01.elf`), the exports in `work/decomp/level01.elf/`, and the savestate's EE RAM
(`work/trace/novalis_spawn_ee.bin`, moby array 0x1e9a480, stride 0x100). Addresses are level01. The H
tolerances of this report are now in `hardware_fidelity_layers.md` "Tolerances".

### a. The 3 deleted class-501 crates: the ammo-crate death gate (M)

* **Where.** `FUN_002eac18` (crate stacking init, called from `CrateUpdate` 0x2ea178 state 0), both the group
  loop and the lone-crate branch:
  `if (class == 0x1f5 && moby+0xb0 != 0xff && deaths[moby+0xb0] < pvar+0xf8) DeleteMoby(crate)`, where
  `deaths` = s32[16] at **0x14ee90**.
* **Writer of 0x14ee90.** The hero death routine `FUN_002319b0` does `deaths[killer+0xb0]++` (killer = 0x1415d0,
  when its +0xb0 ≠ 0xff), plus the per-level copy `0x14e990 + level·0x40 + b0·4`, the level and total death counters,
  and sets 0x141401 (death reload).
* **Reset.** `LoadLevelCoreData` 0x258128 clears 0x14ee90..0x14eecc just before `MobyLoadTimeUpdatePass`, but only
  when `param_2 == 0`. `entry` passes `param_2 = 1` for the death reload (0x141401 set) and 0 for a fresh load.
* **Rule.** A 501 ammo crate with a mission byte (+0xb0) and pvar+0xf8 = N appears only after the player has died N
  times, to enemies of that mission, since entering the level. On a fresh entry every such crate with N > 0 is deleted
  in the load pass.
* **RAM check.** Crates 535 / 539 / 544 have +0xb0 = 0 and pvar+0xf8 = 3 / 2 / 1; deaths[0] = 0. `DeleteMoby` stored
  +0x38 = counter + 2 = **2**, so the deletion happened at counter 0 (the load pass). z is still the disc 40.0 / 40.0 /
  41.0 because the delete returns before the probe. `CrateSpawnIconMoby` 0x300528 skips a state < 0, hence 9 icons
  (806) for 12 crates. The other 501s have f8 = 0 (536–538, 540) or +0xb0 = 0xff.
* **Class: M.** The port already has the test (`crate_::settle_fields`, `MissionState::ammo_crate_gate`). But the
  only implementation is `NoMissions`, which returns `i32::MAX`, so the port never deletes.
* **Fix outline.**
  - A native per-mission death counter `[i32; 16]` in the session state (`rc_game::game_state` session part; it is not
    saved: 0x14ee90 is outside every save descriptor).
  - Cleared on a fresh level load, kept on the death reload.
  - Incremented by the hero death path by the killer's +0xb0.
  - A `MissionState` impl backed by it, passed to `World` in `rc-engine/src/gameplay.rs` and `rc-trace` `port_sim.rs`
    instead of `NoMissions`.

### b. Class-27 emitters: moved by the Blarg flyers (class 660) (M)

* **Not 0x2bd100.** `ParticleEmitterUpdate` writes only its pvar P+0x7c (its own moby pointer) and P+0xc8 (its
  counter). It never writes the moby.
* **Who moves them.** The shared flyer driver `FlyerPathDriver` `FUN_002f5168`, called by the class-660 update 0x2f4428.
  - Every tick, for each set bit k of the flyer's pvar+0x120 (bits 1–4), it takes the linked emitter
    `moby = base + pvar[0x140 + 4k]·0x100`.
  - Emitter position = the flyer's joint k (`FUN_002645a8(flyer, k)`).
  - Emitter P+0x80..0x8c = unit(joint − reference point).
  - Emitter P+0x70 |= 0x1000 (the random-Euler velocity mode that uses P+0x80).
  - A second loop (bit 0 set) spawns type-10 particles at the joints instead.
* **RAM check.** Flyers 619–628 (disc instances 673–682, class 660, state 1) have flags 0x120 = 2 and link 0 =
  emitters 285, 288, 286, 292, 290, 294, 291, 289, 287, 293. Each emitter sits about 2 u from its flyer. The disc-vs-RAM
  pvar diff of all ten emitters is exactly +0x7c..+0x8c. P+0x80 in RAM is per emitter, e.g. (−0.71, −0.68, 0.20, 1);
  the disc value, shared by all ten, is (0.537, 0.832, −0.139).
* **Class: M, visible.** The emitters are the flyers' exhaust trails. The port emits from the ten static disc points
  with the disc direction. The moving position also changes the Novalis `FastBSphereCheck(240)` cull, and so the
  rand draws.
* **Fix outline.**
  - Port class 660 (0x2f4428 + `FUN_002f5168`: path, scale/fade, the emitter placement) with the pvar moby links through
    the spawn remap (fix 1).
  - Make `type06::emitter_update` read the moby position and P+0x70/P+0x80 live, instead of the `particles::Owner`
    snapshot. Until then, emitters owned by an unported flyer should not emit.

### c. Class 751 and the underwater tint alpha 0x30 (tool gap, not a port rule)

* **Init.** 751's init (state 0 of 0x2fd0e8) does **not** write 0x161200..0x161206.
* **Per tick.** State 1 tests the camera 0x167240 against the 7 zone cuboids (pvar = cuboid indices 2..8,
  `PointInCuboid` 0x274820). The last active zone k in loop order 0..6 (= the highest) sets the colours:
  - k in 1..3: tint (0, 0x28, 0x30, **0x40**), fog colour (0, 0x28, 0x30);
  - any other k: tint (0x18, 0x30, 0x90, **0x30**), fog colour (0x18, 0x30, 0x90);
  - no active zone: nothing is written (the last values stay).
* **RAM check.** Evaluated with the RAM cuboid table (`*0x1600ec`, stride 0x80), the RAM camera (164.407, 132.150,
  62.004) is inside zone 3 (cuboid 5) **and** zone 6 (cuboid 8), so k = 6 → alpha 0x30. The RAM fog colour
  (0x18, 0x30, 0x90) agrees; zones 1–3 would have given (0, 0x28, 0x30). The port camera is 2.5e-4 u away, so it is
  in the same zones.
* **Class: comparison-tool gap.**
  - The engine already applies the rule every tick (`fog_state.rs` → `UnderwaterLook::set_from_ripple_zone(ripple.last.zone)`).
  - `rc-trace` `novalis_spawn.rs::check_fog` compares RAM with `UnderwaterLook::default()` (the overlay's static 0x40).
* **Fix outline.** `check_fog` builds the expected look as default + `set_from_ripple_zone(sim.ripple.last.zone)`,
  using the port sim's 751 state. Expect equal.

### d. The L08 3007 record {count 2, time 3, mask 0x80000001}: the wrench-combo stats record (M)

* **Writer.** `SetState` 0x23cf98, group-6 entry, case new state **0x13** (ground wrench combo):
  - `if (count != 0xffff) count++`;
  - `time = max(time, ticks(play time 0x15eea4)/600)`;
  - `mask |= 1 << level | 0x80000000`,
  on the record at **0x1416c0**.
* **Aliasing.** 0x1416c0 = 0x141680 + 8·8, the slot the per-level save descriptor calls level 8's chunk 3007. The game
  aliases it: the save keeps the "wrench swing used" record inside level 8's section.
* **Sibling writes** (same layout, same pattern):
  - 0x14 (jump attack) → 0x141848 (misc record 0);
  - 0x15 (comet strike) → 0x141850 (misc record 1) and 0x1417a8 (gadget-help record 17).
* **Meaning.** count 2 = two ground-combo starts on Veldin (mask bit 0 = level 0); time 3 = play time ≥ 1800
  ticks at the swing.
* **Class.**
  - **M**: the port's `hero::melee` does not bump these records; its module doc lists them as not ported.
  - The time value is also **S**: the port's play time is 0 on this path.
* **Fix outline.** In the melee SetState entries (`rc_game::hero::melee`), bump the records through `GameState`. Keep
  the aliasing in storage (0x13 → `levels[8].entry`, 0x14 → misc[0], 0x15 → misc[1] + gadget[17]) so saves stay
  byte-compatible. Give it a named accessor so the alias is explicit.

### e. The camera at the end of scene 5: no handoff, the follow camera is frozen (S for the comparison, M in the port sim)

* **During mode 2, nothing runs the follow camera.**
  - The mode-2 frame `CutsceneModeUpdate` 0x2aca80 runs `MobyUpdateLoop`, the level callbacks, `PatchShrubGifs` and
    `UpdateParts`. It never runs `CameraUpdate` 0x20eca8.
  - The scene camera `FUN_002ac8d8` writes only the render camera: 0x167240 pos, rows 0x167450/60/70, and FOV
    0x16cf70 + `UpdateViewContext`.
  - The UpdateCam slot (0x1675d0) and the type-0 spring state are not touched. `CameraType0Update` 0x314e00 never
    reads 0x167240; its leash uses its own cam+0x30.
* **The start and the end do nothing to it either.**
  - `DialogStreamStart` 0x2ac330 has no camera writes.
  - The end `FUN_002ac608` sets FOV 0.63 and `UpdateViewContext`, deletes the actors, and does `SetState(0,1)` + the
    ground snap. The only reset, `HeroTeleport` 0x2368e0 → `CameraResetBehindHero` 0x20ee80, runs only when 0x16cd26 ≠ 0.
    The scene start clears 0x16cce0..0x16cea0, so scene 5 never resets the camera.
* **Handoff rule.** The first mode-0 tick resumes the type-0 springs exactly where they were left. The scene starts from
  inside the moby loop on gameplay tick 1, and that tick's `CameraUpdate` still runs afterwards (read from the code
  path; not traced). So the game's follow camera had 1 + 783 updates since the load snap.
* **Port.**
  - The engine suspends the whole gameplay tick during a scene, so its camera freezes too: consistent.
  - `rc-trace` `port_sim.rs` runs `Game::tick` (hero + camera) on all 1655 cutscene ticks: 2438 camera updates.
* **Fix outline.**
  - `port_sim` skips the camera update on the cutscene ticks after tick 1. Mobys and particles keep running, as in the
    mode-2 frame. Not read: whether `MobyUpdateLoop` runs the hero in mode 2 (the hero is in state 100 and does not
    move, so the camera target is unaffected).
  - Then re-run §c and tighten or drop the provisional camera tolerance.

## Fixes applied (2026-09-27)

Re-run: `cargo run -p rc-trace -- compare-novalis-spawn --ee work/trace/novalis_spawn_ee.bin` (no knobs; report
`work/trace/novalis_spawn_report_after.txt`). The knob `--load-emitters-culled` is gone (the port's default
now culls). `--load-emitters-visible` re-creates the old behaviour as a diagnostic. `--load-pre-draws N` now adds
draws on top of HeroInit's own draw. Regression: `tools/trace/tests/novalis_spawn.rs` asserts every check marked
"all" below (since 2026-09-27 against the distilled fixture `tools/trace/tests/fixtures/novalis_spawn.tsv`, no
savestate needed).

| # | fix | rule (level01) | before | after |
|---|---|---|---|---|
| 1 | spawn test + instance → moby map | loader 0x256890..0x256a58: see below | 983 created, pairing by (class, spawn id) only; 54-slot shift | **929/929 slots equal by index**, 54/54 rejected records absent from RAM (577 × 54, flags 2, instances 593–646) |
| 2 | ship hidden on the first arrival | `MissionNpcUpdate` 0x2fad68 state 1 → `FUN_002a2450` | port slot 983: 531 visible, updating | slot 929: 531, mode 3 (hidden, no update, no collision), as RAM (0x13) |
| 3 | tick counter | `LoadLevelCoreData`: `0x15f5cc++` after `MobyLoadTimeUpdatePass` | 2438 vs 2439 | **equal** (2439) |
| 4 | HeroInit's fidget draw | 0x226b70 end: `0x140360 = rand_range(ticks(180), ticks(300))` | no draw | the first draw after `srand(1234)` = **229**; load = 1 + 1407 draws |
| 5 | class-27 emitters culled in the load pass | 0x2bd100 → `FastBSphereCheck` with the unbuilt view | 80 load draws (10 × 8) | 0 |
| 7 | 0x13f640 | the ground probe 0x232dc0 | "ground surface" −1 vs 0 | it is the **water level** (f32): 0.0 = 0.0; surface id 0x140630 −1 = −1, footstep 0x14063d 0 = 0 |

Load pass on the stream, together (4 + 5): **bolt spin bits 281/281** (before 149/281: offset 0 vs 1), **class-500 crate
turns 199/199** (before 118/199), moby `rot` 817 → 898/929. Hero 76/87 → 79/89. The rest of the hero list is H plus
item 6: `seq_timer` 14/710 and `fidget_timer` 39/73. The fidget countdown / re-arm 0x241e00 is not ported, so the
drawn 229 is re-armed by the idle entry's own `rand_range(50, 100)`.

**1. Spawn test.** The test runs per record when `flags` (+0x08) ≠ 0, with `id` = +0x0c and `m` = +0x04:
* 0x10 → b1 = the spawner slot `FUN_0029ab50` (0x14d590 + L·0x100, 64 slots, from 63 down); otherwise b1 = 0xff.
* `0x1bbb04[id] ≠ 0` → not created.
* flags & 3 ≠ 0: when `0x15fc88[m] == 0xff` (mission done; 0x15fc88 is `LoadLevelCoreData`'s copy of the save's
  0x14c050 + L·16), the record needs flag 2. b4/b6 then come from +0x14 (b4 halved when this visit's death bit
  0x1ba950 is set). Otherwise it needs flag 1, and b4 is halved when the killed bit 0x14c190 + L·0x100 is set.
* else flag 8: not created when the death bit 0x1ba950 is set.
* else flags & 0xc == 4: not created when the killed bit is set.

The created records get consecutive slots. `0x1acc00[i]` = slot or −1, and the loader applies it to the group lists and
the pvar moby links. Port: `rc_formats::moby_spawn::{spawn_test, loader_spawns, instance_to_moby}` and
`rc_game::moby_update::scheduler::{load_level_mobys, LevelStatics, spawn_save}`. `LevelStatics` holds the maps, the
groups and the pending list. Pvars come from `parse_pvars_spawned`. The engine hides the rejected instances' entities,
and its renderer is driven through moby → instance. Nothing in the game creates a rejected record later: only the
loader reads the records. A pending record appears on a later load whose save passes the test, e.g. the flag-2 577s
once mission 0/1/2 is done.

**2. Ship.** While a created mission NPC (730/790) has its mission byte 0x14c050[L·16 + moby+0xb0] ≠ 0xff, its
state 1 hides the ship every tick (mode |= 3, +0x94 = 0). State 8 (mission done) shows it again (`FUN_002a2480`). The
port applies the hide at creation (`moby_spawn::ship_hidden_on_arrival`): hidden on the first arrival, and visible on a
revisit after mission 0 and on levels without such an NPC. Engine check: the pad right of the spawn is empty, as in
`novalis_spawn.png`.

**5. Emitters.** 0x16d140..0x16d17f lie in the overlay's `.data` and are all zero on disc; `BuildRotationViewProj`
(DrawWorld) first writes them. `FastBSphereCheck` scales the sphere by the view's w (0x16d17c) with `vmulw`, so on the
zero view r = 0, and "behind" `0 − (0 + 0)` = +0 → −1. So `type06::emitter_update(view = None)` now culls on level 1.

**Also fixed (from "Open reads resolved"):**
* (a) `LevelMissions` (`rc_game::moby_update::services`): 0x15fc88, 0x14c050 and the deaths 0x14ee90. It is wired
  into the engine and `port_sim`. It is cleared on a fresh load and kept across the engine's respawn. `hero_death`
  exists, but the port has no killer 0x1415d0, so nothing bumps it yet. Crates 535/539/544 are now deleted in the load
  pass: moby state 709 → 712.
* (c) Underwater look: `port_sim` keeps 751's colours per tick. This also found an **M** in both 751 ports: the init
  (0x2fd0e8 state 0) sets **update distance 0xff and z + 0.5**, which the external updates missed. Without it, 751
  never ran after the load, since it is 64 u from the spawn camera. Now zone 6 → tint (0x18, 0x30, 0x90, **0x30**):
  **equal**. 751 now draws its drops each tick, so the port's ticks draw 50.06 on average (was 48.05).
* (d) The melee entries count 0x13/0x14/0x15 in `Melee::entered`. The engine applies them after each tick through
  `hero::melee::MeleeRecords` on `GameState`: 0x1416c0 = `levels[8].entry` (the alias), misc 0/1, gadget-help 17.
* (e) `Game::camera_paused`: `port_sim` skips `CameraUpdate` on the mode-2 ticks after the first. The result is
  unchanged to the last digit: pos 2.5e-4 u, yaw 5.4e-5 rad. The springs are at rest either way (784 or 2438
  updates), so the remaining delta is not the update count. It stays S/H: the springs' rest point from the game's
  scene-start state.

Still open: game state 250/254 (scene-5 flags, entry-record times), the whole-run rand state (the scene), the unported
classes, and tie ±1.

## Second savestate: idle-tick activity (2026-09-27)

Input: slot 2, `SCUS-97199 (CE4933D0).02.p2s`, taken after slot 1 with the pad untouched. Kept as
`~/PS2/ratchet1/savestates/novalis_idle.p2s` with `novalis_idle.png`; dumps `novalis_idle_ee.bin` and
`novalis_idle_spr.bin` in `work/trace/` (git-ignored). The screenshot shows Ratchet at the same spot, mid-fidget, with fresh smoke above the city.

The game side is a RAM diff of the two states. The port side is `compare-novalis-spawn` run on each state's EE dump
(`--ee …novalis_spawn_ee.bin` / `--ee …novalis_idle_ee.bin`, default options, i.e. the fixes above):
* The timeline is the same in both runs: 1655 mode-2 ticks and ✕ at port tick 1657.
* Slot 2 runs 1072 more ticks.
* The port's window figures are the differences between the two runs.

The consumer analysis used throwaway scripts on the dumps and on `work/decomp/level01.elf`.

### Counters

| global | slot 1 | slot 2 | Δ |
|---|---|---|---|
| tick 0x15f5cc | 2439 | 3511 | **Δt = 1072** |
| VSync 0x15f3f8 / play time 0x15eea4 | 3445 / 6610 | 4517 / 7682 | 1072 / 1072 (no dropped frame) |
| mode 0x15f5c4 / frames in mode 0x15f5c8 | 0 / 783 | 0 / 1855 | mode 0 throughout |
| idle counter 0x160ff0 | 780 | 1852 | +1072: no pad input |
| rand 0x12f4d8 | 0x0705d91a (254 936 draws) | 0xa20aea72 (311 008 draws) | **56 072 draws** |

**Game: 52.31 draws per tick. Port: 50.14.** The port drew 53 752 in the same window: 1005 ticks × 50, 58 × 52 and
9 × 54. The port's tick counter is now equal (3511). The game's rate is not an integer and not constant. It is a
steady base of about 6 per tick (751, the sound origin, the hero). On top of that come the emitters, which switch
between 0 and 10 × 8 as the flyers move, plus event bursts: foam respawns, glints, one drip and one gunship volley.

### Attribution (game, over the 1072 ticks)

| consumer (level01) | draws | per tick | evidence / how measured | port |
|---|---|---|---|---|
| class-27 emitters, type 6 (`0x2bd100`, 8 draws per particle, 1 particle per visible tick) | ≈ 33 300 (30 000–35 500) | ≈ 31 | Each emitter rides a flyer (660) along its spline (`*0x1b0930[pvar+0x74]`). `FastBSphereCheck(240, r 20)` was replayed against the RAM view 0x16d140..0x16d1af, which is identical in both states (the camera did not move). This gives 3750 visible emitter-ticks. The same model predicts the last 149 ticks before each state as 435 / 334 particles; the pool holds 483 / 370 type-6 records (life 149, one per tick, owners +0x2c). So the model was scaled by 1.11, giving ≈ 3.9 visible emitters on average (0–5 at a time) | **51 456** (48/tick): the 6 of 10 emitters at their static disc points that are always in view |
| sound: `sound_update` 0x2a0638 → `SoundOcclusionOrigin` 0x2a0190 every frame in mode 0/2 (`rand_vec(0.5, 6)` = 3 draws) | 3 216 | 3.0 | code path, unconditional. Also: the 6-origin batch per tick with a new occluded sound (18 draws; 0x13f2e8 = 3483, so at least one batch in the window) and the pitch-bend `randi` per play (flyer loop sounds in slots 1 and 5 carry random bends) | 0: the engine's `AudioSystem` draws from its own `srand(1234)` stream; `port_sim` has no audio |
| ripple manager 751 (`0x2fd0e8`): `randi(2000)` per active patch (camera in zones 3 and 6 → 2 patches) | 2 144 (+2 per drop) | 2.0 | camera zones as in §c | **equal** (2.0) |
| Ratchet idle: fidget chance `0x241e00` (1 `randf` per tick in seq 0 once the 120-tick cooldown 0x13f538 is 0), head look `0x22b928` (in seq 1/2 it re-arms 0x140360 with `rand_range(ticks(40), ticks(70))` every tick; in seq 0, 4 draws per expiry), idle secondaries `0x22bdd0` and blink `0x2278c0` | ≈ 1 200 | ≈ 1.1 | code paths + RAM. Slot 1: seq 0, seq_timer 14 (a fidget had just ended), cooldown 0. Slot 2: seq 2 (fidget record 1, 0x1415c0 = 1) for 100 ticks, cooldown 20 = ticks(120) − 100, record-1 cooldown 230, 0x140360 = 44 (inside the 39..69 re-arm range) | 0 (no fidget: port seq_timer 1782) |
| 760 foam elements (draw callback `0x2fe080`: respawn `0x2fe888` = 6 draws + `randi(2)`) | **1 344** (exact) | 1.25 | 120 elements, each with cycle (j + 3)·60 ticks (fade-in 60, wait (j + 1)·60, fade-out 60). Replaying them from slot 1 reproduces all 120 timer triples of slot 2 (0x1d5bc0 / 0x1d6520 / 0x1d6e80), with 192 respawns | 0 (760 not ported) |
| bolt glints (`BoltUpdate`: `rand_range(300, 600)` + `rand_angle`) | 132–158 | 0.13 | 31 bolts updated all 1072 ticks (spin +0x58 advanced 0.834 rad = 1072·0.0125 mod 2π). Waits +0x6e/+0x6c before and after give 66–79 glints | same rate (phase differs: S) |
| drip (751 drip timer 4, 787 creation 2, splash `0x2ffdc0` 16 × 7, type-35 → type-45 rings ≈ 48) | ≈ 170 (1 drip) | 0.16 | 787 slot 944: the previous drip was deleted 79 ticks before slot 1; the current one hit 94 ticks before slot 2 (pvar +0x14 = 26 of 120); 6 type-45 rings alive | 4 per drip (751's own draws only) |
| critters 577 (`GroundCritterUpdate`: wander pick every ticks(120) = 3 draws, `randi(19)` checks) | ≈ 100 | 0.1 | 4 critters wandering in states 0xe/0xf (targets +0x1e0 changed) | 0 |
| gunship 688 volley: one shot at path node 198 (fire nodes +0x180 = 11, 27, 198, 210; last-fired 27 → 198; joint toggle +0x184 0 → 1). Includes projectile 686 (`0x2f6a30`, 40 draws per tick of flight), the impact (5 × type-11 pairs, 4 × type 8, 40 × type 15, 8 flashes 112), the type-11 split cascade (35 per split), and impact smoke 700 (`0x2f8c58`, 15 per tick, armed with `rand_range(60, 180)` = 99, 33 ticks so far) | ≈ 8 000–10 500 | burst | Pool: type 11 × 227, type 15 × 478 (ages 1–33, the burst of 40 at 33), type 4 × 50, type 16 × 66 (= 2 per tick × 33 ticks), 700 pvar[0] = 66 left. The impact was at (82.6, 271.9, 44.7), where the eight 112 flashes are, deleted 15–22 ticks before slot 2. Also 696 debris alive | 0 |
| other (774 NPC look-around, 806 icons, 1504 light, flyer sound timers +0x138 = −1) | < 300 | — | — | 0 |
| **attributed** | ≈ 51 000 | | the remaining ≈ 5 000 (9 %) is within the emitter and volley ranges | |

### Differences, classified (all M; no H term shows up in the counts)

1. **Emitters, +18 000 in the port. M, visible.** The port emits from the ten static disc points, 6 of which are always
   in view. The game's emitters are the Blarg flyers' exhaust (§ "Open reads resolved" b), in view 0–5 at a time. The
   fix is still class 660 (`FlyerPathDriver` 0x2f5168; in this window it draws nothing) plus the live emitter moby
   position and P+0x70/P+0x80. Doing only the interim rule "an unported owner does not emit" would take away the whole
   33 000 instead.
2. **Sound-layer draws on the game stream: −3 300. M**, not visible by itself, but it shifts every later draw. The fix
   is wiring, not porting: `rc_game::audio::voices` already has `occlusion_origin` / `random_offset` and the pitch-bend
   `randi`. The engine's `AudioSystem` must use `Game::rng` at the game's point in the tick (sound after the camera,
   before `0x15f5cc++`), not its private `srand(1234)` copy. `port_sim` should then run the sound slots too, so the RNG
   comparison includes them.
3. **Gunship 688 volley: −8 000 to −10 500 per volley. M, visible** (the city bombardment and the smoke near the spawn).
   Needed: 688 (path, fire nodes, target `randi`, the 700 arming list), 686, the impact FX particle types 4 / 8 / 11
   (11 is ported) / 15, flash 112, smoke 700 (type 16), debris 696–698.
4. **Hero idle: −1 200 (≈ 1.1 per tick). M, visible** (Ratchet never fidgets or looks around in the port). Port
   `0x241e00` (candidates 0x179d10 + k·0x70: chance Σ 1/(+0x58·60), per-record cooldown +0x60, global cooldown
   0x13f538 = ticks(120)), `0x22b928`, `0x22bdd0` and `0x2278c0`.
   **Naming correction:** 0x140360 is the head-look timer of `0x22b928` (HeroUpdate), not a fidget countdown. The
   harness's "fidget_timer" row and fix #4 above keep the right draw (`HeroInit`'s `rand_range(180, 300)`), but the
   field is the look timer.
5. **760 foam: −1 344 (exact). M, visible** (no foam on the waterfalls). Element timers + `0x2fe888` + the draw.
6. **Drip splash (−166 per drip) and critters 577 (−100): M, minor.**
7. **Equal:** 751's per-patch draws, the glint rate, the tick counter, the camera (unchanged: the same camera).
   Bolt orbit phases remain S (the arrival scene).

Moby state over the window: 31 bolts, 10 flyers + 10 emitters, the gunship, 5 of 14 critters, 2 elevators 703 and 5
elevators 715, the 695 floats, the 774 NPC and the drip changed. In the port only the bolts and its glints change (+
grass reacts to nothing, since the hero stands still). Particles alive at slot 2, game → port:
* type 6: 370 → about 894 (6 × 149);
* types 4 / 11 / 15 / 16 / 45: 50 / 227 / 478 / 66 / 6 → none;
* type 62: 56 persistent records in both game states → none in the port. This spawner is not identified yet.

### Implications for the fix list (by draw share, then visibility)

1. **Class 660 + live emitters** (the largest term, and it hides everything else in the stream comparison).
2. **Sound draws on the game stream** (a steady 3 per tick; wiring only).
3. **Gunship 688 + 686 + FX + 700** (bursts of about 9 000 per volley; visible).
4. **Hero idle fidget / look / secondaries** (about 1.1 per tick; visible).
5. **760 foam elements** (1.25 per tick, exact rule).
6. Drip 787 splash, critters 577, type-62 spawner, 806 / 774.

With 1, 2, 4 and 5 done, a no-volley window of this state should match the game to within the emitter model: about
40 per tick, against the game's steady part. A second idle savestate taken **without** a gunship volley in the window
(the gunship fires up to 4 times per loop, at nodes 11, 27, 198 and 210, subject to its camera-cuboid and line-of-sight tests) would pin the steady part exactly.

## Blarg flyers and the challenge gate ported (2026-09-27)

Ported: class 660 (`BlargFlyerUpdate` 0x2f4428 + `FlyerPathDriver` 0x2f5168), the live emitter owner
(`type06::emitter_update_live`: the class-27 update reads its moby's position, rotation, P+0x70 and P+0x80), the
challenge gate and idle states of the teleporter pads 1135 (`TeleporterPadUpdate` 0x308bd8, arms `FUN_003092d0`) and
of the gold-weapon offers 304 / 1456–1465 (`ItemOfferUpdate` 0x2e1ac0). Spec: moby_update_catalogue.md "In the port:
Blarg flyers, teleporter pads, gold-weapon offers". New report section §f2 in `compare-novalis-spawn` (spawn
savestate, `--ee work/trace/novalis_spawn_ee.bin`).

| check | before | after |
|---|---|---|
| moby state (929 paired) | 712 | **743** (+10 flyers, +1 pad, +20 offers / props) |
| moby mode | 443 | **475** |
| moby update distance | 883 | 884 |
| classes 1135, 304, 1456–1465: every field | 1135 state 1/2, mode 0/2; 304/1456+ state 0, mode 0 | **all equal** (pad 961: state 0, mode 0x51, no collision; 962: state 1, mode 0x50, collision; the 5 even offers and their props deleted in the load pass) |
| pad 962's arms (315 ×3, dynamic) | not created | created; positions and rotations equal RAM's slots 941–943 to 0.01 (port slots 939–941: the port has no vendor hologram 1143 / 1007 before them) |
| flyers in state 1, mode 0x5020 | 0/10 (mode 2, never ran) | **10/10** |
| flyer splines: count (closing point dropped), z, per-point arc lengths vs RAM's copy | — | **10/10** within 2.7e-4 |
| RAM segment tangents vs the port's at RAM's point index | — | **10/10** within 2.4e-4 |
| port Hermite point at RAM's t vs RAM's curve point +0xd0 | — | **10/10** within 9.2e-5 |
| emitter at the flyer's joint (emitter-to-flyer distance, RAM vs port) | 0/10 (static disc points) | **10/10** within 0.25 (e.g. 2.023 / 2.011), P+0x70 0x170e equal, P+0x80 unit and close (e.g. RAM (−0.710, −0.676, 0.199), port (−0.697, −0.688, 0.199)) |
| flyer positions vs RAM | 50–290 u (parked at the disc points) | 0.1 / 1.6 / 1.7 / 2.2 / 2.5 / 2.9 u (6 flyers), 9.3 / 9.7 / 11.2 / 17.8 u (4); RAM (81.7, 181.5, 71.1) vs port (64.7, 184.8, 75.1), RAM (11.8, 247.7, 83.8) vs port (9.9, 258.7, 85.1) |
| rand draws per gameplay tick (port, 2438 ticks) | 50.06 (min 50: six emitters at static points always in view) | **27.44** (min 2, max 86: the emitters ride the flyers in and out of the view) |

**Classification.** Flyer positions: **S**. The speed is `base · f(distance to the camera 0x167240)`, and for the 1655
scene-5 ticks the game's camera is the scene camera while the port's is the follow camera at the hero; every
position-independent part (processed splines, tangents, curve evaluation, segment switch, state, mode, emitter
placement) matches. The per-tick draw count now has the emitter term's shape: the second savestate's attribution
(§ "Second savestate") puts the game's emitters at ≈ 31 draws per tick in that window (≈ 3.9 visible); the port's
emitters here average ≈ 25 (27.44 − 751's ≈ 2). The whole-run stream still cannot match (scene 5).

**One result-level reproduction** (hardware_fidelity_layers.md "Result-level reproductions"): `FUN_0028bb90` takes 20
chord samples per segment on the PS2 (truncating adds end at t = 0.99999946), 19 with IEEE rounding; the port samples
t = 0.05·k, k = 1..=20. With 19 the arc lengths would be about 5 % short and the spline check above would fail.

**Not ported (counted in `Services::fx.unported`, printed in §f2):** the flyer kill effects, group sync, bit-0 sparks,
level-3/9 combat; the pad's activation cuboid, group command, prompt and teleport states; the offer's help
registration (the success branch is taken; 5 counts in the load pass), buy prompt and memory-card save.

**Engine** (`RC_SCENE=0`, frame 60, deterministic: two runs pixel-identical): the flyers fly their paths (three pass
the spawn area near (194, 155, 98) at tick 60); pad 961 is not drawn; alpha 0 (z ≥ 175) hides a flyer. The exhaust
particles spawn and follow the flyers (`RC_PART_STATS`: 197 type-6 alive at tick 60, 111 in the view; `port_sim`
records lie along the flight paths), but they are not visible on screen: the texture is part_defs[4] frame 0, the
darkest smoke frame, drawn additive with vertex colour 0x2c. Whether the game shows these trails brighter is open
(a PCSX2 screenshot with a flyer close to the camera would decide it).

## World props and effects; the exhaust rate (2026-09-27)

Ported: the fire / smoke fields 760 and 809 (formerly "760 waterfall foam"), the waterfall foam proper (751's zone-5
particle types 57 / 56), the props 705, 703 / 715, 768 / 769, 1042, 701 and the breakables 704, 709–711, 729, 778 / 779,
754, 1813 / 1816 (creatures.md §7). `port_sim` now sets the level's volumes (the engine did already), so the cuboid
users (760, 805, 701) see them.

**The flyers' exhaust rate is exact; the ≈ 31 per tick attribution above was the model's error.** A scratch harness
(`rc-trace` as a library: the port run to the first savestate's tick, then the RAM state of the ten flyers and their
emitters copied in, then the 1072 window ticks) gives: flyer positions at the second savestate equal RAM's to ≤ 0.002
u (all ten), and the type-6 pool equal record for record (370 alive; per owner 38 / 108 / 75 / 149 with the same timer
ranges 1–38, 42–149, 1–75, 1–149 as RAM). The window's emitter term is **3523 puffs = 28 184 draws = 26.29 per tick**
(the port's own run, with the scene-period flyer offsets of § "Blarg flyers": 3540, 26.42). The earlier 33 300 (≈ 31)
came from scaling a visibility replay by 1.11 to fit the pool; every emitter emits exactly one puff per visible tick in
both. Nothing in `flyer.rs` or `type06.rs` needed changing (the display-byte trail blend is untouched).

**Fire fields vs RAM.** With the first savestate's element arrays (0x1cbc60 … 0x1d6e80) and 0x161374 copied in, the
port's 1072 ticks reproduce all 136 timer triples of the second savestate and exactly **192 respawns = 1344 draws
(1.25 per tick)**, the § "Attribution" figure; unchanged elements keep their positions, scrolls agree to ≤ 1.4e-3
(float accumulation, hardware_fidelity_layers.md "Tolerances"). From the port's own start (the scene period
registers the fields differently: S) the window has 164 respawns (1.07 per tick).

**rand draws** (`compare-novalis-spawn` on both dumps; window = second − first, 1072 ticks):

| | load pass | first savestate | second | window per tick |
|---|---|---|---|---|
| before (tree at the start of this pass) | 1457 | 79 849 | 115 901 | 33.63 |
| after (this pass + the other agents' concurrent work) | 2419 | 129 624 | 171 419 | 38.99 |
| game | | 254 936 | 311 008 | 52.31 |

This pass's own share: load pass +962 (136 fire-field elements × 7 = 952, five spinners × 2); window +1148 (fire-field
respawns; 1344 from the RAM phase). The props, breakables and the zone-5 foam draw nothing in this idle window (no hit,
the camera outside zone 5, the spinners / lifts / doors / 701 draw only at init or when triggered). The rest of the
+5.4 per tick is the enemies' and hero agents' work (the gunship volleys: bursts of up to 927 draws in a tick).

**Corrected attribution of the game's window** (§ "Attribution"): emitters 28 184 (exact, not ≈ 33 300), sound
3 216, 751 2 144, fire fields 1 344 (the "760 foam" row), hero idle ≈ 1 200, drip ≈ 170, glints ≈ 150, critters
≈ 100, the gunship volley 8 000–10 500: ≈ 44 500–47 000 of 56 072. The unexplained remainder is **≈ 9 000–11 500**
(8–11 per tick), no longer hidden in the emitter range. Candidates for the next read: the type-62 records (56
persistent, spawner unknown), 1504 (the wandering light's `rand_range` walk), 774's look-around, the volley's split
cascades and embers.

**Fixture / digest:** `cargo test -p rc-trace --test novalis_spawn` passes (the new updates change no load-time slot
fact); `novalis_hero_digest` byte-identical (2800 lines, the same file as the other agents' today). The compare's
moby tallies moved up (state 790 → 880, mode 517 → 629 of 929; 760 × 4 and 809 now "every field equal").
