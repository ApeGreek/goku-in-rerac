#include "core/sky.h"
#include <cstring>

namespace rc {

Sky parse_sky(Buffer b) {
    Sky sky;
    sky.header = b.read<SkyHeader>(0, "sky header");
    const SkyHeader& h = sky.header;
    if (h.shell_count < 0 || h.shell_count > 8) throw FormatError("sky: shell count out of range");
    if (h.fx_count > 0 && h.fx_list > 0) sky.fx_list = b.read_multiple<u8>(size_t(h.fx_list), size_t(h.fx_count), "sky fx list");
    if (h.texture_count > 0 && h.texture_defs > 0) sky.texture_defs = b.read_multiple<SkyTextureDef>(size_t(h.texture_defs), size_t(h.texture_count), "sky texture defs");
    for (int i = 0; i < h.shell_count; i++) {
        size_t so = size_t(h.shells[i]);
        SkyShell shell;
        shell.cluster_count = b.read<s32>(so);
        shell.flags = b.read<s32>(so + 4);
        if (shell.cluster_count < 0 || shell.cluster_count > 10000) throw FormatError("sky: implausible cluster count");
        std::vector<SkyClusterHeader> chs = b.read_multiple<SkyClusterHeader>(so + 0x10, size_t(shell.cluster_count), "sky cluster headers");
        for (const auto& ch : chs) {
            SkyCluster c;
            c.header = ch;
            size_t d = size_t(ch.data);
            // The game DMAs only data_size bytes of the cluster to the scratchpad (boot 0x22b6e8).
            auto within = [&](s32 off, size_t len) { return off >= 0 && ch.data_size >= 0 && size_t(off) + len <= size_t(ch.data_size); };
            if (ch.data < 0 || ch.vertex_count < 0 || ch.tri_count < 0 ||
                !within(ch.vertex_offset, size_t(ch.vertex_count) * sizeof(SkyVertex)) ||
                !within(ch.st_offset, size_t(ch.vertex_count) * sizeof(SkyTexCoord)) ||
                !within(ch.tri_offset, size_t(ch.tri_count) * sizeof(SkyFace)))
                throw FormatError("sky cluster arrays outside data_size");
            c.vertices = b.read_multiple<SkyVertex>(d + size_t(ch.vertex_offset), size_t(ch.vertex_count), "sky vertices");
            c.st = b.read_multiple<SkyTexCoord>(d + size_t(ch.st_offset), size_t(ch.vertex_count), "sky st");
            c.faces = b.read_multiple<SkyFace>(d + size_t(ch.tri_offset), size_t(ch.tri_count), "sky faces");
            for (const auto& f : c.faces) {
                for (u8 ix : f.indices) if (ix >= ch.vertex_count) throw FormatError("sky face index out of range");
                if (f.texture != 0xff && f.texture >= h.texture_count) throw FormatError("sky face texture out of range");
            }
            shell.clusters.push_back(std::move(c));
        }
        sky.shells.push_back(std::move(shell));
    }
    return sky;
}

Image decode_sky_texture(Buffer b, const Sky& sky, size_t index) {
    const SkyTextureDef& t = sky.texture_defs.at(index);
    size_t base = size_t(sky.header.texture_data);
    return decode_indexed8(b.sub(base + size_t(t.texture_offset), size_t(t.width) * size_t(t.height)).bytes(),
                           u32(t.width), u32(t.height), b.sub(base + size_t(t.palette_offset), 1024).bytes());
}

std::vector<SkyGsVertex> sky_gs_vertices(const SkyShell& shell, const SkyCluster& c) {
    std::vector<SkyGsVertex> out;
    out.reserve(c.faces.size() * 3);
    for (const SkyFace& f : c.faces) {
        for (u8 ix : f.indices) {
            const SkyVertex& v = c.vertices[ix];
            const SkyTexCoord& a = c.st[ix];
            SkyGsVertex g{};
            g.x = v.x; g.y = v.y; g.z = v.z;
            if (shell.flags == 0) {
                g.texture = f.texture;
                g.s = f32(u16(a.s)) / 4096.0f;
                g.t = f32(u16(a.t)) / 4096.0f;
                g.rgba[0] = g.rgba[1] = g.rgba[2] = 0x80;
                g.rgba[3] = u8(v.alpha);
            } else {
                g.texture = 0xff;
                std::memcpy(g.rgba, &a, 4);
            }
            out.push_back(g);
        }
    }
    return out;
}

} // namespace rc
