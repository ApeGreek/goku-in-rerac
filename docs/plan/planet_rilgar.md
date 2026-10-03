# Rilgar (level 05): the batch C checklist

Batch C's eighth planet, started 2026-10-03 after Kalebo III. Before this pass Rilgar ran ported code for its story
NPCs (918 / 919 / 925), the hatches, the rocking floats 810, the flaps 895 and the shared water, hoverboard and race
systems. The census of 2026-10-03 left these placed classes without a port; this page tracks them.

## Classes placed without a port (census 2026-10-03)

| class | placed | what | status |
|---|---|---|---|
| 1099, 984, 1550 | — | the hidden marker; the bobbing pickups' display (×2.3, ±1 at 50°/s); the infobot's thrusters in scenes 10–11 | **ported** (`units::rilgar_small`) |
| 841 | 1 | the sewer fog switch: the level fog in and out of the sewer, the underwater tint, the water level | **ported** (`units::rilgar_fog_switch`, the fog channel below) |
| 920, 447 | — | the watching bystanders: the head look-at, the scene big head | **ported** (`units::rilgar_watcher`) |
| 846 | — | breakable posts: a burst of loose pieces and sparkles | **ported** (`units::rilgar_breakable`) |
| 877 | — | lift pads: called, or a cutaway ride (fade, camera, the spline wall as a ring) | **ported** (`units::rilgar_lift_pad`) |
| 79 | — | the trail riders: banked spline loops with ribbon trails and joint glows | **ported** (`units::rilgar_trail_rider`) |
| 35 | — | the sea beasts: swimmers, and the lurker that takes Ratchet | **ported** (`units::rilgar_sea_beast`) |
| 998 | — | the cars: five stations, a route choice by mission at station 2 | **ported** (`units::hover_car`, shared with Kalebo's 1410) |
| 844 | 3 | the rising rocks: a rumble, the rise (hurting what stands on it), the float, the fall | **ported** (`units::rilgar_rising_rock`) |
| 625 | 4 | the flame tanks: path driving, the turret's flames and their burns, the treads, the blasts | **ported** (`units::rilgar_flame_tank`) |
| 623 | 56 | the biters: arena graph, weaving chase, wind-up sparks and bite, knockback, death | **ported** (`units::rilgar_biter`) |

## Shared-system changes in this pass

- **The fog channel**: `WaterWorld::fog` mirrors the level fog globals (0x15f444..) and `store_fog(tick, g)` asks the
  engine to apply new ones once (`rc-engine` `fog_state`: the store applied by tick, the level's state mirrored back
  each frame; the mirror set at load). First consumer: 841.
- **`units::hover_car`**: Kalebo's car generalised by a `Layout` (stations, routes, prompts, offsets); 998 and 1410
  are its two layouts.
- **`fx::part02_rec`**: a type-2 particle keeping its record (625 hit-tests its kept flames).
- **`region::push_out_dist`**: the wall push-out with the nearest wall's distance (level05 `0x304058`).
- The story NPCs 918 / 919 / 925 now register their joint lists (their look-at manipulators had none).

## Notes

- 625 deletes itself when its damage record's +0xe is 2; nothing on level 05 sets it [L].
- 625's zero-damage hits kill it, and its wrench hits are ignored (attacker class 0 / 0x47): the game's.
- 623 in state 0xc (placed with +0x2b4) waits 20 below for other code; none on level 05 wakes it [L].
- 623's look-at aims only at Ratchet's moby; a hunted group member (+0x2c8) gets the bite without the look.

## Open

- The user's QA: docs/test-requests/2026-10-03-rilgar.md.
