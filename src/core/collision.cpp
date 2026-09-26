#include "core/collision.h"

namespace rc {

Collision parse_collision(Buffer b) {
    Collision c;
    c.header.mesh = b.read<s32>(0);
    c.header.hero_groups = b.read<s32>(4);
    s32 mesh = c.header.mesh, hero = c.header.hero_groups;
    if (mesh <= 0) throw FormatError("collision: no mesh pointer");
    if (hero != 0 && hero < mesh) throw FormatError("collision: hero section before the mesh");
    // The mesh runs to the hero section (spec 2.1), or to the end of the block when there is none.
    Buffer m = b.sub(size_t(mesh), (hero > 0 ? size_t(hero) : b.size()) - size_t(mesh));
    s16 z_base = c.z_base = m.read<s16>(0);
    u16 z_count = c.z_count = m.read<u16>(2);
    if (z_count > 4096) throw FormatError("collision: implausible z count");
    for (u16 zi = 0; zi < z_count; zi++) {
        u16 zoff = m.read<u16>(4 + size_t(zi) * 2);
        if (zoff == 0) continue;
        size_t ynode = size_t(zoff) * 4;
        c.slab_offsets.push_back(u32(ynode));
        s16 y_base = m.read<s16>(ynode);
        u16 y_count = m.read<u16>(ynode + 2);
        if (y_count > 4096) throw FormatError("collision: implausible y count");
        for (u16 yi = 0; yi < y_count; yi++) {
            u32 yoff = m.read<u32>(ynode + 4 + size_t(yi) * 4);
            if (yoff == 0) continue;
            c.row_offsets.push_back(yoff);
            s16 x_base = m.read<s16>(yoff);
            u16 x_count = m.read<u16>(yoff + 2);
            if (x_count > 4096) throw FormatError("collision: implausible x count");
            for (u16 xi = 0; xi < x_count; xi++) {
                u32 word = m.read<u32>(size_t(yoff) + 4 + size_t(xi) * 4);
                if (word == 0) continue;
                size_t leaf = size_t(word >> 8);
                CollisionCell cell;
                cell.cx = s16(x_base + xi); cell.cy = s16(y_base + yi); cell.cz = s16(z_base + zi);
                cell.leaf_word = word;
                u16 face_count = cell.face_count = m.read<u16>(leaf);
                u8 vertex_count = cell.vertex_count = m.read<u8>(leaf + 2);
                u8 quad_count = cell.quad_count = m.read<u8>(leaf + 3);
                if (quad_count > face_count) throw FormatError("collision: quad count exceeds face count");
                // Low byte of the tree word = leaf size in quadwords (holds on every retail cell).
                size_t payload = 4 + 4 * size_t(vertex_count) + 4 * size_t(face_count) + quad_count;
                if ((payload + 15) / 16 != (word & 0xff)) throw FormatError("collision: leaf size byte does not match its contents");
                m.sub(leaf, (payload + 15) / 16 * 16);
                float cx = cell.cx * 4.0f + 2.0f, cy = cell.cy * 4.0f + 2.0f, cz = cell.cz * 4.0f + 2.0f;
                size_t p = leaf + 4;
                for (u32 i = 0; i < vertex_count; i++, p += 4) {
                    u32 w = m.read<u32>(p);
                    cell.packed.push_back(w);
                    s32 x = s32(w << 22) >> 22;          // bits 0-9 signed
                    s32 y = s32(w << 12) >> 22;          // bits 10-19 signed
                    s32 z = s32(w) >> 20;                // bits 20-31 signed
                    cell.vertices.push_back(cx + x / 16.0f);
                    cell.vertices.push_back(cy + y / 16.0f);
                    cell.vertices.push_back(cz + z / 64.0f);
                }
                size_t faces_at = p;
                size_t quad4_at = faces_at + size_t(face_count) * 4;
                cell.quad_v3 = m.read_multiple<u8>(quad4_at, quad_count);
                for (u32 i = 0; i < face_count; i++) {
                    CollisionFace f{};
                    f.v[0] = m.read<u8>(faces_at + i * 4); f.v[1] = m.read<u8>(faces_at + i * 4 + 1); f.v[2] = m.read<u8>(faces_at + i * 4 + 2);
                    f.type = m.read<u8>(faces_at + i * 4 + 3);
                    f.quad = i < quad_count;
                    f.v[3] = f.quad ? cell.quad_v3[i] : f.v[0];
                    for (int k = 0; k < (f.quad ? 4 : 3); k++) if (f.v[k] >= vertex_count) throw FormatError("collision: face index out of range");
                    cell.faces.push_back(f);
                }
                c.cells.push_back(std::move(cell));
            }
        }
    }
    if (hero > 0) {
        Buffer hb = b.sub(size_t(hero));
        s32 groups = c.hero_group_count = hb.read<s32>(0);
        if (groups < 0 || groups > 100000) throw FormatError("collision: implausible hero group count");
        for (s32 g = 0; g < groups; g++) {
            size_t r = 0x10 + size_t(g) * 0x10;
            HeroGroup hg;
            for (int k = 0; k < 4; k++) { hg.raw[k] = hb.read<u16>(r + size_t(k) * 2); hg.sphere[k] = hg.raw[k] / 64.0f; }
            u16 tri_count = hg.triangle_count = hb.read<u16>(r + 8);
            u16 vert_count = hg.vertex_count = hb.read<u16>(r + 10);
            u32 data = hg.data = hb.read<u32>(r + 12);
            hg.raw_vertices = hb.read_multiple<u16>(data, size_t(vert_count) * 4);
            for (u32 i = 0; i < vert_count; i++) {
                const u16* v = &hg.raw_vertices[i * 4];
                hg.vertices.push_back(v[0] / 64.0f); hg.vertices.push_back(v[1] / 64.0f); hg.vertices.push_back(v[2] / 64.0f);
                if (v[3] != 0) throw FormatError("collision: unknown hero vertex variant");
            }
            size_t t0 = data + size_t(vert_count) * 8;
            for (u32 i = 0; i < tri_count; i++) {
                for (int k = 0; k < 3; k++) { u8 ix = hb.read<u8>(t0 + i * 4 + k); if (ix >= vert_count) throw FormatError("collision: hero index out of range"); hg.triangles.push_back(ix); }
                if (hb.read<u8>(t0 + i * 4 + 3) != 0) throw FormatError("collision: unknown hero triangle variant");
            }
            c.hero_groups.push_back(std::move(hg));
        }
    }
    return c;
}

std::vector<CollisionTriangle> collision_triangles(const Collision& c) {
    std::vector<CollisionTriangle> out;
    for (size_t ci = 0; ci < c.cells.size(); ci++) {
        const CollisionCell& cell = c.cells[ci];
        for (size_t fi = 0; fi < cell.faces.size(); fi++) {
            const CollisionFace& f = cell.faces[fi];
            int n = f.quad ? 2 : 1;
            for (int t = 0; t < n; t++) {
                const u8 ix[3] = {f.v[0], f.v[t + 1], f.v[t + 2]};
                CollisionTriangle tri{};
                f32* dst[3] = {tri.a, tri.b, tri.c};
                for (int k = 0; k < 3; k++) for (int j = 0; j < 3; j++) dst[k][j] = cell.vertices[size_t(ix[k]) * 3 + size_t(j)];
                tri.cell = u32(ci); tri.face = u16(fi); tri.surface = f.type;
                tri.part = u8(f.quad ? t + 1 : 0);
                out.push_back(tri);
            }
        }
    }
    return out;
}

} // namespace rc
