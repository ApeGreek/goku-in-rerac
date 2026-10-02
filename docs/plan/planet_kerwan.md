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
| 455 | 2 | the creature spawner (children 545; also levels 08 / 14) | open |
| 578 | 5 | a creature | open |
| 631 | 1 | a creature (the largest unit left) | open |

## Shared-system changes in this pass

- `interact::set_talked` writes nothing for a store of the same value (816 / 1012 store it every tick).
- `Interact::class_shown`: a class's own request for the prompt element (1012 raises it at once).

## Open

- 455, 578, 631.
- "manipulator: target joint list not loaded" ×4 on the smoke run (another class's manipulator target).
- The user's QA: docs/test-requests/2026-10-02-kerwan.md.
