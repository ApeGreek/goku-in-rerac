# Veldin's last level (level 18, Kyzil Plateau again): the batch C checklist

Veldin's second visit: the run through Drek's base to the last arenas and the boss 1422 (`units::veldin_boss`), the
scenes 3 → 4 → 5 after it, the slideshow and the ending movie. The first visit is [planet_veldin.md](planet_veldin.md).
Started 2026-10-02.

## Classes placed (level18 class table; census 2026-10-01)

Ported before this page (earlier batches): the vendor 11, bolts 13, the Hydrodisplacer pads 341, the props of 419,
crates 500 / 501 / 502 / 505 / 511, the rolling mines 568, the boss's pads 583 and countdown 586, the floaters 587,
buried bolts 605, the Trespasser locks 615, the cutaway director 644, horny toads 749, the floor switch 830, the swing
targets 758 / 803, checkpoints 805, 832, the barricades 885..936, the petal door 1021, the kill cuboid 1039, lamps 1060,
gold bolts 1134, the teleporter pads 1135, the divers 1355, the falling platforms 1381, the boss 1422, 1432, path
gliders 1564, 1584, the save before the boss 1750, Giant Clank's pad 1899, the hoppers 1906.

| class | placed | what | status |
|---|---|---|---|
| 582 | 1 | the rail chooser: four rails laid both ways, only the one starting nearer Ratchet grindable; the grind catch widened; two nanotech clusters | **ported 2026-10-02** (`units::veldin_rails`; the rails reach the hero through `HeroFields::rail_radius`) |
| 1392 | 2 | the fields beside the Trespasser locks (a hum, pulsing bands until the lock is solved) | **ported 2026-10-02** (`units::veldin_lock_field`) |
| 1402 | 3 | the Hydrodisplacer pools (two heights, the group carried, three surface meshes, flag 0x7e swaps the starts) | **ported 2026-10-02** (`units::veldin_pool`; the meshes read from the overlay at the level load) |
| 1434 / 1435 | 1 / 2 | the pieces that turn over once the boss's checkpoint is set | **ported 2026-10-02** (`units::veldin_turnover`) |
| 1799 | 1 | the boss's jet in the scenes 0..3 | **ported 2026-10-02** (`units::veldin_scene_jet`) |
| 1890 | 1 | the energy fan (gone once its switch group is thrown; also Kalebo III's 1443) | **ported 2026-10-02** (`units::energy_fan`) |
| 638 | 46 | a creature (also on Kalebo III, 16) | open |
| 1356 | 10 | a creature (also on Kalebo III, 16) | open |
| 1454 | 3 | a creature | open |
| 1563 | 1 | a creature / effect driver | open |

## Open

- The classes 638, 1356, 1454, 1563 (next on this page).
- The grind-path cuts of the falling platforms (G-HERO-039); the rail chooser's radius channel is the hero side
  such a cut would also need.
- The pools' ripple patch word (`0x1db508 + n·0x1190`) has no reader on level 18: no ripple manager sets up the patch
  table there [L].
- A full play-through.
