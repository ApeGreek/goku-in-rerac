#pragma once
#include <string>
#include <vector>
#include "core/buffer.h"

namespace rc {

// Byte-based {offset, size} pair used inside the level data WAD. offset -1 / size 0 = absent.
struct ByteRange { s32 offset = -1; s32 size = 0; bool present() const { return offset >= 0 && size > 0; } };

// Header at the start of a RAC1 level data WAD (uncompressed container, 0x58 bytes).
// Spec: docs/formats/wad_layouts_rac1.md section 2.3.
struct LevelDataHeader {
    ByteRange overlay;
    ByteRange sound_bank;
    ByteRange core_index;
    ByteRange gs_ram;
    ByteRange hud_header;
    ByteRange hud_banks[5];
    ByteRange core_data;
};
static_assert(sizeof(LevelDataHeader) == 0x58);

LevelDataHeader read_level_data_header(Buffer data);

// Insomniac's "ratchet executable": bare [16-byte header][data] blocks, no
// file header. Used for level code overlays and the global frontbin.
// Spec: docs/formats/wad_layouts_rac1.md section 4.
struct OverlaySection {
    u32 dest_address = 0;
    u32 section_type = 0;  // ELF sh_type
    u32 entry_point = 0;
    std::vector<u8> data;
};

std::vector<OverlaySection> parse_ratchet_executable(Buffer bytes);

// Emits an ELF32 MIPS file for Ghidra from overlay sections. Names come from
// the RAC1 donor table when the layout matches (7 sections, types 1,8,1,1,1,1,1),
// otherwise sections are named .unknown_N. Returns the file bytes.
std::vector<u8> write_overlay_elf(const std::vector<OverlaySection>& sections, u32 e_flags);

} // namespace rc
