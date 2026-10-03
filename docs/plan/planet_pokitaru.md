# Pokitaru (level 11): the batch C checklist

Batch C's sixth planet, started 2026-10-03 (chosen ahead of play order as the smallest remaining one). Before this pass
Pokitaru already ran ported code for its story characters, the commando, the boats, the biters, the ball throwers, the
gates, the teleporters, the cutaway machine 1157 and the jet mission. The census of 2026-10-03
(`cargo run -p rc-trace -- class-census`) left eight placed classes without a port; this page tracks them.

## Classes placed without a port (census 2026-10-03)

| class | placed | what | status |
|---|---|---|---|
| 361 | 1 | the spline wall: names spline 57 (a ring round the island at z 223) for the hero's capsule pass every tick | **ported** (`units::pokitaru_wall`; the hero side below) |
| 1350 | 1 | skill point 0x13d41b once every moby of its group is gone | **ported** (`units::pokitaru_skill`) |
| 1179 | 2 | the Thruster-Pack floor buttons: stomped down for good, saved on Pokitaru, help hints near them (also on 15) | **ported** (`units::pokitaru_button`) |
| 1180 | 1 | the piece button 488 turns 60° (flag 91 when turned in play) | **ported** (`units::pokitaru_turner`) |
| 1178 | 4 | the tilting platforms: tip under Ratchet, spring back, throw him off past 40° | **ported** (`units::pokitaru_tilt`) |
| 1156 | 1 | the unfolding machine: 18 pieces fold out in a cutaway when button 489 is pressed (flag 86, skippable after) | **ported** (`units::pokitaru_unfold`) |
| 1903, 1919 | 1 + 1 | the pool overlays: three layers of scrolling strips per pool, a shimmer that fades in near the camera; 1919 gated by three camera cuboids | **ported** (`water::sea` ports 12 / 13, `pool_ref`) |

## Shared-system changes in this pass

- **The spline wall** (`Hero::wall_spline` 0x14162a, `hero::physics::spline_wall` = `FUN_00276cb8`): a class names a
  spline during the moby loop (`HeroFields::wall_spline`); the hero's capsule pass (`0x233940`, `FUN_00249e58`) pushes
  Ratchet the capsule radius off its segments across the gravity direction, sets 0x13f546 = 4 on a push and clears the
  name. The tick hands the hero that spline's points (`MobySystem::spline`, `Carriers::wall`).
- **Subtractive effect blend**: `FxPrimParams::subtract` / `display_blend::specialize_subtract` (`Cd − Cs·As` on the
  display bytes, the GS's ALPHA 0x62 with As = FIX); `sea_render`'s groups carry a `Blend` (mix, add, subtract).
- **The pool overlay module** as a `SeaKind::Pool` row (two copies on level 11, read through `Relocation`), reusing the
  Hoven strips' parse and colour rescale.

## Notes

- 1179's save check reads the level's collected byte and death bit only on planet 0xb; level 15's copy is never saved.
- 1156's camera script starts at cuboid +0x04's centre with cuboid +0x08's Euler, and the glide's first tick uses
  cuboid +0x04's Euler (the game's).
- The pool overlays' shimmer stops scrolling and keeps its alpha while the camera is beyond the far distance (the game's).
- The talking characters 114, 298 and 90 report "manipulator: target joint list not loaded" once each at load (their
  head-turn lists 0 / 1); this predates the pass.

## Open

- The user's QA: docs/test-requests/2026-10-03-pokitaru.md.
