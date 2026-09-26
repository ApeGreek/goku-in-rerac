# RAC1 shrub and sky formats

*An independent format specification for the "shrub" (small instanced vegetation/prop) mesh format and the "sky" format of* Ratchet & Clank *(2002, PS2 — Wrench game id `rac` / RAC1), derived by reading the Wrench source tree (GPL, reference only; commit `1b48f4d1`). Every claim is attributed to the Wrench file it was derived from. Facts about VIF code decoding are assumed known from the tfrag specification and are not repeated.*

---

## Part 1 — Shrubs

### 1.1 Where shrub data lives

| Thing | Location | Wrench source |
| --- | --- | --- |
| Shrub class table (index) | `LevelCoreHeader.shrub_classes` (`ArrayRange` at 0x28 of the core index) | `src/wrenchbuild/level/level_core.h` |
| Shrub texture table | `LevelCoreHeader.shrub_textures` (`ArrayRange` at 0x48), array of `TextureEntry` (0x10 bytes each) | `level_core.h`, `level_textures.h` |
| Shrub class blobs | Decompressed level core *data* (the "asset wad"), one blob per table entry, at `ShrubClassEntry.offset_in_asset_wad` | `src/wrenchbuild/level/level_classes.cpp` |
| Billboard texture pixels + palette | GS RAM block (`LevelCoreHeader.gs_ram`) | `src/wrenchbuild/level/level_textures.cpp` |
| Shrub instances | Gameplay file, block pointer at offset **0x3c** of the RAC1 gameplay header; the class-number list is at **0x38** | `src/instancemgr/gameplay.cpp` (`RAC_GAMEPLAY_BLOCKS`), `docs/gameplay.md` |

Blob extents are not stored. Wrench recovers a class blob's size by collecting every known block offset in the core data (tfrags, occlusion, sky, collision, textures base, decompressed size, and every moby/tie/shrub class offset) into a sorted boundary list, then taking the distance from a blob's offset to the next boundary above it (`enumerate_level_core_block_boundaries` / `level_core_block_range`, `level_core.cpp`). Class blobs are written aligned to **0x40** (`pack_shrub_classes` passes alignment `0x40` to `pack_asset`).

Shrub classes have **no separate LOD meshes** — unlike ties and tfrags there is exactly one mesh, optionally plus a billboard (`src/instancemgr/instance_schema.wtf`: *"Small static objects with only a single LOD level and optionally a billboard"*). Shrubs are also excluded from the occlusion-culling system, "as they are usually only visible up close" (`docs/occlusion_culling.md`).

#### `ShrubClassEntry` — core index record, 0x30 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | s32 | `offset_in_asset_wad` | Byte offset of the class blob in the decompressed core data. 0 = no geometry. |
| 0x04 | s32 | `o_class` | Class number referenced by instances. |
| 0x08 | s32 | pad | Always 0 in Wrench's output. |
| 0x0c | s32 | pad | Always 0. |
| 0x10 | u8[16] | `textures` | Up to 16 indices into the shrub texture table (`TextureEntry` array). Unused slots are 0 / terminated early. |
| 0x20 | — | `billboard` | 0x10-byte billboard *texture* descriptor, table below. |

#### Billboard texture descriptor (0x10 bytes, at 0x20 of the entry)

From `src/wrenchbuild/level/level_textures.h` and `unpack_shrub_billboard_texture` in `level_textures.cpp`:

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s16 | `texture_width` | Billboard texture width in pixels. **0 means the class has no billboard texture** — Wrench uses this as the presence test. |
| 0x2 | s16 | `texture_height` | Height in pixels. |
| 0x4 | s16 | `maximum_mipmap_level` | **Level count** (3 or 4 retail): the class init sets TEX1 MXL = this − 1 (verified, boot `fun_00203b08`). |
| 0x6 | s16 | `palette_offset` | Palette address in GS RAM, in units of 0x100 bytes. 256 × u32 CLUT follows. |
| 0x8 | s16 | `texture_offset` | Level-0 pixel address in GS RAM, in units of 0x100 bytes. 8-bit indexed, `width * height` bytes. TEX0 TBP0 = this + level base. |
| 0xa | s16 | `mipmap_1_offset` | Mip level 1 address (units of 0x100 bytes) = MIPTBP1 TBP1 (+ base). |
| 0xc | s16 | `mipmap_2_offset` | Mip level 2 address = TBP2. |
| 0xe | s16 | `mipmap_3_offset` | Mip level 3 address = TBP3 (0 when `maximum_mipmap_level` = 3; unused). |

Verified use (2026-09-26, boot `fun_00203b08`, the shrub class init): TEX0 = TBP0 | TBW max(1, w/64) | PSMT8 | log2 w/h
| TCC | CBP = `palette_offset` + base | CLD 4; MIPTBP1 = the three mip addresses with TBW1..3 = max(1, w >> 7..9); all
levels are resident in gs_ram (the port decodes them with `texture::decode_billboard_mip_levels`, every retail
billboard decodes). Full register formulas: docs/plan/shrub_lighting.md §8.

Note a naming hazard in Wrench: there are **two unrelated structs called `ShrubBillboardInfo`** — the 0x10-byte on-disc texture descriptor above (`level_textures.h`) and a purely internal 4-float struct (`src/engine/shrub.h`) used to pass fade distance / width / height / z-offset through the asset pipeline. Only the former appears in `ShrubClassEntry`.

---

### 1.2 Class blob layout

A shrub class blob is self-contained and **position-independent**: every internal pointer is a byte offset relative to the start of the blob (the header). Derived from `read_shrub_class` / `write_shrub_class` in `src/engine/shrub.cpp`.

```
+0x00  ShrubClassHeader                      (0x40 bytes)
+0x40  ShrubPacketEntry[packet_count]        (8 bytes each)
 ...   VIF command lists, one per packet, each 0x10-aligned
 ...   ShrubBillboard                        (0x40 bytes, 0x10-aligned, optional)
 ...   ShrubNormal[24]                       (0x10-aligned, 0x180 bytes)
```

#### `ShrubClassHeader` — 0x40 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | f32[4] | `bounding_sphere` | Centre (x,y,z) and radius (w). Units: multiples of `scale` — see §1.5. |
| 0x10 | f32 | `mip_distance` | Per-class texture-paging distance: the class init stores `trunc(mip_distance·1024)` at 0x1d8cb0 + class·4, which `fun_0022a330` compares per frame to decide whether the base level / mip 1 of the class textures are uploaded. Wrench derives TEX1 K from it; at run time K comes from the ad-gif on disc (verified; §1.4). |
| 0x14 | u16 | `mode_bits` | Render-mode flag word. Contents unknown to Wrench; round-tripped verbatim when re-packing a binary class, but forced to **0** when a class is rebuilt from a glTF mesh (`pack_shrub_class` in `src/wrenchbuild/classes/shrub_class.cpp` passes `0`). |
| 0x16 | s16 | `instance_count` | Runtime scratch: number of instances of this class queued for drawing this frame. Not written on disc. |
| 0x18 | s32 | `instances_pointer` | Runtime scratch: EE pointer to the queued instance array. Not written on disc. |
| 0x1c | s32 | `billboard_offset` | Offset to the `ShrubBillboard` record, relative to the blob start. **> 0 means a billboard exists**; 0 means none. |
| 0x20 | f32 | `scale` | Position dequantisation factor (§1.5). |
| 0x24 | s16 | `o_class` | This class's own class number (duplicates `ShrubClassEntry.o_class`). |
| 0x26 | s16 | `s_class` | Purpose unknown. Presumably a secondary/"shrub class" id. Not preserved by Wrench. |
| 0x28 | s16 | `packet_count` | Number of `ShrubPacketEntry` records immediately following the header. |
| 0x2a | s16 | pad | 0. |
| 0x2c | s32 | `normals_offset` | Offset to the 24-entry normal/colour-index table, relative to blob start. |
| 0x30 | s32 | pad | 0. |
| 0x34 | s16 | `drawn_count` | Runtime scratch: instances actually drawn as meshes. |
| 0x36 | s16 | `scis_count` | Runtime scratch: presumably "scissored"/culled instance count. |
| 0x38 | s16 | `billboard_count` | Runtime scratch: instances drawn as billboards this frame. |
| 0x3a | s16[3] | pad | 0. |

Evidence that `instance_count`, `instances_pointer`, `s_class`, `drawn_count`, `scis_count` and `billboard_count` are **zero in retail files**: `read_shrub_class` discards them and `write_shrub_class` writes zeros, yet `test_shrub_class_core` (`shrub_class.cpp`) performs a byte-exact read→write→diff over the whole blob and is enabled for RAC. A non-zero value on disc would fail that test. They are therefore fields the EE fills in at load/draw time.

#### `ShrubPacketEntry` — 8 bytes, `packet_count` of them at 0x40

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `offset` | Offset of the packet's VIF command list, relative to the blob start. |
| 0x4 | s32 | `size` | Length of the command list in bytes. |

Each packet is one self-contained upload to VU1: it must fit in one 0x76-quadword VU1 input buffer and produce at most 168 quadwords of GS packet (§1.6).

---

### 1.3 Per-packet VIF command list

Each packet's command list, as emitted by `write_shrub_class` (and therefore, given the byte-exact round-trip test, as found in retail RAC1 files):

| # | Word / code | Meaning |
| --- | --- | --- |
| 1 | `0x01000404` | `STCYCL` with CL=4, WL=4 — write 4 quadwords per 4-quadword cycle, i.e. contiguous, no skipping or filling. |
| 2 | `0x00000000` | `NOP`. |
| 3 | `0x05000000` | `STMOD`, MODE=0 — decompression writes values directly (no ROW addition or offsetting). |
| 4 | `UNPACK V4_32`, `num = 1 + gif_tag_count + texture_count*4`, `ADDR = 0`, FLG=1, USN=signed | Uploads the **packet header block**: 1 header quadword, then the vertex GIF tags, then the AD-GIF blocks. |
| 5 | `0x05000000` | `STMOD`, MODE=0 again. |
| 6 | `UNPACK V4_16`, `num = vertex_count`, `ADDR = vertex_offset`, FLG=1, USN=signed | Uploads **vertex part 1** (positions + GS-packet offsets). |
| 7 | `0x05000000` | `STMOD`, MODE=0 again. |
| 8 | `UNPACK V4_16`, `num = vertex_count`, `ADDR = vertex_offset + vertex_count`, FLG=1, USN=signed | Uploads **vertex part 2** (STs + colour index + stop bit). |

Encoding notes (from `src/core/vif.h`, `VifCode::encode_unpack` in `src/core/vif.cpp`): the UNPACK command byte is `0b1100000 | VNVL`, so V4-32 → `0x6C` and V4-16 → `0x6D`; FLG=1 means the destination address is **relative to VIF1 TOPS**, i.e. the double-buffered input area described in §1.6; USN=0 means the 16-bit components are sign-extended.

`read_shrub_class` asserts there are **exactly three UNPACKs** per packet. There is no `MSCAL`/`MSCNT` and no `FLUSH` in the stored list: the microprogram kick and the DMA chain that strings the packets together are supplied by the EE at draw time, not stored with the class.

#### Header block layout in VU1 memory (V4-32 upload)

| Quadword offset | Contents |
| --- | --- |
| 0 | `ShrubPacketHeader` |
| 1 … 1+`gif_tag_count`-1 | `ShrubVertexGifTag`, one per triangle strip / triangle list |
| 1+`gif_tag_count` … | `ShrubTexturePrimitive` (AD-GIF block), 4 quadwords each, `texture_count` of them |

This matches the VU1 input-buffer layout table in `docs/shrub_renderer.md`.

#### `ShrubPacketHeader` — 0x10 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `texture_count` | Number of AD-GIF blocks in this packet. |
| 0x4 | s32 | `gif_tag_count` | Number of vertex GIF tags (= number of strips/lists). |
| 0x8 | s32 | `vertex_count` | Number of vertices, including padding vertices. |
| 0xc | s32 | `vertex_offset` | VU1 quadword address of vertex part 1, i.e. `1 + gif_tag_count + texture_count * 4`. Part 2 starts at `vertex_offset + vertex_count`. |

#### `ShrubVertexGifTag` — 0x10 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | u64 + u32 | `tag` | The first 12 bytes of a GS GIFtag, copied verbatim into the output GS packet. |
| 0xc | s32 | `gs_packet_offset` | Destination quadword index within the GS packet being built. Occupies the GIFtag's unused fourth word. |

GIFtag fields written by Wrench's builder (`write_shrub_class`, field accessors in `src/engine/gif.h`):

| Field | Value |
| --- | --- |
| `NLOOP` | vertex count of this strip |
| `EOP` | 0, except on the **last** GIF tag of the packet, where it is 1 |
| `PRE` | 1 (PRIM field is used) |
| `PRIM` | see below |
| `FLG` | 0 (PACKED mode) |
| `NREG` | 3 |
| `REGS` | `0x00000412` → register descriptors, in order: **ST (2), RGBAQ (1), XYZF2 (4)** |

So each vertex emits three GS quadwords: texture coordinates, colour, then position+fog. The PRIM register is built with `IIP=1` (Gouraud), `TME=1` (texture mapping on), `FGE=1` (fog on), `ABE=1` (alpha blending on), `AA1=0`, `FST=0` (ST/Q, not UV), `CTXT=0`, `FIX=0`, and primitive type `GS_PRIMITIVE_TRIANGLE` (0b011) or `GS_PRIMITIVE_TRIANGLE_STRIP` (0b100). `read_shrub_class` rejects any other primitive type, so RAC1 shrub geometry is only triangle lists and triangle strips (triangle fans are supported by the builder but never seen).

#### `ShrubTexturePrimitive` (AD-GIF block) — 0x40 bytes = 4 quadwords

Four GS "A+D" (address+data) entries that switch texture state. Each is a `data_lo`/`data_hi`/`address` triple; the first one's otherwise-unused fourth word carries the GS-packet offset.

| Offset | Type | Name | Contents as written by Wrench |
| --- | --- | --- | --- |
| 0x00 | s32 | `d1_tex1_1.data_lo` | LOD `K` value derived from `mip_distance` (§1.4). |
| 0x04 | s32 | `d1_tex1_1.data_hi` | `0x04` — the `MMIN` mipmap filter selector. |
| 0x08 | u8 | `d1_tex1_1.address` | `0x14` = `TEX1_1`. |
| 0x0c | s32 | `gs_packet_offset` | Destination quadword index in the GS packet. |
| 0x10 | s32 | `d2_clamp_1.data_lo` | 1 if the S axis clamps, else 0. |
| 0x14 | s32 | `d2_clamp_1.data_hi` | 1 if the T axis clamps, else 0. |
| 0x18 | u8 | `d2_clamp_1.address` | `0x08` = `CLAMP_1`. |
| 0x20 | s32 | `d3_miptbp1_1.data_lo` | Texture index (see note). |
| 0x28 | u8 | `d3_miptbp1_1.address` | `0x34` = `MIPTBP1_1`. |
| 0x30 | s32 | `d4_tex0_1.data_lo` | Texture index (see note). |
| 0x38 | u8 | `d4_tex0_1.address` | `0x06` = `TEX0_1`. |

**Important:** the payloads do **not** match the bit layouts of the real GS registers they name. Wrench's builder stores plain indices where the GS expects packed `TBP0`/`CBP`/`TBW`/`PSM` bitfields, with the comment that the data "is fixed up at runtime by the game" (`build_shrub_class`, `shrub.cpp`). Concretely, `d3_miptbp1_1.data_lo` and `d4_tex0_1.data_lo` both hold the **index into the class's 16-entry texture list** (`ShrubClassEntry.textures`); the EE patches in the real GS RAM addresses, mip base pointers, buffer widths and CLUT pointers when the level's textures are uploaded. `recover_shrub_class` reads the material index back out of `d4_tex0_1.data_lo`. The unused halves (`d3_miptbp1_1.data_hi`, `d4_tex0_1.data_hi`, and the `pad` words) are zero.

Each AD-GIF block costs **5 quadwords** in the output GS packet: one GIFtag in A+D mode generated by the microprogram, plus the 4 entries above.

#### Vertex part 1 — `ShrubVertexPart1`, 8 bytes per vertex (V4-16)

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s16 | `x` | Local X, quantised (§1.5). |
| 0x2 | s16 | `y` | Local Y. |
| 0x4 | s16 | `z` | Local Z. |
| 0x6 | s16 | `gs_packet_offset` | Destination quadword index of this vertex's 3-quadword group in the GS packet. |

#### Vertex part 2 — `ShrubVertexPart2`, 8 bytes per vertex (V4-16)

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s16 | `s` | Texture coordinate S, 4.12 fixed point. |
| 0x2 | s16 | `t` | Texture coordinate T, 4.12 fixed point. |
| 0x4 | s16 | `h` | Third texture-coordinate component, always `4096` = 1.0 in 4.12. Almost certainly the perspective `Q` that accompanies ST in the RGBAQ register; Wrench neither exports nor interprets it. |
| 0x6 | s16 | `n_and_stop_cond` | Bits 0–14: index (0–23) into the class's normal table, which doubles as the index into the per-instance 24-entry colour palette (§1.7). Bit 15: end-of-vertex-stream flag. |

`read_shrub_class` masks with `0x7FFF` to get the index and notes "if this is negative the strip ends". `write_shrub_class` sets bit 15 on the vertex **four from the end** of the list (`part_2[size - 4]`), which implies the VU1 vertex loop runs three iterations of software pipeline past the flagged vertex before draining.

#### Reconstructing draw order

The three uploaded arrays are each sorted by kind, not by draw order; the true order is recovered by walking a running GS-packet quadword counter and, at each step, picking whichever of the three streams has a record whose `gs_packet_offset` equals the counter (`read_shrub_class`):

* a vertex GIF tag consumes **1** quadword,
* an AD-GIF block consumes **5** quadwords,
* a vertex consumes **3** quadwords (ST, RGBAQ, XYZF2).

The walk terminates when all three streams are exhausted, or early when the next vertex's `gs_packet_offset` equals `counter - 3`, which identifies the **padding vertices**: if a packet contains fewer than 6 real vertices, the last vertex is duplicated until there are 6, and every duplicate keeps the original's `gs_packet_offset`, so they all overwrite the same three GS quadwords. This is a VU1 pipeline-priming requirement, not geometry.

---

### 1.3b What VU1 actually does with a packet (program 56467, verified 2026-09-26)

Derived from the disassembly of VU1 program 56467 (`extracted/vu/56467.txt`, EE 0x101768), the DMA chain `ShrubProc` (level01 0x29cdf0, boot 0x228be8) builds, and a VU1 interpreter run over every retail packet (all 5,374, two instances each) that reproduced the rule below on every kick. `src/core/shrub.cpp` and `crates/rc-formats/src/shrub.rs` implement it; the Wrench walk of §1.3 gives the same strips on all retail packets.

* **Chain.** Per packet: `REF` of the VIF list with `BASE` = 0x02 / 0x78 (input double buffer), then `MSCAL` 0x11 / 0x15 (the two entry points toggle the buffers). Then per batch of up to **5** instances: `UNPACK S-32` of the batch count to VU 0xee / 0x17b, per instance a `REF` of 6 qw (24 RGBA8 palette words, `UNPACK V4-8` unsigned to slot + 4) and a `REF` of 5 qw (`UNPACK V4-32` to slot + 0: 4 matrix columns + one qw that overwrites palette entry 0), then `MSCALF 0x63`. Instance slots are 0x1c qw apart starting at buffer + 1.
* **Constants** (VU 0..2, boot template 0x1de9b0): `vf31` = A+D GIF tag (`NLOOP 1, NREG 4, REGS 0xeeee`), `vf30` = (3·2²³ + 0x810, 2²³, fog scale, fog offset), `vf29` = (fog min, fog max). The 2²³ bias turns float adds into integer address arithmetic.
* **Input buffer.** The three unpacks write a 0x76-qw buffer; VU1 reads the header at qw 0, the GIF tags from qw 1, the ad-gif blocks after them, part 1 at `vertex_offset` and part 2 at `vertex_offset + vertex_count` **by address**. The tag and ad-gif copy loops are do-while loops, so both counts must be ≥ 1.
* **Output buffers.** Three GS-packet buffers of **0xa8 qw** at 0x208, 0x2b0, 0x358 (this is Wrench's "168 qw" limit). At `MSCAL` time tags and ad-gif blocks (A+D tag `vf31` + 4 qw) are copied into two of them; each instance writes its vertices into one and kicks it, alternating.
* **Vertex loop.** Software-pipelined, two vertices per step. Vertex *i* writes ST·Q (Q = 1/w times `h`), RGBAQ (`palette[n]` rgb, alpha = w of the instance's 5th qw), XYZF2 (`ftoi4` screen xyz, fog from the instance origin depth, one value per instance) to slots `off`, `off+1`, `off+2`. The stop bit is tested **from vertex 2 on** and the loop drains **three more** vertices after the flagged one, so vertices `0 ..= stop + 3` are written. That relies on the VU rule that a branch reads the *old* value of a VI register written by the instruction just before it (the steady-state exits at instructions 232 and 244 test a register set by the preceding `mtir`; without the rule vertices `stop+2` and `stop+3` are never written). Retail data always has `vertex_count == stop + 4`.
* **Kick.** `XGKICK` at slot 0 of the buffer. The GIF reads a tag there: an A+D block (5 qw, EOP 0) or a vertex tag (PACKED, `NREG 3`, `REGS 0x412`, 1 + 3·NLOOP qw), until the vertex tag with EOP. Slots are resolved last-writer-wins in VU order (tags, ad-gifs, vertices): padding vertices rewrite their original's slots. (A second-instance skip of the A+D block for single-texture packets exists in the code but adds `vf31.z` = 0xeeee, a denormal, so it is a no-op.)
* **Texture state** is the tex0 `data_lo` of the last A+D block the GIF read and carries across packets; on retail every packet has an A+D block at slot 0.
* **VU1 quirk.** When the stop flag is on vertex 2 (6-vertex packets), the colour of vertex 3 is fetched from `n + 2·vi13` (the palette base is added twice on the exit at instruction 182); 7 retail packets hit this. Only colour is affected.
* Program 912339 (EE 0x1022e8, the second list in `ShrubProc`) has the same header/ad-gif copy but loops `vertex_count` times and clips; with `vertex_count == stop + 4` it processes the same vertices.

### 1.4 LOD, mip and billboard switching data

* **`mip_distance`** (header 0x10) controls texture mip selection for the mesh. The GS `TEX1.K` value stored in every AD-GIF block is derived from it by `compute_lod_k` (`shrub.cpp`): `K = round(-log2(distance) * 16 - 73)`, truncated to s16 and stored as an unsigned 16-bit pattern in a s32. Wrench's author notes this equation "is similar to the equation in the GS User's Manual and seems to fit most of the points in the original files", but is "kinda off for larger distances such as those of billboards" — so the exact retail derivation, especially for billboards, is **not confirmed**. At run time (verified 2026-09-26) the game does not derive K: the class init copies the on-disc K into TEX1 bits 32..43 and `mip_distance` only sets the texture-paging distance (§1.2 table; docs/plan/shrub_lighting.md §8).
* **`ShrubBillboard`** (pointed to by `billboard_offset`) — 0x40 bytes:

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | f32 | `fade_distance` | F = `trunc(fade_distance) & 0xff` (loader byte at runtime record +0x17; 256 → 0 = billboard only). Mesh opaque below F, mesh/billboard cross-fade over F..F + 8, billboard only beyond; D ≥ F + 24. **Verified** (level01 `ShrubProc` 0x29d274). |
| 0x04 | f32 | `width` | Quad width in **class units**: world width = `width · lo · scale / 1024`, lo = mean length of instance columns 0 and 1 (the loader's packed col2.w, 1/4096). Retail: `width · scale` = 1024. **Verified.** |
| 0x08 | f32 | `height` | Quad height, class units, × the column-2 length (`hi`) × scale / 1024. **Verified.** |
| 0x0c | f32 | `z_ofs` | World-Z offset of the quad's **bottom** edge from the instance origin, same scaling as `height` (retail 0 or about −height/2, i.e. centred). **Verified.** |
| 0x10 | GifAdData16 | `d1_tex1_1` | TEX1_1 (address byte 0x14 **on disc**): `data_lo` = K (s16, 1/16; retail −207..−167), `data_hi` = MMIN (4). Converted at load. |
| 0x20 | GifAdData16 | `d2_tex0_1` | TEX0_1 (address 0x06 on disc); on-disc data (1, 2 or 8) overwritten at load from the texture descriptor. |
| 0x30 | GifAdData16 | `d3_miptbp1_1` | MIPTBP1_1 (address 0x34 on disc); built at load from the descriptor's mip addresses. |

Correction (2026-09-26): the retail records **do** carry the register address bytes; the class init (boot `fun_00203b08`) rewrites only the data words (`sd` at +0x10/+0x20/+0x30), and `ShrubProc` sends the three qwords verbatim after an A+D tag. The quad is not fully camera-facing: it turns about world Z only (a cylindrical billboard), its width axis `(d.y, −d.x, 0)` with d = the 3D-normalised direction to the instance origin (so it narrows by cos(elevation)), height along world Z, drawn as a 4-vertex triangle strip (PRIM 0x7c) coloured with the average lit palette colour and the billboard alpha, in two GS passes (alpha-tested Z-writing pass for fully faded-in sprites, then a no-Z pass for fading ones). The switching policy, the sprite construction and the GS state are specified in docs/plan/shrub_lighting.md §6 (verified in the disassembly); `rc_formats::shrub::{shrub_fade, billboard_extent, billboard_corners}`.

#### `ShrubNormal` table — 24 entries, 0x10 bytes each

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x0 | s16 | X, as a fraction of 32767 |
| 0x2 | s16 | Y |
| 0x4 | s16 | Z |
| 0x6 | s16 | pad, 0 |

`read_shrub_class` reads **exactly 24** entries, unconditionally — the count is not stored anywhere. Each entry is a unit normal quantised to s16/32767. `write_shrub_class`'s own comment calls this block "the palette", which is the key to §1.7: the table is simultaneously a normal codebook for the mesh and the index space of the per-instance lighting palette. Retail tables are a fixed 24-direction sphere covering; Wrench, when building a class from scratch, generates a canonical evenly-distributed 24-point sphere and assigns each vertex the nearest direction by Euclidean distance (`compute_normal_clusters`), with a `TODO` about refining the clusters with k-means. Wrench does **not** know whether the retail files use this exact 24-direction set or a per-class fitted one.

---

### 1.5 Fixed-point scales

| Quantity | Storage | Conversion to world / float |
| --- | --- | --- |
| Vertex position | s16 per axis | `world = raw * scale / 1024`. The class `scale` (header 0x20) is a per-class float, so the quantum is `scale/1024` world units. Z is up; world units are the usual 1024-subunit ones. |
| Texture coordinate S, T | s16 | 4.12 fixed: `float = raw / 4096` (`vu_fixed12_to_float`, `src/core/vif.h`). |
| `h` | s16 | 4.12 fixed; always 1.0. |
| Normal | s16 per axis | `float = raw / 32767`. |
| Bounding sphere | f32[4] | Wrench's builder stores `sphere / scale`, i.e. centre and radius expressed in multiples of `scale` — **1024× coarser than the vertex quantum**. See the caveat below. |
| Instance colour | 3 × s32 | `0…255` per channel, `float = raw / 255`. |
| Instance matrix | 16 × f32 | Translation in world units; element [3][3] is `0.01` on disc. |

Wrench's optimal-`scale` heuristic (`compute_optimal_scale`) is `scale = max(|component| over all vertices) * 1024 / 32766`, i.e. pick the scale that maps the largest coordinate to nearly the full s16 range. This is a re-authoring choice, not a constraint of the format.

**Caveat on the bounding sphere:** the `/ scale` convention is used only by Wrench's *builder*; the *reader* copies the four floats through verbatim, so the byte-exact round-trip test for shrub class blobs does not validate the interpretation. Treat "bounding sphere is in units of `scale`" as an unverified Wrench assumption.

---

### 1.6 Packet size constraints

`setup_shrub_constraints` in `shrub.cpp` encodes the limits the original data obeys, expressed as two independent cost budgets per packet:

| Budget | Per packet | Per strip | Per vertex | Per material change | Limit |
| --- | --- | --- | --- | --- | --- |
| VU1 unpacked data (quadwords) | 1 (header) | 1 (GIF tag) | 2 (one quadword in each of the two vertex tables) | 4 (AD-GIF block) | **118** = 0x76 |
| GS packet (quadwords) | 0 | 1 (GIF tag) | 3 (ST + RGBAQ + XYZF2) | 5 (A+D GIFtag + 4 entries) | **168**, described as "max GS packet size in original files" |

The 118-quadword figure is exactly the VU1 input buffer size from `docs/shrub_renderer.md`. The VIF packet byte size needs no separate check because it is bounded by the unpacked size. The 168-quadword GS limit is empirical, not a hardware limit.

---

### 1.7 World transform, per-instance colour, and the VU1 program

#### What `docs/shrub_renderer.md` records about the microprogram

The shrub microprogram runs four sequential loops:

1. copy GIF tags into the GS packet,
2. copy the AD-GIF texture-state blocks into the GS packet,
3. convert the s16 vertex positions to floats,
4. transform the vertices and write them into the GS packet.

VU1 data memory map (quadword addresses; total 0x400 quadwords = 16 KB):

| Address | Size | Region |
| --- | --- | --- |
| 0x000 | 0x002 | Bookkeeping |
| 0x002 | 0x076 | Input buffer 1 |
| 0x078 | 0x076 | Input buffer 2 |
| 0x0ee | 0x11a | Instance array (up to **10** instances) |
| 0x208 | 0x1f8 | Output buffer (the GS packet under construction) |

The two input buffers are the double-buffered target of the packets' `FLG=1` (VIF1 TOPS-relative) UNPACKs, which is why packet destination addresses are small and relative.

Per-instance data in that array is **0x1c quadwords**:

| Offset (qw) | Size (qw) | Contents |
| --- | --- | --- |
| 0x0 | 0x4 | Instance matrix (object → clip/view) |
| 0x4 | 0x18 | **Colour palette — 24 entries** |

10 × 0x1c = 0x118, which fits the 0x11a region. Two register facts are recorded: integer register `vi11` is the input-buffer pointer across loops 1–4, and float register `vf17` holds the GS-packet write addresses. A second, separate program named **"Shrub Near"** exists with the same memory map — presumably a higher-quality path for close-up shrubs.

#### The colour mechanism

The 24-entry palette in the instance block lines up exactly with the 24-entry `ShrubNormal` table in the class, and each vertex's `n` index (0–23) selects one entry. Since each vertex emits an `RGBAQ` quadword and the vertex data contains no colour of its own, the only consistent reading is:

> The EE, once per instance per frame, computes 24 RGBAQ values by lighting each of the class's 24 canonical normals with the level's directional lights selected by the instance's `dir_lights` field, modulated by the instance's `colour`. It uploads those 24 values with the instance matrix. The microprogram then does no per-vertex lighting at all: it just looks up `palette[n]` and writes it as the vertex colour.

This makes shrub lighting **per-instance and per-normal-cluster**, quantised to 24 directions — cheap, static, and baked per frame rather than per vertex. Wrench itself implements none of this: the editor uploads only the instance matrices and draws shrub classes with plain textures, ignoring `colour` and `dir_lights` entirely (`draw_shrub_instances` in `src/editor/renderer.cpp`), and the glTF exporter drops vertex colour and writes the dequantised normal instead (§1.9). Note also that the editor renderer batches **runs of consecutive instances sharing a class**, which implies the gameplay file's shrub instance list is expected to be sorted by class number.

#### World transform

The instance's 4×4 matrix is applied directly to the dequantised local positions (`local = raw * scale / 1024`), so the full object→world transform is

```
world = M * (raw_xyz * scale / 1024, 1)
```

with rotation and any non-uniform scale in the 3×3 part and translation in the fourth column. The stored `[3][3]` element is `0.01`, not `1.0`; Wrench substitutes `1.0` on read and rewrites `0.01` on write (`swap_matrix`, `src/instancemgr/gameplay_impl_common.inl`), exactly as it does for tie instances. The value is presumably a constant the EE or VU code consumes for something else (a scale or fade factor) rather than a homogeneous-coordinate `w`.

---

### 1.8 Shrub instance record — 0x70 bytes

From `ShrubInstancePacked` and `swap_instance` in `src/instancemgr/gameplay_impl_classes.inl`, plus the `Shrub` instance type in `src/instancemgr/instance_schema.wtf`.

The instance block itself is: a **0x10-byte table header** whose first s32 is the instance count (rest padding), followed by `count` × 0x70-byte records (`InstanceBlock` in `gameplay_impl_common.inl`).

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | s32 | `o_class` | Shrub class number; matches a `ShrubClassEntry.o_class`. |
| 0x04 | f32 | `draw_distance` | Per-instance cull/fade distance. Note this is a **float**, unlike the tie instance's integer `draw_distance`. |
| 0x08 | s32 | unused | Always 0. |
| 0x0c | s32 | unused | Always 0. Ties keep their `occlusion_index` here; shrubs do not participate in occlusion culling. |
| 0x10 | f32[16] | `matrix` | Object→world matrix, column-major, 4 × `Vec4f`. `[3][0..2]` is the world position; `[3][3]` is `0.01` on disc. |
| 0x50 | s32 | `colour.r` | Base colour red, 0–255. |
| 0x54 | s32 | `colour.g` | Green. |
| 0x58 | s32 | `colour.b` | Blue. (Wrench's `Rgb96` struct has a stale `/* 0xc */` comment on `b`; it is three consecutive s32 so `b` is at +0x8.) |
| 0x5c | s32 | unused | Always 0. Exposed in Wrench's instance schema as `unknown_5c` but forced to zero on write, so retail files have 0 here. |
| 0x60 | s32 | `dir_lights` | Index into the level's directional-light list (the gameplay "lights" block), same role as `TieInstance.directional_lights`. Drives the per-instance colour palette. |
| 0x64 | s32 | unused | Always 0 (schema `unknown_64`). |
| 0x68 | s32 | unused | Always 0 (schema `unknown_68`). |
| 0x6c | s32 | unused | Always 0 (schema `unknown_6c`). |

Fields the caller might expect that are **absent**:

* **No `uid`.** Ties carry a `uid` at 0x54; shrub instances do not. Wrench synthesises an identity from the record's index in the table (`InstanceBlock::read` calls `set_id_value(index++)`), so a shrub's identity is purely positional.
* **No occlusion index** (see 0x0c above and `docs/occlusion_culling.md`).
* **No ambient RGBA array.** RAC1 tie instances embed a 0x80-byte per-vertex ambient colour array; shrubs instead get the 24-entry runtime palette described in §1.7.
* **No group membership field.** Groups are a separate block, and RAC1 does not have one for shrubs — see below.

#### Companion blocks

* **Shrub class list** (RAC1 gameplay header **0x38**): an s32 count followed by that many s32 class numbers — the distinct set of `o_class` values used by the level's shrub instances, in first-appearance order (`ShrubClassBlock::write`). Wrench's reader ignores it and rebuilds it on write, so it is pure redundancy/preload hinting.
* **Shrub groups**: **RAC1 has none.** `RAC_GAMEPLAY_BLOCKS` in `gameplay.cpp` defines only "shrub classes" (0x38) and "shrub instances" (0x3c); the `GroupBlock<ShrubGroupInstance>` entry first appears in the R&C2/3 table (at 0x44) and in Deadlocked's art-instances table. `docs/gameplay.md`'s RAC1 header map agrees. So a `ShrubGroup` instance type exists in Wrench's editor model but has no RAC1 on-disc representation.

---

### 1.9 Export algorithm (cross-check reference)

Wrench's shrub unpack path is `unpack_shrub_class` (`src/wrenchbuild/classes/shrub_class.cpp`) → `read_shrub_class` + `recover_shrub_class` (`src/engine/shrub.cpp`). To reproduce it:

1. Read `ShrubClassEntry` from the core index. Resolve the blob's byte range with the boundary-list method of §1.1.
2. Read `ShrubClassHeader` at blob+0; read `packet_count` × `ShrubPacketEntry` at blob+0x40.
3. For each packet: slice `[offset, offset+size)`, decode the VIF command list, keep only the UNPACKs, and assert there are exactly three.
4. From UNPACK 0's payload: `ShrubPacketHeader` at +0x00, `gif_tag_count` × `ShrubVertexGifTag` at +0x10, then `texture_count` × `ShrubTexturePrimitive`.
5. From UNPACK 1 and UNPACK 2: `vertex_count` records each of `ShrubVertexPart1` and `ShrubVertexPart2`, index-parallel.
6. Replay the GS packet order using the running quadword counter (§1.3), emitting: material switches at AD-GIF blocks; primitive-type changes at GIF tags; vertices otherwise. Stop on the padding-vertex condition. Any other state is corrupt data.
7. Read `ShrubBillboard` at `billboard_offset` if that field is > 0. Read 24 `ShrubNormal` entries at `normals_offset`.
8. Per vertex, produce: position `= (x,y,z) * scale / 1024`; normal `= normals[n] / 32767`; UV `= (s,t) / 4096`. Start a new output mesh primitive whenever the current texture index (from the most recent AD-GIF block's `d4_tex0_1.data_lo`) changes.
9. Convert triangle strips to triangle lists by emitting `(i, i+1, i+2)` for each window — **without** alternating winding — and triangle lists one-for-one.
10. Post-process: deduplicate vertices, drop zero-area triangles, then **recompute every triangle's winding order from its vertex normals**. Wrench's comment explains why: "the winding orders of the faces weren't preserved by Insomniac's triangle stripper". This is the single most important caveat for any independent exporter — the strip data does not carry reliable face orientation, and the normal table is the only orientation signal.
11. Materials: the 16 texture indices in `ShrubClassEntry.textures` index the shrub `TextureEntry` table; each is an 8-bit indexed texture whose pixels live in the core data at `textures_base_offset + entry.data_offset` and whose 256-entry CLUT lives in GS RAM at `entry.palette * 0x100`. Alphas are doubled with `0x80 → 0xff`, and the CLUT index order is unswizzled by swapping the middle two bits of the index (`index ^ 0b00011000` when bit 4 ≠ bit 3) — `Texture::multiply_alphas` / `swizzle_palette` / `map_palette_index` in `src/core/texture.cpp`. RAC1 pixel data is **not** re-swizzled (only Deadlocked is).
12. Billboard texture: `width * height` bytes of 8-bit indices at `texture_offset * 0x100` in GS RAM, CLUT at `palette_offset * 0x100`, same alpha and CLUT-order fixups.

Correctness of the *structural* half of this is well supported: `test_shrub_class_core` byte-compares read→write over whole retail blobs, and is enabled for RAC.

### 1.9b Verified on the retail NTSC-U disc (2026-09-26)

`src/core/shrub.cpp` implements §1.2–§1.3b and §1.8; `rc_extract shrub --level N` writes `shrub_dump.bin`, and `crates/rc-formats/src/shrub.rs` matches it byte for byte on all 19 levels (`shrubs_match_cpp_for_every_level`): **551 classes, 5,374 packets, 18,783 strips, 228,383 vertices (68 padding), 190,749 triangles, 25,572 instances, 83 billboards**. Every draw is a triangle strip, every `h` is 0x1000, every texture slot resolves to the shrub texture table, the header runtime fields are 0, and a class has a `ShrubBillboard` record exactly when its core-index billboard texture has `width != 0`. Correction to §1.2: the `ShrubNormal` table is 24 × **8 bytes** (s16 x, y, z, pad), unit length, not 0x180 bytes.

Instances: 0x08/0x0c/0x5c/0x64–0x6c are 0; matrix `[3][3]` is 0.01 on 25,367 and **0.0** on 205; `dir_lights` is 0–8 or 15 (never uses the blend byte); `draw_distance` 0–512 (mostly 32). The gameplay class list (0x38) is the distinct instance classes in first-appearance order. Class `mode_bits` is 0 (484), 4 (64) or 2 (3): `ShrubProc` skips a class with bit 0 and uses `(mode_bits & 6) >> 1` as a wind-sway mode (EE-side matrix perturbation with the 256-byte table it copies to scratchpad 0x70003b00). `s_class` is 0 everywhere; `mip_distance` is 5 on 531 classes.

The level loader (`FUN_00255958`, after pointer 0x3c) turns each instance into a 0x20-byte runtime record (+0x00 bounding sphere, +0x10 draw distance clamped to ≥ 16 and, for billboard classes, ≥ `trunc(fade_distance) + 24`; +0x17 `trunc(fade_distance)`; +0x18 index; +0x1a class index; +0x1c light selector from `dir_lights`; +0x1e point-light nibbles), a 0x40-byte matrix block (the matrix with column 0 w = `r | g<<8 | b<<16 | 0x80<<24`, column 1 w = average palette colour, column 2 w = packed column lengths, column 3 w = class `scale`) and a 0x60-byte palette slot. Lighting: `docs/plan/shrub_lighting.md`.

### 1.10 Shrub unknowns

* `mode_bits` bits other than 0 (skip) and 1–2 (sway mode). The sway math is decoded (docs/plan/shrub_lighting.md §7, `rc_formats::shrub::wind_sway`); its phase depends on the run-time EE address of the instance matrix block and the frame counter.
* `s_class` (header 0x26) — 0 on every retail class.
* The exact `TEX1.K` derivation at build time; Wrench's `-log2(d)*16 - 73` is a fit (the game uses the stored K as is).
* Resolved: the GS register values of the mesh ad-gifs (= the tfrag rule, boot `fun_00203b08`; per-frame TBP patch `fun_00228a30` / level01 0x29cc38) and the billboard A+D entries (docs/plan/shrub_lighting.md §8).
* The meaning of the `0.01` (or 0.0) in instance matrix `[3][3]`; the loader overwrites the runtime copy with the class `scale`.
* Which instances `ShrubProc` routes to the clipping program 912339 (presumably frustum-edge instances).
* The bounding-sphere `/ scale` convention (unvalidated, §1.5).

---

## Part 2 — The sky

### 2.1 Where the sky lives

The sky is a single self-contained block in the decompressed level core data, pointed to by `LevelCoreHeader.sky` (offset **0x10** of the core index header). Its size, like class blobs', is recovered from the block boundary list; `level_core_block_range` returns `{0,0}` when `header.sky == 0`, with the comment "e.g. if there is no sky", so **a level may legitimately have no sky block**. The block is written 0x40-aligned both inside itself (`write_sky` starts with `dest.pad(0x40)`) and in the core data (`pack_asset(..., 0x40)`). Sources: `src/wrenchbuild/level/level_core.cpp`, `src/wrenchbuild/level/sky_asset.cpp`, `src/engine/sky.cpp`.

All offsets inside the sky block are byte offsets **relative to the start of the block**.

```
+0x00  SkyHeader                     (0x40 bytes)
 ...   FX index list                 (fx_count bytes, 0x10-aligned)
 ...   SkyTexture[texture_count]     (0x10 each, 0x10-aligned)
 ...   texture data region           (0x40-aligned; palettes and pixels, each 0x40-aligned)
 ...   sprite scratch area           (0x40-aligned, maximum_sprite_count * 0x20 bytes, zeroed)
 ...   shell 0 … shell n-1           (each 0x10-aligned)
```

### 2.2 `SkyHeader` — 0x40 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | u8 | `colour.r` | Red. **Not read by the RAC1 shell code** (gouraud shells take per-vertex colours, §2.5); Wrench's "gouraud colour" reading is wrong for RAC1. |
| 0x01 | u8 | `colour.g` | Green. |
| 0x02 | u8 | `colour.b` | Blue. |
| 0x03 | u8 | `colour.a` | Alpha, PS2 convention: `0x80` means fully opaque (1.0); otherwise `a / 127`. |
| 0x04 | s16 | `clear_screen` | Frame-clear gate. **The disc value is dead**: the loader (boot 0x2028e0) overwrites it with 1; the frame render emits its clear packet while it is non-zero, and the per-level sky dispatch may zero it every frame (docs/plan/sky_render_notes.md). |
| 0x06 | s16 | `shell_count` | Number of shells, **maximum 8** (Wrench hard-verifies this on both read and write). |
| 0x08 | s16 | `sprite_count` | 0 on every retail disc. Run-time star-sprite count: the boot `update_sky_effects` (0x22ae70) sets it to 0x100 on first use and fills the sprite records. |
| 0x0a | s16 | `maximum_sprite_count` | Capacity: how many sprite slots to reserve. `docs/asset_reference.md`: "Controls how much memory to allocate for sprites." |
| 0x0c | s16 | `texture_count` | Number of `SkyTexture` definitions. |
| 0x0e | s16 | `fx_count` | Number of FX texture indices. |
| 0x10 | s32 | `texture_defs` | Offset to the `SkyTexture` array. |
| 0x14 | s32 | `texture_data` | Offset to the base of the texture data region; all `SkyTexture` offsets are relative to **this**. |
| 0x18 | s32 | `fx_list` | Offset to the FX index list (`fx_count` × u8). |
| 0x1c | s32 | `sprites` | Offset to the sprite scratch area, or 0 if `maximum_sprite_count == 0`. |
| 0x20 | s32[8] | `shells` | Offset of each shell's header. Only the first `shell_count` entries are meaningful; the rest are 0. |

The header layout is identical across RAC1, GC, UYA and DL — `read_sky`/`write_sky` use one struct for all four games.

**FX list.** `fx_count` single bytes, each an index into the `SkyTexture` definition array. Wrench treats them as a distinct leading run of the texture table: FX textures are `texture_defs[0 .. fx_count-1]` and shell materials are `texture_defs[fx_count .. texture_count-1]` — when duplicate texture definitions are collapsed the reader asserts the duplicate's index is `>= fx_count` ("Weird fx texture mapping"), and the packer always writes the identity mapping `fx[i] = i`. FX textures are presumably consumed by particle/lens-flare/sun code rather than by shell geometry. What they are *used for* is not known to Wrench.

**Sprites.** Wrench reserves `maximum_sprite_count * 0x20` zeroed bytes at 0x40 alignment and never interprets them. So sprite records are **0x20 bytes** each and are runtime scratch, built by the EE each frame. Their layout is unknown.

### 2.3 `SkyTexture` — texture definition, 0x10 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `palette_offset` | Offset of the 256-entry CLUT, relative to `texture_data`. |
| 0x4 | s32 | `texture_offset` | Offset of the pixel data, relative to `texture_data`. |
| 0x8 | s32 | `width` | Width in pixels. |
| 0xc | s32 | `height` | Height in pixels. |

At load the game rewrites each record in place (boot `0x2028e0`): `+0 u64` TEX0 cache (zeroed, and re-zeroed every frame by `setup_sky_gif_paging`), `+8 s16 texture_offset >> 4`, `+0xa s16 palette_offset >> 4`, `+0xc s16 log2(width)`, `+0xe s16 log2(height)`. The draw code (boot `0x22c208`) builds TEX0 = PSMT8, TBW = `1 << max(log2 w - 6, 0)`, TCC 1 (RGBA), TFX MODULATE, CPSM CT32, CSM1, CLD 4, and pages the pixels and CLUT into GS memory each frame. All retail sizes are powers of two (32…512).

Unlike level textures, sky textures are **stored inside the sky block itself, not in GS RAM**: the palette is 256 × u32 at `texture_data + palette_offset`, and the pixels are `width * height` bytes of 8-bit palette indices at `texture_data + texture_offset` (`read_sky_textures`). Both are written 0x40-aligned. There are no mipmaps.

Several definitions may point at the same pixel/palette pair; Wrench deduplicates byte-identical `SkyTexture` records into one exported image while keeping one table slot per duplicate.

Colour fixups for RAC1 (`read_sky_textures`, `src/core/texture.cpp`):

| Step | Operation |
| --- | --- |
| `multiply_alphas` | Per CLUT entry: `a < 0x80 → a * 2`, else `a = 0xff`. |
| `swizzle_palette` | Un-swizzle CLUT index order: swap bits 3 and 4 of the index (`i ^ 0b00011000` when they differ). |
| pixel re-swizzle | **RAC1: none.** Only Deadlocked calls `reswizzle()` / `swizzle()`. |

### 2.4 Shell header

A shell is one concentric layer of the sky (a dome/sphere shell, or a cloud layer). Read by `read_sky_shell`.

**RAC1 and GC — 8 bytes:**

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `cluster_count` | Number of clusters in this shell. |
| 0x4 | s32 | `flags` | **0 = textured, any other value = gouraud**: `sky_draw_shell` (boot 0x22b690) tests the whole word. Retail values are 0 and 1. |

(Wrench's `RacGcSkyShellHeader` carries stale `/* 0x0 */ /* 0x2 */` comments; the fields are two s32, so `flags` is at 0x4.)

**UYA and DL — 0x10 bytes**, for contrast:

| Offset | Type | Name |
| --- | --- | --- |
| 0x0 | s16 | `cluster_count` |
| 0x2 | s16 | `flags` — bit 0 untextured, bit 1 **bloom** |
| 0x4 | s16[3] | `rotation` — starting rotation X/Y/Z |
| 0xa | s16[3] | `angular_velocity` — X/Y/Z |

In **both** variants the cluster header array begins at **shell_offset + 0x10**, so RAC1 wastes 8 bytes of padding where the later games put animation state (zero on every retail shell).

**The RAC1 difference that matters most:** RAC1 (and GC) shells have **no rotation and no angular velocity** — there is no per-shell sky animation data in the format at all, and no bloom flag. Wrench explicitly gates these attributes to UYA/DL in both `sky_asset.cpp` (`if (config.game() != Game::RAC && config.game() != Game::GC)`) and `docs/asset_reference.md` (the `SkyShell` `bloom` / `starting_rotation` / `angular_velocity` attributes are listed as UYA/DL only). `docs/asset_system.md` records this as a deliberate correction in asset-format version 27: the attributes "now only will only apply for UYA and DL (which is more correct)". Any apparent sky motion in RAC1 must therefore be driven by code, not data. For the later games the s16 angle unit is **32768 = 2π**, applied per frame, so `radians_per_second = raw * framerate * 2π / 32768` (`rotation_to_radians_per_second`).

### 2.5 `SkyClusterHeader` — 0x20 bytes, `cluster_count` of them

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | f32[4] | `bounding_sphere` | Centre (x,y,z) + radius (w), in vertex units, for the per-cluster view cull (boot 0x22c4c8). |
| 0x10 | s32 | `data` | Offset of this cluster's data, **relative to the start of the sky block**. |
| 0x14 | s16 | `vertex_count` | Vertices in this cluster. In practice ≤ 127 (Wrench splits at `INT8_MAX`), and hard-limited to < 256 by the u8 face indices. |
| 0x16 | s16 | `tri_count` | Triangles in this cluster. |
| 0x18 | s16 | `vertex_offset` | Offset of the vertex array within `data` (always 0 in Wrench's output). |
| 0x1a | s16 | `st_offset` | Offset of the texture-coordinate array within `data`. |
| 0x1c | s16 | `tri_offset` | Offset of the face array within `data`. |
| 0x1e | s16 | `data_size` | Total size of the cluster's data, rounded up to 0x10. The game DMAs exactly `data_size >> 4` quadwords to the scratchpad, so all three arrays must lie inside it (checked by both parsers; true on all 19 levels). |

Within `data`: the vertex array starts 0x10-aligned, the ST array is 4-byte aligned, the face array is 4-byte aligned, and the whole thing is padded to 0x10. Clusters are the sky's culling and VU1-batching unit: each is a small, independently-drawable patch of the shell.

#### `SkyVertex` — 8 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | s16 | `x` | Position X. The game transforms the raw integers (`vitof0`, no /1024); since the sky is drawn with GS Z = 0 only the direction matters. **No per-shell or per-cluster scale factor** — unlike shrubs. |
| 0x2 | s16 | `y` | Position Y. |
| 0x4 | s16 | `z` | Position Z (up). |
| 0x6 | s16 | `alpha` | Textured shells: the low byte is the vertex alpha sent in RGBAQ = (0x80, 0x80, 0x80, alpha), 0x80 = 1.0. Ignored by gouraud shells. No normals. |

#### `SkyTexCoord` — 4 bytes

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x0 | u16 | S, 4.12 fixed (`/4096`); the game zero-extends (`pextlh` with `$zero`, then `vitof12`) |
| 0x2 | u16 | T, 4.12 fixed |

One 4-byte attribute per vertex, index-parallel with the vertex array (`vertex_count` of them). **In gouraud shells the same word is the vertex colour**, R, G, B, A bytes (0x80 = 1.0), written to RGBAQ verbatim (boot 0x22c0e0); Wrench's reading of it as ST plus a header-colour material is wrong for RAC1. Retail gouraud domes carry real gradients (e.g. level 0 `4d 28 19 80`).

#### `SkyFace` — 4 bytes

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0 | u8[3] | `indices` | Cluster-local vertex indices. Winding order is the **opposite** of glTF/OpenGL convention — Wrench reverses it in both directions. |
| 0x3 | u8 | `texture` | Index into the `SkyTexture` definition array, or **0xFF in gouraud shells** (the gouraud path never reads it). Validated against `texture_count`. |

So sky geometry is an **indexed triangle list** — no strips, no VIF command lists, no GIF tags, no AD-GIF blocks. The game emits each face as three GS vertices in stored index order (PRIM triangle list, no back-face culling, so winding is irrelevant), re-sending TEX0 whenever a face's texture differs from the previous drawn face of the cluster; a face is skipped only when all three vertices share an outside clip flag. That is the sharpest structural contrast with shrubs and tfrags: the sky's on-disc form is a plain, compact vertex/index mesh that the EE or VU1 must convert into GS packets at draw time.

A shell must be entirely textured or entirely untextured. Wrench's packer enforces this ("Sky shell contains both textured and untextured faces"), consistent with the single `flags` bit 0 that describes the whole shell. Faces within a textured shell may still switch texture freely, face by face.

### 2.6 FX / lighting / animation fields — summary for RAC1

| Concern | RAC1 representation |
| --- | --- |
| Overall sky colour | `SkyHeader.colour` (RGBA). Not used by the shell code; gouraud shells use per-vertex colours. |
| Screen clear | `SkyHeader.clear_screen`: disc value dead (loader sets 1); run-time gate, see the render notes. |
| Lighting | None. No normals or lights. Gouraud shells: per-vertex RGBA; textured shells: per-vertex alpha. |
| FX | `fx_count` byte indices at `fx_list` selecting textures from the definition table. Purpose not determined. |
| Sprites | `maximum_sprite_count` × 0x20 bytes of reserved scratch at `sprites`; layout and use unknown. |
| Animation / rotation | **Absent from the RAC1 data**; per-level code rotates chosen shells (docs/plan/sky_render_notes.md). (UYA/DL: per-shell starting rotation + angular velocity, s16 where 32768 = 2π per frame.) |
| Bloom | **Absent in RAC1.** (UYA/DL: shell flags bit 1.) |

### 2.7 How the sky is drawn

**Superseded by the decomp: see docs/plan/sky_render_notes.md.** The sky is not a VU1 renderer: the EE (VU0 macro mode) transforms vertices and builds GS triangle-list packets sent via VIF1 DIRECT; shells are drawn in index order with Z test ALWAYS, GS Z = 0, fog off and alpha blending on. The Wrench-based paragraphs below are kept for history.

`docs/renderers.md` lists a "Sky Renderer — Used for drawing the sky" among the engine's renderers but, unlike the moby, shrub and tfrag renderers, **there is no document for it**: `docs/README.md` lists "Sky Renderer" as a plain bullet with no link, and no VU1 memory map, register map or microprogram description exists anywhere in the tree. So: Wrench knows the sky is drawn by its own dedicated renderer over PATH1 (VIF1 → VU1 → GIF → GS), like the others, and knows nothing further — no program identity, no memory map, no GS state.

What the *data* implies: because clusters carry bounding spheres, the EE almost certainly frustum-culls per cluster, then for each surviving cluster builds a VIF list uploading its ≤127 vertices, STs and face indices for a VU1 program to transform (with a view-rotation-only matrix, since the sky does not translate with the camera) and expand into GS primitives. The absence of pre-baked GIF tags means primitive assembly happens at runtime, which is also what makes UYA/DL's per-shell rotation cheap to add. Shells are drawn innermost-to-outermost or the reverse in `shells[]` order, blended by per-vertex alpha — the order is not documented.

### 2.8 Export algorithm (Wrench's glTF path)

Unpack is `unpack_sky_asset` (`src/wrenchbuild/level/sky_asset.cpp`) → `read_sky` (`src/engine/sky.cpp`), producing one `.glb` with one glTF mesh and node per shell (`shell_0`, `shell_1`, …), plus PNG material files.

1. Read `SkyHeader`; verify `shell_count <= 8`.
2. Export `colour` (with the `0x80`→1.0 alpha rule), `clear_screen` and `maximum_sprite_count` as asset attributes.
3. Read the `fx_count` FX indices from `fx_list`.
4. Read the `texture_count` `SkyTexture` definitions. Deduplicate byte-identical definitions, build one image per unique texture plus a `texture_mappings` array (one slot per table entry → unique-image index). For each unique texture, read `width * height` index bytes and a 256-entry CLUT from the texture-data region, apply the alpha-doubling and CLUT-order fixups of §2.3 (no pixel re-swizzle for RAC1), write a PNG.
5. Emit the first `fx_count` table slots as standalone FX `Texture` assets; emit the remaining slots as glTF materials named `material_0…` (index-shifted down by `fx_count`), each `BLEND` and double-sided; emit one extra material named `gouraud` carrying the header colour as a base-colour factor, used for `texture == 0xFF` faces.
6. For each shell: read the game-appropriate shell header; derive `textured` from flags bit 0; for RAC1 skip rotation/bloom entirely. Then for `cluster_count` clusters at `shell_offset + 0x10 + i * 0x20`:
   * read `vertex_count` `SkyVertex` and `vertex_count` `SkyTexCoord`;
   * position `= (x,y,z) / 1024`; UV `= (s,t) / 4096`; colour `= (255, 255, 255, alpha == 0x80 ? 255 : alpha * 2)`;
   * read `tri_count` `SkyFace`; start a new mesh primitive whenever `face.texture` changes; emit indices **reversed** (`i2, i1, i0`) with the cluster's base vertex index added; a `texture` of 0xFF leaves the primitive material unset, which later becomes the `gouraud` material.
7. Merge all clusters of a shell into a single glTF mesh and deduplicate vertices — **cluster boundaries are not preserved in the export**. Repacking re-derives them (see below).
8. Write the `.glb`, and a `SkyShell` asset per shell referencing mesh `shell_i` by name.

#### Re-clustering on pack (why round-tripping is lossy)

`write_sky_shell` does not know the original cluster partition, so it invents one: it bins triangles by the direction of the normalised sum of their three positions, using 12 azimuth sectors (each 1/6 of a half-turn) crossed with three elevation bands (|elev| < 20°, 20°–65°, plus two polar caps covering > 65°, with a half-sector azimuth bias applied to the equatorial band). Within each bin it accumulates vertices until 127 vertices or 32767 faces, then flushes a cluster. The source comment: it "mimics how the sky shells are split up into clusters in the original games. It's not exactly accurate, but I think it's close enough." There is also a `TODO: Should maybe check the cluster size too`.

Consistent with that, the sky diff test (`test_sky_asset`) compares the header region byte-for-byte but **explicitly ignores every cluster's `bounding_sphere`** — direct evidence that Wrench's bounding-sphere approximation does not reproduce the retail values. Header-region equality does hold, so the header, FX list, texture-definition and shell-header layouts above can be considered confirmed against retail RAC1 data.

### 2.8b Verified on the retail NTSC-U disc (2026-09-26)

`src/core/sky.cpp` implements §2.2–§2.5. `rc_extract sky --level N` parses levels 0, 1, 5, 9 and 18: 2–7 shells, 50–135 clusters, every face index below its cluster's vertex count and every texture reference below `texture_count`; shell 0 is the untextured (flag bit 0) dome where present. Sky textures decode to sensible PNGs with the same palette rules as level textures. Header colours seen: alpha 0 or 0x80, `clear_screen` 0 or 1.

### 2.8c Rust port (2026-09-26)

`crates/rc-formats/src/sky.rs` parses all 19 retail skies byte-identically to `src/core/sky.cpp` (`sky_dump.bin`, golden test `sky_matches_cpp_for_every_level`): 75 shells (15 gouraud), 1700 clusters, 27349 vertices, 26496 triangles, 102 textures. Every shell's 8 padding bytes are zero, `flags` is 0 or 1, `vertex_offset` is 0, all arrays lie inside `data_size`, all texture sizes are powers of two, and no shell mixes textured and 0xFF faces.

### 2.9 Sky unknowns

* Sprite record layout (0x20 bytes each) and what sprites are for.
* What the FX texture list feeds — Wrench exports the textures but has no model of their use.
* Shell flags beyond bit 0 in RAC1; whether bit 1 (bloom in UYA/DL) is inert or means something else here.
* `SkyHeader.sprite_count` at 0x08 — assumed runtime-only; never written.
* Draw order of shells, blending state, depth/Z handling, and how the sky is positioned relative to the camera.
* Which VU1 microprogram draws the sky, its memory map, and how clusters are batched into VIF packets — completely undocumented in Wrench.
* The retail cluster-partitioning rule (Wrench approximates it) and the exact bounding-sphere computation (excluded from the diff test).
* Whether `vertex_offset` is ever non-zero in retail data (Wrench always writes 0).
* The RAC1 8-byte shell header's padding to 0x10 — whether those 8 bytes are truly unused or hold something Wrench zeroes.

---

## Appendix — Wrench files consulted

| File | Contributed |
| --- | --- |
| `src/engine/shrub.h` | All shrub on-disc struct layouts. |
| `src/engine/shrub.cpp` | Shrub read/write/recover/build algorithms, VIF prologue, fixed-point scales, packet constraints, normal clustering, LOD `K` formula. |
| `src/engine/sky.h` | All sky on-disc struct layouts. |
| `src/engine/sky.cpp` | Sky read/write, texture handling, cluster/shell layout, re-clustering heuristic, rotation units. |
| `src/engine/gif.h` | GIFtag and A+D field layouts, GS register addresses, PRIM bits. |
| `src/engine/basic_types.h` | `Vec3f`/`Vec4f`/`Mat3`/`Mat4` packing. |
| `src/core/vif.h`, `src/core/vif.cpp` | UNPACK encoding, VNVL/FLG/USN, 4.12 fixed-point macros. |
| `src/core/tristrip.h`, `src/core/tristrip_packet.h` | Packet/primitive/constraint model used by the shrub builder. |
| `src/core/texture.cpp` | Alpha doubling, CLUT index un-swizzle (`map_palette_index`). |
| `src/core/mesh.cpp` | `approximate_bounding_sphere`. |
| `src/wrenchbuild/level/level_core.h` | `LevelCoreHeader`, `ShrubClassEntry`. |
| `src/wrenchbuild/level/level_core.cpp` | Block boundary enumeration, sky/shrub unpack dispatch, alignment. |
| `src/wrenchbuild/level/level_classes.cpp` | Shrub class table unpack/pack, billboard texture hookup. |
| `src/wrenchbuild/level/level_textures.h`, `.cpp` | `TextureEntry`, the 0x10-byte billboard texture descriptor, GS RAM addressing. |
| `src/wrenchbuild/level/sky_asset.cpp` | Sky glTF export/import, FX and material mapping, diff-test exclusions. |
| `src/wrenchbuild/classes/shrub_class.cpp` | Shrub glTF export/import, billboard asset mapping, byte-exact core diff test. |
| `src/instancemgr/gameplay_impl_classes.inl` | `ShrubInstancePacked`, `ShrubClassBlock`, tie comparison. |
| `src/instancemgr/gameplay_impl_common.inl` | `InstanceBlock`, `TableHeader`, `Rgb96`, `swap_matrix`, `SWAP_COLOUR`. |
| `src/instancemgr/gameplay.cpp` | RAC1 gameplay block table (no shrub groups). |
| `src/instancemgr/instance_schema.wtf` | Shrub/ShrubGroup instance field names and component set. |
| `src/instancemgr/instance.h` | Instance component accessors. |
| `src/editor/renderer.cpp`, `src/editor/level.cpp` | How Wrench actually draws shrub instances (class-run batching; colour ignored). |
| `docs/shrub_renderer.md` | Shrub VU1 four-loop structure, memory map, instance layout (matrix + 24-entry colour palette), register notes, "Shrub Near" program. |
| `docs/renderers.md`, `docs/README.md` | PATH1 overview; existence of a sky renderer with no documentation. |
| `docs/level_core.md` | Core block ordering for R&C1. |
| `docs/gameplay.md` | RAC1 gameplay header map. |
| `docs/asset_reference.md` | ShrubClass/ShrubClassCore/ShrubBillboard/Sky/SkyShell attribute semantics and per-game availability. |
| `docs/instance_reference.md` | Shrub instance field list. |
| `docs/occlusion_culling.md` | Shrubs excluded from occlusion culling. |
| `docs/asset_system.md` | History of the sky rotation / glTF changes and the RAC/GC gating. |
| `docs/collision.md`, `docs/collision_recovery.md` | Shrub collision is baked into the level's static collision mesh, not stored per class. |

---

**Process note for the caller:** everything above came from reading the files listed in the appendix; no files were created or modified. Two Wrench-side issues worth flagging if you ever upstream anything: the duplicate `ShrubBillboardInfo` type name across `src/engine/shrub.h` and `src/wrenchbuild/level/level_textures.h` (two different layouts, one of which is an on-disc struct — an ODR hazard if those headers ever meet in one translation unit), and stale offset comments in `Rgb96` (`/* 0xc */` on a field at +0x8) and `RacGcSkyShellHeader` (`/* 0x2 */` on a field at +0x4).
