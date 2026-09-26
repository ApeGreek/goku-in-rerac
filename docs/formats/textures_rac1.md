# RAC1 textures, palettes and GS RAM

A format specification for the texture and palette data of *Ratchet & Clank* (2002, PS2; Wrench game id `rac` / RAC1), reverse-engineered by reading the Wrench source tree (GPL — used as reference only; no code is reproduced here). Every claim is attributed to the file it was derived from. Facts that Wrench's code does not establish are collected in the **Unknowns** section at the end.

Conventions: all integers are little-endian; `s16`/`s32` are signed; offsets written `0x…`. "core_index" is the uncompressed level-core index lump, "core_data" is the decompressed core data lump, "gs_ram" is the third, raw lump. Their locations come from the level `data` WAD header (`src/wrenchbuild/level/level_data_wad.cpp`, `RacLevelDataHeader`): `core_index` at 0x10, `gs_ram` at 0x18, `core_data` at 0x50, each a `ByteRange {s32 offset; s32 size;}`. Only `core_data` is compressed (Wrench's `decompress_wad`); `core_index` and `gs_ram` are stored raw (`src/wrenchbuild/level/level_core.cpp`, `unpack_level_core`).

---

## 1. Overview of the three lumps

| Lump | Compression | Contents relevant to textures |
|---|---|---|
| `core_index` | none | `LevelCoreHeader` (0xbc bytes) at offset 0; class tables; the four `TextureEntry` tables; the `part_textures` and `fx_textures` tables; the `gs_ram` descriptor table; `part_defs` |
| `core_data` | WAD-compressed | The shared 8-bit texture pixel blob (starting at `textures_base_offset`), the particle bank (`part_bank_offset`) and the FX bank (`fx_bank_offset`), plus geometry/collision/sky |
| `gs_ram` | none | Palettes, small mipmap images, and (RAC2+ only) whole "stashed" textures. A flat staging image that the game DMAs into GS local memory. |

Everything is referenced by byte offsets or by 0x100-byte block indices; there are **no per-texture headers anywhere in the level core** — width, height, format and palette all live in the index tables.

Source: `src/wrenchbuild/level/level_core.cpp` (`unpack_level_core`, `pack_level_core`), `src/wrenchbuild/level/level_data_wad.cpp`.

---

## 2. Index structures

### 2.1 `LevelCoreHeader` (core_index + 0x00, 0xbc bytes)

`ArrayRange` is `{s32 count; s32 offset;}` — note **count first** (`src/core/util/binary_util.h:74`). Texture-relevant fields (full struct in `src/wrenchbuild/level/level_core.h`):

| Offset | Type | Name | Meaning |
|---|---|---|---|
| 0x00 | ArrayRange | `gs_ram` | Table of `GsRamEntry`, at a core_index offset |
| 0x18 | ArrayRange | `moby_classes` | `MobyClassEntry[]` |
| 0x20 | ArrayRange | `tie_classes` | `TieClassEntry[]` |
| 0x28 | ArrayRange | `shrub_classes` | `ShrubClassEntry[]` |
| 0x30 | ArrayRange | `tfrag_textures` | `TextureEntry[]` |
| 0x38 | ArrayRange | `moby_textures` | `TextureEntry[]` |
| 0x40 | ArrayRange | `tie_textures` | `TextureEntry[]` |
| 0x48 | ArrayRange | `shrub_textures` | `TextureEntry[]` |
| 0x50 | ArrayRange | `part_textures` | `ParticleTextureEntry[]` |
| 0x58 | ArrayRange | `fx_textures` | `FxTextureEntry[]` |
| 0x60 | s32 | `textures_base_offset` | Base of the shared pixel blob, **in core_data** |
| 0x64 | s32 | `part_bank_offset` | Base of the particle texture bank, in core_data |
| 0x68 | s32 | `fx_bank_offset` | Base of the FX texture bank, in core_data |
| 0x6c | s32 | `part_defs_offset` | Particle definition block, **in core_index** |
| 0x80 | s32 | `gadget_count_rac1` | RAC1 only: number of gadget entries |
| 0x84 | s32 | `gadget_offset_rac1` | RAC1 only: `RacGadgetHeader[]` in core_index |
| 0x90 | s32 | `chrome_map_texture` | environment-map texture address (unit unknown) |
| 0x94 | s32 | `chrome_map_palette` | environment-map palette address |
| 0x98 | s32 | `glass_map_texture` | glass map texture address |
| 0x9c | s32 | `glass_map_palette` | glass map palette address |

Wrench hard-codes `glass_map_texture = 0x4000` and `glass_map_palette = 0x400` when packing and leaves the chrome fields zero (`level_core.cpp:399-400`), so these four values are observed constants rather than understood addresses.

Fields at 0x84/0x80 are reused for the moby GS stash in RAC2/3/DL; in RAC1 they are the gadget table instead, and there is **no moby texture stash in RAC1** — Wrench skips the stash list and the stash base entirely when the game is RAC (`level_core.cpp`, the `config.game() != Game::RAC` guards).

### 2.2 `TextureEntry` (0x10 bytes) — `src/wrenchbuild/level/level_textures.h`

| Offset | Type | Name | Meaning |
|---|---|---|---|
| 0x0 | s32 | `data_offset` | Byte offset of the pixel data, relative to `textures_base_offset` inside decompressed core_data |
| 0x4 | s16 | `width` | pixels |
| 0x6 | s16 | `height` | pixels |
| 0x8 | s16 | `type` | see §5 |
| 0xa | s16 | `palette` | Palette address in the gs_ram lump, in **0x100-byte units** |
| 0xc | s16 | `mipmap` | Mipmap address in the gs_ram lump, in **0x100-byte units**; -1 = none |
| 0xe | s16 | `pad` | -1 in Wrench-built files |

### 2.3 `GsRamEntry` (0x10 bytes) — `src/wrenchbuild/level/level_textures.h`

| Offset | Type | Name |
|---|---|---|
| 0x0 | s32 | `psm` |
| 0x4 | s16 | `width` |
| 0x6 | s16 | `height` |
| 0x8 | s32 | `address` |
| 0xc | s32 | `offset` |

### 2.4 Class entries — `src/wrenchbuild/level/level_core.h`

`MobyClassEntry` and `TieClassEntry` are identical 0x20-byte structures: `s32 offset_in_asset_wad; s32 o_class; s32 unknown_8; s32 unknown_c; u8 textures[16];`.
`ShrubClassEntry` is 0x30 bytes: the same 0x20 bytes followed by a 0x10-byte `ShrubBillboardInfo` at 0x20.
`RacGadgetHeader` (RAC1 only, 0x10 bytes): `s32 offset_in_asset_wad; s32 class_number; s32 compressed_size; s32 pad;`.

---

## 3. Pixel format of the shared texture blob

**Confirmed 8-bit indexed (PSMT8 / `IDTEX8`, psm 0x13).** A texture's pixels are exactly `width * height` bytes read from `core_data[textures_base_offset + data_offset]`, one byte per pixel, and are handed straight to an 8-bit paletted image constructor — there is no header, no stride padding, and no length field (`src/wrenchbuild/level/level_textures.cpp`, `unpack_level_material`; `src/core/texture.cpp`, `Texture::create_8bit_paletted` asserts `data.size() == width * height`).

**Row-major, top-down, unswizzled for RAC1.** The pixel index → GS-native-layout permutation (`Texture::swizzle` / `reswizzle`, implemented by the static helper literally named `map_pixel_index_rac4` in `src/core/texture.cpp`) is applied **only** when the game is Deadlocked (`if (game == Game::DL)` in `unpack_level_material`, `unpack_shrub_billboard_texture`, `unpack_particle_textures`, `unpack_fx_textures`). RAC1, RAC2 and RAC3 level textures are therefore plain linear rasters: pixel `(x, y)` is byte `y * width + x`, first row at the top (this follows from the PNG writer handing row `y` the pointer `data + y*width` with no flip — `src/core/png.cpp`, `write_png`, `PALETTED_8` case).

**Alignment.** When Wrench writes the blob each texture is aligned to 0x100 bytes and `textures_base_offset` itself is 0x100-aligned (`level_textures.cpp`, `write_shared_level_textures`). Retail data is presumably the same, since `palette`/`mipmap` are expressed in 0x100 units.

**Dimensions.** Powers of two, at least 8 pixels wide (asserted in `Texture::generate_mipmaps`, `src/core/texture.cpp`). Widths/heights in `TextureEntry` are the real pixel dimensions, not log2 values.

### 3.1 Mipmaps

The `mipmap` field is **not** an index into the `TextureEntry` table; it is an address in the gs_ram lump, in 0x100-byte units, of a single reduced copy of the texture. Two facts pin this down (`level_textures.cpp`, `write_shared_level_textures` and `write_level_texture_table`):

* Wrench builds one reduced image of `width/4 × height/4` pixels by point-sampling every 4th pixel in both axes, writes it into the gs_ram lump 0x100-aligned, and stores `mipmap = gs_offset / 0x100`.
* It simultaneously emits a `GsRamEntry` for that image with `psm = 0x13`, `width = width/4`, `height = height/4`, and `address = offset = ` the byte offset in the lump.

`mipmap = -1` means no mipmap (the struct's default, and what a GS-resident/stashed texture gets in RAC2+). Wrench's **unpacker ignores `mipmap` entirely** — it never reads the mip data back — so the "one quarter-size level" layout is Wrench's writing convention, verified only insofar as rebuilt levels work in-game. See Unknowns.

---

## 4. Palettes

### 4.1 Where they live and how `palette` addresses them

`TextureEntry.palette` is a **0x100-byte block index into the raw `gs_ram` lump**, not an index into the `GsRamEntry` table and not a GS hardware address that Wrench needs to decode. The palette is read as 256 consecutive `u32`s (1024 bytes) at `gs_ram + palette * 0x100` (`level_textures.cpp`, `unpack_level_material`). The packer confirms the inverse: it 0x100-aligns each palette in the lump, writes 256 `u32`s, and stores `palette_offset / 0x100` (`write_shared_level_textures`, `write_level_texture_table`).

Because a palette occupies 1024 bytes = four 0x100 blocks, consecutive distinct palettes differ by 4 in this field.

### 4.2 Pixel format of a palette entry

**RGBA32, 256 entries, 4 bytes per entry, byte order R, G, B, A.** The `u32` read from the file is decomposed as red = bits 0-7, green = bits 8-15, blue = bits 16-23, alpha = bits 24-31 (`src/core/png.cpp`, `write_png`; `src/core/texture.cpp`, `to_rgba`). Every palette Wrench reads or writes for level textures, billboards, particles and FX is 256×RGBA32 unconditionally.

The `GsRamEntry.psm` enumeration allows other palette formats (`level_textures.h`):

| psm | Symbol | Meaning |
|---|---|---|
| 0x00 | `PSM_RGBA32` | palette, 32-bit RGBA |
| 0x01 | `PSM_RGBA16` | palette, 16-bit RGBA (`RGBA16`, presumably 1-5-5-5) |
| 0x13 | `PSM_IDTEX8` | 8-bit indexed image (a texture or a mipmap) |

Wrench only ever *writes* 0x00 and 0x13 and never has a code path that decodes an RGBA16 palette, so psm 0x01 is declared but unhandled.

### 4.3 CLUT storage order (CSM1 8-entry-block swizzle)

Palettes are stored in the PS2's CSM1 CLUT layout and must be permuted to become a linear 256-entry palette. The permutation (`map_palette_index` in `src/core/texture.cpp`, also written out in `docs/textures.md`) swaps bits 3 and 4 of the index whenever they differ:

```
given index i in 0..255
  b3 = (i >> 3) & 1 ;  b4 = (i >> 4) & 1
  j  = (b3 != b4) ? (i ^ 0x18) : i
linear_palette[i] = raw_palette[j]
```

Expressed as blocks: treat the 256 entries as 8 groups of 32, and each group of 32 as four 8-entry blocks numbered 0,1,2,3. **Within every 32-entry group, swap the middle two 8-entry blocks** — the stored order 0,1,2,3 becomes the linear order 0,2,1,3. Concretely, per 32-entry group: linear 0-7 ← stored 0-7; linear 8-15 ← stored 16-23; linear 16-23 ← stored 8-15; linear 24-31 ← stored 24-31.

The permutation is an involution, so the identical mapping converts a linear palette back to CLUT order — Wrench calls the same `swizzle_palette()` on both the unpack and pack paths (`unpack_level_material` vs. `pack_level_core`).

This applies to **all** RAC1 level palettes: shared textures, shrub billboards, particle textures, FX textures, and 8-bit PIF files (§9). It is *not* applied to 4-bit PIF palettes (`src/wrenchbuild/common/texture_asset.cpp`, `unpack_pif`, format 0x94).

### 4.4 Alpha scale

PS2 alpha is 0-0x80 with **0x80 = fully opaque**. Conversion to 8-bit PNG alpha (`Texture::multiply_alphas`, `src/core/texture.cpp`):

```
a' = (a < 0x80) ? a * 2 : 0xFF
```

and the inverse used when packing (`divide_alphas`, with `handle_80s = true`):

```
a  = (a' == 0xFF) ? 0x80 : a' / 2
```

For paletted textures this operates on the palette entries, not the index bytes. Note that alpha values above 0x80 are *clamped* to opaque on export and are therefore lost — in UYA/DL such values select bloom or reflectivity effects (`docs/textures.md`); whether RAC1 uses >0x80 alpha for anything is unknown. The variant `divide_alphas(false)` (no 0xff→0x80 special case) is used only for RGBA assets (`texture_asset.cpp`).

---

## 5. The `type` field

Wrench **never reads** `TextureEntry.type` when unpacking. It only writes it (`level_textures.cpp`, `write_level_texture_table`):

| Value written | Condition |
|---|---|
| 0 | the texture is "stashed" — permanently resident in GS memory, pixel data living in the gs_ram lump (RAC2/3/DL only) |
| 3 | ordinary texture, pixel data in the core_data blob |

Since RAC1 has no GS stash (`level_core.cpp` guards the stash list and stash base with `config.game() != Game::RAC`; the asset-schema attribute `stash_textures` is documented as GC/UYA/DL only in `docs/asset_reference.md:957`), Wrench writes `type = 3` for every RAC1 level texture. The real enumeration and what values retail RAC1 discs contain are unknown; presumably it selects the GS texture function / TEX0 bits (e.g. decal vs. modulate) or the residency mode.

---

## 6. The `gs_ram` lump and its descriptor table

The lump is a flat staging image of a region of GS local memory; the `GsRamEntry` table describes the objects inside it. **Both palettes and images are described** (`level_textures.cpp`, `write_shared_level_textures`):

| Object | psm | width/height | address | offset |
|---|---|---|---|---|
| Texture palette | 0x00 | 0 / 0 as written by Wrench | byte offset in lump | same as `address` |
| Reduced mipmap image | 0x13 | `tex_w/4` / `tex_h/4` | byte offset in lump | same as `address` |
| Stashed texture (RAC2+ only) | 0x13 | full texture w/h | absolute byte offset in lump | offset **relative to the start of the stash region** |

So in Wrench's writer `address` and `offset` are both byte offsets into the lump and are equal, *except* for stash entries where `offset` is stash-relative — which is exactly what the comment on the field in `level_textures.h` says. The only place Wrench ever consumes `address` is to obtain the stash base, and it uses it directly as a byte offset into the lump (`level_core.cpp`: `moby_stash_addr = gs_table[header.gs_ram.count].address`, then `gs_ram.read_multiple<u8>(moby_stash_addr + entry.data_offset, …)`).

Layout ordering as Wrench writes it: for each unique texture, its palette (0x100-aligned, 1024 bytes) then its mipmap (0x100-aligned) interleaved in texture order; then, after all of those, the stash region (RAC2+). Both `TextureEntry.palette` and `TextureEntry.mipmap` are these byte offsets divided by 0x100, which is why nothing in the lump is ever finer-grained than 256 bytes.

**Interpretation of the 0x100 unit** (inference, not established by Wrench): 0x100 bytes is exactly one GS memory *block*, and the GS `TEX0.TBP0` / `TEX0.CBP` fields are specified in units of 64 words = 256 bytes. A 0x100-block index into a lump that is DMA'd to a fixed GS base therefore doubles as a GS block address relative to that base, which is the most plausible reason the format uses this unit. The hard-coded `glass_map_texture = 0x4000` sits awkwardly with a pure block reading, since 0x4000 blocks is the full 4 MB of GS RAM; it may be a byte offset instead. Flagged as unknown.

For RAC1 the table has exactly `header.gs_ram.count` entries; for RAC2+ an extra `moby_gs_stash_count_rac23dl` entries follow it (`level_core.cpp`, `unpack_level_core`).

---

## 7. Class → texture mapping

### 7.1 `u8 textures[16]`

Each `MobyClassEntry` / `TieClassEntry` / `ShrubClassEntry` carries a 16-byte array of **indices into that class kind's `TextureEntry` table** (`moby_textures`, `tie_textures`, `shrub_textures` respectively). Iteration stops at the **first `0xff`**; there is no separate count (`level_textures.cpp`, `unpack_level_materials`: the loop runs `i` from 0 to 15 and `break`s on `indices[i] == 0xff`). Slot `i` becomes material `i` of the class.

Consequences confirmed by the packer (`write_level_texture_indices`, plus the `verify` in `read_level_textures`): a class may have at most 15 textures so that a terminating `0xff` always fits, and a table index must be < 0xff (`"Too many textures."`). Bytes after the terminator are filled with 0xff.

### 7.2 Which slot the geometry references

Inside class geometry the material is selected by the low word of a GS `TEX0` register embedded in the VIF/GIF A+D data, and that value is the **slot index 0-15 within the class's own list**, not a table index:

* moby: `d3_tex0_1.data_lo` (`src/engine/moby_high.cpp:94,195`, `src/engine/moby_packet.h:49`)
* shrub: `d4_tex0_1.data_lo` (`src/engine/shrub.cpp:329,441`, `src/engine/shrub.h:106`)
* tie: `d1_tex0_1` (`src/engine/tie.h:180`)
* tfrag: `d1_tex0_1.data_lo` (`src/engine/tfrag_high.cpp:80,199`, `src/engine/tfrag_low.h:76`)

Negative values are special for mobies (`src/engine/moby_packet.h:57-59`): `-1` = no texture, `-2` = chrome (environment map), `-3` = glass — which is what the header's `chrome_map_*` / `glass_map_*` fields serve.

### 7.3 tfrag textures

The tfrag table is not addressed through a 16-byte list. Wrench unpacks one material per table entry, in table order, so a tfrag's material index equals its index in `tfrag_textures` (`level_core.cpp`, the loop over `tfrag_textures`).

### 7.4 RAC1 gadgets

RAC1 gadget classes have their own `RacGadgetHeader` table (`gadget_offset_rac1` / `gadget_count_rac1`). Their textures are *not* in a separate table: Wrench matches each gadget's `class_number` against the `moby_classes` table to find the corresponding `MobyClassEntry`, then uses that entry's `textures[16]` against the ordinary `moby_textures` table (`level_core.cpp`, the `config.game() == Game::RAC` block at the end of `unpack_level_core`). Gadget geometry itself is separately compressed inside core_data (`compressed_size` in the header).

---

## 8. Particle and FX textures

### 8.1 Particle textures

`ParticleTextureEntry` (0x10 bytes, `level_textures.h`):

| Offset | Type | Name | Meaning |
|---|---|---|---|
| 0x0 | s32 | `palette` | byte offset of a 256×RGBA32 palette, **relative to `part_bank_offset`** in core_data |
| 0x4 | s32 | `unknown_4` | written as 0 by Wrench |
| 0x8 | s32 | `texture` | byte offset of the pixel data, relative to `part_bank_offset` |
| 0xc | s32 | `side` | edge length; particle textures are **square**, so the data is `side * side` bytes |

Pixels are 8-bit indexed, unswizzled for RAC1; the palette is CSM1-swizzled and alpha-scaled exactly as in §4 (`unpack_particle_textures`). Both palettes and pixel data live in the particle bank inside core_data — *not* in the gs_ram lump — and Wrench 0x100-aligns each of them when packing (`write_nonshared_texture_data`).

The mapping from a particle (an animation) to its frames goes through `part_defs_offset`, which points **inside core_index** to:

```
PartDefsHeader  (0x10 bytes)
  0x0 s32 particle_count
  0x4 s32 unknown_4
  0x8 s32 indices_offset     // relative to part_defs_offset
  0xc s32 indices_size       // number of index bytes == number of frames total
0x10  s32 offsets[particle_count]
…
indices_offset: u8 indices[indices_size]
```

`offsets[p] == 0` means particle `p` is unused. Otherwise particle `p`'s frames are `indices[offsets[p] - indices_offset .. next_nonzero_offset - indices_offset)`, i.e. the offsets are part_defs-relative pointers into the index array and each particle's frame run ends where the next used particle's run begins (last one ends at `indices_size`). Each index byte selects an entry in the `part_textures` table (`unpack_particle_textures`, `pack_particle_textures`). Wrench limits the table to fewer than 0x100 entries, consistent with the u8 indices. For RAC1 it writes `particle_count = 0x51` when rebuilding (`pack_particle_textures`), i.e. 81 particle slots.

### 8.2 FX textures

`FxTextureEntry` (0x10 bytes, `level_textures.h`):

| Offset | Type | Name | Meaning |
|---|---|---|---|
| 0x0 | s32 | `palette` | byte offset of a 256×RGBA32 palette, **relative to `fx_bank_offset`** in core_data |
| 0x4 | s32 | `texture` | byte offset of the pixel data, relative to `fx_bank_offset` |
| 0x8 | s32 | `width` | pixels |
| 0xc | s32 | `height` | pixels |

Same 8-bit indexed, unswizzled-for-RAC1 pixel format; `width * height` bytes; palette CSM1-swizzled and alpha-scaled (`unpack_fx_textures`). An absent entry is written as all `-1`. Wrench has curated FX texture *name* lists for GC, UYA and DL (things like `lame_shadow`, `font_1`, `jp_thrust_fire`, `target_reticule`, `tv_scanlines`) but **no list for RAC1**, so RAC1 FX slots are exported numbered (`unpack_fx_textures`, `GC_FX_TEXTURE_NAMES` etc. in `level_textures.cpp`). The GC list is a reasonable starting guess for RAC1's ordering but is not verified for RAC1.

---

## 9. Shrub billboard textures

`ShrubBillboardInfo` occupies bytes 0x20-0x2f of a `ShrubClassEntry` (`level_core.h:103`, struct in `level_textures.h`):

| Offset | Type | Name | Unit |
|---|---|---|---|
| 0x0 | s16 | `texture_width` | pixels; 0 = the class has no billboard |
| 0x2 | s16 | `texture_height` | pixels |
| 0x4 | s16 | `maximum_mipmap_level` | level count/index |
| 0x6 | s16 | `palette_offset` | **0x100-byte blocks into the gs_ram lump** |
| 0x8 | s16 | `texture_offset` | **0x100-byte blocks into the gs_ram lump** |
| 0xa | s16 | `mipmap_1_offset` | presumed same unit |
| 0xc | s16 | `mipmap_2_offset` | presumed same unit |
| 0xe | s16 | `mipmap_3_offset` | presumed same unit |

Units are established for the two fields Wrench actually uses: it reads `texture_width * texture_height` pixel bytes at `gs_ram + texture_offset * 0x100` and 256 `u32`s at `gs_ram + palette_offset * 0x100` (`unpack_shrub_billboard_texture`). Billboard pixel data therefore lives in the **gs_ram lump**, unlike ordinary level textures. Same format otherwise: 8-bit indexed, unswizzled for RAC1, CSM1 palette, alpha ×2.

Wrench never reads `maximum_mipmap_level` or the three mip offsets, and its shrub packer does not write `ShrubBillboardInfo` back at all (`pack_shrub_classes` in `src/wrenchbuild/level/level_classes.cpp` zero-initialises the entry and only fills `o_class`, `offset_in_asset_wad` and `textures[16]`) — so billboard textures survive unpacking but are lost on rebuild in this version. Treat the mip-offset units as "presumed identical to `texture_offset`", not verified.

Do not confuse this with the *other* billboard structure, `ShrubBillboard`, which lives inside the shrub class geometry in core_data (`src/engine/shrub.h:37`): `f32 fade_distance; f32 width; f32 height; f32 z_ofs;` followed by three 16-byte GIF A+D entries setting `TEX1_1`, `TEX0_1` and `MIPTBP1_1`. When Wrench builds one it puts a LOD-k value derived from the fade distance into `TEX1`, `data_hi = 4`, and `TEX0.data_lo = 1` (`src/engine/shrub.cpp:469-478`), implying the billboard's texture is referenced as material slot 1 of the class.

---

## 10. How Wrench exports these as PNG (for cross-checking)

All of the following is from `src/core/png.cpp` (`write_png`) and the call sites in `level_textures.cpp` / `texture_asset.cpp`.

* Output is an **8-bit indexed PNG**: `PNG_COLOR_TYPE_PALETTE`, bit depth 8, non-interlaced, with a `PLTE` chunk of up to 256 RGB triples and a `tRNS` chunk carrying the per-entry alpha. Indices are written verbatim — the file keeps the original palette indices, and the swizzle/alpha work happens on the palette only.
* **No vertical or horizontal flipping.** Row `y` of the PNG is bytes `[y*width, (y+1)*width)` of the texture data.
* **Colour conversion:** palette entry bytes R,G,B go straight into `PLTE`; the 4th byte goes through the alpha ×2 rule of §4.4 into `tRNS`.
* **Palette order:** CSM1 → linear is applied before writing, so the exported PNG's palette is linear. Re-importing applies the same involution.
* **Deduplication is a pack-time-only concept.** The unpacker writes one PNG per material slot of every class (`unpack_level_material` writes `<slot>.png` into the material's directory), so a texture shared by several classes is exported repeatedly. The packer then re-merges: `deduplicate_level_textures` groups records whose width, height, format, index data *and* palette all compare equal and points duplicates at the lowest-indexed representative (`out_edge`), while `deduplicate_level_palettes` separately merges identical palettes (`palette_out_edge`). `write_level_texture_table` additionally emits at most one `TextureEntry` per unique texture per table, so two classes sharing a texture share a table index.
* PNG import (`read_png`) accepts RGB, RGBA, grayscale and 1/2/4/8-bit palette PNGs; 1-, 2- and 8-bit palettes all become 8-bit indexed, and missing `tRNS` entries default to opaque.

---

## 11. Global (non-level) 8bpp textures: the PIF format

Help images, planets, mission/options screenshots, item images, the debug font, sketches, renders, epilogue images and so on in the RAC1 global WAD are standalone **PIF** files, unpacked with the hint `pif,8,1,unswizzled` (`FMT_TEXTURE_PIF8` in `src/assetmgr/asset_dispatch.h:40`; call sites in `src/wrenchbuild/globals/global_wad.cpp`, `unpack_rac_global_wad`). Most are additionally WAD-compressed inside the WAD (`unpack_compressed_assets`), a few — `help_controls`, the debug font — are stored raw.

Header (0x20 bytes) — `PifHeader` in `src/wrenchbuild/common/texture_asset.cpp`, also in `docs/textures.md`:

| Offset | Type | Name | Notes |
|---|---|---|---|
| 0x00 | char[4] | `magic` | ASCII `"2FIP"` (i.e. "PIF2" byte-reversed) |
| 0x04 | s32 | `file_size` | purpose unclear; Wrench zeroes it during round-trip tests because it appears unused |
| 0x08 | s32 | `width` | pixels; sanity-checked ≤ 2048 |
| 0x0c | s32 | `height` | pixels; ≤ 2048 |
| 0x10 | s32 | `format` | 0x13 = 8-bit indexed with 256-entry palette; 0x94 = 4-bit indexed with 16-entry palette |
| 0x14 | s32 | `clut_format` | not interpreted by Wrench |
| 0x18 | s32 | `clut_order` | not interpreted by Wrench |
| 0x1c | s32 | `mip_levels` | number of mip images that follow |

File layout: header, then the palette, then the mip images from **largest to smallest** (each level half the width and height of the previous). For format 0x13 the palette is 256 × 4 bytes RGBA and mip 0 is `width * height` bytes; for 0x94 the palette is 16 × 4 bytes and mip 0 is `width * height / 2` bytes (two 4-bit indices per byte, **high nibble first** — `Texture::to_rgba`, `PALETTED_4` case, takes `>> 4` for even x).

Processing on load: for 0x13 the palette gets the §4.3 CSM1 un-swizzle and the §4.4 alpha ×2; for 0x94 only the alpha scaling (no palette swizzle). Pixel data is only un-swizzled if the hint says `swizzled`, which no RAC1 global asset uses — so RAC1 PIF pixels are linear row-major. Wrench's unpacker reads **only the largest mip and discards the rest**; its packer regenerates mips by point-sampling (every 2nd pixel per axis, stopping when width < 8) and reuses the base palette for every level, writing `mip_levels` accordingly.

Two related global formats for completeness:

* **Texture lists** (hint `texlist,pif,8,1,unswizzled`, `FMT_COLLECTION_PIF8`): `s32 count` followed by `s32 offsets[count]`, each offset pointing at a PIF within the same blob; an element's size is the next offset minus its own (last runs to end of file). Used for RAC1 `space_plates` (`global_wad.cpp:89`) and, in later games, bot/landstalker/dropship textures and level transition textures (`src/wrenchbuild/common/collection_asset.cpp`, `unpack_texture_list`).
* **Raw RGBA** (`FMT_TEXTURE_RGBA`): a 0x10-byte header `s32 width; s32 height; u32 pad[2];` then `width*height*4` RGBA bytes. RAC1's credits images instead use `rawrgba` with dimensions baked into the hint and no header at all: 512×416 for NTSC, 512×448 for PAL (`FMT_TEXTURE_RGBA_512_416` / `_512_448`, used at `global_wad.cpp` for `credits_images_ntsc` / `credits_images_pal`). Alpha in all RGBA variants is also 0-0x80 and gets the same ×2 treatment.

---

## 12. Recipe: decoding one RAC1 level texture end to end

1. Read `LevelCoreHeader` at `core_index + 0`.
2. Decompress `core_data`. Let `T = textures_base_offset`.
3. Pick a `TextureEntry` — either directly from `tfrag_textures[i]`, or via `class.textures[slot]` as an index into `moby_textures` / `tie_textures` / `shrub_textures` (stop at 0xff).
4. Read `width * height` bytes at `core_data[T + data_offset]`. These are palette indices, row-major, top-down, unswizzled.
5. Read 1024 bytes at `gs_ram[palette * 0x100]` as 256 RGBA quadruples.
6. Un-swizzle the palette: `linear[i] = raw[(((i>>3)&1) != ((i>>4)&1)) ? (i ^ 0x18) : i]`.
7. Scale alpha: `a < 0x80 ? a*2 : 255`.
8. Look up each index in the linear palette.

For a shrub billboard, replace steps 3-5 with `ShrubBillboardInfo` and read the pixels from `gs_ram[texture_offset * 0x100]`, the palette from `gs_ram[palette_offset * 0x100]`. For particles/FX, read both pixels and palette from `core_data[part_bank_offset + …]` / `core_data[fx_bank_offset + …]`.

---

## 12b. Verified on the retail NTSC-U disc (2026-09-26)

* The retired C++ `rc_extract textures` decoded 9104 level textures across all 19 levels with the recipe in §12 (8-bit linear indices, CSM1 palette permutation, alpha ×2); spot-checked PNGs are correct (e.g. Novalis tfrag 0 is a clean riveted metal panel).
* `TextureEntry.type` values observed on the retail disc: 4 (7375 textures), 3 (1012), 1 (623), 2 (11). So Wrench's "write 3 for everything" is a simplification; the enumeration is still to be recovered from the game code (likely the GS texture-function / mip / clamp selection set up when the entry is bound).
* Shrub billboards decode from the gs_ram lump with the 0x100-block units as described in §9.

### Rust loader

`crates/rc-formats/src/texture.rs` (`parse_textures`, `decode_indexed8`, `clut_index`, `scale_alpha`) is a port of the retired C++ reference's texture decoder plus the table walk of its `rc_extract textures`. Pixel data is bounded by the `textures` block of the core-data boundary rule (`LevelCore::block(data, "textures")`). The retired `rc_extract textures` also wrote `textures/rgba.bin` (uncompressed RGBA per texture); the golden test `textures_for_every_level` checked all 9104 textures of the 19 levels byte-for-byte against it and now checks them against the committed snapshot hashes. No disagreement with this document was found.

## 13. Unknowns

Things Wrench's source does **not** answer, and which a real disc must settle:

1. **`TextureEntry.type` enumeration.** Wrench only writes 0 (stashed) and 3 (normal) and never reads the field. Real RAC1 values and their meaning (GS texture function? CLUT load mode? wrap/decal?) are unknown.
2. **Mipmap layout in retail data.** Wrench writes exactly one quarter-scale (`w/4 × h/4`) mip per texture and never reads mips back. Whether retail RAC1 stores one quarter-scale level, a chain (1/2, 1/4, 1/8) contiguously at `mipmap * 0x100`, or something addressed via `MIPTBP` is unverified. Likewise whether the mip for a given texture is guaranteed to have its own `GsRamEntry`.
3. **`GsRamEntry.address` semantics in retail data.** Wrench sets `address == offset == ` byte offset in the lump (except stash entries). Whether retail files put a true GS block/word address there — and whether the game adds a fixed GS base — is not established. Also unknown: whether retail palette entries carry real `width`/`height` (Wrench writes 0/0).
4. **psm 0x01 (`RGBA16`) palettes.** Declared in the enum but no code produces or consumes them; whether RAC1 ever uses 16-bit CLUTs, and if so their bit layout and whether the CSM1 swizzle differs, is unknown.
5. **`chrome_map_texture`/`chrome_map_palette`/`glass_map_texture`/`glass_map_palette`.** Units and target lump unknown; Wrench hard-codes `0x4000` and `0x400` for glass and leaves chrome zero. `0x4000` is inconsistent with a pure 256-byte-block GS address (that would be past the end of 4 MB GS RAM).
6. **`ShrubBillboardInfo.maximum_mipmap_level` and `mipmap_1..3_offset`.** Never read or written by Wrench; the 0x100-block unit for the mip offsets is assumed by analogy with `texture_offset`. Whether `maximum_mipmap_level` is a count or a highest-index is unknown.
7. **`ParticleTextureEntry.unknown_4`** and **`PartDefsHeader.unknown_4`**: always written as 0, meaning unknown. Also unverified: whether non-square particle textures can occur (Wrench refuses them).
8. **RAC1 FX texture slot names/ordering.** Wrench has name tables for GC/UYA/DL only.
9. **Alpha values above 0x80.** In UYA/DL they drive bloom and reflectivity; whether RAC1 uses them is unknown, and Wrench's PNG round-trip destroys them either way (everything ≥ 0x80 becomes 0xff becomes 0x80).
10. **PIF `clut_format` and `clut_order`** header fields, and `file_size` (Wrench considers it unused and zeroes it during tests). Also: what PIF `format` values other than 0x13/0x94 exist.
11. **Whether retail RAC1 4-bit (PSMT4) level textures exist.** The level-core path is unconditionally 8-bit; only PIF supports 4-bit, and 4-bit palette swizzling is explicitly marked as not figured out in `texture_asset.cpp` and `Texture::reswizzle`.
12. **Exact alignment guarantees in retail data** (Wrench assumes/imposes 0x100 for texture data, palettes and mips, 0x10 for tables, 0x40 for geometry blocks).
13. **`TextureEntry.pad`** (0xe): Wrench writes -1; whether retail uses it is unknown.
14. **`LevelCoreHeader.unknown_74`, `unknown_a0`**, and the 0x30-byte `EndOfRacLevelCoreHeader` blob declared in `level_core.h` but unused — possibly texture-related, unverified.

### Source map

| Fact area | File |
|---|---|
| Lump layout, header struct, RAC1 gadget textures | `src/wrenchbuild/level/level_core.h`, `src/wrenchbuild/level/level_core.cpp`, `src/wrenchbuild/level/level_data_wad.cpp` |
| `TextureEntry`, `GsRamEntry`, `ParticleTextureEntry`, `FxTextureEntry`, `ShrubBillboardInfo`, psm constants, dedup, particle/FX/billboard unpacking, gs_ram layout | `src/wrenchbuild/level/level_textures.h`, `src/wrenchbuild/level/level_textures.cpp` |
| Class `textures[16]` traversal and packing | `src/wrenchbuild/level/level_classes.cpp`, `src/wrenchbuild/level/level_core.h` |
| 8-bit/4-bit pixel handling, CSM1 palette permutation, alpha ×2 and ÷2, DL-only pixel swizzle, mip reduction | `src/core/texture.h`, `src/core/texture.cpp` |
| PNG output/input, palette + tRNS, no flipping | `src/core/png.cpp` |
| PIF format, RGBA/rawrgba formats, format hints | `src/wrenchbuild/common/texture_asset.cpp`, `src/assetmgr/asset_dispatch.h` |
| Texture lists | `src/wrenchbuild/common/collection_asset.cpp` |
| RAC1 global WAD asset list and formats | `src/wrenchbuild/globals/global_wad.cpp` |
| TEX0 material-slot references, moby chrome/glass/none constants, shrub billboard GIF data | `src/engine/moby_high.cpp`, `src/engine/moby_packet.h`, `src/engine/shrub.h`, `src/engine/shrub.cpp`, `src/engine/tie.h`, `src/engine/tfrag_high.cpp`, `src/engine/tfrag_low.h`, `src/engine/gif.h` |
| Material/diffuse asset model, `stash_textures` doc | `src/assetmgr/material_asset.cpp`, `src/assetmgr/asset_schema.wtf`, `docs/asset_reference.md` |
| PIF layout + palette swizzle prose, bloom/reflectivity note | `docs/textures.md` |
| Level core block ordering per game | `docs/level_core.md` |

---

Notes for the caller, outside the document: (a) Wrench's `pack_shrub_classes` does not write `ShrubBillboardInfo` back, so its shrub-billboard understanding is read-only and less trustworthy than the rest; (b) `src/wrenchbuild/level/level_core.cpp` carries an author comment saying the texture packing code needs to be redone, which is worth keeping in mind for any fact derived only from the pack path (notably the single quarter-scale mipmap and `GsRamEntry.address == offset`); (c) the RAC1-specific code paths are noticeably thinner than the GC/UYA/DL ones — there is no RAC1 FX name table and no RAC1 GS stash support — so RAC1 specifics in Wrench are less exercised than later games'.
