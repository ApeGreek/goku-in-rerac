#pragma once
#include <vector>
#include "core/buffer.h"
#include "core/tfrag.h"   // AdGif

namespace rc {

// Tie (instanced static prop) class. Spec: docs/formats/tie_rac1.md; the packet
// semantics below follow the EE DMA builder (TieProc) and the VU1 program 13507
// (docs/formats/tie_rac1.md section 3.4), not only Wrench.

struct TieLodInfo {              // per-LOD totals, all verified on the retail disc
    u32 strip_vertex_count;      // sum of TieStrip::vertex_count over the LOD's packets
    u32 triangle_count;          // sum of (vertex_count - 2)
    u32 strip_count;             // sum of strip counts
    u32 pad;
};
static_assert(sizeof(TieLodInfo) == 0x10);

struct TieClassHeader {          // RAC1 form, 0x80 bytes
    s32 packets[3];              // 0x00 packet-header table per LOD (blob-relative)
    u32 normals;                 // 0x0c 64 x s16[4] light-slot normals (0x200 bytes, just before the ad-gifs)
    f32 near_dist, mid_dist, far_dist;   // 0x10
    f32 unknown_1c;              // 0x1c equals unknown_48 on disc; TieProc reuses it as per-frame counters
    u8 packet_count[3];          // 0x20
    u8 texture_count;            // 0x23 ad-gif records at ad_gif_ofs
    u16 flags_24;                // 0x24 render mode bits (TieProc: &9 skips the class, (&6)>>1 selects the path)
    u16 instance_count_26;       // 0x26 0 on disc (run-time)
    u32 instance_list_28;        // 0x28 0 on disc (run-time)
    u32 ad_gif_ofs;              // 0x2c
    f32 bsphere[4];              // 0x30
    f32 scale;                   // 0x40 class-space position = s16 * scale / 1024
    s32 o_class;                 // 0x44 the class's own number
    f32 unknown_48;              // 0x48
    u32 unknown_4c;              // 0x4c
    TieLodInfo lod_info[3];      // 0x50
};
static_assert(sizeof(TieClassHeader) == 0x80);

struct TiePacketHeader {         // 0x10 bytes; _ofs/_size fields are quadwords relative to the packet data
    s32 data;                    // 0x0 packet data offset, relative to the LOD's packet table
    u8 shader_count;             // 0x4 ad-gif blocks uploaded (= 1 + leading positive ad_gif_dest entries)
    u8 ad_gif_qwc;               // 0x5 = 5 * shader_count
    u8 control_count;            // 0x6 = 3 + strip_count (V4_8 elements that matter)
    u8 control_size;             // 0x7 qw of unpack header + strips (uploaded V4_8 USN)
    u8 vert_ofs, vert_size;      // 0x8 vertex region (V4_16 to VU 0x32)
    u8 color_ofs, color_count;   // 0xa colour-index region: 2 copies of color_count x 4 bytes, each padded to qw
    u8 slot_table_ofs, slot_table_size;   // 0xc per-strip-vertex GS slot steps (strip_vertex_count + 1 bytes)
    u8 strip_count;              // 0xe = unpack header strip_count
    u8 strip_vertex_count;       // 0xf sum of TieStrip::vertex_count
};
static_assert(sizeof(TiePacketHeader) == 0x10);

struct TieUnpackHeader {         // 12 bytes at packet data + 0x20 (V4_8 USN: one VU qword per 4 bytes)
    u8 dinky_single_only;        // 0 non-zero: no double-write phase for dinky vertices
    u8 no_fat;                   // 1 non-zero: no fat vertices
    u8 unknown_2;
    u8 strip_count;              // 3
    u8 dinky_single_end;         // 4 GS slot marking the end of the dinky single-write loop
    u8 dinky_double_end;         // 5 GS slot marking the end of the dinky double-write loop
    u8 fat_single_end;           // 6 GS slot of the last fat single-write vertex
    u8 fat_double_end;           // 7 GS slot of the last fat vertex
    u8 dinky_qwc_plus_four;      // 8 = 2 * dinky_count + 4
    u8 fat_qwc_plus_six;         // 9 = 3 * fat_count + 6
    u8 dinky_count;              // 10
    u8 fat_count;                // 11
};
static_assert(sizeof(TieUnpackHeader) == 12);

struct TieStrip { u8 vertex_count, pad, gif_tag_offset, winding; };

struct TieDinkyVertex { s16 x, y, z; u16 gs_slot; s16 s, t; u16 q, gs_slot_2; };
static_assert(sizeof(TieDinkyVertex) == 0x10);
struct TieFatVertex { s16 dx, dy, dz; u16 gs_slot; s16 x, y, z; u16 pad; s16 s, t; u16 q, gs_slot_2; };
static_assert(sizeof(TieFatVertex) == 0x18);

struct TieAdGifs { AdGif tex0, tex1, miptbp1, clamp, miptbp2; };
static_assert(sizeof(TieAdGifs) == 0x50);

// One vertex as VU1 processes it (dinky vertices first, then fat), resolved.
struct TieVertex {
    s16 x, y, z;                 // class-space position (fat: the base position)
    s16 dx, dy, dz;              // fat: LOD-morph delta added as delta * k (per-instance k); 0 for dinky
    s16 s, t;                    // texture coordinates, 1/4096
    u16 q;
    u16 gs_slot;                 // GS-packet quadword this vertex is written to
    u16 gs_slot_2;               // second slot (double-write phase), 0 if none
    u8 color;                    // light/colour slot (0..63): instance palette entry and class normal
    u8 morph_color[2];           // fat: the two slots averaged in as the vertex morphs; = color for dinky
    u8 fat;
};
static_assert(sizeof(TieVertex) == 26);

struct TieDraw {                 // one GS triangle strip in GS-packet order
    u8 ad_gif = 0;               // material: index into the class ad-gifs and TieClassEntry::textures
    u8 winding = 0;
    std::vector<u16> vertices;   // indices into TiePacket::vertices
};

struct TieTriangle { u16 a, b, c, ad_gif; };

struct TiePacket {
    TiePacketHeader header;
    s32 ad_gif_dest[4], ad_gif_src[4];
    TieUnpackHeader unpack;
    std::vector<TieStrip> strips;
    std::vector<TieDinkyVertex> dinky;
    std::vector<TieFatVertex> fat;
    std::vector<u8> colors, colors_b;   // colour-index copies for VU buffers A and B (B = A + 0x40)
    std::vector<u8> slot_table;         // slot_table_size quadwords, raw
    std::vector<TieVertex> vertices;    // VU processing order
    std::vector<TieDraw> draws;
};

struct TieClass {
    TieClassHeader header;
    std::vector<u8> header_ext;          // 0x80 .. first packet table: bbox min/max vec4s, then 8 corner points
    std::vector<s16> normals;            // 64 x (x, y, z, 0), 1/32767
    std::vector<TiePacket> lods[3];
    std::vector<TieAdGifs> ad_gifs;
};

TieClass parse_tie_class(Buffer blob);
std::vector<TieTriangle> tie_triangles(const TiePacket& p);

// Gameplay tie instance, 0xe0 bytes on disk.
struct TieInstance {
    s32 o_class;
    s32 draw_distance;  // world units, s32 not f32: TieProc does cvt.s.w then min with the loader's 720.0 cap; 0 = never drawn (720 on every Novalis tie)
    s32 pad_8;
    s32 occlusion_index;
    f32 matrix[4][4];   // column-major: matrix[col][row]; translation in column 3; [3][3] is 0.01 (or 0) on disk, kept raw
    u16 ambient_rgbas[64];   // RGBA5551 per light slot (LightTies expands with PEXT5)
    s32 directional_lights;
    s32 uid;
    s32 pad_d8, pad_dc;
};
static_assert(sizeof(TieInstance) == 0xe0);

std::vector<TieInstance> parse_tie_instances(Buffer section);

} // namespace rc
