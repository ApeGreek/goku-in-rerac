#include "core/moby.h"
#include "core/vif.h"
#include <cmath>
#include <cstring>
#include <memory>

namespace rc {

namespace {

// VU0 data memory of program 104691 as matrix slots: each quadword address that a matrix was
// stored at holds either one joint-palette matrix (a transfer) or a 2-/3-way blend of them.
struct Vu0Slots {
    MobySkin slot[256];
    bool defined[256] = {};
    void store(u32 addr, const MobySkin& s) {
        if (addr % 4) throw FormatError("moby skin: unaligned VU0 store address");
        slot[addr] = s; defined[addr] = true;
    }
    const MobySkin& load(u32 addr) const {
        if (addr % 4) throw FormatError("moby skin: unaligned VU0 load address");
        if (addr >= 0xf4) throw FormatError("moby skin: load from the VU0 sink/constant area");
        if (!defined[addr]) throw FormatError("moby skin: load from a VU0 slot that was never written");
        return slot[addr];
    }
    const MobySkin& load_joint(u32 addr) const {
        const MobySkin& s = load(addr);
        if (s.count != 1) throw FormatError("moby skin: blend input is itself a blend");
        return s;
    }
};

MobySkin joint_skin(u8 joint) { MobySkin s; s.count = 1; s.joints[0] = joint; s.weights[0] = 256; return s; }

// Index walk (spec 2.4, corrected): 1-based indices, bit 7 = no GS kick, 0 = texture switch
// (next ad-gif block) whose vertex is the next secret index; a 0 secret index ends the packet
// and the last three indices pushed (the 1,1,1 flush trailer) are never kicked.
std::vector<MobyTriangle> walk_indices(const MobyPacket& pk, s32* material) {
    struct Push { u32 idx; bool nokick; s32 material; };
    std::vector<Push> pushes;
    size_t secret_i = 0, adgif_i = 0;
    bool ended = false;
    for (size_t i = 0; i < pk.index_bytes.size(); i++) {
        u8 b = pk.index_bytes[i];
        u32 idx; bool nk;
        if (b == 0) {
            if (secret_i >= pk.secret_indices.size()) throw FormatError("moby packet: ran out of secret indices");
            u8 s = pk.secret_indices[secret_i++];
            if (s == 0) { ended = true; break; }
            if (adgif_i >= pk.texture_indices.size()) throw FormatError("moby packet: texture switch without an ad-gif block");
            *material = pk.texture_indices[adgif_i++];
            idx = s & 0x7f; nk = true;
        } else {
            idx = b & 0x7f; nk = (b & 0x80) != 0;
        }
        if (idx == 0 || idx > pk.vertices.size()) throw FormatError("moby packet: vertex index out of range");
        pushes.push_back({idx - 1, nk, *material});
    }
    if (!ended) throw FormatError("moby packet: index stream not terminated");
    if (pushes.size() < 3) throw FormatError("moby packet: fewer than 3 indices before the terminator");
    pushes.resize(pushes.size() - 3);
    std::vector<MobyTriangle> tris;
    for (size_t n = 2; n < pushes.size(); n++)
        if (!pushes[n].nokick) tris.push_back({pushes[n - 2].idx, pushes[n - 1].idx, pushes[n].idx, pushes[n].material});
    return tris;
}

MobyPacket read_packet(Buffer blob, const MobyPacketEntry& e, bool metal, std::vector<MobyVertex>* cache, Vu0Slots* slots, s32* material) {
    MobyPacket pk;
    pk.entry = e;
    pk.metal = metal;

    // --- VIF list: ST, indices, optional ad-gifs (metal packets have no ST). ---
    std::span<const u8> list = blob.sub(e.vif_list_offset, size_t(e.vif_list_size) * 0x10).bytes();
    std::vector<VifPacket> codes = parse_vif(list);
    std::vector<const VifPacket*> unpacks;
    for (const auto& c : codes) if (c.is_unpack()) unpacks.push_back(&c);
    size_t u = 0;
    if (!metal) {
        if (unpacks.size() < 2) throw FormatError("moby packet: fewer than 2 unpacks");
        const VifPacket& st = *unpacks[u++];
        if (st.vn() != 1 || st.vl() != 1) throw FormatError("moby packet: first unpack is not V2_16");
        pk.st.resize(size_t(st.count()) * 2);
        std::memcpy(pk.st.data(), st.data.data(), pk.st.size() * 2);
    }
    if (u >= unpacks.size()) throw FormatError("moby packet: missing index unpack");
    const VifPacket& ix = *unpacks[u++];
    if (ix.vn() != 3 || ix.vl() != 2) throw FormatError("moby packet: index unpack is not V4_8");
    size_t ix_bytes = size_t(ix.count()) * 4;
    if (ix_bytes < 4) throw FormatError("moby packet: index unpack too small");
    pk.index_bytes.assign(ix.data.begin() + 4, ix.data.begin() + ix_bytes);
    pk.secret_indices = {ix.data[2]};
    if (u < unpacks.size()) {
        const VifPacket& ad = *unpacks[u++];
        if (ad.vn() != 3 || ad.vl() != 0) throw FormatError("moby packet: texture unpack is not V4_32");
        size_t blocks = size_t(ad.count()) / 4;
        for (size_t b = 0; b < blocks; b++) {
            s32 tex0_lo; std::memcpy(&tex0_lo, ad.data.data() + b * 0x40 + 0x20, 4);
            pk.texture_indices.push_back(tex0_lo);
            // One extra index per ad-gif block, taken from successive *quadwords* (qw b, byte 0xc),
            // so block 0 carries the first four (verified: every packet on the disc then ends on 1,1,1,0).
            pk.secret_indices.push_back(ad.data[b * 0x10 + 0x0c]);
        }
    }

    // --- Vertex table. ---
    if (metal) {
        pk.metal_header = blob.read<MobyMetalVertexTableHeader>(e.vertex_offset, "moby metal vertex table header");
        s32 count = pk.metal_header.vertex_count;
        if (count < 0 || count > 4096) throw FormatError("moby metal packet: implausible vertex count");
        pk.raw_vertices = blob.copy(e.vertex_offset + 0x10, size_t(count) * 0x10);
        for (s32 i = 0; i < count; i++) {
            const u8* r = pk.raw_vertices.data() + size_t(i) * 0x10;
            MobyVertex v{};
            std::memcpy(&v.x, r, 2); std::memcpy(&v.y, r + 2, 2); std::memcpy(&v.z, r + 4, 2);
            v.normal_azimuth = r[6]; v.normal_elevation = r[7];
            std::memcpy(v.raw, r + 8, 8);
            v.id = u16(i);
            v.type = 4;
            // s16 x,y,z; u8 az,el; u8 joint[3]; u8 count; u8 weight[3]; pad. count <= 1: one joint (byte 8).
            u8 n = r[11];
            if (n <= 1) v.skin = joint_skin(r[8]);
            else {
                if (n > 3) throw FormatError("moby metal vertex: joint count > 3");
                v.skin.count = n;
                u32 sum = 0;
                for (u8 k = 0; k < n; k++) { v.skin.joints[k] = r[8 + k]; v.skin.weights[k] = r[12 + k]; sum += r[12 + k]; }
                if (sum != 256) throw FormatError("moby metal vertex: weights do not sum to 256");
            }
            pk.vertices.push_back(v);
        }
    } else {
        pk.vth = blob.read<MobyVertexTableHeader>(e.vertex_offset, "moby vertex table header");
        const MobyVertexTableHeader& h = pk.vth;
        u32 in_file = h.two_way_blend_vertex_count + h.three_way_blend_vertex_count + h.main_vertex_count;
        if (h.transfer_vertex_count != in_file + h.duplicate_vertex_count) throw FormatError("moby packet: transfer count mismatch");
        if (h.transfer_vertex_count != e.transfer_vertex_count) throw FormatError("moby packet: entry/table transfer count mismatch");
        if (e.unknown_d != u8((0xf + e.transfer_vertex_count * 6) / 0x10) || e.unknown_e != u8((3 + e.transfer_vertex_count) / 4))
            throw FormatError("moby packet: redundant entry fields do not match");
        pk.transfers = blob.copy(e.vertex_offset + 0x20, size_t(h.matrix_transfer_count) * 2);
        size_t ofs = e.vertex_offset + 0x20 + size_t(h.matrix_transfer_count) * 2;
        if (ofs % 4) ofs += 2;
        if (ofs % 8) ofs += 4;
        pk.duplicates = blob.read_multiple<u16>(ofs, h.duplicate_vertex_count, "moby duplicate vertices");
        size_t vbase = e.vertex_offset + h.vertex_table_offset;
        if (h.unknown_e < h.vertex_table_offset) throw FormatError("moby packet: unknown_e before vertex table");
        size_t epilogue = (h.unknown_e - h.vertex_table_offset) / 0x10 - in_file;
        if (epilogue >= 7 || epilogue < 1) throw FormatError("moby packet: epilogue vertex count " + std::to_string(epilogue));
        // Per-vertex RGBA lighting multipliers (docs/plan/moby_skinning_lighting.md 4).
        size_t mult_size = size_t(e.vertex_data_size) * 0x10;
        if (mult_size < h.unknown_e) throw FormatError("moby packet: multiplier blob starts past the vertex data");
        pk.rgba_multipliers = blob.copy(e.vertex_offset + h.unknown_e, mult_size - h.unknown_e);
        if (pk.rgba_multipliers.size() != ((size_t(h.transfer_vertex_count) * 4 + 15) & ~size_t(15)))
            throw FormatError("moby packet: multiplier blob is not align16(4 * transfer_vertex_count)");
        pk.raw_vertices = blob.copy(vbase, (in_file + epilogue) * 0x10);
        struct Raw { u8 b[16]; };
        std::vector<Raw> raw = blob.read_multiple<Raw>(vbase, in_file + epilogue, "moby vertices");
        std::vector<u16> ids;
        for (size_t i = 7; i < raw.size(); i++) { u16 lo; std::memcpy(&lo, raw[i].b, 2); ids.push_back(lo & 0x1ff); }
        for (size_t k = 0; k < 6 && ids.size() < in_file; k++) { u16 v; std::memcpy(&v, raw.back().b + 4 + k * 2, 2); ids.push_back(v & 0x1ff); }
        if (ids.size() < in_file) throw FormatError("moby packet: not enough vertex ids");

        // VU0 slot machine (docs/plan/moby_skinning_lighting.md 4-5): pre-loop transfers, then per
        // vertex the scheduled transfer is stored before the loads.
        for (u32 t = 0; t < h.matrix_transfer_count; t++) slots->store(pk.transfers[t * 2 + 1], joint_skin(pk.transfers[t * 2]));
        u32 n2 = h.two_way_blend_vertex_count, n3 = h.three_way_blend_vertex_count;
        for (size_t i = 0; i < in_file; i++) {
            const u8* r = raw[i].b;
            MobyVertex v{};
            std::memcpy(v.raw, r, 8);
            v.normal_azimuth = r[8]; v.normal_elevation = r[9];
            std::memcpy(&v.x, r + 10, 2); std::memcpy(&v.y, r + 12, 2); std::memcpy(&v.z, r + 14, 2);
            v.id = ids[i];
            v.type = i < n2 ? 1 : i < n2 + n3 ? 2 : 3;
            u8 joint = u8(r[1] >> 1);   // low_halfword >> 9
            if (v.type == 1) {
                slots->store(r[6], joint_skin(joint));
                const MobySkin& a = slots->load_joint(r[2]);
                const MobySkin& b = slots->load_joint(r[3]);
                MobySkin m; m.count = 2;
                m.joints[0] = a.joints[0]; m.joints[1] = b.joints[0];
                m.weights[0] = r[4]; m.weights[1] = r[5];
                if (r[4] + r[5] != 256) throw FormatError("moby skin: 2-way weights do not sum to 256");
                v.skin = m;
                slots->store(r[7], m);
            } else if (v.type == 2) {
                const MobySkin& a = slots->load_joint(r[2]);
                const MobySkin& b = slots->load_joint(r[3]);
                const MobySkin& c = slots->load_joint(r[1] & 0xfe);
                MobySkin m; m.count = 3;
                m.joints[0] = a.joints[0]; m.joints[1] = b.joints[0]; m.joints[2] = c.joints[0];
                m.weights[0] = r[4]; m.weights[1] = r[5]; m.weights[2] = r[6];
                if (r[4] + r[5] + r[6] != 256) throw FormatError("moby skin: 3-way weights do not sum to 256");
                v.skin = m;
                slots->store(r[7], m);
            } else {
                slots->store(r[3], joint_skin(joint));
                v.skin = slots->load(r[2]);
            }
            pk.vertices.push_back(v);
        }
        // Duplicates reference the 512-entry vertex cache by the 9-bit id; the cache persists
        // across the packets of one LOD list. A duplicate copies position, normal and skin.
        for (size_t i = 0; i < in_file; i++) { const MobyVertex& v = pk.vertices[i]; (*cache)[v.id & 0x1ff] = v; (*cache)[v.id & 0x1ff].type |= 0x80; }
        for (u16 d : pk.duplicates) {
            u16 id = (d >> 7) & 0x1ff;
            MobyVertex v = (*cache)[id];
            if (!(v.type & 0x80)) pk.unresolved_duplicates++;
            v.type &= 0x7f;
            v.id = id;
            v.duplicate = true;
            pk.vertices.push_back(v);
        }
        if (pk.st.size() / 2 < pk.vertices.size()) throw FormatError("moby packet: ST array shorter than vertex list");
    }

    pk.initial_texture = *material;
    pk.triangles = walk_indices(pk, material);
    return pk;
}

} // namespace

void moby_normal(u8 azimuth, u8 elevation, f32 out[3]) {
    const double k = 3.14159265358979323846 / 128.0;
    double a = double(azimuth) * k, e = double(elevation) * k;
    out[0] = f32(std::cos(a) * std::cos(e));
    out[1] = f32(std::sin(a) * std::cos(e));
    out[2] = f32(std::sin(e));
}

MobyClass parse_moby_class(Buffer blob) {
    MobyClass mc;
    mc.header = blob.read<MobyClassHeader>(0, "moby class header");
    const MobyClassHeader& h = mc.header;
    if (h.sequence_count) mc.sequence_pointers = blob.read_multiple<s32>(0x48, h.sequence_count, "moby sequence pointers");
    if (h.packet_table_offset > 0) {
        size_t total = size_t(h.high_lod_count) + h.low_lod_count + h.metal_count;
        if (h.metal_count && size_t(h.metal_begin) + h.metal_count > total) throw FormatError("moby class: metal packets past the packet table");
        std::vector<MobyPacketEntry> entries = blob.read_multiple<MobyPacketEntry>(size_t(h.packet_table_offset), total, "moby packet table");
        // Vertex cache, VU0 slots and the GS texture state carry across the packets of one LOD list.
        auto list = [&](size_t first, size_t count, bool metal, std::vector<MobyPacket>* out) {
            std::vector<MobyVertex> cache(512);
            auto slots = std::make_unique<Vu0Slots>();
            s32 material = -1;
            for (size_t i = 0; i < count; i++) out->push_back(read_packet(blob, entries[first + i], metal, &cache, slots.get(), &material));
        };
        list(0, h.high_lod_count, false, &mc.high_lod);
        list(h.high_lod_count, h.low_lod_count, false, &mc.low_lod);
        list(h.metal_begin, h.metal_count, true, &mc.metal);
    }
    if (h.skeleton > 0 && h.joint_count) mc.skeleton = blob.read_multiple<f32>(size_t(h.skeleton), size_t(h.joint_count) * 16, "moby skeleton");
    if (h.common_trans > 0 && h.joint_count) mc.common_trans = blob.read_multiple<MobyClass::Trans>(size_t(h.common_trans), h.joint_count, "moby common trans");
    return mc;
}

std::vector<MobyInstance> parse_moby_instances(Buffer section) {
    s32 count = section.read<s32>(0);
    if (count < 0 || count > 100000) throw FormatError("implausible moby instance count");
    std::vector<MobyInstance> out = section.read_multiple<MobyInstance>(0x10, size_t(count), "moby instances");
    for (const auto& m : out) if (m.size != 0x78) throw FormatError("moby instance size != 0x78");
    return out;
}

} // namespace rc
