# Oltanis (level 14): the batch C checklist

Batch C's sixteenth planet, done 2026-10-04 after Gemlik. Before this pass Oltanis ran ported code for its
switchboard 1397, the pop-up turrets 30 and their shots 681, the story NPCs (Qwark 851, the scrap merchant 924, the
Morph-o-Ray 1354) and the shared crates, bolts and checkpoints. The census of 2026-10-03 left these classes without
a port; this page tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 8 | the searchlight sentries (beam, spot, turret alarm, the group's draw master) | **ported** (`units::oltanis_sentry`) |
| 28 (+ 325, 403) | the zapper bots, the pieces they leave | **ported** (`units::oltanis_zapper`) |
| 31 (+ 81, 1193) | the grenade drones, their pieces and grenades | **ported** (`units::oltanis_drone`) |
| 211 | the grind-rail bots | **ported** (`units::oltanis_rail_bot`) |
| 250, 309, 643, 1352, 1395, 1416, 1559 | hatches, leaning floats, floating mines, risers, the stair builder, pressure pads, the skill point's thrusters | **ported** (`units::oltanis_small`) |
| 386 | the rail arcs | **ported** (`units::oltanis_arc`) |
| 557 | the pull-target gliders | **ported** (`units::oltanis_glider`) |
| 610 | the wind tunnels | **ported** (`units::oltanis_wind`) |
| 684 | the arrival scene and the lightning strikes | **ported** (`units::oltanis_lightning`) |
| 685 | the ride cart | **ported** (`units::oltanis_cart`) |
| 712 | the ferries | **ported** (`units::oltanis_ferry`) |
| 903 | the path platforms (Oltanis's copy) | **ported** (`classes::path_platform`) |
| 908, 921, 922 | the fighters, missile carriers and their missiles | **ported** (`units::oltanis_carrier`) |
| 923 | the mine drones (Oltanis's copy of 1401) | **ported** (`units::kalebo_mine_drone`) |
| 1224 | the lightning cuboids | **ported** (`units::oltanis_bolt`) |
| 1331 | the arc slots' keeper (shared with Quartu) | **ported** (`units::oltanis_arcs`) |
| 1417 | the flying cars carrying Swingshot targets | **ported** (`units::oltanis_car`) |

## Shared-system changes in this pass

- The swing camera can be told a follow yaw by a class (`cinematic::swing_follow`, the flying cars).
- The camera's class is kept on the world (`World::camera_class`).
- Particle helpers: `fx::part19`, `fx::part69`, `fx::part23_rec`, `fx::rec_mut`, `fx::rand_vec_ab`.
- The pop-up turrets got their group calls (`popup_turret::{raise_group, any_raised, lower_group}`) for the sentries.

## Notes

- [L] The rail bots' state 6 enters the rail camera (`0x2d6570`), which is not ported.
- [L] The cart's volumes use 0x400 as read; the sentries' record +0xe has no other reader found.
- Headless screenshots in this session came out black on every level (Gemlik too): a capture problem here, not a
  regression of this pass.

## Open

- The user's QA: docs/test-requests/2026-10-04-oltanis.md.
