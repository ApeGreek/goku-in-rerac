#include "core/level_core.h"
#include <algorithm>
#include <cstring>
#include <set>

namespace rc {

template <typename T>
static std::vector<T> read_table(Buffer b, ArrayRange r, const char* what) {
    if (r.count <= 0 || r.offset <= 0) return {};
    if (r.count > 100000) throw FormatError(std::string("implausible count for ") + what);
    return b.read_multiple<T>(size_t(r.offset), size_t(r.count), what);
}

LevelCore parse_level_core(Buffer idx, size_t data_size) {
    LevelCore c;
    if (idx.size() < sizeof(LevelCoreHeader)) throw FormatError("core index too small");
    std::memcpy(&c.header, idx.data(), sizeof c.header);
    const LevelCoreHeader& h = c.header;

    c.gs_ram = read_table<GsRamEntry>(idx, h.gs_ram, "gs_ram table");
    c.moby_classes = read_table<ClassEntry>(idx, h.moby_classes, "moby class table");
    c.tie_classes = read_table<ClassEntry>(idx, h.tie_classes, "tie class table");
    c.shrub_classes = read_table<ShrubClassEntry>(idx, h.shrub_classes, "shrub class table");
    c.tfrag_textures = read_table<TextureEntry>(idx, h.tfrag_textures, "tfrag textures");
    c.moby_textures = read_table<TextureEntry>(idx, h.moby_textures, "moby textures");
    c.tie_textures = read_table<TextureEntry>(idx, h.tie_textures, "tie textures");
    c.shrub_textures = read_table<TextureEntry>(idx, h.shrub_textures, "shrub textures");
    if (h.gadget_count > 0 && h.gadget_offset > 0) {
        c.gadgets = idx.read_multiple<GadgetEntry>(size_t(h.gadget_offset), size_t(h.gadget_count), "gadget table");
    }
    if (h.ratchet_seqs > 0) c.ratchet_seqs = idx.read_multiple<s32>(size_t(h.ratchet_seqs), 256, "ratchet seq table");

    // Boundary algorithm (spec 2.5): every known start offset is a boundary; a
    // block extends to the next larger boundary, with the decompressed size as sentinel.
    std::set<s32> bounds;
    auto add = [&](s32 v) { if (v > 0) bounds.insert(v); };
    add(h.tfrags); add(h.occlusion); add(h.sky); add(h.collision); add(h.textures_base_offset);
    add(h.part_bank_offset); add(h.fx_bank_offset);
    // moby_sound_remap_offset (0xb4) is 0x9c00 in every RAC1 level and is not a data offset here; not a boundary.
    for (const auto& e : c.moby_classes) add(e.offset_in_asset_wad);
    for (const auto& e : c.tie_classes) add(e.offset_in_asset_wad);
    for (const auto& e : c.shrub_classes) add(e.base.offset_in_asset_wad);
    for (s32 v : c.ratchet_seqs) add(v);
    for (const auto& g : c.gadgets) add(g.offset_in_asset_wad);
    s32 end_sentinel = h.assets_decompressed_size > 0 ? h.assets_decompressed_size : s32(data_size);
    bounds.insert(end_sentinel);

    auto block = [&](const std::string& name, s32 start, bool allow_zero = false) {
        if (start < 0 || (start == 0 && !allow_zero)) return;
        auto it = bounds.upper_bound(start);
        s32 end = it == bounds.end() ? end_sentinel : *it;
        c.blocks.push_back({name, start, end - start});
    };
    // tfrags open the data region (offset 0) and run to the first of occlusion/sky/collision (spec 2.5).
    {
        s32 tf_end = h.occlusion > 0 ? h.occlusion : h.sky > 0 ? h.sky : h.collision;
        if (tf_end > h.tfrags) c.blocks.push_back({"tfrags", h.tfrags, tf_end - h.tfrags});
    }
    block("occlusion", h.occlusion);
    block("sky", h.sky);
    block("collision", h.collision);
    block("textures", h.textures_base_offset);
    block("part_bank", h.part_bank_offset);
    block("fx_bank", h.fx_bank_offset);
    char buf[64];
    for (const auto& e : c.moby_classes) { std::snprintf(buf, sizeof buf, "moby_class/%04d", e.o_class); block(buf, e.offset_in_asset_wad); }
    for (const auto& e : c.tie_classes) { std::snprintf(buf, sizeof buf, "tie_class/%04d", e.o_class); block(buf, e.offset_in_asset_wad); }
    for (const auto& e : c.shrub_classes) { std::snprintf(buf, sizeof buf, "shrub_class/%04d", e.base.o_class); block(buf, e.base.offset_in_asset_wad); }
    for (size_t i = 0; i < c.ratchet_seqs.size(); i++) { std::snprintf(buf, sizeof buf, "ratchet_seq/%03zu", i); block(buf, c.ratchet_seqs[i]); }
    for (const auto& g : c.gadgets) { std::snprintf(buf, sizeof buf, "gadget/%04d", g.class_number); block(buf, g.offset_in_asset_wad); }
    return c;
}

} // namespace rc
