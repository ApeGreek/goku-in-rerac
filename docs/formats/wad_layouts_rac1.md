# RAC1 (Ratchet & Clank, 2002, PS2) — WAD container formats

Scope: **container layouts only**. Mesh, texture pixel, collision and sound-bank internals are out of scope. Derived from reading the Wrench source tree (reference only — no code copied). Wrench calls this game `Game::RAC`, build-config string `"rac"`, disc header file name `rc1.hdr`.

All integers are **little-endian**. `s32`/`u32` = 32-bit signed/unsigned, `s16`/`u16` = 16-bit, `f32` = IEEE-754 single. Structures are byte-packed with no implicit padding.

---

## 0. Primitives and conventions

### 0.1 Sector

One sector is **0x800 bytes**. All "sector" quantities below are in units of 0x800 bytes.

### 0.2 Range types

| Name | Size | Layout | Meaning |
|---|---|---|---|
| `Sector32` | 4 | `s32 sectors` | A sector number/count. `<= 0` means "absent". |
| `SectorRange` | 8 | `s32 offset_sectors; s32 size_sectors` | Position **and size both in sectors**. Size is rounded up to a whole sector, so real payload size is ≤ `size_sectors * 0x800`. Empty when `size_sectors <= 0`. |
| `SectorByteRange` | 8 | `s32 offset_sectors; s32 size_bytes` | Position in sectors, **exact size in bytes**. Empty when `size_bytes <= 0`. |
| `ByteRange` | 8 | `s32 offset; s32 size` | Both in bytes. Empty when `size <= 0`. Absent entries are often written as `offset = -1, size = 0`. |
| `ArrayRange` | 8 | `s32 count; s32 offset` | **Count comes first**, then a byte offset. Used for tables inside the level core index. |

"Sector-based lump" below means `SectorRange`/`SectorByteRange`/`Sector32`; "byte-based lump" means `ByteRange`.

### 0.3 WAD (LZ) compression container

Many lumps are individually compressed with Insomniac's LZ variant. The container header is 0x10 bytes:

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0x00 | 3 | magic | ASCII `"WAD"` |
| 0x03 | 4 | `s32 compressed_size` | Total size **including** this 0x10-byte header |
| 0x07 | 9 | tag | A 9-character ASCII tag ("muffin"), NOT padding. Insomniac and Wrench both put a short identifier here, e.g. `coredata`, `gameplay`, `hud_bank`, `chnktfrag`, `chunkcoll`, `gadget`, `transition`. Useful for sanity checks but not load-bearing. |
| 0x10 | … | packet stream | LZ packets |

Decoding: read `compressed_size`, treat bytes `[0x10, compressed_size)` as a packet stream. Each packet begins with a flag byte. The full packet table is documented in Wrench's `docs/file_loading.md` ("WAD Compression"). Two implementation-relevant quirks:

* Match/literal packets never cross a 0x2000-byte boundary in the *compressed* stream (a scratchpad-buffering artefact). Insomniac's compressor emits a `12 00 00` packet followed by `0xEE` filler until `(pos - header_pos) % 0x2000 == 0x10`.
* Two literal packets never occur back to back; a dummy `11 00 00` packet separates them.

Because a compressed lump's true length is self-describing (`compressed_size`), sector-based ranges pointing at compressed lumps can be over-sized without harm. Wrench uses this to determine lump extents when the header only gives a start sector (see §1.4).

### 0.4 Determining size when only a start sector is given

Two file types (level audio, level scene) reference payloads by start sector only. Sizes are recovered from the payload itself:

* **VAG audio**: header begins with ASCII `"VAGp"`; the 32-bit `data_size` at offset 0x0C is **big-endian**; total size = `0x30 + byte_swap(data_size)`, rounded up to sectors. If the magic does not match, assume 1 sector.
* **WAD-compressed blob**: read the `"WAD"` header, take `compressed_size`, round up to sectors. If the magic does not match, assume 1 sector.

---

## 1. Disc layout and the global (non-level) WADs

### 1.1 Boot path

RAC1 has a normal ISO 9660 filesystem containing only `SYSTEM.CNF`, the boot ELF, and a handful of extras. Everything else is read by raw sector I/O. Volume identifier is `RATCHETANDCLANK` padded to 32 characters.

* `SYSTEM.CNF` is at a **hardcoded LBA 289**.
* The table of contents is at a **hardcoded LBA 1500** (byte offset `1500 * 0x800 = 0x2EE000`). Wrench names this file `rc1.hdr` when rebuilding, but retail discs do not list it in the filesystem.

### 1.2 The list of global WADs in RAC1

**RAC1 has exactly one global WAD.** This is the single most important structural difference from R&C2/3/Deadlocked, which split globals into separate `MPEG.WAD`, `MISC.WAD`, `HUD.WAD`, `BONUS.WAD`, `AUDIO.WAD`, `SPACE.WAD`, `SCENE.WAD`, `GADGET.WAD`, `ARMOR.WAD`, `ONLINE.WAD` files.

In RAC1 the "global WAD" **is** the table of contents itself: one big C struct at LBA 1500 whose fields are sector ranges pointing directly at all global assets scattered across the disc. Wrench identifies RAC1 WAD types purely by the `header_size` field:

| WAD type | Header size | Notes |
|---|---|---|
| global | `0x2960` | The ToC struct at LBA 1500 |
| level | `0x0030` | Wrench-synthesised per-level header (see §2.2) |
| level audio | `0x0164` | Wrench-synthesised (see §1.5) |
| level scene | `0x22b8` | Wrench-synthesised (see §1.6) |

So: there is **no** separate mpeg/misc/hud/bonus/space/scene/gadget/armor/audio global WAD in RAC1. The content that those files hold in later games is present here as field groups inside the one 0x2960-byte struct — MPEG videos, HUD banks, bonus/goodies images, space-combat assets, help audio, music, IRX modules and so on all hang off this one header.

### 1.3 Global WAD header layout (0x2960 bytes, at LBA 1500)

All entries are **sector-based**. Sector numbers on disc are **absolute LBAs**. (When Wrench extracts the ToC to a standalone file it rewrites them to be relative to the file start; a from-disc extractor should just use them as absolute.)

Wrench's names are shown; where a name is a placeholder the "Meaning" column says so.

| Offset | Type | Count | Name | Meaning / content |
|---|---|---|---|---|
| 0x0000 | `s32` | 1 | version | Must equal **1**. Acts as the magic. |
| 0x0004 | `s32` | 1 | header_size | Must equal **0x2960**. |
| 0x0008 | `SectorRange` | 1 | debug_font | Paletted 8-bit texture (PIF8) |
| 0x0010 | `SectorRange` | 1 | save_game | Save-game template blob |
| 0x0018 | `SectorRange` | 28 | ratchet_seqs | WAD-compressed Ratchet animation sequences |
| 0x00f8 | `SectorRange` | 20 | hud_seqs | WAD-compressed HUD animation sequences |
| 0x0198 | `SectorRange` | 1 | vendor | Vendor (weapon shop) data |
| 0x01a0 | `SectorRange` | 37 | vendor_audio | VAG audio |
| 0x02c8 | `SectorRange` | 12 | help_controls | PIF8 textures, **uncompressed** |
| 0x0328 | `SectorRange` | 15 | help_moves | WAD-compressed PIF8 textures |
| 0x03a0 | `SectorRange` | 15 | help_weapons | WAD-compressed PIF8 |
| 0x0418 | `SectorRange` | 14 | help_gadgets | WAD-compressed PIF8 |
| 0x0488 | `SectorRange` | 7 | help_ss | WAD-compressed PIF8 (help screenshots) |
| 0x04c0 | `SectorRange` | 7 | options_ss | WAD-compressed PIF8 (options screenshots) |
| 0x04f8 | `SectorRange` | 1 | frontbin | Front-end code overlay, stored in the Ratchet overlay format (§4) |
| 0x0500 | `SectorRange` | 81 | mission_ss | WAD-compressed PIF8 (mission screenshots) |
| 0x0788 | `SectorRange` | 19 | planets | WAD-compressed PIF8 (planet images; 19 = level count) |
| 0x0820 | `SectorRange` | 38 | stuff2 | **Unknown.** WAD-compressed binaries. |
| 0x0950 | `SectorRange` | 10 | goodies_images | WAD-compressed PIF8 |
| 0x09a0 | `SectorRange` | 19 | character_sketches | WAD-compressed PIF8 |
| 0x0a38 | `SectorRange` | 19 | character_renders | WAD-compressed PIF8 |
| 0x0ad0 | `SectorRange` | 31 | skill_images | WAD-compressed PIF8 (skill points) |
| 0x0bc8 | `SectorRange` | 12 | epilogue_english | WAD-compressed PIF8 |
| 0x0c28 | `SectorRange` | 12 | epilogue_french | " |
| 0x0c88 | `SectorRange` | 12 | epilogue_italian | " |
| 0x0ce8 | `SectorRange` | 12 | epilogue_german | " |
| 0x0d48 | `SectorRange` | 12 | epilogue_spanish | " |
| 0x0da8 | `SectorRange` | 30 | sketchbook | WAD-compressed PIF8 |
| 0x0e98 | `SectorRange` | 4 | commercials | WAD-compressed PIF8 |
| 0x0eb8 | `SectorRange` | 9 | item_images | WAD-compressed PIF8 |
| 0x0f00 | `Sector32` | 240 | qwark_boss_audio | **Start sector only** — VAG audio, size from VAG header (§0.4) |
| 0x12c0 | `SectorRange` | 1 | irx | WAD-compressed IOP module bundle. Internal layout **unknown** (§6) |
| 0x12c8 | `SectorRange` | 4 | spaceships | Binary |
| 0x12e8 | `SectorRange` | 20 | anim_looking_thing_2 | **Unknown**; name is a placeholder. Binary, looks animation-like. |
| 0x1388 | `SectorRange` | 6 | space_plates | WAD-compressed PIF8 texture *lists* (collections) |
| 0x13b8 | `SectorRange` | 1 | transition | WAD-compressed; level-transition assets |
| 0x13c0 | `SectorRange` | 36 | space_audio | VAG audio |
| 0x14e0 | `SectorRange` | 1 | sound_bank | Global 989snd sound bank |
| **0x14e8** | `SectorRange` | 1 | wad_14e0 | **Unknown.** ⚠ Wrench's comment says `0x14e0`; the real offset is **0x14e8** (its comment is a copy-paste error). |
| **0x14f0** | `SectorRange` | 1 | music | VAG music. ⚠ Wrench's comment says `0x14e0`; real offset is **0x14f0**. |
| 0x14f8 | `SectorRange` | 1 | hud_header | HUD bank index/header |
| 0x1500 | `SectorRange` | 5 | hud_banks | HUD banks, **uncompressed** here (unlike in the level data WAD) |
| 0x1528 | `SectorRange` | 1 | all_text | All UI/localisation text |
| 0x1530 | `SectorRange` | 28 | things | **Unknown**; name is a placeholder. Binary. |
| 0x1610 | `SectorRange` | 1 | post_credits_helpdesk_girl_seq | WAD-compressed animation sequence |
| 0x1618 | `SectorRange` | 18 | post_credits_audio | VAG audio |
| 0x16a8 | `SectorRange` | 20 | credits_images_ntsc | Raw RGBA, fixed 512 × 416 |
| 0x1748 | `SectorRange` | 20 | credits_images_pal | Raw RGBA, fixed 512 × 448 |
| 0x17e8 | `SectorRange` | 2 | wad_things | **Unknown.** WAD-compressed binaries. |
| 0x17f8 | `SectorByteRange` | 88 | mpegs | PSS video streams (exact byte sizes) |
| 0x1ab8 | `Sector32` | 900 | help_audio | **Start sector only** — VAG audio, size from VAG header |
| 0x28c8 | `SectorRange` | 19 | levels | Level table: `offset` = absolute LBA of that level's amalgamated header (§2.1). |
| 0x2960 | — | — | *end* | |

Note on the `levels` table: when Wrench rebuilds an ISO it writes `size_sectors = 1` for each entry even though the amalgamated header is 0x2434 bytes (5 sectors). The game evidently ignores the size field here. **Treat `levels[i].size` as meaningless; only use `levels[i].offset`.**

Rebuilding also relies on a "flat" reinterpretation of the same 0x2960 bytes for bulk sector-number relocation, which incidentally confirms the field grouping: `s32 version; s32 header_size; SectorRange[479]; Sector32[240]; SectorRange[167]; SectorByteRange[88]; Sector32[900]; SectorRange[19]`.

### 1.4 Locating the level headers

Wrench finds level headers by scanning the ToC in 8-byte steps from offset 8, reading each 4-byte value as a candidate sector number, seeking there, and accepting it if the 32-bit value at candidate+4 equals `0x2434`. An independent extractor should just read the 19 entries at 0x28c8 and validate `header_size == 0x2434`.

### 1.5 Per-level audio WAD header (Wrench-synthesised, 0x164 bytes)

RAC1 does **not** store a separate audio WAD header on disc — audio sector numbers live in the amalgamated level header (§2.1). Wrench synthesises this header so that RAC1 looks like R&C2+. If you extract from a disc you don't need it, but the field grouping tells you what the amalgamated header's audio fields mean.

| Offset | Type | Count | Name | Meaning |
|---|---|---|---|---|
| 0x000 | `s32` | 1 | header_size | `0x164` |
| 0x004 | `s32` | 1 | pad | Zero. (In R&C2+ headers this slot holds the file's absolute LBA.) |
| 0x008 | `SectorByteRange` | 36 | bindata | Per-level VAG audio ("bin data") |
| 0x128 | `Sector32` | 15 | music | **Start sector only** — VAG music, size from VAG header |
| 0x164 | — | — | *end* | |

Wrench does **not** implement unpacking/packing of the RAC1 level audio WAD — the handlers are empty stubs. The header layout is known; the semantic grouping of the 36 bindata slots is not documented.

### 1.6 Per-level scene WAD header (Wrench-synthesised, 0x22b8 bytes)

| Offset | Type | Count | Name | Meaning |
|---|---|---|---|---|
| 0x000 | `s32` | 1 | header_size | `0x22b8` |
| 0x004 | `s32` | 1 | pad | Zero |
| 0x008 | `RacSceneHeader` | 30 | scenes | 30 cutscene slots, 0x128 bytes each |
| 0x22b8 | — | — | *end* | |

`RacSceneHeader` (0x128 bytes):

| Offset | Type | Count | Name | Meaning |
|---|---|---|---|---|
| 0x000 | `Sector32` | 6 | sounds | Start sectors of VAG speech/audio streams; size from VAG header |
| 0x018 | `Sector32` | 68 | wads | Start sectors of WAD-compressed scene chunks; size from LZ header |
| 0x128 | — | — | *end* | |

Wrench also does **not** implement RAC1 scene WAD unpack/pack (empty stubs). The meaning of the 6 sound slots and 68 wad slots is not documented; by analogy with Deadlocked they are probably per-language speech channels plus streamed animation/geometry chunks, but that is inference, not established.

---

## 2. The RAC1 level WAD

### 2.1 What is actually on the disc: the amalgamated level header (0x2434 bytes)

RAC1 stores **one header per level** covering what later games split across three files. It sits at the LBA named by `globals.levels[i].offset`, and all sector numbers inside are **absolute LBAs** — including ones pointing at sectors *before* the header itself.

| Offset | Type | Count | Name | Meaning |
|---|---|---|---|---|
| 0x000 | `s32` | 1 | id | Level ID (the number the engine uses at runtime) |
| 0x004 | `s32` | 1 | header_size | `0x2434` — use as the type check |
| 0x008 | `SectorRange` | 1 | data | The **level data WAD** (§2.3). Uncompressed container. |
| 0x010 | `SectorRange` | 1 | gameplay_ntsc | WAD-compressed gameplay/instances file, NTSC variant (§3) |
| 0x018 | `SectorRange` | 1 | gameplay_pal | WAD-compressed gameplay/instances file, PAL variant |
| 0x020 | `SectorRange` | 1 | occlusion | Occlusion grid, **uncompressed** (§2.7) |
| 0x028 | `SectorByteRange` | 36 | bindata | Per-level VAG audio |
| 0x148 | `Sector32` | 15 | music | Start sectors of VAG music |
| 0x184 | `RacSceneHeader` | 30 | scenes | 30 × 0x128 = 0x22b0 bytes of cutscene tables (§1.6) |
| 0x2434 | — | — | *end* | |

The gameplay NTSC/PAL split exists because help-message text differs by region; both are complete gameplay files. Wrench writes the same data to both slots when repacking.

### 2.2 Wrench's rewritten level WAD header (0x30 bytes)

When Wrench extracts a level it splits the amalgamated header into three, converts absolute LBAs to offsets relative to the start of each extracted file, and prepends the rewritten header at file offset 0. The level part looks like this:

| Offset | Type | Name | Meaning |
|---|---|---|---|
| 0x00 | `s32` | header_size | `0x30` |
| 0x04 | `s32` | unused_4 | Always 0 in RAC1. (In R&C2+ this is the file's absolute LBA.) |
| 0x08 | `s32` | id | Level ID |
| 0x0c | `s32` | unused_c | Always 0 in RAC1. (In R&C2+ this is `reverb`.) |
| 0x10 | `SectorRange` | data | Level data WAD |
| 0x18 | `SectorRange` | gameplay_ntsc | WAD-compressed gameplay file |
| 0x20 | `SectorRange` | gameplay_pal | WAD-compressed gameplay file |
| 0x28 | `SectorRange` | occlusion | Uncompressed occlusion grid |
| 0x30 | — | *end* | |

How the file boundary is chosen: Wrench takes the minimum start sector over {data, gameplay_ntsc, gameplay_pal, occlusion}, subtracts the header size in sectors (1), and uses that as the file base LBA; the file end is the maximum end sector over the same four ranges. So the extracted file is `[min_start - 1, max_end)` with the 0x30-byte header living in the first sector and zero padding up to `min_start`.

Notice what is **absent** compared to R&C2/3/DL level headers: no `reverb`, no top-level `sound_bank` range, no `chunks[3]`/`chunk_banks[3]`, no `art_instances`, no `missions`. RAC1's sound bank is *inside* the data WAD, and RAC1 has no chunk streaming.

### 2.3 The level data WAD (inner container), byte-based

`amalgamated.data` / `RacLevelWadHeader.data` points at an **uncompressed** container whose own header is 0x58 bytes. **All offsets in this header are byte offsets relative to the start of the data WAD** (i.e. relative to `data.offset * 0x800`).

| Offset | Type | Count | Name | Compressed? | Content |
|---|---|---|---|---|---|
| 0x00 | `ByteRange` | 1 | overlay | No | Level code overlay in Ratchet overlay format (§4) |
| 0x08 | `ByteRange` | 1 | sound_bank | No | The level's main 989snd sound bank |
| 0x10 | `ByteRange` | 1 | core_index | No | Level core **index** — the `LevelCoreHeader` and all its sub-tables (§2.4) |
| 0x18 | `ByteRange` | 1 | gs_ram | No | Raw GS (graphics-synthesizer) memory image: texture pixel data and palettes (§5.1) |
| 0x20 | `ByteRange` | 1 | hud_header | No | HUD bank index for this level |
| 0x28 | `ByteRange` | 5 | hud_banks[5] | **Yes** (each individually WAD-compressed) | HUD graphics banks |
| 0x50 | `ByteRange` | 1 | core_data | **Yes** (one single WAD stream) | Level core **data**: tfrags, collision, sky, occlusion grid copy, class geometry, texture pixels, ratchet seqs, gadgets |
| 0x58 | — | — | *end* | | |

Absent/empty lumps are written as `offset = -1, size = 0`. When repacking, Wrench aligns each lump to 0x40 bytes.

Comparison for orientation (not needed for RAC1): R&C2/3 use a 0x58-byte data header with no `sound_bank` and an extra `transition_textures` at 0x50; Deadlocked uses 0x70 bytes with `moby8355_pvars`, `art_instances`, `gameplay_core`, `global_nav_data`.

### 2.4 Level core index — `LevelCoreHeader` (0xbc bytes)

This sits at byte 0 of the `core_index` lump. It is the master index for the level's art.

**Two distinct address spaces.** Getting these right is the single most error-prone part of walking a RAC1 level:

* **Index-relative offsets** — relative to the start of the `core_index` lump (= to this header). These are: `gs_ram.offset`, `moby_classes.offset`, `tie_classes.offset`, `shrub_classes.offset`, `tfrag_textures.offset`, `moby_textures.offset`, `tie_textures.offset`, `shrub_textures.offset`, `part_textures.offset`, `fx_textures.offset`, `part_defs_offset`, `sound_remap_offset`, `ratchet_seqs_rac123`, `gadget_offset_rac1`.
* **Data-relative offsets** — relative to the start of the **decompressed** `core_data` blob. These are: `tfrags`, `occlusion`, `sky`, `collision`, `textures_base_offset`, `part_bank_offset`, `fx_bank_offset`, `scene_view_size`, every `offset_in_asset_wad` in the class tables, every non-zero ratchet-seq offset, and every gadget `offset_in_asset_wad`.

| Offset | Type | Name | Address space | Meaning |
|---|---|---|---|---|
| 0x00 | `ArrayRange` | gs_ram | index | Table of `GsRamEntry` (§5.1) describing the GS memory image. `count` then `offset`. |
| 0x08 | `s32` | tfrags | data | Start of the tfrag (terrain fragment) block |
| 0x0c | `s32` | occlusion | data | Start of the occlusion grid copy inside core data; 0 if absent |
| 0x10 | `s32` | sky | data | Start of the sky block; 0 if absent |
| 0x14 | `s32` | collision | data | Start of the level collision block |
| 0x18 | `ArrayRange` | moby_classes | index | Moby class table (§5.2) |
| 0x20 | `ArrayRange` | tie_classes | index | Tie class table (§5.3) |
| 0x28 | `ArrayRange` | shrub_classes | index | Shrub class table (§5.4) |
| 0x30 | `ArrayRange` | tfrag_textures | index | `TextureEntry` table for tfrags (§5.5) |
| 0x38 | `ArrayRange` | moby_textures | index | `TextureEntry` table for mobies |
| 0x40 | `ArrayRange` | tie_textures | index | `TextureEntry` table for ties |
| 0x48 | `ArrayRange` | shrub_textures | index | `TextureEntry` table for shrubs |
| 0x50 | `ArrayRange` | part_textures | index | `ParticleTextureEntry` table (§5.6) |
| 0x58 | `ArrayRange` | fx_textures | index | `FxTextureEntry` table (§5.7) |
| 0x60 | `s32` | textures_base_offset | data | Base of shared texture pixel data inside core data. All `TextureEntry.data_offset` values are relative to **this**. |
| 0x64 | `s32` | part_bank_offset | data | Base of the particle texture bank |
| 0x68 | `s32` | fx_bank_offset | data | Base of the FX texture bank |
| 0x6c | `s32` | part_defs_offset | index | Particle definition blob (§5.6) |
| 0x70 | `s32` | sound_remap_offset | index | Sound remap table (§5.8) |
| 0x74 | `s32` | unknown_74 | ? | **Unknown.** |
| 0x78 | `s32` | ratchet_seqs | index | Offset of a fixed array of **256 `s32`** data-relative offsets to Ratchet animation sequences. 0 = section absent; individual 0 entries = that slot unused. |
| 0x7c | `s32` | scene_view_size | data | A data-space watermark; Wrench records it after writing all class geometry. Exact runtime meaning **unknown**. |
| 0x80 | `s32` | gadget_count | — | Number of gadget entries (RAC1-only field; other games use this slot differently) |
| 0x84 | `s32` | gadget_offset | index | Offset of the gadget table, `gadget_count` × `RacGadgetHeader` (§5.9) |
| 0x88 | `s32` | assets_compressed_size | — | Size of the compressed `core_data` stream |
| 0x8c | `s32` | assets_decompressed_size | — | Size of `core_data` after decompression. Doubles as the final data-space block boundary. |
| 0x90 | `s32` | chrome_map_texture | ? | Chrome/environment map texture reference. **Semantics unknown.** |
| 0x94 | `s32` | chrome_map_palette | ? | **Unknown** |
| 0x98 | `s32` | glass_map_texture | ? | Glass map texture. Wrench hardcodes `0x4000` when packing. **Semantics unknown.** |
| 0x9c | `s32` | glass_map_palette | ? | Wrench hardcodes `0x400`. **Unknown** |
| 0xa0 | `s32` | unknown_a0 | data? | **Unknown.** Wrench has commented-out code that would read 0x40 bytes at this data-space offset. |
| 0xa4 | `s32` | heightmap_offset | ? | Heightmap. **Unread by Wrench; layout unknown.** |
| 0xa8 | `s32` | occlusion_oct_offset | ? | Occlusion octant data. **Unread; unknown.** |
| 0xac | `s32` | moby_gs_stash_list | index | Not used in RAC1 (R&C2+ only). Expect 0. |
| 0xb0 | `s32` | occlusion_rad_offset | ? | **Unread; unknown.** |
| 0xb4 | `s32` | moby_sound_remap_offset | index | Deadlocked-only in practice; if non-zero in RAC1 it participates in block-boundary computation but Wrench does not parse it. |
| 0xb8 | `s32` | occlusion_rad2_offset | ? | **Unread; unknown.** |
| 0xbc | — | *end* | | |

Note: RAC1 core index data is followed by 0x30 bytes that Wrench models as an opaque `EndOfRacLevelCoreHeader` blob. Its meaning is **unknown**.

### 2.5 How to size the blocks inside `core_data`

Blocks in the decompressed core data have **no explicit sizes** — only start offsets. The size of a block is `next_boundary - start`, where `next_boundary` is the smallest known offset strictly greater than `start`. Build the boundary set from:

1. `tfrags`, `occlusion`, `sky`, `collision`
2. `textures_base_offset`
3. `assets_decompressed_size` (the end sentinel)
4. every `offset_in_asset_wad` in the moby, tie and shrub class tables
5. every non-zero entry in the 256-slot ratchet-seq offset array
6. every gadget `offset_in_asset_wad`
7. `moby_sound_remap_offset` if non-zero

Special case for the tfrag block, which starts the data region: its size is taken as the first non-zero of `occlusion`, `sky`, `collision` (in that order). If all three are zero the level is malformed.

A `start` of 0 means "absent"; treat the block as empty rather than computing a size.

### 2.6 Level "chunks" in RAC1

**RAC1 does not support chunk streaming.** There is exactly one implicit chunk, chunk 0, and its contents are the core data's `tfrags` and `collision` blocks. There is no chunk sound bank.

For contrast, R&C2/3/DL level headers carry a `ChunkWadHeader`:

| Offset | Type | Count | Meaning |
|---|---|---|---|
| 0x00 | `SectorRange` | 3 | chunk data regions |
| 0x18 | `SectorRange` | 3 | per-chunk 989snd sound banks |

…where each chunk region begins with an 8-byte header `{ s32 tfrags; s32 collision; }` holding offsets **relative to the chunk region start**, each pointing at a WAD-compressed blob. **None of this exists in RAC1** — do not look for it.

### 2.7 Occlusion

Two copies of occlusion data exist:

* `RacLevelWadHeader.occlusion` (sector-based, top level) — the occlusion **grid**, stored **uncompressed**, copied verbatim.
* `LevelCoreHeader.occlusion` (data-relative) — a copy inside the core data.

Conceptually (per Wrench's `docs/occlusion_culling.md`): playable space is divided into 4×4×4 octants; each octant references a 128-byte visibility bit mask; masks are shared between octants; a tree maps position → mask. The gameplay file's occlusion-mappings section (§3, pointer 0x8c) maps tfrag/tie/moby occlusion indices to bit indices. Shrubs are excluded from the system. Internal grid/tree byte layout is **out of scope here and only partially documented in Wrench**.

### 2.8 Summary: which lumps are WAD-compressed

| Lump | Compressed? |
|---|---|
| Level data WAD container itself | No |
| `overlay` | No |
| `sound_bank` (level) | No |
| `core_index` | No |
| `gs_ram` | No |
| `hud_header` | No |
| `hud_banks[0..4]` | **Yes**, each separately |
| `core_data` | **Yes**, one stream (tag `coredata`) |
| `gameplay_ntsc`, `gameplay_pal` | **Yes** (tag `gameplay`) |
| `occlusion` (top level) | No |
| Gadget moby classes (inside decompressed core data) | **Yes**, each separately (tag `gadget`) |
| Moby / tie / shrub class geometry (inside core data) | No (already inside the compressed core data) |
| tfrags / collision / sky / occlusion copy (inside core data) | No |
| Global WAD: most textures, sequences, `irx`, `transition`, `space_plates`, `stuff2`, `wad_things` | **Yes** |
| Global WAD: `help_controls`, `hud_banks`, `credits_images_*`, VAG audio, MPEGs, `all_text`, `save_game`, `debug_font`, `vendor`, `sound_bank`, `music`, `frontbin` | No |

---

## 3. The RAC1 gameplay (instances) file

### 3.1 Container

`gameplay_ntsc` / `gameplay_pal` point at a single **WAD-compressed** blob. Decompress it; everything below is relative to the start of the decompressed buffer.

### 3.2 Section pointer table

The file starts with a table of **37 `s32` pointers** — a 0x94-byte header. Each pointer is a **byte offset from the start of the decompressed file** to the start of that section. **A pointer of 0 means the section is absent.**

| Offset | Section | Notes |
|---|---|---|
| 0x00 | level settings | §3.3 |
| 0x04 | directional lights | table of `0x40`-byte records |
| 0x08 | cameras | table of `0x20`-byte records |
| 0x0c | sound instances | table of `0x90`-byte records |
| 0x10 | help messages — US English | §3.4 |
| 0x14 | help messages — UK English | |
| 0x18 | help messages — French | |
| 0x1c | help messages — German | |
| 0x20 | help messages — Spanish | |
| 0x24 | help messages — Italian | |
| 0x28 | help messages — Japanese | |
| 0x2c | help messages — Korean | different string encoding |
| 0x30 | tie classes | `s32 count` then `count` × `s32` class numbers |
| 0x34 | tie instances | table of `0xe0`-byte records |
| 0x38 | shrub classes | `s32 count` then `count` × `s32` class numbers |
| 0x3c | shrub instances | table of `0x70`-byte records |
| 0x40 | moby classes | `s32 count` then `count` × `s32` class numbers |
| 0x44 | moby instances | §3.5 |
| 0x48 | moby groups | §3.6 |
| 0x4c | shared data (global pvars) | §3.7 |
| 0x50 | pvar moby-link fixup table | §3.8 |
| 0x54 | pvar table | §3.8 |
| 0x58 | pvar data | §3.8 |
| 0x5c | pvar relative-pointer fixup table | §3.8 |
| 0x60 | cuboids | table of `0x80`-byte shape records |
| 0x64 | spheres | same record |
| 0x68 | cylinders | same record |
| 0x6c | pills | same record |
| 0x70 | paths (splines) | §3.9 |
| 0x74 | grind paths | §3.9 |
| 0x78 | point light grid | §3.10 |
| 0x7c | point lights | table of `0x20`-byte records |
| 0x80 | env transitions | §3.11 |
| 0x84 | camera collision grid | §3.10 |
| 0x88 | env sample points | table of `0x30`-byte records |
| 0x8c | occlusion mappings | §3.12 |
| 0x90 | (pad / unused) | Wrench never reads or writes it |

Two structural differences from R&C2/3: RAC1 has **no** tie-groups, shrub-groups, tie-ambient-RGBA or areas sections, and RAC1 uniquely has a **point light grid** at 0x78. Its pointer ordering also differs from every other game, so the offsets above are not transferable.

Sections are 0x10-aligned in the file (except help messages, which are not padded, and occlusion mappings, which are 0x40-aligned when present).

### 3.3 Level settings (RAC1 variant, 0x50 bytes)

Only the "first part" exists in RAC1 — no chunk planes, no core-sounds count, no later-game tail sections.

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32 r, g, b` | background colour (0..255 per channel; `r == -1` means "no colour") |
| 0x0c | `s32 r, g, b` | fog colour (same -1 convention) |
| 0x18 | `f32` | fog near distance |
| 0x1c | `f32` | fog far distance |
| 0x20 | `f32` | fog near intensity |
| 0x24 | `f32` | fog far intensity |
| 0x28 | `f32` | death height (Y below which the player dies) |
| 0x2c | `f32 × 3` | ship position (x, y, z) |
| 0x38 | `f32` | ship rotation about Z |
| 0x3c | `s32` | ship path index (into the paths section) |
| 0x40 | `s32` | ship camera cuboid, start index |
| 0x44 | `s32` | ship camera cuboid, end index |
| 0x48 | `u32 × 2` | padding, zero |
| 0x50 | | *end* |

(For contrast, R&C2+ insert `is_spherical_world` + `sphere_centre` before the ship fields, making the first part 0x5c bytes, and then append chunk-plane records and more.)

### 3.4 Help messages

Header: `s32 count; s32 size`. In RAC1 `size` is the **total section size in bytes including the 8-byte header** — so the whole section is `[0, size)`. (In UYA/DL, `size` excludes the header, so `size + 8` is used; RAC1 and R&C2 do not need that adjustment.)

Followed by `count` × 0x10-byte entries:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `s32` | offset of the NUL-terminated string, relative to the start of the section (0 = no string) |
| 0x4 | `s16` | id |
| 0x6 | `s16` | short id |
| 0x8 | `s16` | third-person id |
| 0xa | `s16` | co-op id |
| 0xc | `s16` | vag (voice clip index) |
| 0xe | `s16` | character |

### 3.5 Moby instances

Section header (0x10 bytes): `s32 static_count; s32 spawnable_moby_count; s32 pad[2];`

Then `static_count` × `RacMobyInstance` (**0x78 bytes**) starting at section offset 0x10. Instance IDs are the array indices.

| Offset | Type | Field | Notes |
|---|---|---|---|
| 0x00 | `s32` | size | Always `0x78`; validate it |
| 0x04 | `s32` | unknown_4 | **Unknown** |
| 0x08 | `s32` | unknown_8 | **Unknown** |
| 0x0c | `s32` | unknown_c | **Unknown** |
| 0x10 | `s32` | unknown_10 | **Unknown** (in R&C2+ the equivalent region holds `mission`, `uid`, `bolts`) |
| 0x14 | `s32` | unknown_14 | **Unknown** |
| 0x18 | `s32` | o_class | Moby class number |
| 0x1c | `f32` | scale | |
| 0x20 | `f32` | draw_distance | **`f32` in RAC1** (it is `s32` in R&C2+) |
| 0x24 | `s32` | update_distance | |
| 0x28 | `s32` | unused_28 | Wrench writes constant `32` |
| 0x2c | `s32` | unused_2c | Wrench writes constant `64` |
| 0x30 | `f32 × 3` | position | |
| 0x3c | `f32 × 3` | rotation | Euler, radians |
| 0x48 | `s32` | group | Index into moby groups, or `-1` |
| 0x4c | `s32` | is_rooted | |
| 0x50 | `f32` | rooted_distance | |
| 0x54 | `s32` | unknown_54 | **Unknown** |
| 0x58 | `s32` | pvar_index | Index into the pvar table, or `-1` |
| 0x5c | `s32` | occlusion | `0` = precompute occlusion for this instance |
| 0x60 | `s32` | mode_bits | Bitfield; individual bit meanings **not documented by Wrench** |
| 0x64 | `s32 r, g, b` | colour | 0..255 per channel, stored as 32-bit |
| 0x70 | `s32` | light | Directional-light index |
| 0x74 | `s32` | unknown_74 | **Unknown** |
| 0x78 | | *end* | |

`spawnable_moby_count` is the count of dynamically spawnable mobies reserved by the engine; it is not the length of this array.

### 3.6 Groups (moby groups)

Header (0x10 bytes): `s32 group_count; s32 data_size; s32 pad[2];`

Then `group_count` × `s32` pointers at section offset 0x10. Each pointer is a **byte offset into the member array** (so member index = pointer / 2); a negative pointer means an empty group. After the pointer array, round up to the next 0x10 boundary; the member array is `data_size / 2` × `u16` member indices. Each group's member list runs from its start index until a terminator/next group (Wrench walks `u16` members from the start index).

### 3.7 Shared data (global pvars)

Header (0x10 bytes): `s32 data_size; s32 pointer_count; s32 unused[2];`

Then `data_size` bytes of shared pvar data at section offset 0x10, followed by `pointer_count` × 8-byte entries:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `u16` | pvar index |
| 0x2 | `u16` | pointer offset within that pvar |
| 0x4 | `s32` | offset into the shared data block |

### 3.8 Pvars (per-instance variables)

Four cooperating sections:

* **pvar table** (0x54) — an array of `{ s32 offset; s32 size; }`, 8 bytes per entry. There is **no count in the file**: the entry count is `max(pvar_index) + 1` over all moby instances, cameras and sound instances. You must parse those three sections first.
* **pvar data** (0x58) — a flat byte blob; each pvar occupies `[offset, offset + size)`. Total size = `max(offset + size)` over the table.
* **pvar moby-link fixup table** (0x50) — array of `{ s32 pvar_index; u32 offset; }`, terminated by an entry whose `pvar_index < 0`. Marks 4-byte fields inside pvar data that hold moby references needing relocation.
* **pvar relative-pointer fixup table** (0x5c) — identical record layout and terminator. Marks fields holding pointers relative to the pvar's own base.

Wrench writes two `s32 -1` values as the terminator for both fixup tables.

### 3.9 Paths and grind paths

Both use the same 0x10-byte header: `s32 spline_count; s32 data_offset; s32 data_size; s32 pad;` where `data_offset` is **relative to the start of the section header**.

* **Paths** (0x70): at section offset 0x10, `spline_count` × `s32` relative spline offsets; the spline data itself lives at `data_offset`, each spline being a count-prefixed run of `f32 × 4` control points.
* **Grind paths** (0x74): at section offset 0x10, `spline_count` × `GrindPathData` (0x20 bytes each), then the `s32` relative-offset array, then the spline data.

`GrindPathData` (0x20 bytes):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 4` | bounding sphere (x, y, z, radius) |
| 0x10 | `s32` | unknown_4 — **Unknown** |
| 0x14 | `s32` | wrap (closed loop flag) |
| 0x18 | `s32` | inactive |
| 0x1c | `s32` | pad |

### 3.10 Spatial grids (camera collision grid, point light grid)

Both share a shape: a 0x10-byte header (contents not parsed by Wrench), then a **0x40 × 0x40 = 4096-entry `s32` grid** at section offset 0x10, indexed `y * 0x40 + x`. A grid entry of 0 means the cell is empty; otherwise it is a byte offset **relative to section offset 0x10** to a list. Each list starts with `s32 prim_count`.

* **Camera collision grid** (0x84): after the count, skip to `list + 0x10` and read `prim_count` × `CamCollGridPrim` (0x30 bytes):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 4` | bounding sphere |
| 0x10 | `s32` | volume type: 3 = cuboid, 5 = sphere, 6 = cylinder, 7 = pill |
| 0x14 | `s32` | index into the corresponding shape section |
| 0x18 | `s32` | flags |
| 0x1c | `s32` | integer value (per-type meaning **unknown**) |
| 0x20 | `f32` | float value (per-type meaning **unknown**) |
| 0x24 | `s32 × 3` | pad |

* **Point light grid** (0x78, RAC1-only): after `s32 prim_count` at `list`, the list is `prim_count` × `s32` point-light indices starting at `list + 4`. Wrench regenerates this grid from point-light positions and radii rather than round-tripping it verbatim, so the exact original packing rule is **not fully pinned down**.

### 3.11 Environment transitions

Header: the standard 0x10-byte table header (`s32 count; s32 pad[3];`). Then at section offset 0x10 there are `count` × `f32 × 4` (0x10-byte) records — Wrench skips these without interpreting them (**unknown**; probably bounding spheres) — and only then `count` × `EnvTransitionPacked` (0x80 bytes):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 16` | inverse matrix (4 rows of vec4) |
| 0x40 | `u8 r,g,b,pad` | hero colour 1 |
| 0x44 | `u8 r,g,b,pad` | hero colour 2 |
| 0x48 | `s32` | hero light 1 |
| 0x4c | `s32` | hero light 2 |
| 0x50 | `u32` | flags (bit meanings **unknown**) |
| 0x54 | `u8 r,g,b,pad` | fog colour 1 |
| 0x58 | `u8 r,g,b,pad` | fog colour 2 |
| 0x5c | `f32` | fog near dist 1 |
| 0x60 | `f32` | fog near intensity 1 |
| 0x64 | `f32` | fog far dist 1 |
| 0x68 | `f32` | fog far intensity 1 |
| 0x6c | `f32` | fog near dist 2 |
| 0x70 | `f32` | fog near intensity 2 |
| 0x74 | `f32` | fog far dist 2 |
| 0x78 | `f32` | fog far intensity 2 |
| 0x7c | `s32` | unused |

### 3.12 Occlusion mappings

Header (0x10 bytes): `s32 tfrag_mapping_count; s32 tie_mapping_count; s32 moby_mapping_count; s32 pad;`

Total section size = `0x10 + (tfrag + tie + moby) * 8`. Each mapping is an 8-byte pair; Wrench treats the body as opaque bytes, so the pair's internal field split is **not documented** (it is an occlusion-index → bit-index mapping).

### 3.13 Other RAC1 record layouts

All of these are preceded by the standard 0x10-byte table header `s32 count; s32 pad[3];` with the array starting at section offset 0x10. Instance IDs are array indices.

**Camera** (0x20 bytes, section 0x08):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32` | type (used as the camera class) |
| 0x04 | `f32 × 3` | position |
| 0x10 | `f32 × 3` | rotation |
| 0x1c | `s32` | pvar index |

**Sound instance** (0x90 bytes, section 0x0c):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s16` | o_class |
| 0x02 | `s16` | m_class |
| 0x04 | `u32` | update function pointer (runtime value; zero on disc / when repacking) |
| 0x08 | `s32` | pvar index |
| 0x0c | `f32` | range |
| 0x10 | `f32 × 16` | matrix |
| 0x50 | `f32 × 12` | inverse matrix (3 rows of vec4) |
| 0x80 | `f32 × 3` | rotation |
| 0x8c | `f32` | pad |

**Directional light** (0x40 bytes, section 0x04): `f32 × 4` colour A, `f32 × 4` direction A, `f32 × 4` colour B, `f32 × 4` direction B.

**Point light** (0x20 bytes, section 0x7c) — RAC1 uses full-precision floats, unlike the 16-bit-packed R&C2/3 form:

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 3` | position |
| 0x0c | `f32` | radius |
| 0x10 | `u8 r,g,b,pad` | colour |
| 0x14 | `u32 × 3` | unused |

**Shape** — cuboid / sphere / cylinder / pill, all identical, 0x80 bytes (sections 0x60/0x64/0x68/0x6c):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 16` | matrix |
| 0x40 | `f32 × 12` | inverse matrix |
| 0x70 | `f32 × 3` | rotation |
| 0x7c | `f32` | unused |

**Tie instance** — RAC1 variant, **0xe0 bytes** (section 0x34):

⚠ Wrench's struct comments are wrong here: it labels both `ambient_rgbas` and `directional_lights` as offset `0x50`. The real packing, which is what `sizeof == 0xe0` implies, is:

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32` | o_class |
| 0x04 | `s32` | draw distance |
| 0x08 | `s32` | pad |
| 0x0c | `s32` | occlusion index |
| 0x10 | `f32 × 16` | matrix |
| 0x50 | `u8 × 0x80` | ambient RGBAs (per-vertex ambient lighting, inline) |
| **0xd0** | `s32` | directional lights |
| **0xd4** | `s32` | uid |
| **0xd8** | `s32` | pad |
| **0xdc** | `s32` | pad |
| 0xe0 | | *end* |

In R&C2/3/DL the tie instance is only 0x60 bytes and the ambient RGBAs live in a separate "tie ambient rgbas" section. RAC1 inlines them — a real format difference, not just a naming one.

**Shrub instance** (0x70 bytes, section 0x3c):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32` | o_class |
| 0x04 | `f32` | draw distance |
| 0x08 | `s32` | unused |
| 0x0c | `s32` | unused |
| 0x10 | `f32 × 16` | matrix |
| 0x50 | `s32 r, g, b` | colour |
| 0x5c | `s32` | unused |
| 0x60 | `s32` | directional lights |
| 0x64 | `s32 × 3` | unused |

**Env sample point** — RAC1 variant, 0x30 bytes (section 0x88):

| Offset | Type | Field |
|---|---|---|
| 0x00 | `f32 × 3` | position |
| 0x0c | `f32` | constant 1.0 (purpose **unknown**) |
| 0x10 | `s32 r, g, b` | hero colour |
| 0x1c | `s32` | hero light |
| 0x20 | `s32` | reverb depth |
| 0x24 | `u8` | reverb type |
| 0x25 | `u8` | reverb delay |
| 0x26 | `u8` | reverb feedback |
| 0x27 | `u8` | enable reverb params |
| 0x28 | `s32` | music track |
| 0x2c | `s32` | unused |

Note RAC1's env sample point has **no fog colour**, unlike the R&C2+ variant.

### 3.14 Ordering note for byte-identical repacking

Sections are *written* in a fixed order that is **not** the pointer-table order. In particular env sample points come first, help messages are unpadded, and occlusion mappings are 0x40-aligned. An extractor doesn't care, but a repacker that wants byte-identical output must reproduce the order: env sample points, level settings, the eight help-message blocks, directional lights, env transitions, cameras, sound instances, moby classes, moby instances, pvar table, pvar data, moby-link fixups, relative-pointer fixups, moby groups, shared data, tie classes, tie instances, shrub classes, shrub instances, paths, cuboids, spheres, cylinders, pills, camera collision grid, point lights, point light grid, grind paths, occlusion mappings.

---

## 4. The level code overlay

### 4.1 Storage

The overlay is stored **uncompressed** in the level data WAD at `RacLevelDataHeader.overlay`. It is *not* an ELF. It is Insomniac's own minimal "Ratchet executable" format: a bare sequence of `[16-byte section header][section data]` pairs with no file header, no magic, and no section count.

Section header (0x10 bytes):

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x00 | `s32` | dest_address | EE main-memory address to copy this section's data to |
| 0x04 | `s32` | copy_size | Section data size in bytes, excluding this header |
| 0x08 | `s32` | section_type | ELF section type value (e.g. `SHT_PROGBITS` = 1, `SHT_NOBITS` = 8). Recorded but unused by the game. |
| 0x0c | `s32` | entry_point | Address of the level's `startlevel` function — **repeated identically in every section header** |

### 4.2 Walking it

Start at offset 0. Read a header, note `entry_point` from the first one, then read `copy_size` bytes of data and advance. Termination: the loop ends when a header's `entry_point` differs from the first section's `entry_point`. The retail game reads slightly out of bounds at the end doing exactly this; a safe implementation should additionally stop at end-of-lump and sanity-check `copy_size`.

Constraints for writing: `dest_address` must be 4-byte aligned and `copy_size` must be a multiple of 4.

### 4.3 Load address and entry point

There is no single load address: **each section carries its own `dest_address`**, and the loader copies the data there. The entry point is the shared `entry_point` field — the address of `startlevel`.

### 4.4 How Wrench turns it into an ELF

The overlay has no section names, flags, alignments or program headers, so they cannot be recovered from the file. Wrench reconstructs them from a hardcoded **donor table**, matching by count and section type:

1. Prepend a null section (index 0), per ELF convention.
2. For each overlay section, create an ELF section with `sh_addr = dest_address`, `sh_size = sh_type = copy_size / section_type`, `sh_addralign = 1`, everything else zero. Name it provisionally `.unknown_N`.
3. Overlay the donor table for RAC/GC/UYA level overlays, which supplies names, flags, alignments and one `PT_LOAD` segment. Section order and expected types:

| Index | Name | Type | Flags | Align |
|---|---|---|---|---|
| 0 | (null) | `SHT_NULL` | 0 | 0 |
| 1 | `.lit` | `SHT_PROGBITS` | W+A+MIPS_GPREL | 64 |
| 2 | `.bss` | `SHT_NOBITS` | W+A+MIPS_GPREL | 64 |
| 3 | `.data` | `SHT_PROGBITS` | W+A | 64 |
| 4 | `lvl.vtbl` | `SHT_PROGBITS` | A | 1 |
| 5 | `lvl.camvtbl` | `SHT_PROGBITS` | A | 1 |
| 6 | `lvl.sndvtbl` | `SHT_PROGBITS` | A | 1 |
| 7 | `.text` | `SHT_PROGBITS` | A+X | 64 |

Plus one program header: `PT_LOAD`, flags R+W+X, align 0x1000.

Matching fails (and Wrench errors out) if the section count or any section type differs from the donor. That means the donor table is itself a documented assumption about RAC1 level overlays, not something derived per file.

The `lvl.vtbl` / `lvl.camvtbl` / `lvl.sndvtbl` sections hold tables of function pointers for the moby, camera and sound classes present in the level — e.g. `lvl.vtbl` holds every moby update function pointer.

Writing back out is the inverse: emit one 0x10-byte header + data per ELF section that has a non-zero `sh_addr` and non-empty data, with `entry_point` copied into every header.

For reference: the RAC1 **boot** ELF is a normal, unpacked ELF (unlike UYA/DL, whose boot ELFs are compressed overlays). The RAC1 front-end overlay `frontbin` in the global WAD uses the same Ratchet overlay format as level overlays.

---

## 5. Tables referenced by the level core header

### 5.1 GS RAM table — `GsRamEntry` (0x10 bytes each)

Located in the core **index** at `gs_ram.offset`, `gs_ram.count` entries. Describes the contents of the `gs_ram` lump (a raw GS memory image of texture pixels and palettes).

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x00 | `s32` | psm | GS pixel storage mode: `0x00` = RGBA32 palette, `0x01` = RGBA16 palette, `0x13` = IDTEX8 (8-bit indexed texture) |
| 0x04 | `s16` | width | |
| 0x06 | `s16` | height | |
| 0x08 | `s32` | address | GS memory address |
| 0x0c | `s32` | offset | Byte offset into the `gs_ram` lump |

RAC1-specific: the entry count is exactly `gs_ram.count`. R&C2/3/DL append an extra `moby_gs_stash_count` entries for classes whose textures are permanently resident in GS memory; **RAC1 has no moby texture stash**, so `moby_gs_stash_list` and `moby_gs_stash_count` are unused and the table is not extended.

### 5.2 Moby class table — `MobyClassEntry` (0x20 bytes each)

At core index offset `moby_classes.offset`, `moby_classes.count` entries.

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x00 | `s32` | offset_in_asset_wad | Data-relative offset of the class's geometry/animation blob inside decompressed core data. **0 = no geometry** (table entry exists for texture bookkeeping only). Size via the boundary algorithm (§2.5). |
| 0x04 | `s32` | o_class | Moby class number (the ID the gameplay file references) |
| 0x08 | `s32` | unknown_8 | **Unknown** |
| 0x0c | `s32` | unknown_c | **Unknown** |
| 0x10 | `u8 × 16` | textures | Up to 16 indices into the **moby** `TextureEntry` table. Semantics of the terminator/unused slots are handled positionally by Wrench; a slot's meaning beyond "index into the moby texture table" is not documented. |

RAC1 payload format note: RAC1 moby class blobs use the older ("phat") moby layout, distinct from R&C2+.

### 5.3 Tie class table — `TieClassEntry` (0x20 bytes each)

At `tie_classes.offset`, `tie_classes.count` entries. Layout is byte-for-byte identical to `MobyClassEntry`:

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32` | offset_in_asset_wad (data-relative; 0 = none) |
| 0x04 | `s32` | o_class |
| 0x08 | `s32` | unknown_8 — **Unknown** |
| 0x0c | `s32` | unknown_c — **Unknown** |
| 0x10 | `u8 × 16` | indices into the **tie** `TextureEntry` table |

### 5.4 Shrub class table — `ShrubClassEntry` (0x30 bytes each)

At `shrub_classes.offset`, `shrub_classes.count` entries. Same first 0x20 bytes as the others plus an inline billboard descriptor.

| Offset | Type | Field |
|---|---|---|
| 0x00 | `s32` | offset_in_asset_wad (data-relative) |
| 0x04 | `s32` | o_class |
| 0x08 | `s32` | pad |
| 0x0c | `s32` | pad |
| 0x10 | `u8 × 16` | indices into the **shrub** `TextureEntry` table |
| 0x20 | `ShrubBillboardInfo` | billboard (0x10 bytes, below) |

`ShrubBillboardInfo` (0x10 bytes) — the low-LOD billboard texture, addressed directly in the `gs_ram` lump rather than through a `TextureEntry`:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `s16` | texture width. **`0` means the class has no billboard** — use this as the presence test. |
| 0x2 | `s16` | texture height |
| 0x4 | `s16` | maximum mipmap level |
| 0x6 | `s16` | palette offset (in `gs_ram`) |
| 0x8 | `s16` | texture offset (in `gs_ram`) |
| 0xa | `s16` | mipmap 1 offset |
| 0xc | `s16` | mipmap 2 offset |
| 0xe | `s16` | mipmap 3 offset |

### 5.5 Texture tables — `TextureEntry` (0x10 bytes each)

Four separate tables (tfrag, moby, tie, shrub), each located via its `ArrayRange` in the core index.

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x0 | `s32` | data_offset | Offset of the pixel data, relative to `textures_base_offset` within the **decompressed core data** |
| 0x4 | `s16` | width | |
| 0x6 | `s16` | height | |
| 0x8 | `s16` | type | Texture format/type selector. Enumeration **not documented by Wrench**. |
| 0xa | `s16` | palette | Palette index/offset |
| 0xc | `s16` | mipmap | Mipmap reference; `-1` = none |
| 0xe | `s16` | pad | `-1` |

The class tables' `u8 textures[16]` arrays index into the table matching the class kind. RAC1 textures are 8-bit paletted, **unswizzled** (Deadlocked swizzles them; RAC1 does not).

### 5.6 Particle textures

* `part_textures` (`ArrayRange`, index-relative) → array of `ParticleTextureEntry`, 0x10 bytes each:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `s32` | palette |
| 0x4 | `s32` | unknown_4 — **Unknown** |
| 0x8 | `s32` | texture |
| 0xc | `s32` | side |

* `part_defs_offset` (index-relative) → a blob of particle definitions consumed alongside the entry array. Its record layout is **not broken out by Wrench**; it is round-tripped as opaque bytes.
* `part_bank_offset` (data-relative) → the pixel bank the entries index into.

### 5.7 FX textures

`fx_textures` (`ArrayRange`, index-relative) → array of `FxTextureEntry`, 0x10 bytes each:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `s32` | palette |
| 0x4 | `s32` | texture |
| 0x8 | `s32` | width |
| 0xc | `s32` | height |

`fx_bank_offset` (data-relative) is the pixel bank.

### 5.8 Sound remap table (RAC1 form)

At index offset `sound_remap_offset`. The header is 8 bytes:

| Offset | Type | Field |
|---|---|---|
| 0x0 | `s16` | second part offset (relative to the header start) |
| 0x2 | `s16` | second part size |
| 0x4 | `s16` | third part offset |
| 0x6 | `s16` | third part count |

Sizing the RAC1 (and R&C2/3) table is awkward: read the 4-byte element **immediately before** `second_part_ofs` — i.e. at `sound_remap_offset + second_part_ofs - 4` — interpreted as `{ s16 offset; s16 size; }`, and the total table size is `that.offset + that.size * 4`. (Deadlocked instead uses `third_part_ofs + third_part_count * 4`, and has a second `moby_sound_remap` table with an explicit `s32 size` at its offset 0.) Wrench treats the body as opaque bytes; the element semantics are **not documented**.

### 5.9 Gadget table — `RacGadgetHeader` (0x10 bytes each) — RAC1 only

At index offset `gadget_offset`, `gadget_count` entries. RAC1 stores the player's gadget/weapon moby classes inside the level core (later games use a dedicated global `GADGET.WAD`).

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x0 | `s32` | offset_in_asset_wad | Data-relative offset of a **WAD-compressed** moby class blob inside the decompressed core data |
| 0x4 | `s32` | class_number | Moby class number |
| 0x8 | `s32` | compressed_size | Size of the compressed blob (redundant with the LZ header's own `compressed_size`, but present) |
| 0xc | `s32` | pad | |

Each gadget's textures are resolved by scanning the **moby class table** for the entry whose `o_class` equals `class_number` and using that entry's 16 texture indices against the moby `TextureEntry` table. There must be a matching moby class entry; absence is a corrupt file.

### 5.10 Ratchet sequences

`ratchet_seqs` (index-relative) points at a fixed array of **256 `s32`** data-relative offsets. Slot index = sequence ID. `0` means unused. A header value of `0` means the whole section is absent. Sizes come from the boundary algorithm (§2.5).

---

## 6. Known unknowns

**Not implemented in Wrench at all (headers known, contents not parsed):**

* **RAC1 level audio WAD contents.** The 0x164-byte header layout is known (36 `SectorByteRange` bindata + 15 `Sector32` music) but the unpack/pack handlers are empty stubs. What each of the 36 bindata slots corresponds to is undocumented.
* **RAC1 level scene WAD contents.** The 0x22b8-byte header and 30 × `RacSceneHeader` layout are known; the unpack/pack handlers are empty stubs. The roles of the 6 `sounds` slots and 68 `wads` slots per scene are undocumented.
* **RAC1 IRX module bundle.** The global WAD's `irx` entry is decompressed and kept as an opaque binary; Wrench explicitly refuses to parse RAC1 IRX layout ("not yet implemented"). R&C2's equivalent is a `ByteRange` table (`image`, `sio2man`, `mcman`, `mcserv`, `dbcman`, `sio2d`, `ds2u`, `stash`, `libsd`, `_989snd`) — RAC1's is presumably similar but unverified.

**Global WAD fields with placeholder names / unknown content:**

* `wad_14e0` (0x14e8) — purpose unknown; the field name is literally derived from a (mistaken) offset.
* `stuff2` (0x0820, 38 compressed blobs)
* `things` (0x1530, 28 blobs)
* `wad_things` (0x17e8, 2 compressed blobs)
* `anim_looking_thing_2` (0x12e8, 20 blobs) — "looks like animation data"
* The grouping/meaning of `space_plates`, `spaceships` and `transition` contents beyond "texture lists / binary".

**Level core header fields never read by Wrench (offsets known, contents unknown):**

* `unknown_74` (0x74)
* `unknown_a0` (0xa0) — there is dead code suggesting a 0x40-byte blob at this data-space offset
* `heightmap_offset` (0xa4)
* `occlusion_oct_offset` (0xa8), `occlusion_rad_offset` (0xb0), `occlusion_rad2_offset` (0xb8) — three separate occlusion-related pointers, none parsed
* `chrome_map_texture` / `chrome_map_palette` (0x90/0x94) — passed through unchanged
* `glass_map_texture` / `glass_map_palette` (0x98/0x9c) — Wrench hardcodes `0x4000` / `0x400` when packing, which strongly suggests these are GS addresses rather than data offsets, but that is inference
* `scene_view_size` (0x7c) — a data-space watermark whose runtime use is unclear
* The 0x30 bytes following `LevelCoreHeader` in the core index (`EndOfRacLevelCoreHeader`)

**Class-table unknowns:** `unknown_8` and `unknown_c` in both `MobyClassEntry` and `TieClassEntry`.

**Gameplay-file unknowns:**

* `RacMobyInstance` fields at 0x04, 0x08, 0x0c, 0x10, 0x14, 0x54, 0x74. The 0x04–0x14 region is where R&C2+ keep `mission`, `uid` and `bolts`, so some of these are likely the RAC1 equivalents, but Wrench does not identify them.
* `mode_bits` (0x60) — individual bit meanings undocumented.
* The 0x10-byte per-record array preceding `EnvTransitionPacked` in the env-transitions section — skipped without interpretation.
* `EnvTransitionPacked.flags` (0x50) bit meanings.
* `GrindPathData.unknown_4`.
* The internal field split of occlusion-mapping 8-byte pairs.
* `CamCollGridPrim` `i_value` / `f_value` semantics per volume type.
* The point-light grid's exact original packing rule (Wrench regenerates rather than preserving it).
* Sound remap element semantics (treated as opaque).
* Particle definition blob record layout (treated as opaque).

**Structural quirks worth guarding against:**

* Wrench writes `size_sectors = 1` for every entry in the global WAD's `levels[19]` table even though each amalgamated level header is 0x2434 bytes (5 sectors). Do not trust that size.
* Three of Wrench's own struct offset comments are wrong and will mislead a transcriber: `RacWadInfo.wad_14e0` (real offset 0x14e8, comment says 0x14e0), `RacWadInfo.music` (real 0x14f0, comment says 0x14e0), and `RacTieInstance.directional_lights`/`uid`/`pad`/`pad` (real 0xd0/0xd4/0xd8/0xdc, comments say 0x50/0x54/0x58/0x5c). The `static_assert` sizes (0x2960 and 0xe0) confirm the real layouts.
* `Rgb96`'s third field is at offset 0x8, not 0xc as its comment says; the struct is 0xc bytes (confirmed by `RacLevelSettingsFirstPart` placing `fog_colour` at 0x0c).
* RAC1 moby `draw_distance` is `f32`; in every later game it is `s32`. Easy silent bug.
