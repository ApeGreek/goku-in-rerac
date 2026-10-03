# Blarg (level 06): the batch C checklist

Batch C's ninth planet, started 2026-10-03 after Rilgar. Before this pass Blarg ran ported code for its story NPCs,
the shuttle, the Clank station, the split doors, the creature wakers and the shared crates, bolts and checkpoints. The
census of 2026-10-03 left these placed classes without a port; this page tracks them.

## Classes placed without a port (census 2026-10-03)

| class | placed | what | status |
|---|---|---|---|
| 55, 1551 | 2, 1 | the landing bay doors; the scene FX driver | **ported** (`units::blarg_small`) |
| 827 | 169 | the crawlers: surface crawling, pods, the entry leap, goo, the gadgetbot pick | **ported** (`units::blarg_crawler`) |
| 1048 | 27 | the troopers: group arenas, the surround, jabs, the wall push | **ported** (`units::blarg_trooper`) |
| 1068 | 10 | the fire-wave bots: the rolling wall of fire, the swing | **ported** (`units::blarg_wave_bot`) |
| 1051 | 1 | the mini-boss: cutaways, phases, the boss meter, the arena wall | **ported** (`units::blarg_boss`) |
| 1123 | 4 | the energy barriers: four crackling beams, the hurt lines, switches | **ported** (`units::blarg_barrier`) |
| 1035 | 5 | the laser gates: nine shared beams, the touch hint, the generators | **ported** (`units::blarg_laser_gate`) |
| 1302 | 1 | the gadgetbot pads: the hologram arrow, the count, the links | **ported** (`units::blarg_bot_pad`) |
| 857 | 4 | Clank's gadgetbots: the bubble, the commands, the share-out of targets, the pads, the glow | **ported** (`units::blarg_gadgetbot`, with the bubble 302 and the marker 303) |
| 1118 | 1 | the launch tube ride | **ported** (`units::blarg_launch_tube`) |
| 1108 | 1 | the escape: the held groups, the countdown | **ported** (`units::blarg_escape`) |
| 1028 | 1 | the bridge Clank extends | **ported** (`units::blarg_bridge`) |
| 1062, 1083 | 1, 2 | the station glass, the breakable window and its shards | **ported** (`units::blarg_glass`) |

## Shared-system changes in this pass

- **The gameplay white fade**: `cinematic::set_white`, drawn by the fade pass (`rc-engine` scene_render).
- **`veldin_pads::push_countdown`**: the 2-D countdown digits (first consumer: 1108).
- **`fx::Goo` / `goo_burst_with`**: the goo burst's spread, lift and blob parameters (827).
- **`attack::swept_lines`**: hits along the lines between last tick's and this tick's segment (1068's swing).
- **`fx::part02_rec`**: a type-2 spawn that keeps its record.
- **`react::Wrappers::release_seq`** and `refuse()`: the reaction tables' refusal blend (827's table).
- **`projectile::part26_joint`**: the type-26 glow on a joint list (857's merge).
- The overlay meshes read at load: `LevelFx::blarg_glass` (1062 / 1083) and `LevelFx::blarg_bubble` (302).

## Notes

- The gadgetbot command menu (0x238b18.., G-UI-018) is not ported, so the bots only follow; attacking and going to a pad
  need its commands (`Bodies::command`, wired in the bots). Its listener count 0x17ec84 is not modelled.
- 302's draw blends at a fixed alpha 0x20 in the game; the port draws it as an alpha blend at 0x20 (the texture's
  alpha also counts) [L].
- 1051's camera type 0x12 release in its state 0xf is not modelled; the 20000-range voice streams are not extracted.
- 827's probe falls back to its own up where the game reads a stale collision normal [L].

## Open

- The user's QA: docs/test-requests/2026-10-03-blarg.md.
