# Breakables, glass and the bomb in water (2026-09-28)

Addresses are level01 unless a level is named. Code: `crates/rc-game/src/moby_update/classes/breakables.rs` (the
general system), `breakables/novalis.rs` (Novalis's own props), `bomb_water.rs`; `crates/rc-engine/src/fx_draw.rs`
(the ship glass); tests `crates/rc-game/tests/breakables_levels.rs`, `bomb_water::tests`.

## 1. Survey method

Every level's class table (`lvl.vtbl`) was walked and each update function's calls checked (decomp exports plus
`tools/ghidra/names/clusters.tsv` to name the shared helpers on every level) for the break helpers `BreakFxA`
0x2787a0, `BreakFxB` 0x278ad8, `BreakFxC` 0x278e20, the level02 burst `0x265408` (absent from level 01; cluster
`6e363a2f`), `SetDeathBits` 0x26c250 and `BoltBurst` 0x275988; then every update with the wrench-hit / state-2 /
`DeleteMoby` shape was listed. Class names from Wrench's table were used for orientation only.

## 2. The shared break template

22 classes on 13 levels run one template, compiled once per class with its constants (so each copy is its own
function): state 0 → 1; state 1 + a hit record with flags 0x10000 and damage > 0 → 2; state 2: `PlayClassSound(0, 0)`;
on levels 16 / 17 `SetDeathBits(m, 0, −1)` (bolts from the placement and the death bits: kept broken); `BreakFxA`
(`BoltBurst(m, 4, 7, Ratchet within 7 (xy) ? 2 : 0, −1)`: 4–7 bolts near Ratchet, none farther); the pieces; the
remains; `DeleteMoby`.

* Pieces: `BreakFxB(0, m, class, m.pos, m.rot, 0, flag, 0, 0)` (a `FxGroupUpdate` piece: random throw, spin, bounce,
  then an explosion), the Novalis pot's ring of four, or the burst `0x265408(m, a, na, b, nb, n, flag)` (read from its
  disassembly; registers a0..t2): pieces `a .. a + na` at the prop, then up to `n` pieces of class `b + randi(nb)` at
  random points of the prop's **collision volume** (`0x26fba0(1000.0, m, n, out, 0xffff)`: primitives with mask bit 1
  weighted by volume, sphere / cylinder / capsule sampling, `rand_angle`×3 rotations), all with piece flag 2 when
  `flag ≠ 0`. Flag-2 pieces end in `0x2742a8` (three type-11 spark pairs of 500000·size and two flashes, no shake or
  sound; `fx::piece_explosion`).
* Remains: `BreakFxC(m, class)` (scaled like the prop, update distance 0, mode 0) or, on levels 6 / 7, the same inline
  without the scale. Their updates are empty.

| level | fn | class(es) | kept | pieces | remains | placed |
|---|---|---|---|---|---|---|
| 01 | 0x2fd9a0 | 754 | | ring of four 1817 | 1816 (C) | 12 |
| 01 | 0x30d0f0 | 1813 | | 1815 | 1816 (C) | 4 |
| 02 | 0x2f1f60 | 1819 | | burst 1821, n 5 | 1820 (C) | 18 |
| 03 | 0x2d3cd8 | 817 | | 1831, 1832, 1833 | – | 1 |
| 03 | 0x2e2948 | 1827 | | burst 1829, n 1 | 1828 (C) | 39 |
| 04 | 0x2e79a8 | 1834 | | burst 1836, n 1 | 1835 (C) | 12 |
| 06 | 0x309860 | 1655 | | 1657, 1658, 1659 | 1656 (inline) | 10 |
| 06 | 0x309e08 | 1660 | | 1662 ×2 | 1661 (inline) | 6 |
| 06 | 0x309fa8 | 1664 | | 1666 ×2 | 1665 (inline) | 20 |
| 07 | 0x31f3d8 | 1650 | | 1652, 1653, 1654 | 1651 (inline) | 11 |
| 07 | 0x31f5b0 | 1663, 1677 | | 1678, 1679, 1680 | 1677 (inline) | 7 + 0 |
| 07 | 0x31f780 | 1686 | | 1688, 1689, 1690 | – | 5 |
| 08 | 0x30bc00 | 1844 | | burst 1846 + 1847, n 1 | 1845 (C) | 18 |
| 09 | 0x30a678 | 1849 | | burst 1851, n 7 | 1850 (C) | 83 |
| 10 | 0x2eb6f0 / 0x2eb7e0 | 1855 / 1856 | | burst 1858, n 11 / – | – / 1857 (C) | 24 / 24 |
| 11 | 0x31d7d8 | 1859 | | burst 1861, n 11 | 1860 (C) | 29 |
| 12 | 0x30b6c8 | 1862 | | burst 1864, n 11 | 1863 (C) | 31 |
| 16 | 0x2d6740 | 800 | yes | burst 1870, n 11 | – | 13 |
| 17 | 0x2f5260 | 1873 | yes | burst 1875 n 11, burst 1878 n 1 | 1874 (C) | 28 |
| 17 | 0x2f5390 | 1876 | yes | 1875, burst 1878 n 1 | 1877 (C) | 24 |

Level 07's 1663 leaves the remains class 1677, which the table runs through the same function: its remains can be
broken again (more bolts). Kept as the table says. Levels 00, 05, 13, 14, 15, 18 place no template breakable.

**In the port** one module and one update (`breakables::update`) with one [`Recipe`] per copy
(`breakables::RECIPES`: reference level, address, classes, kept, pieces, remains). The registry has one variant
`ClassUpdate::Breakable(i)`; `LevelPorts` matches each recipe's function in every level's table by code identity
like any other port (`ClassUpdate::every()`), so the classes resolve on their levels without class-number lists
(`reference_level` is the recipe's level). The Novalis pot (754) and box (1813) are recipes 0 and 1 (the former
`Pot` / `Box` variants are gone).

## 3. Classes that do not fit

* Crates 500–511 (`CrateUpdate`, all 19 levels): their own stacking, TNT, respawn and drop rules; they already share
  `SetDeathBits` / `BoltBurst` with the template's kept-broken variant (`crate_::set_death_bits`, `bolt_burst`).
* Novalis 704 / 709–711 / 729 / 778 + 779 (`breakables/novalis.rs`): each its own burst of type-22 smoke and
  rock bits (696–698); 704 sets its death bits, the walls theirs by hand.
* Level 05 / 14 lamp 1511: breaks on hits 0x810000 from the front only, pieces through its own spawner, 24 light
  bursts and sparks; level 13 Blarg ship 1805: health, a mission trigger, pieces 0x709..0x70c ×2 and a beam
  explosion; level 15 construction arm 221: an animated machine; level 09 displays 276 / 1298 / 1299 and the
  airship parts 1182–1189: multi-stage mission props; levels 02 / 09 / 10 background ships 1212 / 1213 / 1973 and
  level 03 / 04 ship props: scripted fly-bys that only use `BreakFxB`. Not ported.

## 4. Glass and reflective surfaces

| mechanism | where | what |
|---|---|---|
| Moby metal packets, texture −2 chrome (128², the level's env map) | 12–23 classes per level (Clank, the wrench, …) | sphere-mapped per vertex; `moby_metal.wgsl` (existing) |
| Moby metal packets, texture −3 glass (64² env map) | **only the bolts 13–16**, on every level | same pass, the glass map |
| Draw-callback FX-21 sheens: the nanotech crate's panes (`0x301c00`, view vector ×2 → ST) and the ship canopy (`0x2a70a8`, the reflection `e − 2(n·e)n` → sphere map) | 806 on every level; ships 530–533 in scenes | quads through `fx_draw`'s one primitive material (`FxPrimMaterial`), FX texture 0x15, ALPHA 0x44 |
| Water reflections (ripple patches 751, the env overlay 1848) | water levels | `water_render` (not glass) |
| Screens (vendor, infobot) | textured mobys | no separate mechanism |

So the ship's glass is **not** the moby glass map: it is the draw-callback sheen the nanotech crate uses too (same
texture, same draw path, its own ST formula). The FX textures now reach `fx_prim.wgsl` in GS alpha units (0x80 =
1.0), as the particle textures do; before, their decoded 0..0xff alpha doubled every draw-callback alpha (the
nanotech glow, the fire fields and the glass).

**The canopy frame.** `MobyAttachToJoint(ship, 0, M)` 0x264508's second argument is a joint-**list** index
(`MobyMarkJointChain` 0x268d80 reads the list from class header +0x1c, as `FUN_002645a8` does): list 0 is the
canopy joint (530: `[0, 1, 2, 3, 6]`, joint 6 = the orange hatch; 531: joint 18; 533: joint 4), not joint 0. The
tables sit on it: posed with the class's own sequences, the 531 / 533 glass lies on the mesh within 0.026 / 0.056
units (530's cockpit is open: its rim matches the hatch outline). `SceneActorState::joint_matrix` now takes the
list (the infobot thrusters' `0x264508(m, 0, M)` too). The normal goes through the full matrix with w = 1 (the
translation is added), then is scaled to 0.1: kept. Not modelled: the ST cross-fade outside scenes, FX 1 in mode 6.
Open: in the port the scene actors Ratchet and Clank draw in front of the glass at frames 104–116 of scene 5,
while PCSX2 (images 27 / 28) shows them behind it; the hatch and the glass outline match.

**Infobot screen** (`images/46.webp`): the port's screen during scene 1 (frame 3291) is black with the same faint
edge highlight (bottom right); no change.

## 5. The bomb in water (`bomb_water.rs`)

`0x2c3300`'s water branch (the Mine Glove's `0x2bfe40`, class 74, has the same calls; not ported): the entry
(ripple −0.35 additive on every patch, the splash 775 of size 2 at alpha 0x70, 15 type-35 drops), a type-34 bubble
1 tick in `ticks(6) − 1` while sinking, and at the explosion: after `ticks(20)` in water the deep burst (150
bubbles at the water level 1 above, Ratchet's class sound 0x16, no hits / rings / flashes / high fireballs), and
in any water the shallow scorch (100 type-64 + 20 type-15 sparks when the surface is within 1.2 of the ground).
The water level and the ripple reach the engine's ripple patches through `ExternalUpdates::{water_height,
ripple_disturb}`. `dry` is now `water < ticks(20)` (was 20). Ledger: `bomb_water::tests` (three pools: shallow
floor, fuse just under the surface, deep water) checks every tick's draws; nothing is left in `fx.unported`.
