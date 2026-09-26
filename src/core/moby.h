#pragma once
#include <vector>
#include "core/buffer.h"

namespace rc {

// Moby (animated object) class. Spec: docs/formats/moby_rac1.md.
struct MobyClassHeader {
    s32 packet_table_offset;
    u8 high_lod_count, low_lod_count, metal_count, metal_begin;
    u8 joint_count, unknown_9, rac1_byte_a, rac1_byte_b;
    u8 sequence_count, sound_count, lod_trans, shadow;
    s32 collision, skeleton, common_trans, joints;
    s32 gif_usage;
    f32 scale;
    s32 sound_defs;
    u8 bangles, mip_dist;
    s16 rac1_short_2e;
    f32 bsphere[4];
    s32 glow_rgba;
    s16 mode_bits;
    u8 type, mode_bits2;
};
static_assert(sizeof(MobyClassHeader) == 0x48);

struct MobyPacketEntry {
    u32 vif_list_offset;
    u16 vif_list_size;              // 16-byte units
    u16 vif_list_texture_unpack_offset;
    u32 vertex_offset;
    u8 vertex_data_size;            // 16-byte units, incl. header
    u8 unknown_d, unknown_e, transfer_vertex_count;
};
static_assert(sizeof(MobyPacketEntry) == 0x10);

struct MobyVertexTableHeader {     // RAC1 form
    u32 matrix_transfer_count, two_way_blend_vertex_count, three_way_blend_vertex_count, main_vertex_count;
    u32 duplicate_vertex_count, transfer_vertex_count, vertex_table_offset, unknown_e;
};
static_assert(sizeof(MobyVertexTableHeader) == 0x20);

struct MobyMetalVertexTableHeader { // spec 2.11 / docs/plan/moby_skinning_lighting.md 6
    s32 vertex_count;
    s32 unknown_4, unknown_8, unknown_c;   // output offsets of positions, colours, total output bytes
};
static_assert(sizeof(MobyMetalVertexTableHeader) == 0x10);

// Skin attributes resolved at load time by replaying the VU0 matrix-slot machine
// (docs/plan/moby_skinning_lighting.md 5): up to 3 joint-palette indices, weights /256.
struct MobySkin {
    u8 count = 0;          // 1..3 (0 only on a default-constructed vertex)
    u8 joints[3] = {};
    u16 weights[3] = {};   // sum to 256
};

struct MobyVertex {
    u8 raw[8];         // regular: type-dependent skinning bytes 0-7 (spec 2.9); metal: bytes 8-15 (joint[3], count, weight[3], pad)
    u8 normal_azimuth, normal_elevation;
    s16 x, y, z;       // world = raw * scale / 1024
    u16 id = 0;        // 9-bit vertex cache id (decoded from the +7 shifted scheme); metal: vertex number
    u8 type = 3;       // 1 two-way blend, 2 three-way blend, 3 regular, 4 metal
    bool duplicate = false;
    MobySkin skin;
};

// Normal from the packed spherical angles, as VU0 program 104691 builds it from the (cos, sin)
// table at 0x165500: x = cos a cos e, y = sin a cos e, z = sin e, a/e in 2pi/256 steps.
// (Wrench and moby_rac1.md before 2026-09-26 had x and y swapped.)
void moby_normal(u8 azimuth, u8 elevation, f32 out[3]);

struct MobyTriangle { u32 a, b, c; s32 material; };  // vertex indices into the packet's vertex list

struct MobyPacket {
    MobyPacketEntry entry;
    MobyVertexTableHeader vth{};
    MobyMetalVertexTableHeader metal_header{};
    std::vector<u8> transfers;          // matrix transfer records, 2 bytes each (spr joint, VU0 addr)
    std::vector<u16> duplicates;        // raw duplicate entries (cache id << 7)
    std::vector<u8> raw_vertices;       // in-file + epilogue vertex records (metal: vertex records)
    std::vector<u8> rgba_multipliers;   // RAC1 unknown_e blob: RGBA per transfer vertex, 0x80 = 1.0
    std::vector<MobyVertex> vertices;   // in-file vertices then duplicates (copies with fresh ST)
    std::vector<s16> st;                // s,t pairs, 4.12 fixed, indexed like vertices
    std::vector<u8> index_bytes;
    std::vector<u8> secret_indices;     // index header byte 2, then byte 0xc of each ad-gif quadword (one per ad-gif block)
    std::vector<s32> texture_indices;   // TEX0 data_lo per ad-gif block
    s32 initial_texture = -1;           // texture in effect at packet start (GS state carried from the previous packet)
    std::vector<MobyTriangle> triangles;
    bool metal = false;
    u32 unresolved_duplicates = 0;   // duplicate ids whose cache slot was never written in this LOD list
};

struct MobyClass {
    MobyClassHeader header;
    std::vector<MobyPacket> high_lod, low_lod, metal;
    std::vector<f32> skeleton;          // joint_count x 16 floats (Mat4 rows)
    struct Trans { f32 v[3]; u16 parent_offset, seventy; };
    std::vector<Trans> common_trans;
    std::vector<s32> sequence_pointers;
};

MobyClass parse_moby_class(Buffer blob);

struct MobyInstance {
    s32 size;
    s32 unknown_4, unknown_8, unknown_c, unknown_10, unknown_14;
    s32 o_class;
    f32 scale;
    f32 draw_distance;
    s32 update_distance;
    s32 unused_28, unused_2c;
    f32 position[3];
    f32 rotation[3];    // euler radians, applied X then Y then Z
    s32 group;
    s32 is_rooted;
    f32 rooted_distance;
    s32 unknown_54;
    s32 pvar_index;
    s32 occlusion;
    s32 mode_bits;
    s32 r, g, b;
    s32 light;
    s32 unknown_74;
};
static_assert(sizeof(MobyInstance) == 0x78);

std::vector<MobyInstance> parse_moby_instances(Buffer section);

} // namespace rc
