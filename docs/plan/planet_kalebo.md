# Kalebo III (level 16): the batch C checklist

Batch C's seventh planet, started 2026-10-03 after Pokitaru. Before this pass Kalebo ran ported code for its story,
the hoverboard race and racers, the rail cars, belts, mines, air traffic, the rising floats 650 and the barriers 647.
The census of 2026-10-03 left sixteen placed classes without a port; this page tracks them.

## Classes placed without a port (census 2026-10-03)

| class | placed | what | status |
|---|---|---|---|
| 1943, 1947, 1948, 1952, 1953 | 1 each | reflections: static env-mapped meshes, FX 0x29 at FIX 0x40, culled per mesh at 512 (1952 not culled) | **ported** (`water::sea` ports 14–18, `reflect_ref`) |
| 1439, 1441 | 3 + 3 | pass-through gates: pulse, click and set their command byte when Ratchet passes, rest 10 s | **ported** (`units::kalebo_gates`) |
| 1442 | 3 | rail switches: on for good when Ratchet leans into one while grinding past, or hits it | **ported** (`units::kalebo_rail_switch`) |
| 1891 | 2 | the spinning sign | **ported** (`units::kalebo_spinner`) |
| 1561 | 1 | the grind skill point 0x13d421 (cuboid to cuboid without leaving the rail) | **ported** (`units::kalebo_grind_skill`) |
| 1923 | 1 | the chicken pad: sends hidden chickens through teleporter pad #1192 | **ported** (`units::kalebo_chicken_pad`, `chicken::respawn_at`) |
| 1826 | 2 | lifts: call or ride to the other end of a cuboid; a column of glowing rings while moving | **ported** (`units::kalebo_lift`) |
| 1812 | 1 | the jets of actor 3 in the first scene | **ported** (`units::kalebo_scene_jet`) |
| 654 | 4 | arena triggers: barriers, the script camera's slide, the enemies, the spline wall, the arena flags 114.. | **ported** (`units::kalebo_arena`) |
| 1410 | 2 | the cars: ride between three stations on △, the spline wall rewritten round the car | **ported** (`units::kalebo_car`) |
| 541 | 29 | the arena troopers: grenadiers (281), sweepers and flamers on their paths, lives, their glow | **ported** (`units::kalebo_trooper`, `units::kalebo_grenade`) |

## Shared-system changes in this pass

- **`creature::flame`**: the fire-spraying creatures' emitter (level02 `0x264e70` and its byte-identical level16 copy
  `0x25eab8`, the same numbers), lifted out of the Aridia flamer with the record's offset as a parameter.
- **Sea meshes**: `MeshSet::far` (the sphere check's distance: 256, the reflections 512) and `SeaKind::Reflect`.
- **The chicken**: level16's own `0x2c44f0` (respawn at a point) and `0x2c4710` (hidden, free to send).
- **`World::sphere_mobys_list`**: the sphere query's list (the sweepers read its first moby).
- The teleporter's state 7 / 8 now has its user (1923).

## Notes

- The sweepers' hurt sound reads the collision output's moby 0x174258 as the sphere list's first [L].
- 1410's car writes its wall into the level's spline +0xa0 every ride tick, as the game does.
- 541's tick runs the flash update twice (the game's).
- Class 1377 reports "manipulator: target joint list not loaded" at load (head-turn lists 0 / 2); this predates the pass.

## Open

- The user's QA: docs/test-requests/2026-10-03-kalebo.md.
