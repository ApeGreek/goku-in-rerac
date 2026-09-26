#pragma once
#include <array>
#include <vector>
#include "core/buffer.h"

namespace rc {

// Tfrag terrain block. Spec: docs/formats/tfrag_rac1.md.
struct TfragBlockHeader { s32 table_offset; s32 tfrag_count; f32 unknown_8; u32 unknown_c; };
static_assert(sizeof(TfragBlockHeader) == 0x10);

struct TfragHeader {
    f32 bsphere[4];
    s32 data;            // relative to table_offset
    u16 lod_2_ofs, shared_ofs, lod_1_ofs, lod_0_ofs, tex_ofs, rgba_ofs;
    u8 common_size, lod_2_size, lod_1_size, lod_0_size;
    u8 lod_2_rgba_count, lod_1_rgba_count, lod_0_rgba_count, base_only;
    u8 texture_count, rgba_size, rgba_verts_loc, occl_index_stash;
    u8 msphere_count, flags;
    u16 msphere_ofs, light_ofs, light_end_ofs;
    u8 dir_lights_one, dir_lights_upd;
    u16 point_lights, cube_ofs, occl_index;
    u8 vert_count, tri_count;
    u16 mip_dist;
};
static_assert(sizeof(TfragHeader) == 0x40);

struct TfragVuHeader {   // 20 x u16, see spec 2.4
    // Each vertex-info tier is [one entry per position] ++ [extra entries]: unk_02/unk_06/unk_0a are the
    // extra counts (common/lod01/lod0), unk_10/unk_14/unk_18 their VU addresses (= tier addr + position count).
    // unk_06/unk_0a are also the lengths of the LOD-01/LOD-0 "unknown indices 2" arrays. Verified on all 19 levels.
    u16 positions_common_count, unk_02, positions_lod_01_count, unk_06;
    u16 positions_lod_0_count, unk_0a, positions_common_addr, vertex_info_common_addr;
    u16 unk_10, vertex_info_lod_01_addr, unk_14, vertex_info_lod_0_addr;
    u16 unk_18, indices_addr, parent_indices_lod_01_addr, unk_indices_2_lod_01_addr;
    u16 parent_indices_lod_0_addr, unk_indices_2_lod_0_addr, strips_addr, texture_ad_gifs_addr;
};
static_assert(sizeof(TfragVuHeader) == 0x28);

struct TfragPosition { s16 x, y, z; };
struct TfragVertexInfo { s16 s, t, parent, vertex; };   // parent/vertex: VU qword offsets, position index = /2
// y >= 0: load ad-gif z; y < 0: XGKICK, then load ad-gif z if z >= 0 (z = -1: keep texture). z = qword offset, index z/5.
struct TfragStrip { s8 vertex_count_and_flag, end_of_packet_flag, ad_gif_offset, pad; };
struct TfragRgba { u8 r, g, b, a; };
struct TfragLight { s8 unknown_0, intensity, azimuth, elevation; s16 color, pad; };
struct AdGif { s32 data_lo, data_hi; u8 address; u8 pad[7]; };  // one GIF A+D quadword
struct TfragAdGifs { AdGif tex0, tex1, clamp, miptbp1, miptbp2; };
static_assert(sizeof(TfragAdGifs) == 0x50);

struct TfragLod {
    std::vector<TfragStrip> strips;
    std::vector<u8> indices;
};

struct Tfrag {
    TfragHeader header;
    TfragVuHeader vu;
    s32 origin[4] = {0, 0, 0, 0};
    // Concatenated common ‖ lod01 ‖ lod0. Position index space (rgba, lights); vertex-info index space (strip indices).
    std::vector<TfragPosition> positions;
    std::vector<TfragVertexInfo> vertex_info;
    u32 positions_common = 0, positions_lod01 = 0, positions_lod0 = 0;
    u32 vinfo_common = 0, vinfo_lod01 = 0, vinfo_lod0 = 0;
    std::vector<u8> parent_indices_lod01, unk_indices_2_lod01, parent_indices_lod0, unk_indices_2_lod0;
    TfragLod lod[3];   // [0] = highest detail
    std::vector<TfragAdGifs> ad_gifs;
    std::vector<TfragRgba> rgba;
    std::vector<TfragLight> lights;
};

struct TfragTriangle { u16 a, b, c; u16 ad_gif; };  // vertex-info indices, ad-gif index

std::vector<Tfrag> parse_tfrags(Buffer block);
// Walks a LOD's strip list into triangles (GS triangle-strip semantics), texture per the VU1 rule. Spec 2.6.
std::vector<TfragTriangle> tfrag_triangles(const Tfrag& t, int lod);

} // namespace rc
