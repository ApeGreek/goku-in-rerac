#include "core/tie.h"
#include <algorithm>
#include <string>

namespace rc {

namespace {

[[noreturn]] void fail(const std::string& msg) { throw FormatError("tie packet: " + msg); }

// Resolves the stored vertices the way VU1 program 13507 (entry MSCAL 0, L9..L39) writes
// them into the GS packet: dinky vertices first, then fat, each written to gs_slot and,
// in a double-write phase, also to gs_slot_2. Phase ends are GS-slot markers in the
// unpack header (docs/formats/tie_rac1.md 3.4).
void resolve_vertices(TiePacket& pk) {
    const TieUnpackHeader& u = pk.unpack;
    const size_t D = pk.dinky.size(), F = pk.fat.size();
    auto find = [](auto& vs, size_t from, u16 slot) -> size_t {
        for (size_t i = from; i < vs.size(); i++) if (vs[i].gs_slot == slot) return i;
        return vs.size();
    };
    // Dinky: the single-write loop (L10) exits after storing the marker vertex; the three
    // vertices already in its pipeline are stored single-write too (L13 / L21).
    size_t d_single = 0, d_end = 0;
    if (D) {
        size_t i1 = find(pk.dinky, 0, u.dinky_single_end);
        if (i1 == D) fail("dinky single-write marker not found");
        d_single = i1 + 4;
        if (u.dinky_single_only) {
            d_end = d_single;
        } else {
            // Double-write loop (L14..L17) exits on its marker, then flushes two more (L18..L20).
            size_t i5 = find(pk.dinky, d_single, u.dinky_double_end);
            if (i5 == D) fail("dinky double-write marker not found");
            d_end = i5 + 3;
        }
        if (d_end != D) fail("dinky phase markers do not end at the last dinky vertex");
    }
    size_t f_single = 0;
    if (u.no_fat) {
        if (F) fail("fat vertices present with no_fat set");
    } else {
        if (!F) fail("no fat vertices but no_fat clear");
        // Fat: single-write loop (L29..L32) up to and including its marker, then the
        // double-write loop (L33..L39) up to and including the fat_double_end vertex.
        size_t j6 = find(pk.fat, 0, u.fat_single_end);
        if (j6 == F) fail("fat single-write marker not found");
        f_single = j6 + 1;
        size_t j7 = find(pk.fat, f_single, u.fat_double_end);
        if (j7 + 1 != F) fail("fat double-write marker is not the last fat vertex");
    }
    // Colour indices: one byte per dinky vertex, then one 4-byte element (c0, c1, c2, 0xff)
    // per fat vertex, starting at the next element boundary.
    size_t fat_base = (D + 3) / 4 * 4;
    if (fat_base + 4 * F > pk.colors.size() || D > pk.colors.size()) fail("colour indices too short");
    pk.vertices.clear();
    for (size_t i = 0; i < D; i++) {
        const TieDinkyVertex& v = pk.dinky[i];
        bool dbl = i >= d_single;
        if (dbl && v.gs_slot_2 == 0) fail("double-write dinky vertex with gs_slot_2 = 0");
        u8 c = pk.colors[i];
        pk.vertices.push_back(TieVertex{v.x, v.y, v.z, 0, 0, 0, v.s, v.t, v.q, v.gs_slot, u16(dbl ? v.gs_slot_2 : 0), c, {c, c}, 0});
    }
    for (size_t j = 0; j < F; j++) {
        const TieFatVertex& v = pk.fat[j];
        bool dbl = j >= f_single;
        if (dbl && v.gs_slot_2 == 0) fail("double-write fat vertex with gs_slot_2 = 0");
        const u8* c = &pk.colors[fat_base + 4 * j];
        pk.vertices.push_back(TieVertex{v.x, v.y, v.z, v.dx, v.dy, v.dz, v.s, v.t, v.q, v.gs_slot, u16(dbl ? v.gs_slot_2 : 0), c[0], {c[1], c[2]}, 1});
    }
}

// Walks the GS packet the way the GIF unit consumes it (docs/formats/tie_rac1.md 3.4):
// ad-gif blocks where VU1's L2 loop put them, strip GIF tags with NLOOP = vertex_count,
// EOP on the last strip.
void walk_gs_packet(TiePacket& pk) {
    // L1/L2: ad-gif 0 at slot 0, ad-gif k at ad_gif_dest[k-1] while that entry is > 0.
    std::vector<s32> adgif_pos{0};
    while (adgif_pos.size() <= 4 && pk.ad_gif_dest[adgif_pos.size() - 1] > 0) adgif_pos.push_back(pk.ad_gif_dest[adgif_pos.size() - 1]);
    if (adgif_pos.size() > 4) fail("ad_gif_dest[3] > 0 (VU1 L2 would never terminate)");
    if (adgif_pos.size() != pk.header.shader_count) fail("placed ad-gif count != shader_count");
    for (size_t k = 0; k < adgif_pos.size(); k++) {
        if (pk.ad_gif_src[k] < 0 || pk.ad_gif_src[k] % 0x50) fail("ad_gif_src not a multiple of 0x50");
    }
    // Slot -> vertex index, last writer in VU processing order wins.
    std::vector<s32> slot(0x400, -1);
    std::vector<u8> consumed(0x400, 0);
    for (size_t i = 0; i < pk.vertices.size(); i++) {
        const TieVertex& v = pk.vertices[i];
        if (v.gs_slot >= slot.size() || v.gs_slot_2 >= slot.size()) fail("GS slot out of range");
        slot[v.gs_slot] = s32(i);
        if (v.gs_slot_2) slot[v.gs_slot_2] = s32(i);
    }
    size_t cursor = 0, next_ad = 0, si = 0;
    u8 material = 0;
    pk.draws.clear();
    while (si < pk.strips.size()) {
        if (next_ad < adgif_pos.size() && size_t(adgif_pos[next_ad]) == cursor) {
            material = u8(pk.ad_gif_src[next_ad] / 0x50);
            next_ad++;
            cursor += 6;
        } else if (pk.strips[si].gif_tag_offset == cursor) {
            TieDraw d;
            d.ad_gif = material;
            d.winding = pk.strips[si].winding;
            for (u32 n = 0; n < pk.strips[si].vertex_count; n++) {
                size_t s = cursor + 1 + 3 * n;
                if (s >= slot.size() || slot[s] < 0) fail("strip reads unwritten GS slot " + std::to_string(s));
                consumed[s] = 1;
                d.vertices.push_back(u16(slot[s]));
            }
            cursor += 1 + 3 * size_t(pk.strips[si].vertex_count);
            pk.draws.push_back(std::move(d));
            si++;
        } else {
            fail("no GIF tag at GS packet offset " + std::to_string(cursor));
        }
    }
    if (next_ad != adgif_pos.size()) fail("ad-gif block after the last strip");
    for (const TieVertex& v : pk.vertices) {
        if (!consumed[v.gs_slot] || (v.gs_slot_2 && !consumed[v.gs_slot_2])) fail("vertex written to a GS slot no strip reads");
    }
}

TiePacket read_packet(Buffer blob, size_t lod_table, const TiePacketHeader& ph) {
    TiePacket pk;
    pk.header = ph;
    size_t base = lod_table + size_t(ph.data);
    for (int i = 0; i < 4; i++) pk.ad_gif_dest[i] = blob.read<s32>(base + size_t(i) * 4, "tie ad_gif_dest");
    for (int i = 0; i < 4; i++) pk.ad_gif_src[i] = blob.read<s32>(base + 0x10 + size_t(i) * 4, "tie ad_gif_src");
    pk.unpack = blob.read<TieUnpackHeader>(base + 0x20, "tie unpack header");
    pk.strips = blob.read_multiple<TieStrip>(base + 0x2c, pk.unpack.strip_count, "tie strips");
    if (size_t(ph.control_size) * 16 < 12 + 4 * pk.strips.size()) fail("control region smaller than unpack header + strips");

    size_t vert_start = base + size_t(ph.vert_ofs) * 0x10;
    size_t D = pk.unpack.dinky_count, F = pk.unpack.fat_count;
    if (D * sizeof(TieDinkyVertex) + F * sizeof(TieFatVertex) > size_t(ph.vert_size) * 0x10) fail("vertices overrun vert_size");
    pk.dinky = blob.read_multiple<TieDinkyVertex>(vert_start, D, "tie dinky vertices");
    pk.fat = blob.read_multiple<TieFatVertex>(vert_start + D * sizeof(TieDinkyVertex), F, "tie fat vertices");

    size_t color_start = base + size_t(ph.color_ofs) * 0x10;
    size_t color_bytes = size_t(ph.color_count) * 4;
    size_t color_qw = (size_t(ph.color_count) + 3) / 4;
    pk.colors = blob.copy(color_start, color_bytes);
    pk.colors_b = blob.copy(color_start + color_qw * 0x10, color_bytes);
    pk.slot_table = blob.copy(base + size_t(ph.slot_table_ofs) * 0x10, size_t(ph.slot_table_size) * 0x10);

    resolve_vertices(pk);
    walk_gs_packet(pk);
    return pk;
}

} // namespace

TieClass parse_tie_class(Buffer blob) {
    TieClass tc;
    tc.header = blob.read<TieClassHeader>(0, "tie class header");
    const TieClassHeader& h = tc.header;
    size_t first = blob.size();
    for (int lod = 0; lod < 3; lod++) if (h.packet_count[lod] && h.packets[lod] > 0) first = std::min(first, size_t(h.packets[lod]));
    if (first < sizeof(TieClassHeader)) throw FormatError("tie packet table overlaps the class header");
    tc.header_ext = blob.copy(sizeof(TieClassHeader), first - sizeof(TieClassHeader));
    tc.normals = blob.read_multiple<s16>(h.normals, 64 * 4, "tie normals");
    for (int lod = 0; lod < 3; lod++) {
        if (h.packet_count[lod] == 0) continue;
        if (h.packets[lod] <= 0) throw FormatError("tie LOD with packets but no packet table");
        std::vector<TiePacketHeader> phs = blob.read_multiple<TiePacketHeader>(size_t(h.packets[lod]), h.packet_count[lod], "tie packet headers");
        for (const auto& ph : phs) tc.lods[lod].push_back(read_packet(blob, size_t(h.packets[lod]), ph));
    }
    if (h.texture_count) tc.ad_gifs = blob.read_multiple<TieAdGifs>(h.ad_gif_ofs, h.texture_count, "tie ad-gifs");
    return tc;
}

std::vector<TieTriangle> tie_triangles(const TiePacket& p) {
    std::vector<TieTriangle> out;
    for (const TieDraw& d : p.draws) {
        size_t parity = d.winding ? 1 : 0;
        for (size_t i = 2; i < d.vertices.size(); i++) {
            if (i % 2 == parity) out.push_back({d.vertices[i - 2], d.vertices[i - 1], d.vertices[i], d.ad_gif});
            else out.push_back({d.vertices[i], d.vertices[i - 1], d.vertices[i - 2], d.ad_gif});
        }
    }
    return out;
}

std::vector<TieInstance> parse_tie_instances(Buffer section) {
    s32 count = section.read<s32>(0, "tie instance count");
    if (count < 0 || count > 100000) throw FormatError("implausible tie instance count");
    return section.read_multiple<TieInstance>(0x10, size_t(count), "tie instances");
}

} // namespace rc
