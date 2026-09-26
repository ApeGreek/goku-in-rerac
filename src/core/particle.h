#pragma once
#include <optional>
#include <vector>
#include "core/buffer.h"
#include "core/level_core.h"
#include "core/texture.h"

namespace rc {

// Particle textures, particle frame lists (part_defs) and FX textures of a level.
// Specs: docs/formats/textures_rac1.md 8, docs/plan/particles.md 6. Rust port: crates/rc-formats/src/particle_tex.rs.

// part_textures entry (core header +0x50/+0x54, index-relative), 0x10 bytes.
struct PartTextureEntry { s32 palette, unk4, texture, side; };   // offsets relative to part_bank_offset
static_assert(sizeof(PartTextureEntry) == 0x10);

// fx_textures entry (core header +0x58/+0x5c, index-relative), 0x10 bytes; all -1 = absent.
struct FxTextureEntry { s32 palette, texture, width, height; };  // offsets relative to fx_bank_offset
static_assert(sizeof(FxTextureEntry) == 0x10);

// part_defs (core header +0x6c, index-relative): {count, texture count, data_off, data_size},
// count s32 offsets (relative to part_defs, 0 = null), then data_size u8 part-texture indices at data_off.
struct PartDefs {
    s32 header[4] = {};
    std::vector<s32> offsets;
    std::vector<u8> blob;
    // ParseParticleTexs (level01 0x253648): def[type] = blob + (offset - data_off), or the blob start when 0.
    // Returns the index into blob, or -1 when out of range.
    s32 start(size_t type) const;
};

struct ParticleTextures {
    std::vector<PartTextureEntry> entries;
    std::vector<Image> textures;               // decoded entries (side x side RGBA8)
    PartDefs defs;
    std::vector<FxTextureEntry> fx_entries;
    std::vector<std::optional<Image>> fx_textures;
};

PartDefs parse_part_defs(Buffer core_index, s32 offset);
// core_index: the raw lump; part_bank / fx_bank: the core_data blocks at part_bank_offset / fx_bank_offset.
ParticleTextures parse_particle_textures(Buffer core_index, const LevelCoreHeader& h, Buffer part_bank, Buffer fx_bank);

} // namespace rc
