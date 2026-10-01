---
status: open
job: blarg-shuttle
date: 2026-10-01
commit: d4579f7
areas: [classes, camera, story]
---

# Blarg's shuttle 1109: the routes, the bursts, the last ride's blast, planet 5

## 1. Summary

Planet 5 (Rilgar) was blocked: it unlocks only at the end of the shuttle's last ride on the Blarg station (level 6).
Ported the shuttle as per-class code on the shared pieces (the context prompt, the script camera, `CarryRiders`, the
story layer, the infobot, the bobbing blocks).

| class (level, instances) | update (reference) | module |
|---|---|---|
| 1109 Blarg's shuttle (6, 1) | level06 0x302578 (+ 0x302fd8, 0x303160, 0x303748, 0x303b68, 0x304028) | `units/blarg_shuttle.rs` |

New shared pieces:
- The script camera's spring settings (`0x312b40`): `cinematic::script_springs` / `CinematicCall::ScriptSprings`,
  `ScriptCamera::springs` (cleared by every `camera_script`).
- `infobot::show` (state 2, shown, glow), `bob_block::drift` (drift and spin out of a group).

## 2. Where it lives

Coverage tables in the module doc. Docs: gaps.md G-CLS-032 (1109 struck), G-UI-018 (the gadgetbot help box on the
station, from the unported gadgetbots 857).

## 3. Behaviours to verify

dt = 1/60. Blarg = level 6.

### B1. The prompt and the first ride
- **Claim:** standing on the shuttle on the ground the context prompt shows the route by the stop (0x1782 / 0x1785 /
  0x1786 / 0x1784); on the very first ride (record 0x53 unset) with the help box free the help line 0x177a plays
  instead and no route is offered.
- **Edge:** the help box only enables after the first stick input and 120 ticks; another help line open blocks △.
- **Method:** QA in game.

### B2. A ride
- **Claim:** △ fades to black over 10 ticks, Ratchet is seated (cuboid +0xac), the script camera starts at the camera
  path's start looking along the look path, the shuttle flies its path over 1200 ticks with the camera on its own
  path, scaling between the route's start and end scale (0.6 / 2.2), turning round past 0.8; marked path points set off
  bursts (level sound 0) and on the last route the blast (level sound 1). At the stop Ratchet is placed behind it facing
  out, the camera returns over 30 ticks. Riders stay on it while it moves.
- **Method:** QA in game; unit test of `flight`'s point-w firing (each point fires once).

### B3. The last ride
- **Claim:** entering cuboid +0xf0 makes the next ride the last one: the blast flashes at (426, 189.75, 173.07), the
  block group bursts and drifts away spinning, the shuttle lands by the infobot; walking within 3.5 of it plays
  scene 12, removes it, unlocks planet 5, marks the mission done, sets the checkpoint, plays movie 5, then the planet
  banner and the save (or scene 5 first if the Hoverboard is already owned).
- **Edge:** coming back with the mission done the block group is gone from the start.
- **Method:** QA in game.

## 4. Known not done (gaps)

- The gadgetbot help box shown on the station (the gadgetbots 857, G-UI-018, batch B).
- The forced-route / flash unit words (gp−0x4b00 / −0x4afc): nothing on level 6 writes them.
