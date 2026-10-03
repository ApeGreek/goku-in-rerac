# Gaspar (level 09): the batch C checklist

Batch C's twelfth planet, done 2026-10-03 after Batalia. Before this pass Gaspar ran ported code for its help
director 1000, the story NPC 1290, the hazards 1206, the chain anchors 1172, the riding floats 664 / 1293 / 1320, the
lava liquid 317 and the shared crates, bolts and checkpoints. The census of 2026-10-03 left these classes without a
port; this page tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 263–267, 417 | the meteor shower: the drifting rocks along their path, the falling meteors and their bursts, debris, the dust puffs | **ported** (`units::gaspar_meteors`) |
| 276, 1298, 1299, 1300, 320, 321 | the staged breakable rocks: each hit the next stage, the spray, the chunks | **ported** (`units::gaspar_breakable`) |
| 1285–1288 | the ring rocks and their story flag | **ported** (`units::gaspar_breakable::ring_update`) |
| 1150, 1151 | the path platforms (Gaspar's copy of 726, with the riding state and sounds) | **ported** (`classes::path_platform::gaspar_update`) |
| 1201, 324, 1258 | the cannons: the mount, the aim, the two barrels, the seat camera and light, the crosshair; their bases and shells | **ported** (`units::gaspar_cannon`) |
| 1766 | the lava raft: waiting, rocking, sailing its path with its paddle wheel and wake | **ported** (`units::gaspar_raft`) |

## Shared-system changes in this pass

- **`HeroFields::marker`**: a class's screen marker (`FUN_0020fb60`) into the guns' marker list (the cannon's
  crosshair).
- **`path_platform`**: one update for both copies (`run(…, gaspar)`).

## Notes

- The meteor splash puffs' centre is the last probe point (the game's stack slot) [L]; the shower is drawn relative to
  the camera, as the game does.
- The cannon's base 324 and the last rock stage 1300 have empty updates in the game.

## Open

- The user's QA: docs/test-requests/2026-10-03-gaspar.md.
