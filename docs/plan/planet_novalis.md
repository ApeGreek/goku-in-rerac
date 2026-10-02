# Novalis (level 01, Tobruk Crater): the batch C checklist

Batch C's second planet. Novalis was the port's reference level, so every placed class already runs ported code
(census 2026-10-01: the empty port column only lists class 27, the particle emitter in `particles::type06`, and the
water strips 676 / 678 / 761 / 1225 the water renderer draws). This pass (2026-10-02) closed what those ports still
skipped.

## Done in this pass

| what | where |
|---|---|
| The visit-state records (`0x29b0a0`, the checkpoint copy 0x1baaa0 → 0x1bb700, the restore after a death reload): the bolt cranks' progress, the bridge halves' lowered command, the Water Pump Worker's sold state (also Kerwan's Helga) | `moby_update::visit` (G-SAV-003) |
| The cranks with uids 0x34 / 0x35 set global flags 0x0c / 0x0d (0x13d394 / 0x13d395) | `classes::bolt_crank` |
| The Water Pump Worker's save after selling the Infobot and the gold-weapon vendor's save: both were a no-op write | `cinematic::save` (the `GameWrite::Save` variant removed) |
| The burning wreck 1510 of a shot-down flyer 660 / gunship 688 | `classes::burning_wreck` |
| The Summoner mouse 1818's glow sprites (`0x30de68`) and jets (type 74, `0x30dda0`; `creature::fx::part74` now shared with Gemlik's ship) | `classes::mouse` |
| The blob shadows (`0x26eec8`, list 0x16e500, draw `fun_001f4880`): the mouse, the Drone Device's drones, the Mine Glove's mines, the Visibomb, and on other levels the pods 1886, ship pickups, Gemlik's ship, the fleet ship, the ring shell | `shadows::blob` (G-REN-025) |
| The Drone Device's trails: real type-55 ribbons instead of a count | `classes::drone` |
| Dev switch `RC_UNPORTED=1`: the moby loop's unported calls every 600 ticks | docs/workflows/dev-switches.md |

## Checked, nothing to port

- The flyers' group sync: every Novalis flyer has group −1 (the sync is for the other flyer levels, G-CLS-015).
- The Summoner helmet's state writes from the mouse (1, 2): class 0x1b1's update is `jr ra` and nothing else reads them.
- The critter path clamp: no Novalis critter has a path.

## Open

- A play-through after the pass (the user's QA: docs/test-requests/2026-10-02-novalis.md).
- The checkpoint record's other copies (the hero's light word / ambient / control mode / music on the death reload,
  G-SAV-003).
- PCSX2-only items for Novalis stay in gaps.md (G-TOOL-005 / 006 / 007 / 012).
