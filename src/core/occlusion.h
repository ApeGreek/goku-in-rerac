#pragma once
#include <vector>
#include "core/buffer.h"

namespace rc {

// Precomputed occlusion (potentially-visible sets). Spec: docs/formats/occlusion_rac1.md;
// per-frame rule: docs/plan/occlusion_culling.md. Addresses are the level01 overlay's.

constexpr size_t OCCL_MASK_BYTES = 0x80;      // 1024 visibility bits per mask
constexpr u16 OCCL_ALWAYS_VISIBLE = 0x7f80;   // byte 0x7f, bit 0x80 = bit 1023, forced on every frame
constexpr size_t GAMEPLAY_OCCLUSION_MAPPINGS = 0x8c;  // gameplay pointer (FUN_00255958: piVar27[0x23])

// One populated 4x4x4-unit cell of the grid tree (core occlusion block). Coordinates are
// trunc(world * 0.25) as BuildOcclVisibility (0x219008) computes them.
struct OcclusionCell { u16 x, y, z, mask; };
static_assert(sizeof(OcclusionCell) == 8);

struct OcclusionGrid {
    s32 masks_offset = 0;             // block +0x00: byte offset of the mask array
    u16 z_base = 0, z_count = 0;      // block +0x04 / +0x06
    std::vector<OcclusionCell> cells; // tree order: z, then y, then x
    u32 mask_count = 0;               // highest referenced mask index + 1
    std::vector<u8> masks;            // mask_count * 0x80
};

// Walks the tree like ParseOcclGrid (level01 0x218e78). Returns the mask index or -1.
s32 occlusion_lookup(Buffer block, s32 x, s32 y, s32 z);
OcclusionGrid parse_occlusion_grid(Buffer block);

// Gameplay occlusion mappings (pointer 0x8c; the level WAD's `occlusion` lump is a copy).
struct OcclusionMapping { s32 bit_index; s32 occlusion_id; };
struct OcclusionMappings { std::vector<OcclusionMapping> tfrag, tie, moby; };
OcclusionMappings parse_occlusion_mappings(Buffer section);

// Load-time resolution (FUN_00255958): each object's u16 = (byte << 8) | bit mask.
inline u16 occl_bits(s32 bit) { return u16(((bit >> 3) << 8) | (1 << (bit & 7))); }
inline bool occl_visible(u16 bits, const u8* frame_mask) { return (frame_mask[bits >> 8] & (bits & 0xff)) != 0; }

// tfrags: positional, only when the counts match and every tfrag header byte 0x3d equals the
// mapping's occlusion_id; otherwise every tfrag is always visible ("occlusion out of date on tfrag").
std::vector<u16> resolve_tfrag_occlusion(const OcclusionMappings& m, const std::vector<u8>& header_3d, bool* out_of_date);
// ties: positional when the counts match and every (s16) occlusion_index equals (s16) id, else a
// first-match search on the u16 occlusion_index; not found = always visible.
std::vector<u16> resolve_tie_occlusion(const OcclusionMappings& m, const std::vector<s32>& occlusion_index, size_t* not_found);
// mobys: only instances with gameplay `occlusion` == 0 take part; first match of id == (s16) spawn id
// (instance +0x0c); not found or not taking part = always visible.
struct MobyOcclusionKey { s32 occlusion; s32 spawn_id; };
std::vector<u16> resolve_moby_occlusion(const OcclusionMappings& m, const std::vector<MobyOcclusionKey>& mobys, size_t* not_found);

} // namespace rc
