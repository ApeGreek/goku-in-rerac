# Quartu (level 15): the batch C checklist

Batch C's seventeenth planet, done 2026-10-04 after Oltanis. Before this pass Quartu ran ported code for its alarms
408 and alarm drones 77, the shaking critters 221, the one-way belts 1250, the story pickup and director, the Giant
Clank mission NPC 1446, the copies of Oltanis's zapper bots 28, hatches 250 and arc keeper 1331, and the shared
crates, bolts and checkpoints. The census of 2026-10-03 left these classes without a port; this page tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 44 (+ 934) | the guards: watch, patrol, raise the alarm, shoot; fooled by the Hologuise | **ported** (`units::quartu_guard`) |
| 67 | the sliding doors | **ported** (`units::quartu_small`) |
| 78 | the energy barriers (also on the Fleet) | **ported** (`units::barrier_field`) |
| 92 | the gates (Giant Clank's sink) | **ported** (`units::quartu_small`) |
| 148, 255, 154 | the buildings and walls Giant Clank knocks down | **ported** (`units::quartu_building`) |
| 233 | the flame drones with their riders | **ported** (`units::quartu_hover`) |
| 491 | Giant Clank's jet robots | **ported** (`units::quartu_jet_bot`) |
| 655 | the electrified water (also on the Fleet) | **ported** (`units::water_shock`) |
| 1394 (+ 1257) | the bomb droppers and their bombs | **ported** (`units::quartu_small`) |
| 1408, 1409, 1565, 1567 | the rippling water meshes (also the Fleet's 1405..1407) | **ported** (`units::wave_mesh`) |
| 1430 (+ 1428) | the dispenser and its piece | **ported** (`units::quartu_small`) |
| 1560 | the scene thrusters | **ported** (`units::quartu_small`) |

## Shared-system changes in this pass

- Particle type 71 (the barrier's drifting motes), with the hero body point and the pvar heads the moby loop hands the
  particles.
- The water meshes' strips are read from the overlay at the level load (`rc-engine` fx_draw, like the Veldin pools).
- The electrified water draws its shock through the Tesla Claw's chain draw (`hero::tesla::beam_quads`).
- The alarms got the guards' poke (`quartu_alarm::poke`); Orxon's drone shot and muzzle flash are shared with the
  jet robots.

## Notes

- [L] 1430's fly-by camera (camera class 19) is not run (G-HERO-027); it is logged when armed.
- [L] A guard's shot whose timer runs out with no hit flies on (the game reads the last collision's moby there).
- [L] 1428 does nothing on level 15 (its own test); the rest of its code has no caller here.
- Headless screenshots in this session came out black on every level: a capture problem here, not a regression.

## Open

- The user's QA: docs/test-requests/2026-10-04-quartu.md.
