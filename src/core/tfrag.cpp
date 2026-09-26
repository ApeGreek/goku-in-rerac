#include "core/tfrag.h"
#include "core/vif.h"
#include <cstring>

namespace rc {

namespace {

template <typename T>
std::vector<T> as_records(std::span<const u8> data, u32 count) {
    std::vector<T> v(count);
    if (data.size() < size_t(count) * sizeof(T)) throw FormatError("unpack payload smaller than element count");
    if (count) std::memcpy(v.data(), data.data(), size_t(count) * sizeof(T));
    return v;
}

struct ListWalk {
    Tfrag& t;
    bool have_origin = false;
    void run(std::span<const u8> list) {
        if (list.empty()) return;
        for (const VifPacket& p : parse_vif(list)) {
            if (p.cmd == VIF_STROW) {
                s32 row[4];
                std::memcpy(row, p.data.data(), 16);
                // The tfrag origin row precedes every V3_16 position unpack; remember the last non-index row.
                if (row[0] != 0x45000000 && u32(row[0]) != t.vu.vertex_info_common_addr) {
                    std::memcpy(t.origin, row, 16);
                    have_origin = true;
                }
                continue;
            }
            if (!p.is_unpack()) continue;
            u16 a = p.addr();
            u32 n = p.count();
            const TfragVuHeader& vu = t.vu;
            if (p.vn() == 3 && p.vl() == 1 && p.usn() && a == 0) {
                std::memcpy(&t.vu, p.data.data(), sizeof(TfragVuHeader));
            } else if (p.vn() == 3 && p.vl() == 0) {
                t.ad_gifs = as_records<TfragAdGifs>(p.data, n / 5);
            } else if (p.vn() == 2 && p.vl() == 1) {
                auto pos = as_records<TfragPosition>(p.data, n);
                if (a == vu.positions_common_addr) t.positions_common = n;
                else if (n && t.positions_lod01 == 0 && a != vu.positions_common_addr && t.positions_lod0 == 0 && a == vu.positions_common_addr + 2 * t.positions_common) t.positions_lod01 = n;
                else t.positions_lod0 = n;
                t.positions.insert(t.positions.end(), pos.begin(), pos.end());
            } else if (p.vn() == 3 && p.vl() == 1) {
                auto vi = as_records<TfragVertexInfo>(p.data, n);
                if (a == vu.vertex_info_common_addr) t.vinfo_common = n;
                else if (a == vu.vertex_info_lod_01_addr) t.vinfo_lod01 = n;
                else t.vinfo_lod0 = n;
                t.vertex_info.insert(t.vertex_info.end(), vi.begin(), vi.end());
            } else if (p.vn() == 3 && p.vl() == 2) {
                std::vector<u8> bytes(p.data.begin(), p.data.begin() + size_t(n) * 4);
                if (!p.usn() && a == vu.strips_addr) {
                    current_strips = as_records<TfragStrip>(p.data, n);
                } else if (a == vu.indices_addr) {
                    current_indices = bytes;
                }
                // An empty region shares its VU address with the next one (e.g. with no LOD-01
                // "unknown indices 2", unk_indices_2_lod_01_addr == parent_indices_lod_0_addr in
                // ~90% of retail tfrags), so only match regions whose VU-header count is non-zero.
                else if (a == vu.parent_indices_lod_01_addr && vu.positions_lod_01_count) t.parent_indices_lod01 = bytes;
                else if (a == vu.unk_indices_2_lod_01_addr && vu.unk_06) t.unk_indices_2_lod01 = bytes;
                else if (a == vu.parent_indices_lod_0_addr && vu.positions_lod_0_count) t.parent_indices_lod0 = bytes;
                else if (a == vu.unk_indices_2_lod_0_addr && vu.unk_0a) t.unk_indices_2_lod0 = bytes;
                else throw FormatError("unexpected V4_8 unpack address in tfrag list");
            } else {
                throw FormatError("unexpected unpack format in tfrag list");
            }
        }
    }
    std::vector<TfragStrip> current_strips;
    std::vector<u8> current_indices;
    void take_lod(int lod) {
        t.lod[lod].strips = std::move(current_strips);
        t.lod[lod].indices = std::move(current_indices);
        current_strips.clear();
        current_indices.clear();
    }
};

} // namespace

std::vector<Tfrag> parse_tfrags(Buffer block) {
    TfragBlockHeader bh = block.read<TfragBlockHeader>(0, "tfrag block header");
    if (bh.tfrag_count < 0 || bh.tfrag_count > 100000 || bh.table_offset < 0x10) throw FormatError("implausible tfrag block header");
    std::vector<Tfrag> out;
    out.reserve(size_t(bh.tfrag_count));
    for (s32 i = 0; i < bh.tfrag_count; i++) {
        Tfrag t{};
        t.header = block.read<TfragHeader>(size_t(bh.table_offset) + size_t(i) * sizeof(TfragHeader), "tfrag header");
        const TfragHeader& h = t.header;
        size_t base = size_t(bh.table_offset) + size_t(h.data);
        auto slice = [&](size_t from, size_t to) { return block.sub(base + from, to - from).bytes(); };
        size_t lod0_start = size_t(h.shared_ofs) + size_t(h.lod_1_size) * 0x10;

        ListWalk w{t};
        // The common list carries the VU header; parse it first so addresses are known.
        w.run(slice(h.shared_ofs, h.lod_1_ofs));
        // Then the LOD lists in transfer order.
        Tfrag fresh{};
        fresh.header = t.header;
        fresh.vu = t.vu;
        ListWalk w2{fresh};
        w2.run(slice(h.shared_ofs, h.lod_1_ofs));        // common: header, ad-gifs, common vinfo, origin, common positions
        w2.run(slice(h.lod_2_ofs, h.shared_ofs));        // LOD 2 strips/indices
        w2.take_lod(2);
        w2.run(slice(h.lod_1_ofs, h.lod_0_ofs));         // LOD 1 strips/indices
        w2.take_lod(1);
        w2.run(slice(h.lod_0_ofs, lod0_start));          // LOD 0+1 parents, vinfo, positions
        w2.run(slice(lod0_start, h.rgba_ofs));           // LOD 0 positions, strips, indices, parents, vinfo
        w2.take_lod(0);
        t = std::move(fresh);

        t.rgba = block.read_multiple<TfragRgba>(base + h.rgba_ofs, size_t(h.rgba_size) * 4, "tfrag rgba");
        s32 origin2[4];
        std::memcpy(origin2, block.sub(base + h.light_ofs, 16).data(), 16);
        if (!w2.have_origin) std::memcpy(t.origin, origin2, 16);
        t.lights = block.read_multiple<TfragLight>(base + h.light_ofs + 0x10, h.vert_count, "tfrag lights");
        out.push_back(std::move(t));
    }
    return out;
}

// Texture assignment follows the VU1 strip processor (program 55907, EE 0x103578: L84/L93 for the first
// record, L79-L82 / L122-L125 for the rest). The first record always loads the ad-gif at z and is unbiased.
// Later: x > 0 plain run; x == 0 end; x < 0 run of x + 128 that loads the ad-gif at z when y >= 0, or XGKICKs
// and then loads the ad-gif at z when z >= 0 (L81 `ibgez vi13, L82`); a kick with z < 0 keeps the texture.
std::vector<TfragTriangle> tfrag_triangles(const Tfrag& t, int lod) {
    std::vector<TfragTriangle> tris;
    const TfragLod& L = t.lod[lod];
    size_t cursor = 0;
    u16 ad_gif = 0;
    for (size_t k = 0; k < L.strips.size(); k++) {
        const TfragStrip& s = L.strips[k];
        int n = s.vertex_count_and_flag;
        if (k == 0 || n <= 0) {
            if (k != 0 && n == 0) break;
            if (k == 0 || s.end_of_packet_flag >= 0 || s.ad_gif_offset >= 0) {
                int z = s.ad_gif_offset;
                if (z < 0 || z % 5 != 0 || size_t(z / 5) >= t.ad_gifs.size()) throw FormatError("strip ad-gif offset outside the ad-gif array");
                ad_gif = u16(z / 5);
            }
            n += 128;
        }
        if (cursor + size_t(n) > L.indices.size()) throw FormatError("strip runs past index array");
        for (int i = 2; i < n; i++) {
            u16 a = L.indices[cursor + size_t(i) - 2], b = L.indices[cursor + size_t(i) - 1], c = L.indices[cursor + size_t(i)];
            if (i & 1) tris.push_back({b, a, c, ad_gif}); else tris.push_back({a, b, c, ad_gif});
        }
        cursor += size_t(n);
    }
    return tris;
}

} // namespace rc
