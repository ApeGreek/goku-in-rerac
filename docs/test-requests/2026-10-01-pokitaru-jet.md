---
status: open
job: pokitaru-jet
date: 2026-10-01
commit: 49c5513
areas: [classes, world, hud, render]
---

# Pokitaru's jet mission: the jet 1242 and its HUD, the convoys 1264 / 1265 / 1524, the fighters 1319, their shots and drops

## 1. Summary

Pokitaru's (level 11) convoy mission had no code: the jet, its targets and its enemies were all unported. Ported the
whole set as per-class code on shared pieces. The jet is the level's own copy of Gemlik's ship flight; the code that
is literally the same (springs, exhaust, screen point, HUD marker / strip / sprite) is shared.

| class (level, instances) | update (reference) | module | shared with |
|---|---|---|---|
| 1242 jet (11, 1) + HUD callback 0x311d50 | level11 0x313290 | `units/pokitaru_jet.rs`, `units/pokitaru_jet_hud.rs` | Gemlik's ship's helpers |
| 1264 convoy (11, 4), 1265 car (created), 1524 sludge (created) | level11 0x3172c0 / 0x3181f0 / 0x31ab08 | `units/pokitaru_convoy.rs` | — |
| 1319 fighter (11, 52) + contrails 0x3192c8 | level11 0x319838 | `units/pokitaru_fighter.rs` | — |
| 1017 fighter laser (created) | level11 0x309098 | `units/fighter_shot.rs` | levels 13, 17 |
| 1034 missile (created) | level11 0x3094f8 | `units/ship_missile.rs` (`spawn_early`) | levels 13, 17 |
| 1218 / 1220 pickups, 1219 parachute (created) | level11 0x30f728 / 0x310028 | `units/ship_pickup.rs` | — |

Shared changes: `ship_laser::hit_sparks` (the laser hit's five sparks, also the fighter shot's); `ScreenText::font`
(the Regular font for `font_print_center_large`); `water::sea::ocean_z` (the sea height the sludge splashes on);
Gemlik's `spring` / `spring_angle` / `exhaust` and HUD `marker` (now with its FX) / `strip` / `sprite` / `PIP` opened
to the jet.

## 2. Where it lives

Coverage tables in each module doc. Docs: gaps.md G-CLS-032 (1242 struck), G-LVL-009 (1319 struck), new G-REN-035
(the HUD's additive 0x48 prims drawn as 0x44).

## 3. Behaviours to verify

dt = 1/60. Pokitaru = level 11. The jet sits at (625.5, 579.6, 234.0); `RC_HERO_AT=623.5,577.5,234.5` lands next to it;
`RC_GIVE_ITEMS=7` gives the Pilot's Helmet.

### B1. The mount
- **Claim:** touching the jet without the Pilot's Helmet plays the helmet help line (again after 18 s); with it the
  "Fly" prompt shows and △ fades to black, then the jet lifts 30 up and flies: Ratchet hidden, music track 2
  (stinger 6), the record at 255 health and 10 of 20 missiles, the flying help line the first time.
- **Method:** QA in game (smoke-run: mounted at tick ~330).

### B2. Flight
- **Claim:** speed creeps up for 2 s, then 24·dt cruise, ✕ boosts to 30·dt and widens the view (65° → 85°); the
  stick turns (±80° pitch); Left on the d-pad toggles the cockpit view; near the play area's wall (its cylinder) or
  below 260 / near the top the stick is bent back inside; touching the level wrecks the jet (death after 1.5 s);
  other mobys scrape it (−5 every 10 ticks).
- **Method:** QA (feel); smoke-run: level flight at z 264 with a neutral stick.

### B3. Weapons and HUD
- **Claim:** □ / L1 lasers every 4 ticks from alternating wings; ○ / R1 a missile every 30 ticks at the lock (the cars
  under the crosshair, locked after 500 − 600 of the timer); the HUD: the gauge (FX 0x3c cut above the eased health)
  with its frame, radar blips for each convoy and car, two columns of missile pips, the crosshair, the lock marker
  (green growing, then red / green blinking); help lines for low health, few missiles and the fighters.
- **Method:** visual QA against PCSX2; smoke-run screenshot shows pips, crosshair, radar with blips.

### B4. Convoys
- **Claim:** four convoys of eight cars each fly their paths, switching to the battle path once Ratchet flies; each
  eighth of the 80 health lost, the hit car's place is taken by the tail, which breaks into pieces; the last car
  gone, the convoy bursts; cars passing marked points drop sludge that splashes on the sea and sinks; with no convoys
  left the jet flies 3 s more, "Mission complete" shows, the screen fades and Ratchet lands at the exit, mission done.
- **Method:** QA in game (the whole mission).

### B5. Fighters and drops
- **Claim:** the escorts fly the convoy paths with contrails; downed ones burst, drop a missile or health pickup on a
  parachute, and come back once off screen; while convoys are on screen the jet calls in ambushers (every 4 s, up to
  5 − convoys left) that close in and fire at the jet leading its motion; the jet collects a pickup by touching it or
  passing within 4 (+5 missiles / +¼ health); shooting the parachute drops it; three fighters downed by the Visibomb
  give skill point 18.
- **Method:** QA in game.

## 4. Known not done (gaps)

- The HUD's additive blend (G-REN-035), the parked jet's blob shadow and the pickups' (G-REN-025), the view tangent
  outside the main projection (G-REN-034), the hit record bump at 0x157150 (no reader found), the counter 0x15ee00
  kept as a unit word.
