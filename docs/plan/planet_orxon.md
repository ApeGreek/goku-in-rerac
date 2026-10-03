# Orxon (level 10): the batch C checklist

Batch C's thirteenth planet, done 2026-10-03 after Gaspar. Before this pass Orxon ran ported code for its help
director 1344, the path scouts and swoop flyers 1196 / 1199, the brawlers 1202, Clank's section 22, the trip blocks
1015 / 1282, the particle vents 1544, the spark fountains 1240, the Magneboots pickup 18, the Nanotech seller 1326 and
the shared crates, bolts and checkpoints. The census of 2026-10-03 left these classes without a port; this page
tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 939, 938 | the lava spouts and their glowing rocks: thrown, bouncing, cooling, batted back, bursting | **ported** (`units::orxon_lava`) |
| 857 | the gadgetbots (Blarg's code, its own row: the code identity did not pair them) | **ported** (`units::blarg_gadgetbot`) |
| 1067, 1073, 794 | the energy barriers, their strand field and their generators (zap, switch off, fade) | **ported** (`units::orxon_curtain`) |
| 1100, 1033, 1031, 353, 1117, 1421, 1424, 1555 | the cracked walls, lift, pressure plates, sliding gate, sinking platforms, bridge, sliding block, scene thrusters | **ported** (`units::orxon_small`) |
| 1346 | the guard turret (Gaspar's cannon shell) | **ported** (`units::orxon_small`) |
| 1047, 1122, 1921 | the generator core and its rubble | **ported** (`units::orxon_small`) |
| 947, 1090 | the live wires and their sparks | **ported** (`units::orxon_wire`) |
| 351, 1301 | Clank's teleport pads | **ported** (`units::orxon_pads`) |
| 702 | the flame vents | **ported** (`units::orxon_flame`, particle type 40) |
| 1229, 819 | the gun drones and their shots | **ported** (`units::orxon_drone`) |
| 1378 | the air curtains: no air on their front, the six-layer shimmer | **ported** (`units::orxon_airlock`) |

## Shared-system changes in this pass

- **Particle type 40** (the flame line, `particles::type40`): its fresh lines are queued and run against the mobys by
  the next tick's moby loop, which delivers their hits.
- **Follow camera through cinematic calls**: the horizontal spring, stick off, the smoothed look point, the leash and
  the row blend (the pads' settle).
- **`path::at_distance`**: a point at a distance along equal segments (the live wires).
- **Subtractive FX quads**: `FxQuads::subtract` / `FxGroup::subtract` draw a group through the engine's subtract
  blend (`(Cd − Cs)·FIX`), the air curtains' odd layers.

## Notes

- [L] The flames pass through the mobys they hit instead of bouncing off them (the particles only see the world mesh).
- [L] The fly-by camera the gate, bridge and block arm (class 19) is logged as unported, as on Qwark's ship.
- [L] The drones' help guard 0x141c48 is read as 0.
- [L] The air curtains' VSync count is the draw tick.
- The rubble 1122 / 1921 has empty updates in the game.

## Open

- The user's QA: docs/test-requests/2026-10-03-orxon.md.
