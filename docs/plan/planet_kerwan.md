# Kerwan (level 03, Metropolis): the batch C checklist

Batch C's third planet, started 2026-10-02. Most of Kerwan already ran ported code (the air traffic, the train, Helga
and the Heli-Pack giver, the troopers and hounds, the movers, the help director). The census of 2026-10-02
(`cargo run -p rc-trace -- class-census`) left nine placed classes without a port; this page tracks them.

## Classes placed (level03 class table; census 2026-10-02)

| class | placed | what | status |
|---|---|---|---|
| 11 | 1 | Gadgetron vendor | ported (`vendor`) |
| 13, 500 / 501 / 505 / 511, 605, 1134, 1827, 817 | | bolts, crates, buried bolts, gold bolts, breakables | ported |
| 75, 115–120, 132, 795 | 246 | the air traffic | ported (`units::air_traffic`) |
| 573 / 574 | 87 / 19 | the hounds and the gun troopers | ported (`units::kerwan_hound`, `units::kerwan_trooper`) |
| 604 / 1818 | 1 / 1 | the Summoner's mouse house and mouse | ported |
| 726, 737, 750, 758 / 803, 805, 832, 994 | | path lift, camera triggers, the infobot, swing targets, checkpoints, the RC range, the sea | ported |
| 822 / 845 / 1210 | | the train | ported (`units::kerwan_train`) |
| 868 / 905 / 928, 899 | | the movers, the path-mover lines | ported |
| 890 / 909 | 1 / 1 | Helga (Swingshot), the Heli-Pack giver | ported (`units::kerwan_story`) |
| 915–917, 1342 | | markers, the help director | ported |
| 825 | 1 | the turntable | **ported 2026-10-02** (`units::kerwan_turntable`) |
| 997 | 1 | the riser | **ported 2026-10-02** (`units::kerwan_riser`) |
| 1548 | 1 | the cutscene FX driver | **ported 2026-10-02** (`units::kerwan_scene_fx`) |
| 914 | 1 | the talking bystander (skill point 5) | **ported 2026-10-02** (`units::kerwan_bystander`) |
| 816 / 1012 | 3 / 2 | the called platforms and the two-way shuttles | **ported 2026-10-02** (`units::kerwan_transport`) |
| 455 / 545 | 2 / 0 | the pod spawners (also levels 08 / 14) and their pods: they delete their placed 573s at init and hatch them back from lobbed pods once Ratchet is near | **ported 2026-10-02** (`units::pod_spawner`; bounce and revival shared with `units::pod_launcher`) |
| 578 / 627 | 5 / 0 | the Blarg mine layers and the mines they drop | **ported 2026-10-02** (`units::kerwan_layer`) |
| 631 / 848 | 1 / 0 | the Blarg hover ship (a 574 gunner riding it) and its flame stream 848 | **ported 2026-10-02** (`units::hover_ship`; the flame also serves Quartu's 233) |

## Shared-system changes in this pass

- `interact::set_talked` writes nothing for a store of the same value (816 / 1012 store it every tick).
- `Interact::class_shown`: a class's own request for the prompt element (1012 raises it at once).

## Decode notes for the open units (2026-10-02)

**631** (level03 `0x2cca00`, 1 placed at (260, 195, 53.5); the hit handler `0x2ce238`; jump table 0x1e3320):
- Pvars: +0x20 damage (health 3.0, +0x24 3, +0x28 2, +0x2e 1 alive), +0x38 alert → +0x254 `ticks(240)` (search 80, else 40),
  +0x58 / +0x59 / +0x5a = 0x19 / 1 / 6, +0x60 flash, +0x120 / +0x150 the cursors of patrol paths A / B (+0x130 / +0x160 the
  splines of +0x134 / +0x164), +0x180 / +0x1c0 manipulators (joint lists 0 / 1: turret yaw / pitch, `set_axis` axis 2),
  +0x200 yaw vel, +0x204 s16 fire timer, +0x206 s16 pause, +0x208 range cuboid, +0x20c / +0x244 exit cuboids (A / B),
  +0x210 bob phase, +0x214 speed, +0x218 / +0x21c roll / pitch vel, +0x220 / +0x228 turret yaw / pitch (+0x224 / +0x22c
  vel), +0x230 / +0x234 exit flight paths, +0x238 the rider 574 (moby index), +0x23c the path (0 A, 1 B), +0x240 the
  live shot 848, +0x248 arrival trigger cuboid, +0x24c arrival path, +0x250 voice.
- Prologue: this visit's death bit (level03 0x1ba5d0) or path A −1 or (state ≠ 0 and +0x2e ≠ 1) → delete rider and
  self; the rider (574, alive) `big_head_scale(2.5, rider, +0x150)`, `look(rider, rider, +0x150, list 2, 0.03, 0.3)`.
- 0 init (mission done or no arrival path / cuboid → 2; else arrival lengths, → 1 at its point 0); 1 arrival (once
  Ratchet is in +0x248: `SplineSample(996, 5)`, along + 2, speed ±10·dt² (≥ 0.1·dt, ≤ 20·dt), lerp target, yaw
  SpringTurn(0.01, 0.3, 0.1), the bank); 2 patrol (face the target; the turret aims at the live shot or the target;
  point advance; speed ±4·dt² ≤ 10·dt; exit cuboid → 4 then 3; in the range cuboid fire `0x2d5208(t.z, (t − joint 0)·6·dt
  with z 0, joint 0, m, ticks(240))` every 360 ticks); 3 exit flight (no path: rise to z 200 at 8·dt; else the arrival
  code, at the end switch A ↔ B → 4 then 2); 4 pause (`ticks(15)`, back off, then +0xbc); 5 destroyed (rider +0x262 = 2,
  update 0xff, mode &= ~6, a hit of 20 flags 0x830000 type 3/3 pushing it off; `SpawnBeamExplosion(0, 0, 4, 2, 100000,
  3, 15, …, 20, 3, 4, 2, 1, 1)`, `SetDeathBits`, `BreakFxB` 1591..1593, detach, delete).
- The bank (`0x2cdd0c`): pos += move; a = atan(move) − yaw; pitch target v·20°·cos a / (dt·K), roll −v·20°·sin a /
  (dt·K) (K 20, 10 in 2 / 4); SpringTurn(30°/60°·dt², 45°·dt) / (60°/120°·dt², 90°·dt).
- Tail: shadow within 40; bob z ±0.17·cos(phase), phase += 2π / (2·60·scale); the rider at the ship's position and
  Euler, `MobyAnimAdvance`, `MobyBuildMatrix`, rider mode |= 6.
- Hits: mask 0x330000, column 4; reaction 1 or health ≤ 0 → 5; else ≠ 0xb: red 0xb4, flash; an exact push (w 5627.925)
  outside 4: tilt from the hit point, speed 4·dt, pause `ticks(60)`, state 4 (+0xbc the old), class sound 1.
**848** (level03 `0x2d47c0`, from `0x2d5208`: pvars +0 velocity (w = target z), +0x10 owner (631 or 233), +0x14 life,
+0x18 side): state 1 moves (vz toward the target height, ±8·dt), turns toward the target within 90°, keeps the owner's
voice (+0x250 / 233's +0x1b8), and every `ticks(6)` alternates sides and sprays type-27 flames from the owner's joint list
3 − side plus a `0x260130` stream and line / sphere hits on Ratchet (or a 479) → 2; life out or owner dead → 2;
state 2 releases the voice and deletes.

## Open

- The user's QA: docs/test-requests/2026-10-02-kerwan.md (K1–K9).

Closed 2026-10-02: 455 / 545 (`units::pod_spawner`), 631 / 848 (`units::hover_ship`), and the "manipulator: target joint
list not loaded" warnings: Helga 890 and the Heli-Pack giver 909 turn their heads with manipulators on their own joint
lists 0 / 1, which their unit rows did not ask the loader for (`joints` now names their classes). The smoke run's
unported log is empty.
