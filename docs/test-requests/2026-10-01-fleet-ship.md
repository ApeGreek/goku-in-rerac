---
status: open
job: fleet-ship
date: 2026-10-01
commit: TBD
areas: [classes, world, hud]
---

# The fleet's ship mission: the ship 1379 and its HUD, the turrets 347 and their bolt, the fighters 1843, the floating pickups 224 / 228

## 1. Summary

Drek's fleet (level 17) had no ship mission: the ship, its targets and its enemies were unported. The ship is the
level's copy of Gemlik's ship (the same flight, steering pull, camera, guns and missiles) with Pokitaru's edge bend; it
shares the code that is the same. The fighters 1843 are Pokitaru's 1319 (one module, two classes). The floating pickups
224 / 228 also fill Gemlik's battle (eight of each placed there, unported until now).

| class (level, instances) | update (reference) | module | shared with |
|---|---|---|---|
| 1379 ship (17, 1) + HUD 0x2eb9a8 | level17 0x2ed018 | `units/fleet_ship.rs`, `units/fleet_ship_hud.rs` | Gemlik's helpers |
| 347 turret (17, 8), 1368 bolt (created) | level17 0x2cb310 / 0x2e8e08 | `units/fleet_turret.rs` | — |
| 1843 fighter (17, 60) | level17 0x2f40d8 | `units/ship_fighter.rs` (renamed from `pokitaru_fighter.rs`) | 1319 (11) |
| 224 / 228 pickups (13: 8 + 8; 17: created) | level13 0x2e1cc8 | `units/ship_pickup_float.rs` | levels 13, 17 |

Shared changes: `World::part80` (the type-80 glow spawner of level 17); Gemlik's HUD `blip_from` (the blip from any
reference point, mirrored under the mirror cheat on the fleet), `ring`, `ARROW`, `H` opened.

## 2. Where it lives

Coverage tables in each module doc. Docs: gaps.md G-CLS-032 (1379 struck), G-LVL-009 (1843 struck).

## 3. Behaviours to verify

Level 17. The ship sits at (725.4, 446.3, 169.7); `RC_HERO_AT=723.4,444.3,170.5` lands next to it.

### B1. Mount and flight
- **Claim:** within 2 the "Enter" prompt; △ fades to black, Ratchet is placed in the ship at its mount cuboid and flies:
  Gemlik's flight (cruise 24·dt, ✕ boost to 30·dt with its sound, the FOV widening), the steering pull toward the
  steering cuboid, the stick bent back inside the play area's cuboid (walls, top and bottom); flying into the level
  or a turret wrecks the ship (reset to the start after ½ s).
- **Method:** QA in game; smoke-run: mounted, flew, 8 targets counted, the crash into the hull wrecks it.

### B2. HUD
- **Claim:** the gauge, missile pips, crosshair, lock marker (FX 0x2e), radar blips; each turret ringed red when on
  screen; when none is on screen a pulsing green arrow toward the nearest; the turrets' blips sit at the radar's centre
  (the game takes them from the turret's own raised point); "Mission complete" while landing.
- **Method:** visual QA against PCSX2; smoke-run screenshot shows pips, crosshair, radar, arrow.

### B3. Turrets
- **Claim:** within 254 of the flying ship a turret turns and raises its barrel and fires a bolt every 45 ticks when
  it has the ship in its line of fire; with Ratchet on a magnetic floor it fires at random points of its target
  cuboid on its own phase of 90 ticks; it launches a fighter of its group now and then; only the ship's lasers and
  missiles hurt it (15 health, a red flash); destroyed, it breaks in two and bursts; all eight destroyed → the ship
  lands after 3 s, mission done, checkpoint.
- **Method:** QA in game.

### B4. Fighters and pickups
- **Claim:** the fleet's fighters behave as Pokitaru's (escorts on paths, ambush runs, shots, contrails) but drop the
  floating 224 / 228 pickups; a pickup spins and bobs, is drawn to the ship within 40 and collected within 4 (+5
  missiles / +10 health), shrinks away. On Gemlik the eight + eight placed pickups now work the same.
- **Method:** QA in game (both levels).

## 4. Known not done (gaps)

- The parked ship's blob shadow (G-REN-025); the HUD's additive blend (G-REN-035); 1772 (the fleet's scenery effect),
  1378 / 1380 / 1382 not part of this job.
- A created 224 / 228 waits for moby 0's state 0 (zero pvars), as the game would; the muzzle glint's velocity is an
  unset stack vector in the game (0 here).
