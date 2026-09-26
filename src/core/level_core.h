#pragma once
#include <map>
#include <string>
#include <vector>
#include "core/buffer.h"

namespace rc {

// {count, offset} pair used in the level core index (offsets relative to the index).
struct ArrayRange { s32 count = 0; s32 offset = 0; };

// LevelCoreHeader: master index for a level's art, at byte 0 of the core_index
// lump. Field-by-field description in docs/formats/wad_layouts_rac1.md 2.4.
// "index" offsets are relative to core_index; "data" offsets are relative to
// the decompressed core_data blob.
struct LevelCoreHeader {
    ArrayRange gs_ram;             // index
    s32 tfrags;                    // data
    s32 occlusion;                 // data
    s32 sky;                       // data
    s32 collision;                 // data
    ArrayRange moby_classes;       // index
    ArrayRange tie_classes;        // index
    ArrayRange shrub_classes;      // index
    ArrayRange tfrag_textures;     // index
    ArrayRange moby_textures;      // index
    ArrayRange tie_textures;       // index
    ArrayRange shrub_textures;     // index
    ArrayRange part_textures;      // index
    ArrayRange fx_textures;        // index
    s32 textures_base_offset;      // data
    s32 part_bank_offset;          // data
    s32 fx_bank_offset;            // data
    s32 part_defs_offset;          // index
    s32 sound_remap_offset;        // index
    s32 unknown_74;
    s32 ratchet_seqs;              // index: 256 x s32 data offsets
    s32 scene_view_size;           // data
    s32 gadget_count;
    s32 gadget_offset;             // index
    s32 assets_compressed_size;
    s32 assets_decompressed_size;
    s32 chrome_map_texture;
    s32 chrome_map_palette;
    s32 glass_map_texture;
    s32 glass_map_palette;
    s32 unknown_a0;
    s32 heightmap_offset;
    s32 occlusion_oct_offset;
    s32 moby_gs_stash_list;
    s32 occlusion_rad_offset;
    s32 moby_sound_remap_offset;
    s32 occlusion_rad2_offset;
};
static_assert(sizeof(LevelCoreHeader) == 0xbc);

struct GsRamEntry { s32 psm; s16 width; s16 height; s32 address; s32 offset; };
static_assert(sizeof(GsRamEntry) == 0x10);

struct ClassEntry {                // moby and tie class tables (0x20 bytes)
    s32 offset_in_asset_wad;       // data; 0 = no geometry
    s32 o_class;
    s32 unknown_8;
    s32 unknown_c;
    u8 textures[16];
};
static_assert(sizeof(ClassEntry) == 0x20);

struct ShrubBillboardInfo { s16 width, height, max_mip, palette_offset, texture_offset, mip1, mip2, mip3; };
struct ShrubClassEntry { ClassEntry base; ShrubBillboardInfo billboard; };
static_assert(sizeof(ShrubClassEntry) == 0x30);

struct TextureEntry { s32 data_offset; s16 width, height, type, palette, mipmap, pad; };
static_assert(sizeof(TextureEntry) == 0x10);

struct GadgetEntry { s32 offset_in_asset_wad; s32 class_number; s32 compressed_size; s32 pad; };
static_assert(sizeof(GadgetEntry) == 0x10);

// A named byte range inside the decompressed core_data blob.
struct CoreBlock {
    std::string name;   // e.g. "tfrags", "moby_class/0123", "ratchet_seq/017"
    s32 offset = 0;
    s32 size = 0;
};

struct LevelCore {
    LevelCoreHeader header;
    std::vector<GsRamEntry> gs_ram;
    std::vector<ClassEntry> moby_classes, tie_classes;
    std::vector<ShrubClassEntry> shrub_classes;
    std::vector<TextureEntry> tfrag_textures, moby_textures, tie_textures, shrub_textures;
    std::vector<GadgetEntry> gadgets;
    std::vector<s32> ratchet_seqs;   // 256 entries, 0 = unused, or empty if absent
    std::vector<CoreBlock> blocks;   // every block in core_data with its size from the boundary algorithm
};

LevelCore parse_level_core(Buffer core_index, size_t core_data_size);

} // namespace rc
