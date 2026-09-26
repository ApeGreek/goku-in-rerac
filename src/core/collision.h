#pragma once
#include <vector>
#include "core/buffer.h"

namespace rc {

// Level collision block. Spec: docs/formats/collision_rac1.md.
// Uniform 4x4x4-unit cells indexed Z -> Y -> X through a three-level sparse table.
// Tree offsets (slab/row/leaf) are byte offsets from the mesh (header.mesh), not from the block.
struct CollisionHeader { s32 mesh, hero_groups; };
struct CollisionFace { u8 v[4]; u8 type; bool quad; };   // v[3] == v[0] for triangles
struct CollisionCell {
    s16 cx, cy, cz;                 // cell coordinates; centre = cell*4 + 2
    u32 leaf_word;                  // tree entry: (leaf offset from mesh) << 8 | leaf size in 16-byte units
    u16 face_count;                 // leaf header
    u8 vertex_count, quad_count;
    std::vector<u32> packed;        // raw packed vertices (x:10 y:10 z:12, signed)
    std::vector<f32> vertices;      // xyz world-space triples
    std::vector<CollisionFace> faces;
    std::vector<u8> quad_v3;        // trailing fourth index of each quad
};
struct HeroGroup {
    u16 raw[4];                     // sphere x, y, z, radius at 1/64
    u16 triangle_count, vertex_count;
    u32 data;                       // offset of the group data from the hero section
    f32 sphere[4];
    std::vector<u16> raw_vertices;  // x, y, z, pad per vertex (u16 at 1/64)
    std::vector<f32> vertices;      // xyz world-space triples (u16/64)
    std::vector<u8> triangles;      // v0 v1 v2 per triangle
};
struct Collision {
    CollisionHeader header;
    s16 z_base; u16 z_count;
    std::vector<u32> slab_offsets;  // every non-empty Z slab node, in Z order
    std::vector<u32> row_offsets;   // every non-empty Y row node, in walk order
    std::vector<CollisionCell> cells;
    s32 hero_group_count = 0;
    std::vector<HeroGroup> hero_groups;
};

// One world-space triangle of the main mesh. Quads split along v0-v2: (v0,v1,v2) then (v0,v2,v3).
struct CollisionTriangle {
    f32 a[3], b[3], c[3];
    u32 cell;                       // index into Collision::cells
    u16 face;                       // index into CollisionCell::faces
    u8 surface;                     // face type byte
    u8 part;                        // 0 = triangle face, 1 = first half of a quad, 2 = second half
};
static_assert(sizeof(CollisionTriangle) == 44);

Collision parse_collision(Buffer block);
std::vector<CollisionTriangle> collision_triangles(const Collision& c);

} // namespace rc
