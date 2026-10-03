# Batalia (level 08): the batch C checklist

Batch C's eleventh planet, done 2026-10-03 after Umbris. Before this pass Batalia ran ported code for its story
NPCs (the commando, the deserter, the turret's host), the fighters 438, the runners, the lifts, the kill cuboids,
the liquid 327 and the shared crates, bolts and checkpoints. The census of 2026-10-03 left these classes without a
port; this page tracks them.

## Classes without a port (census 2026-10-03)

| class | what | status |
|---|---|---|
| 1400 | the weather: rain round the camera with the wind and the splashes (also Hoven's and Oltanis' snow) | **ported** (`units::weather`) |
| 253, 425 | the tanks and their shells: through their walls or behind a crank's gate, the patrol, the turret, the wreck | **ported** (`units::batalia_tank`) |
| 333, 248 | the grenadiers and their grenades | **ported** (`units::batalia_grenadier`) |
| 424 | the ferries between the three docks | **ported** (`units::batalia_ferry`) |
| 468, 469, 1553, 1629, 1641 | the crank bridges, the scene thrusters, the streaks, the steam | **ported** (`units::batalia_small`) |
| 440 | the anti-aircraft turret: the seat, the aim, the shells, the waves, the radar HUD, the end with the host | **ported** (`units::batalia_turret`) |
| 444, 462, 463 | the gunships: the intro loops, the attack runs, the missiles, the pods knocked off, the crash | **ported** (`units::batalia_gunship`) |
| 441–443, 445, 448–451, 464, 465 | the gunships' parts: aligned joint to joint, then falling, smoking and splashing | **ported** (`units::batalia_gunship::part_update`) |
| 435 | the bombers that crash through the walls, and the gunships' missiles | **ported** (`units::batalia_bomber`) |
| 671 | the wreck flames: the swaying fire sheet, embers, sparks, the wandering light | **ported** (`units::batalia_flame`) |
| 600, 1649 | the pool overlays (two more copies of Pokitaru's pool module) | **ported** (`water::sea::pool_ref::C`, `D`) |

## Shared-system changes in this pass

- **`water::sea::pool_ref::Module`** now carries what differs between the module's copies: the shimmer passes'
  order and FIX words, the L0 pass (`0x44` or `FIX | 0x64`), the fade span, the camera heights, the colour
  rescales and the FX textures. Sea ports 19 and 20.
- **Particle type 0** moves only the ribbon ends' xyz (the w lanes are the width).
- **`loose_piece::fling`**: a class sending a loose piece flying (the bombers' walls and debris).

## Notes

- The gunships' parts are separate mobys placed joint onto joint every tick, as the game does; the alignment reads
  the parts' joint lists (the rows' `joints`).
- The intro path's first segment starts at the path's header in the game (point −1); its weight is 0 on the first
  tick, so the port uses the last point [L: the same yaw].
- The flame's VSync count is the draw tick; the gunship splash's first velocity starts from zero where the game
  reads stale stack [L].

## Open

- The user's QA: docs/test-requests/2026-10-03-batalia.md.
