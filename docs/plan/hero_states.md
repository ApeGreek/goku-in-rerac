# Hero states: inventory, port packages, code structure

Every state of Ratchet's (and the other hero bodies') state machine, what reaches it, what it does, which module
of the port owns it and whether it is ported; the shared hero subsystems the missing states need; the six port
packages for the "finish Ratchet" push; and the restructured hero code that lets them be ported in parallel.
Companion of `docs/plan/player_controller.md` (the detailed spec of the ported states) — where the names in its §3
"noted only" list differ from this page, this page is the corrected one.

Confidence: **H** read from the instructions / decompiler C of the switch case itself, **M** from the case plus its
callers, **L** inferred (identity guessed from behaviour, flagged). Addresses are level01 unless marked `L00`
(level00, the superset build; decompiler C in `decomp/export/level00.elf/`).

## 0. How the state machine is built (and one surprise)

- Three switches on the state `0x1413d4`: **SetState** `0x23cf98` (entry: group byte `0x1413dc`, jump block, anim),
  **per-state physics** `0x2370b8` (called by the driver `0x231d18` before the move), **transitions** `0x242930`
  (after the move and the surface reaction; ends in `SetState(new, 1)`). Several ids share one case in each switch
  (the whole jump group shares one entry, one physics case and one transition case). **H**
- **Each level overlay compiles its own copy of the three switches** (clusters.tsv: `SetState` is unique per
  level, 13832 bytes on level01), with only the states that level can reach. Level01 (Novalis) has 83 ids; the
  union over the 19 levels has 126 SetState case labels; level00 (Veldin 1) has all but 0x3e (level 16 only) and
  0x6b..0x6f (levels 5 and 16: the Hoverboard). The surface reaction `0x22cd48` is per-level too (level01 lacks
  surfaces 1, 3 and 7; level00's is `0x20b960`). Per-level SetState: L00 0x2223f8, 02 0x22b728, 03 0x216648,
  04 0x2159d8, 05 0x24cee8, 06 0x2356a0, 07 0x24a260, 08 0x230b38, 09 0x2420d8, 10 0x215a60, 11 0x24db50,
  12 0x2400d0, 13 0x22dee0, 14 0x22ff58, 15 0x216c38, 16 0x21e398, 17 0x21e530, 18 0x227dd0; level00 physics
  0x217970, transitions 0x229b70. **H**
- **Port consequence:** the port implements the superset once (the "general" rule), with level00 as the reference
  for code that level01 does not contain. A state a level does not compile is unreachable there anyway (nothing in
  its data leads to it). Surface rules are the one place where the superset could change a level's behaviour
  (a face with surface 7 on a level whose reaction ignores 7): package P1 checks the surface ids each level's faces
  use against that level's own reaction.
- Scripts used for this inventory: `decomp/export/*/` (all levels' unique functions), the per-level case union and
  level01-name mapping through `decomp/names/clusters.tsv` (the scripts were throwaway; rerun by grepping each
  level's export for `param_1 != 100` + `iRam001413dc == 0x14`).

### 0.1 Items (the game state's owned table `0x13d4c0 + id`; `GetClankModule(slot)` 0x22ddd8 = the ready item of item slot `slot`, slot 0 the hand, slot 3 the back)

| id | item | owned flag | used by |
|---|---|---|---|
| 2 | Heli-Pack | 0x13d4c2 | glide 8, 10, 0xf; back module 2 |
| 3 | Thruster-Pack | 0x13d4c3 | 0x10, 0xd, 0x22, 0x81; back module 3 (the Thruster tests read only the module) |
| 4 | Hydro-Pack | 0x13d4c4 | 0x35 (ported); back module 4 blocks the glide |
| 6 | O2 mask | 0x13d4c6 | no air drain (ported) |
| 7 | Pilot's helmet | 0x13d4c7 | (ship) |
| 8 | wrench | — | melee (ported) |
| 12 | Swingshot | 0x13d4cc | hand item → 0x24 / 0x2c |
| 28 | Magneboots | 0x13d4dc | gravity mode 1 on surface 2; 0x3f / 0x70 / 0x71 |
| 29 | Grind Boots | 0x13d4dd | grind 0x28.. (no ownership test found in the hero code; the rail contact decides) |
| 30 | Hoverboard | 0x13d4de | 0x6b..0x6f |
| 31 | Hologuise | — | hand item → body mode 3 |

Names from Wrench's `types_rac.h` gadget enum (item ids), matched to the flags the hero code tests. **H** for the
flags read, **M** for the names.

### 0.2 Bodies (`0x1413f4`, "character mode"; `SwitchCharacter(mode, state, moby)` L00 0x210a08)

0 Ratchet · 1 Clank (own health 0x1415fc) · 2 Giant Clank (energy 0x140980 = 200) · 3 the Hologuise disguise (moby
class 0x27a, entered after the Hologuise's 18-tick timer 0x14162e, state 0x53). The states 0x43..0x52, 0x5a..0x62,
0x53..0x59 belong to bodies 1, 2, 3. **M** (1 and 2 from their levels and moves: **L**)

### 0.3 Groups (`0x1413dc`)

0 standing · 1 walking · 2 falling · 3 ledge · 4 jumping · 5 gliding · 6 melee · 7 hurt · 8 weapon stance ·
9 scripted · 0xa rebound · 0xb stomp · 0xc crouch · 0xd Swingshot pull · 0xe Swingshot swing · 0xf grind ·
0x10 sinking floor · 0x11 underwater · 0x12 water surface · 0x13 gadget pose · 0x14 dead (SetState refuses
everything but 100) · 0x15 (level 16) · 0x16 Hoverboard · 0x18 cutscene · 0x19 sinking liquid · 0x1a cable.

## 1. State inventory (0..=0x82)

Columns: **grp** = group byte; **reached by** = where SetState(id) comes from; **leaves to** = the transitions;
**module** = the port file / package; **st** = port status (**P** ported, **E** entry only, **–** not). The same
table is `crates/rc-game/src/hero/registry.rs::STATES` (name, group, module, ported).

### 1.1 Ratchet on foot (ported)

| id | name | grp | reached by | leaves to | anims | module | st |
|---|---|---|---|---|---|---|---|
| 0 | idle | 0 | landing, stop, walk release | 1 L1/L2, 0x81 hover latch, 7/0xb jump, 0x71 (magnet ✕), 4 R1/R2, 6 coyote, 2 stick, 0x79 pit, 0x40 fidget record 2 | 0 / 0x54 (health 1) | ground.rs | P |
| 2 | walk / run | 1 | idle, landing, stop | 6, 7/9/0xb, 1, 4, 0x73 / 0x2f / 0x3f by surface, 0x28 rail, 3, 0 | 3 walk, 4 run | walk.rs | P |
| 3 | stop / skid | 1 | walk, landings | 1, 7, 4, 6, 0, 2 | 5 / 6, 0x14 | ground.rs | P |
| 4 | crouch | 0xc | R1/R2 | 1, 0 release, 0xb / pack crouch jumps / 7, 0x71, 0x15 (□), 6 | 0xd, 0xe/0xf turn | ground.rs | P |
| 6 | fall | 2 | coyote, jumps' fall-over | landing 0/2/3 (0xc hard), 0x3d, 8 glide, jump (stuck), 0x10, 0x18 ledge, 0x28 rail, 0x74 cable | 10, 0xb, 0xc | air.rs | P |
| 7 | jump | 4 | ✕ | 0xb flip chain, 10/0x10 R1 tap, 0x22, 6, landing picker, 9/7 bunny hop, 0x11 wall, 0xe, 8 glide, 0x18 | 7 | jump.rs | P |
| 9 | running jump | 4 | ✕ running | as 7 (raw write to 7 when slow) | 8 / 9 by run frame (0x17c258) | jump.rs | P |
| 0xb | side / back flip | 4 | crouch + side/back stick + ✕ | as 7 (no double jump) | 0x1c/0x1d/0x1f | jump.rs | P |
| 0xe | double jump | 4 | ✕ in 7/9 after 14 ticks | landing, 6, 8 glide, 0x18 | 0x16 fr 3 | jump.rs | P |
| 0x12 | water jump | 4 | ✕ treading water | as 7, 0x37 | 0x72 | jump.rs | P |
| 0x13 | wrench combo | 6 | □ (item 8) | 0x13 chain, 0x15, 7, 0 | 0x17..0x19 | melee.rs | P |
| 0x14 | wrench jump attack | 6 | □ in the air | 7, 0 | 0x2b | melee.rs | P |
| 0x33 / 0x34 / 0x35 / 0x36 / 0x37 / 0x6a | swim (see player_controller §14) | 0x11 / 0x12 / 0x14 | water | — | 0x3b..0x3d, 0x65, 0x3a, 0x5f | swim.rs | P |
| 0x73 | wade | 1 | walk in 0.25..0.85 of water | 2, 0 | 0x60 | walk.rs | P |

### 1.2 Ratchet, not ported

Physics column: the case of `0x2370b8` (L00 `0x217970` where level01 lacks it) and the helpers it calls.

| id | name | grp | reached by | leaves to | physics / helpers | anims | module (package) | st |
|---|---|---|---|---|---|---|---|---|
| 1 | look stance | 0 | L1/L2 in 0/2/3/4 | 0 release (anim `0x226f10(1)`), 4 | ground case + camera-facing turn (0x1413f5) | `0x226f10(1)`, 0xd | stance.rs (P2) | – |
| 8 | pack glide / hover | 5 | ✕ held in 6 (> 0.6) / jumps (> 0.6, 1.0, 1.5), Heli-Pack owned, back ≠ Hydro | 6 after 30 ticks without ✕, 0x18, 0x11, landing 3/4/0/2 (0x13f524 = 12 − T) | target 3·dt (5·dt with 0x1404f8 = 3), TurnTo(0.025, 0.3, 720°/s), SpeedStep(15, 7)·dt², vz 0x248f68/0x248b68, sound slots, `HeroWallLedgeCheckA` | 0x13 curve −2 | packs.rs (P4) | – |
| 0xa | Heli-Pack long jump | 4 | crouch + ✕ moving (stick > 0.7, fwd > 0.9·x) with the Heli-Pack; R1 tap in 7/9 | 3 on landing (anim 5, momentum ×0.8), 0x7a wall | jump case: h 1.9, g 11·dt², air 120 | 0x12 | packs.rs (P4) | – |
| 0xc | jump variant | 4 | unknown (no parameters in the jump entry) | jump case | jump case | — | jump.rs (none) | – |
| 0xd | Thruster-Pack high jump | 4 | crouch + ✕ standing, Thruster | jump case | curve jump: h 0.1..0.2, takeoff 9, table 0x17c3a0, window 9..44 | 0x11 | packs.rs (P4) | – |
| 0xf | Heli-Pack high jump | 4 | crouch + ✕ standing, Heli-Pack | jump case | curve jump: h 1.9, takeoff 9, table gp−0x7518, fall-over 4.5, air 130 | 0x15 | packs.rs (P4) | – |
| 0x10 | Thruster-Pack long jump | 4 | crouch + ✕ moving / ✕+R1 combo in 6 / R1 tap in 7/9, Thruster | 6 after 60 ticks > 1.5, 0x7a, 0x22 | jump case: h 0.4..0.9, ramp 1, g 8.5·dt², acc 44 / dec 50·dt², 0x13f744 = 11.5·dt | 0x26 | packs.rs (P4) | – |
| 0x11 | wall jump | 4 | ✕ within 7 while the wall window 0x13f504 is open (7, 9, 0x11 after 25) | jump case | jump case: h 3.3, takeoff 9, g 29·dt², 0x13f7c0 = the wall normal | 0x23 | ledge.rs (P3) | – |
| 0x15 | comet strike | 6 | crouch + □ | 0 / wrench throw states | 0x15 case (speed 0, aim, `0x236da0`) | 0x1a | melee.rs (later) | E |
| 0x16 | hurt (knockback) | 7 | hit intake on foot | 6 (airborne after frame 12.5 above 1), 0x3d, 0 / 0x81 after 27 | |vel| − 4·dt² (10·dt² grounded after 10), edge brake on 0x140637, vz −= 0.004 | 0x10 curve −3 fr 3 | damage.rs (P2) | – |
| 0x17 | weapon fire stance | 8 | hand item fire | 0 when the fire button is released and 0x13f520 = 0 | ground case (`0x2324f8` turn) | idle blend 10 | (Weapons, later) | – |
| 0x18 | ledge grab | 3 | 6, 8, jumps with 0x13f838 | 0x19 after frame 13.5 | turn to ledge yaw + π (0.04, 0.2, 360°/s), pull to 0x13f820, ClampLen 4·dt | 0x20 | ledge.rs (P3) | – |
| 0x19 | ledge hang | 3 | 0x18, shimmy end | 0x1c (✕), 6 (back + ✕ / R1, 0x13f500 = 10 / 40), 0x1a / 0x1b (probe `HeroWallLedgeCheckC` 0x20c758 at ±90° 0.3) | as 0x18 | 0x21 | ledge.rs (P3) | – |
| 0x1a / 0x1b | shimmy left / right | 3 | 0x19 + stick | 0x19 (wrap / no probe), 0x1c, 6 | two probes `HeroWallLedgeCheckB`, speed from table 0x1c4130 by frame | 0x24 / 0x25 | ledge.rs (P3) | – |
| 0x1c | ledge climb / jump up | 4 | ✕ hanging | jump case | jump case: h 2.0, takeoff 10, g 25·dt²; carry follows 0x13f848 | 0x22 | ledge.rs (P3) | – |
| 0x1d / 0x1f / 0x32 / 0x72 / 0x78 / 0x3b | scripted control (0x1413fc no control; 0x32 frozen; 0x78 vel = 0; 0x3b item use, hand seq 0xb) | 9 | cutscene / moby scripts | set by the scripts | none / ground case | idle / 0xb / 0x44 | (Scripted, later) | – |
| 0x1e | look stance (mobys) | 0 | four moby classes | 0 | ground case | `0x226f10(1)` | stance.rs (P2) | – |
| 0x20 | gadget lunge | 6 | fire with a hand item (group < 3, 4, 5) | 0 after frame 20 | lunge 18·dt ticks 10..18, 3 hit spheres (0.8 ahead, ±50°) | — | melee.rs (later) | E |
| 0x21 | wrench rebound | 0xa | wrench hit on a flag-2 target | 0 on wrap | speed → 0 by 24·dt², away from 0x13fdb8 | 0x27 + row | melee.rs (later) | – |
| 0x22 | Thruster stomp | 0xb | R1/R2 in the air (7, 9, 0xd, 0xf, 0xe) with the Thruster | 0x10 (✕ early), 0 on wrap | up 5.7·dt for 12 ticks, then 100·dt² down after 33, landing shockwave `coll_sphere_mobys` (0.8, flags 0x30000), camera shake 0x167260 | 0x2a | packs.rs (P4) | – |
| 0x23 | melee 0x23 | 6 | hand item | 7 (✕ after frame 19), 0 | aim, SpeedStep(30, 35) | — | melee.rs (later) | E |
| 0x24 | Swingshot fire | 0xd | Swingshot at a pull target 0x13fcb4 | 0 | turn to the target, edge brake | 0x2f loop 10..14 | swingshot.rs (P6) | – |
| 0x25 | Swingshot pull | 0xd | 0x24 | 0, 6 | fly to the target (`0x2595a0`, `0x25bc00`) | 0x30 | swingshot.rs (P6) | – |
| 0x26 | Swingshot arrive | 0xd | 0x25 | 0, 6 | approach, distance 0x13fccc | 0x30 | swingshot.rs (P6) | – |
| 0x27 / 0x30 | weapon stances | 8 | hand items | 0 (0x30: anim 0x36 wrap) | ground-like (0x30: drag, edge brake, gravity) | idle | (Weapons, later) | – |
| 0x28 | grind | 0xf | rail contact 0x13f8bc from 2 / 6 | 0x29 (✕), 0x2a (rail switch), 0x2b (□), 0x42, 0x16, 6, 0 | `SplineProject` / `SplineSample` on the grind paths (section 0x74), sparks PartType25 | 0x31 / 0x32 by stance | boots.rs (P5) | – |
| 0x29 / 0x2a | grind jump / rail switch | 0xf | ✕ / ✕ + side on a rail | the grind case | grind case (jump block h 2.4 (+4.7 booster 0x140638) / 2.5) | 0x50 / 0x1e, 0x1f | boots.rs (P5) | – |
| 0x2b | grind wrench | 0xf | □ grinding | grind case | grind case | 0x4e / 0x4f, wrench 0xc / 0xd | boots.rs (P5) | – |
| 0x2c | Swingshot swing | 0xe | Swingshot at a swing target 0x13fce0 | 0x2d (release) | rope (length 0x13fcf4, pvar +0x78 k/d/max), gravity 27·dt² | 0x34 / 0x35 | swingshot.rs (P6) | – |
| 0x2d | fall after a swing | 2 | 0x2c | fall case of 6 | fall case (lean) | 0xb | swingshot.rs (P6) | – |
| 0x2e | weapon draw walk | 1 | hand item | 0x17 / 0 after frame 28 | — | 0x2e | (Weapons, later) | – |
| 0x2f | slippery-floor walk | 1 | walk on surface 7 (0x140632; L00) | 2 off the surface, 0 | own case (L00 0x217970) | 0x37 / 0x6d | surface.rs (P1) | – |
| 0x31 | sinking floor | 0x10 | surface 4 (0x140633), grounded, not in groups 7/0x10/0x14 | 0 when off it and 0x13f530 = 0 (momentum = the carried part) | vz −40·dt² (≥ disp.z − 50·dt²), turns with 0x13f440, particles 47, sound slot 0x141570, body roll springs | 100 | surface.rs (P1) | – |
| 0x38..0x3a | gadget poses | 0x13 | hand items (L) | 0 | stop | 0x46 / 0x47 / 0x48 | (Weapons, later) | – |
| 0x3c | burn bounce | 4 | surface 1 (0x140635; L00), damage 1 | jump case | jump case: h 5.5, takeoff 5, g 15·dt², 150 ticks; → 0x7c on level 10 / twice | 0x43 | damage.rs (P2) | – |
| 0x3d | death | 0x14 | health < 1 in 0/2, landing without health | fade 0x2319b0 on wrap | ground case | 0x45 | damage.rs (P2) | – |
| 0x3f | Magneboots walk | 1 | walk with 0x13f658 (magnetic floor moby) | 0, 4, 0x71 | own case, gravity mode 1 | 0x5b | boots.rs (P5) | – |
| 0x40 / 0x41 | fidget state / end | — | fidget record 2 | 1, 0x41 / 6, jumps, 4, 2, 0 | none | idle | stance.rs (P2) | – |
| 0x42 | grind hurt | 0xf | hit while grinding | grind case | grind case | 0x10 curve −3 | boots.rs (P5) | – |
| 0x65 / 0x66 / 0x67 | walk to point (0x140990) | 1 / 1 / 0 | scripts (vendor, ship) | 0x67 / 0x65 / 0 | own cases | walk anims | stance.rs (P2) | – |
| 0x68 / 0x69 / 0x7b | sinking liquid (surface 3, to the level 0x13f644) / jump out / no health | 0x19 | surface 3 (0x140636; L00), level 0xd body contact type 0xb | 0x69 (✕) / jump case / fade | none (the liquid holds him) | 0x71 / 7 | surface.rs (P1) | – |
| 0x70 | Magneboots wrench swing | 6 | □ with 0x13f658 = 1 | 0 | ground case | 0x5c + row | boots.rs (P5) | – |
| 0x71 | Magneboots jump | 1 | ✕ in 0 / 4 with 0x13f658 = 1 | 0 | ground case, gravity mode 1 | 0x5e | boots.rs (P5) | – |
| 0x74 | cable slide (**L**) | 0x1a | fall with 0x13f94c | 6 | spline follower (speed → 14 u/s, spring hang), PartType25 | 0x73 / 0x66 | boots.rs (P5) | – |
| 0x75 / 0x76 | hurt on the surface / under water | 7 | hit intake in groups 0x12 / 0x11 | 0x37 / 0x34 after 50, 0x6a / 0x82 without health | small cases | 0x70 / 0x6f | damage.rs (P2) | – |
| 0x77 | death fall | 2 | z below the level's death height and > 2 above ground | fade after 120 (level-specific z shortcuts) | fall-like, random spin (2 draws at entry) | 10 / 0xb | damage.rs (P2) | – |
| 0x79 | pit fall | 2 | surfaces 8 / 0xc (0x14063a) | fade after 300; 6 / 0 when off the pit surface | fall case | 0xb | damage.rs (P2) | – |
| 0x7a | pack jump rebound | 0xa | 10 / 0x10 into a wall | 0 on wrap | rebound case (with 0x21) | 0x29 | packs.rs (P4) | – |
| 0x7c | burn death | 0x14 | surface 1 without health (level 6: 0x3d) | fade | none | 0x71 | damage.rs (P2) | – |
| 0x7f | sinking death | 0x14 | surface 0xd (0x14063c) sinking | fade after 220 | own case (bubbles, CreateMoby) | 0x74 | damage.rs (P2) | – |
| 0x80 | hazard death | 0x14 | hit by class 0x4eb / 0x558 | fade | ground case | 0x7c | damage.rs (P2) | – |
| 0x81 | Thruster-Pack hover | 1 | R1 double tap on the ground (Thruster), idle with latch 0x14161a | 0 / land (✕ or R1 after 20 ticks; lockout 25) | own case (Spring, 0x2338d0) | 0x13 curve −2 | packs.rs (P4) | – |
| 0x82 | eaten in the water | 0x14 | hit by class 0x28f in water groups | fade | swim-death case | 0x80 | damage.rs (P2) | – |

### 1.3 Other bodies, Hoverboard, unused (not in this push)

| ids | what | grp | module |
|---|---|---|---|
| 0x43 idle, 0x44 walk (+0x50), 0x45 / 0x52 fall, 0x46 hurt, 0x47 death, 0x49 / 0x4c jumps (anims 7 / 9), 0x4a / 0x4b / 0x4d / 0x4e ledge (anims 10..0xc), 0x4f glide (0xe), 0x51 kick (group-6 entry in melee.rs), 0x7d burn, 0x48 (physics no-op only) | Clank (body 1; levels 0, 4, 6, 7, 9, 10, 13, 17) | 0..0x14 | Bodies (later) |
| 0x5a idle, 0x5b walk, 0x5c fall, 0x5d hurt, 0x5e jump (h 4.2), 0x5f..0x61 attacks, 0x62 death | Giant Clank (body 2, **L**; levels 0, 4, 7, 9, 10, 13, 15, 18) | | Bodies (later) |
| 0x53 idle, 0x54 walk, 0x55 fall, 0x56 hurt, 0x57 death, 0x58 pit fall, 0x59 ○ action | Hologuise disguise (body 3; every level) | | Bodies (later) |
| 0x6b ride, 0x6c, 0x6d, 0x6e into water, 0x6f; 0x3e (group 0x15, level 16) | Hoverboard (levels 5, 16) | 0x16 | Hoverboard (later) |
| 0x63 / 0x64 | cutscene control; 100 is the only state group 0x14 accepts (respawn) | 0x18 | Scripted (later) |
| 0x05, 0x7e | no SetState case (0x7e: a walk-case label only) | — | Unused |

### 1.4 Counts

131 ids: 2 unused, 32 other bodies, 6 Hoverboard, 91 Ratchet. Ported 19 (ground 3, walk 2, air 1, jump 5, melee 2,
swim 6). Not ported, by package: P1 surface 5, P2 damage 11 + stance 7, P3 ledge 6, P4 packs 8, P5 boots 9, P6
Swingshot 5 = **51 in this push**; later: weapons 7 + melee 5 (0x15, 0x20, 0x21, 0x23, 0x51), scripted 8, jump
0xc 1. By game group (all ids, ported / not): 0: 1/7, 1: 3/10, 2: 1/8, 3: 0/8, 4: 5/11, 5: 0/2, 6: 2/8, 7: 0/6,
8: 0/3, 9: 0/6, 0xa: 0/2, 0xb: 0/1, 0xc: 1/0, 0xd: 0/3, 0xe: 0/1, 0xf: 0/5, 0x10: 0/1, 0x11: 3/0, 0x12: 2/0,
0x13: 0/3, 0x14: 1/9, 0x15: 0/1, 0x16: 0/2, 0x18: 0/2, 0x19: 0/3, 0x1a: 0/1, none: 0/9.

## 2. Shared hero subsystems

| subsystem | game | port today | needed by |
|---|---|---|---|
| jump system (jump block 0x13f720.., vertical 0x2345f0 incl. the scripted curves 0x13f76c, air control 0x234b40, landing picker, descent anim) | shared case | jump.rs for 7/9/0xb/0xe/0x12; curve table 0x17c3e0 only | P3 (0x11, 0x1c), P4 (10, 0xd, 0xf, 0x10), P5 (0x29, 0x2a entries), P1 (0x69), P2 (0x3c) |
| ledge / wall probes | 0x22c9a0 A, 0x22d090 B, 0x20c758 C; 0x13f504, 0x13f838, 0x13f820/834/848 | stubs called at the game's points | P3, P4 (glide), P1 (carry of 0x13f848) |
| surface classification | ground probe → 0x140630; reaction 0x22cd48 / L00 0x20b960 → flags 0x140632..0x14063e | water, 0xe, 2, 8/0xc flags | P1 (rules), P2 (hazard deaths), P5 (surface 2) |
| slopes / sliding | 50° walkable limit, capsule slide, edge brake 0x236a68, steep-wall stop 0x232820, slope ratio 0x13f4bc, pitch/roll 0x13f634/638 | ported | P1 (slippery 0x2f), P5 (gravity mode 1 frame) |
| platform carry | `HeroPlatformUpdate` 0x249618 + `triggers::platform_delta` / `carry_point`, 0x13f440 / 0x13f44c, 0x13f6b0/6b4 | stub at the start of the move; the move already applies 0x13f440 | P1; the lift 726 / elevators 703 / 715; P3 (hang on a moving ledge) |
| damage / knockback / invulnerability | hit intake 0x231580 (L00 0x210ce8), hit records 0x178110, `0x210c80` knockback → 0x13f680.., push 0x233850 (ported), `HeroTakeDamage`, 0x13f510, 0x13f53e | push generator only; timers counted down | P2; every hazard state |
| death / respawn | 0x2319b0 (deaths++, fade, 0x141401) | `fell_out`; the engine respawns on entering 0x77 / 0x3d | P2 with the engine owner |
| item ownership | `0x13d4c0 + id` | `swim.hydro_pack`, `swim.o2_mask` (synced by gameplay.rs) | P4, P5, P6: replace with one owned-items mirror (`Hero` field synced from `GameState::global.owned`) |
| back slot (pack module) | item slot 3, `GetClankModule(3)`, back anim table 0x2476d0 (rows 2 / 3 / 4) | `idle::Back` (heli-pack model 607 + Clank 601), no slot state / module id | P4 (module id, pack models 607..609, Hydro model on 0x35) |
| spline follower | `SplineProject` / `SplineSample` (L00 0x25d808 / …), grind paths section 0x74 (`rc_formats::volumes::GrindPath`), paths 0x70 | none | P5 (grind, cable) |
| targets | Swingshot targets 0x13fcb4 / 0x13fce0 (+0x78 pvar record), target search in the weapon check | none | P6 (needs the target classes' moby updates) |
| gravity frame | gravity mode 0x141403 (0x248ad8), gravity dir 0x13f5e0, `SetPlanarVel` / TurnTo in mode 1 | mode computed (rule differs, §4) | P5 |
| timers | FastDecTimer list of 0x23c710 | all counters now in `Hero` and counted down (§5) | all |
| hero sounds | class sounds of the anim triggers inside 0x247d48; voices `0x236738` / `0x236810` | `HeroSounds` hook (§5); engine replays the triggers in its sound step | P2 item (below) |

## 3. Packages (the "finish Ratchet" push)

Rules for every package: own only your files; the shared files (`hero.rs`, `registry.rs`, `states.rs`,
`common.rs`, `physics.rs`, `ground.rs`, `walk.rs`, `air.rs`, `jump.rs`, `melee.rs`, `swim.rs`, `idle.rs`,
`items.rs`, `anim.rs`) are frozen except (a) setting `ported: true` on your rows in `registry.rs` (one-line edits),
(b) the owner of `jump.rs` named below, (c) a new seam agreed with the orchestrator (one call into your module at
the game's branch point, listed in your report). Behaviour of the ported states must not change (run the
`novalis_hero_digest` guard, §5). Native floats for new code (`Pf::f` / `to_f32` at the boundary, as swim.rs does).

| # | package | files owned | states | depends on | wave |
|---|---|---|---|---|---|
| P1 | platform carry + surfaces / slopes / sliding | `platform.rs`, `surface.rs` | 0x2f, 0x31, 0x68, 0x69, 0x7b | the moby table in `Env` (below) | 1 |
| P2 | damage, knockback, death + stances + hero sounds | `damage.rs`, `stance.rs` | 0x16, 0x3c, 0x3d, 0x75, 0x76, 0x77, 0x79, 0x7c, 0x7f, 0x80, 0x82; 1, 0x1e, 0x40, 0x41, 0x65..0x67 | hit records from moby_update (`HitSink` side), engine respawn contract | 1 |
| P3 | ledges + wall jump | `ledge.rs` (+ `jump.rs` during wave 1) | 0x11, 0x18..0x1c | — (P1's carry for moving ledges, later) | 1 |
| P4 | Heli-Pack + Thruster-Pack | `packs.rs` (+ `jump.rs` during wave 2) | 8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81 | item ownership mirror, back slot module id + pack models | 2 |
| P5 | Magneboots + Grind Boots + cable | `boots.rs` | 0x28..0x2b, 0x3f, 0x42, 0x70, 0x71, 0x74 | grind paths + spline follower, the gravity-mode rule (physics.rs: P5 may edit the gravity-mode paths), level data in `Env` | 2 |
| P6 | Swingshot (+ the other hand items' weapon check) | `swingshot.rs`, `gadgets.rs` | 0x24..0x26, 0x2c, 0x2d | target mobys (moby_update classes), level data in `Env` | 2 |

Order: wave 1 = P1, P2, P3 in parallel (P1 unblocks the lift and every carrier; P2 unblocks every hazard and the
engine's death handling; P3 is self-contained and Novalis has ledges). Wave 2 = P4, P5, P6 in parallel.

**`Env` extension (one shared edit, first thing in wave 1, by P1):** the hero needs level / moby data it cannot
reach today (carriers' platform blocks, later grind paths and swing targets). Add one field to `hero::physics::Env`
(e.g. `world: Option<&'a dyn HeroWorld>`, a trait in `platform.rs` with default methods P5 / P6 extend) and set it
in `tick.rs` (`Game::tick_with_sound`) and the hero test constructors. `MobyScene` only exposes collision.

### P1 — platform carry, surfaces, slopes and sliding
- Port `HeroPlatformUpdate` 0x249618 in `platform::platform_update` (spec: triggers.md §5; the move already adds
  0x13f440 and re-collides). Needs `Env::world` for the ground moby's platform block (`triggers::platform_delta`).
- The superset surface reaction (L00 0x20b960) in `surface.rs`: surfaces 1, 3, 4, 7, 9, 0xb, 0xd and the level-0xd
  type-0xb contact, with their states (0x3c / 0x7c / 0x7f / 0x79 are P2's: call SetState, P2 ports the states).
  Data check first: surface ids used per level (collision faces) vs the level's own reaction.
- States 0x2f (slippery walk; `walk_surface` / `walk_tail` seams), 0x31, 0x68 / 0x69 / 0x7b.
- Verify: a Novalis script riding the lift 726 (trace 0x13f3d0 / 0x13f440 against the lift's +0x10); PCSX2
  savestate standing on the lift (record 0x13f440, 0x13f6b0, 0x13f6b4 per tick); surface ids per level printed by
  a test; sliding: a script up a > 50° slope on Novalis (falls and slides back, unchanged by the port).

### P2 — damage, knockback, death, stances, hero sounds
- The hit intake 0x231580 (seam `damage::hit_intake`, first call of the transitions): hit record, invulnerability
  0x13f510, knockback 0x210c80 → `Hero::knock`, `HeroTakeDamage`, the hurt state per group; the hit flash 0x13f53e.
- States in the table (0x16, 0x75, 0x76, the deaths, 0x77 incl. its two RNG draws at entry, 0x79, 0x3c via the jump
  system) and the death fade 0x2319b0 → `fell_out` / 0x141401; agree with the engine owner to respawn on
  0x141401 instead of on entering 0x77 / 0x3d.
- Stances 1 / 0x1e (look), 0x40 / 0x41 (fidget state), 0x65..0x67 (walk to point).
- **Move the animation-trigger sound draws into the hero update:** `hero::hero_update_with_sounds` calls
  `HeroSounds::anim_advanced(moby, before, after, rng)` right after Ratchet's advance 0x247d48 (the game's point,
  before the back items' 0x247800 draw). Implement `HeroSounds` for the audio layer (`audio::class_sounds::
  ratchet_trigger` + `AudioSystem::play_class_sound`), call `hero_update_with_sounds` from `Game::tick_with_sound`
  (tick.rs) and drop the deferred replay of `TriggerAnim` in `sound_step`. The draw count is unchanged; only their
  position in the tick moves (the RNG stream after the particles changes: re-baseline traces). Coordinate with the
  audio / tick owner.
- Verify: Novalis enemies' contact hits (needs their moby updates' hit records), a savestate taking one hit
  (0x1413d4, 0x13f680..688, 0x1415f8, 0x13f510 per tick), drowning (swim), the death fade.

### P3 — ledges and the wall jump
- Probes A / B / C and the states; 0x11 / 0x1c through the jump system (`jump_block_defaults`-style entry +
  `phys_jump` / `tr_jump`); seams exist: `wall_ledge_probe_a` (jump physics), `wall_ledge_probe_b`
  (`hero_update`), `wall_jump` (jump transitions), `Hero::f838` → 0x18 (fall and jumps).
- P3 owns `jump.rs` in wave 1 (for 0x11 / 0x1c branches). Entries: `Hero::jump_block_defaults` (the shared part
  of the jump entry) + the id's parameters and anim.
- Verify: Novalis ledges (the drop off the spawn plateau): script jumping at a ledge edge; savestate hanging;
  unit tests on a hand-built step (testkit::cell).

### P4 — Heli-Pack and Thruster-Pack
- Item ownership mirror + back slot module id (item slot 3 state / id) + the pack models on Clank (607..609 and
  the back table rows 3 / 4 in idle.rs); then the states and seams listed in `packs.rs` (crouch jumps, glide from
  fall / jumps, R1 taps, stomp, wall rebound, hover 0x81 and its latch, the heli branch of the double-jump boost in
  0x2345f0, the scripted curves 0x17c3a0 / gp−0x7518 for 0xd / 0xf beside the water jump's).
- P4 owns `jump.rs` in wave 2.
- Verify: an engine debug switch like `RC_GIVE_HYDROPACK` (own item 2 / 3, back module 2 / 3) on Novalis: glide from
  a jump off the plateau, crouch-jump heights (apex against the jump-block model as in player_controller §4.4),
  the long jumps; PCSX2 savestate on Kerwan (Heli-Pack) for a trace.

### P5 — Magneboots, Grind Boots, cable
- Magneboots: the gravity-mode rule 0x248ad8 (fix the port's inverted air-tick test and the missing 0x13d4dc test),
  the gravity frame in mode 1 (probe, `SetPlanarVel`, TurnTo, capsule along 0x13f5e0), 0x3f / 0x70 / 0x71, the
  ground-physics magnet branch (seam `boots::ground_magnet`).
- Grind: one spline follower for 0x28..0x2b / 0x42 and 0x74 (L00 physics 0x217970 case 0x28.., transitions
  0x229b70 case 0x28..; `SplineProject` / `SplineSample` / 0x25d7a0), the rail contact 0x13f8bc (seam
  `boots::rail_contact` in walk and fall), grind paths from `Env::world`.
- Verify: savestates on Orxon (10; magnetic walkways) and a grind level (Gaspar 9 / Kalebo 16); unit tests on a
  straight and a curved hand-built path.

### P6 — Swingshot and the other hand items
- `gadgets::pda_item` (the non-wrench cases of 0x240ed8, must call `packs::pda_epilogue` when the state did not
  change), the target search, 0x24..0x26 (pull), 0x2c / 0x2d (swing), the straightening exclusions.
- Needs the target mobys' updates (moby_update classes) and their pvar records via `Env::world`.
- Verify: a Kerwan / Eudora savestate at a target; unit test pulling to a fixed target and swinging on a fixed
  pivot (rope length kept, release into 0x2d).

### Later (not in this push)
Weapons (0x15 comet, 0x17, 0x20, 0x21, 0x23, 0x27, 0x2e, 0x30, 0x38..0x3a, the bomb glove's throw), scripted
states (0x1d, 0x1f, 0x32, 0x3b, 0x63, 0x64, 0x72, 0x78; with the cutscene port), other bodies (Clank, Giant Clank,
Hologuise), the Hoverboard. Each gets a module when started; the registry already names its rows.

## 4. Findings to fix inside the packages (not fixed now: the restructure is behaviour-neutral)
- **Gravity mode** (`Hero::input_physics_move`): 0x248ad8 returns 1 in state 0 on surface 2 when the Magneboots are
  owned and the air ticks are **below** 4; the port forces 1 with ≥ 4 air ticks and no ownership test. (P5)
- **Surface 2 is the magnetic floor** (Magneboots), not "slippery" as an old comment said; the slippery floor is
  surface 7 (0x140632, level00's reaction). (P1 / P5)
- `HeroItems::f52a` / `f52c` (0x13f52a / 0x13f52c) are set on a swap but never counted down (0x23c710 counts them).
  Nothing reads them yet. (weapons, later)
- `HeroPdaGadget` 0x240ed8: whether its early returns (no hand moby, slot not ready) skip the Thruster epilogue is
  not checked; `gadgets::pda_item` / `packs::pda_epilogue` must follow the disassembly. (P4 / P6)
- The jump buffers read 0x13f524 (1 tick instead of 7 / 9 / 6 while it runs): now in the port (0 until P4 sets it).
- player_controller.md §3 "noted only" names corrected here: 1 / 0x1e are the look stance (not strafe), 8 the
  pack glide, 10 / 0x10 the pack long jumps, 0xd / 0xf the pack high jumps, 0x22 the Thruster stomp, 0x81 the
  Thruster hover (not strafe-move), 0x16 / 0x75 / 0x76 hurts, 0x17 / 0x27 / 0x30 weapon stances, 0x18..0x1b
  ledge, 0x31 the sinking floor, 0x41 / 0x42 are the fidget end and the grind hurt (not one group).

## 5. The restructure (2026-09-26)

`crates/rc-game/src/hero/`:
- `registry.rs` — `STATES` (131 rows: name, game group, module, ported), `module_of`, `implemented` (replaces the
  hard-coded list; `hero::state::implemented` delegates), and the three dispatchers `dispatch_entry` (SetState's
  per-state part; `None` = continue with the epilogue, `Some(r)` = return `r`), `dispatch_physics`,
  `dispatch_transitions`. Tests: the implemented set is unchanged; no ported state sits in a stub module.
- `states.rs` — the driver: `set_state` (refusals, bookkeeping, the 0x1413f7 restore), `set_state_body` (blink
  reset → dispatch → epilogue), `transitions` / the prologue (hit intake seam, death check, weapon check, water
  checks, holster seam, death plane) → dispatch. `Ctx`, `clamp_len_2745f0` (re-exported) stay here.
- `common.rs` — `yaw_ago`, `idle_seq`, `gravity_from`, the jump choosers (`stick_sector`, `flip_window`,
  `try_jump`, `crouch_jump` with the pack seam, `jump_or_running_jump`, `step_down_ahead`), `land_eta`,
  `quadratic`, `clip_to_world`, `blend`, `HALF_PI` / `QUARTER_PI`.
- Group files (code moved verbatim, only visibility and seams changed): `ground.rs` (0, 3, 4: entries,
  `phys_ground`, `tr_idle` / `tr_stop` / `tr_crouch`), `walk.rs` (2, 0x73: entry, `phys_walk`, `tr_walk`,
  `walk_run_anim`), `air.rs` (6: entry, `phys_fall`, `tr_fall`), `jump.rs` (the jump group: `jump_group_entry`,
  `jump_block_defaults` (the shared part of the entry, split out for the package jump ids), `jump_entry`, `phys_jump`, `jump_vertical`, `tr_jump`, `landing_picker`, `descent_anim`); `melee.rs`, `swim.rs`
  unchanged apart from the weapon-check seams.
- `physics.rs` keeps the primitives, `Env`, air control, the move pipeline; `state_physics` is the dispatcher
  wrapper; the move starts with `platform::platform_update`; `post_move` counts down every FastDecTimer of
  0x23c710 (new `Hero` fields f500, f502, f508, f510, f518, f51a, f520, f524, f530, f534, f536, f53e, f546).
- `hero.rs` — module list; `surface_reaction` moved to `surface.rs` (now takes the SetState context);
  `hero_update` = `hero_update_with_sounds(.., &mut NoHeroSounds)`; the `HeroSounds` hook after the advance;
  `ledge::wall_ledge_probe_b` after the surface reaction.
- Stubs, routed by the registry, all behaviour-neutral: `platform.rs`, `surface.rs` (P1), `damage.rs`,
  `stance.rs` (P2), `ledge.rs` (P3), `packs.rs` (P4), `boots.rs` (P5), `swingshot.rs`, `gadgets.rs` (P6). Each
  module doc lists its states, the game functions and the seams wired for it.
- Seams in shared code (each a call into a stub that returns "nothing happened" or plain code on a field that is
  always 0 today): transitions prologue (`damage::hit_intake`, `gadgets::holster_check`), weapon check
  (`gadgets::pda_item`, `packs::pda_epilogue`), idle (`packs::hover_latched`, 0x13f524 buffer), walk
  (`surface::walk_surface`, 2 → 0x3f on `f658`, `boots::rail_contact`, `surface::walk_tail`, 0x13f524), fall
  (`packs::fall_to_glide`, 0x13f524, `packs::fall_to_thruster`, `f838` → 0x18, `boots::rail_contact`,
  `boots::cable_contact`), jumps (`packs::jump_pack_moves`, `packs::jump_crouch_press`, `packs::pack_jump_wall_hit`,
  `packs::heli_long_jump_landed`, `packs::thruster_fallover`, `ledge::wall_jump`, `packs::jump_to_glide`, `f838`
  → 0x18, `ledge::wall_ledge_probe_a`), crouch jump (`packs::crouch_jump`), ground physics
  (`boots::ground_magnet`), move (`platform::platform_update`), hero update (`ledge::wall_ledge_probe_b`,
  `HeroSounds::anim_advanced`).

**Proof of identical behaviour.**
- `crates/rc-game/tests/hero_novalis.rs::novalis_hero_digest` (new guard): with `RC_HERO_DIGEST=<file>` it runs
  the lake swim / dive with and without the Hydro-Pack and a move script (jumps, double jump, running jump,
  crouch, flip, fall off the plateau, L1 into the unported look stance), 2800 ticks, and writes per tick the state,
  timer, position and a hash of the whole hero block, Ratchet's anim state, camera, pad and RNG. Before vs after
  the restructure: byte-identical (the new zero timer fields are dropped from the hashed text,
  `DIGEST_NEW_FIELDS`). States covered: 0, 1, 2, 3, 4, 6, 7, 9, 0xb, 0xe, 0x33, 0x35, 0x36, 0x37.
- Engine, `RC_SCENE=0`, frame-exact: `RC_PLAY_SCRIPT="0-5:rstick 1 0,20-226:stick 0 -1,250-310:rstick 1
  0,330-331:press SQUARE,420-421:press X,440-441:press X,500-560:stick 1 0,520-521:press X,600-601:press SQUARE"`
  to frame 650 (crate combo, jumps, double jump; states 0, 2, 6, 0x13, 7, 0xe, 3) and the swim script of
  player_controller §14.7 with `RC_GIVE_HYDROPACK=1` plus `940-941:press X,990-991:press X` to frame 1050 (0x37,
  0x36, 0x35, 0x34, 0x33): `RC_PLAY_TRACE` identical tick for tick before and after (the engine's trace gained a
  `| back pack … | snd …` suffix from a concurrent gameplay.rs change between the runs; compared without it) and
  identical PNGs.
- All rc-game tests pass (200 unit + the integration tests), `cargo clippy -p rc-game --tests` clean for the hero files.
