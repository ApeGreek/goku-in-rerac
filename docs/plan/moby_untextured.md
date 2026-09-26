# Moby triangles with texture −1 ("none")

Status: resolved from the decomp and a disc-wide scan (2026-09-26). The renderer currently skips these triangles
(`skipped … untextured` in `moby_render.rs`). **The game draws them**, textured with a solid 8×8 grey (0x80)
texture, which under GS MODULATE means **plain Gouraud: colour = lit vertex colour, alpha = vertex alpha**.

## 1. What the game does

- **Where −1 comes from.** It is not "no ad-gif yet". Every one of the 3,941 packet lists on the disc (high, low,
  metal) starts with a texture switch (first index byte 0), so the loader's initial `ListState` −1 never reaches a
  triangle. All −1 triangles come from ad-gif blocks whose `TEX0.data_lo` is literally −1 (334 of the disc's 11,087
  blocks), plus later packets of the same list that carry that state on (e.g. class 933 packets 6–8).
- **Load-time rewrite, not a draw-time branch.** The level core loader (L01 `0x258128`) calls
  `fun_00203640` (boot 0x203640 / L01 0x2549d0) → `fun_00203338` (boot 0x203338 / L01 0x2546c8) for every class.
  That walks every packet's ad-gif unpack (`vif_list + (size − texture_unpack_offset)·16`, 4 qw per block) and
  rewrites the four A+D data words of each block in place:
  - index ≥ 0: mapped through the class's `textures[16]` and built from the moby texture table by
    `fun_00202d78` (boot 0x202d78 / L01 0x254108);
  - index −2 / −3: the chrome / glass words at 0x19e6c0 / 0x19e6d8;
  - **index −1**: fixed words (same in `fun_00202d78` and `set_up_vis_gif_viewer` boot 0x202fd0 / L01 0x254360):
    - `TEX1_1 = data_hi<<6 | 0x20 | data_lo<<32` (MXL 0, MMAG linear),
    - `CLAMP_1 = 5` (WMS = WMT = CLAMP),
    - `TEX0_1 = 0x80000004cc007ffb` → TBP0 0x3ffb, TBW 1, PSMCT32, 8×8 (TW = TH = 3), TCC 1 (RGBA),
      TFX 0 (MODULATE), CLD 4,
    - `MIPTBP1_1 = 0`.
  - Bytes 8–15 of each qword (register address, smuggled secret index) are untouched.
- **The texture at GS block 0x3ffb.** `init_once` (boot 0x201650 / L01 `InitOnce` 0x2521a0) fills 0x100 bytes with
  the 32-bit word 0x80808080 (`fill_transfer_words` 0x1f97e8, `sw` loop; the 64-bit constant is truncated) and
  uploads it with `sce_gs_set_def_load_image(dbp 0x3ffb, dbw 1, PSMCT32, 0, 0, 8, 8)`. So it is an 8×8 texture,
  every texel R = G = B = A = 0x80. The texture allocators (`build_tfrag_texture_dma` 0x234d48, 0x211408,
  0x2370c0, 0x22a330, L01 0x2a8bf0 …) stop before 0x3ffb, and nothing else in the exports writes it: it is a
  permanent reserved "white" texture.
- **Draw.** `MobyProc` (L01 0x26a7a0 / boot 0x211808) DMAs the relocated VIF lists as-is; the VU1 program copies
  the block into the GS stream at the 0 escape, and the strip GIF tag is PRIM 0x7c (tristrip, IIP, **TME**, FGE,
  ABE; `moby_skinning_lighting.md` §7). No moby-level special case for −1.
- **Resulting pixel** (GS MODULATE, TCC 1): `Cv = Cf·0x80 >> 7 = Cf`, `Av = Af·0x80 >> 7 = Af`. Filtering and
  UVs are irrelevant (uniform texture, clamped). Then the usual moby ALPHA_1 0x8000000044 / TEST_1 0x5360b / fog.
  With the default moby alpha 0x80 the triangles are opaque, vertex-lit, untextured.

Confidence: high for the rewrite and the 0x80808080 texture (decomp + disassembly); medium-high that 0x3ffb is never
overwritten at runtime (no writer found besides `init_once`; allocators bound at 0x3ffb).

## 2. Data (Rust loader probe, high LOD, triangles × gameplay instances)

- Per level the −1 × instance totals equal the renderer's "untextured" counts exactly (16: 9,118; 05: 6,616; …).
  No triangle has a texture ≥ 0 on a 0xff class slot (so "untextured" in the log is only −1).
- Low LOD also has them (not rendered yet): e.g. level 08 12,676, level 16 1,925, level 04 1,512 (× instances).
- Metal lists never use −1 (they are −2/−3 by construction).

Level 16 (Kalebo III), extents in world units (−1 part vs whole class):

| o_class | inst | −1 tris / all | × inst | packets (ad-gif indices) | −1 extent | class extent |
|---|---|---|---|---|---|---|
| 933 | 25 | 308 / 644 | 7,700 | p5 [−1], p6–p8 no ad-gif (inherit) | 1.28³ | 1.28³ |
| 1139 | 5 | 160 / 160 | 800 | p0 [−1] | 0.40×4.40×4.40 | same |
| 471 | 37 | 12 / 416 | 444 | p6 [−1] | 0.30×0×0.71 | 4.01×4.55×0.71 |
| 1670 | 18 | 6 / 294 | 108 | p6 [−1] | 2.00×1.98×0.33 | 3.17×2.82×1.41 |
| 1387 | 4 | 12 / 12 | 48 | p0 [−1] | 0.04×1.00×1.00 | same |
| 1441 | 3 | 4 / 2,692 | 12 | p21 [−1] | 0.43×2.49×6.65 | 13.7×2.8×13.4 |
| 1439 | 3 | 2 / 2,312 | 6 | p1 [−1] | 0.15×2.36×0.16 | 13.5×2.7×11.5 |

Level 05 (Rilgar):

| o_class | inst | −1 tris / all | × inst | packets (ad-gif indices) | −1 extent | class extent |
|---|---|---|---|---|---|---|
| 838 | 26 | 192 / 192 | 4,992 | p0 [−1], p1 no ad-gif (inherit) | 0.20×8.15×7.00 | same |
| 1139 | 7 | 160 / 160 | 1,120 | p0 [−1] | 0.40×4.40×4.40 | same |
| 853 | 7 | 32 / 812 | 224 | p3 [−1] | 1.00×0.25×2.34 | 1.21×0.63×6.60 |
| 812 | 25 | 8 / 556 | 200 | p10 [−1] | 0.07×3.13×0.44 | 2.00×4.00×2.00 |
| 893 | 4 | 20 / 598 | 80 | p4 [−1] | 0.64×0.01×4.00 | 0.72×3.01×4.01 |

- The −1 parts are either whole classes with every slot 0xff (838, 1139, 1387: flat panels/discs, likely glows,
  beams, shadows or effect planes whose look comes from vertex colour and moby alpha) or small untextured pieces of
  textured classes (flat strips, trim). None of these classes is named in `moby_update_catalogue.md`; whether
  838/1139 run with moby alpha < 0x80 or a special mode (glow 0x10) at runtime is not checked here.
- Moby vertices carry no baked colour (the RGBA multiplier is 0x80 everywhere), so the drawn colour is the
  per-vertex lighting result (`moby_skinning_lighting.md` §5).

## 3. Renderer change (for the `moby_render.rs` owner)

- Stop skipping `texture == -1`. Treat it as its own texture batch bound to a synthetic image: 1×1 (or 8×8)
  `Rgba8Unorm` with bytes `[0x80, 0x80, 0x80, 0x80]`, shared by all classes. The existing shader then gives
  `rgb = min(128/255 · vertex_rgb·255/128, 1)` = vertex colour and `As = 0x80·Af/0x80 = Af`, exactly the GS result.
- Its `AlphaRange` is the constant 0x80 (texel alpha), so the As/AREF split depends only on vertex alpha, as for any
  other part. Same PRIM (MODULATE, fog, both faces), ALPHA_1 0x8000000044, TEST_1 0x5360b.
- Sampler: clamp-to-edge (CLAMP_1 = 5); UVs may be left as the packet's ST (ignored by a uniform texture).
- In the three places that filter on `t.texture >= 0` / `texture_table_index` (mesh build ~l.297, overlap check
  ~l.613, skip counter ~l.524), map −1 to this image instead of dropping it; keep −2/−3 skipped until the
  metal/glass passes exist.
- Apply the same to the low LOD when it is ported.

Side note (same function, resolves an open item in `moby_render_notes.md` §4): for index ≥ 0 `fun_00202d78`
builds `TEX1_1 = (mip_count−1)<<2 | data_hi<<6 | 0x20 | data_lo<<32` (MXL from the texture entry, MMAG linear,
MMIN = TEX1 `data_hi`, K = TEX1 `data_lo`), and `CLAMP_1 = clamp.data_lo | clamp.data_hi<<2 | table_index<<24`
(WMS, WMT; the index lands in MINV, unused unless region clamp).
