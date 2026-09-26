# RAC1 collision format

Reverse-engineering notes for Ratchet & Clank (2002, PS2; Wrench game id `rac` / RAC1), derived by reading the Wrench source tree at `/Users/aslanhud/Globals/wrench` (GPL — used here as reference only; no Wrench code is reproduced). Everything below is attributed to the Wrench file it came from. Where I state something Wrench does not actually document, it is marked **(inference)** or listed under Unknowns.

Key sources:

| File | What it gives us |
| --- | --- |
| `src/engine/collision.cpp` | The only real definition of the on-disc collision block: header, three-level tree, leaf/vertex/face encoding, hero groups, export-to-mesh and rebuild algorithms, all limits |
| `src/engine/collision.h` | Public API shape (`read_collision` / `write_collision`, `CollisionOutput` / `CollisionInput`) |
| `docs/collision.md` | Prose: one baked mesh per chunk (per *level* in RAC1), 4×4×4 "octants", tree lookup, what the face type controls, hero collision semantics |
| `docs/collision_recovery.md` | Consequences of world-space baking; quantisation artefacts; the Collision Fixer algorithm |
| `src/wrenchbuild/level/collision_asset.cpp` / `.h` | Asset-level unpack/pack, per-game registration, baking of instance collision via `has_static_collision` |
| `src/wrenchbuild/level/level_core.h` / `.cpp` | `LevelCoreHeader` (collision pointer at 0x14), how the block's size is inferred, chunk-collision padding |
| `src/wrenchbuild/level/level_chunks.cpp` / `.h` | Per-chunk collision in GC/UYA/DL chunk files |
| `src/iso/table_of_contents.h`, `src/wrenchbuild/level/level_wad.cpp` | RAC1 level WAD header has no chunk ranges → one collision block per level |
| `src/instancemgr/gameplay.cpp`, `gameplay_impl_misc.inl`, `gameplay_impl_env.inl`, `gameplay_impl_common.inl` | Gameplay block table for RAC1, the 0x80-byte shape record, camera collision grid, point light grid |
| `src/engine/occlusion.cpp` / `.h`, `docs/occlusion_culling.md` | The sibling 4×4×4 octant tree (same idea, different encoding) — useful cross-check |
| `src/editor/instanced_collision_recovery.{h,cpp}`, `src/editor/gui/collision_fixer.cpp` | Collision Fixer parameters/defaults |
| `src/engine/moby_low.{h,cpp}` | Moby-class collision blob (separate system) |
| `src/instancemgr/instance_schema.wtf`, `src/editor/gui/inspector.cpp` | `has_static_collision` is a Wrench-side authoring flag, not disc data |

Units: "game units" = the units used in the level files. `docs/collision.md` calls a cell "4x4x4 in metres/game units"; `src/engine/collision.cpp` comments likewise.

---

## 1. Where the collision block lives

### 1.1 Level core

`LevelCoreHeader` (`src/wrenchbuild/level/level_core.h`, 0xbc bytes, sits at the start of the level core *index* section) has `s32 collision` at **0x14**. The value is a byte offset into the **decompressed** level-core *data* section, the same space `tfrags` (0x08), `occlusion` (0x0c) and `sky` (0x10) point into.

**No size is stored.** `level_core_block_range()` in `src/wrenchbuild/level/level_core.cpp` recovers the size by collecting every known block boundary in the data section — the tfrag/occlusion/sky/collision offsets, `textures_base_offset`, `assets_decompressed_size`, every moby/tie/shrub class `offset_in_asset_wad`, `moby_sound_remap_offset`, the 256 ratchet-seq offsets, and (RAC1 only) the gadget entries at `gadget_offset_rac1`/`gadget_count_rac1` — and taking the next boundary above `collision`. So Novalis's 0x1957c0 is a *derived* size (distance to the next block), and the tail may legitimately contain alignment padding. When packing, Wrench aligns the collision block to 0x40 before writing it.

For the RAC1 header specifically, note 0x80/0x84 are `gadget_count_rac1`/`gadget_offset_rac1` (they are GS-stash fields in later games), and 0x78 is `ratchet_seqs_rac123`. The collision pointer is at 0x14 in all four games.

### 1.2 One block per level in RAC1

`docs/collision.md`: the collision for all static geometry (tfrags, ties, shrubs) is baked into a *single logical mesh* per chunk, "or in the case of R&C1, a given level".

This is structural, not a convention: `RacLevelWadHeader` (`src/iso/table_of_contents.h`, 0x30 bytes) contains only `header_size`, `id`, `data`, `gameplay_ntsc`, `gameplay_pal`, `occlusion` — there are no chunk sector ranges, whereas `GcUyaLevelWadHeader`/`DlLevelWadHeader` (`src/wrenchbuild/level/level_wad.cpp`) embed a `ChunkWadHeader` with three chunk ranges and three chunk sound banks. `unpack_rac_level_wad()` correspondingly never calls `unpack_level_chunks()`.

In the later games, chunk 0's collision lives in the level core exactly as in RAC1, and chunks 1–2 carry their own copies in the chunk files (`ChunkHeader { s32 tfrags; s32 collision; }`, LZ-compressed, in `src/wrenchbuild/level/level_chunks.cpp`). `pack_level_core()` writes chunk 0's collision at `header.collision` and then pads with zeroes up to the size of the *largest* chunk's collision, i.e. the core reserves a buffer big enough for any chunk to be streamed in over it **(inference about intent; Wrench only comments "Insert padding so there's space for the collision from the other chunks")**. For RAC1 there is only one chunk, so no reserve padding.

Wrench's asset model keeps collision under a Chunk asset even for RAC1 (`CHANGELOG.md`: tfrag and collision assets are "now only stored inside Chunk assets, even in the case of R&C1").

### 1.3 Format identity across games

`src/wrenchbuild/level/collision_asset.cpp` registers the *same* unpack and pack function for `rac1`, `rac2`, `rac3` and `dl`, and `src/engine/collision.cpp` has no `Game` parameter anywhere. So as far as Wrench is concerned the collision container is byte-for-byte the same format in RAC1 as in the sequels; the RAC1 differences are all about *where* it lives and what else is in the level (section 5).

---

## 2. Collision block layout

All offsets in this section are relative to the **start of the collision block** unless stated otherwise.

### 2.1 Block header (8 bytes)

| Offset | Type | Name | Meaning |
| --- | --- | --- | --- |
| 0x00 | s32 | `mesh` | Offset of the main collision mesh tree. In practice 0x40 (Wrench pads to 0x40 after the header). |
| 0x04 | s32 | `hero_groups` | Offset of the hero-collision section, or **0** if the level has none. |

From `read_collision()` in `src/engine/collision.cpp`. The main mesh's extent is taken as `hero_groups - mesh` when hero groups exist, else "to the end of the block" — i.e. the mesh size is not stored and the hero section must follow the mesh. Bytes 0x08–0x3f are padding.

### 2.2 The spatial hierarchy

A three-level, coordinate-indexed, sparse array ("tree" in the docs) over a uniform grid of **4×4×4-unit cells aligned to a 4×4×4 boundary** (`CollisionOctant` comment, `src/engine/collision.cpp`). Wrench calls a cell an *octant*; it is really a uniform grid cell, not an octree node.

Lookup order is **Z, then Y, then X** — the source comment states a cell at (x,y,z) is reached by taking the z-th child of the root, the y-th child of that, then the x-th child of that.

All node offsets are **relative to `mesh`** (the start of the tree), not to the block or to the parent node.

**Level 0 — root (at `mesh`)**

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | s16 | `z_base` — cell Z coordinate of entry 0 |
| 0x02 | u16 | `z_count` |
| 0x04 | u16 × `z_count` | Z entries. Byte offset from `mesh` = **value × 4**. Value 0 = empty slab. |

**Level 1 — one per non-empty Z slab (4-byte aligned)**

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | s16 | `y_base` for this slab |
| 0x02 | u16 | `y_count` |
| 0x04 | u32 × `y_count` | Byte offset from `mesh` of the row node. 0 = empty row. |

**Level 2 — one per non-empty Y row (4-byte aligned)**

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | s16 | `x_base` for this row |
| 0x02 | u16 | `x_count` |
| 0x04 | u32 × `x_count` | Packed leaf pointer, see below. Whole word 0 = empty cell. |

**Leaf pointer word:** bits 8–31 = byte offset from `mesh` of the leaf; bits 0–7 = leaf size in 16-byte units. Wrench's reader takes `value >> 8` and ignores the low byte; the writer emits `(size/0x10) | (offset << 8)`. Since leaves are 0x10-aligned, the low four bits of the offset are always zero, so nothing is lost. The size byte is most plausibly a DMA length for uploading the leaf to scratchpad/VU memory **(inference — Wrench never reads it)**.

Note the asymmetry: Z offsets are u16 scaled by 4, Y and X entries are full u32. Occlusion's tree (`src/engine/occlusion.cpp`) is the same shape but uses u16-scaled-by-4 at *both* internal levels, unsigned coordinates, and a 0xffff sentinel for empty leaves — so do not assume the two are interchangeable.

**Cell coordinates.** A leaf reached at index (i, j, k) over (x, y, z) has cell coordinates `(x_base + i, y_base + j, z_base + k)`, and its **cell centre** in world space is

```
centre = (cell_x * 4 + 2, cell_y * 4 + 2, cell_z * 4 + 2)
```

(`read_collision_mesh()` computes exactly this as the per-cell `displacement`.) The cell therefore spans `[cell*4, cell*4+4)` on each axis. Coordinates are signed on disc (s16), but see section 3 — Wrench's builder cannot emit negative ones.

**(Inference, for the port)** the runtime lookup is presumably: `cell = floor(pos / 4)` per axis; index the Z table with `cell_z - z_base` (bounds-checked against `z_count`), then Y, then X; a zero entry or out-of-range index means "no static collision here". Wrench does not document the game's clamping behaviour.

### 2.3 Leaf ("octant") payload

16-byte aligned. Header is 4 bytes:

| Offset | Type | Name | Notes |
| --- | --- | --- | --- |
| 0x00 | u16 | `face_count` | Total faces (quads **+** triangles) |
| 0x02 | u8 | `vertex_count` | Max 255 |
| 0x03 | u8 | `quad_count` | Max 255; must be ≤ `face_count` (Wrench errors out otherwise) |

Then, contiguously:

1. `vertex_count` × **u32 packed vertex** (4 bytes each)
2. `face_count` × **4-byte face record** — the first `quad_count` records are the quads, the remaining `face_count - quad_count` are triangles
3. `quad_count` × **u8**, the fourth index of each quad, in the same order as the quad records
4. padding to the next 0x10 boundary

Total size = `4 + 4·V + 4·F + Q`, rounded up to 0x10, and must be **< 0x1000** (the low-byte size field only holds 255 units of 0x10).

**Packed vertex (u32).** Bit layout, from low to high: X in bits 0–9, Y in bits 10–19, Z in bits 20–31 — all **signed** two's-complement fields. The scales differ per axis:

| Field | Bits | Signed range | Scale | World range relative to cell centre | Precision |
| --- | --- | --- | --- | --- | --- |
| X | 0–9 (10) | −512 … 511 | 1/16 | −32 … +31.9375 | 0.0625 |
| Y | 10–19 (10) | −512 … 511 | 1/16 | −32 … +31.9375 | 0.0625 |
| Z | 20–31 (12) | −2048 … 2047 | 1/64 | −32 … +31.984375 | 0.015625 |

World position = cell centre + decoded offset. The ±32 range is far larger than the 4-unit cell, which is what allows a large face straddling several cells to keep all of its vertices in each cell it is inserted into. The X/Y-vs-Z asymmetry (Z has four times the precision from two extra bits) is what `read_collision_mesh`/`write_collision_mesh` implement consistently in both directions, so it round-trips — but see Unknowns; getting Z's scale wrong would be a silent geometry error.

Wrench's writer truncates toward zero when quantising (`(s32)(v * 16.f)`), so up to one quantum of error per component; `docs/collision_recovery.md` explicitly calls out "quantization artifacts introduced by how the game stores the world-space collision meshes" and the Collision Fixer defaults to a 0.25-unit vertex merge distance to compensate.

**Face record (4 bytes).**

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | u8 | `v0` — index into this leaf's vertex array |
| 0x01 | u8 | `v1` |
| 0x02 | u8 | `v2` |
| 0x03 | u8 | **collision type / surface id** (see section 4) |

For a quad, the fourth index comes from the trailing byte array; the loop is `v0 → v1 → v2 → v3`. Wrench's exporter emits the reversed order (`v2,v1,v0` for triangles, `v3,v2,v1,v0` for quads) into the COLLADA mesh, and its importer reverses again, so **the on-disc winding is opposite to Wrench's mesh-space winding**. Whether the game relies on winding for facing at all is not documented.

Indices are leaf-local and bounded by `vertex_count`; Wrench only bounds-checks them for *hero* triangles, not for main-mesh faces.

### 2.4 How faces are grouped per cell

Every cell holds its own private vertex array and its own face list. A face that overlaps N cells is **duplicated into all N of them**, with its vertices re-encoded relative to each cell's centre. The comment in `collision_to_scene()` spells out why: "The vertices and faces stored in the games files are duplicated such that only one octant must be accessed to do collision detection." Hence the exporter has to deduplicate vertices and faces to recover a clean mesh, and hence a port can query exactly one cell per test point.

Ordering inside the cell is quads first, then triangles, then the quad `v3` bytes. Faces are not sorted by type; Wrench groups by type only when building the display mesh.

### 2.5 Hero collision section

Present only when the header's `hero_groups` is non-zero. Offsets below are relative to the **start of the hero section**.

`docs/collision.md`: hero collision affects only the player — invisible walls, fences, grates. It is *not* grid-partitioned; it is a flat array of groups, each with a bounding sphere, so "for optimal runtime performance, all vertices in a given group should be close together". Hero faces must be triangles and **have no type byte**.

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | s32 | group count |
| 0x04 | — | padding to 0x10 |
| 0x10 | group record × count | 0x10 bytes each, see below |

Group record (0x10 bytes, `PackedHeroCollisionGroup`):

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | u16 | bounding sphere X, scale 1/64 |
| 0x02 | u16 | bounding sphere Y, scale 1/64 |
| 0x04 | u16 | bounding sphere Z, scale 1/64 |
| 0x06 | u16 | bounding sphere radius, scale 1/64 |
| 0x08 | u16 | triangle count |
| 0x0a | u16 | vertex count |
| 0x0c | u32 | offset of this group's data, relative to the hero-section start |

Group data: `vertex_count` × 8 bytes (`u16 x, u16 y, u16 z` at scale 1/64, then a u16 that Wrench *verifies is zero* — a non-zero value makes it reject the file as "unknown type of hero collision vertex"), immediately followed by `triangle_count` × 4 bytes (`u8 v0, u8 v1, u8 v2`, then a u8 likewise verified zero). Groups are written 0x10-aligned.

Because positions and the bounding sphere are **unsigned** u16 at 1/64, hero geometry is confined to roughly 0 … 1023.98 units per axis. That matches the 1024×1024-unit footprint implied by the gameplay grids in section 6 **(inference)**.

Two writer bugs in `write_hero_collision_groups()` worth knowing so you don't mirror them: the vertex quantisation casts to u16 *before* multiplying by 64 (so hero vertices are written with the wrong scale), and the bounding-sphere components are cast to unsigned without a range check.

---

## 3. Export to a mesh, and rebuild

### 3.1 Export (`read_collision` → `collision_to_scene`)

1. Walk the tree in Z→Y→X order. For each leaf, append `cell_centre + decoded_offset` for every vertex to one big mesh.
2. For each face, look up (or create) a submesh keyed by the face's type byte, and append the face with reversed winding (`v2,v1,v0`, or `v3,v2,v1,v0` for quads). The submesh's material index *is* the type byte, so there is one submesh per distinct surface id actually used.
3. Deduplicate vertices, then deduplicate faces (`core/mesh.cpp`) — this undoes the per-cell duplication described in 2.4. Vertex equality uses a 1e-5 epsilon per component.
4. Each hero group becomes its own mesh named `hero_collision_group_<n>`, in world space already, with a single material (Wrench uses index 256, "hero_group_collision", flat blue).

`unpack_collision_asset()` in `collision_asset.cpp` writes all of this to one `collision.dae`, with the main mesh keeping the name `collision`, plus a `materials` collection of `CollisionMaterial` assets mapping material name → collision id. The 256 materials are named `col_0` … `col_ff` and get debug colours (`create_collision_materials()`), using a formula lifted from Replanetizer: `r = ((id & 0x03) << 6)/255`, `g = ((id & 0x0c) << 4)/255`, `b = (id & 0xf0)/255`.

`wrench unpack_collision <file>` (see `src/wrenchbuild/main.cpp`) will do this for a loose collision block, which is the fastest way to sanity-check a port's reader against Wrench.

### 3.2 Rebuild (`build_collision_octants` + `optimise_collision` + `write_collision_mesh`)

Input is one merged world-space mesh whose submesh materials carry collision ids 0–255 (anything outside that range is an error).

Grid construction, per face:

1. Take the 3 or 4 vertices (for a triangle, `v3` is aliased to `v0`).
2. Compute a cell-space AABB: per vertex and per axis, `min = (s32)(coord * 0.25f)` and `max = ceilf(coord * 0.25f)`. Note `(s32)` truncates **toward zero**, so this is not a floor for negatives.
3. If min == max on an axis (the face lies exactly on a cell plane), widen that axis by one cell in each direction.
4. Clamp each min to 0. Together with (2) this means **the builder cannot place anything at negative cell coordinates**, even though the on-disc coordinates are signed.
5. For every cell in that AABB, run a separating-axis AABB-vs-triangle test (13 axes: 9 edge-cross-axis, the 3 face normals, and the triangle normal) against a cube of half-extent 2 centred on the cell centre. Quads are tested as the two triangles `(v0,v1,v2)` and `(v2,v3,v0)` and accepted if **either** hits.
6. On acceptance: create the cell if needed (the sparse arrays grow and shift their base coordinate as required — `lookup_octant`), then merge each vertex into the cell's array with a 1e-5 epsilon, and append the face (indices reversed again) with the material's collision id as the type byte. Wrench asserts each face lands in at least one cell.

Optimisation pass, per cell (`optimise_collision`): if a quad has exactly one of its two triangles intersecting the cube, the quad is replaced by that single triangle — both diagonal splits are tried (`(v0,v1,v2)`/`(v2,v3,v0)` first, then `(v1,v2,v3)`/`(v3,v0,v1)`). Then dead faces are dropped, and unreferenced vertices are removed and indices renumbered.

Serialisation order in `write_collision_mesh`: root node → all Z-slab nodes (4-aligned, written even when their row count is 0) → all Y-row nodes (4-aligned; an empty row writes 0 into the parent slot) → all leaves (0x10-aligned; an empty cell writes 0 into the parent slot).

**Limits and failure modes** (all from `write_collision_mesh`, and worth mirroring as validation in a port):

| Condition | Wrench behaviour |
| --- | --- |
| `vertex_count` ≥ 256 in a cell | **cell silently dropped** (pointer set to 0) with a warning — collision hole |
| `quad_count` ≥ 256 in a cell | **cell silently dropped** with a warning |
| faces in a cell ≥ 65536 | hard error |
| leaf payload ≥ 0x1000 bytes | hard error |
| Z/Y/X count ≥ 65536 | hard error |
| A Z-slab node farther than 65535×4 bytes (256 KiB) from `mesh` | hard error ("offset too high") — because Z entries are u16-scaled-by-4 |

(The warning messages themselves print the wrong coordinates — they use the X base twice — a cosmetic bug in `write_collision_mesh`.)

### 3.3 What gets baked in

`pack_level_collision()` (`collision_asset.cpp`) starts from the level's authored collision mesh, then appends, for each moby / tie / shrub instance whose `has_static_collision` flag is set **and** whose position maps to the chunk being built, the class's `static_collision` mesh transformed by the instance matrix. Chunk assignment uses `chunk_index_from_position()` (`src/instancemgr/level_settings.cpp`): two optional planes from level settings, returning 1, 2 or 0 — for RAC1 there is only chunk 0.

This is the reason `docs/collision_recovery.md` exists: since the original discs only contain the baked world-space result, per-class collision meshes for ties and shrubs are *not recoverable directly*. The Collision Fixer (`src/editor/instanced_collision_recovery.{h,cpp}`) transforms every face of the world mesh by each instance's inverse matrix, counts how many instances of a class agree on a face, and keeps faces above a hit threshold; defaults are `min_hits = 3`, `merge_dist = 0.25`, reject faces outside a bounding box = on. It notes the process neither strips the recovered faces from the world mesh nor sets the static-collision flags.

---

## 4. The collision type / surface id byte

**Wrench does not enumerate it.** There is no table, no enum, no named constants anywhere in the tree — `collision_id` is carried around as a plain 0–255 integer (`src/core/collada.h`: "Only used by the collision code"), materials are auto-named `col_<hex>`, and `CollisionMaterial.id`'s schema description (`src/assetmgr/asset_schema.wtf`) says only "The collision ID to use for this material. This controls the type of the surface."

All the semantics Wrench records is one sentence in `docs/collision.md`: the type determines

- whether the player can **walk** on the face or **slides off** it,
- the **sound effect** played when the player walks on it,
- whether the face is part of a **death** surface or a **magneboot** surface.

There is no mention of water, lava, or grind-rail surface ids in the collision context (grind rails in these games are *splines* — `GrindPathData` / `PathBlock` in `gameplay_impl_misc.inl` — not collision face types). So for a port this byte must be treated as an opaque index whose behaviour table has to be recovered from the game executable; only the debug colouring (section 3.1, shared with Replanetizer) exists on the tooling side. See Unknowns.

One structural fact that *is* known: the byte is per-face, in the face record's fourth byte, and hero collision has no equivalent.

---

## 5. RAC1 differences from later games

| Aspect | RAC1 | GC / UYA / DL |
| --- | --- | --- |
| Collision container format | identical (same `read_collision`/`write_collision`, no game switch) | identical |
| Number of collision blocks | **one per level**, in the level core data section only | chunk 0 in the level core plus up to two more in the chunk files (`ChunkHeader.collision`, LZ-compressed) |
| Level WAD header | `RacLevelWadHeader`, 0x30 bytes, no chunk ranges (`src/iso/table_of_contents.h`) | `GcUyaLevelWadHeader` (0x60) / `GcLevelWadHeader68` / `DlLevelWadHeader` carry a `ChunkWadHeader` with 3 chunk + 3 sound-bank ranges |
| Core reserve padding after collision | none needed (single chunk) | core reserves room for the largest chunk's collision |
| Level core header neighbours | 0x80/0x84 = gadget count/offset | 0x80/0x84 = texture index / GS-stash count, and DL moves other fields |
| Gameplay shape blocks | 0x60 cuboids, 0x64 spheres, 0x68 cylinders, 0x6c pills | RAC2/3: 0x68/0x6c/0x70/0x74; DL: 0x4c/0x50/0x54/0x58 |
| Camera collision grid | gameplay 0x84 | RAC2/3 0x88, DL 0x6c |
| Point lights | gameplay 0x7c, 0x20-byte float records, **plus a separate point light grid at 0x78** | RAC2/3 0x80 / DL 0x64: one block containing a built-in 0x800-byte mask grid followed by compact 0x10-byte lights; **no separate grid block** |
| Moby instance record | `RacMobyInstance`, 0x78 bytes, self-describing `size` field | different layouts |
| Occlusion | separate copy also written at the level-WAD level (`write_occlusion_copy`) | in-core only (plus DL art instances) |

Nothing in Wrench suggests RAC1's *collision leaves or tree* differ in any way from the sequels, and `collision_asset.cpp` deliberately registers one implementation for all four games. Treat that as strong but not absolute evidence.

---

## 6. Related sections and runtime notes

### 6.1 In-game usage, as far as Wrench documents it

Wrench is a build tool, not a decompilation, so this is thin:

- **Spatial query**: position → cell, via the Z→Y→X coordinate-indexed tree (`docs/collision.md`: "a tree is generated for efficiently looking up a given octant from a world space position"; the child-order comment in `collision.cpp`).
- **Single-cell sufficiency**: geometry is duplicated across cells specifically so that "only one octant must be accessed to do collision detection".
- **Hero collision broad phase**: per-group bounding spheres exist for runtime rejection, hence the "keep vertices in a group close together" advice in both `docs/collision.md` and the asset schema.
- **Leaf size byte**: present in the leaf pointer, unread by Wrench; most likely a transfer size **(inference)**.
- **Ray casts**: nothing. There is no mention of ray casting, sweeps, or any specific collision query in the Wrench tree.
- **Moby vs static collision**: entirely separate systems. Moby *classes* carry their own collision blob (`MobyClassHeader.collision` at 0x10, `src/engine/moby_low.h`): a 0x10-byte header (`u16 unknown_0`, `u16 unknown_2`, `s32 first_part_size`, `s32 third_part_size`, `s32 second_part_size`) followed by three parts. Wrench keeps parts 1 and 3 as opaque bytes and only interprets part 2 as an array of `s16 x, y, z, 0` vectors at scale **1/1024**. That format is essentially undocumented.
- **Static collision flags on instances**: `has_static_collision` exists on `Moby`, `Tie` and `Shrub` in `src/instancemgr/instance_schema.wtf`, is exposed in the editor inspector, and is read *only* by `pack_level_collision()`. Crucially it is **not present in any packed instance struct** — `RacMobyInstance`, `RacTieInstance`, `ShrubInstancePacked` have no such field and none of the `swap_instance` functions touch it. It is a Wrench-side authoring flag stored in the `.instances` WTF file (introduced in asset format version 26, `docs/asset_system.md`), meaning "bake this class's `static_collision` mesh into the level's world collision at build time". A port should not look for it on disc; at runtime there is only the baked world mesh.

### 6.2 The 0x80-byte gameplay shape record (cuboids / spheres / cylinders / pills)

One record type, `ShapePacked` (`src/instancemgr/gameplay_impl_misc.inl`), used unchanged by all four shape arrays and all four games. Each array is an instance block: a 0x10-byte `TableHeader` (`s32 count`, 0x0c bytes of zero padding — `gameplay_impl_common.inl`) followed by `count` × 0x80-byte records; blocks are 0x10-aligned within the gameplay file, whose header is a flat array of s32 block offsets (`read_gameplay`/`write_gameplay` in `gameplay.cpp`; RAC1 offsets in `docs/gameplay.md` and `RAC_GAMEPLAY_BLOCKS`).

| Offset | Size | Type | Meaning |
| --- | --- | --- | --- |
| 0x00 | 0x40 | 4 × Vec4f | Forward transform, glm/column-major (columns 0–3). Column 3 is the translation. |
| 0x40 | 0x30 | 3 × Vec4f | **Columns 0–2 of the inverse transform** — i.e. the inverse's rotation/scale part. The inverse's translation column is *not* stored. |
| 0x70 | 0x0c | Vec3f | Euler rotation, redundant with the matrix |
| 0x7c | 0x04 | f32 | Unused; Wrench writes 0.0 |

So: matrix **and** a partial inverse matrix, as the question suspected. `swap_matrix_inverse_rotation()` (`gameplay_impl_common.inl`) reconstructs the missing inverse translation column by inverting the forward matrix, and keeps the three on-disc inverse columns as authoritative (they may differ from a freshly computed inverse — the game's own precision).

Magic values in the w slots: on write, the forward matrix's `[3][3]` is set to **0.01** instead of 1.0, and on read it is forced back to 1.0; the same treatment applies to tie, shrub and sound instances. The intent is apparently `[3][3] = 0.01` in the forward matrix and `100.0` in the inverse (a reciprocal pair, purpose unknown), but note that because the stored inverse is truncated to three columns, the 100.0 never actually reaches the file. Any port reading these matrices must overwrite `[3][3]` with 1.0 before using them as transforms.

Geometry convention: the shape is a canonical primitive transformed by the matrix. The camera-collision grid builder samples the eight corners at ±1 on each axis, so a **cuboid is the [−1,1]³ cube transformed by the matrix** (half-extents come from the matrix). By analogy a sphere is the unit sphere, and cylinders/pills unit primitives along one axis, but Wrench never states which axis or how pill caps are defined — it treats all four types identically and only distinguishes them by which array they live in and by the type tag in the camera grid.

### 6.3 Camera collision grid (RAC1: gameplay block pointer at 0x84)

From `CamCollGridBlock` in `gameplay_impl_env.inl`. A 2-D grid over X/Y only.

| Offset | Size | Meaning |
| --- | --- | --- |
| 0x00 | 0x10 | `TableHeader` whose first s32 is used as a **size**, not a count: Wrench writes `total_block_bytes − 4`. Rest zero. |
| 0x10 | 0x40 × 0x40 × 4 | `s32` cell entries, index `y * 0x40 + x`. 0 = empty; otherwise a byte offset **relative to 0x10** (the start of the grid array) to that cell's primitive list. |
| … | | Primitive lists, each 0x10-aligned: a `TableHeader` whose first s32 is the primitive count, then that many 0x30-byte primitive records. |

Grid geometry: 64 × 64 cells, cell size **16 units** (the builder multiplies world coordinates by 0.0625), so the grid covers 0 … 1024 on X and Y with no Z subdivision.

Primitive record (0x30 bytes, `CamCollGridPrim`):

| Offset | Type | Meaning |
| --- | --- | --- |
| 0x00 | Vec4f | Bounding sphere; Wrench forces **z = 0** when writing, making it effectively a circle in the XY plane |
| 0x10 | s32 | Volume type: **3 = cuboid, 5 = sphere, 6 = cylinder, 7 = pill** (`CamCollGridVolumeType`; 0,1,2,4 unobserved) |
| 0x14 | s32 | Index into the corresponding shape array |
| 0x18 | s32 | Flags (meaning unknown) |
| 0x1c | s32 | Integer parameter (meaning unknown) |
| 0x20 | f32 | Float parameter (meaning unknown) |
| 0x24 | 3 × s32 | Padding; Wrench writes −1, 0, 0 |

Wrench's reader uses this block only to recover per-instance camera-collision parameters: a shape referenced anywhere in the grid gets `camera_collision().enabled = true` plus the flags/int/float values, and a bad type tag is a hard error. The writer regenerates the grid from scratch: transform the eight ±1 corners of the unit cube by the instance matrix, take the cell-space AABB of those corners (truncate for min, ceil for max), clamp to the grid, and add the primitive to **every** cell in that AABB — no per-cell intersection refinement, and no dedup. The bounding sphere is an approximate sphere over those eight corners with z zeroed.

### 6.4 Point light grid (RAC1 only: gameplay block pointer at 0x78)

From `PointLightGridBlock` in `gameplay_impl_env.inl`. Same outer shape as the camera grid:

| Offset | Size | Meaning |
| --- | --- | --- |
| 0x00 | 0x10 | `TableHeader` with the first s32 again used as `total_block_bytes − 4` |
| 0x10 | 0x40 × 0x40 × 4 | `s32` per cell, index `y * 0x40 + x`; 0 = empty, else byte offset relative to 0x10 |
| … | | Per-cell list, 0x10-aligned: `s32 count` immediately followed by `count` × `s32` **indices into the RAC1 point-light array** (no 0x10 header here, unlike the camera grid) |

Cells are 16 units square (0.0625 factor), 64 × 64 as above. Important caveat: **Wrench does not parse this block at all** — the read function is empty except for a `GAMEPLAY_DEBUG_LIGHT_GRID` debug path that dumps a PNG; the block is regenerated from the point-light array on write. The regeneration uses `radius × 0.2` as the world-space radius and adds the light to every cell in the radius-expanded AABB (floor/ceil, clamped to the grid). A helper for proper circle-vs-cell testing exists but is unused. So the 0.2 factor and the list layout should be re-verified against real data before trusting them.

The RAC1 point-light record itself is `PointLightPacked`, 0x20 bytes: `Vec3f position`, `f32 radius`, `Rgb32 colour`, then three unused u32s. For contrast, RAC2/3 (`GcUyaPointLightsBlock`) put a 0x800-byte grid **inside** the point-lights block — two 0x40-entry arrays of 16-byte bitmasks, one for X slabs and one for Y slabs, one bit per light, 16-unit slabs, max 128 lights — followed by 0x10-byte quantised lights (position and radius at 1/64, colour at 1/65535). That is a genuinely different design from RAC1's offset-list grid.

---

## 6b. Verified on the retail NTSC-U disc (2026-09-26)

`src/core/collision.cpp` implements §2.1–§2.5. `rc_extract collision --level N` walks the Z→Y→X tables of levels 0, 1, 5 and 18 (5,783–33,981 cells, 53k–161k faces) with every face index below its cell's vertex count and every hero-group pad field zero. Veldin finale (level 18) has 21 hero groups, so RAC1 does use them. The 10/10/12-bit vertex packing with the 1/16, 1/16, 1/64 scales produces a top-down collision map that coincides with the tfrag terrain footprint, so the X/Y/Z scales are right. Surface type bytes seen on Novalis: 0, 4, 8, 9, 10, 12, 31, 36, 42, 63, 72, 73, 74, 76, 95, 96, 127 (31 and 8 dominate); the enumeration remains to be recovered from the game code.

### 6c. Rust port and all-level verification (2026-09-26)

`crates/rc-formats/src/collision.rs` ports `src/core/collision.cpp` and is golden-tested byte for byte against `rc_extract collision` (`extracted/levels/NN/collision_dump.bin`: header, every tree node's raw bytes, per-cell record + packed words + decoded f32 vertices + face records + quad `v3` bytes, hero groups raw and decoded, and the triangle list) on all 19 levels. Facts established on the whole disc:

* `mesh` = 0x40 and bytes 0x08..0x40 are zero on every level. `hero_groups` is **non-zero on every level**, even the six with zero groups (the section is then just the `s32 0` + padding), and the last leaf always ends at or before it — so "mesh = [mesh, hero_groups)" is exact.
* The leaf word's low byte equals `ceil((4 + 4V + 4F + Q) / 16)` on all 429,484 cells; the parsers now reject a mismatch. Leaf padding is zero; no leaf is shared by two tree entries.
* Grid coordinates are all ≥ 0 (min cell 0,0,0 on level 14; max 200,220,111 on level 13); one cell (level 15) has zero faces.
* Totals: 429,484 cells, 4,687,748 vertices, 2,373,683 faces (1,312,557 quads), 3,686,240 triangles after splitting quads along v0–v2; 516 hero groups with 7,315 triangles.
* 39 distinct surface ids (face counts over all levels): 0:17150 1:27550 2:2279 3:1258 4:1669 5:586 7:2154 8:169357 9:24147 10:83616 11:11813 12:397417 13:3023 14:37 31:1480315 32:1025 36:625 40:304 41:733 42:904 44:1014 63:25753 71:713 72:335 73:4733 74:3080 76:7505 95:77279 96:56 105:169 106:743 108:1183 110:333 127:17554 131:2521 136:2942 137:63 140:1082 159:663. Their runtime meaning: `docs/plan/collision_queries.md`.
* Resolved from the game code (`docs/plan/collision_queries.md`, level01 addresses): Unknown 3 — the block at 0x211c98 inside `CollLine_Fix` decodes the vertices exactly as x,y = s10/16, z = s12/64 about `cell*4+2`; Unknown 4 — the size byte is the DMA qword count of the leaf's scratchpad upload; Unknowns 5/6 — queries outside `[0,1024)^3` return no hit, and the lookup (0x2117d0) reads the node bases unsigned (`lhu`), so negative cell coordinates are not supported at run time; Unknown 10 — faces are one-sided by default with normal `(v2−v0)×(v1−v0)`; quads split along v0–v2 as `(v0,v1,v2)+(v0,v2,v3)`; the type byte is a bitfield (bits 0–4 id, 5–6 footstep class, 7 query-exclusion bit). The game reads a hero group's triangle count as a byte (`lbu +8`); every retail group has ≤ 214 triangles.

## 7. Unknowns

Collision block:

1. **The 256 surface ids.** No enumeration exists in Wrench. Only the categories in `docs/collision.md` (walk/slide, footstep sound, death, magneboot) are attested. Water, lava, grind-rail-like behaviours are not attributed to this byte anywhere. This must come from the executable.
2. **Whether the type byte is a bitfield or a plain index.** The debug colouring splits it into 2+2+4 bits, but that is only Replanetizer's visualisation scheme, not a claim about the data.
3. **The X/Y (10 bits @ 1/16) vs Z (12 bits @ 1/64) vertex encoding.** Self-consistent in Wrench and round-trips, but not cross-checked against game code in any comment. Verify before trusting; an error here would produce subtly wrong geometry only on one axis.
4. **The leaf pointer's low size byte.** Written, never read. Purpose (DMA length?) unconfirmed; whether the game requires it to be exact is unknown.
5. **Negative cell coordinates.** On-disc coordinates are s16 and the reader handles them, but the builder clamps to ≥ 0 and truncates toward zero, so whether real levels or the runtime support negative grid coordinates is untested.
6. **Out-of-grid and empty-cell behaviour at runtime** (presumably "no collision", but not documented).
7. **Whether faces may reference vertices far outside their own cell in practice.** The encoding allows ±32 units; Wrench's builder does produce this (it inserts whole faces into each overlapping cell), but no limit on face size is documented.
8. **The 256 KiB reach of the Z-offset table** — is that a game constraint or only Wrench's serialisation choice?
9. **Whether RAC1 levels actually use hero collision groups**, and whether the RAC1 hero record is byte-identical (Wrench assumes yes for all games). Also unknown: what the group bounding sphere is used for beyond broad-phase rejection, and what the two verified-zero pad fields would mean if non-zero (Wrench treats non-zero as an unknown variant, implying it has seen the possibility).
10. **Winding/facing convention.** Wrench reverses indices on both import and export but never says which orientation the game treats as the front face.
11. **Face ordering requirements.** Quads-before-triangles is what Wrench writes and what its reader assumes; whether the game requires it (it very likely does, given the trailing `v3` array) is not stated.
12. **Whether the collision block's trailing bytes are meaningful.** Sizes are inferred, so the tail of Novalis's 0x1957c0 may be padding.

Gameplay-side:

13. **Camera collision primitive `flags` / int / float parameters** — completely unnamed in Wrench.
14. **The unused camera-grid type values** 0, 1, 2, 4.
15. **The `[3][3] = 0.01` / `100.0` matrix magic values** in shape and other matrix instances: purpose unknown, and the 100.0 does not survive into the file at all because the inverse is truncated to three columns.
16. **Canonical geometry of sphere / cylinder / pill volumes** (axis, cap definition) — only the cuboid's [−1,1]³ convention is implied.
17. **The RAC1 point light grid** is unparsed by Wrench; the list layout is only known from an unmaintained debug path, and the `radius × 0.2` factor comes from the writer alone.
18. **Whether the camera-collision and point-light grids share the same world origin/extent as the collision grid** (both are 64 × 16 = 1024 units on X/Y; the collision grid's s16 cell coordinates would allow far more). The 1024-unit footprint is an inference from grid sizes plus the unsigned 1/64 hero-collision coordinates.
19. **Moby class collision blob format** — three parts, only the middle one (s16 vectors at 1/1024) partially understood.
20. **Ray casts and any other query the game performs** — nothing in Wrench.
