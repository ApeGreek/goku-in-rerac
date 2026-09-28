# Explosions: the game's routines, their callers, and the port

The user's question (2026-09-28): are all explosions now "the same explosion"? This page is the audit. It lists every
explosion-like routine in the game with its address, the copies the 19 level overlays carry, and its callers by code
identity. It classifies each one from the disassembly: one shared function, identical copies with other constants, or
a genuinely different effect. It then compares each routine with the port. The merges and fixes it led to are in
§B, §C, §F and "Divergences fixed".

Addresses are level01 unless marked `Lnn`. Everything was read from `work/decomp/` and checked in the disassembly
(Ghidra `/levels/levelNN.elf`) wherever the decompiler drops arguments.

## Method (to regenerate)

A scratch tool (not committed: it is a one-off audit) does the following for each target function:

1. It finds the target's copies in the 19 overlays: the `tools/ghidra/names/clusters.tsv` hash first, then a fuzzy
   match. The fuzzy match compares masked words with `difflib`, masking j/jal targets, `lui` immediates, the
   `$gp`-relative and `%lo` offsets, and the `lui`-paired immediates.
2. It compares each copy's referenced data too: every `$gp` word (gp = 0x166c00) and every `lui`/`%lo` table, read from
   `extracted/levels/NN/overlay.bin`. Identical code can still carry per-level constants.
3. It scans every overlay for `jal` to each copy and maps each site to its enclosing function (the cluster table's
   function starts).
4. It groups the callers by their hash and prints a call skeleton: the sequence of known callees, such as FLASH70,
   FLASH1192, FIREBALL122, P11, LIGHT and BEAM. The skeleton exposes inline copies of an explosion compiled into
   class code.
5. It recovers the arguments at each call site from the disassembly with a small register tracker: float args in
   f12..f18, int args in a0..t3, then the stack words `0(sp)`, `8(sp)`, `0x10(sp)`.

The building blocks it scanned (all 19 overlays):

| block | copies | call sites | caller functions |
|---|---|---|---|
| `SpawnBeamExplosion` 0x273310 | 19, identical code and data | 362 | 305 |
| death explosion 0x273f50 | 19 | 101 | 85 |
| piece explosion 0x2742a8 | 19 | 31 | 30 |
| `FlashSpawn` 0x2c20e0 (class 0x70) | 19 | 466 | 127 |
| flash 0x309a68 (class 1192) | 19 (L13 0x305718 same hash) | 195 | 39 |
| fireball 0x2c4c20 (class 122) | 19 | 173 | 78 |
| light 0x2f3570 (class 0x27f) | 19 | 195 | 138 |

## The explosion map

| § | routine (game) | copies | callers (by code identity) | class | port | status |
|---|---|---|---|---|---|---|
| A | `SpawnBeamExplosion` 0x273310 (18 params) | 19 levels; code and data identical: only `%lo` relocations differ | engine code on all 19: the Visibomb 0x2cbda8, the R.Y.N.O. 0x2e5738 / 0x2e5a48, the body pieces `FxGroupUpdate` 0x30cd18; on 01 / 10 / 12: the drones 0x2e92b8, the Doom bots 0x2d5c10, the decoys 0x2d9fe8, the mine 0x2bfe40, the chicken 0x2defb0; level class code: the path enemies 0x2e6bf0 (×3), the flyer 0x2f4428, `FlyerPathDriver` 0x2f5168 (01, 03, 04, 14), the gunship 0x2f7728, the cutscene FX 0x30c190 (×2), the flyer wreck 0x30ba18 (unported, G-CLS-015); about 140 more class functions on the other levels (G-CLS-001) | one shared function with parameters | `creature::fx::beam_explosion` / `beam_explosion_shift` (`Beam` row per caller) | **same code, single port**; 2 divergences fixed (below) |
| B | the Bomb Glove's blast, inline in the bomb update 0x2c3300 | 19 levels (same code; `%lo` only) | the bomb 121 | **one source compiled three times with other constants** (k, the fireball divisor, the colour tables, the fireball class) | `classes::bomb::blast` + `BlastRow` `GLOVE_BLAST` | **merged** (was `bomb::explode`'s own copy) |
| B | the Suck Cannon burst 0x304798 | 19 levels, identical | the fired creature's landing 0x3051a8 (`react::land`) | same source as B: k = 1 + gold, n = 2 / 1, tables 0x20b820 / 0x20b838 (= the bomb's words) | `bomb::blast(GLOVE_BLAST, k, n, shift = gold)` | **merged** (was `react::burst`'s own copy) |
| B | Gemlik's tank burst L13 0x3073c8 | level 13 only | the tank 1261 (0x307b10) | same source as B: k = 1.5, green tables 0x1f5560 / 0x1f5578, green flash literals, its own fireball spawner, no shift calls | `bomb::blast(TANK_BLAST)` | **merged** (was `explosive_tank::burst`'s own copy) |
| B | fireball spawner 0x2c4c20 (class 122) | 19 levels | B's three blasts, A's debris (param_16), the Devastator missile 0x2c5b70, 29 hash groups | one function | `bomb::spawn_fireball(GLOVE_FIREBALL)` (`bomb::fireball` wrapper) | same code, single port |
| B | fireball spawner L13 0x30c138 (class 1634) | level 13 | the tank burst | a copy of 0x2c4c20 with other constants: random alpha, ambient, class, a `randf(2, 5)` growth, no gold bit | `bomb::spawn_fireball(TANK_FIREBALL)` | **merged** (was `explosive_tank::fireball`) |
| C | death explosion 0x273f50(size, light, moby, pos, sound) | 19 | 68 hash groups: the critters 0x2efc60, the glove update 0x2ed5d8 (the oldest mine), the barricade / vent / tethered-platform units, … | one source compiled three times (below) | `fx::spark_burst(DEATH_BURST)` = `fx::death_explosion` | merged |
| C | piece explosion 0x2742a8(size, light, moby, pos) | 19 | `FxGroupUpdate` 0x30cd18 (flag 2), 10 level functions | copy of 0x273f50: sprites 500000·size, **colour shift 1**, no shake, no sound | `fx::spark_burst(PIECE_BURST)` = `fx::piece_explosion` | merged; **divergence fixed** (colours) |
| C | glob burst 0x2fa1b0(size, light, moby, pos, sound) | level 01 only | the path enemies' glob 0x2fa4c0 | copy of 0x273f50: flashes 2·s (0x7f, 0, 0x40) / 1.5·s (0x20, 0, 0x20), no shake; its tables 0x20b4f0 / 0x20b508 and light template 0x1e3440 hold the same words | `fx::spark_burst(SHOT_BURST)` | **merged** (was `path_enemy::shot_burst`'s own copy) |
| D | the Devastator missile's explosion, inline in 0x2c5b70 | 19 levels | the missile 153 | different effect: streaks spread in the hit face's plane, fewer rings, a throttle that thins everything, 4 flashes | `classes::devastator_missile::explode` | separate (checked, faithful) |
| D | the Visibomb's burst 0x2cb968(m, 1) | 19 levels | the missile 172 0x2cbda8 (×5 after its `SpawnBeamExplosion`) | different effect: six type-13 puffs, up to 10 rings, 5 flashes with their own sizes and gating | `classes::visibomb::explosion` | separate |
| D | the crate's TNT burst, the tail of `CrateBreakFx` 0x2eb918 | 19 levels | the crates | different effect: rings (at most 6 when suppressed), 5 flashes gated on load, distance and suppression | `classes::crate_::break_fx` | separate |
| D | the mine's explosion, inline in 0x2bfe40 | 19 levels | the mine 74 | different effect: an area hit, then `SpawnBeamExplosion` in the air or 15 type-16 puffs on the ground, the water bubbles / scorch, the shake and light | `classes::mine::explode` (+ A) | separate |
| D | the gunship shell's blast 0x2f6a30 | 01, 14 | the gunship shells | different effect: two rounds of flashes (a second set beyond 100), 10 / 40 streaks | `classes::gunship` | separate |
| E | level class copies: L13 0x2ce688 (class 170: the Visibomb burst's code with other constants plus debris 0x2c8868); L16 0x2d41f8 / L18 0x2dde20 (class 638: rings + 5 flashes); L18 0x2ea800; L15 0x29af88, L16 0x2a0940 / L18 0x2a7a38, L17 0x2a2e88 (a single flash); L15 0x29d408 / L18 0x2aa0f8 (a missile 0x100 from the levels' hero code, an inline copy of D's Devastator explosion, G-TRI-016) | per level | their classes | copies of D, compiled into class code | — | not ported (class code, G-CLS-001) |
| F | `FlashSpawn` 0x2c20e0 (class 0x70) and 0x309a68 (class 1192; L13 0x305718 is the same hash) | 19 each | A, C, D (0x2c20e0); B only (0x309a68) | **one code, two constants**: the class and a `ticks(4)` head start (0x2c20e0) or none (0x309a68) | `classes::debris::flash_spawn_as(FlashKind)`: `FLASH_SHELL`, `FLASH_BOMB` | **merged** (was `debris::flash_spawn` + `bomb::flash`) |
| G | explosion light 0x2f3570 (class 0x27f) with the templates 0x1b0770 (C), 0x1b06d0 / 0x1b0720 (A), 0x20a930 (B, and the equal copies 0x20b7d0, 0x20a9b0, 0x20a890) | 19 | every explosion | one function; the templates are data | `fx::light_spawn` + `LIGHT_DEATH` / `LIGHT_BEAM` / `LIGHT_BEAM_GOLD` / `LIGHT_BOMB` | same code, single port |

**The spark colour tables.** The explosions read one 2 × 6-word table from many addresses: 0x20a090 / 0x20a0a8 (A,
C), 0x20a980 (B, bomb), 0x20b820 (B, burst), 0x20aa50 (D, Devastator), 0x20aab8 (D, Visibomb), 0x20b0b0 (D, crate)
and 0x20b4f0 (C, glob). All of them hold the same words (checked byte for byte). The port has one table,
`fx::SPARK_A` / `SPARK_B`, and the other modules alias it. The tank's 0x1f5560 is different: it is the same table with
red and green swapped.

**Where the explosions stand.** Every explosion routine that a ported class calls is ported once. The Walloper
has no explosion (its hits make sparkle bursts, `0x2bdb18`). The Morph-o-Ray's chicken burst and the Glove of Doom's
bots are callers of A. The intro crash is two calls of A. The reaction layer's burst is B.

## §A `SpawnBeamExplosion` 0x273310: coverage

| address / step | what it does | ported (file::fn) / not |
|---|---|---|
| param_17 = −1 | the throttle: 1 when the two frame loads 0x15f5d0 + 0x15f5d4 exceed 1.7 | `fx::beam_explosion_shift` |
| param_10 = 0 | the moby's position | callers pass the position (every ported caller has one) |
| param_1 > 0 | `0x26e7d8(param_2, tmpl, moby, 0x810001)`: +0x18 = 2, +0x19 = 1, +0x1a = the moby's class; `coll_sphere_mobys(param_1, pos, 0x10, moby, tmpl)`: every moby in the sphere is hit | `World::sphere_mobys` + `HitTemplate` |
| param_11 streaks | per streak `randf(15°, 80°)`, `rand_angle`, `randf(7, 10.5)`; `0x277b50` polar; z += 3·dt; colours `0x270fa8(0x4f007fff / 0x1f00007f, p18)`; life `rand_range(ticks 60, 120) − 10·throttle`; `PartType15Spawn(40000·scale, …, split 1, −1, −1)` | `fx::part15` |
| camera distance | `VecSub(cam 0x167240, pos)`, `FastVecLength` | `World::camera` |
| param_16 debris | 3 × `randf(±1)`; within 14 of the camera one fireball toward it (`(d/5)·dt` + `2d·dt`, +0.5·d up, clamp 10·dt, `rand_range(ticks 60, 90)`, type 0); then param_16 − 1 thrown out (`randf(0x3e860a92, 0x3fa78d36)` pitch, yaw, `randf(7, 10.5)`, z += 2·dt, `rand_range(ticks 60, 90)`, type `i % 3 == 0`) through 0x2c4c20 (gold 0) | `bomb::fireball` (`spawn_fireball`) |
| param_12 sparks | n = param_12, or trunc(d)/2 when d < 2·n; slower within 7; per pair `randf(8, 10)`, `randi(6)` ×2 through `0x270fa8`, lives `rand_range(ticks 15, 20) − 3·thr` / `(30, 45) − 5·thr`, a white pair `(5, 10) − 2·thr` / `(15, 20) − 3·thr` at half speed; base (0, 0, 8·dt·scale) | `World::part11` |
| param_13 puffs | 3 × `randf(±1)`, `randf(0, 3)`, length `scale·s·dt`, `randi(6)` ×2 shifted, life `rand_range(ticks 30, 45) − 5·thr`, `PartType08Spawn(200000, …)` | `fx::part08` |
| param_3 flashes | when there is a moby: colours 0x96 / 0x96 / 0x7f (0x46 / 0x46 / 0x3c with p18); beyond param_5 and load < 0.95: `ticks(16)` and `ticks(22)`; always `ticks(30)`; **the vector is sp+0x1b0 = (0, 0, 8·dt·scale)** | `debris::flash_spawn` (vector fixed 2026-09-29) |
| param_4 | `ticks(27)` white flash | `debris::flash_spawn` |
| param_15 shake | `FastBSphereCheck(10, (pos, 2))`: amplitude 0.4 − 0.0175·d (0.05 beyond 20) along up for `ticks(25)` | `World::shake_camera` |
| param_14 | `PlayClassSound(param_14, 0, moby)` when there is a moby and param_14 ≠ −1 | `World::play_sound` |
| param_7 light | with no throttle and `FastBSphereCheck(100, (pos, param_7))`: the radii from param_7 (≤ 0: 15), template 0x1b06d0 (0x1b0720 with p18) | `fx::light_spawn` |

The callers' arguments were re-read from the call sites (stack words included). Every ported `Beam` row matches, except
the crash's first call (fixed, below). The gold shift (param_18) comes from the gold flags in the Doom bots
(`lbu 0x14(0x13e520)`), the decoys, the mine (+0x11) and the chicken. Those flags are not mirrored (G-WPN-009); only
the chicken passes its own value (`beam_explosion_shift`).

## §B The Bomb Glove's blast (one source, three copies): coverage

`bomb::blast(row, id, pos, base, normal, k, n, dry, shift)`:

| step (0x2c3300 / 0x304798 / L13 0x3073c8) | what it does | ported / not |
|---|---|---|
| k | 0x2c3300 / 0x304798: `convert_integer_to_float(gold) + 1`; L13: the literal 1.5 | the caller's `k` (bomb 1: the gold glove is G-WPN-009; burst 1 + `react.gold`; tank 1.5) |
| base | bomb: its drift (the reflected direction ·2·dt, or the velocity at the fuse's end) scaled by k (`VecScale`); burst / tank: (0, 0, dt + dt)·k | the caller's `base` |
| camera | `VecSub(camera, pos)`, `FastVecLength` (0x167240; L13 0x1670c0: the same record at that level's address) | `World::camera` |
| 10/n low fireballs | 2 × `randf(±1)` in xy, `FastVecDot` with the normal, `FastVecNormalize(dot, normal)` subtracted, `FastVecNormalize(randf(0, 1), normal)` added, `FastVecNormalize(k·randf(3.5, 6.5)·dt)`, + base; `rand_range(ticks 60, 120)`; fireball type 0 (gold in +0xbc bit 1) | `blast` (`creature::set_len3` = `FastVecNormalize`'s `v·(l/√d)`) |
| bomb only: under water | the lows only, then the deep burst | `blast(dry = false)`; `bomb_water::deep_burst` |
| 4/n high fireballs | as the lows with `randf(6.5, 10)`, `rand_range(ticks 60, 90)`, type 1 | `blast` |
| camera fireball | 3 × `randf(±1)`, `(d/5)·dt` + `2d·dt` toward the camera with 0.5·d up, `ClampVecLength(10·dt)`, `(ticks 60, 90)`, type 1 | `blast` |
| rings | `trunc(d) + 1` within 6, else 4; slower by (7 − d)·dt within 7; `randf(8, 10)·k·dt`; `randi(6)` ×2 through `0x270fa8(c, gold)` (L13: no shift call); `rand_range(ticks 15, 20)`, `(25, 30)`; `PartType11Spawn(k·400000, speed, pos, base, …, 0, 0)` | `blast` → `World::part11` |
| flashes | beyond 9: `ticks(15)` (0x7f, 0x7f, 0x7f, 0x20) and `ticks(24)`; then `ticks(20)`, `ticks(27)`, `ticks(29)`; sizes k·4, 4, 4, 3.5, 3; colours through `0x270f48(&r, &g, &b, gold)` except the first; vector = base; `0x309a68` (L13 0x305718) | `blast` → `debris::flash_spawn_as(FLASH_BOMB)` |
| fireball spawn 0x2c4c20 / L13 0x30c138 | `CreateMoby`, drawn, distances 0xff, alpha 0x80 (L13: `rand_range(0x40, 0x80)` first), ambient 0x7f (L13: 0x20 / 0x60 / 0x10), the whole position, velocity, 2 × `randf(2π·dt, 4π·dt)`, life ×2, type 0: scale ·`randf(0.5, 0.75)`, +0x1c = scale, L13: scale ·`randf(2, 5)`, +0xbc = type \| gold << 1 (L13: type), `MobyBuildMatrix` | `bomb::spawn_fireball` + `GLOVE_FIREBALL` / `TANK_FIREBALL` |
| then, per caller | bomb: the sphere hit before the fireballs, class sound 0 (0x16 by class under water), the scorch, the shake, the light 0x20a930, the growing sphere; burst: the shake, the light 0x20b7d0, the gold area hit; tank: nothing (its caller plays sound 0 and leaves a scorch) | each caller (unchanged) |

**Evidence that it is the same code.** The three routines have the same call skeleton: VecScale, VecSub,
FastVecLength, 10 × (randf, randf, FastVecDot, FastVecNormalize, VecSub, randf, FastVecNormalize, VecAdd, randf,
FastVecNormalize, VecAdd, ticks, ticks, rand_range, fireball), the highs, the camera ball with ClampVecLength, the ring
loop (randf, randi, [shift], randi, [shift], 4 × ticks, 2 × rand_range, P11), then five flashes with their `ticks`.
The operands are the same, except the constants listed in the rows. The burst also carries the `10 / n` division
and the shift calls. The tank's literals are the bomb's times 1.5 (6, 5.25, 4.5, 600000), and its tables are the
bomb's colours with red and green swapped. Float order: the game computes `(k·s)·dt` (bomb, burst) and `(s·1.5)·dt`
(tank). These are the same product, so the merged `k·s·dt` is bit-exact for all three.

**Not the same: the fireball updates.** 0x2c4d88 gives the high fireballs a type-4 smoke puff each tick (gold colours
in a second path). L13 0x30c3b8 gives them two passes of type-2 trails and a `CollLine_Fix` hit that leaves a scorch.
These are different effect code, so the two stay separate updates (`bomb::fireball_update`,
`explosive_tank::fireball_update`).

## §C The spark burst (0x273f50 / 0x2742a8 / 0x2fa1b0): coverage

`fx::spark_burst(row, size, light, moby, pos, sound)`:

| step | what it does | ported / not |
|---|---|---|
| sparks | 3 pairs: `randf(8, 10)·dt`, `randi(6)` ×2 (0x2742a8 through `0x270fa8(c, 1)`), `rand_range(ticks 15, 20)`, `(25, 30)`, `PartType11Spawn(size·sprite, randf·dt·size, pos, 0, …)`; sprite 400000 (0x2742a8: 500000) | `spark_burst` |
| flashes | with a moby: 0x273f50 / 0x2742a8 4·s `ticks(20)` (0x7f, 0x40, 0, 0x30), 3·s `ticks(29)` (0x60, 0x20, 0, 0x20), both through `0x270f48(…, 1)` in 0x2742a8; 0x2fa1b0 2·s (0x7f, 0, 0x40, 0x30), 1.5·s (0x20, 0, 0x20, 0) | `spark_burst` → `debris::flash_spawn` |
| shake | 0x273f50 only: `FastBSphereCheck(10, (pos, 2))` → amplitude size·0.1 for `ticks(20)` | `SparkBurst::shake` |
| sound | 0x273f50, 0x2fa1b0: the moby's class sound when its state is not 0xfe / 0xfd and sound ≠ −1 | `SparkBurst::sound` |
| light | light ≠ 0: the radii (≤ 0: 13) into 0x1b0770 (0x2fa1b0: 0x1e3440, the same words), `0x2f3570` | `fx::explosion_light` |

The three are copies of one source (same skeleton and operands; the diff is the constants above, the shake block and
the shift calls).

## §F The flash spawners 0x2c20e0 / 0x309a68

The two functions are word-for-word the same code except for three differences: the class immediate
(`li a0, 0x70` / `0x4a8`), the branch offsets, and the timer tail. 0x2c20e0 stores `T − ticks(4)` and the size
`(T − timer)·full / T`. 0x309a68 stores T and leaves the size 0.

| step | what it does | ported |
|---|---|---|
| create | `CreateMoby(class)` | `flash_spawn_as` |
| draws | 3 × `randf(−π, π)` | yes |
| moby | state 0, alpha a, drawn, distances 0xff, ambient (r, g, b) via `0x2650d0`, +0x18 = class scale · size, scale 0, +0x10 parent, the whole position, +0x00 the vector, the rotation, +0x14 T, +0x1e a | yes |
| timer | 0x2c20e0: +0x1c = T − `ticks(4)`, scale `(T − timer)·full / T`; 0x309a68: +0x1c = T | `FlashKind::head` |
| matrix | `MobyBuildMatrix` | yes |

Both classes run `FlashUpdate` 0x2c22a8 (`debris::flash_update`), which was one port already.

## Divergences fixed (2026-09-29)

1. **The intro crash's fireballs.** The first crash call in `CutsceneFxUpdate` 0x30c190 passes param_16 = 60 (`li v0,
   0x3c; sw v0, 0(sp)` at 0x30c69c / 0x30c6c4; the decompiler drops the stack words). So 60 fireballs 122 fly out of
   the crash. The port had 0 (`cutscene_fx::CRASH_BEAM.debris`). The crash frames from 132 on now show the small
   tumbling fireballs.
2. **The piece explosion's colours.** 0x2742a8 passes shift 1 to `0x270fa8` / `0x270f48` (`li a1, 1` / `li a3, 1`), so
   its sparks and flashes swap red and green. The port said "identity" and used the death explosion's colours. This
   affects the broken props' flag-2 pieces and the body pieces with flag 2.
3. **`SpawnBeamExplosion`'s flash vector.** Its flashes take sp+0x1b0 = (0, 0, 8·dt·scale), the sparks' base, as
   their +0x00. The port passed zero. `FlashUpdate` does not read it, so nothing visible changes.
4. **Small fidelity points taken in the merges.** Bit-identical frames confirm that none of these changed a frame
   (below):
   - The bomb and the burst used raw tick counts where the game calls `ticks()`. At the NTSC timer scale 1 these are
     equal.
   - The bomb's `setlen` computed `(a·l)/n`. The game's `FastVecNormalize` computes `a·(l/√d)`.
   - The bomb flattened onto the face with `n·d`. The game uses `FastVecNormalize(d, n)`.
   - The fireballs and the 1192 flashes kept their position's w. The game copies the whole quadword.
   - The gold Suck Cannon's burst skipped the colour shift. The shift is ported now, but it cannot be reached until
     0x13e529 is mirrored (G-WPN-009).

## What stays separate, and why

- **D, the Devastator missile's explosion:** it spreads its streaks in the face's plane, uses fewer rings (d/2) and
  a throttle that thins everything, and has 4 flashes. It is not a `SpawnBeamExplosion` call, and its operands differ
  throughout.
- **D, the Visibomb's burst and the crate's TNT burst:** they have the same shape (rings plus 5 flashes), but the
  counts, flash sizes and gating differ in the code, not only in the constants. The crate's burst is part of
  `CrateBreakFx`.
- **D, the mine's explosion:** an area hit, then A or the ground puffs, then the water branch.
- **D, the gunship shells:** their own flash rounds and streaks.
- **The fireball updates** (§B).

## In the port

| file | what |
|---|---|
| `crates/rc-game/src/moby_update/classes/bomb.rs` | `blast`, `BlastRow` (`GLOVE_BLAST`), `BlastFlash`, `spawn_fireball`, `FireballRow` (`GLOVE_FIREBALL`); `explode` calls `blast` |
| `crates/rc-game/src/moby_update/creature/react.rs` | `burst` calls `bomb::blast(GLOVE_BLAST, k, n, shift = gold)` |
| `crates/rc-game/src/moby_update/classes/units/explosive_tank.rs` | `TANK_BLAST`, `TANK_FIREBALL`; `burst` calls `bomb::blast` |
| `crates/rc-game/src/moby_update/classes/debris.rs` | `FlashKind`, `FLASH_SHELL`, `FLASH_BOMB`, `flash_spawn_as` (`bomb::flash` removed) |
| `crates/rc-game/src/moby_update/creature/fx.rs` | `SparkBurst` (`DEATH_BURST`, `PIECE_BURST`, `SHOT_BURST`), `spark_burst`; the beam flashes' vector; the one spark table |
| `crates/rc-game/src/moby_update/classes/path_enemy.rs` | `shot_burst` calls `fx::spark_burst(SHOT_BURST)` |
| `crates/rc-game/src/moby_update/classes/cutscene_fx.rs` | `CRASH_BEAM.debris` = 60 |
| `crate_.rs`, `visibomb.rs`, `devastator_missile.rs` | their spark tables alias `fx::SPARK_A` / `SPARK_B` |

**Tests.**
- `bomb::tests`:
  - `blast_glove_row`: counts, flashes (sizes, T, colours, no head start) and draws; within 9; under water;
  - `blast_suck_cannon_row`: halved; gold k = 2 with the shift and the gold bit;
  - `blast_tank_row`: class 1634, alpha, ambient, growth, the green flashes, draws;
  - the existing rand-ledger test (`dry_explosion_rand_stream_matches_the_game_ledger`) is unchanged.
- `creature::fx::tests`:
  - `flash_kinds`;
  - `spark_burst_death_row`, `spark_burst_piece_row_swaps_red_and_green` and `spark_burst_shot_row`;
  - `beam_flashes_carry_the_base_vector`;
  - `crash_beam_throws_sixty_fireballs`.

**Frames** (scratch `expl/shots/{before,after}/<scene>/{A,B}/`). Two runs per scene give identical PNGs, before and
after. Before and after are byte-identical for the bomb (frames 30..70), the Devastator (40..150), the Visibomb
(120..140), the tank (L13, 60..90) and the Suck Cannon burst (262..300). The crash (120..150) changes from frame 132
on (divergence 1). The Novalis hero digest with `RC_HERO_DIGEST_NO_IDLE=1` is unchanged.

**Not ported (filed):**
- the E copies in level class code (G-CLS-001; L15 / L18's missile, G-TRI-016);
- the flyer wreck 0x30ba18 (G-CLS-015);
- the gold versions (G-WPN-009): the Bomb Glove's k = 2, sound 1 and gold smoke; the gold shift arguments of the Doom
  bots, the decoys and the mine.
