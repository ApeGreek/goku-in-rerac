#pragma once
#include <vector>
#include "core/buffer.h"
#include "core/texture.h"

namespace rc {

// Sky block. Spec: docs/formats/shrub_sky_rac1.md part 2.
struct SkyHeader {
    u8 r, g, b, a;
    s16 clear_screen, shell_count, sprite_count, maximum_sprite_count, texture_count, fx_count;
    s32 texture_defs, texture_data, fx_list, sprites;
    s32 shells[8];
};
static_assert(sizeof(SkyHeader) == 0x40);

struct SkyTextureDef { s32 palette_offset, texture_offset, width, height; };
struct SkyClusterHeader { f32 bsphere[4]; s32 data; s16 vertex_count, tri_count, vertex_offset, st_offset, tri_offset, data_size; };
static_assert(sizeof(SkyClusterHeader) == 0x20);
struct SkyVertex { s16 x, y, z, alpha; };
// Per-vertex attribute word at st_offset. Textured shells: s, t as u16 4.12 fixed (the game
// zero-extends them: pextlh + vitof12). Untextured (gouraud) shells: the same 4 bytes are the
// vertex colour RGBA (0x80 = 1.0), sent to RGBAQ verbatim (boot 0x22c0e0).
struct SkyTexCoord { s16 s, t; };
struct SkyFace { u8 indices[3]; u8 texture; };  // texture 0xff = untextured

struct SkyCluster {
    SkyClusterHeader header;
    std::vector<SkyVertex> vertices;
    std::vector<SkyTexCoord> st;
    std::vector<SkyFace> faces;
};
struct SkyShell {
    s32 cluster_count, flags;   // flags != 0 = untextured/gouraud (sky_draw_shell tests the whole word)
    std::vector<SkyCluster> clusters;
};
struct Sky {
    SkyHeader header;
    std::vector<u8> fx_list;
    std::vector<SkyTextureDef> texture_defs;
    std::vector<SkyShell> shells;
};

Sky parse_sky(Buffer block);
Image decode_sky_texture(Buffer block, const Sky& sky, size_t index);

// One vertex of the GS triangle list the game builds for a cluster, three per face in face
// order (boot 0x22c208 textured / 0x22c0e0 gouraud; the per-frame clip-flag face rejection is
// view dependent and not applied). Textured: s, t = u16 / 4096, rgba = (0x80, 0x80, 0x80,
// low byte of SkyVertex::alpha), texture = face.texture. Gouraud: s = t = 0, rgba = the
// vertex's attribute bytes, texture = 0xff. Position is the raw s16 vertex.
struct SkyGsVertex { s16 x, y, z; u8 texture, pad; f32 s, t; u8 rgba[4]; };
static_assert(sizeof(SkyGsVertex) == 20);
std::vector<SkyGsVertex> sky_gs_vertices(const SkyShell& shell, const SkyCluster& cluster);

} // namespace rc
