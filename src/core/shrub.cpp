#include "core/shrub.h"
#include "core/vif.h"
#include <array>
#include <cstring>
#include <string>

namespace rc {

namespace {

constexpr size_t INPUT_QWC = 0x76;    // VU1 input buffer (TOPS-relative, double buffered at 0x02 / 0x78)
constexpr size_t OUTPUT_QWC = 0xa8;   // one of the three GS-packet buffers at 0x208 / 0x2b0 / 0x358

[[noreturn]] void fail(const std::string& msg) { throw FormatError("shrub packet: " + msg); }

// The VU1 input buffer after the packet's UNPACKs (32-bit lanes, as VU memory holds them).
struct InputBuffer {
    std::array<std::array<u32, 4>, INPUT_QWC> qw{};
    std::array<bool, INPUT_QWC> written{};

    const std::array<u32, 4>& at(size_t a, const char* what) const {
        if (a >= INPUT_QWC) fail(std::string(what) + " outside the input buffer");
        if (!written[a]) fail(std::string(what) + " reads an input quadword no unpack wrote");
        return qw[a];
    }
    template <typename T> T bytes(size_t a, const char* what) const {   // 16-byte record from one quadword
        static_assert(sizeof(T) == 16);
        const auto& q = at(a, what);
        T out;
        std::memcpy(&out, q.data(), 16);
        return out;
    }
    // Four 32-bit lanes truncated to s16 (the V4_16 view of a vertex quadword).
    std::array<s16, 4> halves(size_t a, const char* what) const {
        const auto& q = at(a, what);
        return {s16(q[0]), s16(q[1]), s16(q[2]), s16(q[3])};
    }
};

InputBuffer run_vif(std::span<const u8> list) {
    InputBuffer buf;
    int unpacks = 0;
    for (const VifPacket& c : parse_vif(list)) {
        if (c.is_unpack()) {
            unpacks++;
            bool v4_32 = c.vn() == 3 && c.vl() == 0, v4_16 = c.vn() == 3 && c.vl() == 1;
            if (!v4_32 && !v4_16) fail("unpack is neither V4_32 nor V4_16");
            if (!(c.imm & 0x8000)) fail("unpack without FLG (not TOPS-relative)");
            if (size_t(c.addr()) + c.count() > INPUT_QWC) fail("unpack runs past the 0x76-qw input buffer");
            for (u32 i = 0; i < c.count(); i++) {
                auto& q = buf.qw[c.addr() + i];
                for (int k = 0; k < 4; k++) {
                    if (v4_32) std::memcpy(&q[k], c.data.data() + i * 16 + k * 4, 4);
                    else {
                        u16 h;
                        std::memcpy(&h, c.data.data() + i * 8 + k * 2, 2);
                        q[k] = c.usn() ? u32(h) : u32(s32(s16(h)));
                    }
                }
                buf.written[c.addr() + i] = true;
            }
        } else if (c.cmd == VIF_STCYCL) {
            if ((c.imm & 0xff) != (c.imm >> 8)) fail("STCYCL with CL != WL");
        } else if (c.cmd == VIF_STMOD) {
            if (c.imm & 3) fail("STMOD mode != 0");
        } else if (c.cmd != VIF_NOP) {
            fail("unexpected VIF code " + std::to_string(c.cmd));
        }
    }
    if (unpacks != 3) fail("expected 3 unpacks, got " + std::to_string(unpacks));
    return buf;
}

ShrubPacket read_packet(std::span<const u8> list, ShrubPacketEntry entry, int& texture) {
    InputBuffer in = run_vif(list);
    ShrubPacket out;
    out.entry = entry;
    out.header = in.bytes<ShrubPacketHeader>(0, "packet header");
    const ShrubPacketHeader& h = out.header;
    // VU1 reads the counts as 16-bit integers; its tag / ad-gif copy loops are do-while loops.
    if (h.texture_count < 1 || h.gif_tag_count < 1) fail("texture_count and gif_tag_count must be >= 1");
    if (h.vertex_count < 0 || h.vertex_offset < 0 || size_t(h.vertex_offset) + 2 * size_t(h.vertex_count) > INPUT_QWC) fail("vertex tables outside the input buffer");
    for (s32 k = 0; k < h.gif_tag_count; k++) out.gif_tags.push_back(in.bytes<ShrubGifTag>(1 + size_t(k), "GIF tag"));
    for (s32 k = 0; k < h.texture_count; k++) {
        ShrubAdGifs a;
        size_t base = 1 + size_t(h.gif_tag_count) + 4 * size_t(k);
        a.tex1 = in.bytes<AdGif>(base, "ad-gif");
        a.clamp = in.bytes<AdGif>(base + 1, "ad-gif");
        a.miptbp1 = in.bytes<AdGif>(base + 2, "ad-gif");
        a.tex0 = in.bytes<AdGif>(base + 3, "ad-gif");
        out.ad_gifs.push_back(a);
    }
    size_t p1 = size_t(h.vertex_offset), p2 = p1 + size_t(h.vertex_count);
    for (s32 i = 0; i < h.vertex_count; i++) {
        auto a = in.halves(p1 + size_t(i), "vertex part 1");
        auto b = in.halves(p2 + size_t(i), "vertex part 2");
        out.part1.push_back({a[0], a[1], a[2], a[3]});
        out.part2.push_back({b[0], b[1], b[2], u16(b[3])});
    }
    // The vertex loop tests the stop bit from vertex 2 on and drains three more vertices after it.
    size_t stop = 2;
    while (!(in.at(p2 + stop, "vertex stop scan")[3] & 0x8000)) stop++;
    for (size_t i = 0; i < stop + 4; i++) {
        auto a = in.halves(p1 + i, "vertex part 1");
        auto b = in.halves(p2 + i, "vertex part 2");
        u16 n = u16(b[3]);
        if ((n & 0x7fff) > 23) fail("normal index > 23");
        out.vertices.push_back({a[0], a[1], a[2], a[3], b[0], b[1], b[2], u8(n & 0x7fff), u8(n >> 15)});
    }

    // GS-packet slots in VU1 write order: tags, then ad-gif blocks, then vertices (later writes win).
    struct Slot { u8 kind = 0; u8 sub = 0; u16 index = 0; };   // kind: 0 unwritten, 1 tag, 2 ad-gif, 3 vertex
    std::array<Slot, OUTPUT_QWC> slot{};
    auto put = [&](s32 at, u8 kind, u16 index, u8 sub) {
        if (at < 0 || size_t(at) >= OUTPUT_QWC) fail("GS slot outside the 0xa8-qw output buffer");
        slot[size_t(at)] = {kind, sub, index};
    };
    for (size_t k = 0; k < out.gif_tags.size(); k++) put(out.gif_tags[k].gs_packet_offset, 1, u16(k), 0);
    for (size_t k = 0; k < out.ad_gifs.size(); k++) {
        s32 at;
        std::memcpy(&at, out.ad_gifs[k].tex1.pad + 3, 4);
        for (u8 j = 0; j < 5; j++) put(at + j, 2, u16(k), j);
    }
    for (size_t i = 0; i < out.vertices.size(); i++)
        for (u8 j = 0; j < 3; j++) put(out.vertices[i].gs_packet_offset + j, 3, u16(i), j);

    // GIF from slot 0 until the EOP tag.
    size_t cursor = 0;
    for (;;) {
        if (cursor >= OUTPUT_QWC) fail("GIF runs past the output buffer");
        Slot s = slot[cursor];
        if (s.kind == 2 && s.sub == 0) {
            for (u8 j = 1; j < 5; j++) {
                Slot t = cursor + j < OUTPUT_QWC ? slot[cursor + j] : Slot{};
                if (t.kind != 2 || t.index != s.index || t.sub != j) fail("ad-gif block partly overwritten");
            }
            s32 tex = out.ad_gifs[s.index].tex0.data_lo;
            if (tex < 0 || tex > 15) fail("ad-gif texture slot out of range");
            texture = tex;
            cursor += 5;
        } else if (s.kind == 1) {
            const ShrubGifTag& g = out.gif_tags[s.index];
            u32 nloop = u32(g.tag & 0x7fff), eop = u32(g.tag >> 15) & 1, pre = u32(g.tag >> 46) & 1;
            u32 prim = u32(g.tag >> 47) & 7, flg = u32(g.tag >> 58) & 3, nreg = u32(g.tag >> 60);
            if (flg != 0 || nreg != 3 || (g.tag_hi & 0xfff) != 0x412 || !pre) fail("vertex GIF tag is not PACKED ST/RGBAQ/XYZF2 with PRIM");
            if (prim != 3 && prim != 4) fail("unexpected GS primitive type " + std::to_string(prim));
            if (texture < 0) fail("vertices drawn before any ad-gif");
            ShrubDraw d{u8(texture), u8(prim), {}};
            for (u32 m = 0; m < nloop; m++) {
                size_t q = cursor + 1 + 3 * size_t(m);
                if (q + 2 >= OUTPUT_QWC) fail("GIF runs past the output buffer");
                Slot a = slot[q], b = slot[q + 1], c = slot[q + 2];
                if (a.kind != 3 || a.sub != 0 || b.kind != 3 || b.sub != 1 || c.kind != 3 || c.sub != 2 || b.index != a.index || c.index != a.index)
                    fail("GIF reads a vertex slot not written by one vertex at " + std::to_string(q));
                d.vertices.push_back(a.index);
            }
            out.draws.push_back(std::move(d));
            cursor += 1 + 3 * size_t(nloop);
            if (eop) break;
        } else {
            fail("no GIF tag at GS slot " + std::to_string(cursor));
        }
    }
    return out;
}

} // namespace

ShrubClass parse_shrub_class(Buffer blob) {
    ShrubClass sc;
    sc.header = blob.read<ShrubClassHeader>(0, "shrub class header");
    const ShrubClassHeader& h = sc.header;
    if (h.packet_count < 0 || h.packet_count > 1000) throw FormatError("implausible shrub packet count");
    std::vector<ShrubPacketEntry> entries = blob.read_multiple<ShrubPacketEntry>(0x40, size_t(h.packet_count), "shrub packet table");
    int texture = -1;   // GS texture state carries from packet to packet
    for (const ShrubPacketEntry& e : entries) {
        if (e.offset < 0 || e.size < 0) throw FormatError("negative shrub packet offset/size");
        sc.packets.push_back(read_packet(blob.sub(size_t(e.offset), size_t(e.size)).bytes(), e, texture));
    }
    if (h.normals_offset <= 0) throw FormatError("shrub class without normals");
    sc.normals = blob.read_multiple<ShrubNormal>(size_t(h.normals_offset), 24, "shrub normals");
    if (h.billboard_offset > 0) sc.billboard.push_back(blob.read<ShrubBillboard>(size_t(h.billboard_offset), "shrub billboard"));
    return sc;
}

std::vector<ShrubTriangle> shrub_triangles(const ShrubPacket& p) {
    std::vector<ShrubTriangle> out;
    for (const ShrubDraw& d : p.draws) {
        const auto& v = d.vertices;
        if (d.prim == 4) {
            for (size_t i = 2; i < v.size(); i++)
                out.push_back(i % 2 == 0 ? ShrubTriangle{v[i - 2], v[i - 1], v[i], d.texture} : ShrubTriangle{v[i], v[i - 1], v[i - 2], d.texture});
        } else {
            for (size_t i = 0; i + 2 < v.size(); i += 3) out.push_back({v[i], v[i + 1], v[i + 2], d.texture});
        }
    }
    return out;
}

std::vector<ShrubInstance> parse_shrub_instances(Buffer section) {
    s32 count = section.read<s32>(0);
    if (count < 0 || count > 100000) throw FormatError("implausible shrub instance count");
    return section.read_multiple<ShrubInstance>(0x10, size_t(count), "shrub instances");
}

} // namespace rc
