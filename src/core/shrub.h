#pragma once
#include <vector>
#include "core/buffer.h"
#include "core/tfrag.h"   // AdGif

namespace rc {

// Shrub (small instanced prop) class. Spec: docs/formats/shrub_sky_rac1.md part 1. The packet
// semantics follow the VU1 shrub program 56467 (EE 0x101768; shrub_sky_rac1.md 1.3b), not only Wrench:
// * the three UNPACKs fill a 0x76-quadword input buffer; VU1 reads the header at qw 0, the GIF tags
//   from qw 1, the ad-gif blocks after them, vertex part 1 at `vertex_offset` and part 2 at
//   `vertex_offset + vertex_count` (by address, whatever unpack wrote there);
// * VU1 copies every GIF tag to its GS-packet slot, then every ad-gif block (A+D tag + 4 qw), then
//   writes each vertex's ST / RGBAQ / XYZF2 to slots off..off+2 for vertices 0 ..= stop + 3, where
//   `stop` is the first vertex from index 2 on with bit 15 of `n_and_stop` set (the loop never tests
//   vertices 0 and 1);
// * XGKICK starts the GIF at slot 0; the GIF reads tags (A+D blocks: 5 qw, vertex tags: 1 + 3 * NLOOP)
//   until the vertex tag with EOP.

struct ShrubClassHeader {
    f32 bsphere[4];
    f32 mip_distance;
    u16 mode_bits;
    s16 instance_count;        // runtime scratch
    s32 instances_pointer;     // runtime scratch
    s32 billboard_offset;      // 0 = none
    f32 scale;
    s16 o_class, s_class;
    s16 packet_count, pad_2a;
    s32 normals_offset;
    s32 pad_30;
    s16 drawn_count, scis_count, billboard_count, pad_3a[3];
};
static_assert(sizeof(ShrubClassHeader) == 0x40);

struct ShrubPacketEntry { s32 offset, size; };                     // at blob + 0x40
struct ShrubPacketHeader { s32 texture_count, gif_tag_count, vertex_count, vertex_offset; };  // VU qw 0
struct ShrubGifTag { u64 tag; u32 tag_hi; s32 gs_packet_offset; };  // first 12 bytes of a GIFtag + GS slot
static_assert(sizeof(ShrubGifTag) == 0x10);
struct ShrubAdGifs { AdGif tex1, clamp, miptbp1, tex0; };            // w lane of tex1 = GS slot of the block
static_assert(sizeof(ShrubAdGifs) == 0x40);
struct ShrubVertexPart1 { s16 x, y, z, gs_packet_offset; };
struct ShrubVertexPart2 { s16 s, t, q; u16 n_and_stop; };

// One vertex as VU1 processes it (index i reads part 1 / part 2 entry i), 16 bytes.
struct ShrubVertex {
    s16 x, y, z;          // raw; class space = raw * scale / 1024
    s16 gs_packet_offset; // GS-packet quadword of its ST (RGBAQ +1, XYZF2 +2)
    s16 s, t, q;          // 4.12 fixed; q multiplies the perspective Q (0x1000 on every retail vertex)
    u8 normal;            // 0..23: class normal / per-instance palette entry
    u8 stop;              // 1 = bit 15 of n_and_stop
};
static_assert(sizeof(ShrubVertex) == 16);

struct ShrubDraw {        // one vertex GIF tag in GS order
    u8 texture;           // tex0 data_lo of the ad-gif in effect: slot into ShrubClassEntry::textures
    u8 prim;              // GS PRIM type: 3 = triangle list, 4 = triangle strip
    std::vector<u16> vertices;   // indices into ShrubPacket::vertices
};

struct ShrubTriangle { u16 a, b, c, texture; };

struct ShrubPacket {
    ShrubPacketEntry entry;
    ShrubPacketHeader header;
    std::vector<ShrubGifTag> gif_tags;
    std::vector<ShrubAdGifs> ad_gifs;
    std::vector<ShrubVertexPart1> part1;   // vertex_count entries from the input buffer
    std::vector<ShrubVertexPart2> part2;
    std::vector<ShrubVertex> vertices;     // the stop + 4 vertices VU1 writes
    std::vector<ShrubDraw> draws;
};

struct ShrubNormal { s16 x, y, z, pad; };  // 8 bytes, /32767
static_assert(sizeof(ShrubNormal) == 8);

struct ShrubBillboard {    // at billboard_offset, 0x40 bytes
    f32 fade_distance, width, height, z_ofs;
    AdGif tex1, tex0, miptbp1;
};
static_assert(sizeof(ShrubBillboard) == 0x40);

struct ShrubClass {
    ShrubClassHeader header;
    std::vector<ShrubPacket> packets;
    std::vector<ShrubNormal> normals;      // 24 entries
    std::vector<ShrubBillboard> billboard; // 0 or 1 entry
};

ShrubClass parse_shrub_class(Buffer blob);
// Triangulates a packet: strips as (i-2, i-1, i) for even i, (i, i-1, i-2) for odd i (same as tie_triangles),
// lists three at a time.
std::vector<ShrubTriangle> shrub_triangles(const ShrubPacket& p);

struct ShrubInstance {
    s32 o_class;
    f32 draw_distance;
    s32 unused_8, unused_c;
    f32 matrix[4][4];     // column-major, [3][3] = 0.01 on disc (kept raw)
    s32 r, g, b;          // 0..255
    s32 unused_5c;
    s32 dir_lights;
    s32 unused_64, unused_68, unused_6c;
};
static_assert(sizeof(ShrubInstance) == 0x70);

std::vector<ShrubInstance> parse_shrub_instances(Buffer section);

} // namespace rc
