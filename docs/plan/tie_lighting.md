# Tie lighting (`LightTies`) and the tie draw rules the renderer uses

Port: `crates/rc-formats/src/tie_light.rs` (math + unit tests + all-level run),
`crates/rc-engine/src/tie_light.rs` (load-time glue, `RC_NO_LIGHT=1`), `crates/rc-engine/src/tie_render.rs`
and `assets/shaders/tie.wgsl` (renderer). Float model: `tfrag_light::ps2` (docs/plan/tfrag_lighting.md §6).

Tags: **[verified]** read from disassembly / data; **[inferred]**; **[model]** depends on the PS2 float model.

## 1. Code and data

| What | Where | Notes |
| --- | --- | --- |
| `LightTies(u16 *list)` | boot **0x237370** (Lombyte exact match), level01 0x2ab218 | list of instance indices, 0xffff-terminated. Hand-written VU0 macro code; read from the Ghidra disassembly (the decompile hides the bc fields). [verified] |
| VU0 micro routines | program **436083**, entries `vcallms 0x58` (byte 0x2c0) and `0x84` (0x420) | 4 slots per call, alternating register sets vf01–04/vf09–12 and vf05–08/vf13–16. 436083 is loaded by `VU0_loadMicroProgram(0x100ae0)` just before the EE lighting half of the frame and at level load (boot `fun_001e9b10`). [verified] |
| Callers | level load `FUN_00255958` (list 0..n-1 in scratchpad); per frame in `DrawDebugProfiler` after `VU1_syncChain(4)`: `LightTies(0x1c7780)` (visible list from `TieProc`), or both lists in the two-pass mode (`register_entity_render_resources`). [verified] |
| Directional bank | EE 0x180340 (level01) / 0x19bdc0 (boot), 16 × 0x40 | same bank as tfrags and mobys (`tfrag_light::parse_light_bank`). [verified] |
| Point-light bank | 0x180740 / 0x19c1c0, 8 × 0x20 | same as tfrags. [verified] |

## 2. Instance records the pass reads (built by the level loader, `FUN_00255958`)

Run-time record, 0x20 bytes per instance: +0x00 world bounding-sphere centre (w = radius), +0x10 pointer to
the 0x1c0 record, +0x14 draw distance (**`cvt.s.w` of the disc word: instance +0x04 is an s32**, 720 on every
Novalis tie; `TieInstance::draw_distance` is typed f32 in `tie.rs` and reads as a denormal), +0x18 occlusion
index, +0x1a class index, +0x1b dirty flag (cleared by `LightTies`), +0x1c u16 `directional_lights`,
+0x1e point-light nibble list (0xffff at load). [verified]

0x1c0 record: +0x00 the disc matrix, then **column i's w = 1.0 / FastVecLength(column i)** (EE `div.s`) and
column 3's w = class scale; +0x40 the **64 lit RGBA words (output)**; +0x140 the 64 ambient RGBA5551 (copied
from instance +0x50). [verified]

Centre: `fun_001f9cf8` (M3·bsphere.xyz, w = bsphere.w), w := bsphere.w × max column length, `fun_001f9a80`
(× class scale, all lanes), `fun_001f9a10` (+ translation). `tie_light::instance_centre`. [verified]

## 3. The math (per instance)

```
sel = rec+0x1c; set/blend exactly as LightTfrags (tfrag_lighting.md §4): colA, dirA, colB, dirB,
   blend t = itof12((sel>>4)&0xff0), w = 1-t, dirs renormalised with rsqrt(1, (x²+y²)+1·z²)
wA = 0+colA.w, wB = 0+colB.w; colA.w = colB.w = 0
point lights (nibbles of rec+0x1e | 0xf0000, until 0xf): d = centre - L.pos; skip if r² - |d|² < 0;
   dirP += d·(1/|d|) (unweighted); colP += L.col·(1 - (1/r)·|d|); renormalise dirP only if >= 2 lights hit
wP = 0+colP.w; colP.w = 0
n_i = column_i.xyz · column_i.w  (unit axes);  L_k = -Nᵀ·dir_k = Σ_c (-n_c) ...  (class-space vector to the light)
   rounding: ((-n.x)·d.x + (-n.y)·d.y) + (-n.z)·d.z
VU0, per slot j: n = itof15(class normal j .xyz)            (s16 / 32768)
   d_k = (L_k.x·n.x + L_k.y·n.y) + L_k.z·n.z ;  f_k = max(d_k, d_k·w_k)      k = A, B, P
   c = ((pext5(ambient j) as 65536+b/128)·1.0 + colA·f_A) + colB·f_B + colP·f_P   (xyzw)
   c.rgb = min(c.rgb, 0x478000f3)  -> clamp at 243, not 255 (I of program 436083)
   rgba_j = low byte of each lane (ppach/ppacb)   alpha = ambient bit 15 ? 0x80 : 0
```
[verified; float results **[model]**]. Unit tests pin the rules with synthetic values (identity, rotation with
scale 3 giving the 0x3f7fffff unit column, blend, set 15 = zero, clamp 243, point-light merge).

## 4. Output and consumer

Output: **64 RGBA per instance, indexed by light slot** (not per vertex). `TieProc` uploads the 0x10 qw at
+0x40 (V4_8, STMOD 1) to VU1 0x346 / 0x386 (double buffer; colour-index copy B = A + 0x40). Program 13507:
dinky vertex RGBAQ = `lq vf11, 838(vi09)` stored as is; fat vertices blend `c0·z + ((c1+c2)·y)·w` with
per-instance weights (at full detail the renderer uses c0). [verified; fat weights at LOD 0 inferred]

## 5. Timing

Level load for all instances, then every frame for the visible list. The pass restarts from the ambient
table every time, so without point lights the per-frame result equals the load-time one; the port lights
once at load (Novalis: 1508 instances in ~6 ms) and adds the point lights in the vertex shader
(docs/plan/tfrag_lighting.md §9, "Point lights on world geometry"). [verified from code]

## 6. Draw rules used by the renderer (TieProc, boot 0x235be8)

* Cull: `pminw(rec.dist, cap)` with cap = 0x44340000 = **720.0** (boot `fun_001e9b10` → 0x160f70); an
  instance is skipped when `(dist - r) - (depth - r) < 0`, depth = view-space z of the centre; a zero
  distance skips always. Ported in the vertex shader. [verified]
* Fog: 13507 writes F from per-instance qw 5.w (constant per instance). `TieProc` computes it from the
  **centre** depth (clamped at 0): `cf24 + depth·cf20`, clamped to [If, In], with vf18 = 0x18cf20 =
  (cf20, cf24, In, If) from `UpdateViewContext`; the port replays this in `tie_lod::fog_value`. [verified]
* Texture ad-gifs (order TEX0, TEX1, MIPTBP1, CLAMP, MIPTBP2): converted at load by boot `fun_00203730`
  exactly like tfrags (TEX0 from `TieClassEntry.textures[i]`, MXL = ty−1, MMIN = TEX1.hi, K = TEX1.lo,
  CLAMP = lo | hi<<2 | index<<24, MIPTBP1 from mipmap/pad); paging `fun_002370c0` uses the tfrag layout
  (square base, mip 1 right after it), so `decode_tfrag_mip_levels` decodes tie mips. [verified]
* Mip level: the port uses the tfrag rule `round(log2(z/32) + K)`; 13507 sends Q = 1/w of its own
  per-instance matrix, whose w scale was not checked. [inferred]
* Ported since: LOD selection (near/mid/far) and the fat-vertex morph (`tie_lod.rs`), the occlusion bits
  (+0x18/+0x19, tested first; docs/plan/occlusion_culling.md "In the port"). Not ported: the reflection pass.

## 7. Results (Novalis)

1508 instances / 95 classes, 593 class meshes, 5548 entities, ≤ 593 instanced draws, 624,706 LOD-0
triangles; 12 mirrored and 92 sheared instance matrices (kept exact in the shader). Mean lit slot colour
(73, 73, 63); no used slot reaches the 243 clamp. All 44,712 retail instances light without error.
