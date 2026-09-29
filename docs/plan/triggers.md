# Trigger volumes and moving platforms

Addresses are **level01.elf** unless marked; every function named here is engine code present in all 19 level
overlays (tools/ghidra/names/clusters.tsv) unless its row says otherwise. gp = 0x166c00. Hero globals: 0x13f3d0
position (feet), 0x13f420 body point, 0x13f64c ground moby, 0x13f65e air ticks, 0x1413d4 state, 0x1413dc
movement group. Camera position 0x167240. dt 0x15ed6c.

Port: `crates/rc-formats/src/volumes.rs` (sections), `crates/rc-game/src/moby_update/triggers.rs` (the shared
tests and the carrier helpers), `crates/rc-game/src/moby_update/classes/path_platform.rs` (first consumer, lift
726). Standard `f32` (no PS2 float model): a point exactly on a face can round the other way.

## 1. Answer in brief

- The game has **four shape tables** (cuboids, spheres, cylinders, pills) with one 0x80-byte record format, and
  uses **paths** as 2-D polygons for area tests. Classes hold **indices** into these tables in their pvars
  (s32, −1 = none) and call **three stateless point tests** (cuboid 0x274820, cylinder 0x2748f8, sphere
  0x2749b0) and a **polygon test** (0x26e6c0). A fifth test (0x20ff18, sphere-vs-shape) serves only the camera
  collision grid.
- There is **no shared enter / exit logic**. Each class keeps its own "was inside" word or one-shot latch in its
  pvars (§4). The port therefore ports the tests once and leaves edges to the classes, as the game does.
- On Novalis almost every test is Ratchet's feet against a cuboid (checkpoint 805, camera trigger 737, mission
  NPCs 730/790, help director 1341, dropship 666, gunship 688, critter 1818, path enemy 459); the water manager
  751 tests the camera.
- **Moving platforms** are a separate shared mechanism: a carrier (mode 0x20) publishes its per-tick
  displacement in a *platform block* its pvar+0x08 points at (`CarryRiders` 0x2755f8), and the hero code
  (`HeroPlatformUpdate` 0x249618) moves Ratchet with the moby he stands on. The hero side is not ported (§5).
- The requested consumer: **no elevator uses a volume**. Elevators 703/715 ping-pong on a timer. The path lift
  726 is the platform class that calls the cuboid test (an appear cuboid and an activation cuboid), but the
  Novalis instance has both at −1: there it is called by distance or sent by riding it. It is ported whole,
  cuboid branches included (Kerwan's instance 3 and Crocker's instance 1 use them), and verified on Novalis by
  riding (§6, §7).

## 2. Sections, records and runtime tables

Loader `InitLevelRenderGlobals` 0x255958 (after the grind paths). Gameplay header pointers:

| ptr | section | runtime count / base | stride | readers |
|---|---|---|---|---|
| 0x60 | cuboids | 0x1600f0 / **0x1600ec** | 0x80 | 0x274820, 0x20ff18, placement readers (§4) |
| 0x64 | spheres | 0x1600f8 / 0x1600f4 | 0x80 | 0x2749b0, 0x20ff18, camera push-out 0x30f358, grid debug 0x314c78 |
| 0x68 | cylinders | 0x160100 / 0x1600fc | 0x80 | 0x2748f8, 0x20ff18 |
| 0x6c | pills | 0x1600e8 / 0x1600e4 | **0x90** | 0x20ff18 only |
| 0x70 | paths | 0x160104 / `0x1b0930[i]` | — | 0x26e6c0, spline sample 0x272e28 / project 0x2726c8, classes |
| 0x74 | grind paths | 0x15f710 / 0x15f70c | 0x20 | grind code 0x314e98 / 0x315358 (5 levels) |
| 0x84 | camera collision grid | 0x15ef80 | — | 0x20fdb0 (64×64 cells of 16 units, primitives `{bsphere, type 3/5/6/7, index}`) |

Each shape section is `s32 count, pad[3]`, then `count` records (0x80 bytes):

| off | contents |
|---|---|
| 0x00 | local → world rows (VU row-vector form): `world = l.x·r0 + l.y·r1 + l.z·r2 + r3`; r3 = centre |
| 0x40 | world → local rows for `p − centre`: `l = d.x·i0 + d.y·i1 + d.z·i2` (3 rows, no translation) |
| 0x70 | Euler angles of the matrix (read by placement users, e.g. 737's `HeroTeleport` / `CameraScript`) |
| 0x7c | unused (0) |

Canonical shapes in local space: cuboid `|l.x|, |l.y|, |l.z| ≤ 1`; sphere `|l| < 1`; cylinder `|l.xy| < 1`,
`|l.z| ≤ 1` (axis local z); pill = cylinder plus cap spheres at local z = ±1 of radius runtime +0x80.

**Pill quirk.** The loader copies **0x90 bytes from each 0x80-byte source record**, so the runtime +0x80 (the
cap radius) is the first word of whatever follows the record in the file. No RAC1 level has a pill; the parser
keeps that word anyway (`Volumes::pill_tail`).

**Paths** (0x70): `{count, data_offset, data_size, pad}`, `count` s32 offsets at +0x10 into the data; each path
is `s32 n, pad[3]` then n × (x, y, z, w). **Grind paths** (0x74): the same header, then `count` × 0x20 records
`{f32 bsphere[4], s32 flag (0/1), pad[3]}`, then `count` offsets; runtime record `{bsphere, spline pointer
+0x10, flag +0x14}`.

**Counts on the disc** (`volumes::tests::all_levels_disc`; every level parses, every stored inverse is the
inverse of its matrix to 2e-3):

| level | 00 | 01 | 02 | 03 | 04 | 05 | 06 | 07 | 08 | 09 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| cuboids | 14 | 83 | 86 | 74 | 63 | 76 | 109 | 47 | 29 | 51 | 74 | 50 | 88 | 69 | 177 | 91 | 72 | 147 | 78 |
| spheres | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cylinders | 0 | 0 | 3 | 5 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 2 | 0 | 0 |
| pills | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| paths | 42 | 82 | 32 | 112 | 84 | 103 | 108 | 66 | 121 | 87 | 171 | 65 | 137 | 56 | 141 | 110 | 159 | 43 | 125 |
| grind paths | 0 | 0 | 0 | 3 | 0 | 0 | 1 | 0 | 7 | 0 | 0 | 0 | 0 | 0 | 5 | 1 | 11 | 0 | 9 |

Other boxes with their **own** matrix (not a table index): fog transition zones (section 0x80, 4-row inverse,
`rc_formats::gameplay::FogZone`, world_animation.md §5) and the sound-instance boxes (section 0x0c records:
centre +0x40, inverse rows +0x50; audio.md §3.3). The same box math applies (`triggers::box_local` +
`in_unit_box`).

## 3. The tests

All tests subtract the centre (`VecSub` 0x2211b8, xyz) and apply the inverse rows (`MatrixMulVec3`
0x2215e0: `vmulax/vmadday/vmaddaz` over rows +0x40/+0x50/+0x60, then `vmaddw` with vf0 on the zeroed w).

| fn | port | math | index guard |
|---|---|---|---|
| 0x274820 `PointInCuboid(p, i)` | `point_in_cuboid` | `−1 ≤ l.x ≤ 1`, same for y, z (`c.le.s` both sides) | only `i == −1` → 0 |
| 0x2748f8 `PointInCylinder(p, i)` | `point_in_cylinder` | `sqrt(l.x² + l.y²) < 1` (0x221318, `vsqrt`) and `−1 ≤ l.z ≤ 1` | `i < 0` → 0 |
| 0x2749b0 `PointInSphere(p, i)` | `point_in_sphere` | `sqrt(l.x² + l.y² + l.z²) < 1` (0x2212e8) | `i < 0` → 0 |
| 0x26e6c0 `PointInPathPolygon(p, pts, n)` | `point_in_path_polygon` | even-odd: for edge k → k+1 (last → 0), toggle when `p.y ∈ (min(y), max(y)]` and `a.x + (p.y − a.y)/(b.y − a.y)·(b.x − a.x) < p.x`; z ignored | `n ≤ 0` → 0 |
| 0x20ff18 `(r, centre, prim)` | `sphere_touches_shape` | type 5: `|c − s.centre| < |row0| + r`; 3 / 6: the point tests on the centre (r unused); 7: cylinder, else a cap `s.centre ± row2` within the +0x80 radius | — |

The port additionally returns false for an out-of-range index (the game would read outside the table).
0x20ff18 is reached only through the camera-collision grid lookup 0x20fdb0 (cell `(x/16, y/16)`, primitive
bounding circle `|c − b.xy| < b.r + r` first); its callers are the camera (0x30f358 sphere push-out, 0x30f468,
0x316880). It is ported as a test; the camera collision itself is not part of this work.

### Callers (level01)

Test point: **H** = Ratchet's feet 0x13f3d0, **C** = camera 0x167240, **B** = hero body 0x13f420, other = a
moby's position. Offsets are in the caller's pvar block.

| caller | class | test | point | volume index |
|---|---|---|---|---|
| 0x2b9eb0 `PathPlatformUpdate` | 726 | cuboid ×3 | H | +0xcc appear, +0xc0 activation |
| 0x300220 `CheckpointTriggerUpdate` | 805 | cuboid | H | +0x00 |
| 0x2fb5b0 `CameraTriggerUpdate` | 737 | cuboid | H | +0x20 (+0x24 / +0x28 are placement cuboids) |
| 0x2fad68 `MissionNpcUpdate` | 730 / 790 | cuboid | H | +0x10 |
| 0x2f4960 `DropshipUpdate` | 666 | cuboid | H | pvar (trigger) |
| 0x2f7728 `GunshipUpdate` | 688 | cuboid ×5 | H, C | +0x170 (H), +0x188 (C), +0x18c (H), linked +0x08 (H and C) |
| 0x2fd0e8 `WaterSurfaceFxUpdate` | 751 | cuboid ×7 | C | +0x00..+0x18 (zones) |
| 0x308bd8 `TeleporterPadUpdate` | 1135 | cuboid | H | +0x04 |
| 0x30acb8 `HelpHintDirectorUpdate` | 1341 | cuboid ×7 | H | +0x20, +0x24, +0x30, +0x38, +0x3c |
| 0x30df40 `MouseCritterUpdate` | 1818 | cuboid ×2 | H, critter | +0x40 |
| 0x2e6bf0 `PathEnemyUpdate` | 459 | cuboid | H | +0x1c0 |
| 0x2edca0 `AmoeboidUpdate` | 865/866 | cuboid | B | +0x25c |
| 0x30b618 `WanderingLightUpdate` | — | cuboid | own point | +0x80 |
| 0x2f2eb8 `NearestAreaMarkerUpdate` | — | polygon, cuboid | a candidate point | path, +0x14 |
| 0x302648 `RcRangeLimiterUpdate` | 832 | polygon, cuboid | the RC vehicle | path, +0x0c |
| 0x274df8 / 0x274b78 target select | (enemy AI) | cuboid list (first n−1) or polygon | H and candidate mobys | caller's list |
| 0x318c40 camera region test (14 levels) | camera records (0x15ef50 + i·0x20, pvar +0x1c) | cuboid +0x0c, cuboid +0x4c, cylinder +0x10, sphere +0x08, polygon path +0x14, in that order, any → inside | H | camera pvars |

Cuboids are also read as **placement markers** (centre +0x30, Euler +0x70, never tested): 737 (camera and
teleport targets), 730/790 (drop-in points), 760 (FX field extent), 688, 750, 459, `GameStateUpdate`,
0x3087e0, 0x309cd0, 0x2f9000.

## 4. Edges: kept by each class

The helpers return "inside now"; nothing in the engine remembers the previous answer. Patterns seen:

- **Rising edge, re-armed on exit** — checkpoint 805: `prev = P[5]; P[5] = inside; if inside && !prev` →
  checkpoint (mission done, respawn point from its position, death bits cleared). Leaving writes P[5] = 0.
- **One-shot latch that clears the index** — lift 726's appear cuboid (+0xcc = −1 on entry) and activation
  cuboid (+0xc0 = −1 and the spawn id's death bits, so later visits start latched); mission NPC 730/790
  (unhide on entry).
- **Sticky flag** — camera trigger 737: entering, or moby +0xbc = 1 (already fired), runs the camera script.
- **Inside flag with exit side** — music box (sound instance class 6, 0x31a128): inside sets pvar[2] = 1;
  on the first outside tick with pvar[2] set it requests track pvar[1] when leaving through +x (`l.x > 0`),
  else pvar[0], and clears the flag.
- **Level-triggered** — water 751 (zone active while the camera is inside), camera region 0x318c40.

## 5. Moving platforms (carry)

- **Carrier:** moby mode bit 0x20 (instance mode bits 0x20, Wrench's `HAS_SUB_VARS`) and pvar+0x08 = the
  platform block (a relative pointer fixed up by the loader; the port keeps the block-relative offset, 0x60 for
  726 / 703 / 715). `FUN_00275290(m)` returns it (0 = not a carrier). Port: `triggers::platform_block`.
- **Publish:** `CarryRiders(block, delta, rot_old, rot_new)` 0x2755f8: block +0x00 = Euler of
  `EulerToMatrix(rot_old)ᵀ · EulerToMatrix(rot_new)` (`FUN_002721f0`), +0x10 = `delta` (4 words). Callers pass
  `pos − old_pos` and their own +0x40 twice, so the rotation part is the identity: Euler (0, 0, 0).
  Port: `triggers::carry_riders`, read back with `triggers::platform_delta`.
- **Ride test:** `HeroOnMoby(m)` 0x277fb8: in group 3 or state 0x1c, `0x13f848 == m` (the ledge moby probe B
  records, `Hero::ledge_blk.moby`); otherwise `0x13f65e == 0 && 0x13f64c == m`. Port: `World::hero_on_moby`.
- **Hero side (ported: `rc-game/src/hero/platform.rs`):** `HeroPlatformUpdate` 0x249618, called from the move pipeline
  0x233de0. Outline:
  1. Pick the platform 0x13f6b0: the ground moby when grounded; in state 0x1c or group 3 the attach moby
     0x13f848; while airborne keep the last one, with a fade by air ticks (`ticks(120)`).
  2. Return unless it is a carrier.
  3. When grounded and idle (group 0 or 0xc, no carried momentum 0x13f4a0, 0x13f546 = 0) or in states
     0x18/0x19, record Ratchet's point in the platform's local space once (0x13f660, 0x13f680 via
     `FUN_00275528`: `(p − m.pos)·m.rowsᵀ`), flag 0x13f6b4 bit 0; bit 1 records the facing point (0x13f670).
  4. Each tick: the carried point `FUN_002752c0(p) = R(Δrot)·((p + Δ) − m.pos) + m.pos` (port:
     `triggers::carry_point`), or, when attached (bits 0/1), the recorded local point mapped back through the
     platform's current rows `FUN_002753b0` (`l·rows + m.pos`; with block flags bit 2 clear, when the hero
     moby's class slot +0x22 is non-zero and below the platform's, plus the platform's displacement clamped
     to length 1, an update-order correction). The yaw 0x13f3e8 turns with the platform.
  5. Correction = carried − current position; > 2 units drops the attachment; < 1e-4 snaps. It is added to
     0x13f440 (the platform delta the move pipeline adds to the position; the port has the field, always 0)
     and 0x13f44c gets the yaw change. Airborne with block flag bit 1 clear, the last delta decays
     (`Approach` by 25·dt² vertically, 2·dt² horizontally).

  **What the hero port needs** (for 726, 703, 715 and any carrier): in the move pipeline, before the ground
  probe, if the ground moby (or the last one while airborne, faded) is a carrier, add
  `platform_delta(m).displacement` (726 / 703 / 715 never rotate) to `0x13f440`, i.e. to the position, and
  keep the local-attachment path of step 3–4 for idle Ratchet. The mobys run before the hero in a tick, so the
  block always holds this tick's move. The two writes 726 makes while ridden when its pvar+0xc8 = 0 (0x13f544 edge
  brake = 4, 0x13f542 jump lockout = 4) go through the hero-block write channel (`services::HeroFields`, applied by
  the tick after the moby loop; every disc instance has 1, so none happen in RAC1).
- **Moby ledges:** probe B accepts a ledge top on a moby when its pvar record (`FUN_002711f8`: pvar+0x00, a
  block-relative pointer) has bit 0 of the u16 at +0x1e, or its platform block's flags +0x3c bit 0
  (`triggers::record_ledge_flag`, `HeroWorld::moby_ledge_flag`).

## 5a. Crank-driven objects (a third mechanism: a progress link)

The bolt crank 280 (`0x2e0c68`; levels 1, 4, 8) drives what it opens neither by a volume nor as a carrier: each
driven moby holds the crank's **moby index** in its own pvar s32 +0x00 (a loader moby link, −1 = none), checks that
moby's class is 280 (`+0xa6 == 0x118`) and reads the crank's pvar f32 +0x00, its **progress** 0..1 (turned angle /
(turns · 2π)), every tick. The driven moby maps it onto its own motion; the crank knows nothing of its consumers.
A crank let go before it is done unwinds (1/60 a tick) and the consumers follow back.

| consumer | fn | motion | carrier? |
|---|---|---|---|
| 641 rotator (Novalis #672, crank #297) | `0x2f4348` | Euler x = start + wrap((pvar+0x08° − start) · progress); sets the crank's update distance 0xff | no |
| 665 slider (Novalis door pairs #683 / #684 ← #296, #685 / #686 ← #297) | `0x2f4710` | x, y = start + (pvar+0x0c / +0x10 − start) · progress (only when both targets ≠ 0); loop sound 0 while moving, sound 1 at 0 / 1 | no |
| Eudora `0x2c6858`, `0x2c6bb8`, `0x2cdda0` (a lift: z − 15 · progress), `0x2e4418` (help) | level04 | not ported | — |

None of them has a platform block, so `HeroPlatformUpdate` never carries Ratchet on them (as in the game); they move
under the scheduler's `MobyBuildMatrix` and grid re-registration like every moby, so their collision follows. Port:
`moby_update/classes/bolt_crank.rs` (the crank, 641, 665; `bolt_crank::progress`), the hero side
`hero/crank.rs` (docs/plan/hero_states.md "Bolt crank").

## 6. First consumer: path lift 726

`crates/rc-game/src/moby_update/classes/path_platform.rs` (pvar table in its module doc), registered as
`ClassUpdate::PathPlatform` (0x2b9eb0). Whole function, from the disassembly:

1. pvar+0xb4 = −1 → return. Appear cuboid (+0xcc): outside → mode |= 0x41, no collision, update distance
   0xff, return; inside → shown, collision back, +0xcc = −1.
2. Activation (+0xc0): −1 → active; else inside, or spawn id's `0x1bbb04` byte, or its level death bit →
   set both death bits, +0xc0 = −1, active; else inactive (calls and rides ignored).
3. `FastDecTimer(+0xba)`. Init (state 0): state byte = pvar+0xa0 (0 → at t = 1, else t = 0), position = that
   end's path point (all four words), update / draw distance 0xff, voice −1.
4. Waiting at an end (+0xbc = 0 at t = 1, 1 at t = 0), when active: depart when (re-armed +0xb8 and
   `HeroOnMoby`) or Ratchet is nearer the other end (`VecDistance` 0x221360, 3-D); +0xac = ±1 / scale(time·60),
   +0xb8 = 0.
5. Moving (2): Ratchet riding with pvar+0xc8 = 0 → hero hook (counted). Wait timer → pause (step 0, voice
   released). Descending (`+0xbc < 0`) with Ratchet within 2.83 in XY, `|Δz| < 4` and more than 0.9 below →
   30-tick pause. Else the loop sound 0 (flags 4) when no voice, step += full·dt capped at full, t += step;
   `|t − 0.5| > 0.5` → arrive (state 0 / 1, t = 1 / 0, step 0, 15-tick wait, voice released). Position =
   piecewise linear on the path at `(n−1)·t`; +0xbc = z change.
6. `CarryRiders(+0x60, pos − old)`. Not ridden, more than 2 away in XY and no wait → re-armed (+0xb8 = 1).

Novalis: instance 1, spline 40 (50 points) from (161.70, 146.01, 60.00) on the landing-pad plateau down the
cliff to (174.50, 159.50, 41.00), travel 4 s, starting at the top (t = 0), no cuboids, +0xc8 = 1. A call or a
ride takes 271 ticks (60-tick ramp, then 1/240 of the path per tick). Kerwan (level 3) has three, one with
activation cuboid 7; Crocker (level 13) one with appear cuboid 16.

**Tests.** `triggers::tests` (faces inclusive / strict per shape, index guards, polygon edge rule and a concave
polygon, the camera-grid variant with a pill, the carry block, Novalis checkpoint / mission cuboids and
agreement with the water port's PS2-float cuboid test off the faces); `path_platform::tests` (wait / call /
ramp / arrival / call back, re-arm and ride, air ticks, pause above Ratchet, activation latch into the death
bits and a later visit, appear cuboid, the Novalis lift from the disc: 271 ticks, ends exactly on the last
point).

## 7. Engine check (Novalis)

`RC_SCENE=0 RC_LEVEL=01 RC_PLAY_SCRIPT="0-130:stick 0.33 -0.94" RC_PLAY_TRACE=1 RC_SCREENSHOT_FRAME=N`: Ratchet
runs from his spawn (162.53, 136.39, 60) onto the lift waiting at the top of its path (the landing-pad
plateau's edge).

- Load: `scheduler-driven statics per class {…, 726: 1, …}`; the load pass initialises it at t = 0.
- Tick 0: Ratchet more than 2 away in XY → re-armed (+0xb8 = 1). Ticks 110–114: he lands on its deck (the hero's
  ground probe finds the lift's class collision: ground moby = the lift, air ticks 0) → `HeroOnMoby` → it
  departs (state 2) and ramps up.
- Frame 100 (before): Ratchet a step from the lift, which sits at the top. Frames 200 / 300 (after the hero
  carry, P1 `hero::platform`): the lift is going down the cliff face with Ratchet idle on its deck, pinned in its
  local space (carry flags 3): (164.75, 152.39, 57.96) at tick 200, (169.57, 157.76, 48.19) at tick 300; he
  arrives with it at the bottom (tick 384). Before the carry he was left behind when the deck slid from under
  him at tick ~155 and fell to the meadow. Headless: `tests/hero/hero_platform_novalis.rs`, the same positions.
- A headless run of the same script (the port without Bevy) has the lift arrive at the bottom
  (174.50, 159.50, 41.00) at tick ~386, state 0.
- Determinism: two runs to frame 200 give byte-identical screenshots (md5 6eb46855…) and identical per-tick
  traces.

## 8. Not ported / open

- ~~`HeroOnMoby`'s attach-moby branch~~ (ported: the ledge moby `Hero::ledge_blk.moby`).
- The camera-collision grid and its users (0x20fdb0, 0x30f358, 0x318c40), the grind rails.
- `CarryRiders` with a real rotation change (no caller has one).
- The sound voice table: a voice counts as alive until released; without a sound sink the lift requests its
  loop sound every moving tick.
- Every other consumer in §3 (their classes are not ported yet); they call `World::in_cuboid` & co. when they
  are.

## 9. Cinematic consumers (2026-09-27, docs/plan/cutscenes.md)

- **Camera trigger 737** (`classes/camera_trigger.rs`, 0x2fb5b0): cuboid P[8] on Ratchet's feet **or a command**
  (moby+0xbc = 1 with the hold time in P[11], set by another class) → the cutaway (hero 0x72, script camera at cuboid
  P[9], letterbox); the "sticky flag" of §4 is that command byte. Novalis: 860 (cuboid 42, and commanded by 790), 863
  (commanded by 790), 859 / 861 / 862 dormant.
- **Mission NPC 730 / 790** (`classes/mission_npc.rs`, 0x2fad68): drop-in cuboid +0x10 (42) → commands 860; its end
  commands 863 and the hinged bridge 746 (+0xbc = 2). Placement cuboids: +0x30 the checkpoint record (55).
- **Gunship 688**: cuboid +0x170 → its fly-by (the enemies port) → `creature::ScriptRequest` → `rc_game::cinematic`.
- Tested headless in `crates/rc-game/tests/world/cutscene_novalis.rs` (trigger ticks, holds of 420 / 180 / 390 ticks).

