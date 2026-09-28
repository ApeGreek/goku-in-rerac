# Class census: the unported moby classes and the shared systems they need

A static census (2026-09-28) of every moby class placed on levels 00–18 whose level update the port does not run
(gaps.md G-CLS-001). For each class it finds the update function on every level, groups the level copies that are the
same code into one **class-port unit** (the way `LevelPorts` matches ports), walks the unit's static call graph down to
the **shared functions** it calls (engine code in the overlay, level code other classes also call, the boot ELF), and
tags each shared function with the game system it belongs to and whether the port has it.

Nothing in `crates/` changed. The tool is a new `rc-trace` subcommand; the tags are a committed table.

**Numbers at this run.** 668 unported class-levels: **6,029 placed, 5,533 created** instances (G-CLS-001 said 5,543:
the sea port landed during the run and took 10), in **448 class-port units**. They call **473 shared functions**;
430 are tagged, 43 (all of them rare) are not. Every unit's call graph was walked (100 % of classes and instances); a
system verdict exists for **429 units / 5,485 created instances (99.1 %)**; 19 units / 48 instances call an untagged
function and nothing else the port lacks.

| verdict (per unit) | units | created instances |
|---|---|---|
| **cheap**: every shared call is one the port has (or trivial arithmetic, or the class family's own level code) | 226 | 2,794 |
| **missing**: calls at least one function of a system the port lacks | 162 | 2,486 |
| **partly**: calls a function the port has only in part | 41 | 205 |
| **unknown**: nothing missing, but one untagged function | 19 | 48 |

"Cheap" means the class needs **only its own code** ported: the shared systems are there. It is not free: the
appendix gives each unit's private code size (functions / words).

Three systems are **conditional**: they run only when the player does something specific, so a class that needs only
them still works in normal play. The big-head **cheat** manipulator (the enemies' `0x278720` and its level copies) runs
only with the cheat on; the **Suck Cannon** interface (`0x304100..0x305260`) only when the player captures the enemy;
**memcard_Save** only at a save point. Counting those as non-blocking, 238 units / 2,935 instances are cheap without the
Suck Cannon, and 244 units / 3,208 instances with it.

## Method (to regenerate)

```
cargo run -p rc-trace -- class-census          # writes work/census/*.tsv (about 15 s)
```

`tools/trace/src/class_census.rs` (the `class-census` command; `--out DIR`, `--tags FILE`, `--extracted DIR`). Inputs:
`extracted/levels/NN/{overlay.bin, gameplay_ntsc.bin}`, the port's registry `rc_game::moby_update::classes::LevelPorts`,
`tools/ghidra/names/clusters.tsv`, the decompiler export's `work/decomp/<program>/index.tsv` (names only) and the tag
table `tools/ghidra/names/census_systems.tsv`.

1. **Class list and instances.** Per level the class table (`LevelOverlay::vtbl`), and per class the placed instances
   (the gameplay instance list) and the created ones (`moby_spawn::loader_spawns` with an empty save, as
   `all_levels_smoke` counts them). A class is unported when `LevelPorts::get` gives no port and its update is not 0;
   the created total (5,533) equals `all_levels_smoke`'s report at the same moment.
2. **Call graph.** Function starts exactly as `LevelOverlay` finds them (`jal` targets, class-table updates, a frame
   set-up after `jr ra`, code addresses formed with `lui`/`%lo`). Edges: `jal`; a `j` out of the function (tail call);
   a code address formed with `lui`/`%lo` when the target also has start evidence of its own (a `jal` target, a
   class-table entry, or right after `jr ra` + delay slot) — callbacks such as draw callbacks and state functions
   stored into moby +0x74; a `.data` table of such starts formed the same way; and the fall-through into a start that
   has no evidence of its own. `jalr` sites are counted as unresolved indirect calls: **none** of the 448 units' own
   functions contains one (indirect calls live in engine code), so no call is left unresolved.
3. **Units.** From the update, a callee joins the unit when every caller of it is already in the unit; the callees
   left outside are the unit's shared functions. Level copies of an update are one unit when their masked code (the
   `Relocation` masking: `rc_formats::level_overlay::mask`) hashes the same.
4. **Identity of a shared function across levels**: its boot address (`boot:`); its level-01 copy by the port's
   `Relocation` (a function start in level 01, same extent for functions under 8 words); else its level-01 member in
   `clusters.tsv`; else the cluster (`C:` + 12 hex of the cluster hash: engine code that the level-01 overlay does not
   link); else the masked-code hash (`H:`).
5. **Has / partly / missing** was decided per function by searching `crates/` and `docs/` for the level-01 address
   (`shared.tsv` columns `crates_refs`, `crates_files`) and reading the cited code: a function the port cites as its
   model is "has", one it cites as counted / not ported is "missing", one without a citation was read in the decomp
   (`work/decomp/`, and Ghidra for spot checks of the graph, e.g. level03 `0x29dba8`'s callees) and tagged by what it
   does, marked `L` (inferred). Two extra statuses: `trivial` (arithmetic a port writes inline: zero a vector, fabs,
   memset) and `family` (level code shared only by the classes of one family, so part of their own port).
6. **Outputs** in `work/census/`: `classes.tsv` (level, class, placed, created, update, unit, port), `units.tsv`
   (instances, levels, classes, update copies, level-01 copy, private functions / words, `jalr`, shared keys),
   `shared.tsv` (per shared function: units, instances, the port's citations, callees), and with the tags
   `unit_systems.tsv` (verdict, systems, blocking functions, untagged) and `systems.tsv` (the ranking below).

**Limits.** The census sees **calls only**. A class that reads or writes engine state directly (the hero block, the
saved-game flags 0x13d388 of G-SAV-010, the moby's own fields) shows no dependency here; G-SAV-010 found 78 such
classes. Tags marked `L` were read from the decompiled code without a port to compare with. A function's status is
about the function; a system's status is "partly" when the port has any of its functions.

## Main table: missing or partial systems, ranked by the instances they block

A unit counts toward every system it needs; "classes" counts distinct class numbers. Addresses are level 01 unless
marked; `C:` rows cite one level copy (the others: `clusters.tsv`).

| system | status | gap row | classes | units | instances | example classes (levels: instances) | key shared functions (× units calling) |
|---|---|---|---|---|---|---|---|
| Big-head cheat manipulator [conditional] | missing | G-SAV-006 | 46 | 46 | 1,470 | 827 (L06, 10: 155); 1246 (L11: 126); 749 (L00, 18: 106); 193 (L09, 15: 93) | `0x278720` ×36; level copies `0x251d70` (L03; 9 levels) ×8, `0x25ea60` (L16) ×2 |
| Path helpers not linked on Novalis | partly | **G-CLS-025** (new) | 43 | 31 | 1,108 | 75/115/116… (L03: 246); 827 (L06, 10: 155); 749 (L00, 18: 106); 1564 (L00, 07, 10, 18: 92) | position + heading at t along a path: L00 `0x262e40` (13 levels) ×12 and level01 `0x277d40` ×3; nearest path segment within r: L00 `0x261d78` (9 levels) ×8; path point nearest a distance: L03 `0x264558` ×3, L05 `0x2a0260`; segment crosses a path wall: L02 `0x263710` (8 levels) ×3, L00 `0x261b48`; nearest points: L15 `0x265b38`; `0x277260` |
| Suck Cannon capture interface [conditional] | missing | G-WPN-003 | 16 | 16 | 919 | 827 (155); 1246 (126); 749 (106); 193 (93) | `0x305260` ×15, `0x304168` ×7, `0x304390` ×6, `0x3044a0` ×5, `0x304690` ×3, `0x304100` |
| Creature-layer copies not linked on Novalis | partly | **G-ENM-009** (new) | 28 | 29 | 555 | 749 (106); 238 (L12: 54); 1023 (L08, 09: 40); 294 (L12: 40) | decoy-aware target acquisition L03 `0x24e830` (4 levels) ×18; moby move-collide `0x26d270` ×11; steering toward a target L03 `0x247c10` (6 levels) ×7 and L02 `0x25b710`; random wander L00 `0x261630`; burn sparks `0x271258` (G-ENM-005) |
| NPC look-at manipulators and animation leftovers | partly | G-HERO-009 | 42 | 42 | 422 | 638 (L16, 18: 64); 238 (54); 623 (L05: 51); 294 (40) | `0x2777d8` ×40 (head look-at); moby kept on another moby's joint L12 `0x27b9c0` ×2; `MobyAnimBlendEx` `0x26c7a8` ×2; manipulator spin `0x2b52a8` |
| Effect-moby spawners not linked on Novalis | partly | **G-CLS-026** (new) | 17 | 18 | 281 | 623 (51); 79 (L05: 30); 541 (L16: 29); 1511 (L05: 26) | child moby with scale / spin / random velocity L05 `0x28dee8` (9 levels) ×15; random-class debris L06 `0x300c60` ×2; class-428 debris `0x2e2cc0` ×2 |
| Point-light flicker | partly | **G-REN-024** (new) | 6 | 6 | 256 | 1246 (126); 580 (L02: 82); 615 (19); 612 (L02: 15) | timed flicker of a point light L02 `0x25fb60` (02, 11) ×6 (the lights themselves, `0x252750` / `0x252850`, are ported) |
| Moby platforms and riders | partly | G-CLS-024 | 7 | 7 | 181 | 1246 (126); 812 (L05: 25); 574 (L03: 17); 1381 (L18: 8) | `0x2753b0` ×3, `0x275290`, `0x2752c0`, `0x275528` ×2 each; child mobys placed by the parent's matrix L18 `0x265358` ×2 |
| Hero scripted states and bodies | partly | G-HERO-002, G-HERO-005, G-LVL-002 (H2) | 37 | 34 | 119 | 587 (L18: 28); 615 Trespasser lock (19); 1059 (L07: 14); 1150/1151 (L09: 5) | the level's own hero `SetState` copies (not code-identical to level 01: H2) L04 `0x2159d8` ×11, L06 `0x2356a0` ×5, L08 `0x230b38` ×4, and five more level copies (L02, L05, L11, L12, L16, L17, L18); idle state per body L06 `0x239528` ×4; states 0x65..0x67 `0x249580` / `0x2495d0` ×4; body switch `0x231348`, leave body 2 L15 `0x208ca8` |
| Help messages | partly | G-UI-017 | 56 | 54 | 109 | 615 (19); 326 (L12: 8); 998 (L05: 5); 1035 (L06: 5) | `Help_Request` `0x225818` ×38, `try_set_help_message` `0x278f58` ×17, `force_help_message` `0x279000` ×3, help-box state `0x2258b0` / `0x225a28` / `0x225a88` |
| Particle types not ported | missing | G-PRT-001 | 13 | 14 | 90 | 28 (L14, 15: 43); 1139 (L05, 16: 12); 1440 (L00: 9); 170 (L13: 5) | type 69 L00 `0x274948` ×5; `SpawnImpactSparks` `0x2780b0` ×2; types 67 `0x289850`, 5 `0x27e750`, 28 `0x282ef0`, 40 `0x284d88`, 41 `0x285768`, 58 `0x287e70`, 74 `0x28a7a8` |
| Break-effect variant | partly | G-CLS-014 | 5 | 5 | 87 | 1382 (L17: 34); 573 (L03: 27); 1048 (L06: 20); 556 (L16: 5) | pieces + volume points L03 `0x251f08` (13 levels) ×5 |
| Follow / script camera settings from mobys | partly | G-HERO-026 | 32 | 29 | 74 | 615 (19); 351/1301 (L10: 10); 877 (L05: 6); 21/244 (L13: 4) | script-camera focus point L03 `0x2e9758` (15 levels) ×24, L02 `0x2f89b0` ×4; follow-camera fields `0x313740`, `0x313768`, `0x313820`, `0x313858`, owner lock `0x313560` / `0x313598`, `0x3137f8`, `0x313b48` |
| Blob shadows | partly | **G-REN-025** (new) | 7 | 7 | 70 | 1112 (L07: 31); 1269 (L12: 20); 1110 (L07: 12); 871 (L07: 4) | blob-shadow list 0x16e200 L04 `0x24d018` (13 levels) ×6; `0x26eec8` |
| Save bytes variant [L] | partly | G-SAV-003 | 2 | 3 | 45 | 44 (L15: 21, L17: 17); 78 (L15, 17: 7) | killed / visit bytes at 0x1bb024 L15 `0x2a35d8` ×3 |
| Save [conditional] | missing | G-SAV-002 | 39 | 38 | 39 | 1005/1016 (L02, 06: 2); single instances on most levels | `memcard_Save` `0x261448` ×37, `memcard_MakeWholeSave` `0x260e80` |
| Level sound on a slot | partly | G-AUD-007 | 11 | 1 | 38 | 466/480/485… (L04: 38) | L04 `0x27eca0` |
| Draw-callback sprite helper | partly | G-REN-020 | 20 | 8 | 20 | 1907/1908/1910 (L16, 17: 7); 1405/1406/1407 (L15, 17: 7) | `DrawSpriteHelper_A` `0x21e340` ×8 |
| Water-entry splash | missing | G-ENM-005 | 2 | 2 | 19 | 1059 (14); 35 (L05: 5) | `0x2ff768` ×2 |
| Leaving the level | missing | G-LVL-001 | 6 | 6 | 6 | 1109, 1448, 834, 436 | `OpenShipMenu` `0x279070` ×4, game-mode request `0x2a29a0` ×2 |
| Level height grid | missing | **G-LVL-008** (new) | 1 | 2 | 3 | 1400 (L08, 12, 14) | `0x278020` (byte grid 0x15fc98..0x15fca8) |
| Drone Device path | missing | G-WPN-002 | 1 | 1 | 1 | 1342 (L03) | `0x249530` |
| Slideshow | missing | G-CUT-003 | 1 | 1 | 1 | 1422 (L18) | `EnterSlideshowMode` `0x2ad558` |

Sole blockers (the one system a unit still needs, cheats and saves aside): paths 519 instances (11 units), Suck Cannon
273 (6), effect-moby spawners 103 (8), animation 81 (8), particle types 78 (8), creature layer 76 (6), hero states 43
(7), level sound 38 (1), platforms 33 (2), help 28 (18), camera 21 (4).

**What the port already has** (and the unported classes lean on most): math, timers and RNG (VecAdd, `ticks`, the
`rand` stream: called by most units), moby create / delete / matrix, animation (`MobyAnimBlend`, joints, manipulators), class
sounds and voice slots, hit messages and the hit resolver, the creature layer (knockback, hit flash, turn, walker,
target, arena paths, death explosion, `SpawnBeamExplosion`, `BreakFxB`), the ported particle types' spawners, collision queries,
trigger volumes, death bits and bolt drops, the shadow probe, draw callbacks with `FastDrawQuadReal` / FX textures /
the glow quad, the script camera and cinematic calls, the HUD banner, groups, splines and the flyer path driver, point
lights, the target list, mission flags.

## Cheap wins: classes whose shared calls the port already has

226 units (337 class-levels, **2,794 created instances**) call only functions the port has (or trivial arithmetic, or
their family's own level code). Each needs its own update ported, nothing shared first. The largest 45 (the rest are
in the appendix, verdict "cheap"); "private" is the unit's own code in 32-bit words:

| unit | classes | levels | created | update (first copy) | private words |
|---|---|---|---|---|---|
| U21 | 1060 | 00, 02, 05, 18 | 334 | L00:2df4f8 | 100 |
| U400 | 212, 1412 | 13 | 130 | L13:2e1638 | 708 |
| U296 | 1181 | 09 | 124 | L09:304360 | 584 |
| U546 | 885, 888, 891, 892, 894, 900, 901, 936 | 18 | 119 | L18:2e9768 | 450 |
| U261 | 344, 547, 548, 549, 550, 551, 588, 589, 590, 591, 592, 593, 594, 595, 596, 597, 598, 782, 783, 784, 785 | 08 | 85 | L08:2dba40 | 220 |
| U287 | 1182, 1183, 1184, 1185, 1186, 1187, 1188, 1189 | 09 | 80 | L09:2c26c8 | 852 |
| U92 | 87, 283, 346, 419, 690, 765, 789, 1104, 1140, 1278, 1280, 1672 | 02, 05, 06, 07, 10, 12, 14, 15, 16, 18 | 67 | L02:2dd4d0 | 2 |
| U536 | 1843 | 17 | 60 | L17:2f40d8 | 1180 |
| U409 | 1261 | 13 | 55 | L13:307b10 | 852 |
| U362 | 1319 | 11 | 52 | L11:319838 | 1446 |
| U526 | 1359, 1360, 1361, 1362, 1363, 1364, 1367, 1369, 1372, 1373 | 17 | 52 | L17:2e87d8 | 396 |
| U466 | 408 | 15, 17 | 48 | L15:2cb4c8 | 604 |
| U469 | 937 | 15 | 48 | L15:2e46b0 | 48 |
| U460 | 123 | 15, 17 | 46 | L15:2a6e68 | 496 |
| U88 | 296, 652, 653 | 02, 12 | 42 | L02:2dc6b0 | 216 |
| U556 | 1584 | 18 | 38 | L18:2fa728 | 88 |
| U485 | 471 | 16 | 37 | L16:2c9878 | 524 |
| U456 | 77 | 15, 17 | 36 | L15:2a2488 | 248 |
| U547 | 1355 | 18 | 36 | L18:2efb88 | 606 |
| U521 | 99 | 17 | 35 | L17:2a8da0 | 616 |
| U234 | 886 | 07, 12 | 30 | L07:30bf90 | 350 |
| U493 | 650 | 16 | 29 | L16:2d5cb8 | 62 |
| U240 | 129, 130, 182, 183, 360, 1063, 1078, 1079, 1131 | 07, 13 | 28 | L07:310df0 | 310 |
| U465 | 233 | 15 | 28 | L15:2c5630 | 2072 |
| U423 | 30 | 14 | 27 | L14:2b3bf0 | 786 |
| U173 | 838 | 05 | 26 | L05:30dc68 | 356 |
| U222 | 1512 | 06 | 26 | L06:308c68 | 374 |
| U273 | 621 | 08 | 25 | L08:2f44c0 | 358 |
| U496 | 933 | 16 | 25 | L16:2ddde0 | 302 |
| U398 | 127, 128, 159, 169 | 13 | 24 | L13:2c7f38 | 278 |
| U442 | 1224 | 14 | 24 | L14:3015d0 | 894 |
| U178 | 852, 853 | 05 | 21 | L05:314eb0 | 144 |
| U204 | 1038 | 06, 10, 17 | 21 | L06:2f7288 | 426 |
| U214 | 1091, 1092, 1093, 1094, 1095, 1096, 1097, 1098, 1103 | 06 | 21 | L06:300df0 | 96 |
| U274 | 648 | 08, 10 | 21 | L08:2f5830 | 346 |
| U161 | 133 | 05 | 20 | L05:2dac80 | 558 |
| U489 | 552 | 16 | 20 | L16:2cf4a8 | 330 |
| U132 | 915, 916, 917 | 03, 10 | 19 | L03:2dc310 | 28 |
| U380 | 1259 | 12 | 18 | L12:302f30 | 648 |
| U471 | 1250 | 15 | 18 | L15:2e73c0 | 72 |
| U163 | 341 | 05, 07, 11, 12, 18 | 17 | L05:2f8080 | 168 |
| U175 | 843 | 05 | 16 | L05:30e508 | 188 |
| U241 | 1069 | 07 | 16 | L07:3112c8 | 600 |
| U401 | 224, 228 | 13 | 16 | L13:2e1cc8 | 416 |
| U492 | 647 | 16 | 16 | L16:2d59e0 | 60 |

Notes: U21 (class 1060, 334 lamps on 00 / 02 / 05 / 18) is a pulsing ambient colour plus a draw callback of glow
quads over its group; U92 (12 classes on 10 levels, 67 instances) has a 2-word update (`jr ra`): it does nothing, so
a registry entry is the whole port. Under G-CLS-027 (new) these are one batch of class ports.

## Unique classes: the ones that need something no other class needs

A blocking function (missing or partly) that exactly one unit calls:

| unit | classes | levels | created | its own blocker [system] |
|---|---|---|---|---|
| U145 | 466,480,485,486,488,490,493,494,495,498,555 | 04 | 38 | `C:bcf4ccdaa994` (FUN_0027eca0) [lsound] |
| U382 | 1269 | 12 | 20 | `L01:0026eec8` (FUN_0026eec8) [shadow] |
| U86 | 615 | 02,04,06,08,11,13,18 | 19 | `C:0e46409356dd` (FUN_0022c418) [herostate] |
| U410 | 1262 | 13 | 16 | `C:7c53e29c001b` (FUN_00261b48) [path] |
| U85 | 612 | 02 | 15 | `C:a8d6490c966c` (FUN_0025b710) [creature] |
| U192 | 1139 | 05,16 | 12 | `L01:00289850` PartType67Spawn [ptype] |
| U420 | 8 | 14 | 11 | `L01:00271258` (FUN_00271258) [creature] |
| U148 | 563 | 04 | 10 | `L01:002b52a8` (FUN_002b52a8) [anim] |
| U399 | 170 | 13 | 5 | `L01:0027e750` PartType05Spawn [ptype] |
| U384 | 1281 | 12 | 4 | `L01:00282ef0` PartType28Spawn [ptype] |
| U125 | 816 | 03 | 3 | `L01:00285768` PartType41Spawn [ptype] |
| U311 | 702 | 10 | 3 | `L01:00284d88` PartType40Spawn [ptype] |
| U164 | 439 | 05,16 | 2 | `C:4d615d92f5c8` (FUN_0021e398) [herostate], `C:ef4e6bb0b47f` (FUN_0024cee8) [herostate] |
| U315 | 947 | 10 | 2 | `L01:00277260` (FUN_00277260) [path] |
| U136 | 1342 | 03 | 1 | `L01:00249530` (FUN_00249530) [gadget] |
| U207 | 1051 | 06 | 1 | `L01:003137b0` (FUN_003137b0) [camera] |
| U209 | 1061 | 06 | 1 | `L01:00231348` (FUN_00231348) [herostate] |
| U243 | 1106 | 07 | 1 | `L01:00287e70` PartType58Spawn [ptype] |
| U358 | 1242 | 11 | 1 | `L01:0028a7a8` PartType74Spawn [ptype] |
| U381 | 1267 | 12 | 1 | `C:84d6abe16fc6` (FUN_002400d0) [herostate] |
| U527 | 1379 | 17 | 1 | `L01:00225a88` (FUN_00225a88) [help] |
| U551 | 1422 | 18 | 1 | `L01:002ad558` EnterSlideshowMode [cine] |
| U557 | 1750 | 18 | 1 | `L01:00260e80` memcard_MakeWholeSave [save] |

Besides these, the unit-private code of the big units is by definition unique: U118 (level 03's nine 75..132 / 795
classes, 246 instances, 1,820 words behind one update `0x29dba8`), U198 (827, 2,150 words), U359 (1246, 1,924 words),
U15 (749, 1,736 words) and U289 (193, 1,636 words) are the largest.

## Classes the census could not resolve

19 units / 48 instances have no missing function but call an untagged one (reason: the function was not read; they
are all rare). 41 more units already have a known blocker and also call an untagged function (listed in the
appendix as "untagged ×n").

| unit | classes | levels | created | verdict | untagged shared functions |
|---|---|---|---|---|---|
| U461 | 148,255 | 15 | 11 | unknown | `C:7a28b0d324fa` |
| U462 | 154 | 15 | 7 | unknown | `C:7a28b0d324fa` |
| U291 | 276,1298,1299 | 09 | 5 | unknown | `C:a0194c2bd0e8` (FUN_002ef868) |
| U219 | 1123 | 06 | 4 | unknown | `H:cd35c1d4fa65eb86` (FUN_00217300) |
| U299 | 1285,1286,1287,1288 | 09 | 3 | unknown | `C:a0194c2bd0e8` (FUN_002ef868) |
| U268 | 462 | 08 | 2 | unknown | `C:01476b25f86a` (FUN_002de3e0), `C:60ae58782f11` (FUN_002e1c98) |
| U283 | 1629 | 08,14 | 2 | unknown | `C:b376dbd21da4` (FUN_00272cb8) |
| U333 | 1424 | 10 | 2 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U348 | 1075 | 11 | 2 | unknown | `C:ada069a1b3e9` (FUN_00316128) |
| U266 | 444 | 08 | 1 | unknown | `C:01476b25f86a` (FUN_002de3e0), `C:60ae58782f11` (FUN_002e1c98) |
| U269 | 463 | 08 | 1 | unknown | `C:01476b25f86a` (FUN_002de3e0), `C:60ae58782f11` (FUN_002e1c98) |
| U332 | 1421 | 10 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U356 | 1180 | 11 | 1 | unknown | `C:075ded76f1bf` (FUN_0030f270) |
| U433 | 685 | 14 | 1 | unknown | `C:a3aa13a8e1c8` (FUN_00263ad0) |
| U438 | 921 | 14 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U439 | 922 | 14 | 1 | unknown | `C:a3aa13a8e1c8` (FUN_00263ad0) |
| U445 | 1352 | 14 | 1 | unknown | `C:ecc426044254` (FUN_002f5a50) |
| U447 | 1395 | 14 | 1 | unknown | `C:a3aa13a8e1c8` (FUN_00263ad0) |
| U555 | 1563 | 18 | 1 | unknown | `L01:002d2d08` (FUN_002d2d08) |

Eleven units call **no shared function at all** (U92's empty update, U552 1432, U448 1397, U83 331, U98 743, U105 854,
U134 997, U191 1099, U310 353, U405 533, U451 1418): pure class code, counted as cheap.

### Untagged shared functions (for a later pass)

Key, a level copy, units calling it, created instances of those units, size in words. `C:` keys are cluster hashes
(`tools/ghidra/names/clusters.tsv` lists every copy).

| function | site | units | instances | words |
|---|---|---|---|---|
| `C:7a28b0d324fa` | L00:25a400 | 2 | 18 | 8 |
| `boot:001251e8` | L07:1251e8 | 1 | 14 | 0 |
| `H:cd35c1d4fa65eb86` (FUN_00217300) | L06:217300 | 2 | 9 | 2 |
| `C:16bfd13f798c` (FUN_0025c758) | L02:25c758 | 8 | 8 | 12 |
| `C:a0194c2bd0e8` (FUN_002ef868) | L09:2ef868 | 2 | 8 | 82 |
| `C:ecc426044254` (FUN_002f5a50) | L10:2f5a50 | 4 | 5 | 10 |
| `C:1d746ca2af76` (FUN_00312b40) | L06:312b40 | 4 | 5 | 12 |
| `C:e00f1d73700a` (FUN_0028ec18) | L07:28ec18 | 3 | 5 | 20 |
| `C:b376dbd21da4` (FUN_00272cb8) | L08:272cb8 | 3 | 5 | 128 |
| `C:f638735f808a` (FUN_002d3918) | L03:2d3918 | 2 | 5 | 202 |
| `C:4875a26d9e16` (FUN_00258090) | L08:258090 | 4 | 4 | 30 |
| `C:60ae58782f11` (FUN_002e1c98) | L08:2e1c98 | 3 | 4 | 54 |
| `C:01476b25f86a` (FUN_002de3e0) | L08:2de3e0 | 3 | 4 | 82 |
| `L01:00265210` (FUN_00265210) | L11:276260 | 3 | 3 | 20 |
| `L01:00219580` UpdateViewContext__Fv | L03:1f12e8 | 3 | 3 | 392 |
| `C:cdf5239bdc5f` (FUN_002f53e8) | L06:2f53e8 | 3 | 3 | 46 |
| `C:a717d063371e` (FUN_002f5360) | L06:2f5360 | 3 | 3 | 34 |
| `C:a3aa13a8e1c8` (FUN_00263ad0) | L14:263ad0 | 3 | 3 | 18 |
| `C:8d520530637e` (FUN_0029aec0) | L06:29aec0 | 3 | 3 | 40 |
| `C:c836aca3587d` (FUN_00204070) | L10:204070 | 2 | 3 | 74 |
| `C:ada069a1b3e9` (FUN_00316128) | L11:316128 | 2 | 3 | 14 |
| `L01:002bcb90` (FUN_002bcb90) | L00:2a7210 | 1 | 3 | 138 |
| `L01:00274738` (FUN_00274738) | L00:25f878 | 1 | 3 | 26 |
| `C:fa408a22bb62` (FUN_0020d760) | L08:20d760 | 1 | 3 | 66 |
| `C:fe1252f3da20` | L06:2e5e10 | 2 | 2 | 56 |
| `C:bcbd2dbe1730` (FUN_002f99b0) | L06:2f99b0 | 2 | 2 | 30 |
| `C:9c39817b4fe9` (FUN_002e9ea0) | L06:2e9ea0 | 2 | 2 | 36 |
| `C:883715506a19` (FUN_002b84e8) | L13:2b84e8 | 2 | 2 | 42 |
| `C:40920504317a` (FUN_002f9948) | L06:2f9948 | 2 | 2 | 26 |
| `C:075ded76f1bf` (FUN_0030f270) | L11:30f270 | 2 | 2 | 14 |
| `L01:00300518` (FUN_00300518) | L01:300518 | 1 | 1 | 4 |
| `L01:002d2d08` (FUN_002d2d08) | L18:2bf908 | 1 | 1 | 748 |
| `L01:002a2360` (FUN_002a2360) | L13:298cb0 | 1 | 1 | 60 |
| `L01:0025dd98` (FUN_0025dd98) | L07:271508 | 1 | 1 | 72 |
| `L01:0024b108` (FUN_0024b108) | L02:238fc8 | 1 | 1 | 26 |
| `L01:0021c4f0` fun_001f5ab0 | L11:22c158 | 1 | 1 | 280 |
| `L01:00218838` (FUN_00218838) | L11:2284a0 | 1 | 1 | 78 |
| `L01:0020f230` (FUN_0020f230) | L02:1fe518 | 1 | 1 | 30 |
| `L01:0020cf28` (FUN_0020cf28) | L07:213110 | 1 | 1 | 72 |
| `C:dd572bc43542` (FUN_0029b8d8) | L07:29b8d8 | 1 | 1 | 94 |
| `C:a80dbefc5193` (FUN_00301058) | L18:301058 | 1 | 1 | 18 |
| `C:754a80d04806` (FUN_00227cf8) | L06:227cf8 | 1 | 1 | 70 |
| `C:12bb06e65f13` (FUN_00204110) | L04:204110 | 1 | 1 | 84 |

## Appendix: every unit

One row per class-port unit, largest first. *created/placed*: instances over all its levels. *updates*: the update in
each level (`Lnn:address`). *private*: the unit's own functions / words. *needs*: the systems of the functions it calls
that the port lacks (missing) or has in part (partly); *has*: systems it calls that the port has. System keys: `anim`
animation, `breakfx` break effects, `camera` follow / script camera settings, `cheat` big-head cheat, `cine` script
camera and cinematic calls, `coll` collision queries, `creature` creature layer, `death` death bits and bolt drops,
`family` the family's own level code, `fxdraw` draw callbacks, `fxmoby` effect-moby spawners, `gadget` gadget
paths, `group` moby groups, `heightmap` level height grid, `help` help messages, `herostate` hero states and bodies,
`hit` hit messages, `hud` HUD, `levelexit` leaving the level, `light` point lights, `lsound` level sounds, `math` math /
timers / RNG, `menu` menu mobys, `mission` mission flags and items, `moby` moby lifecycle, `particles` / `ptype` ported /
unported particle types, `path` paths and splines, `platform` platforms, `react` Suck Cannon interface, `save` saves,
`shadow` shadows, `sound` class sounds, `target` target list, `trig` trigger volumes, `water` water. The per-function
tags are `tools/ghidra/names/census_systems.tsv`.

| unit | classes | levels | created/placed | updates | private | verdict | needs | has |
|---|---|---|---|---|---|---|---|---|
| U21 | 1060 | 00 02 05 18 | 334/334 | L00:2df4f8, L02:2eabf8, L05:319ae8 +1 | 2/100 | cheap | — | fxdraw group math moby |
| U118 | 75, 115, 116, 117, 118, 119, 120, 132, 795 | 03 | 246/246 | L03:29dba8 | 9/1820 | missing | missing: path | anim creature death group hit hud lsound math moby particles path sound |
| U198 | 827 | 06 10 | 155/193 | L06:2e8678, L10:2cc8b8 | 11/2150 | missing | missing: cheat,path,react | anim coll creature death fxmoby hit math moby particles shadow sound |
| U400 | 212, 1412 | 13 | 130/130 | L13:2e1638 | 3/708 | cheap | — | coll creature hit math moby sound |
| U359 | 1246 | 11 | 126/126 | L11:314318 | 4/1924 | missing | missing: cheat,react; partly: light,platform | anim creature death hit math moby particles shadow |
| U296 | 1181 | 09 | 124/124 | L09:304360 | 2/584 | cheap | — | anim creature math moby |
| U546 | 885, 888, 891, 892, 894, 900, 901, 936 | 18 | 119/119 | L18:2e9768 | 2/450 | cheap | — | coll creature group hit math moby particles sound |
| U15 | 749 | 00 18 | 106/106 | L00:2d4610, L18:2e02e8 | 9/1736 | missing | missing: cheat,creature,path,react | anim creature death hit math moby shadow sound |
| U289 | 193 | 09 15 | 93/170 | L09:2e27d8, L15:2bc608 | 2/1636 | missing | missing: cheat,react | anim coll creature death group hit math moby shadow |
| U26 | 1564 | 00 07 10 18 | 92/92 | L00:2e3a88, L07:31ede0, L10:2eada8 +1 | 1/136 | missing | missing: path | anim math moby |
| U510 | 1667, 1668, 1669, 1670, 1671 | 16 | 90/90 | L16:2e76b8 | 3/466 | missing | missing: path | math moby particles sound |
| U261 | 344, 547, 548, 549, 550, 551, 588, 589, 590, 591, 592, 593, 594, 595, 596, 597, 598, 782, 783, 784, 785 | 08 | 85/85 | L08:2dba40 | 1/220 | cheap | — | anim coll math moby |
| U84 | 580 | 02 | 82/115 | L02:2d3e50 | 8/2582 | missing | missing: cheat,path,react; partly: light | anim coll creature death hit math moby particles shadow sound trig |
| U287 | 1182, 1183, 1184, 1185, 1186, 1187, 1188, 1189 | 09 | 80/80 | L09:2c26c8 | 3/852 | cheap | — | anim creature death math moby particles sound |
| U257 | 252 | 08 14 | 77/148 | L08:2d2af0, L14:2d98f0 | 4/1414 | missing | missing: cheat,react | anim creature death fxdraw hit math moby shadow sound |
| U92 | 87, 283, 346, 419, 690, 765, 789, 1104, 1140, 1278, 1280, 1672 | 02 05 06 07 10 12 14 15 16 18 | 67/67 | L02:2dd4d0, L02:2e2220, L05:31ace8 +12 | 1/2 | cheap | — | — |
| U491 | 638 | 16 18 | 64/65 | L16:2d2cf0, L18:2dc918 | 9/1990 | missing | missing: cheat; partly: anim | anim creature death fxmoby hit hud lsound math moby particles sound |
| U536 | 1843 | 17 | 60/60 | L17:2f40d8 | 6/1180 | cheap | — | anim coll creature fxdraw hit hud lsound math moby sound |
| U394 | 63 | 13 | 58/85 | L13:2b50d8 | 9/3080 | missing | missing: cheat,react | anim coll creature death hit math moby particles path shadow sound trig |
| U324 | 1202 | 10 | 57/65 | L10:2e1d38 | 7/2976 | missing | missing: cheat | anim coll creature death hit hud lsound math moby particles shadow |
| U409 | 1261 | 13 | 55/55 | L13:307b10 | 4/852 | cheap | — | coll creature death fxmoby hit math moby particles sound |
| U368 | 238 | 12 | 54/71 | L12:2e1be0 | 11/2310 | missing | missing: cheat,creature,path,react; partly: anim | anim creature death group hit math moby particles shadow |
| U362 | 1319 | 11 | 52/52 | L11:319838 | 7/1446 | cheap | — | anim coll creature fxdraw hit hud lsound math moby sound |
| U526 | 1359, 1360, 1361, 1362, 1363, 1364, 1367, 1369, 1372, 1373 | 17 | 52/52 | L17:2e87d8 | 1/396 | cheap | — | math moby sound trig |
| U165 | 623 | 05 | 51/56 | L05:301f48 | 8/2570 | missing | missing: cheat,fxmoby; partly: anim | anim coll creature death hit math moby particles shadow trig |
| U466 | 408 | 15 17 | 48/48 | L15:2cb4c8, L17:2cbe80 | 4/604 | cheap | — | anim coll fxdraw math sound |
| U469 | 937 | 15 | 48/48 | L15:2e46b0 | 1/48 | cheap | — | math |
| U288 | 52 | 09 16 | 46/61 | L09:2c5990, L16:2a1088 | 2/902 | missing | missing: cheat | anim creature death group hit math moby shadow sound |
| U460 | 123 | 15 17 | 46/46 | L15:2a6e68, L17:2ac740 | 1/496 | cheap | — | anim coll fxdraw math particles |
| U422 | 28 | 14 15 | 43/58 | L14:2b17d8, L15:2955c0 | 19/2144 | missing | missing: cheat,ptype | anim coll creature death fxdraw hit light math moby path |
| U88 | 296, 652, 653 | 02 12 | 42/42 | L02:2dc6b0, L12:2e8c10 | 1/216 | cheap | — | math platform |
| U276 | 1023 | 08 09 | 40/41 | L08:301158, L09:3009d0 | 5/1788 | missing | missing: cheat,creature | anim coll creature death hit math moby shadow |
| U372 | 294 | 12 | 40/40 | L12:2e72c0 | 12/1836 | missing | missing: cheat,creature,path; partly: anim | anim coll creature death group hit math moby sound trig |
| U145 | 466, 480, 485, 486, 488, 490, 493, 494, 495, 498, 555 | 04 | 38/38 | L04:2ca420 | 14/3032 | partly | partly: lsound | anim coll creature group hit hud math moby particles sound trig |
| U556 | 1584 | 18 | 38/38 | L18:2fa728 | 1/88 | cheap | — | math moby platform |
| U485 | 471 | 16 | 37/37 | L16:2c9878 | 4/524 | cheap | — | fxdraw math platform sound trig |
| U456 | 77 | 15 17 | 36/36 | L15:2a2488, L17:2a7b58 | 1/248 | cheap | — | anim creature fxdraw hit math particles |
| U547 | 1355 | 18 | 36/36 | L18:2efb88 | 5/606 | cheap | — | creature fxdraw math moby sound |
| U521 | 99 | 17 | 35/35 | L17:2a8da0 | 3/616 | cheap | — | anim coll fxdraw hit math moby particles trig |
| U529 | 1382 | 17 | 34/40 | L17:2eeb68 | 15/1568 | missing | missing: cheat,react; partly: anim,breakfx | anim coll creature death fxdraw hit math moby shadow trig |
| U245 | 1112 | 07 | 31/31 | L07:3196e0 | 2/556 | missing | missing: creature,shadow | anim coll creature death hit math moby shadow |
| U313 | 857 | 10 | 31/31 | L10:2d4078 | 17/4300 | missing | missing: creature,path | anim coll creature fxdraw group hit math moby particles shadow sound target trig |
| U160 | 79 | 05 | 30/30 | L05:2d7140 | 3/576 | missing | missing: fxmoby,path | anim fxdraw hit math moby sound |
| U234 | 886 | 07 12 | 30/30 | L07:30bf90, L12:2ff838 | 1/350 | cheap | — | creature group math mission moby sound |
| U487 | 541 | 16 | 29/29 | L16:2cddb8 | 10/1538 | missing | missing: cheat,fxmoby | anim coll creature death fxdraw hit math moby particles sound |
| U493 | 650 | 16 | 29/29 | L16:2d5cb8 | 1/62 | cheap | — | math sound |
| U141 | 340 | 04 | 28/63 | L04:2c2270 | 8/2512 | missing | missing: cheat,creature,react | anim coll creature death hit math moby particles shadow sound trig |
| U240 | 129, 130, 182, 183, 360, 1063, 1078, 1079, 1131 | 07 13 | 28/28 | L07:310df0, L13:2c8390 | 1/310 | cheap | — | math sound |
| U465 | 233 | 15 | 28/28 | L15:2c5630 | 8/2072 | cheap | — | anim coll creature death group hit math moby path shadow sound trig |
| U543 | 587 | 18 | 28/28 | L18:2d82c0 | 3/546 | partly | partly: herostate | math moby particles platform sound |
| U120 | 573 | 03 | 27/87 | L03:2c5bb8 | 4/1138 | missing | missing: cheat,creature,react; partly: breakfx | anim creature death hit math moby path shadow trig |
| U423 | 30 | 14 | 27/27 | L14:2b3bf0 | 4/786 | cheap | — | anim creature death hit math moby trig |
| U173 | 838 | 05 | 26/26 | L05:30dc68 | 2/356 | cheap | — | fxdraw math sound |
| U194 | 1511 | 05 | 26/26 | L05:31c8e0 | 4/606 | missing | missing: fxmoby | death fxdraw hit math moby particles sound |
| U222 | 1512 | 06 | 26/26 | L06:308c68 | 1/374 | cheap | — | creature math moby particles |
| U169 | 812 | 05 | 25/25 | L05:30bf98 | 1/68 | partly | partly: platform | math platform |
| U264 | 438 | 08 | 25/25 | L08:2de848 | 2/422 | missing | missing: path | coll creature hit hud lsound math moby particles |
| U273 | 621 | 08 | 25/25 | L08:2f44c0 | 1/358 | cheap | — | creature hit math moby sound |
| U496 | 933 | 16 | 25/25 | L16:2ddde0 | 1/302 | cheap | — | creature hit math moby sound |
| U398 | 127, 128, 159, 169 | 13 | 24/24 | L13:2c7f38 | 1/278 | cheap | — | math sound |
| U442 | 1224 | 14 | 24/24 | L14:3015d0 | 4/894 | cheap | — | coll fxdraw math moby particles |
| U464 | 221 | 15 | 24/24 | L15:2c2938 | 2/450 | missing | missing: fxmoby; partly: anim | anim creature death hit math moby particles sound |
| U507 | 1445 | 16 | 24/52 | L16:2e5e08 | 5/796 | missing | missing: cheat,react | anim creature death hit math moby shadow |
| U452 | 1511 | 14 | 22/22 | L14:307ad8 | 5/606 | missing | missing: fxmoby | death fxdraw hit math moby particles sound |
| U178 | 852, 853 | 05 | 21/21 | L05:314eb0 | 1/144 | cheap | — | creature math sound |
| U204 | 1038 | 06 10 17 | 21/21 | L06:2f7288, L10:2d9550, L17:2e3690 | 3/426 | cheap | — | hit math moby particles |
| U214 | 1091, 1092, 1093, 1094, 1095, 1096, 1097, 1098, 1103 | 06 | 21/21 | L06:300df0 | 1/96 | cheap | — | math moby |
| U274 | 648 | 08 10 | 21/21 | L08:2f5830, L10:2c74b8 | 1/346 | cheap | — | math particles |
| U454 | 44 | 15 | 21/21 | L15:2979d8 | 16/3360 | partly | partly: anim,death | anim coll creature death fxdraw hit math moby particles shadow sound trig |
| U482 | 257 | 16 | 21/21 | L16:2c3d38 | 1/174 | missing | missing: path | math moby sound trig |
| U161 | 133 | 05 | 20/20 | L05:2dac80 | 4/558 | cheap | — | anim creature hit math moby particles sound |
| U206 | 1048 | 06 | 20/27 | L06:2f7d78 | 4/1780 | missing | missing: cheat,path; partly: breakfx | anim coll creature death hit math moby shadow trig |
| U382 | 1269 | 12 | 20/20 | L12:304e00 | 1/428 | missing | missing: creature,shadow | anim coll creature death hit math moby |
| U489 | 552 | 16 | 20/20 | L16:2cf4a8 | 6/330 | cheap | — | fxdraw math sound |
| U539 | 568 | 18 | 20/20 | L18:2d5918 | 2/404 | missing | missing: react | anim creature hit math |
| U132 | 915, 916, 917 | 03 10 | 19/19 | L03:2dc310, L10:2d7fe8 | 1/28 | cheap | — | math moby |
| U86 | 615 | 02 04 06 08 11 13 18 | 19/19 | L02:2d8ad0, L04:2d4ae0, L06:2e1940 +4 | 3/694 | missing | missing: camera; partly: help,herostate,light | anim cine creature fxdraw herostate math sound |
| U380 | 1259 | 12 | 18/18 | L12:302f30 | 3/648 | cheap | — | coll fxdraw hit math moby trig |
| U471 | 1250 | 15 | 18/18 | L15:2e73c0 | 1/72 | cheap | — | math platform |
| U559 | 1906 | 18 | 18/18 | L18:2fbb98 | 4/834 | missing | missing: path,react | anim creature fxdraw hit math |
| U121 | 574 | 03 | 17/19 | L03:2c6fd0 | 7/3452 | missing | missing: cheat,creature; partly: anim,platform | anim camera coll creature death hit math moby particles path shadow sound trig |
| U163 | 341 | 05 07 11 12 18 | 17/17 | L05:2f8080, L07:2f5508, L11:2f30b0 +2 | 2/168 | cheap | — | anim math |
| U520 | 44 | 17 | 17/17 | L17:29f8d8 | 25/3300 | missing | missing: cheat; partly: anim,death | anim coll creature death fxdraw hit math moby particles shadow sound trig |
| U175 | 843 | 05 | 16/16 | L05:30e508 | 1/188 | cheap | — | math sound |
| U241 | 1069 | 07 | 16/16 | L07:3112c8 | 2/600 | cheap | — | creature math moby platform sound |
| U401 | 224, 228 | 13 | 16/16 | L13:2e1cc8 | 1/416 | cheap | — | anim coll math moby sound |
| U410 | 1262 | 13 | 16/16 | L13:307e98 | 7/2028 | missing | missing: cheat,path | anim coll creature death fxdraw herostate hit math moby particles shadow trig |
| U483 | 270 | 16 | 16/16 | L16:2c4720 | 1/1400 | missing | missing: cheat,creature,fxmoby,react | anim coll creature hit math moby particles sound |
| U492 | 647 | 16 | 16/16 | L16:2d59e0 | 1/60 | cheap | — | math |
| U109 | 1213, 1973 | 02 09 10 | 15/15 | L02:2ece18, L09:306720, L10:2ebb08 | 4/726 | missing | missing: path | anim creature fxdraw group hit hud lsound math moby particles sound |
| U323 | 1199 | 10 | 15/51 | L10:2e01a8 | 3/1764 | missing | missing: cheat | anim coll creature death hit math moby shadow sound |
| U467 | 491 | 15 | 15/15 | L15:2cf3a8 | 7/2642 | missing | missing: cheat,creature,fxmoby | anim breakfx coll creature death group herostate hit math moby particles path sound trig |
| U470 | 1209 | 15 17 | 15/15 | L15:2e5958, L17:2e5f50 | 2/354 | cheap | — | anim group math moby sound |
| U498 | 1356 | 16 18 | 15/15 | L16:2e22d8, L18:2f0920 | 8/1032 | missing | missing: fxmoby,path; partly: anim | anim creature death hit math moby sound |
| U523 | 669 | 17 | 15/15 | L17:2d77f0 | 2/518 | cheap | — | anim coll fxdraw herostate math moby particles |
| U85 | 612 | 02 | 15/15 | L02:2d7748 | 6/1580 | missing | missing: cheat,creature; partly: light | anim coll creature death hit math moby particles shadow trig |
| U200 | 911 | 06 | 14/14 | L06:2f3ad8 | 4/690 | cheap | — | coll math moby particles sound trig |
| U239 | 1059 | 07 | 14/14 | L07:30f5f0 | 20/3740 | missing | missing: cheat,creature,water; partly: anim,herostate; untagged ×1 | anim coll creature death group hit math moby particles target trig water |
| U334 | 1544 | 10 | 14/14 | L10:2ea1f0 | 4/698 | cheap | — | creature math moby particles |
| U501 | 1401 | 16 | 14/14 | L16:2e37a0 | 7/856 | cheap | — | anim creature death hit math moby particles path trig |
| U142 | 427 | 04 | 13/13 | L04:2c4850 | 8/2400 | missing | missing: creature; partly: anim | anim coll creature death hit math moby particles shadow sound |
| U212 | 1068 | 06 10 | 13/15 | L06:2fdbd0, L10:2dabe8 | 6/2074 | missing | missing: cheat,fxmoby,path | anim coll creature death fxdraw herostate hit hud lsound math moby particles shadow trig |
| U552 | 1432 | 18 | 13/13 | L18:2f7ab0 | 1/10 | cheap | — | — |
| U116 | 868, 905, 928 | 03 | 12/12 | L03:294c08 | 2/356 | cheap | — | math platform sound |
| U192 | 1139 | 05 16 | 12/12 | L05:31abe0, L16:2e0f78 | 1/66 | missing | missing: ptype | math moby |
| U244 | 1110 | 07 | 12/12 | L07:319040 | 1/406 | missing | missing: creature,shadow | anim coll creature death group hit math moby shadow |
| U459 | 93 | 15 | 12/12 | L15:2a3ba8 | 1/200 | cheap | — | math sound target trig |
| U201 | 1021 | 06 | 11/11 | L06:2f4f00 | 1/280 | cheap | — | creature math moby sound |
| U314 | 939 | 10 | 11/11 | L10:2d8c00 | 2/146 | cheap | — | math moby sound |
| U375 | 336 | 12 | 11/11 | L12:2ebea8 | 3/394 | cheap | — | creature death hit hud lsound math moby |
| U420 | 8 | 14 | 11/11 | L14:2ac618 | 15/2198 | missing | missing: creature | anim coll creature death fxdraw hit math moby particles path shadow sound target |
| U426 | 250 | 14 15 | 11/11 | L14:2d96e0, L15:2c6f08 | 1/100 | cheap | — | anim math |
| U461 | 148, 255 | 15 | 11/11 | L15:2a7880 | 1/496 | unknown | untagged ×1 | creature death fxmoby math moby particles |
| U148 | 563 | 04 | 10/17 | L04:2d16b8 | 7/2424 | missing | missing: cheat,creature; partly: anim | anim coll creature death hit math moby shadow |
| U271 | 468, 469 | 08 | 10/10 | L08:2ea4c0 | 1/200 | cheap | — | math sound |
| U295 | 1172 | 09 | 10/10 | L09:303d10 | 2/294 | cheap | — | creature death hit math moby sound |
| U30 | 27 | 01 | 10/10 | L01:2bd100 | 2/352 | cheap | — | math moby particles |
| U309 | 351, 1301 | 10 | 10/10 | L10:2be858 | 1/484 | missing | missing: camera | cine math particles sound |
| U463 | 196, 197, 1958 | 15 17 | 10/10 | L15:2bddb0, L17:2c0f00 | 1/192 | cheap | — | math moby sound trig |
| U497 | 1143 | 16 | 10/10 | L16:2e1088 | 1/54 | cheap | — | anim math |
| U108 | 1212 | 02 09 | 9/9 | L02:2ec4b8, L09:305dc0 | 3/542 | missing | missing: path | anim creature fxdraw group hit hud lsound math moby particles sound |
| U208 | 1054 | 06 | 9/9 | L06:2fbfb0 | 2/444 | cheap | — | math moby sound trig |
| U23 | 1440 | 00 | 9/9 | L00:2e0b88 | 12/1972 | missing | missing: ptype | anim breakfx coll creature death fxdraw hit light math moby shadow |
| U298 | 1206 | 09 | 9/9 | L09:305a28 | 1/122 | cheap | — | coll math sound |
| U316 | 1015, 1282 | 10 | 9/9 | L10:2d90a8 | 2/174 | cheap | — | group math moby |
| U376 | 339 | 12 | 9/9 | L12:2ec1d0 | 1/170 | cheap | — | anim math |
| U393 | 29 | 13 | 9/9 | L13:2b41b8 | 5/630 | missing | missing: cheat | anim creature death math moby trig |
| U48 | 695 | 01 | 9/9 | L01:2f8268 | 2/218 | missing | missing: creature | math water |
| U152 | 1101, 1102, 1531, 1532 | 04 | 8/8 | L04:2e17d8 | 2/170 | cheap | — | math sound |
| U170 | 823 | 05 07 | 8/8 | L05:30c0a8, L07:304738 | 1/94 | cheap | — | anim fxdraw math moby |
| U197 | 367 | 06 | 8/8 | L06:2d9d10 | 1/62 | cheap | — | math |
| U238 | 1041 | 07 | 8/8 | L07:30d4d8 | 2/516 | missing | missing: creature,fxmoby | anim creature death hit math moby shadow sound |
| U242 | 1080 | 07 | 8/8 | L07:311bc8 | 2/208 | cheap | — | math moby platform sound |
| U260 | 333 | 08 | 8/8 | L08:2dabf0 | 7/1612 | missing | missing: cheat,creature | anim creature death hit math moby particles shadow |
| U304 | 1885 | 09 13 | 8/9 | L09:30ab80, L13:30d550 | 7/686 | missing | missing: fxmoby | anim creature death group hit math moby |
| U369 | 240 | 12 | 8/8 | L12:2e3ed8 | 6/1108 | cheap | — | creature group math moby particles platform |
| U373 | 326 | 12 | 8/8 | L12:2e9f68 | 8/1640 | missing | missing: cheat,creature; partly: help | anim creature death hit math moby particles shadow sound trig |
| U397 | 111 | 13 | 8/8 | L13:2c4428 | 9/1632 | cheap | — | creature hit math moby path sound |
| U418 | 1805 | 13 | 8/8 | L13:30cdb8 | 2/256 | missing | missing: fxmoby | creature death group hit hud lsound math moby sound |
| U450 | 1417 | 14 | 8/8 | L14:306ee0 | 4/742 | cheap | — | anim creature hit math moby path sound trig |
| U455 | 67 | 15 | 8/8 | L15:29aff0 | 1/296 | cheap | — | math sound trig |
| U484 | 470 | 16 | 8/8 | L16:2c9480 | 3/254 | cheap | — | creature math particles |
| U486 | 482 | 16 | 8/8 | L16:2cb600 | 1/8 | cheap | — | moby |
| U522 | 347 | 17 | 8/8 | L17:2cb310 | 6/1138 | partly | partly: anim | anim coll creature hit math moby particles sound |
| U548 | 1381 | 18 | 8/8 | L18:2f16f0 | 4/546 | missing | missing: platform | camera math moby |
| U179 | 855 | 05 | 7/7 | L05:3150f0 | 2/458 | cheap | — | group math particles sound |
| U322 | 1196 | 10 | 7/7 | L10:2df270 | 9/974 | missing | missing: cheat | anim coll creature death hit hud lsound math moby shadow |
| U357 | 1231 | 11 | 7/7 | L11:310180 | 7/802 | missing | missing: cheat; partly: light | anim creature death hit math moby particles shadow |
| U457 | 78 | 15 17 | 7/7 | L15:2a2bf0, L17:2a82c0 | 3/724 | partly | partly: death | fxdraw math particles sound |
| U462 | 154 | 15 | 7/7 | L15:2aa280 | 1/434 | unknown | untagged ×1 | creature death fxmoby math moby particles |
| U474 | 1405, 1406, 1407, 1408, 1409, 1565, 1567 | 15 17 | 7/7 | L15:2eb0a0, L15:2eb458, L15:2edf58 +4 | 1/26 | partly | partly: fxdraw | fxdraw math moby |
| U518 | 1907, 1908, 1910, 1911, 1913, 1918, 1952 | 16 17 | 7/7 | L16:2e9df0, L17:2f57a0, L17:2f5ab8 +4 | 1/20 | partly | partly: fxdraw | fxdraw math |
| U90 | 668 | 02 | 7/7 | L02:2dcb38 | 4/568 | partly | partly: light | anim creature death hit math moby shadow |
| U119 | 455 | 03 08 14 | 6/7 | L03:2bef68, L08:2e6530, L14:2e1b20 | 3/872 | missing | missing: fxmoby | anim creature death hit math moby shadow trig |
| U180 | 877 | 05 | 6/6 | L05:3156d0 | 3/616 | missing | missing: camera | cine math platform sound |
| U211 | 1066 | 06 | 6/6 | L06:2fda30 | 1/104 | cheap | — | anim math trig |
| U236 | 1013, 1014, 1064, 1065 | 07 | 6/6 | L07:30cf90 | 1/292 | cheap | — | math sound |
| U247 | 1126 | 07 | 6/6 | L07:31a250 | 2/578 | cheap | — | anim creature death hit math moby sound |
| U292 | 664 | 09 | 6/6 | L09:2f86a0 | 1/310 | cheap | — | math platform sound |
| U301 | 1293, 1320 | 09 | 6/6 | L09:3091b0 | 1/414 | cheap | — | math platform sound |
| U312 | 794 | 10 | 6/6 | L10:2c9c70 | 3/468 | cheap | — | coll hit math moby |
| U319 | 1067 | 10 | 6/6 | L10:2da2c8 | 1/242 | cheap | — | fxdraw math moby sound |
| U331 | 1378 | 10 17 | 6/6 | L10:2e9028, L17:2e9608 | 1/100 | cheap | — | fxdraw math |
| U360 | 1248 | 11 | 6/6 | L11:316320 | 1/362 | cheap | — | math moby sound trig |
| U396 | 101 | 13 | 6/6 | L13:2c25b8 | 4/946 | cheap | — | anim coll creature death hit math moby particles sound |
| U428 | 386 | 14 | 6/6 | L14:2dee28 | 4/442 | cheap | — | fxdraw hit math moby sound trig |
| U448 | 1397 | 14 | 6/6 | L14:3061d8 | 1/22 | cheap | — | — |
| U476 | 1425 | 15 | 6/6 | L15:2eb928 | 1/46 | cheap | — | math particles |
| U488 | 546 | 16 | 6/6 | L16:2cf198 | 2/208 | cheap | — | hit math sound |
| U524 | 835 | 17 | 6/6 | L17:2dc3f8 | 1/242 | cheap | — | coll creature hit math moby particles water |
| U111 | 1479 | 02 | 5/5 | L02:2ef020 | 1/630 | cheap | — | math moby particles |
| U117 | 1210 | 03 | 5/5 | L03:2953f8 | 1/66 | cheap | — | anim math platform |
| U140 | 217 | 04 | 5/5 | L04:2ba520 | 6/1888 | missing | missing: cheat,creature | anim camera creature death hit math moby shadow sound trig |
| U159 | 35 | 05 | 5/5 | L05:2d1688 | 3/724 | missing | missing: water | anim cine creature herostate math moby particles sound |
| U190 | 998 | 05 | 5/5 | L05:318c78 | 9/852 | missing | missing: path; partly: help | cine math platform sound |
| U203 | 1035 | 06 | 5/5 | L06:2f6470 | 2/762 | partly | partly: help; untagged ×1 | coll fxdraw hit math moby particles sound |
| U249 | 104, 106, 1129 | 07 13 | 5/5 | L07:31aee0, L13:2c32d0 | 1/324 | cheap | — | math platform sound |
| U291 | 276, 1298, 1299 | 09 | 5/5 | L09:2edc30 | 2/364 | unknown | untagged ×1 | creature death hit math moby sound |
| U294 | 1150, 1151 | 09 | 5/5 | L09:3033a0 | 2/506 | partly | partly: herostate | math platform sound trig |
| U321 | 1100 | 10 | 5/5 | L10:2dd650 | 1/126 | cheap | — | math moby |
| U325 | 1229 | 10 | 5/5 | L10:2e4a88 | 4/1366 | missing | missing: cheat; partly: help | anim coll creature death hit math moby particles shadow trig |
| U327 | 1302 | 10 | 5/5 | L10:2e7cd8 | 4/990 | cheap | — | fxdraw math mission particles sound |
| U343 | 318 | 11 | 5/5 | L11:2f2518 | 5/742 | missing | missing: herostate; partly: help | cine fxdraw hit math moby music sound |
| U399 | 170 | 13 | 5/5 | L13:2cefc0 | 5/982 | missing | missing: ptype | anim creature death fxmoby hit math moby particles sound |
| U404 | 404, 405 | 13 | 5/5 | L13:2ed4a8 | 3/370 | cheap | — | anim coll hit light math moby |
| U431 | 643 | 14 | 5/5 | L14:2ec810 | 1/312 | cheap | — | creature hit math moby path sound |
| U437 | 908 | 14 | 5/5 | L14:2fc0f0 | 4/686 | cheap | — | creature hit hud lsound math moby particles path sound |
| U490 | 556 | 16 | 5/5 | L16:2d04a0 | 8/1540 | missing | missing: cheat,path; partly: anim,breakfx | anim coll creature group hit math moby particles path |
| U76 | 1504 | 01 06 | 5/5 | L01:30b618, L06:308868 | 1/256 | cheap | — | light math moby trig |
| U129 | 899 | 03 | 4/4 | L03:2db280 | 2/186 | cheap | — | math moby sound |
| U143 | 432, 1052 | 04 | 4/4 | L04:2c6858 | 1/214 | cheap | — | anim math moby sound |
| U146 | 481 | 04 | 4/4 | L04:2cdda0 | 1/176 | partly | partly: anim | anim math moby platform |
| U150 | 617 | 04 | 4/4 | L04:2d6f68 | 1/82 | cheap | — | math |
| U166 | 625 | 05 | 4/4 | L05:304320 | 4/1492 | missing | missing: creature,fxmoby | anim coll creature death hit math moby particles shadow sound |
| U167 | 717 | 05 | 4/4 | L05:307910 | 10/1446 | missing | missing: cheat,path; partly: anim | anim coll creature group hit math moby path shadow |
| U168 | 810 | 05 | 4/4 | L05:30bdd8 | 1/112 | cheap | — | math platform sound |
| U182 | 893 | 05 | 4/4 | L05:316258 | 1/274 | cheap | — | anim creature math sound |
| U183 | 895 | 05 | 4/4 | L05:3166a0 | 1/92 | cheap | — | creature math sound |
| U199 | 857 | 06 | 4/4 | L06:2f0040 | 22/4370 | missing | missing: creature,path | anim coll creature fxdraw group hit math moby particles shadow sound target trig |
| U205 | 1039 | 06 08 13 18 | 4/4 | L06:2f7930, L08:302a78, L13:3036f0 +1 | 1/98 | partly | partly: herostate | cine herostate math trig |
| U219 | 1123 | 06 | 4/4 | L06:3049f8 | 2/478 | unknown | untagged ×1 | coll fxdraw math particles sound |
| U233 | 871 | 07 | 4/4 | L07:30b000 | 2/382 | missing | missing: shadow | creature group hit math moby path target |
| U246 | 1113, 1114, 1115, 1116 | 07 | 4/4 | L07:319f48 | 2/316 | cheap | — | creature hit math moby |
| U258 | 253 | 08 | 4/4 | L08:2d42d8 | 8/2306 | partly | partly: help | anim creature death hit hud lsound math moby particles platform sound |
| U270 | 467, 472 | 08 | 4/4 | L08:2ea398 | 1/74 | cheap | — | math sound |
| U317 | 1031 | 10 | 4/4 | L10:2d92b8 | 1/166 | cheap | — | coll creature math moby sound |
| U326 | 1240 | 10 | 4/4 | L10:2e5be0 | 1/184 | cheap | — | math particles |
| U354 | 1178 | 11 | 4/4 | L11:30eb90 | 1/156 | partly | partly: herostate | math platform |
| U361 | 1264 | 11 | 4/4 | L11:3172c0 | 6/998 | cheap | — | creature hit math moby sound |
| U38 | 613 | 01 | 4/4 | L01:2f3120 | 1/274 | cheap | — | math path |
| U384 | 1281 | 12 | 4/4 | L12:307840 | 3/310 | missing | missing: creature,ptype | anim math moby target |
| U391 | 6 | 13 | 4/4 | L13:2b0e80 | 1/264 | cheap | — | group moby trig |
| U392 | 21, 244 | 13 | 4/4 | L13:2b39e0 | 2/502 | missing | missing: camera; partly: herostate | cine math platform sound |
| U402 | 231 | 13 | 4/4 | L13:2e4448 | 2/396 | cheap | — | creature group hit math moby sound |
| U408 | 674, 677, 680 | 13 | 4/4 | L13:2f8a78 | 1/192 | cheap | — | anim moby |
| U417 | 1577 | 13 | 4/4 | L13:30be60 | 1/84 | cheap | — | moby |
| U424 | 31 | 14 | 4/4 | L14:2b46e8 | 14/1784 | cheap | — | anim creature death fxmoby hit math moby particles path shadow sound |
| U427 | 309 | 14 | 4/4 | L14:2de1f8 | 2/286 | cheap | — | math platform sound |
| U429 | 557 | 14 | 4/4 | L14:2e7bd8 | 14/2084 | cheap | — | anim creature group hit math moby particles platform sound trig |
| U430 | 610 | 14 | 4/4 | L14:2eaf88 | 4/682 | cheap | — | fxdraw math moby trig |
| U458 | 92 | 15 | 4/4 | L15:2a36d0 | 1/310 | cheap | — | sound trig |
| U494 | 654 | 16 | 4/4 | L16:2d5ef8 | 6/474 | missing | missing: camera | cine hit math moby sound trig |
| U500 | 1387 | 16 | 4/4 | L16:2e36e8 | 1/34 | cheap | — | math moby |
| U506 | 1443, 1890 | 16 18 | 4/4 | L16:2e5708, L18:2fae48 | 3/194 | cheap | — | fxdraw math sound |
| U528 | 1380 | 17 | 4/4 | L17:2ee2b0 | 3/558 | missing | missing: camera; partly: herostate | cine math moby platform sound |
| U541 | 583 | 18 | 4/4 | L18:2d6600 | 1/342 | cheap | — | creature math moby sound |
| U93 | 707, 734 | 02 | 4/4 | L02:2ddc00, L02:2df8c0 | 1/34 | cheap | — | math platform |
| U122 | 578 | 03 | 3/5 | L03:2c9eb8 | 3/922 | missing | missing: cheat,creature | anim creature death hit math moby shadow sound |
| U125 | 816 | 03 | 3/3 | L03:2d3198 | 5/566 | missing | missing: herostate,ptype; partly: help; untagged ×1 | cine creature math moby platform trig |
| U139 | 86 | 04 | 3/3 | L04:29ecf8 | 8/1436 | cheap | — | coll math moby |
| U144 | 434 | 04 | 3/3 | L04:2c6bb8 | 1/258 | cheap | — | anim math moby sound |
| U147 | 484 | 04 | 3/3 | L04:2ce060 | 1/8 | cheap | — | moby |
| U149 | 584 | 04 | 3/3 | L04:2d3580 | 1/42 | cheap | — | math platform |
| U176 | 844 | 05 | 3/3 | L05:30e7f8 | 5/1036 | cheap | — | coll creature hit math moby particles platform sound |
| U177 | 846 | 05 | 3/3 | L05:30f5c8 | 3/406 | cheap | — | hit math moby particles sound |
| U186 | 447, 920 | 05 07 13 | 3/3 | L05:317aa0, L07:2f61c0, L13:2edbe8 | 2/388 | partly | partly: anim; untagged ×1 | anim cine math moby shadow trig |
| U231 | 529 | 07 | 3/3 | L07:2fbdb8 | 1/14 | cheap | — | fxdraw |
| U267 | 452 | 08 | 3/3 | L08:2e2df0 | 6/2680 | missing | missing: cheat; partly: platform; untagged ×3 | anim camera coll creature death fxdraw hit hud math moby particles shadow sound |
| U275 | 671 | 08 | 3/3 | L08:2f70a0 | 4/1558 | cheap | — | fxdraw light math moby particles |
| U297 | 1201 | 09 | 3/3 | L09:304c80 | 4/1078 | missing | missing: camera; partly: herostate | anim cine light math moby particles sound target |
| U299 | 1285, 1286, 1287, 1288 | 09 | 3/3 | L09:3082d8 | 2/380 | unknown | untagged ×1 | creature hit math moby sound |
| U311 | 702 | 10 | 3/3 | L10:2c7a20 | 1/120 | missing | missing: ptype | math sound |
| U353 | 1159 | 11 | 3/3 | L11:30e978 | 1/134 | cheap | — | math moby sound |
| U355 | 1179 | 11 15 | 3/3 | L11:30ee00, L15:2d9100 | 1/284 | partly | partly: help | math sound |
| U377 | 384 | 12 | 3/3 | L12:2ec720 | 1/232 | cheap | — | fxdraw math particles |
| U385 | 1345 | 12 | 3/3 | L12:3081b0 | 2/304 | cheap | — | fxdraw math mission moby sound |
| U425 | 211 | 14 | 3/3 | L14:2d5f40 | 6/1204 | missing | missing: ptype | anim coll creature fxdraw light math moby path trig |
| U503 | 1439 | 16 | 3/3 | L16:2e5010 | 1/146 | cheap | — | math sound |
| U504 | 1441 | 16 | 3/3 | L16:2e5258 | 1/146 | cheap | — | math sound |
| U505 | 1442 | 16 | 3/3 | L16:2e54a0 | 2/154 | cheap | — | hit math sound |
| U525 | 1021 | 17 18 | 3/3 | L17:2e2c58, L18:2ec3e8 | 1/284 | cheap | — | creature math moby sound |
| U550 | 1402 | 18 | 3/3 | L18:2f2310 | 2/196 | cheap | — | fxdraw math |
| U553 | 1434, 1435 | 18 | 3/3 | L18:2f7ad8 | 1/90 | cheap | — | math |
| U554 | 1454 | 18 | 3/3 | L18:2f7c40 | 5/1050 | missing | missing: fxmoby | anim creature death hit math moby particles sound |
| U83 | 331 | 02 | 3/3 | L02:2cb978 | 1/12 | cheap | — | — |
| U87 | 651 | 02 | 3/3 | L02:2dbd38 | 2/606 | cheap | — | fxdraw math platform sound trig |
| U95 | 732 | 02 | 3/3 | L02:2df1a8 | 1/140 | cheap | — | anim math sound |
| U106 | 1005, 1016 | 02 06 | 2/2 | L02:2ea210, L06:2f4638 | 3/562 | missing | missing: camera,save; partly: help | cine creature dialog hud math mission moby particles |
| U114 | 822 | 03 | 2/2 | L03:292e98 | 9/2558 | missing | missing: path | anim cine creature group hit math moby music particles path platform sound trig |
| U135 | 1012 | 03 | 2/2 | L03:2dccc0 | 5/406 | missing | missing: herostate; partly: help; untagged ×1 | cine hud math platform trig |
| U164 | 439 | 05 16 | 2/2 | L05:2f87a8, L16:2c7788 | 1/230 | partly | partly: help,herostate | anim math particles |
| U196 | 55 | 06 | 2/2 | L06:2b4770 | 1/168 | cheap | — | creature math |
| U213 | 1083 | 06 | 2/2 | L06:2ffcb0 | 7/952 | missing | missing: camera,fxmoby; partly: herostate; untagged ×1 | cine fxdraw math moby sound |
| U262 | 424 | 08 | 2/2 | L08:2dbdb0 | 7/874 | missing | missing: path; partly: help | anim math platform shadow |
| U263 | 435 | 08 | 2/2 | L08:2dd3c0 | 2/1232 | cheap | — | creature hit math moby particles sound water |
| U268 | 462 | 08 | 2/2 | L08:2e8cd0 | 4/884 | unknown | untagged ×2 | anim creature hit math moby sound |
| U283 | 1629 | 08 14 | 2/2 | L08:3085f0, L14:3088b0 | 1/136 | unknown | untagged ×1 | math |
| U284 | 1641 | 08 | 2/2 | L08:308c90 | 2/132 | cheap | — | math particles |
| U306 | 1117 | 10 | 2/2 | L10:295c20 | 2/216 | cheap | — | math platform sound |
| U315 | 947 | 10 | 2/2 | L10:2d8d30 | 2/270 | missing | missing: path | light math moby particles sound |
| U333 | 1424 | 10 | 2/2 | L10:2e9990 | 1/290 | unknown | untagged ×1 | math |
| U348 | 1075 | 11 | 2/2 | L11:309ac0 | 3/874 | unknown | untagged ×1 | anim camera creature math particles platform sound |
| U386 | 1400 | 12 14 | 2/2 | L12:308be0, L14:306390 | 5/830 | missing | missing: heightmap,ptype; untagged ×1 | math particles water |
| U387 | 1404 | 12 | 2/2 | L12:3093c8 | 1/54 | cheap | — | dialog moby trig |
| U419 | 903 | 14 | 2/2 | L14:2aba80 | 3/616 | cheap | — | math platform sound trig |
| U440 | 923 | 14 | 2/2 | L14:2fe2a0 | 7/862 | cheap | — | anim creature death hit math moby particles path trig |
| U444 | 1331 | 14 15 | 2/2 | L14:3039e0, L15:2e8910 | 6/1510 | missing | missing: ptype | fxdraw light math |
| U449 | 1416 | 14 | 2/2 | L14:306b78 | 1/218 | cheap | — | math sound |
| U468 | 655 | 15 17 | 2/2 | L15:2d6810, L17:2d4e88 | 9/2552 | cheap | — | coll creature fxdraw herostate hit light math moby particles sound trig |
| U473 | 1394 | 15 | 2/2 | L15:2eabd8 | 2/82 | cheap | — | math moby |
| U479 | 1451, 1899 | 15 18 | 2/2 | L15:2ed068, L18:2fb868 | 1/204 | missing | missing: herostate; partly: help; untagged ×1 | anim cine creature dialog moby music |
| U502 | 1410 | 16 | 2/2 | L16:2e43e0 | 10/798 | missing | missing: path; partly: help | cine math platform sound |
| U512 | 1826 | 16 | 2/2 | L16:2e7f80 | 1/346 | cheap | — | fxdraw math platform sound |
| U513 | 1891 | 16 | 2/2 | L16:2e88d0 | 1/40 | cheap | — | math |
| U530 | 1426 | 17 | 2/2 | L17:2f1418 | 1/28 | cheap | — | fxdraw moby |
| U535 | 1772 | 17 | 2/2 | L17:2f3848 | 1/200 | cheap | — | fxdraw math trig |
| U542 | 586 | 18 | 2/2 | L18:2d7b20 | 1/274 | partly | partly: help | fxdraw math sound |
| U549 | 1392 | 18 | 2/2 | L18:2f1e68 | 2/298 | cheap | — | fxdraw math sound |
| U89 | 656 | 02 | 2/2 | L02:2dca10 | 1/74 | cheap | — | anim math |
| U97 | 735, 736 | 02 | 2/2 | L02:2df948 | 1/164 | cheap | — | creature math |
| U101 | 762 | 02 | 1/1 | L02:2e0720 | 2/370 | cheap | — | hit math moby particles sound |
| U102 | 786 | 02 | 1/1 | L02:2e0dc0 | 3/778 | missing | missing: camera,save; partly: anim,help; untagged ×1 | anim cine creature herostate lsound math mission moby trig |
| U103 | 788 | 02 | 1/1 | L02:2e1950 | 4/870 | missing | missing: save; partly: anim,help; untagged ×2 | anim cine creature hud lsound math mission moby trig |
| U104 | 792 | 02 | 1/1 | L02:2e2228 | 1/310 | cheap | — | anim math platform sound |
| U105 | 854 | 02 | 1/1 | L02:2ea198 | 5/96 | cheap | — | — |
| U110 | 1324 | 02 | 1/1 | L02:2ee890 | 1/484 | partly | partly: help | hud lsound math trig |
| U115 | 845 | 03 | 1/1 | L03:293c78 | 3/1144 | missing | missing: path | anim creature group hit math moby particles path sound |
| U123 | 631 | 03 | 1/1 | L03:2cca00 | 3/1734 | missing | missing: cheat,creature; partly: anim | anim creature death hit math moby path shadow sound trig |
| U127 | 825 | 03 | 1/1 | L03:2d3e58 | 1/32 | cheap | — | math platform |
| U128 | 890 | 03 | 1/1 | L03:2da870 | 3/652 | missing | missing: save; partly: anim; untagged ×1 | anim cine hud lsound math mission moby |
| U13 | 530 | 00 | 1/1 | L00:2d1e80 | 1/44 | cheap | — | anim fxdraw |
| U130 | 909 | 03 | 1/1 | L03:2db558 | 2/580 | missing | missing: save; partly: anim; untagged ×1 | anim cine hud lsound math mission |
| U131 | 914 | 03 | 1/1 | L03:2dbd90 | 1/352 | cheap | — | anim cine creature hit hud lsound math particles shadow |
| U134 | 997 | 03 | 1/1 | L03:2dca30 | 1/64 | cheap | — | — |
| U136 | 1342 | 03 | 1/1 | L03:2df520 | 1/812 | missing | missing: gadget; partly: help | hud lsound math trig |
| U137 | 1548 | 03 | 1/1 | L03:2e01d0 | 2/312 | cheap | — | anim math particles |
| U151 | 642 | 04 | 1/1 | L04:2d7e90 | 1/394 | cheap | — | coll math |
| U153 | 1120 | 04 | 1/1 | L04:2e1a10 | 3/330 | missing | missing: save | dialog hud lsound math mission moby particles |
| U154 | 1190 | 04 | 1/1 | L04:2e3078 | 4/474 | missing | missing: save | anim cine dialog hud math mission shadow trig |
| U155 | 1343 | 04 | 1/1 | L04:2e4418 | 1/754 | partly | partly: help | math trig |
| U156 | 1549 | 04 | 1/1 | L04:2e53a0 | 2/220 | cheap | — | anim math particles |
| U174 | 841 | 05 | 1/1 | L05:30e1f8 | 1/196 | cheap | — | math trig |
| U184 | 918 | 05 | 1/1 | L05:316ab8 | 3/738 | missing | missing: save; partly: anim; untagged ×1 | anim cine creature hud math music |
| U185 | 919 | 05 | 1/1 | L05:317470 | 2/388 | missing | missing: save; partly: anim; untagged ×1 | anim cine math mission moby |
| U187 | 925 | 05 | 1/1 | L05:3180a0 | 2/420 | missing | missing: save; partly: anim | anim cine creature hud math mission moby shadow |
| U189 | 984 | 05 | 1/1 | L05:318b98 | 1/56 | cheap | — | math |
| U191 | 1099 | 05 | 1/1 | L05:319c28 | 1/14 | cheap | — | — |
| U193 | 1347 | 05 | 1/1 | L05:31bf40 | 2/702 | partly | partly: help | math sound trig |
| U195 | 1550 | 05 | 1/1 | L05:31d5c8 | 2/214 | cheap | — | anim math particles |
| U20 | 834 | 00 | 1/1 | L00:2d9dc8 | 3/386 | missing | missing: levelexit | cine dialog hud math trig |
| U202 | 1028 | 06 | 1/1 | L06:2f55a0 | 2/948 | missing | missing: camera,save; partly: help,herostate | cine hud math mission moby sound |
| U207 | 1051 | 06 | 1/1 | L06:2f9a28 | 21/3324 | missing | missing: camera,cheat,path; partly: breakfx,herostate; untagged ×5 | anim camera cine coll creature death group hit hud lsound math moby shadow trig |
| U209 | 1061 | 06 | 1/1 | L06:2fc640 | 2/722 | missing | missing: herostate; partly: help; untagged ×3 | cine dialog math moby music platform sound trig |
| U210 | 1062 | 06 | 1/1 | L06:2fd088 | 2/246 | cheap | — | fxdraw math trig |
| U215 | 1105 | 06 | 1/1 | L06:301070 | 6/1224 | missing | missing: camera,save; partly: help; untagged ×3 | anim cine creature dialog hud math mission moby shadow trig |
| U216 | 1108 | 06 | 1/1 | L06:301b90 | 11/690 | missing | missing: camera; partly: herostate; untagged ×4 | cine dialog fxdraw group herostate math moby music sound |
| U217 | 1109 | 06 | 1/1 | L06:302578 | 9/1794 | missing | missing: camera,levelexit,save; partly: help,herostate; untagged ×3 | anim cine creature dialog hud lsound math mission moby music particles platform sound trig |
| U218 | 1118 | 06 | 1/1 | L06:304098 | 3/580 | missing | missing: camera,levelexit; partly: help; untagged ×2 | cine math moby particles sound |
| U22 | 1413 | 00 | 1/1 | L00:2e0988 | 1/128 | partly | partly: help | trig |
| U220 | 1302 | 06 | 1/1 | L06:307d30 | 5/990 | cheap | — | fxdraw math mission particles sound |
| U221 | 1348 | 06 | 1/1 | L06:3083b0 | 1/302 | partly | partly: help | group math trig |
| U223 | 1551 | 06 | 1/1 | L06:309348 | 2/218 | cheap | — | anim math particles |
| U227 | 38 | 07 | 1/1 | L07:2cd2b0 | 2/390 | partly | partly: herostate | math platform sound |
| U229 | 436 | 07 | 1/1 | L07:2f5ba0 | 2/406 | missing | missing: levelexit,save; untagged ×1 | cine dialog math mission moby music trig |
| U24 | 1471 | 00 | 1/1 | L00:2e1df0 | 6/1504 | missing | missing: ptype | fxdraw light math |
| U243 | 1106 | 07 | 1/1 | L07:314150 | 32/7704 | missing | missing: cheat,creature,ptype,shadow; partly: anim,herostate; untagged ×4 | anim camera coll creature death fxdraw hit hud light lsound math mission moby particles path pickup shadow sound trig |
| U248 | 1128 | 07 | 1/1 | L07:31a9e8 | 1/318 | cheap | — | math sound |
| U25 | 1545 | 00 | 1/1 | L00:2e3800 | 3/458 | cheap | — | anim math particles |
| U250 | 1133 | 07 | 1/1 | L07:31b3f0 | 1/128 | cheap | — | math sound |
| U251 | 1142 | 07 | 1/1 | L07:31d2e0 | 1/300 | cheap | — | math moby path pickup trig |
| U252 | 1552 | 07 | 1/1 | L07:31eba8 | 2/322 | cheap | — | anim math particles |
| U256 | 1789 | 07 | 1/1 | L07:31f900 | 2/236 | cheap | — | anim math particles |
| U265 | 440 | 08 | 1/1 | L08:2e0328 | 5/1606 | partly | partly: herostate | anim cine fxdraw herostate math moby particles sound |
| U266 | 444 | 08 | 1/1 | L08:2e24e8 | 4/1036 | unknown | untagged ×2 | anim creature hit math moby sound |
| U269 | 463 | 08 | 1/1 | L08:2e9b70 | 4/912 | unknown | untagged ×2 | anim creature hit math moby sound |
| U272 | 600 | 08 | 1/1 | L08:2f1548 | 10/862 | partly | partly: fxdraw; untagged ×1 | fxdraw math moby |
| U277 | 1130 | 08 | 1/1 | L08:302ce8 | 2/476 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby shadow |
| U278 | 1144 | 08 | 1/1 | L08:305270 | 2/460 | missing | missing: save; partly: anim | anim cine math mission moby shadow |
| U279 | 1283 | 08 | 1/1 | L08:3065d8 | 4/512 | missing | missing: save; partly: anim,herostate | anim cine herostate lsound math mission moby |
| U280 | 1349 | 08 | 1/1 | L08:307540 | 1/404 | partly | partly: help | hud lsound math trig |
| U281 | 1400 | 08 | 1/1 | L08:307cf0 | 5/702 | missing | missing: heightmap,ptype; untagged ×1 | math particles water |
| U282 | 1553 | 08 | 1/1 | L08:3084d8 | 2/250 | cheap | — | anim math particles |
| U285 | 1649 | 08 | 1/1 | L08:3097b0 | 10/856 | partly | partly: fxdraw; untagged ×1 | fxdraw math moby |
| U290 | 263, 264, 265, 266, 267 | 09 | 1/1 | L09:2eb970 | 8/1882 | cheap | — | coll creature math moby particles sound |
| U293 | 1000 | 09 | 1/1 | L09:300888 | 1/82 | partly | partly: help | hud lsound math trig |
| U300 | 1290 | 09 | 1/1 | L09:308818 | 3/378 | missing | missing: save; partly: help | dialog hud math mission moby particles |
| U302 | 1766 | 09 | 1/1 | L09:309c08 | 3/668 | cheap | — | anim math particles platform sound trig |
| U305 | 1033 | 10 | 1/1 | L10:295a38 | 1/100 | cheap | — | math moby sound |
| U307 | 18 | 10 | 1/1 | L10:298668 | 3/320 | missing | missing: save | dialog hud lsound math mission moby particles |
| U308 | 22 | 10 | 1/1 | L10:298b68 | 2/330 | partly | partly: help; untagged ×2 | cine math moby |
| U310 | 353 | 10 | 1/1 | L10:2befe8 | 1/102 | cheap | — | — |
| U318 | 1047 | 10 | 1/1 | L10:2d9eb8 | 4/454 | cheap | — | creature hit math moby sound |
| U320 | 1073 | 10 | 1/1 | L10:2dcc58 | 1/238 | cheap | — | math moby |
| U328 | 1326 | 10 | 1/1 | L10:2e8358 | 1/152 | missing | missing: save; partly: help; untagged ×1 | dialog hud math |
| U329 | 1344 | 10 | 1/1 | L10:2e85b8 | 1/318 | partly | partly: help | math trig |
| U330 | 1346 | 10 | 1/1 | L10:2e8ab0 | 2/462 | cheap | — | creature math moby particles |
| U332 | 1421 | 10 | 1/1 | L10:2e9648 | 1/210 | unknown | untagged ×1 | math trig |
| U335 | 1555 | 10 | 1/1 | L10:2eacd8 | 2/232 | cheap | — | anim math particles |
| U338 | 23 | 11 | 1/1 | L11:2cb668 | 1/74 | missing | missing: save; partly: help | math moby |
| U339 | 65 | 11 | 1/1 | L11:2cb810 | 1/96 | cheap | — | creature math moby |
| U340 | 90 | 11 | 1/1 | L11:2d0710 | 2/362 | missing | missing: save; partly: anim,help | anim cine lsound math mission trig |
| U341 | 114 | 11 | 1/1 | L11:2d0fa8 | 17/2084 | missing | missing: save; partly: anim,help,platform; untagged ×1 | anim cine coll creature math mission moby shadow trig |
| U342 | 298 | 11 | 1/1 | L11:2f0e40 | 2/498 | missing | missing: save; partly: anim,help | anim cine hud lsound math mission moby shadow |
| U344 | 361 | 11 | 1/1 | L11:2f3350 | 1/40 | cheap | — | moby |
| U350 | 1156 | 11 | 1/1 | L11:30c788 | 2/1054 | missing | missing: camera; untagged ×1 | cine creature math moby sound |
| U351 | 1157 | 11 | 1/1 | L11:30d800 | 2/542 | missing | missing: camera | cine math moby sound |
| U356 | 1180 | 11 | 1/1 | L11:30f2a8 | 1/92 | unknown | untagged ×1 | creature math sound |
| U358 | 1242 | 11 | 1/1 | L11:313290 | 21/4018 | missing | missing: camera,ptype; partly: help,herostate; untagged ×4 | anim cine coll creature fxdraw herostate hit hud math mission moby music sound target |
| U363 | 1350 | 11 | 1/1 | L11:31aa80 | 2/68 | cheap | — | hud lsound moby |
| U365 | 1903 | 11 | 1/1 | L11:31e2f0 | 10/884 | partly | partly: fxdraw; untagged ×1 | fxdraw math moby |
| U366 | 1919 | 11 | 1/1 | L11:31f150 | 10/922 | partly | partly: fxdraw; untagged ×1 | fxdraw math moby trig |
| U370 | 282 | 12 | 1/1 | L12:2e6c48 | 2/404 | missing | missing: save; partly: anim; untagged ×1 | anim cine hud lsound math mission moby |
| U371 | 293 | 12 | 1/1 | L12:2e7208 | 5/388 | partly | partly: fxdraw | fxdraw moby trig |
| U374 | 328 | 12 | 1/1 | L12:2eb570 | 2/516 | missing | missing: save; partly: anim; untagged ×1 | anim cine hud lsound math mission moby |
| U378 | 422 | 12 | 1/1 | L12:2ed280 | 1/196 | partly | partly: help | group hud lsound math trig |
| U381 | 1267 | 12 | 1/1 | L12:303540 | 5/1242 | missing | missing: camera,save; partly: herostate | anim cine creature dialog fxdraw herostate hit math mission moby particles sound water |
| U383 | 1274 | 12 | 1/1 | L12:3069d0 | 7/2316 | missing | missing: camera; partly: anim | anim camera cine creature dialog hit math moby particles sound trig |
| U388 | 1557 | 12 | 1/1 | L12:3094a0 | 2/218 | cheap | — | anim math particles |
| U395 | 69 | 13 | 1/1 | L13:2bb068 | 12/3030 | missing | missing: camera,shadow; partly: help,herostate; untagged ×3 | anim cine coll creature fxdraw herostate hit hud lsound math mission moby music particles sound target trig |
| U403 | 388 | 13 | 1/1 | L13:2eb098 | 23/4776 | cheap | — | anim coll creature death dialog fxdraw hit hud light math moby particles path sound |
| U405 | 533 | 13 | 1/1 | L13:2f3750 | 1/10 | cheap | — | — |
| U406 | 558 | 13 | 1/1 | L13:2f3778 | 1/118 | partly | partly: help | math trig |
| U407 | 667 | 13 | 1/1 | L13:2f8880 | 1/126 | cheap | — | moby |
| U412 | 1270 | 13 | 1/1 | L13:30a6d8 | 2/484 | missing | missing: camera; partly: herostate | cine math platform sound |
| U413 | 1271 | 13 | 1/1 | L13:30af50 | 2/116 | missing | missing: react | trig |
| U414 | 1353 | 13 | 1/1 | L13:30b628 | 1/338 | missing | missing: save; untagged ×1 | cine dialog mission moby music trig |
| U415 | 1403 | 13 | 1/1 | L13:30bb70 | 2/276 | cheap | — | math moby path |
| U416 | 1558 | 13 | 1/1 | L13:30bdf0 | 2/208 | cheap | — | anim math particles |
| U43 | 676 | 01 | 1/1 | L01:2f6128 | 2/22 | cheap | — | fxdraw water |
| U432 | 684 | 14 | 1/1 | L14:2ed280 | 6/1482 | cheap | — | cine dialog fxdraw math moby particles sound |
| U433 | 685 | 14 | 1/1 | L14:2ee8d8 | 6/940 | unknown | untagged ×1 | math moby path platform sound trig |
| U435 | 712 | 14 | 1/1 | L14:2f0538 | 9/1144 | missing | missing: path; partly: help | camera cine math platform sound trig |
| U436 | 851 | 14 | 1/1 | L14:2fba20 | 5/582 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby shadow trig |
| U438 | 921 | 14 | 1/1 | L14:2fcbb0 | 7/1070 | unknown | untagged ×1 | coll creature hit hud lsound math moby particles path sound trig |
| U439 | 922 | 14 | 1/1 | L14:2fdbb8 | 2/412 | unknown | untagged ×1 | creature math moby particles sound |
| U44 | 678 | 01 | 1/1 | L01:2f6180 | 1/106 | cheap | — | fxdraw math |
| U441 | 924 | 14 | 1/1 | L14:2fefe0 | 2/466 | missing | missing: save; partly: anim | anim cine hud lsound math mission moby trig |
| U445 | 1352 | 14 | 1/1 | L14:305408 | 3/212 | unknown | untagged ×1 | math moby |
| U446 | 1354 | 14 | 1/1 | L14:305758 | 3/372 | missing | missing: save; partly: help | creature dialog hud math mission moby particles |
| U447 | 1395 | 14 | 1/1 | L14:305d28 | 2/298 | unknown | untagged ×1 | math moby sound |
| U451 | 1418 | 14 | 1/1 | L14:307a80 | 2/42 | cheap | — | — |
| U453 | 1559 | 14 | 1/1 | L14:3087c0 | 3/280 | cheap | — | anim hud lsound math particles |
| U472 | 1388 | 15 | 1/1 | L15:2ea748 | 3/292 | missing | missing: save | dialog hud math mission moby particles |
| U475 | 1419 | 15 | 1/1 | L15:2eb4c0 | 1/126 | missing | missing: save | cine dialog fxdraw herostate hud lsound mission trig |
| U477 | 1430 | 15 | 1/1 | L15:2ec030 | 6/544 | cheap | — | cine creature math moby |
| U478 | 1446 | 15 | 1/1 | L15:2ec760 | 5/926 | missing | missing: herostate,save; partly: anim | anim cine dialog group hud lsound math mission moby music trig |
| U480 | 1469 | 15 | 1/1 | L15:2ed398 | 1/482 | partly | partly: help | herostate math trig |
| U481 | 1560 | 15 | 1/1 | L15:2edb20 | 2/228 | cheap | — | anim math particles |
| U499 | 1377 | 16 | 1/1 | L16:2e3190 | 2/400 | missing | missing: save; partly: anim,help | anim cine lsound math mission trig |
| U508 | 1455 | 16 | 1/1 | L16:2e6808 | 8/834 | missing | missing: save; partly: anim,help | anim cine herostate hud lsound math mission music shadow |
| U509 | 1561 | 16 | 1/1 | L16:2e7208 | 1/54 | cheap | — | hud lsound trig |
| U511 | 1812 | 16 | 1/1 | L16:2e7e00 | 2/212 | cheap | — | anim math particles |
| U514 | 1923 | 16 | 1/1 | L16:2e8970 | 6/318 | missing | missing: fxmoby | anim coll creature math sound trig |
| U515 | 1943 | 16 | 1/1 | L16:2e8e80 | 1/146 | cheap | — | fxdraw math |
| U516 | 1947 | 16 | 1/1 | L16:2e93b0 | 1/146 | cheap | — | fxdraw math |
| U517 | 1948 | 16 | 1/1 | L16:2e98e0 | 1/150 | cheap | — | fxdraw math |
| U519 | 1953 | 16 | 1/1 | L16:2ea128 | 1/150 | cheap | — | fxdraw math |
| U527 | 1379 | 17 | 1/1 | L17:2ed018 | 12/2894 | missing | missing: camera,levelexit,shadow; partly: help,herostate; untagged ×3 | anim cine coll creature fxdraw herostate hit hud lsound math mission moby music particles sound target |
| U531 | 1428 | 17 | 1/1 | L17:2f1790 | 3/302 | missing | missing: save | dialog hud math moby particles |
| U532 | 1448 | 17 | 1/1 | L17:2f1940 | 4/880 | missing | missing: camera,levelexit; partly: help | anim cine creature math platform sound |
| U533 | 1470 | 17 | 1/1 | L17:2f26d0 | 1/490 | missing | missing: camera; partly: help,herostate | herostate hud lsound math trig |
| U534 | 1562 | 17 | 1/1 | L17:2f2e78 | 2/220 | cheap | — | anim math particles |
| U540 | 582 | 18 | 1/1 | L18:2d62e8 | 2/232 | cheap | — | math moby |
| U544 | 644 | 18 | 1/1 | L18:2df608 | 1/294 | missing | missing: camera; partly: help | cine math |
| U551 | 1422 | 18 | 1/1 | L18:2f2bf0 | 50/7136 | missing | missing: camera,cheat,cine,herostate,platform; partly: anim; untagged ×1 | anim camera cine creature dialog fxdraw herostate hit hud lsound math mission moby particles shadow sound target trig |
| U555 | 1563 | 18 | 1/1 | L18:2f88e8 | 4/1314 | unknown | untagged ×1 | anim creature fxdraw math particles |
| U557 | 1750 | 18 | 1/1 | L18:2fad08 | 2/248 | missing | missing: save | math |
| U558 | 1799 | 18 | 1/1 | L18:2fad28 | 1/72 | cheap | — | anim math particles |
| U63 | 761 | 01 | 1/1 | L01:2feb58 | 2/144 | cheap | — | fxdraw math water |
| U74 | 1225 | 01 | 1/1 | L01:309c98 | 1/14 | cheap | — | fxdraw |
| U75 | 1341 | 01 | 1/1 | L01:30acb8 | 1/600 | partly | partly: help; untagged ×1 | math moby trig |
| U82 | 1848 | 01 | 1/1 | L01:30f208 | 3/264 | partly | partly: fxdraw | fxdraw math |
| U91 | 675 | 02 | 1/1 | L02:2dd370 | 1/88 | cheap | — | anim math |
| U94 | 713 | 02 | 1/1 | L02:2ddc88 | 7/1502 | missing | missing: camera,herostate,path; partly: help,herostate; untagged ×1 | cine math moby particles platform sound |
| U96 | 733 | 02 | 1/1 | L02:2df3d8 | 4/378 | cheap | — | anim creature math moby sound |
| U98 | 743 | 02 | 1/1 | L02:2dffc8 | 3/20 | cheap | — | — |
| U99 | 744 | 02 | 1/1 | L02:2e0138 | 1/82 | cheap | — | math |
| U228 | 1474 | 07 | 0/1 | L07:2cdb28 | 2/416 | partly | partly: herostate | math platform sound |
