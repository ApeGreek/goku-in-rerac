# Eudora (level 04): the batch C checklist

Batch C's fifth planet, started 2026-10-02. Before this pass Eudora already ran ported code for its story characters,
the help director, the path riders, the crank lifts and followers and the switched movers. The census of 2026-10-02
(`cargo run -p rc-trace -- class-census`) left nineteen placed classes without a port; this page tracks them. Porting
aid: `rc-trace peek` (the overlay disassembled, gp words, instances, cuboids, splines, the class table).

## Classes placed without a port (census 2026-10-02)

| class | placed | what | status |
|---|---|---|---|
| 466, 480, 485, 486, 488, 490, 493, 494, 495, 498, 555 | 38 | the flying machines: path flight (the shared flyer driver), rotors, rider props 476–478, landing pads; a hit blows one apart and it returns when the camera looks away; ten hits give a skill point | **ported** (`units::eudora_flyers`) |
| 642 | 1 | a drifting speck on the level wind (the wind is never set on Eudora: it only sinks) | **ported** (`units::eudora_drifter`) |
| 1549 | 1 | the cutscene FX driver (the infobot's thrusters in scenes 0 / 1) | **ported** (`units::eudora_scene_fx`) |
| 584 | 3 | grabbable blocks (a carrier with a ledge record) | **ported** (`units::eudora_ledge_block`) |
| 340 | 63 (28 created) | the brawler bots: wander or patrol, circle in, swing; pushed back, blown up; engine hum, exhaust | **ported** (`units::eudora_brawler`) |
| 427 | 13 | the gunners: keep their distance in an area, fire bursts of gun shots (184), strike up close, dissolve when killed; gun 499 on joint 1 | **ported** (`units::eudora_gunner`) |
| 217 | 5 | the loggers carrying a prop (570): growl, walk a path, swing; flinch, stagger, knocked down, blown up | **ported** (`units::eudora_logger`; its leg-walker states are unreachable in the game) |
| 563 | 17 (10 created) | the leg walkers on the shared leg walker: chase inside an area, kick, dash away when knocked, chop wood (chips 1516) | **ported** (`units::eudora_walker`, `creature::legs`) |
| 86 | 3 | flock spawners: five class-85 members each, flocking around a centre that follows a path | open (the boids library `0x1f30e0` / `0x1f3a80`) |

## Shared-system changes in this pass

- `creature::legs`: the level-library leg walker (planted feet, hips, ground fit, gaits from the step ahead, body
  capsule); its gait records come from the sequence headers' +0x14 data (`moby_anim::gait_records`, loaded into
  `Services::gaits` for the classes that ask for joint lists).
- `World::coll_capsule`: the vertical capsule query for moby code.
- `knock::ballistic` (`0x250a78`): the ballistic arc set-up, shared by the Kerwan hound and layer and 217.
- `react::SEQ_TABLES`: 340's Suck Cannon sequence table.

## Notes

- The flyers' hit count (gp−0x7e08) lasts the power-on session in the game; the port's starts at 0 on each level load.
- A flyer's landing writes a 32-bit delay over the driver's 16-bit one, so the driver resumes at once and the flyer
  flies on while turning toward the pad (the game's).
- One gunner has no area path: the game prints it and dissolves the gunner at its first update.

## Open

- 86, the flocks.
- The user's QA: docs/test-requests/2026-10-02-eudora.md.
