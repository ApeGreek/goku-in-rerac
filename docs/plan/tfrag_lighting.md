# Tfrag vertex lighting (`LightTfrags`)

Port: `crates/rc-formats/src/tfrag_light.rs` (math, parsers, unit tests),
`crates/rc-engine/src/tfrag_light.rs` (load-time glue, `RC_NO_LIGHT=1` escape),
`crates/rc-formats/tests/tfrag_light_golden.rs` (data assumptions, all 19 levels).
Format table for the per-vertex record: `docs/formats/tfrag_rac1.md` §4.1.

Confidence tags: **[verified]** read from the disassembly and/or checked on the retail data;
**[inferred]** reasoned, not observed; **[model]** depends on the PS2 float model (§6).

## 1. Functions and data

| What | Where | Notes |
| --- | --- | --- |
| `LightTfrags(u16 *list)` | boot `0x234f98` (0x6a4 bytes), level01 overlay `0x2a8e40` | Lombyte: `src/assembly/textbin/fun_00234f98.c` (INCLUDE_ASM only). Disassembled from the boot ELF in Ghidra; the level01 decompile (`work/decomp/level01.elf/002a8e40_LightTfrags.c`) is the same code with overlay addresses. **[verified]** |
| Level-load init | level01 `FUN_00255958` | copies lights, resets point-light lists, calls `LightTfrags` on all tfrags. **[verified]** |
| Per-frame call | level01 `DrawDebugProfiler 0x21a1b8` (real role: frame render) | `if (mask & 2) && DAT_0016a480: LightTfrags(0x1c5880); PatchTfragGifs()` after `VU1_syncChain(2)`. `0x1c5880` is the visible-tfrag list built by `TfragProc`. **[verified call; list origin inferred]** |
| Directional bank | EE `0x180340`, 16 sets x 0x40 | zeroed (`FastMemZero16(0x180340,0x400)`) then filled from the gameplay file. **[verified]** |
| Point-light bank | EE `0x180740`, 8 slots x 0x20 | zeroed at load; written by `FUN_002525f8`/`FUN_00252750`, activated by `UpdateAllPointLights`/`CreatePointLight`. **[verified]** |
| Normal LUT | boot `0x165500` / level01 `0x166500`, 256 x (f32 cos, f32 sin) | DMA'd to scratchpad `0x70003800` (SPR_TO, QWC **0x80** = 2 KiB; render_pipeline.md §6 says 0x100, the code says 0x80). Identical in the boot ELF and every overlay; 296 of 512 values differ from IEEE-rounded `cos/sin(2*pi*i/256)`, so the port reads the table from `extracted/boot/SCUS_971.99`. **[verified]** |
| `vf21` constant | boot `0x15fac0` / level01 `0x15fb40` | `(1.0, 0.5, 1/3, 0.25)`; only `.x = 1.0` is used (the `vmaddz` of each dot product). **[verified]** |

## 2. Where the lights come from

Gameplay file (decompressed `gameplay_ntsc.bin`; the NTSC build loads the NTSC variant): the u32 at
**0x04** is the directional-light section offset. Section: `s32 count`, 12 bytes pad, then
`count` x 0x40 records copied verbatim to 0x180340 (`fun_001f9838(0x180340, sec + 0x10, n << 6)`).
The loader caps `count` at 12 (`if (0xb < n) { printf; n = 0xc; }`). Sets `count..15` stay zero,
so selecting one (retail data uses set 15 for 46 Novalis vertices) adds nothing. **[verified]**

Light set (`DirLightSet`, 0x40 bytes):

| Ofs | Reg | Content |
| --- | --- | --- |
| 0x00 | vf27 | colour A (r, g, b, w) - 1.0 adds 128 to the byte; **w = back-face factor** |
| 0x10 | vf24 | direction A (x, y, z, -) - direction the light travels (sun: z < 0) |
| 0x20 | vf28 | colour B, w = back-face factor |
| 0x30 | vf25 | direction B |

Novalis: 5 sets (set 0 = warm sun (0.97, 0.86, 0.52) + dim fill 0.098). Every retail w is <= 0,
so `max(d, d*w)` is always >= 0 (w < 0 gives a weak "back light" of |w| * |d|). **[verified on data]**

## 3. When it runs and what it writes

* Level load: after the bank copy, the loader sets every tfrag header's 0x36 to 0xffff (no point
  lights), then calls `LightTfrags` on lists of <= 1023 tfrag indices covering **all** tfrags. **[verified]**
* Every frame: again for the visible list (point lights can change). With no point lights active
  the result equals the load-time result, because the directional pass always restarts from the
  per-vertex base colour, never from the previous output. **[verified from code]**
* Output: per tfrag the RGBA words are built in scratchpad (double-buffered 0x70001000/0x70002000)
  and DMA'd (SPR_FROM, QWC = header 0x29 `rgba_size`) **over the tfrag's own RGBA block**
  (`data + header 0x1e`). VU1 then copies that RGBA verbatim. So the stored RGBA on disc is only a
  placeholder; the game never displays it. **[verified]**
* The port runs the directional pass once at load (`level_load.rs` -> `tfrag_light::light_level_tfrags`) and
  replaces `Tfrag::rgba`; the point lights are added on top in the vertex shader ("Point lights on world geometry"
  below). **[equivalent: the game's per-frame pass restarts from the same directional result]**

## 4. The math (per tfrag `h`, per vertex record `e`, i < `h.vert_count`)

Colour representation: a byte `c` is carried as the float with bits `0x47800000 + c`, i.e.
`65536 + c/128`; one ULP at that exponent is exactly 1/128. `pext5 -> pextlb/pextlh -> padduw`
builds it; `vminibcx.xyz` with `0x478000ff` clamps to 255; `qmfc2 -> ppach -> ppacb` stores the low
byte of each lane. So "0x80 = 1.0" falls out naturally: a light colour of 1.0 times a dot of 1.0 adds 128.

```
sel = e.select (u16 @+6); base = pext5(e.color @+4)  // r,g,b = 5-bit << 3, a = bit15 << 7
if h[0x34] >= 0: set = bank[h[0x34]]  (whole tfrag; not used in retail)          [verified code]
elif sel & 0xff00 == 0: set = bank[sel & 0xf]
else:                                                                             [verified code]
    t = itof12(((sel >> 4) & 0xff0)) = (sel >> 8) / 256 ;  w = 1 - t
    A = bank[sel & 0xf]; B = bank[(sel >> 4) & 0xf]
    colA = A.colA*w + B.colA*t (xyzw)   colB likewise
    dirA = A.dirA*w + B.dirA*t (xyz);   dirA *= rsqrt(1.0, (x*x + y*y) + 1.0*(z*z))   dirB likewise
wA = 0 + colA.w ; wB = 0 + colB.w ; colA.w = colB.w = 0
(ca, sa) = LUT[e.azimuth @+2] ; (ce, se) = LUT[e.elevation @+3]        (unsigned bytes)
n = (0 - ca*ce, 0 - sa*ce, 0 - (0 + se))                                 (stored normal, negated)
dA = (n.x*dirA.x + n.y*dirA.y) + 1.0*(n.z*dirA.z) ; dA = max(dA, dA*wA)
dB = ...                                           ; dB = max(dB, dB*wB)
acc = base_f * 1.0 ; acc += colA * dA (xyzw) ; out = acc + colB * dB (xyzw)
out.rgb = min(out.rgb, 0x478000ff) ; rgba = low byte of each lane
```

Because every add lands on the 1/128 grid of 65536 and all contributions are >= 0, the result is
`base + floor(128 * (colA.c * dA)) + floor(128 * (colB.c * dB))`, clamped to 255, with the products
and dots computed in PS2 float. Alpha stays the base alpha (0x80 in all retail data). **[verified + model]**

Normal convention: the true surface normal is `N = (cos az cos el, sin az cos el, sin el)`;
`n = -N`, so `n . dir = N . (-dir)`, i.e. lit when the surface faces against the light's travel.
60 % of Novalis normals have `N.z > 0`. **[verified on data]**

Point-light pass (after the directional pass, same scratch buffers), for each nibble `p` of
`h[0x36] | 0xf0000` from the low end until 0xf (max 4); light `L = pbank[p]` (colour, pos, w = radius r):

```
origin = itof12(q << 2) for the i32x4 quadword at h.light_ofs   (== tfrag origin, /1024)
for each vertex: cur = current rgba as 65536 + c/128 (so lights accumulate)
    P = itof12(pos(e.pos_ofs) << 2) + origin ; v = P - L.pos ; d2 = (v.x^2 + v.y^2) + 1.0*v.z^2
    skip vertex if (r*r - d2) is negative   (vsubx.w vf0 + cfc2 MAC flag 0x10)
    dist = sqrt(d2) ; f = (1 - (1/r)*dist) * (1/dist) ; v *= f
    d = n . v ; d = max(d, d*L.col.w) ; out = cur + L.col * d (xyzw, so alpha += L.col.w * d) ; clamp rgb
```

`pos_ofs` always addresses position `i` in the retail data, so the port uses `Tfrag::positions[i]`;
the origin quadword equals `Tfrag::origin` in every tfrag (golden test). **[verified on data]**
`light_tfrag(.., Some(points))` implements this pass bit-exactly and unit-tests it; the engine does it natively in
the shader instead ("Point lights on world geometry" below). Padding entries `vert_count..rgba_size*4` get scratchpad
leftovers in the game; the port keeps the stored bytes there (no vertex references them). **[verified]**

## 5. Fixed-point / register details that matter for identical bytes

* Loop counts are `lb` (signed byte) of 0x3c / 0x29; all retail values are 1..127. **[verified]**
* `vitof12` = int / 4096: the blend factor is exactly `k/256`; positions become `(s16 << 2)/4096`. **[verified]**
* Dot products are `vmul.xyz; vadday.x; vmaddz.x` with `vf21.x = 1.0`: `(x + y) + 1.0*z`, two
  roundings, in that order. **[verified]**
* `vmulaw ACC, base, vf0` (x 1.0) then two `vmadda/vmadd` steps: product rounded, then added. **[verified]**
* `rsqrt` in the blend path, `div`/`sqrt` in the point path (Q register; `vwaitq`). **[verified]**

## 6. PS2 float model used (`tfrag_light::ps2`)

The VU0 FMACs are not IEEE. The port models, on raw u32 bit patterns:

* no NaN/Inf/denormals: exponent 0 is zero; overflow -> +-0x7fffffff; underflow -> +-0.
* every op truncates (round toward zero).
* `add`: the smaller operand first loses the bits more than one position below the larger
  operand's LSB (mask `0xffffffff << (expdiff - 1)`), then the exact sum is truncated; exponent
  difference >= 25 returns the larger operand unchanged; a zero operand returns the other.
* `mul`: truncated exact product. `div`, `sqrt`: truncated exact results; `rsqrt(a,b) = div(a, sqrt(b))`.

Sources: the documented PS2 behaviour (round-to-zero, no IEEE specials) and the adder/multiplier
characterisation published by PCSX2's soft-float work (PR #12001; the libretro `ps2float.c` model),
read for behaviour only - no code copied. **Known gap [model]:** that research shows the PS2
multiplier's Booth/carry-save tree occasionally returns a result 1 ULP below the truncated exact
product; the port does not model this. It can only change a colour byte when `128 * col * d`
lies within ~2^-17 relative of an integer, and only for dot products whose inputs hit the pattern.
Division/sqrt exactness vs the hardware divider is likewise assumed, not tested; they only affect
the 129 blended Novalis vertices (and point lights).

Since all retail colour contributions are >= 0, the adder's operand-truncation quirk only matters
inside the mixed-sign dot products; the unit test `ps2_arithmetic_truncates` pins the modelled cases.

## 7. Results

* Novalis mean vertex colour: stored (placeholder) RGBA ~ (42, 44, 44); base colours (555) ~ (41, 42, 42);
  lit ~ (75, 76, 66). All 19 levels light without error (`tfrag_light_golden`, prints per-level means).
* Screenshot comparison (same default camera): lit terrain shows bright, sun-shaded green grass and
  warm dirt with visible slope shading; `RC_NO_LIGHT=1` is the old murky blue-grey look.

## 8. Open items

* No ground truth for the exact bytes (emulator RAM dump of the RGBA block after level load would
  settle the float-model questions in §6). **[open]**
* `LightTies` (`0x2ab218`) and the shrub pass (`FUN_0029e7e8`) share the bank and the same idioms: ported
  (docs/plan/tie_lighting.md, shrub_lighting.md), point lights below.
* `header 0x35` (`dir_lights_upd`) is the dirty byte: `DetachPointLight` sets it when a tfrag's point-light list
  empties, `TfragProc` then lists the tfrag for one more relight, and this pass clears it. **[verified]**

## 9. Point lights on world geometry (tfrags, ties, shrubs)

Explosions light the level: the Novalis arrival crash turns the rocks, cliff pillars and trees around it yellow, a
Bomb Glove explosion warms the cave walls and floor. Level01 addresses; the boot copies are named where known.

### 9.1 Which instances a light relights (the attachment)

| What | Where | Notes |
| --- | --- | --- |
| Bank | `0x180740 + 0x20·i`, i < 8 | qw0 colour (r, g, b) + intensity (w, the back factor), qw1 position + radius. [verified] |
| Attachment records | `0x180940 + 0x30·i` | +0x00/+0x02 tie list start/count, +0x04/+0x06 shrub, +0x08/+0x0a tfrag (u16 indices), +0x0c pointer to the slot's 0x200-entry u16 list (`0x180ac0 + 0x400·i`), +0x10 state (0 free, 1 written, 2 attached), +0x20 the position it was attached at. [verified] |
| `WritePointLight_A/B` | 0x2525f8 / 0x252750 | first slot with state 0 (only while the frame load 0x15f5d4 ≤ 0.8): writes the bank slot, zeroes the 0x30 record, state = 1. [verified] |
| `UpdateAllPointLights` | 0x2528a8 (boot 0x201a28), once per tick after the mobys | for each slot with state ≠ 0 whose position is more than **8.0** from the attachment position (`vec_distance` 0x221360, 3D): copy the position, then state 1 → `CreatePointLight`, state 2; state 2 → `refresh_point_light` 0x252dd8 = `DetachPointLight` + `CreatePointLight`. A radius change alone never re-attaches. [verified] |
| `CreatePointLight` | 0x252a28 (boot 0x201ba8) | sphere `(pos, radius + 8)`; tests it (`fun_001f9bb0` 0x2213c8: `(dx² + dy²) + dz² − (r₁ + r₂)² < 0`, strict) against every **tie** record (0x160fc0.., +0x00 sphere), then every **tfrag** header (the sphere ×1024 against header +0x00), then every **shrub** record (0x160494..). A hit takes the first free nibble of the instance's list (tie/shrub +0x1e, tfrag +0x36; 0xffff → `slot | 0xfff0`, then nibble 1, 2, 3; all four taken: skipped) and appends the index to the slot's list. **The list holds 0x200 entries for all three kinds together**: once full, the remaining ties, then all tfrags and shrubs, are not attached. [verified] |
| `DetachPointLight` | 0x252e08 (boot 0x201f88), also from `FreePointLight` 0x252850 | removes the slot's nibble from every listed instance (higher nibbles move down, 0xf fills the top); an instance whose list becomes 0xffff gets its dirty byte (tie/shrub +0x1b, tfrag +0x35). [verified] |

### 9.2 How often, persistence

* The relight lists are built by the procs every frame: `TfragProc` lists a visible tfrag when `+0x35 != 0 ||
  +0x36 != 0xffff` (≤ 511 entries, scratch 0x70003000 → 0x1c5880), `TieProc` a visible tie when `+0x1b || +0x1e !=
  0xffff`, `ShrubProc` likewise (shrub_lighting.md §4). The frame render (0x21a1b8) then runs `LightTfrags(0x1c5880)`,
  `LightTies(0x1c7780)`, `LightShrubs(0x1bd430)` when their counts (0x16a480 / 0x16a488 / 0x16a490) are non-zero.
  [verified]
* Each pass restarts from the baked inputs (per-vertex base colour / ambient table) and redoes the directional part,
  then adds the listed lights **at their current colour, position and radius**. So what is on screen every frame is
  `baked + the listed lights`; nothing accumulates. When a light is freed or moves away, the detach marks the emptied
  instances dirty and they are relit once without it: **the colours revert to the baked ones**. [verified]
* Instances attached but off screen keep stale colours in RAM, but are relit before they are drawn again. [verified]

### 9.3 How a light adds to the colours

* **Tfrags, per vertex** (§4 above): `v = P − L`, skipped when `r² − |v|² < 0` (current radius); `d = (−N)·v ·
  (1 − |v|/r)/|v|`, `d = max(d, d·w)`; `rgba += floor(128 · (colour, w) · d)` on all four lanes, rgb clamped at 255.
  Lights accumulate in nibble order on the already-lit bytes. The morphing LOD vertices blend the relit colours
  (VU1 reads the relit block). [verified]
* **Ties and shrubs, per instance** (tie_lighting.md §3, shrub_lighting.md §2): the listed lights that reach the
  instance's bounding-sphere centre merge into one third light: direction = sum of the unit vectors (centre − L),
  renormalised only when two or more contribute; colour = Σ colour·(1 − dist/r); back factor = Σ w·(1 − dist/r).
  Each class normal n (64 tie slots / 24 shrub palette entries) gets `d = −dir · (n through the unit axis columns)`,
  `max(d, d·back)`, `+ floor(128 · colour · d)`, rgb clamped at **243**; alpha unchanged. The shrub billboards keep
  the load-time average colour (col1.w is written once, at load). [verified]
* Colour units: 1.0 adds 128 to a byte. The explosion light (class 0x27f, `rc_game::moby_update::creature::fx`) ramps
  its channels to 2.55, so everything near it saturates; its intensity byte is 0 (no back light, alpha unchanged).

### 9.4 In the port

* **Attachment:** `rc_game::point_lights::WorldLightLists` reproduces §9.1 exactly (states, the 8-unit move
  threshold, `radius + 8`, ties → tfrags → shrubs, nibble insert/removal, the shared 0x200 capacity), driven after
  every tick from the game's bank (`Services::point_lights`; `PointLights::generation` tells a new light in a
  reused slot). Unit tests: `nibble_lists_fill_from_the_low_end_and_compact`, `overlap_is_strict`,
  `attach_margin_move_threshold_and_free`, `list_capacity_is_shared_ties_first`.
* **Lighting:** native, in the vertex shaders (`assets/shaders/world_lights.wgsl`, imported by `tfrag.wgsl`,
  `tie.wgsl`, `shrub.wgsl`). `crate::world_lights` appends the bank (256 bytes) and one nibble list per tfrag / tie /
  shrub to each renderer's existing per-frame buffer (`TfragLodState` modes, `TieLodState` words, `ShrubSway`
  shears; rewritten only when the bank or an attachment changes); the vertex normals ride in the existing static
  records (tfrag slot word: azimuth | elevation; tie / shrub instance record: the class normals as raw s16). No new
  material binding: a changed bind-group layout reorders the draws (the alpha-test / blended passes are order
  dependent) and changed frames without any light, which the A/B check caught. The baked colours stay as they
  were; an instance with list 0xffff takes the unchanged path, so a frame with no light is byte-identical to the
  port without this feature (Novalis frame 300, level 03 frame 120). No CPU rewrite of vertex or instance buffers.
* **Arithmetic:** IEEE f32 with the game's operation order and the byte grid (`floor(128·c·d)` per light); the PS2
  truncating FMAC is not modelled (≤ 1 byte; docs/plan/hardware_fidelity_layers.md). The bit-exact CPU versions
  (`light_tfrag(.., Some)`, `tie_light::light_regs(.., Some)`, `shrub_light::light_regs(.., Some)`) stay as the
  reference.
* `RC_WORLD_LIGHTS=0` keeps every list empty; `RC_WORLD_LIGHTS_TRACE=1` prints the frame time, the bank and the listed counts every frame.
* **Results:** the arrival crash (scene 5; two radius-60 lights at the ship) attaches 512 + 2 ties and therefore
  **no tfrag and no shrub**: the rocks, pillars and tree turn yellow and the grass hill behind the explosion does
  not, exactly as in the original frame. A Bomb Glove explosion in the Novalis cave (radius 20) attaches 75 tfrags,
  127 ties and 99 shrubs and washes the walls and floor yellow, fading with the light's colour ramp (~30 ticks).

