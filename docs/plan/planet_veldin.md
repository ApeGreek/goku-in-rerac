# Veldin (level 00, Kyzil Plateau): the batch C checklist

Batch C works planet by planet: every story beat, object and interaction of the level, on the shared systems. This
page tracks Veldin's first visit (level 00); the finale (level 18) gets its own page. Started 2026-10-02.

## The level's flow

1. New game → cards "Kyzil Plateau, Planet Veldin" → FMV `mpegs[40]` → card "Approaching Planet Veldin…" → `mpegs[42]`
   (cutscenes_transitions.md §4.3) → level 00.
2. Clank 834 (`units::veldin_story`) at the crash site (171, 271): his first update hides Clank on Ratchet's back
   (0x141628) and sets global flag 8; his talk (radius 255 while flag 8 was clear) starts **scene 4** at once (818
   ticks: Ratchet at his ship, actors 0 / 110 / 530 / 1515 / 1365), then play starts at the ship (132, 115).
3. Ratchet crosses the plateau north (crates, bolts, the horny toads 749, the 1440s around the crash site, the fire
   fields 760 of the wreck) to Clank: his talk (radius 3, or 16 inside his cuboid +0x60) chains scenes 1 → 2 → movie 1
   → 3 (interaction.md §8); a node past 3 played → `0x2a29a0(1)`: the trip to Novalis.

## Classes placed (level00 class table; census 2026-10-01)

| class | placed | what | status |
|---|---|---|---|
| 0 | 1 | Ratchet | hero |
| 13 | 29 | bolts | ported (`pickup`) |
| 500 / 501 / 511 | 103 / 2 / 6 | crates | ported (`crate_`) |
| 530 | 1 | Ratchet's ship (hidden in the scenes with their own ship; canopy glass) | **ported 2026-10-02** (`units::veldin_ship`; the glass's near / far cross-fade in `rc-engine` fx_draw) |
| 749 | 16 | horny toads | ported (`units::horny_toad`) |
| 760 | 6 | the wreck's fire fields | ported (`fire_field`) |
| 809 | 1 | texture scroll | ported |
| 834 | 1 | Clank (the talk, the exit) | ported (`units::veldin_story`) |
| 1060 | 21 | lamps | ported (`units::lamp`) |
| 1413 | 1 | the help director | ported (`units::help_veldin`) |
| 1440 | 9 | the beam drones around the crash site (fly in along a path, fire a crackling beam, two hits; #197 zaps toad #154, its second target +0x1bc) | **ported 2026-10-02** (`units::veldin_beamer`) |
| 1471 | 1 | the beam manager (three slots 0x161bf8.., the strands, sparks, point lights and draw `0x2e2af0`) | **ported 2026-10-02** (`units::veldin_beamer`; the draw through `Callback::UnitFrame` + a multi-group `UnitQuads`) |
| 1545 | 1 | the cutscene FX driver (the infobot's thrusters in scenes 2 / 3, scene 4's dust) | **ported 2026-10-02** (`units::veldin_scene_fx`) |
| 1564 | 25 | path gliders | ported (`units::path_glider`) |
| 1781 / 1782 | 33 / 36 | grass | ported |

Scene-only actor classes (no update): 110, 1365, 1515.

## The hero on Veldin

Level 00's collision uses surfaces 9 (97 triangles), 0xa (3,649), 0xc (38,505) and 0x1f (43,933) (2026-10-02 dump of
the world mesh). Level 00 runs the superset reaction (`hero::surface::LEVEL_RULES[0]`), checked against L00 `0x20b960`:
0xc is the pit flag 0x14063a (the 0x79 pit fall), 9 sets 0x14063e (read by the grind jump test, L00 `0x229b70`,
ported in `hero::boots`), 0xa and 0x1f have no rule in any level. No slippery (7), magnetic (2) or liquid floor, so
level 00's extra hero code behind those surfaces (G-HERO-031) stays inert on this visit.

## Play-through (2026-10-02, smoke runs)

Verified: the opening scene 4 at the level's start; the walk to Clank; the drones waking in their regions and
firing (Ratchet loses health, #197 zaps its toad); Clank's talk chaining scenes 1 → 2 → movie 1 (`mpegs[3]`) → 3; the
travel to Novalis's space flight; the help desk's first message. Test requests: docs/test-requests/2026-10-02-veldin.md.

## Open

- Nothing Veldin-specific left in the port's code. Hands-on QA by the user (the test request) and the finale visit
  (level 18, its own page) remain.
