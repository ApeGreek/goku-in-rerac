# Occlusion culling: the per-frame rule

This note is for the renderer agent. Data format: `docs/formats/occlusion_rac1.md`. Code:
`rc_formats::occlusion`. All addresses are in the level01 overlay (the build the game runs). The boot ELF
has the same functions at `ParseOcclGrid` 0x1f2690, `GetOcclGridFromPair` 0x1f2768,
`BuildOcclVisibility` 0x1f2820 and `UpdateOcclusion` 0x1f2c10, with globals +0x40 (grid 0x15f640) and a
mask at 0x193fc0.

## 1. Load time (`FUN_00255958`, after the moby loader) [verified, decompile]

Each tfrag, tie and moby gets a u16 occlusion word `w = byte << 8 | bitmask`. The word is either derived
from its gameplay mapping, or 0x7f80 (bit 1023, "always visible"). The exact match rules are in the
format doc §3. Retail takes the simple paths in all 19 levels:

* tfrags are positional;
* ties are positional;
* a moby with gameplay `occlusion == 0` gets its record by spawn id, and every other moby gets 0x7f80.

`rc_formats::occlusion::resolve_level_occlusion` reproduces this and returns `LevelOcclusion {tfrag, tie,
moby}`.

The loader also sets the mode `0x16c4f4`: 2 (active) when the level has a grid and mappings, else 0
(off). The debug menu calls this setting "occlusion" with the values off, freeze and active. Retail never
changes it after load.

## 2. Every frame: build the mask [verified, disassembly + decompile]

The frame render `0x21a1b8` (Ghidra name `DrawDebugProfiler`) calls `UpdateOcclusion` (0x2193f8) near
its top, after two setup calls. That is before the sky, tfrags, ties, shrubs and mobys, and it is not
gated by the draw mask `0x15f3f4`.

```
UpdateOcclusion:  mode 0 → mask = all 0xff;  mode 2 → BuildOcclVisibility;  mode 1 (freeze) → mask unchanged
BuildOcclVisibility (0x219008), cam = 0x167240 (render camera position, world units):
  c = cvt.w.s(cam * 0.25)                          // truncation toward zero, per axis
  if m = ParseOcclGrid(c.x, c.y, c.z):  mask = *m; prev(0x15f610) = m; missed(0x15f60c) = 0
  else:
    missed = 1
    if fallback(0x15f608) == 0:                    // neighbours
      for each axis a: f = cam.a*0.25 - float(c.a)
        GetOcclGridFromPair (0x218f50): try c.a-1 first if f < 0.5, else c.a+1 first; take the first hit
      if any of the 3 hits: union(0x174200) = OR of the hits; prev = union; mask = union; done
    if not done:
      fallback 1:            mask = all 0xff
      fallback 2 and octant: mask = oct.mask[(cam.x>cx)*4 + (cam.y>cy)*2 + (cam.z>cz)]
      else (0, or 2 without octant):
                             mask = (debug_cam 0x16c4ec != 0 || prev == 0) ? all 0xff : *prev
  mask[0x7f] |= 0x80                               // bit 1023 always on
```

* **Hysteresis.** A hit always wins. Outside the grid, the game first ORs up to 3 face-neighbour cells
  (one per axis, nearer side first). Failing that, it keeps the **last mask it used** (sticky), not all
  visible. `prev` persists across frames. No level01 code resets it at level load (found no writer other
  than this function) [verified absence by grep; low risk]. The port should reset it to `None` on
  load.
* **Fallback flag** `0x15f608` is cleared at the end of every frame render (`0x21a1b8` tail) and in
  `ResetDrawGlobals`. `UpdateModeFreeze` (game state 0x1734c0 cases 1 and 2) sets it to 2 for the next
  frame. `FUN_002cb540` (a `CollLine_Fix` camera-placement path) sets it to 1. In normal play it is 0.
  [verified writers; purpose of 002cb540 inferred]
* **debug_cam** `0x16c4ec` is the debug menu's camera mode (normal/camera/cam+chr/volume). It is 0 in play.

`OcclusionState::update(occl, mode, fallback, debug_camera, camera)` is this function, line for line.
Unit tests cover the neighbour, previous, debug, all-visible and freeze paths.

## 3. Every frame: the test [verified, disassembly]

Each proc copies the 0x80-byte mask to scratchpad. For every object it runs
`if (spr_mask[w >> 8] & (w & 0xff)) == 0 → skip`. The test comes **before any frustum or distance
test**:

| Proc | Copy | Test | Order inside the object loop |
|---|---|---|---|
| `TfragProc` 0x2a7e58 | 0x70003b00 | 0x2a80b8–0x2a80e4: `lbu 0x3b`, `lbu 0x3a`, `lbu 0x3b00(byte)`, `and`, `beq → next` | First test. The bsphere/near/far/side-plane and box tests (`tfrag_lod.rs`) only run on tfrags that pass. The per-frame visible list at 0x1c5880 (LightTfrags) therefore never contains occluded tfrags. |
| `TieProc` 0x2a9a90 | 0x70003000 | 0x2a9dd8–0x2a9e2c: rec +0x19 / +0x18 | First real test. The `+0x14 == 0` check before it is the SPR DMA wait, not a cull. The draw-distance and sphere tests follow. Occluded ties are not in the LightTies list (0x1c7780). Both passes of `DrawTies_2` use the same test. |
| `MobyProc` 0x26a7a0 | spr+0x3800 | 0x26ab2c–0x26ab58: moby +0x37 / +0x36 | After `state (+0x20) < 0 → skip`, before `+0x34 & 0x81 → skip` and the sphere tests. An occluded moby also gets **+0x31 cleared** (0x26b4a0). DrawMobys and DrawMobyList (menus and weapons) both use it. |
| Shrubs | none | Not tested. `DrawShrubs` never reads 0x174180. | |

Outside the grid the mask is whatever §2 produced: the neighbours' union, the previous mask, all visible
or an octant. Objects with 0x7f80 are always visible, even when the mask is "frozen", because bit 1023
is forced on every time the mask is built. With mode 1 (freeze) it is simply never rebuilt.

## 4. Renderer plan

Load (level_load.rs):
1. Read the grid and override with `occlusion::parse_occlusion(&core, &core_data)`. Treat `Err` for no
   block as `None`.
2. Read the mappings with `occlusion::parse_gameplay_occlusion_mappings(&gameplay)`. `None` means "no
   occlusion".
3. Resolve every object with `occlusion::resolve_level_occlusion(maps.as_ref(), &tfrag_headers,
   &tie_occlusion_indices, &moby_instances)`. If the grid or the mappings are missing, pass `None` and
   use `OcclusionMode::Off`, else `Active`.
4. Keep one `OcclusionState::default()` (a resource), and reset it on level change.

Per frame (one system, running before the tfrag, tie and moby systems, in `PostUpdate` before
`update_tfrag_modes`):
1. `let mask = *state.update(grid.as_ref(), mode, OcclusionFallback::Neighbours, false, camera_pos)`.
   `camera_pos` is the same world-unit camera position `tfrag_lod` scales by 1024 (the game's 0x167240).
   Use `OcclusionFallback::Octants` / `AllVisible` only once the port models `UpdateModeFreeze` or
   `FUN_002cb540`.
2. Publish `mask` (a `[u8; 128]`) as a resource: `OcclusionMask`.

`tfrag_lod.rs`: in `update_tfrag_modes`, before `tfrag_proc(...)`, return `MODE_CULLED` for tfrag `i` when
`!lo.tfrag[i].visible(&mask)`. Keep `tfrag_proc` unchanged; the order matches the game. Remove the "not
modelled" note on `tfrag_proc`. The LightTfrags visible list, if the port builds one, must skip the
occluded tfrags too.

`tie_render.rs`: in the per-instance cull loop (the `TieProc` replay, next to the draw-distance test),
skip instance `i` when `!lo.tie[i].visible(&mask)`, **before** the distance test. `i` is the gameplay
instance index, the same order `parse_tie_instances` returns. The `LightTies` per-frame list must exclude
occluded ties. This only matters for point lights; ambient/directional light is computed at load.

`moby_render.rs`: for static instance `i` (index into `parse_moby_instances`), skip it when
`!lo.moby[i].visible(&mask)`. Put that after any "not spawned/dead" check and before the sphere/distance
test. Spawned mobys (not from the static table) are always visible. If the port tracks moby +0x31, clear
it for occluded mobys.

Shrubs: nothing.

Checks for the renderer agent:
* Inside the grid, `frame_mask == with_always_bit(occl.masks[cell.mask])` for `cell = occl.cell_for(cam)`.
* Novalis (level 01) inside the grid should draw 5–522 tfrags before frustum culling (per the golden
  stats).
* `RC_NO_OCCLUSION=1` (suggested): force `OcclusionMode::Off` for A/B comparison.

## 5. Confidence

* Grid layout, cell mapping, lookup and neighbour and previous logic: **verified** from the disassembly of
  0x218e78, 0x218f50, 0x219008 and 0x222160 (`cvt.w.s`) and the decompile. Golden-tested byte-identical
  against the C++ extractor on 19 levels.
* Load-time match rules (tfrag byte 0x3d, tie `(s16)`/u16 keys, moby spawn id and `occlusion == 0`):
  **verified** from the decompile of `FUN_00255958`. Retail data takes the in-date paths everywhere
  (asserted in the golden test).
* Test placement in the three procs and shrubs having none: **verified** (disassembly; grep for 0x174180).
* The runtime tie order equals the gameplay order, and +0x18 = `(s16)occlusion_index`: **verified**
  (loader loop at `piVar30[7]`).
* Runtime moby order: the resolution is per static gameplay instance. Mobys the spawn test drops at load
  simply never exist [inferred, harmless].
* Initial state of `prev` and of the mask before the first frame: **unknown**. The port uses None and
  all visible.
* Why fallback 1 (`FUN_002cb540`) is set, and the gameplay meaning of the octant override (levels 11, 13,
  17): **inferred** only.

## 6. In the port

Code: `crates/rc-engine/src/occlusion.rs` (load + per-frame mask), consumers in `tfrag_lod.rs`,
`tie_lod.rs`, `moby_render.rs`, `moby_anim.rs`. Disc-data test: `crates/rc-formats/tests/occlusion_frames.rs`.

* **Load** (`occlusion::load`, called from `level_load.rs` after the sky): grid only when the core has an
  `occlusion` block (a malformed block is an error, a missing one is `None`), mappings from gameplay
  pointer 0x8c, then `resolve_level_occlusion` with the tfrag headers (table order), the tie instances'
  `occlusion_index` and the gameplay moby instances. Mode `Active` when grid and mappings exist, else `Off`.
  Same byte source as everything else (disc image or `extracted/`, `disc_source.rs`). Novalis: 6039 cells,
  2822 masks; 1004/1004 tfrags, 1508/1508 ties (positional), 398/983 mobys mapped.
* **Per frame** (`update_occlusion`, `PostUpdate`, `OcclusionSet`): `OcclusionState::update(grid, mode,
  Neighbours, debug_camera = false, game_eye(camera))`, the same world-unit eye `tfrag_lod` scales by 1024.
  The state starts as `OcclusionState::default()` (previous = none, mask all visible). The tfrag, tie and
  moby systems are ordered `.after(OcclusionSet)`; the moby one also `.before(VisibilityPropagate)`.
* **Tfrags**: `update_tfrag_modes` writes `MODE_CULLED` for a tfrag whose word fails, before `tfrag_proc`.
* **Ties**: `update_tie_lods` writes the culled record `(LOD_CULLED, 0, 0, 0)` for a failing instance
  before `tie_cull` (draw distance, near, side planes).
* **Mobys**: `update_moby_occlusion` sets `Visibility::Hidden` on every entity of a failing static
  instance (only when the result changes; cheaper than rewriting the 208-byte storage records every
  frame, and hidden entities leave the instanced batch). The port has no moby sphere/frustum test, so
  occlusion is the only moby cull. **+0x31** is MobyProc's "drawn this frame" byte: set to 1 on the draw
  path, cleared on every skip path, including occlusion (level01 0x26a7a0 decompile, lines around the
  `+0x131` accesses). Its readers are per-class gameplay update functions (0x2e.. range, not ported); the
  generic update loop (0x2792d0: `MobyAnimAdvance` unless mode & 0x40, then the update function) does not
  read it, so occluded mobys keep ticking. MobyProc queues an animation-evaluation job only for mobys it
  draws (`MobyAnimProc` 0x26a6c0 walks that list), so the port skips `MobyAnimEval` for occluded instances
  and re-evaluates when one becomes visible (`MobyAnim::pose_dirty`). [verified decompile]
* **Environment**: `RC_OCCL=0` mode off; `RC_OCCL=1` freeze after the first built mask (the debug menu's
  freeze, switched on at frame 1: fly away to see what the start cell culls); `RC_OCCL_STATS=1` prints
  `occl: cam (...) cell (...) <source>, <mask bits> | tfrags a occluded / b culled / c drawn | ties ... |
  mobys ...` at most once a second. (Instead of the suggested `RC_NO_OCCLUSION=1`.)
* **Checks (Novalis)**: in-grid cameras (158,126,66), (162,122,62), (100,190,44), (102,186,44) give the
  same image with occlusion on and off except for objects behind blended vines or seen through seams in
  the level geometry. The test above checks, for 200 seeded cameras (104 in the grid), that bit 1023 is
  always set, an in-grid mask equals the cell's mask, and visible tfrags stay within 5..=522.
* **Outside the grid** the rule falls back to the neighbour union or the previous mask, and that can hide
  objects in direct view. Example: `RC_CAM=152,126,66,...` sits in cell column (38, 31), which has no grid
  cells at any height (inside the rock mass). The union of the neighbours (39,31,16) etc. hides the dark rock in
  front of the camera (most likely tie 732, class 536, 7 units away). The game would do the same; its camera never goes there. Views from
  above the playable space (e.g. (160,40,95), (100,190,60)) find no cell and no neighbour on the first
  frame, so everything stays visible (previous = none).

