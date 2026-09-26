# Occlusion data (R&C1, SCUS_971.99)

Precomputed potentially-visible sets for tfrags, tie instances and mobys. Three pieces of data, all read
from the game code (level01 overlay addresses; boot ELF equivalents in brackets). Code:
`crates/rc-formats/src/occlusion.rs`, `src/core/occlusion.{h,cpp}`, `rc_extract occlusion --level N`.
Per-frame rule and renderer plan: `docs/plan/occlusion_culling.md`.

Wrench (`src/engine/occlusion.cpp`, `docs/occlusion_culling.md`) was used for orientation only. Its
picture of the grid (4×4×4 octants, 128-byte masks, u16 tree) agrees with the game code. Its "mapping"
semantics do not match the game: in RAC1 the tfrag `occlusion_id` is **not** the tfrag's `occl_index`. See §3.

## 1. Grid block (core header 0x0c)

The loader `FUN_00258128` sets `0x15f600 = core_data + header.occlusion`, or 0 when the word is 0
[boot `0x15f640`]. No size is stored. Its extent comes from the core-index boundary rule
(`level.rs`), and it ends in 0–48 bytes of padding to 0x40.

| Offset | Type | Field | Notes |
|---|---|---|---|
| 0x00 | s32 | `masks_offset` | Byte offset of the mask array from the block start |
| 0x04 | u16 | `z_base` | First z cell |
| 0x06 | u16 | `z_count` | Number of z slots |
| 0x08 | u16 × z_count | z slots | Offset of the y node **in 4-byte units** from the block start; 0 = empty |
| y node | u16, u16, u16 × n | `y_base`, `y_count`, y slots | Offset of the x node in 4-byte units; 0 = empty |
| x node | u16, u16, u16 × n | `x_base`, `x_count`, x slots | **Mask index**; 0xffff = empty (0 is a valid mask) |
| `masks_offset` | 0x80 × n | masks | 1024-bit visibility masks; bit `b` = `mask[b>>3] & 1<<(b&7)` |

The lookup is `ParseOcclGrid(x, y, z)` at 0x218e78 [0x1f2690]. For each axis, in z, y, x order, it computes
`c - base`. It rejects the lookup unless `0 ≤ c - base < count`, rejects an empty slot, and returns
`block + masks_offset + mask*0x80`, or 0 for no cell. All bases and counts are **unsigned**, so no cell
has a negative coordinate. No mask count is stored. The number of masks is the highest index any cell
names plus 1. In every retail level that equals `(size - masks_offset) / 0x80`, and every mask is used by
at least one cell. Several cells can share one mask.

**Cell mapping** (`BuildOcclVisibility` 0x219008 [0x1f2820]): `cell = cvt.w.s(camera * 0.25)` per axis
(`truncate_float_to_s32` 0x222160 is a bare `cvt.w.s`). So cells are 4 world units on a side, and cell
`c` covers `[4c, 4c+4)` for `c ≥ 1`. Truncation toward zero makes cell 0 cover `(-4, 4)`. The camera
is `0x167240` (world units, the same vector `TfragProc` scales by 1024).

## 2. Octant override (core header 0xa8, `occlusion_oct_offset`)

`0x15f604 = core_data + header[0xa8]`, or 0 when the word is 0. It is present in levels 11, 13 and 17
only. The block is 0x410 bytes:

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32 × 4 | centre x, y, z, 0.0 |
| 0x10 | 0x80 × 8 | masks, index `(cam.x > cx)·4 + (cam.y > cy)·2 + (cam.z > cz)` (strict `0.0 < cam − c`) |

It is used only when the camera is outside the grid and the per-frame fallback is 2 (see the plan doc).
Header words 0xb0 (`occlusion_rad_offset`) and 0xb8 (`occlusion_rad2_offset`) are 0xa000 and 0xb400 in
every level. No occlusion code in level01 reads them.

## 3. Mappings (gameplay pointer 0x8c)

The level WAD's `occlusion` lump is a sector-padded byte copy of this section, which the golden test
checks.

| Offset | Type | Field |
|---|---|---|
| 0x00 | s32 × 3 | `tfrag_count`, `tie_count`, `moby_count` |
| 0x0c | s32 | pad |
| 0x10 | 8 × total | `{s32 bit_index; s32 occlusion_id}`: tfrag records, then ties, then mobys |

Bit indices run 0..1021 in retail, and several records share one bit. At load, `FUN_00255958` (right after
the moby loader) turns every record into a u16 **occlusion word**, `(bit >> 3) << 8 | 1 << (bit & 7)`. It
stores that word in the object:

| Kind | Stored at | Match rule (verified in the decompile of `FUN_00255958`) |
|---|---|---|
| tfrag | tfrag header **0x3a** (u16; the byte at 0x3b is the mask byte) | Positional: record `i` goes to tfrag `i`. The loader accepts it only if `tfrag_count` equals the level's tfrag count and **every tfrag header byte 0x3d (`tri_count`) equals `occlusion_id`** (a staleness check). Otherwise every tfrag gets 0x7f80 and the loader prints "occlusion out of date on tfrag". |
| tie | runtime tie +0x18 (the loader writes `(s16)` instance `occlusion_index` there first) | Positional when the counts are equal and every `(s16)` key is equal. Otherwise, per tie, the first record whose `occlusion_id` (u32) equals the u16 key. No match gives 0x7f80. |
| moby | runtime moby +0x36 | Only mobys whose gameplay `occlusion` word (instance 0x5c) is **0** are matched (the others keep 0x7f80). The key is the first record whose `occlusion_id` equals `(s16)` spawn id (instance 0x0c, moby +0xb2). No match gives 0x7f80. Mobys spawned later keep the `InitMobyInstance` default 0x7f80. |

`0x7f80` is bit 1023. The per-frame builder always sets it, so it means "always visible". With no grid
or no mappings, the loader prints "no occlusion", sets every object to 0x7f80 and sets the occlusion mode
`0x16c4f4` to 0 (off). Otherwise it sets the mode to 2 (active).

The on-disc tfrag header u16 at 0x3a is **0 in every tfrag of every level**. It is a runtime slot, and
Wrench's reading of it as an occlusion index does not apply to retail RAC1 data.

**Index spaces.** Tfrags use the header-table order of the core tfrag block. Ties use the gameplay tie
instance order, which the loader also uses for the runtime array (`0x160fc0`, 0x20 bytes each). Mobys use
the static gameplay instance order. Shrubs have no occlusion data.

## 4. Retail results (all 19 levels, from `rc_extract occlusion`)

The load-time checks pass everywhere: no level is out of date for tfrags, ties take the positional
path, and every moby with `occlusion == 0` finds its record. The mapping moby count equals the number of
such mobys.

Totals: 230,109 cells and 139,110 masks. Summed over all cells, 54,079,750 tfrag, 120,817,137 tie and
135,614,377 moby visible entries (mobys include the always-visible ones).

Some mapped objects are never visible from any cell, so they are never drawn while the camera is inside
the grid: 83 tfrags, 1435 ties and 22 mobys. Per level these are tfrags 0–19, ties 0–702 (level 03 has
702) and mobys 0–11. Why they are never visible is not checked; hidden inside geometry is the likely reason [inferred].

| Lvl | cells | masks | x | y | z | tfrag/cell min–max | tie/cell min–max |
|---|---|---|---|---|---|---|---|
| 00 | 3251 | 3250 | 22–51 | 15–79 | 5–11 | 0–303 | 3–804 |
| 01 | 6039 | 2822 | 5–80 | 11–74 | 7–25 | 5–522 | 0–952 |
| 03 | 16445 | 10638 | 21–97 | 14–115 | 7–23 | 12–673 | 50–2397 |
| 09 | 23817 | 22615 | 10–117 | 22–126 | 2–13 | 0–258 | 1–1108 |
| 11 | 26726 | 15888 | 91–169 | 90–170 | 32–65 | 2–526 | 5–489 |

(Every level's line is in `rc_extract occlusion` output.)

## 5. Golden dump (`extracted/levels/NN/occlusion_dump.bin`)

The dump starts with `"RCOC"`. Each section is a u32 length followed by the bytes. In order:

1. The raw block, `masks_offset`, and `z_base | z_count << 16`.
2. The cells (u16 x, y, z, mask).
3. The mask count and the octant block.
4. The three mapping counts and the records.
5. The resolved u16 words (per tfrag, tie and moby).
6. The resolution report.
7. Per mask and per kind, the visible count and an FNV-1a hash of the visible indices (u32 LE), using the
   frame mask (the stored mask with bit 1023 set).

Full per-cell lists would be about 250 MB. The hash plus the exact per-object words and masks determine
them.

## 6. Unknown

* `occl_index_stash` (tfrag header 0x2b) and the rad offsets (header 0xb0/0xb8). No level01 occlusion code
  touches them.
