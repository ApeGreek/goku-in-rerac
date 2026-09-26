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
| `LightTfrags(u16 *list)` | boot `0x234f98` (0x6a4 bytes), level01 overlay `0x2a8e40` | Lombyte: `src/assembly/textbin/fun_00234f98.c` (INCLUDE_ASM only). Disassembled from the boot ELF in Ghidra; the level01 decompile (`decomp/export/level01.elf/002a8e40_LightTfrags.c`) is the same code with overlay addresses. **[verified]** |
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
* The port runs the pass once at load (`level_load.rs` -> `tfrag_light::light_level_tfrags`) and
  replaces `Tfrag::rgba`; the renderer is unchanged. **[equivalent while the port has no point lights]**

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
The port implements this pass and unit-tests it, but the engine passes no point lights (none exist
until gameplay code creates them). Padding entries `vert_count..rgba_size*4` get scratchpad
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
* `LightTies` (`0x2ab218`) and the shrub pass (`FUN_0029e7e8`) share the bank and the same idioms;
  not ported. **[open]**
* `header 0x35` (`dir_lights_upd`) is only cleared by this pass; who sets it is unknown. **[open]**
