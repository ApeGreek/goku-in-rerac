# Ratchet & Clank (2002, PS2) — Disc Layout and Container Formats

Target: the first game, referred to as **RAC1** (Wrench game id `rac`, enum value 1).
Later games are referred to by Wrench's ids: `gc` = R&C2 *Going Commando*,
`uya` = R&C3 *Up Your Arsenal*, `dl` = *Deadlocked / Gladiator*.

This document is an independent format specification written from behavioural
analysis of the Wrench toolchain (GPL, reference only). No Wrench code is
reproduced. Field names marked **(unknown)** are Wrench guesses with no
confirmed meaning.

All integers are **little-endian**. `s32` = signed 32-bit, `u32` = unsigned
32-bit, `u8` = byte. All structures are packed (no implicit padding).

---

## 0. Fundamental units

| Concept | Value | Notes |
| --- | --- | --- |
| Sector size | `0x800` bytes (2048) | Same as the ISO9660 logical block size. Everything on the disc is sector-aligned. |
| LBA / LSN | `s32` sector index from the start of the disc image | Byte offset = `lba * 0x800`. The disc image must be a plain 2048-byte-per-sector dump (`.iso`), **not** a 2352-byte raw dump. |

Three range primitives recur throughout the on-disc structures:

| Name | Size | Layout | Meaning |
| --- | --- | --- | --- |
| `Sector32` | 4 | `s32 sectors` | A single sector number, or a sector count. Value `<= 0` means "absent/empty". |
| `SectorRange` | 8 | `Sector32 offset; Sector32 size` | Offset and size **both in sectors**. `end = offset + size`. Empty when `size <= 0`. |
| `SectorByteRange` | 8 | `Sector32 offset; s32 size_bytes` | Offset in sectors, size in **bytes**. `end = offset + ceil(size_bytes / 0x800)`. Empty when `size_bytes <= 0`. |
| `ByteRange` | 8 | `s32 offset; s32 size` | Byte offset and byte size, relative to the start of the enclosing file. Used *inside* level data WADs, never in the disc-level tables. |

Helper used constantly: `size_in_sectors(n_bytes) = ceil(n_bytes / 0x800)`.

---

## 1. ISO9660 usage

### 1.1 What the filesystem is for

RAC1 discs carry a **conventional ISO9660 (Level 1-ish) filesystem**, but it is
nearly vestigial. Its only job is to let the PS2 BIOS find `SYSTEM.CNF` and the
boot ELF. Once the boot ELF is running, the game addresses every other asset by
**raw absolute sector number**, taken from the table of contents (section 2).
Those sectors are *not* described by any directory record.

This differs per game:

| Game | Assets also present as ISO9660 files? | ToC also a named file? |
| --- | --- | --- |
| **RAC1** | **No** — only `SYSTEM.CNF` + boot ELF (+ occasional extras) | **No** |
| R&C2 (`gc`) | Yes, retail builds list the WADs in the filesystem | Yes, as `RC2.HDR` |
| R&C3 (`uya`) | No | No |
| Deadlocked (`dl`) | No | No |

A consequence for an extractor: **you cannot enumerate RAC1 content by walking
the filesystem.** You must parse the table of contents.

### 1.2 Reading the filesystem

1. Read the **Primary Volume Descriptor (PVD)** at LBA `0x10` (byte offset
   `0x8000`), exactly one sector, `0x800` bytes.
2. Validate: byte `0x00` (`volume_descriptor_type`) must be `0x01`, and bytes
   `0x01..0x05` (`standard_identifier`) must be the ASCII `CD001`.
   A cheap "is this a PS2 R&C ISO?" test is to read 5 bytes at
   `0x10 * 0x800 + 1` and compare with `CD001`.
3. Sanity check: the root directory record's `data_length` should be
   `<= 0x10000`.
4. Recurse the root directory.

PVD field layout (the subset that matters; offsets from the start of the PVD
sector). `LsbMsb16` is `{s16 lsb; s16 msb}`, `LsbMsb32` is `{s32 lsb; s32 msb}`
— the same value stored twice, once little-endian, once big-endian. Always read
the `lsb` half.

| Offset | Size | Field |
| --- | --- | --- |
| 0x000 | 1 | `volume_descriptor_type` (== 1) |
| 0x001 | 5 | `standard_identifier` (`"CD001"`) |
| 0x006 | 1 | `volume_descriptor_version` (1) |
| 0x007 | 1 | unused |
| 0x008 | 32 | `system_identifier` (space-padded, e.g. `"PLAYSTATION"`) |
| 0x028 | 32 | `volume_identifier` (space-padded) |
| 0x048 | 8 | unused |
| 0x050 | 8 | `volume_space_size` (`LsbMsb32`) — total size of the image in sectors |
| 0x058 | 32 | unused |
| 0x078 | 4 | `volume_set_size` (`LsbMsb16`) |
| 0x07c | 4 | `volume_sequence_number` (`LsbMsb16`) |
| 0x080 | 4 | `logical_block_size` (`LsbMsb16`) — `0x800` |
| 0x084 | 8 | `path_table_size` (`LsbMsb32`) |
| 0x08c | 4 | `l_path_table` (LBA, little-endian path table) |
| 0x090 | 4 | `optional_l_path_table` |
| 0x094 | 4 | `m_path_table` |
| 0x098 | 4 | `optional_m_path_table` |
| 0x09c | 0x21 | `root_directory` — a directory record (see below) |
| 0x0bd | 1 | padding |
| 0x0be | 128 | `volume_set_identifier` |
| 0x13e | 128 | `publisher_identifier` |
| 0x1be | 128 | `data_preparer_identifier` |
| 0x23e | 128 | `application_identifier` |
| 0x2be | 38 | `copyright_file_identifier` |
| 0x2e4 | 36 | `abstract_file_identifier` |
| 0x308 | 37 | `bibliographic_file_identifier` |
| 0x32d | 17 | `volume_creation_date_time` |
| 0x33e | 17 | `volume_modification_date_time` |
| 0x34f | 17 | `volume_expiration_date_time` |
| 0x360 | 17 | `volume_effective_date_time` |
| 0x371 | 1 | `file_structure_version` (1) |
| 0x372 | 1 | unused |
| 0x373 | 512 | `application_use` |
| 0x573 | 653 | reserved |

The PVD date/time format is the ISO9660 textual one:
`char year[4], month[2], day[2], hour[2], minute[2], second[2],
hundredths[2]; s8 time_zone` = 17 bytes.

**Directory record** (0x21 = 33 bytes fixed part, then a variable-length
identifier, then optional padding up to `record_length`):

| Offset | Size | Field |
| --- | --- | --- |
| 0x00 | 1 | `record_length` — total bytes of this record including identifier and padding. `0` ⇒ end of records in this sector. |
| 0x01 | 1 | `extended_attribute_record_length` |
| 0x02 | 8 | `lba` (`LsbMsb32`) |
| 0x0a | 8 | `data_length` (`LsbMsb32`), in bytes |
| 0x12 | 7 | `recording_date_time` — `u8 years_since_1900, month, day, hour, minute, second, time_zone` |
| 0x19 | 1 | `file_flags` — bit 1 (`0x02`) set ⇒ directory |
| 0x1a | 1 | `file_unit_size` |
| 0x1b | 1 | `interleave_gap_size` |
| 0x1c | 4 | `volume_sequence_number` (`LsbMsb16`) |
| 0x20 | 1 | `identifier_length` |
| 0x21 | var | identifier (not NUL-terminated) |

Directory traversal rules that a reader should implement:

* Iterate records from the directory extent's start to `start + data_length`.
* If `record_length < 1`, advance by **1 byte** and retry (tolerates zero-filled
  tails and misalignment). Otherwise advance by `record_length` from the start
  of the record.
* The first two records of every directory are `.` and `..` (they have
  `identifier_length == 1` and the directory flag). Skip the first two records
  that have the directory flag set.
* Subdirectories: recurse into `lba * 0x800` for `data_length` bytes. Impose a
  depth limit (Wrench uses 8) and a per-directory record limit (Wrench uses
  1000) to make malformed images non-fatal.
* Files: strip a trailing `";1"` version suffix from the identifier, and
  lowercase the name for comparison purposes. Record `{name, lba, size}`.
* The entire filesystem (PVD + path tables + directory extents) lives in the
  first **1500 sectors** (`1500 * 0x800 = 0x2EE000` bytes) on RAC1, because
  sector 1500 is where the table of contents begins. A reader can safely slurp
  the first 1500 sectors into memory and parse the filesystem out of that
  buffer.

### 1.3 Files present on a RAC1 disc

| File | Where | Notes |
| --- | --- | --- |
| `SYSTEM.CNF` | Root directory, and additionally pinned at **LBA 289** | The game hardcodes this sector number; if `SYSTEM.CNF` is not at sector 289 the game picks the wrong memory-card save directory. For R&C2/3/DL the pinned sector is **1000** instead. |
| Boot ELF, e.g. `SCUS_971.99;1` | Root directory | A plain, unwrapped ELF for RAC1. Name varies by region (section 2.6). |
| (none of the asset data) | — | All asset data is referenced only by LBA from the ToC. |

Other files may exist on non-retail builds. A general extractor should copy any
filesystem file it does not otherwise understand verbatim, excluding:
`system.cnf`, the boot ELF, `*.hdr` (a ToC exposed as a file — R&C2's
`RC2.HDR`), `*.wad`, and a stub file literally named `dummy.` seen on R&C2
discs.

### 1.4 `SYSTEM.CNF`

Plain ASCII with CRLF line endings. Parsing rules:

* Find the literal substring `BOOT2 = cdrom0:\` (note the **backslash**). The
  boot ELF path starts immediately after those 16 characters and ends at the
  first `;` or `\r`.
* Find the literal substring `VER = `. The version string starts 6 characters
  later and ends at the first space or `\r`.
* `VMODE = NTSC` or `VMODE = PAL` gives the video mode. In practice
  region ⇒ video mode: EU ⇒ PAL, everything else ⇒ NTSC.

RAC1's `SYSTEM.CNF` is formatted slightly differently from the later games:
RAC1 has **no trailing space** after each value and ends with an **extra blank
CRLF line**; R&C2/3/DL put a single space before each `\r\n` and have no extra
blank line. Reconstructed RAC1 form:

```
BOOT2 = cdrom0:\SCUS_971.99;1<CR><LF>
VER = 1.00<CR><LF>
VMODE = NTSC<CR><LF>
<CR><LF>
```

### 1.5 The PS2 logo (sectors 0..11)

Sectors `0` through `11` inclusive (`12 * 0x800 = 0x6000` bytes) hold the
obfuscated PS2 boot logo, an 8-bit grayscale bitmap. Deobfuscation, per byte:

1. Take `key = byte at offset 0` of the region.
2. For every byte `b` in the region: `b ^= key`, then **rotate left by 3**
   (`b = ((b << 3) | (b >> 5)) & 0xFF`).

The resulting bitmap dimensions depend on video mode:

| Mode | Width | Height | Pixels |
| --- | --- | --- | --- |
| NTSC | 384 | 64 | 24576 |
| PAL | 344 | 71 | 24424 |

Both fit inside `0x6000` = 24576 bytes; truncate the deobfuscated buffer to
`width * height`. Re-obfuscation is the inverse: rotate **right** by 3, then XOR
with the key.

### 1.6 Rust disc reader

`crates/rc-formats/src/iso9660.rs` implements 1.2 (plain 2048-byte images and raw 2352-byte
mode 1 / mode 2 form 1 dumps, detected by the sync pattern; reads seek to the requested bytes,
the image is never read whole). `crates/rc-formats/src/disc.rs` adds the ToC (2.2-2.4), the boot
ELF located via `SYSTEM.CNF` (2.6 A, probe B as fallback) and each level's lumps: the data
container sliced by its ByteRange table (2.11), `gameplay_ntsc`/`gameplay_pal`/`occlusion` as whole
sector ranges, and the audio/scene lumps sized by the 2.5 probes. The result is byte-identical to
the Tier 0 archive's `extracted/levels/NN/*.bin` (first written by the retired C++ `rc_extract unpack`) for all 19 levels of the NTSC-U disc (a plain
2048-byte image, 2,057,664 sectors); checked by hand with `cargo run --release -p rc-trace -- disc-check` (check 1; no test reads the disc image).

---

## 2. RAC1 table of contents

### 2.1 Location and identification

The ToC is **not** in the ELF and **not** in the filesystem. It sits at a
hardcoded sector:

| Game | ToC LBA | ToC byte offset |
| --- | --- | --- |
| **RAC1** | **1500** | `1500 * 0x800 = 0x2EE000` |
| R&C2 / R&C3 / DL | 1001 | `0x1F4800` |

To decide whether a disc is RAC1 before parsing, either:

* Compare the PVD `volume_identifier` (32 bytes) against the ASCII string
  `RATCHETANDCLANK` right-padded with spaces to 32 characters, **or**
* Identify the boot ELF name (section 2.6), **or**
* Probe: read two `s32` at `1500 * 0x800`; if the first is `1` and the second is
  a plausible size, it is RAC1.

### 2.2 ToC header and validation

> Confirmed from the game's own code (boot ELF `load_disc_sectors_into_global_buffer`, 0x12f2b8): it issues a CD read of 6 sectors at LBA 1500 and copies exactly 0x2960 bytes into a global table at EE address 0x137b80. The in-game copy of the ToC therefore lives at `0x137b80` and every field offset below applies to that address as well.

At `1500 * 0x800`:

| Offset | Type | Name | Value / meaning |
| --- | --- | --- | --- |
| 0x0000 | s32 | `version` | Must equal **1** for RAC1. Acts as the magic number. |
| 0x0004 | s32 | `header_size` | Size of the whole ToC structure in bytes. **`0x2960` on retail RAC1.** |

Validation to apply: `version == 1` and `0 < header_size < 0x40000000`.
Then read `header_size` bytes starting at `1500 * 0x800` — that whole blob *is*
the table of contents (also called the "global WAD header" for RAC1, because
in the asset-pipeline sense RAC1 has exactly **one** global WAD and its header
is the ToC itself).

Practical limits worth enforcing: ToC blob `<= 0x200000` bytes; any individual
sub-header `<= 0x10000` bytes; level count `<= 100`.

### 2.3 The RAC1 global header (`0x2960` bytes)

This is one flat C struct of `SectorRange` / `SectorByteRange` / `Sector32`
entries pointing at **absolute sector numbers** all over the disc. Every field
below is at a byte offset from `1500 * 0x800`.

Field names are Wrench's; several are placeholders. Sizes: `SectorRange` = 8,
`SectorByteRange` = 8, `Sector32` = 4.

| Offset | Count | Entry type | Name | Content (where known) |
| --- | --- | --- | --- | --- |
| 0x0000 | 1 | s32 | `version` | `1` |
| 0x0004 | 1 | s32 | `header_size` | `0x2960` |
| 0x0008 | 1 | SectorRange | `debug_font` | Paletted 8bpp texture |
| 0x0010 | 1 | SectorRange | `save_game` | |
| 0x0018 | 28 | SectorRange | `ratchet_seqs` | WAD-compressed |
| 0x00f8 | 20 | SectorRange | `hud_seqs` | WAD-compressed |
| 0x0198 | 1 | SectorRange | `vendor` | |
| 0x01a0 | 37 | SectorRange | `vendor_audio` | Sony VAG audio |
| 0x02c8 | 12 | SectorRange | `help_controls` | 8bpp textures, **not** compressed |
| 0x0328 | 15 | SectorRange | `help_moves` | WAD-compressed 8bpp textures |
| 0x03a0 | 15 | SectorRange | `help_weapons` | WAD-compressed 8bpp textures |
| 0x0418 | 14 | SectorRange | `help_gadgets` | WAD-compressed 8bpp textures |
| 0x0488 | 7 | SectorRange | `help_ss` | WAD-compressed 8bpp textures |
| 0x04c0 | 7 | SectorRange | `options_ss` | WAD-compressed 8bpp textures |
| 0x04f8 | 1 | SectorRange | `frontbin` | A **"ratchet executable"** overlay (section 2.7) |
| 0x0500 | 81 | SectorRange | `mission_ss` | WAD-compressed 8bpp textures |
| 0x0788 | 19 | SectorRange | `planets` | WAD-compressed 8bpp textures |
| 0x0820 | 38 | SectorRange | `stuff2` **(unknown)** | WAD-compressed blobs |
| 0x0950 | 10 | SectorRange | `goodies_images` | WAD-compressed 8bpp textures |
| 0x09a0 | 19 | SectorRange | `character_sketches` | WAD-compressed 8bpp textures |
| 0x0a38 | 19 | SectorRange | `character_renders` | WAD-compressed 8bpp textures |
| 0x0ad0 | 31 | SectorRange | `skill_images` | WAD-compressed 8bpp textures |
| 0x0bc8 | 12 | SectorRange | `epilogue_english` | WAD-compressed 8bpp textures |
| 0x0c28 | 12 | SectorRange | `epilogue_french` | |
| 0x0c88 | 12 | SectorRange | `epilogue_italian` | |
| 0x0ce8 | 12 | SectorRange | `epilogue_german` | |
| 0x0d48 | 12 | SectorRange | `epilogue_spanish` | |
| 0x0da8 | 30 | SectorRange | `sketchbook` | WAD-compressed 8bpp textures |
| 0x0e98 | 4 | SectorRange | `commercials` | WAD-compressed 8bpp textures |
| 0x0eb8 | 9 | SectorRange | `item_images` | WAD-compressed 8bpp textures |
| 0x0f00 | 240 | Sector32 | `qwark_boss_audio` | Bare VAG sector pointers, no size (section 2.5) |
| 0x12c0 | 1 | SectorRange | `irx` | WAD-compressed archive of IOP (IRX) modules |
| 0x12c8 | 4 | SectorRange | `spaceships` | |
| 0x12e8 | 20 | SectorRange | `anim_looking_thing_2` **(unknown)** | |
| 0x1388 | 6 | SectorRange | `space_plates` | WAD-compressed texture collections |
| 0x13b8 | 1 | SectorRange | `transition` | WAD-compressed |
| 0x13c0 | 36 | SectorRange | `space_audio` | VAG |
| 0x14e0 | 1 | SectorRange | `sound_bank` | |
| 0x14e8 | 1 | SectorRange | `wad_14e0` **(unknown)** | Wrench's name is derived from a wrong offset comment; the field really lives at 0x14e8. |
| 0x14f0 | 1 | SectorRange | `music` | VAG |
| 0x14f8 | 1 | SectorRange | `hud_header` | |
| 0x1500 | 5 | SectorRange | `hud_banks` | |
| 0x1528 | 1 | SectorRange | `all_text` | All localised UI text |
| 0x1530 | 28 | SectorRange | `things` **(unknown)** | |
| 0x1610 | 1 | SectorRange | `post_credits_helpdesk_girl_seq` | WAD-compressed |
| 0x1618 | 18 | SectorRange | `post_credits_audio` | VAG |
| 0x16a8 | 20 | SectorRange | `credits_images_ntsc` | Raw RGBA 512x416 |
| 0x1748 | 20 | SectorRange | `credits_images_pal` | Raw RGBA 512x448 |
| 0x17e8 | 2 | SectorRange | `wad_things` **(unknown)** | WAD-compressed |
| 0x17f8 | 88 | **SectorByteRange** | `mpegs` | PSS video streams (byte-exact sizes) |
| 0x1ab8 | 900 | Sector32 | `help_audio` | Bare VAG sector pointers (section 2.5) |
| **0x28c8** | **19** | SectorRange | **`levels`** | The level table (section 2.4) |
| 0x2960 | — | — | *end of structure* | |

Note the three fields at 0x14e0 / 0x14e8 / 0x14f0: Wrench's source comments
incorrectly label all three as `0x14e0`. The offsets above are the real ones,
derived by summing the field sizes; the sum lands exactly on `0x2960`, which
confirms the layout.

Two ways to view this same blob, which is useful when relocating a rebuilt ToC
(all sector offsets are absolute, so they must be rebased as a block):

| Offset | Count | Entry type | Group |
| --- | --- | --- | --- |
| 0x0000 | 1 | s32 | `version` |
| 0x0004 | 1 | s32 | `header_size` |
| 0x0008 | 479 | SectorRange | all ranges before the first bare-sector array |
| 0x0f00 | 240 | Sector32 | bare sector array 1 |
| 0x12c0 | 167 | SectorRange | all ranges between the bare arrays |
| 0x17f8 | 88 | SectorByteRange | the MPEG array |
| 0x1ab8 | 900 | Sector32 | bare sector array 2 |
| 0x28c8 | 19 | SectorRange | the level table |

### 2.4 The level table and the on-disc level header

RAC1's level table at `0x28c8` is 19 `SectorRange` entries. **Only the `offset`
field matters when reading**; each non-zero offset is an absolute sector number
pointing at one **amalgamated level header**. (The `size` field is not used by a
reader. When Wrench rebuilds a disc it writes `1` there.)

A robust reader should not trust the table offset blindly. The discovery
algorithm that works on every RAC1 build is:

1. Slurp `header_size` bytes of the ToC.
2. For every byte offset `ofs` from `8` to `header_size`, stepping by **8**,
   interpret the `s32` at `ofs` as a candidate absolute sector number `lsn`.
   (Stepping by 8 from offset 8 hits the `offset` field of every `SectorRange`
   and `SectorByteRange`, including the level table, and every second entry of
   the bare `Sector32` arrays — harmless, since those point at VAG audio.)
3. Skip `lsn == 0`.
4. Read `0x2434` bytes at `lsn * 0x800`. Guard against running past the end of
   the image; a candidate near the end of the file must be rejected, not fatal.
5. Read the `s32` at relative offset `4`. **If it equals `0x2434`, this is a
   level header.** Otherwise ignore the candidate.
6. Parse it as the amalgamated level header below.

Because it is a scan, levels are discovered in ToC order. For retail RAC1 the
19 hits are exactly the 19 entries of the `levels` array. Do not hard-code 19;
prototype and demo builds may use fewer.

**Amalgamated level header — `0x2434` bytes, stored on disc.** This is the
*only* level header RAC1 has; later games have three separate headers per level.
It contains **absolute** sector numbers, and — importantly — it points at
sectors **before itself** on the disc, because the header sits at the *front of
the level's data region* but the pointed-to data starts after it only for the
"data" group, not for audio/scene.

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x0000 | s32 | `id` | Level number. Used as the level's identity (not the table index). |
| 0x0004 | s32 | `header_size` | Always `0x2434`. Serves as the discovery signature. |
| 0x0008 | SectorRange | `data` | The level's "primary"/data WAD (section 2.8). |
| 0x0010 | SectorRange | `gameplay_ntsc` | Gameplay/instances section, NTSC variant. |
| 0x0018 | SectorRange | `gameplay_pal` | Gameplay/instances section, PAL variant. On NTSC discs this is normally a duplicate or empty. |
| 0x0020 | SectorRange | `occlusion` | Occlusion-culling data. |
| 0x0028 | 36 × SectorByteRange | `bindata` **(semantics unknown)** | Per-level audio binary data, byte-exact sizes. |
| 0x0148 | 15 × Sector32 | `music` | Bare sector pointers to VAG music streams; sizes come from the VAG headers (section 2.5). |
| 0x0184 | 15 × `SceneRecord` | `scenes` | In-engine scene (cutscene) table, 15 records of `0x250` bytes. |
| 0x2434 | — | — | *end of structure* |

**`SceneRecord` — `0x250` bytes** (corrected 2026-09-27; the game indexes `0x13a664 + id·0x250`, see
docs/plan/cutscenes_transitions.md §1). Wrench (and this document before the correction) read the block as
30 × `0x128` `{sounds[6], wads[68]}`, which splits every scene into an NTSC half and a PAL half and
mislabels PAL chunks 0–2 as "sounds".

| Offset | Count | Type | Name |
| --- | --- | --- | --- |
| 0x000 | 6 | Sector32 | `speech` — VAG per language (0 En, 1 unused, 2 Fr, 3 De, 4 Es, 5 It) |
| 0x018 | 71 | Sector32 | `ntsc` — chunk WAD sectors, then a 1-sector all-zero sentinel |
| 0x134 | 71 | Sector32 | `pal` — the same for PAL (80-tick chunks) |
| 0x250 | — | — | *end* |

A chunk's size is the sector difference to the next entry (verified: equals the padded WAD size for all
4,081 chunks); the chunks of a region are contiguous. The extractor writes each region as one file
`levels/NN/scene/KK_ntsc.bin` / `KK_pal.bin` (chunks + sentinel) and each speech VAG as
`levels/NN/speech/KK_<en|fr|de|es|it>.bin` (`rc_formats::toc::level_stream_lumps`).

### 2.5 Deriving sizes for bare `Sector32` pointers

Several arrays store only a start sector with no size. Two probes resolve them:

**VAG audio.** Read `0x30` bytes at the sector. If bytes `0..3` are the ASCII
`VAGp`, the size is `0x30 + byte_swap_32(data_size)` bytes, rounded up to a
sector. `VagHeader` is **big-endian** in its numeric fields:

| Offset | Size | Field |
| --- | --- | --- |
| 0x00 | 4 | `magic` = `"VAGp"` |
| 0x04 | 4 | `version` (big-endian) |
| 0x08 | 4 | reserved |
| 0x0c | 4 | `data_size` (big-endian) — payload bytes after the header |
| 0x10 | 4 | `frequency` (big-endian) |
| 0x14 | 10 | reserved |
| 0x1e | 1 | `channel_count` |
| 0x1f | 1 | reserved |
| 0x20 | 16 | `name` |
| 0x30 | — | audio data |

If the magic does not match, fall back to a size of **1 sector**.

**WAD-compressed blobs.** Read 7 bytes at the sector. If bytes `0..2` are the
ASCII `WAD`, the size in sectors is `ceil(compressed_size / 0x800)` where
`compressed_size` is the `s32` at relative offset `3` (**unaligned** — read it
bytewise). If the magic does not match, fall back to **1 sector**.

A third, purely structural, technique used for later games: collect every start
sector referenced by a table into a sorted set, add the end-of-file sector, and
take each entry's size as the distance to the next larger member. This is the
right fallback when a blob is neither VAG nor WAD.

### 2.6 Locating the boot ELF and detecting region / version

Two independent methods, best used together:

**A. Via `SYSTEM.CNF`.** Parse `BOOT2` (section 1.4) to get the ELF's
filesystem name (e.g. `SCUS_971.99`), then find the matching root-directory
file record (compare case-insensitively, with `;1` stripped). Read `size` bytes
from `lba * 0x800`.

**B. By probing.** Iterate root-directory files with `size > 4`; the first whose
first four bytes are `7F 45 4C 46` (`\x7fELF`) is the boot ELF. On RAC1 this
always works because the boot ELF is unwrapped. On R&C3 / Deadlocked the boot
ELF *is* wrapped and this probe can fail — see section 2.7.

**Region / build identification** is done by the boot ELF's filename, which
is the disc's SLES/SCUS/SCPS product code:

| ELF filename (lowercased) | Game | Region | Build |
| --- | --- | --- | --- |
| `scus_971.99` | RAC1 | US (NTSC) | Original / Greatest Hits |
| `scus_972.09` | RAC1 | US (NTSC) | Demo 1 |
| `scus_972.40` | RAC1 | US (NTSC) | Demo 2 |
| `sces_509.16` | RAC1 | EU (PAL) | Black Label / Platinum |
| `sced_510.75` | RAC1 | EU (PAL) | Demo |
| `scps_150.37` | RAC1 | Japan (NTSC) | Original |

For completeness, the first ELF names of the sequels (useful to *reject* a disc
as not-RAC1): `scps_150.56`, `sces_516.07`, `scus_972.68`, `scus_973.22`,
`scus_973.23`, `scus_973.74`, `scka_200.11` (R&C2); `papx_905.20`,
`sced_528.47`, `sced_528.48`, `sces_524.56`, `scps_150.84`, `scus_973.53`,
`scus_974.11`, `scus_974.13`, `tces_524.56`, `scka_200.37` (R&C3);
`pcpx_980.17`, `sced_536.60`, `sces_532.85`, `scps_150.99`, `scps_151.00`,
`scus_974.65`, `scus_974.85`, `scus_974.87`, `scka_200.60` (Deadlocked).

Matching rules:

* Exact, case-insensitive match on the lowercased name with `;1` stripped.
* A **fuzzy** match is also useful: normalise both sides by lowercasing and
  deleting every character that is not `[a-z0-9]` (so `SCUS-97199`,
  `SCUS_971.99` and `scus97199` all collapse to `scus97199`).

**Fallback when the ELF name is unknown** (prototypes, patched discs): read the
whole boot ELF into memory and search for the first occurrence of one of these
ASCII substrings, in this priority order — `"Deadlocked"` ⇒ DL,
`"Up Your Arsenal"` ⇒ R&C3, `"Going Commando"` ⇒ R&C2,
`"Ratchet & Clank"` ⇒ **RAC1**. The order matters because the later games' ELFs
also contain the string `"Ratchet & Clank"`. This fallback determines the *game*
but not the region; region must then come from `VMODE` in `SYSTEM.CNF` or be
supplied by the user.

**Version string**: the `VER = ` value in `SYSTEM.CNF` (e.g. `1.00`).

**There are no known ELF checksums / hashes used for build identification.**
Identification is filename-based with a string-search fallback. If you want
hash-based build pinning you will have to build that table yourself.

**Region ⇒ video mode:** PAL if and only if the region is EU; US, Japan and
Korea are NTSC. Frame rates: NTSC 59.94005994005994 Hz (half: 29.97002997…),
PAL 50 Hz (half: 25).

### 2.7 The "packed executable" and "ratchet executable" formats

RAC1's **boot ELF is a normal, unwrapped ELF** — read it as-is. This section
matters for RAC1 only because the same *inner* format is used by RAC1's level
code overlays and by the global `frontbin` asset.

**"Ratchet executable"** (Insomniac's own loadable-image format): a bare
concatenation of `(header, data)` blocks with no file header and no terminator.

| Offset | Type | Field | Meaning |
| --- | --- | --- | --- |
| 0x0 | s32 | `dest_address` | Virtual address to copy the section data to. |
| 0x4 | s32 | `copy_size` | Bytes of data following this header. |
| 0x8 | s32 | `section_type` | The equivalent ELF `sh_type`. Not used by the game. |
| 0xc | s32 | `entry_point` | Address of the level/overlay entry function. Repeated identically in every block. |

Parsing: start at offset 0. Read a 16-byte header, then `copy_size` bytes of
payload, then repeat. **Termination:** the game breaks out of the loop when the
`entry_point` field of a block differs from the `entry_point` of the first
block. This means the game deliberately reads one header past the end of the
data; a well-behaved reader should *also* stop when the read position reaches or
exceeds the buffer size, and should treat a partial trailing header as the end.
There is no section-name table; names must be supplied from a per-game donor
table if you want to emit a real ELF.

**"Packed executable"** (R&C3 and Deadlocked boot ELFs only — **not RAC1**): a
small unwrapped ELF stub, somewhere inside which is embedded a WAD-compressed
stream. To unwrap: scan the file for the first occurrence of the ASCII bytes
`WAD`, decompress from that offset (section 3), and parse the result as a
ratchet executable. If no `WAD` magic is found, the file is an ordinary ELF.
Note that the naive "first file whose first 4 bytes are `\x7fELF`" probe still
finds these, because the stub itself is an ELF.

Consequences for RAC1: the boot ELF needs no unwrapping; the `frontbin` global
asset and each level's `overlay` asset **are** ratchet executables and do need
the block parser above.

### 2.8 How a RAC1 "level" maps onto disc sectors

RAC1 has **no** `LEVELnn.WAD` / `AUDIOnn.WAD` / `SCENEnn.WAD` files. There is
one contiguous run of sectors per level, described by the single amalgamated
header. Three logically distinct asset groups live in that run:

| Group | Fields of the amalgamated header |
| --- | --- |
| Level (a.k.a. "level WAD") | `data`, `gameplay_ntsc`, `gameplay_pal`, `occlusion` |
| Audio | `bindata[36]`, `music[15]` |
| Scene | `scenes[15].speech[6]`, `scenes[15].ntsc[71]`, `scenes[15].pal[71]` |

To carve out a group's extent, compute over that group's non-empty entries:

* `low  = min(entry.offset)`
* `high = max(entry_end)` where `entry_end` is:
  * `offset + size` for a `SectorRange`,
  * `offset + ceil(size_bytes / 0x800)` for a `SectorByteRange`,
  * `offset + probed_size` for a bare `Sector32` (VAG probe for `speech` /
    `music`; for the scene chunk lists the last entry + 1, the sentinel sector — section 2.5).

If a group has no non-empty entries, it is **absent** for that level (this
happens; the audio and scene groups can both be missing).

Note this is an **inference**, not a stored value: RAC1 does not record group
sizes anywhere. Adjacent groups and adjacent levels can therefore not be
distinguished by the header alone; only by the union of what is referenced.

**Wrench-specific rewrite (for interoperability awareness only).** When Wrench
extracts a RAC1 disc it *invents* three files per level so the asset pipeline
can be shared with the sequels. For each group it computes
`header_sectors = ceil(sizeof(new_header) / 0x800)` and
`base = low - header_sectors`, emits a file starting at sector `base` with
length `high - base` sectors, writes its own new header at the start of that
file, and rewrites every sector number as `absolute - base`. The three new
headers are:

*Rewritten level header — `0x30` bytes:*

| Offset | Type | Name |
| --- | --- | --- |
| 0x00 | s32 | `header_size` = `0x30` |
| 0x04 | s32 | unused (0; in the sequels this slot holds the file's own sector) |
| 0x08 | s32 | `id` (level number) |
| 0x0c | s32 | unused (0) |
| 0x10 | SectorRange | `data` |
| 0x18 | SectorRange | `gameplay_ntsc` |
| 0x20 | SectorRange | `gameplay_pal` |
| 0x28 | SectorRange | `occlusion` |

`header_sectors` = 1.

*Rewritten audio header — `0x164` bytes:*

| Offset | Type | Name |
| --- | --- | --- |
| 0x000 | s32 | `header_size` = `0x164` |
| 0x004 | s32 | padding |
| 0x008 | 36 × SectorByteRange | `bindata` |
| 0x128 | 15 × Sector32 | `music` |

`header_sectors` = 1.

*Rewritten scene header — `0x22b8` bytes:*

| Offset | Type | Name |
| --- | --- | --- |
| 0x0000 | s32 | `header_size` = `0x22b8` |
| 0x0004 | s32 | padding |
| 0x0008 | 30 × `SceneHeader` (`0x128` each) | `scenes` |

`header_sectors` = 5 (`ceil(0x22b8 / 0x800)`).

An independent extractor writing raw sector ranges does **not** need any of
this; it is documented so that files produced by Wrench can be recognised
(a `0x30`/`0x164`/`0x22b8` first word instead of `0x2434`).

Also note that when *rebuilding*, Wrench relocates the amalgamated level headers
into the ToC region (immediately after the `0x2960` global header, one sector-
aligned `0x2434` block per level) rather than at the front of each level's data.
Both work, because the level table holds absolute sector numbers. On a real
retail disc the header is at the front of the level's sector run.

Level ordering on a rebuilt disc, for reference: audio groups for all levels
first, then level groups for all levels, then scene groups (the "SoA" layout,
used for RAC1, R&C3 and Deadlocked). R&C2 interleaves them per level ("AoS").

### 2.9 Optional: levels not referenced by the ToC

Some non-retail builds ship levels that the ToC does not point at, but which
*are* present in the ISO9660 filesystem. Heuristic recovery: for every file in
any directory whose lowercased name starts with `level` and ends with `.wad`,
parse the digits between those two as the level table index. If that index is
within the level table and that slot is empty, adopt the file: its header is the
first `s32` bytes at the start of the file (i.e. read `header_size` at offset 0,
then read that many bytes), and its sector numbers are **relative to the file**,
not absolute. This case does not arise on retail RAC1 (no `.wad` files in the
filesystem) but it is cheap to support.

### 2.10 How RAC1 differs from R&C2 / R&C3 / Deadlocked

| Aspect | RAC1 | R&C2 / R&C3 / DL |
| --- | --- | --- |
| ToC sector | 1500 | 1001 |
| `SYSTEM.CNF` pinned sector | 289 | 1000 |
| ToC magic | first `s32` == `1` (a version field) | no magic; first `s32` is the first global header's size |
| ToC structure | one fixed `0x2960` struct, then a 19-entry level table at `0x28c8` | a variable-length run of concatenated global-WAD headers, then the level table |
| Global WAD count | **1** (the ToC itself is its header) | 8–9 separate global WADs (`mpeg`, `misc`, `hud`, `bonus`, `audio`, `space`, `scene`, `gadget`, `armor`, `online`) |
| Global header prefix | no per-WAD `{size, sector}` prefix | every global header starts with `s32 header_size; s32 absolute_sector` |
| Level table entry | 1 `SectorRange` per level, pointing at one amalgamated header | 3 `SectorRange`s per level, pointing at three separate headers |
| Level table entry field order | n/a | R&C2: level, audio, scene. R&C3/DL: audio, level, scene. |
| Level headers | one `0x2434` header, **absolute** sector numbers, at the front of the level's sector run | three headers, each duplicated inside the ToC, sector numbers **relative** to the file, each header starting `{header_size, file_sector}` |
| Level header size | `0x2434` | `0x60` (R&C2/3), `0x68`, `0xc68` (DL) etc. — see section 4 |
| Boot ELF on disc | plain ELF | plain ELF for R&C2; **packed** (ELF stub + embedded WAD stream) for R&C3 and Deadlocked |
| Assets in the filesystem | no | yes for R&C2 retail (plus `RC2.HDR`); no for R&C3/DL |
| Level table located by | fixed offset `0x28c8`, or a step-8 scan for `0x2434` signatures | a brute-force scan (see below) |
| Hardcoded ToC size cap (sectors) | none observed | R&C2: `0x0b`, R&C3: `0x10`, DL: `0x1a` |

For completeness, the sequels' level-table discovery works by sliding a window
over the ToC in 4-byte steps and, at each position, treating six alternating
`s32` as sector numbers; each is converted to a ToC-relative offset
(`lsn * 0x800 - 1001 * 0x800`), the `s32` there is read as a header size, and
that many bytes are run through the WAD identifier (section 4). If all six
positions identify as a known WAD type, that position is the level table. Six
(not two or three) are required to get past a false positive in Deadlocked.

### 2.11 Optional: inside the RAC1 level data WAD

The `data` range of the amalgamated header points at a blob whose first bytes
are a table of `ByteRange`s **relative to the start of that blob**:

| Offset | Type | Name |
| --- | --- | --- |
| 0x00 | ByteRange | `overlay` — a ratchet executable (section 2.7) |
| 0x08 | ByteRange | `sound_bank` |
| 0x10 | ByteRange | `core_index` |
| 0x18 | ByteRange | `gs_ram` |
| 0x20 | ByteRange | `hud_header` |
| 0x28 | 5 × ByteRange | `hud_banks` — each WAD-compressed |
| 0x50 | ByteRange | `core_data` |

An empty entry is encoded as `{offset = -1, size = 0}`. Contained sub-assets are
aligned to `0x40` bytes. For contrast: R&C2/R&C3 drop `sound_bank` and add
`transition_textures`; Deadlocked adds `moby8355_pvars`, `art_instances`,
`gameplay_core` and `global_nav_data`. This is outside the scope of the disc
layout but is the natural next hop for an extractor.

---

## 3. WAD compression ("WAD" LZ scheme)

This is a byte-oriented LZ77 variant. It is **not** related to Doom WAD
archives; here "WAD" is just a 3-byte magic on a compressed stream. It is used
for individual assets scattered across the disc, not for whole files.

### 3.1 Container header — `0x10` bytes

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0x0 | 3 | `magic` | ASCII `WAD`, no NUL |
| 0x3 | 4 | `compressed_size` | `s32`, little-endian, **unaligned** — read bytewise. Counts the whole stream **including** this 16-byte header. |
| 0x7 | 9 | signature / padding | Free-form. Insomniac's value is build-specific; Wrench writes the ASCII `WRENCH010` here by default. A reader must ignore it. |
| 0x10 | … | packet stream | |

Validation before decompressing: the buffer must hold at least `0x10` bytes;
the magic must match; and `compressed_size` must not exceed the available bytes.
Then clamp the input end to `start + compressed_size`.

There is **no stored decompressed size** and **no checksum**. Grow the output
buffer dynamically.

### 3.2 Decompression driver

```
begin = input + 0x10            // packet-stream origin; alignment reference
end   = input + compressed_size
ptr   = begin
out   = empty byte vector
while ptr < end:
    decode_one_packet()         // section 3.3
```

Termination is purely "input exhausted". Any read that would move `ptr` outside
`[begin, end)` is a hard format error, **except** the alignment skip in the pad
packet (3.3.2), which is allowed to push `ptr` past `end` and thereby end the
loop.

### 3.3 Packet decoding

Every packet starts with one **flag byte**. Read it and dispatch on its value.
Throughout, `P` = current output length (`out.size()`), which is the "write
cursor"; back-references are relative to `P` *at the time the match is decoded*.

#### 3.3.0 Literal packets — flag `0x00`–`0x0f`

| Flag | Literal length |
| --- | --- |
| `0x00` | read one more byte `N`; length = `N + 18` (range 18..273) |
| `0x01`–`0x0f` | length = `flag + 3` (range 4..18) |

Copy that many bytes verbatim from the input to the output, advancing `ptr`.

Then, if `ptr < end` and the **next** byte is also `< 0x10`, the stream is
malformed: **two literal packets may never be adjacent.** (This is a real
constraint of the format, not a reader nicety; the game relies on it.) A
literal packet carries **no** trailing "little literal".

Note the `N + 18` big-literal encoding means `N = 0` is unreachable from the
compressor side (an 18-byte literal is encoded as flag `0x0f`), so in practice
`N >= 1` and the range is 19..273.

#### 3.3.1 Far match packets — flag `0x10`–`0x1f`

```
match_len = flag & 7
if match_len == 0: match_len = read_byte() + 7      // "big far match"
b0 = read_byte()
b1 = read_byte()
A  = (flag >> 3) & 1
displacement = 0x4000 * (A + 1) + b1 * 0x40 + (b0 >> 2)      // see caveat
```

**Resolved on retail data (2026-09-26):** the `0x4000 * (A + 1)` form is correct. With it every one of the 4674 WAD streams on the NTSC-U disc decodes and all 19 levels' decompressed `core_data` sizes equal the `assets_decompressed_size` recorded in their core index. The original note follows for history.

**Caveat — original discrepancy.** Wrench's decompressor computes the extra
displacement contributed by bit 3 of the flag as `(flag & 8) * 0x800`, which
evaluates to `0x4000` (because `flag & 8` is the *bit value* 8, not 1). Wrench's
own compressor and prose documentation instead assume `A * 0x800`, i.e.
`displacement = 0x4000 + A * 0x800 + b1 * 0x40 + (b0 >> 2)`. Wrench's matcher
caps lookback at 32704, so it never emits `A = 1` and the two never disagree in
practice. **Neither interpretation is verified against real game data for
`A = 1`.** Implement the `A * 0x4000` form (it is what has been run against real
discs for `A = 0`, where both forms are identical), and put an assertion on
`A == 1` so you find out if real data exercises it.

Two sub-cases:

* **Normal far match** — when `displacement != 0` (equivalently: not all of
  `A`, `b1`, `b0 >> 2` are zero):
  * `match_len += 2` (so a medium far match is 3..9 bytes and a big far match is
    `N + 9`, i.e. 9..264 bytes)
  * source position = `P - displacement`
  * proceed to the copy step (3.3.4) and then the little-literal step (3.3.5).
* **Zero-displacement special case** — when `A == 0`, `b1 == 0` and
  `(b0 >> 2) == 0`, the packet is a control packet, not a match:
  * If `match_len != 1` ⇒ **pad packet** (3.3.2).
  * If `match_len == 1` ⇒ **dummy packet** (3.3.3).

#### 3.3.2 Pad packet

Canonical encoding: `12 00 00`.

Effect: skip `ptr` forward until `(ptr - begin) % 0x1000 == 0`, then **return
immediately** — no data is copied and the little-literal step is skipped. The
bytes skipped over are filler; Wrench writes `0xEE`.

Why it exists: the game DMAs the compressed stream into the Emotion Engine's
16 KiB scratchpad in chunks, so no packet may straddle a chunk boundary. The
compressor inserts a pad packet whenever the next packet would cross one, then
fills to the boundary.

Wrench's writer treats the chunk size as **`0x2000`** (padding so that
`(offset_from_file_start) % 0x2000 == 0x10`), while the reader aligns to
**`0x1000`**. `0x2000` alignment satisfies `0x1000` alignment, so both
interoperate, but the true hardware chunk size is unconfirmed. Align to `0x1000`
when reading (it accepts both) and use `0x2000` when writing (it is what has
been validated on hardware).

If the alignment skip takes `ptr` to or past `end`, decompression ends normally.

#### 3.3.3 Dummy packet

Canonical encoding: `11 00 00`, optionally with a little literal: `11 0N 00`
followed by `N` literal bytes, `N` in 1..3.

Effect: no match copy. Fall through to the little-literal step (3.3.5), which
appends `b0 & 3` bytes. With `b0 == 0` it is a complete no-op.

Purpose: two literal packets may not be adjacent (3.3.0), and small literals
(1..3 bytes) can only ride along inside a match packet. The dummy packet is the
vehicle for both cases.

#### 3.3.4 Medium / big match packets — flag `0x20`–`0x3f`

```
match_len = flag & 0x1f
if match_len == 0: match_len = read_byte() + 0x1f   // "big match"
match_len += 2
b1 = read_byte()
b2 = read_byte()
source = P - (b2 * 0x40 + (b1 >> 2) + 1)
```

So a medium match copies `(flag & 0x1f) + 2` bytes (3..33) and a big match
copies `N + 33` bytes (33..288). Displacement range: 1..16384.

#### 3.3.4b Little match packets — flag `0x40`–`0xff`

```
b1 = read_byte()
match_len = (flag >> 5) + 1                         // 2..8
source    = P - (b1 * 8 + ((flag >> 2) & 7) + 1)    // displacement 1..2048
```

#### 3.3.4c The copy step

Applies to all three match kinds. If `match_len == 1`, skip the copy entirely
(this only arises in the dummy-packet path). Otherwise:

* Validate `0 <= source` and `source < P`. A source at or beyond `P` is a format
  error.
* Copy `match_len` bytes **one byte at a time, forwards**, from `out[source+i]`
  to `out[P+i]`. The copy is **self-overlapping by design**: when
  `P - source < match_len` the bytes written earlier in this same copy are read
  back, producing run-length expansion. Do **not** use `memcpy`; use a byte loop
  or `memmove`-safe forward semantics.

#### 3.3.5 The little-literal step

Applies to every match packet and to the dummy packet (but **not** to literal
packets and **not** to the pad packet, which returns early).

Let `Q = ptr - 2`, i.e. the **second-to-last byte consumed by this packet**. The
little-literal length is `*Q & 3`, in 0..3. Copy that many bytes verbatim from
the input to the output.

Which byte `Q` is, per packet kind:

| Packet | Bytes consumed | `Q` is | Little-literal length |
| --- | --- | --- | --- |
| Far match (`0x10`–`0x1f`) | flag [, size] , b0, b1 | `b0` | `b0 & 3` |
| Medium/big match (`0x20`–`0x3f`) | flag [, size] , b1, b2 | `b1` | `b1 & 3` |
| Little match (`0x40`–`0xff`) | flag, b1 | **the flag byte** | `flag & 3` |
| Dummy (`0x11 xx 00`) | flag, b0, b1 | `b0` | `b0 & 3` |

Note the little match case: the low two bits of the flag byte serve double duty
as the little-literal length, which is why the little-match displacement field
is split into a 3-bit high part in the flag and an 8-bit part in `b1`.

### 3.4 Bitfield summary table

`X[n]` denotes a field `X` of `n` bits, MSB-first within each byte.
`L` is the literal payload. `P` is the output write cursor.

| Flag range | Name | Bit layout | Output |
| --- | --- | --- | --- |
| `0x00` | Big literal | `00000000` `N[8]` `L[(N+18)*8]` | append `L` (`N+18` bytes) |
| `0x01`–`0x0f` | Medium literal | `0000` `N[4]` `L[(N+3)*8]` | append `L` (`N+3` bytes) |
| `0x10`, `0x18` with `b0>>2 != 0` or `b1 != 0` | Big far match | `0001` `A[1]` `000` `M[8]` `B[6]` `N[2]` `C[8]` `L[N*8]` | `M+9` bytes from `P - 0x4000*(A+1) - C*0x40 - B`, then `L` |
| `0x11`–`0x1f` with non-zero displacement | Medium far match | `0001` `A[1]` `M[3]` `B[6]` `N[2]` `C[8]` `L[N*8]` | `M+2` bytes from `P - 0x4000*(A+1) - C*0x40 - B`, then `L` |
| `0x12`–`0x17` with `A=0,B=0,C=0` | Pad packet | `00010` `M[3]` `00000000` `00000000` | nothing; skip input to the next `0x1000` boundary and end the packet |
| `0x11` with `A=0,B=0,C=0` | Dummy packet | `00010001` `000000` `N[2]` `00000000` `L[N*8]` | append `L` (`N` bytes, 0..3) |
| `0x20` | Big match | `00100000` `M[8]` `A[6]` `N[2]` `B[8]` `L[N*8]` | `M+33` bytes from `P - B*0x40 - A - 1`, then `L` |
| `0x21`–`0x3f` | Medium match | `001` `M[5]` `A[6]` `N[2]` `B[8]` `L[N*8]` | `M+2` bytes from `P - B*0x40 - A - 1`, then `L` |
| `0x40`–`0xff` | Little match | `M[3]` `A[3]` `N[2]` `B[8]` `L[N*8]` | `M+1` bytes from `P - B*8 - A - 1`, then `L` |

### 3.5 Optional: compressor behaviour (only needed for repacking)

You do not need any of this to read a disc. It is here so a round-trip tool can
produce streams the game accepts.

**Encoding limits implied by the format:**

| Quantity | Min | Max | Notes |
| --- | --- | --- | --- |
| Literal in its own packet | 4 | 273 | 1..3-byte literals must be injected as little literals |
| Little literal (inside a match/dummy packet) | 0 | 3 | |
| Little match length | 3 | 8 | (`M+1`, `M` in 2..7; `M=1` is reserved for the dummy path) |
| Little match displacement | 1 | 2048 | |
| Medium match length | 3 | 33 | |
| Big match length | 33 | 288 | |
| Medium/big match displacement | 1 | 16384 | `16384` is the boundary; displacement `0` is the control-packet escape |
| Medium far match length | 3 | 9 | |
| Big far match length | 9 | 264 | |
| Far match displacement, `A = 0` | 16385 | 32704 | |
| Far match displacement, `A = 1` | — | 34752 (per the compressor) / 49151 (per the decompressor) | **unverified**; avoid emitting `A = 1` |

Because `A = 1` is ambiguous (3.3.1), a safe compressor restricts its search
window to **32704 bytes** and never sets bit 3 of a far-match flag.

**Hard rules the encoder must respect:**

1. Never emit two literal packets in a row. If you must, insert a dummy packet
   `11 00 00` between them (and you may as well stuff up to 3 literal bytes into
   it).
2. Literals of 1..3 bytes cannot be a packet of their own. Inject them into the
   preceding match packet by OR-ing the length into that packet's `Q` byte
   (3.3.5) and appending the bytes after the packet. If the preceding packet is a
   literal packet, or already carries a little literal, emit a dummy packet
   first and inject into that.
3. No packet (header + its little literal) may straddate a chunk boundary.
   Before writing a packet, if
   `((current_offset_from_file_start + 0x1ff0) % 0x2000) + packet_size > 0x2000 - 3`,
   emit `12 00 00` and then fill with a filler byte (`0xEE` is conventional)
   until `current_offset_from_file_start % 0x2000 == 0x10`.
4. Patch `compressed_size` at offset `0x3` to the final total stream length
   (including the 16-byte header) once everything is written.

**Match search (an approach known to work, not the original):** a 32768-entry
hash-chain matcher. Hash the next three bytes, walk up to ~16 chain entries that
are still inside the window, reject candidates whose first two bytes differ,
count matching bytes up to the maximum match length, keep the longest, and stop
scanning as soon as a match of length >= 3 is found (accumulating the skipped
bytes as the pending literal). Matches shorter than 3 bytes are discarded.
Parallelisation is possible by splitting the input into independent blocks and
concatenating the packet streams, but each join needs a dummy packet inserted
(rule 1) and the chunk-boundary padding must be recomputed over the joined
stream (rule 3). Output will not be byte-identical to Insomniac's compressor.

---

## 4. WAD-identification heuristics

When a header blob is found but its type is unknown, the type is inferred almost
entirely from its **size in bytes**, with tie-breaks on two extra fields. This
is how the sequels' ToC parsers classify the three headers of a level-table
entry. For RAC1 it is only used once: to confirm that the `0x2960` ToC blob is
the (single) global WAD header.

Procedure: walk the table below in order; take the **first** row whose
`header_size` equals the blob's byte length **and** whose optional secondary and
tertiary conditions hold. If none matches, the type is unknown.

* **Secondary condition:** read the `s32` at `secondary_offset`; the row matches
  only if `min <= value <= max`.
* **Tertiary condition:** read the `s32` at `tertiary_offset`; the row is
  **rejected** if the value equals `tertiary_not_equal`.

| Size | Game | Type | Name | Secondary (offset, min..max) | Tertiary (offset, != value) |
| --- | --- | --- | --- | --- | --- |
| `0x2960` | **RAC1** | GLOBAL | `global` | — | — |
| `0x0030` | **RAC1** | LEVEL | `level` | — | — |
| `0x0164` | **RAC1** | LEVEL_AUDIO | `audio` | — | — |
| `0x22b8` | **RAC1** | LEVEL_SCENE | `scene` | — | — |
| `0x0328` | R&C2 | MPEG | `mpeg` | — | — |
| `0x0040` | R&C2 | MISC | `misc` | — | — |
| `0x1870` | R&C2 | HUD | `hud` | — | — |
| `0x0a48` | R&C2 | BONUS | `bonus` | — | — |
| `0x1800` | R&C2 | AUDIO | `audio` | — | — |
| `0x0ba8` | R&C2 | SPACE | `space` | — | — |
| `0x0170` | R&C2 | SCENE | `scene` | — | — |
| `0x03c8` | R&C2 | GADGET | `gadget` | `0x08`, `0` .. `0x586` | — |
| `0x03c8` | R&C3 | GADGET | `gadget` | `0x08`, `0x587` .. `0x1000` | — |
| `0x03c8` | unknown | GADGET | `gadget` | — | — |
| `0x00f8` | R&C2 | ARMOR | `armor` | — | — |
| `0x0060` | unknown | LEVEL | `level` | — | — |
| `0x1018` | R&C2 | LEVEL_AUDIO | `audio` | — | — |
| `0x137c` | R&C2 | LEVEL_SCENE | `scene` | — | — |
| `0x0648` | R&C3 | MPEG | `mpeg` | `0x0c`, `0` .. `0x3b` | — |
| `0x0648` | DL | MPEG | `mpeg` | `0x0c`, `0x3c` .. `0x100` | — |
| `0x0648` | unknown | MPEG | `mpeg` | — | — |
| `0x0048` | R&C3 | MISC | `misc` | — | — |
| `0x0bf0` | R&C3 | BONUS | `bonus` | — | — |
| `0x0c00` | R&C3 | BONUS | `bonus` | — | Japan / Korea variant |
| `0x0c30` | R&C3 | SPACE | `space` | — | — |
| `0x0398` | R&C3 | ARMOR | `armor` | — | — |
| `0x2340` | R&C3 | AUDIO | `audio` | — | — |
| `0x2ab0` | R&C3 | HUD | `hud` | — | — |
| `0x1818` | R&C3 | LEVEL_AUDIO | `audio` | — | — |
| `0x26f0` | unknown | LEVEL_SCENE | `scene` | — | — |
| `0x0050` | DL | MISC | `misc` | — | — |
| `0x02a8` | DL | BONUS | `bonus` | — | — |
| `0x0068` | DL | SPACE | `space` | `0x0c`, `0` .. `0x75d` | `0x14`, `1` |
| `0x0068` | DL | ONLINE | `online` | `0x0c`, `0x75e` .. `0x1000` | `0x14`, `1` |
| `0x0068` | unknown | LEVEL | `level` | — | — |
| `0x0228` | DL | ARMOR | `armor` | — | — |
| `0x0248` | DL | ARMOR | `armor` | — | Japan / Korea variant |
| `0xa870` | DL | AUDIO | `audio` | — | — |
| `0x0f88` | DL | HUD | `hud` | — | — |
| `0x0c68` | DL | LEVEL | `level` | — | — |
| `0x02a0` | DL | LEVEL_AUDIO | `audio` | — | — |
| `0x1000` | unknown | LEVEL_AUDIO | `audio` | — | — |
| `0x2420` | unknown | LEVEL_SCENE | `scene` | — | — |

Notes and caveats:

* The rows with game "unknown" are deliberate catch-alls; they must be tried
  *after* the game-specific rows of the same size, which is why order matters.
* Two sizes are region-dependent: R&C3's BONUS header is `0x0c00` (rather than
  `0x0bf0`) on Japanese and Korean discs, and Deadlocked's ARMOR header is
  `0x0248` (rather than `0x0228`) on Japanese and Korean discs. If you need to
  go the other way (game+region+type ⇒ size), special-case those two first.
* **RAC1's on-disc `0x2434` level header is deliberately absent from this
  table.** Classifying it by size would be a separate rule; RAC1's level
  discovery uses the `s32 == 0x2434` signature at offset 4 directly (section
  2.4). `0x0030` / `0x0164` / `0x22b8` are Wrench's *rewritten* header sizes and
  appear only in files Wrench has already produced.
* This scheme is ambiguous by construction. Do not rely on it where a stronger
  signal exists (e.g. RAC1: the ToC location and the `0x2434` signature).

---

## 5. Validation checklist for an independent extractor

Structural invariants. These are static and should be compile-time asserted.

| # | Assertion |
| --- | --- |
| S1 | `sizeof(Sector32) == 4`, `sizeof(SectorRange) == 8`, `sizeof(SectorByteRange) == 8`, `sizeof(ByteRange) == 8` |
| S2 | `sizeof(PVD) == 0x800` |
| S3 | `sizeof(IsoDirectoryRecord fixed part) == 0x21` |
| S4 | `sizeof(Rac1GlobalHeader) == 0x2960`, with `levels` at `0x28c8` and exactly 19 entries (`0x28c8 + 19*8 == 0x2960`) |
| S5 | `sizeof(Rac1AmalgamatedLevelHeader) == 0x2434`, with `bindata` at `0x0028`, `music` at `0x0148`, `scenes` at `0x0184` |
| S6 | `sizeof(SceneRecord) == 0x250` (6*4 + 71*4 + 71*4); Wrench's `SceneHeader` 0x128 is half a record |
| S7 | `sizeof(VagHeader) == 0x30` |
| S8 | `0x0184 + 30 * 0x128 == 0x2434` |
| S9 | Wrench-compat header sizes, if you emit them: level `0x30`, audio `0x164` (`8 + 36*8 + 15*4`), scene `0x22b8` (`8 + 30*0x128`) |

Disc-level assertions to run against a real retail RAC1 ISO.

| # | Assertion |
| --- | --- |
| D1 | The image size is a whole multiple of `0x800`. |
| D2 | Bytes at `0x10 * 0x800 + 0` == `0x01`; bytes at `0x10 * 0x800 + 1 .. +5` == `"CD001"`. |
| D3 | PVD `logical_block_size.lsb == 0x800`. |
| D4 | PVD `volume_space_size.lsb * 0x800` equals (or is within one sector of) the image size. |
| D5 | PVD `volume_identifier` == `"RATCHETANDCLANK"` + 17 spaces (32 chars). |
| D6 | The root directory contains a file whose lowercased name is `system.cnf`. |
| D7 | That file's LBA is **289**. |
| D8 | `SYSTEM.CNF` contains `BOOT2 = cdrom0:\`, `VER = `, and `VMODE = `. |
| D9 | The boot ELF named by `BOOT2` exists in the root directory, and its first 4 bytes are `7F 45 4C 46`. |
| D10 | The boot ELF's lowercased name is one of: `scus_971.99`, `scus_972.09`, `scus_972.40`, `sces_509.16`, `sced_510.75`, `scps_150.37`. |
| D11 | `VMODE` is `PAL` iff the ELF name starts with `sces` or `sced`; `NTSC` otherwise. |
| D12 | The whole ISO9660 filesystem (PVD, path tables, all directory extents) ends before sector 1500. |
| D13 | `s32` at `1500 * 0x800 + 0` == `1`. |
| D14 | `s32` at `1500 * 0x800 + 4` == `0x2960`. |
| D15 | Feeding the `0x2960`-byte ToC blob to the WAD identifier yields `(RAC1, GLOBAL, "global")`. |
| D16 | The step-8 level scan finds **19** amalgamated headers on a retail build. Each has `s32 @ +4 == 0x2434`. |
| D17 | All 19 hits are reachable from the `levels` array at `0x28c8`; the set of sectors found by the scan equals the set of non-zero `levels[i].offset`. |
| D18 | Level `id` values (`s32 @ +0`) are distinct across the 19 headers. |
| D19 | For every level: `data.size > 0`, and `data.offset > 0`. |
| D20 | For every level: the derived level-group extent `[low, high)` lies entirely within the image, and `low > 1500` (level data comes after the ToC). |
| D21 | For every level: the three derived group extents (level, audio, scene) do not overlap each other. |
| D22 | For every level whose scene group exists: every non-zero `scenes[i].wads[j]` sector begins with the ASCII `WAD`. |
| D23 | For every level whose scene group exists: every non-zero `scenes[i].sounds[j]` sector begins with the ASCII `VAGp`. |
| D24 | For every level whose audio group exists: every non-zero `music[i]` sector begins with `VAGp`. |
| D25 | Every non-zero `qwark_boss_audio[i]` and `help_audio[i]` sector in the global header begins with `VAGp` (900 + 240 candidates). |
| D26 | Every non-empty `mpegs[i]` range (SectorByteRange) has `size_bytes > 0` and lies within the image. |
| D27 | Every non-empty `SectorRange` in the global header satisfies `offset + size <= image_size_in_sectors`. |
| D28 | Every entry whose asset is documented above as WAD-compressed begins with the ASCII `WAD` and its `compressed_size` fits inside the declared `SectorRange`. |
| D29 | `frontbin` decodes as a valid ratchet executable: every block's `entry_point` equals the first block's, and the sum of `(0x10 + copy_size)` over all accepted blocks is within the declared range. |
| D30 | Sum of `ceil(size/0x800)` over all distinct extracted extents is `<=` the image size in sectors (no double-counting bug). |

WAD-codec assertions.

| # | Assertion |
| --- | --- |
| W1 | For every WAD stream on the disc, decompression consumes exactly the declared `compressed_size` bytes (or terminates via a pad packet whose alignment skip lands at/past the end) without throwing. |
| W2 | No decompressed stream contains two adjacent literal packets. Assert on this while decoding; if it ever fires on real data, the format understanding is wrong. |
| W3 | Every match's source position satisfies `0 <= source < P`. |
| W4 | No far-match packet on a retail disc sets flag bit 3 (`A == 1`). If this fires, the ambiguity in section 3.3.1 must be resolved before trusting the output. |
| W5 | Every pad packet's target offset is a multiple of `0x2000` relative to the start of the packet stream (i.e. `0x1000` alignment is over-permissive). If this fires, use `0x1000`. |
| W6 | Round trip: `decompress(compress(x)) == x` for a corpus of decompressed assets extracted from the disc. |
| W7 | Recompressing a disc asset does **not** need to reproduce the original bytes; do not assert byte-identity against Insomniac's compressor output. |
| W8 | A zero-length little literal (`N == 0`) must consume no input bytes. |
| W9 | Self-overlapping matches (`P - source < match_len`) decode via a forward byte-by-byte copy, producing run-length expansion. Unit-test this explicitly; a `memcpy` implementation will silently produce wrong output. |
| W10 | The pad packet must not emit a little literal. |

Cross-checks against the sequels (useful to catch a mis-detected game).

| # | Assertion |
| --- | --- |
| X1 | If the ToC probe at sector 1500 fails but the probe at sector 1001 succeeds, the disc is not RAC1 — reject rather than mis-parse. |
| X2 | A disc where the root directory contains `RC2.HDR` or any `*.WAD` file is not retail RAC1. |
| X3 | A disc whose boot ELF does not start with `\x7fELF` is R&C3 or Deadlocked (packed executable), not RAC1. |
