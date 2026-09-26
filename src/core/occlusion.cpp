#include "core/occlusion.h"
#include <algorithm>

namespace rc {

s32 occlusion_lookup(Buffer b, s32 x, s32 y, s32 z) {
    // ParseOcclGrid: every node offset is a u16 in 4-byte units from the block start.
    z -= s32(b.read<u16>(4));
    if (z < 0 || z >= s32(b.read<u16>(6))) return -1;
    u16 zo = b.read<u16>(8 + size_t(z) * 2);
    if (zo == 0) return -1;
    size_t ynode = size_t(zo) * 4;
    y -= s32(b.read<u16>(ynode));
    if (y < 0 || y >= s32(b.read<u16>(ynode + 2))) return -1;
    u16 yo = b.read<u16>(ynode + 4 + size_t(y) * 2);
    if (yo == 0) return -1;
    size_t xnode = size_t(yo) * 4;
    x -= s32(b.read<u16>(xnode));
    if (x < 0 || x >= s32(b.read<u16>(xnode + 2))) return -1;
    u16 m = b.read<u16>(xnode + 4 + size_t(x) * 2);
    return m == 0xffff ? -1 : s32(m);
}

OcclusionGrid parse_occlusion_grid(Buffer b) {
    OcclusionGrid g;
    g.masks_offset = b.read<s32>(0);
    g.z_base = b.read<u16>(4);
    g.z_count = b.read<u16>(6);
    if (g.masks_offset <= 8 || size_t(g.masks_offset) > b.size()) throw FormatError("occlusion: bad mask offset");
    s32 max_mask = -1;
    for (u16 zi = 0; zi < g.z_count; zi++) {
        u16 zo = b.read<u16>(8 + size_t(zi) * 2);
        if (zo == 0) continue;
        size_t ynode = size_t(zo) * 4;
        u16 y_base = b.read<u16>(ynode), y_count = b.read<u16>(ynode + 2);
        for (u16 yi = 0; yi < y_count; yi++) {
            u16 yo = b.read<u16>(ynode + 4 + size_t(yi) * 2);
            if (yo == 0) continue;
            size_t xnode = size_t(yo) * 4;
            u16 x_base = b.read<u16>(xnode), x_count = b.read<u16>(xnode + 2);
            for (u16 xi = 0; xi < x_count; xi++) {
                u16 m = b.read<u16>(xnode + 4 + size_t(xi) * 2);
                if (m == 0xffff) continue;
                g.cells.push_back({u16(x_base + xi), u16(y_base + yi), u16(g.z_base + zi), m});
                max_mask = std::max(max_mask, s32(m));
            }
        }
    }
    g.mask_count = u32(max_mask + 1);
    g.masks = b.copy(size_t(g.masks_offset), size_t(g.mask_count) * OCCL_MASK_BYTES);
    return g;
}

OcclusionMappings parse_occlusion_mappings(Buffer s) {
    s32 nt = s.read<s32>(0), ni = s.read<s32>(4), nm = s.read<s32>(8);
    if (nt < 0 || ni < 0 || nm < 0 || nt + ni + nm > 100000) throw FormatError("occlusion mappings: bad counts");
    OcclusionMappings m;
    m.tfrag = s.read_multiple<OcclusionMapping>(0x10, size_t(nt), "tfrag mappings");
    m.tie = s.read_multiple<OcclusionMapping>(0x10 + size_t(nt) * 8, size_t(ni), "tie mappings");
    m.moby = s.read_multiple<OcclusionMapping>(0x10 + size_t(nt + ni) * 8, size_t(nm), "moby mappings");
    return m;
}

std::vector<u16> resolve_tfrag_occlusion(const OcclusionMappings& m, const std::vector<u8>& h3d, bool* out_of_date) {
    bool stale = m.tfrag.size() != h3d.size();
    for (size_t i = 0; !stale && i < h3d.size(); i++)
        if (u32(h3d[i]) != u32(m.tfrag[i].occlusion_id)) stale = true;
    if (out_of_date) *out_of_date = stale;
    std::vector<u16> out(h3d.size(), OCCL_ALWAYS_VISIBLE);
    if (!stale) for (size_t i = 0; i < h3d.size(); i++) out[i] = occl_bits(m.tfrag[i].bit_index);
    return out;
}

std::vector<u16> resolve_tie_occlusion(const OcclusionMappings& m, const std::vector<s32>& occl, size_t* not_found) {
    std::vector<u16> out(occl.size(), OCCL_ALWAYS_VISIBLE);
    bool positional = m.tie.size() == occl.size();
    for (size_t i = 0; positional && i < occl.size(); i++)
        if (s16(occl[i]) != s16(m.tie[i].occlusion_id)) positional = false;
    size_t missing = 0;
    if (positional) {
        for (size_t i = 0; i < occl.size(); i++) out[i] = occl_bits(m.tie[i].bit_index);
    } else {
        for (size_t i = 0; i < occl.size(); i++) {
            u32 key = u16(occl[i]);
            auto it = std::find_if(m.tie.begin(), m.tie.end(), [&](const OcclusionMapping& r) { return u32(r.occlusion_id) == key; });
            if (it == m.tie.end()) missing++;
            else out[i] = occl_bits(it->bit_index);
        }
    }
    if (not_found) *not_found = missing;
    return out;
}

std::vector<u16> resolve_moby_occlusion(const OcclusionMappings& m, const std::vector<MobyOcclusionKey>& mobys, size_t* not_found) {
    std::vector<u16> out(mobys.size(), OCCL_ALWAYS_VISIBLE);
    size_t missing = 0;
    for (size_t i = 0; i < mobys.size(); i++) {
        if (mobys[i].occlusion != 0) continue;
        s32 key = s16(mobys[i].spawn_id);
        auto it = std::find_if(m.moby.begin(), m.moby.end(), [&](const OcclusionMapping& r) { return r.occlusion_id == key; });
        if (it == m.moby.end()) missing++;
        else out[i] = occl_bits(it->bit_index);
    }
    if (not_found) *not_found = missing;
    return out;
}

} // namespace rc
