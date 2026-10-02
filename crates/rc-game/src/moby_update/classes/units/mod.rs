//! The class-port units of the census (docs/plan/class_census.md "Cheap wins", gaps.md G-CLS-027): classes whose
//! update calls only shared functions the port already has, each ported once per **unit** (the classes and the level
//! copies that run the same code, `LevelPorts` / `Relocation`) and registered here as `ClassUpdate::Unit(i)`, the
//! index into [`PORTS`]. A row names the unit's reference update (a function of the level's overlay the census lists
//! as its first copy), the classes that level's table runs it for, and the Rust update. Every other level whose class
//! table names the same code runs the same row.
//!
//! | unit | classes (levels) | reference update | port |
//! |---|---|---|---|
//! | U408 | 212, 1412 (13) | level13 0x2e1638 | [`asteroid`] |
//! | U303 | 1181 (09) | level09 0x304360 | [`chain_link`] |
//! | U553 | 885, 888, 891, 892, 894, 900, 901, 936 (18) | level18 0x2e9768 | [`barricade`] |
//! | U294 | 1182–1189 (09) | level09 0x2c26c8 | [`tethered_platform`] |
//! | U417 | 1261 (13), and the fireball 1634 it makes | level13 0x307b10, 0x30c3b8 | [`explosive_tank`] |
//! | U533 | 1359–1364, 1367, 1369, 1372, 1373 (17) | level17 0x2e87d8 | [`fleet_door`] |
//! | U477 | 937 (15) | level15 0x2e46b0 | [`linked_cog`] |
//! | U479 | 1250 (15) | level15 0x2e73c0 | [`quartu_belt`] |
//! | U500 | 650 (16) | level16 0x2d5cb8 | [`rising_float`] |
//! | U499 | 647 (16) | level16 0x2d59e0 | [`extending_piece`] |
//! | U563 | 1584 (18) | level18 0x2fa728 | [`veldin_carrier`] |
//! | U559 | 1432 (18) | level18 0x2f7ab0 | [`hidden_prop`] |
//! | U484 | 1425 (15) | level15 0x2eb928 | [`bubble_vent`] |
//! | U456 | 1397 (14) | level14 0x3061d8 | [`oltanis_switchboard`] |
//! | U493 | 482 (16) | level16 0x2cb600 (`DeleteMoby(self)`) | [`marker::update`] |
//! | U99 | 87, 283, 346, 419, 690, 765, 789, 1104, 1140, 1278, 1280, 1672 (02, 05–07, 10, 12, 14–16, 18) | level02 0x2dd4d0 (`jr ra`) | [`empty`] |
//! | U27 | 1060 lamps (00, 02, 05, 18) | level00 0x2df4f8 | [`lamp`] |
//! | U268 | 344, 547–551, 588–598, 782–785 loose pieces (08) | level08 0x2dba40 | [`loose_piece`] |
//! | U95 | 296, 652, 653 conveyor belts (02, 12) | level02 0x2dc6b0 | [`conveyor`] |
//! | U241 | 886 timed switches (07, 12) | level07 0x30bf90 | [`timed_switch`] |
//! | U247 | 129, 130, 182, 183, 360, 1063, 1078, 1079, 1131 movers on a linked moby's state (07, 13) | level07 0x310df0 | [`linked_mover`] |
//! | U229 | 1512 steam / spark vents (06) | level06 0x308c68 | [`vent`] |
//! | U280 | 621 rail mines (08) | level08 0x2f44c0 | [`grind_mine`] |
//! | U185 | 852, 853 rising blocks (05) | level05 0x314eb0 | [`rising_block`] |
//! | U221 | 1091–1098, 1103 bobbing blocks (06) | level06 0x300df0 | [`bob_block`] |
//! | U139 | 915, 916, 917 markers deleted at once (03, 10) | level03 0x2dc310 | [`marker`] |
//! | U281 | 648 smoke emitters (08, 10) | level08 0x2f5830 | [`smoke_emitter`] |
//! | U170 | 341 Hydrodisplacer pads (05, 07, 11, 12, 18) | level05 0x2f8080 | [`hydro_pad`] |
//! | U96 | 615 Trespasser locks (02, 04, 06, 08, 11, 13, 18) | level02 0x2d8ad0 | [`trespasser_lock`] |
//! | U107 / U108 | 743 / 744 the sliding doors of the Trespasser lock (02) | level02 0x2dffc8 / 0x2e0138 | [`lock_doors`] |
//! | U365 | 1159 the split door of the Trespasser lock / floor switch (11) | level11 0x30e978 | [`lock_doors`] |
//! | U204 | 367 sliders (06) | level06 0x2d9d10 | [`slider`] |
//! | U82 | 1341 the Novalis help-hint director (01) | level01 0x30acb8 | [`help_director`] |
//! | U36 | 1564 path gliders (00, 07, 10, 18) | level00 0x2e3a88 | [`path_glider`] |
//! | U495 | 257 Kalebo rail cars (16) | level16 0x2c3d38 | [`rail_car`] |
//! | U523 | 1667–1671 Kalebo air traffic (16) | level16 0x2e76b8 | [`kalebo_traffic`] |
//! | U25 | 749 Veldin horny toads (00, 18) | level00 0x2d4610 | [`horny_toad`] |
//! | U287 | 1023 hopping gunners (08, 09), and their shot 1292 | level08 0x301158, 0x307298 | [`hop_gunner`] |
//! | U553 | 568 rolling mines (18; the pool of the boss 1422) | level18 0x2d5918 | [`rolling_mine`] |
//! | U301 | 193 pack biters (09, 15) | level09 0x2e27d8 | [`pack_biter`] |
//! | U268 | 252 hover zappers (08, 14), and their draw callbacks (the glow, the arc) | level08 0x2d2af0, 0x2d4108, 0x2d3878 | [`hover_zapper`] |
//! | U407 | 63 flying biters (13) | level13 0x2b50d8 | [`flying_biter`] |
//! | U300 | 52 buzz bombs (09, 16) | level09 0x2c5990 | [`buzz_bomb`] |
//! | U521 | 1445 area stalkers (16) | level16 0x2e5e08 | [`area_stalker`] |
//! | U426 | 1271 the wave gate (13) | level13 0x30af50 | [`wave_gate`] |
//! | U183 | 838 Rilgar laser fences (05; one looping voice per group, G-AUD-010) | level05 0x30dc68 | [`laser_fence`] |
//! | U32 | 1413 the Veldin help director (00) | level00 0x2e0988 | [`help_veldin`] |
//! | U119 | 1324 the Aridia help director (02) | level02 0x2ee890 | [`help_aridia`] |
//! | U145 | 1342 the Kerwan help director (03) | level03 0x2df520 | [`help_kerwan`] |
//! | U165 | 1343 the Eudora help director (04) | level04 0x2e4418 | [`help_eudora`] |
//! | U204 | 1347 the Rilgar help director (05) | level05 0x31bf40 | [`help_rilgar`] |
//! | U232 | 1348 the Blarg help director (06) | level06 0x3083b0 | [`help_blarg`] |
//! | U292 | 1349 the Batalia help director (08) | level08 0x307540 | [`help_batalia`] |
//! | U305 | 1000 the Gaspar help director (09) | level09 0x300888 | [`help_gaspar`] |
//! | U341 | 1344 the Orxon help director (10) | level10 0x2e85b8 | [`help_orxon`] |
//! | U391 | 422 the Hoven help director (12) | level12 0x2ed280 | [`help_hoven`] |
//! | U419 | 558 the Gemlik help director (13) | level13 0x2f3778 | [`help_gemlik`] |
//! | U470 | 77 Quartu alarm drones (15, 17) | level15 0x2a2488 | [`quartu_drone`] |
//! | U480 | 408 Quartu alarms (15, 17), and the drones they release | level15 0x2cb4c8 | [`quartu_alarm`] |
//! | U216 | 1039 kill cuboids (06, 08, 13, 18; off while flying a ship: hero state 0x32, G-HERO-002) | level06 0x2f7930 | [`kill_volume`] |
//! | U274 | 438 Batalia's circling fighters (08; shot down from the turret in hero state 0x32) | level08 0x2de848 | [`batalia_fighter`] |
//! | U474 | 123 swinging lasers (15, 17) | level15 0x2a6e68 | [`swing_laser`] |
//! | U411 | 127, 128, 159, 169 rotators on a linked moby's state (13) | level13 0x2c7f38 | [`linked_rotator`] |
//! | U335 | 1196 Orxon's path scouts (10; wake the brawlers' groups) | level10 0x2df270 | [`orxon_flyers`] |
//! | U336 | 1199 Orxon's swoop flyers (10) | level10 0x2e01a8 | [`orxon_flyers`] |
//! | U337 | 1202 Orxon's brawlers (10; Clank's-part branches: G-HERO-005) | level10 0x2e1d38 | [`orxon_brawler`] |
//! | U323 | 22 Orxon's Clank section (10: Clank in charge without the O2 Mask, `crate::hero::bodies`) | level10 0x298b68 | [`clank_section`] |
//! | U500 | 1451 / 1899 Giant Clank's pads (15, 18: in and out of Giant Clank, `crate::hero::bodies`) | level15 0x2ed068 | [`giant_pad`] |
//! | U220 | 1061 Blarg's Clank station (06: Ratchet and Clank trade places, `crate::hero::bodies`) | level06 0x2fc640 | [`blarg_clank_lift`] |
//! | — | 0x593 Giant Clank's landing shockwave (15, 18; made by the hero code, `crate::hero::bodies::giant`) | level15 0x29ead0 | [`giant_shockwave`] |
//! | — | 0x100 Giant Clank's missiles (15, 18; made by the hero code: a copy of the Devastator missile with its own search) | level15 0x29d6c0 | [`giant_missile`] |
//! | — | 0x5f3 Giant Clank's head beam (15, 18; held by the hero code, then flies) | level15 0x29edb0 | [`giant_beam`] |
//! | U499 | 1446 Quartu's Giant Clank mission NPC (15: the talk, the two groups, out of Giant Clank, planet 16) | level15 0x2ec760 | [`quartu_giant_mission`] |
//! | U307, U316, U255 | 664, 1293 / 1320 Gaspar's sinking floats (09), 1069 Umbris' rocking floats (07): they carry Ratchet by writing his platform delta (`HeroFields::ride`) | level09 0x2f86a0, 0x3091b0, level07 0x3112c8 | [`riding_floats`] |
//! | U407 | 29 Gemlik's gun turrets (13), their rider 36 and shot 1238 (created by code) | level13 0x2b41b8, 0x2b4c80, 0x306300 | [`gemlik_turret`] |
//! | U215 | 1038 orb holders (06, 10, 17), and the orb 1040 each makes | level06 0x2f7288, 0x2f7ab8 | [`orb_holder`] |
//! | U503, U502, U514 | 552 barrier posts, 546 switches, 1387 walls (16) | level16 0x2cf4a8, 0x2cf198, 0x2e36e8 | [`kalebo_barrier`] |
//! | U185 (2026-09-29 run) | 843 sliding blocks placed by a cuboid (05) | level05 0x30e508 | [`cuboid_slider`] |
//! | U473 | 93 swing doors (15) | level15 0x2a3ba8 | [`swing_door`] |
//! | U307 | 1172 chain anchors (09) | level09 0x303d10 | [`chain_anchor`] |
//! | U477 | 196, 197, 1958 sliding doors (15, 17) | level15 0x2bddb0 | [`slide_door`] |
//! | U102, U126, U179 | 707 / 734 turntables (02), 1210 joint-carried platform (03), 812 pinned platforms (05) | level02 0x2ddc00, level03 0x2953f8, level05 0x30bf98 | [`carriers`] |
//! | U565 | 1381 falling platforms carrying the Veldin carriers (18) | level18 0x2f16f0 | [`falling_platform`] |
//! | U207, U472 | 1511 breakable light fixtures (05, 14) | level05 0x31c8e0, level14 0x307ad8 | [`light_fixture`] |
//! | U514 | 1143 Gadgetron logos (16 placed; every level's vendor hologram; a manipulator on its own list 1, `crate::moby_update::manip`) | level16 0x2e1088 | [`hologram_logo`] |
//! | U180 | 823 sweeping searchlights (05, 07; a manipulator on its head, the beam callback 0x30c220) | level05 0x30c0a8 | [`sweep_light`] |
//! | U155 | 481 bobbing floats with three spinning parts (04; look-at records `manip::look`, a platform) | level04 0x2cdda0 | [`spinner_float`] |
//! | U203 | 1139 hoverboard-course sparkles (05, 16) | level05 0x31abe0 | [`board_sparkle`] |
//! | U177 | 717 Rilgar's hoverboard racers (05; their boards are created 439s) | level05 0x307910 | [`rilgar_racer`] |
//! | U171 | 133 the hoverboard course's roaming boost pickups (05) | level05 0x2dac80 | [`board_boost`] |
//! | U503 | 556 Kalebo III's hover racers (16; Rilgar's racer code through `rilgar_racer::Layout`, shot down and back) | level16 0x2d04a0 | [`kalebo_racer`] |
//! | U174 | 439 the Hoverboard (05, 16; mounts Ratchet, its thrusters, and the hero code's stores: `crate::hero::hoverboard`) | level05 0x2f87a8 | [`hoverboard`] |
//! | U440 | 30 Oltanis pop-up turrets (14), and their shot 681 | level14 0x2b3bf0, 0x2ece00 | [`popup_turret`] |
//! | U212 | 1021 Blarg petal doors (06) | level06 0x2f4f00 | [`petal_door`] |
//! | U390 | 339 Hoven's animated idlers (12) | level12 0x2ec1d0 | [`anim_idler`] |
//! | U248 | 1013, 1014, 1064, 1065 panels on a linked moby's state (07) | level07 0x30cf90 | [`linked_slider`] |
//! | U329 | 1015, 1282 Orxon trip blocks (10) | level10 0x2d90a8 | [`trip_block`] |
//! | U162 | 1101, 1102, 1531, 1532 Eudora switched movers (04) | level04 0x2e17d8 | [`switched_mover`] |
//! | U487 | 1209 Quartu pressure pads (15, 17) | level15 0x2e5958 | [`pressure_pad`] |
//! | U211 | 911 Blarg flame jets (06) | level06 0x2f3ad8 | [`flame_jet`] |
//! | U542 | 669 the fleet's underwater laser spinners (17; only while Ratchet is in the water) | level17 0x2d77f0 | [`water_laser`] |
//! | U349 | 1544 Orxon's particle vents: puffs, drips, columns (10) | level10 0x2ea1f0 | [`orxon_vent`] |
//! | U375 | 1246 Pokitaru's biters: beach, swimmer, boat boarders (11) | level11 0x314318 | [`pokitaru_biter`] |
//! | U95, U101 | 580 Aridia's sand sharks and their nests 668 (02) | level02 0x2d3e50, 0x2dcb38 | [`aridia_sandshark`] |
//! | U96 | 612 Aridia's flame-throwing sentries (02) | level02 0x2d7748 | [`aridia_flamer`] |
//! | U373 | 1231 Pokitaru's ball throwers (11), and the ball 1297 they make | level11 0x310180, 0x318b30 | [`pokitaru_thrower`] |
//! | U128 | 75, 115–120, 132, 795 Kerwan's air traffic (03), and the exhaust trail 235 of 75 / 119 | level03 0x29dba8, 0x2bae48 | [`air_traffic`] |
//! | U126 | 868, 905, 928 Kerwan's swinging path movers (03) | level03 0x294c08 | [`kerwan_mover`] |
//! | U506 | 471 Kalebo's reversing belts (16): the group's belt push, the shared voice, two scrolling quad layers | level16 0x2c9878, draw 0x2c9cd0 | [`kalebo_belt`] |
//! | U541 | 99 the fleet's sliding laser emitters (17): beam hit line, type-79 sparks, beam quad | level17 0x2a8da0, draw 0x2a95b8 | [`fleet_laser`] |
//! | U485 | 221 Quartu's shaking critters (15): head look-at, shake, dust, death pieces | level15 0x2c2938 | [`quartu_critter`] |
//! | U567 | 1355 Veldin's divers (18; released by the boss 1422): wander, dive, crash, trails and glows | level18 0x2efb88, draws 0x2f0390 / 0x2f06c0 | [`veldin_diver`] |
//! | U162 | 617 Eudora's path riders (04) | level04 0x2d6f68 | [`eudora_path_rider`] |
//! | U180 | 810 Rilgar's rocking floats (05) | level05 0x30bdd8 | [`rilgar_rocker`] |
//! | U195 | 895 Rilgar's flaps (05) | level05 0x3166a0 | [`rilgar_flap`] |
//! | U283 | 467 / 472 Batalia's linked lifts (08) | level08 0x2ea398 | [`batalia_lift`] |
//! | U436 | 1577 Gemlik's watchers (13) | level13 0x30be60 | [`gemlik_watch`] |
//! | U158 | 484 (04): `DeleteMoby(self)` | level04 0x2ce060 | [`marker::update`] |
//! | U313 | 1206 Gaspar's hazard columns (09) | level09 0x305a28 | [`gaspar_hazard`] |
//! | U60 | 695 Novalis' floating pushables (01) | level01 0x2f8268 | [`floating_pushable`] |
//! | U256 | 1080 Umbris' sinking floats (07) | level07 0x311bc8 | [`umbris_sinker`] |
//! | U505 | 470 Kalebo's glowing beacons (16; the gold bolts' item glow) | level16 0x2c9480 | [`kalebo_glow`] |
//! | U341 | 1240 Orxon's spark fountains (10) | level10 0x2e5be0 | [`orxon_sparks`] |
//! | U263 | 104, 106, 1129 linked platforms (07, 13): `linked_mover`'s state machine at +0xa0 with a carry | level07 0x31aee0 | [`linked_mover::platform_update`] |
//! | U222 | 1054 Blarg's split doors (06) and their half 1055 | level06 0x2fbfb0 | [`blarg_doors`] |
//! | U88 | 1504 wandering point lights (01, 06) | level01 0x30b618 | [`wandering_light`] |
//! | U376 | 1248 Pokitaru's rising gates (11) and their halves 1247 | level11 0x316320 | [`pokitaru_gate`] |
//! | U194 | 893 Rilgar's hinged hatches (05) | level05 0x316258 | [`rilgar_hatch`] |
//! | U191 | 855 Rilgar's water spouts (05): type-50 droplets, type-46 rings, the group voice | level05 0x3150f0 | [`rilgar_spout`] |
//! | U438 | 1805 Gemlik's breakable props (13) | level13 0x30cdb8 | [`gemlik_breakable`] |
//! | U570 | 1750 the save before the last boss (18): `MakeWholeSave` into the ending buffer once | level18 0x2fad08 | [`ending_save`] |
//! | U252 | 1041 Umbris' lobbing turrets (07) and their shot 882 | level07 0x30d4d8, 0x30bbc8 | [`umbris_lobber`] |
//! | U118 | 1212 the path ships (02, 09): path, exhaust ([`engine_trail`], shared with [`batalia_fighter`]), glow quads | level02 0x2ec4b8, draw 0x2ec308 | [`path_ship`] |
//! | U155 | 434 Eudora's crank lifts (04): posed by a bolt crank, sink back unwinding it | level04 0x2c6bb8 | [`eudora_crank_lift`] |
//! | U154 | 432 / 1052 Eudora's crank followers (04): posed by a bolt crank, swap their own class (`class_swap`) | level04 0x2c6858 | [`eudora_crank_follower`] |
//! | U225 | 1066 Blarg's creature wakers (06): the cuboid, then one dormant member of the group thrown out every 20 ticks | level06 0x2fda30 (the wake 0x2e9d10) | [`blarg_waker`] |
//! | U140 | 899 Kerwan's path-mover lines (03) and the movers 898 they create: platforms 2.9 apart gliding along the path, carrying riders | level03 0x2db280, 0x2db198, 0x2db020 | [`kerwan_path_spawner`] |
//! | L03 825 | 825 Kerwan's turntable (03): turns at 0.48 rad/s, carrying its riders | level03 0x2d3e58 | [`kerwan_turntable`] |
//! | L03 997 | 997 Kerwan's riser (03): rises 4 units once its save bits are set | level03 0x2dca30 | [`kerwan_riser`] |
//! | L03 1548 | 1548 Kerwan's cutscene FX driver (03): the infobot's thrusters in scenes 4 / 10, smoke from actor 3 in scene 3 | level03 0x2e01d0 | [`kerwan_scene_fx`] |
//! | L03 914 | 914 Kerwan's talking bystander (03): talks before the Swingshot (a checkpoint after node 2), blown up by an explosion for skill point 5, then smokes | level03 0x2dbd90 | [`kerwan_bystander`] |
//! | L03 816, 1012 | 816 Kerwan's called platforms and 1012 its two-way shuttles (03): flown along splines, ridden with △ | level03 0x2d3198, 0x2dccc0 (the follower 0x2d3918) | [`kerwan_transport`] |
//! | L03 578, 627 | 578 Kerwan's blob layers (03) and the blobs 627 they drop: patrol a path, drop five blobs every 300 ticks; the blobs burst when stepped on or touched | level03 0x2c9eb8, 0x2cbea8 | [`kerwan_layer`] |
//! | U319 | 1885 the pod launchers (09, 13) and their pods 1886: lobbed pods bounce, rest and hatch a revived member of the launcher's group | level09 0x30ab80, 0x30b3b0, 0x30a778 | [`pod_launcher`] |
//! | U134 | 455 the pod spawners (03, 08, 14) and their pods 545: once Ratchet is near, lobbed pods bounce, rest and hatch one of the spawner's placed creatures | level03 0x2bef68, 0x2c4718, 0x2bec60 | [`pod_spawner`] |
//! | — | 1475 Kalebo III's board missile (16; created by the board weapon, item 0x24; the Devastator missile's trail) | level16 0x2a3f38 (0x2a3e30 the spawn) | [`board_missile`] |
//! | U509 | 933 Kalebo's floating mines (16): bob, spin, pulse; blown up by a hit or a touch, the placed ones back out of view | level16 0x2ddde0 | [`kalebo_mine`] |
//! | U521 | 1401 Kalebo's mine drones (16): wait in their cuboid, fly a path carrying a new race mine 933, drop it, fly back | level16 0x2e37a0 (0x2de298 the mine) | [`kalebo_mine_drone`] |
//! | U363 | 1075 Pokitaru's boats: the path, the propellers and wake, the boarders' moving area, the lift (11) | level11 0x309ac0 | [`pokitaru_boat`] |
//! | U130 | 574 Kerwan's gun troopers (03), and the rocket 833 they fire | level03 0x2c6fd0, 0x2d43c8 | [`kerwan_trooper`] |
//! | U280 | 452 Batalia's runners (08) | level08 0x2e2df0 | [`batalia_runner`] |
//! | U359 | 318 Pokitaru teleporter pads (11; the sibling of the pads 1135, `classes::teleporter`), and their beam | level11 0x2f2518, 0x2f2d58 | [`pokitaru_teleporter`] |
//! | — | 787 the cave drips the ripple manager 751 spawns (01) | level01 0x2ffdc0 (spawner 0x2ffcd0) | [`drip`] |
//! | U50 | 613 Novalis's water currents (01) | level01 0x2f3120 | [`water_current`] |
//! | U129 | 573 Kerwan's charging creatures (03): wait, charge, bite, run a path, wait by a trooper | level03 0x2c5bb8 | [`kerwan_hound`] |
//! | U353, U350 | 114 Pokitaru's commando (11): talks, follows Ratchet through four phases, starts and rides the boats 1075, gives the O2 Mask; and the gate 65 he opens | level11 0x2d0fa8, 0x2cb810 | [`pokitaru_commando`] |
//! | U363 | 1157 Pokitaru's cutaway machine (11): its 24 slats 1207 open in a cutaway (fades, Ratchet held at the switch 830, the script camera's glide) | level11 0x30d800, 0x30dd18 | [`pokitaru_cutaway`] |
//! | U564 | 1422 the boss of Veldin's last arena (18): every state, its phases, damage, the arena camera record, the boss meter (HUD slot 6), the scenes, the death and the finale | level18 0x2f2bf0 (draw 0x2f7880) | [`veldin_boss`] |
//! | U557 | 644 the arena's cutaway camera (18), started by the boss | level18 0x2df608, 0x2dfaa0 | [`veldin_cutaway`] |
//! | U556 | 587 the arena's floating platforms (18): bob, carry, sink by the difficulty word 0x1623a8 | level18 0x2d82c0 | [`veldin_floater`] |
//! | U572 | 1906 the boss's hoppers (18), woken by the boss | level18 0x2fbb98 | [`veldin_hopper`] |
//! | U554, U555 | 583 the boss's pads and 586 the button / countdown (18) | level18 0x2d6600, 0x2d7b20 (countdown draw 0x2d8098) | [`veldin_pads`] |
//! | — | 564 / 624 / 628 / 983 / 1898 the boss's shots (18, created only by 1422 / 1906): shell, ring, aura, beam, flash | level18 0x2d5050, 0x2db460, 0x2dc260, 0x2e9e70, 0x2fb548 | [`veldin_shots`] |
//! | U123, U124 | 822 Kerwan's train (03): the locomotive pulling the cars 1210 behind its lead 845 (the flyer driver), the ride, the arrival, the infobot's release | level03 0x292e98, 0x293c78 | [`kerwan_train`] |
//! | U114, U115 | 786 Aridia's surfer (the Sonic Summoner) and 788 his agent (the Hoverboard; the shark-count race) (02) | level02 0x2e0dc0, 0x2e1950 | [`aridia_story`] |
//! | U142, U145 | 890 Helga (the Swingshot) and 909 the Heli-Pack giver (03) | level03 0x2da870, 0x2db558 | [`kerwan_story`] |
//! | U169, U170 | 1120 the Suck Cannon pickup and 1190 the Blarg informant (planet 6) (04) | level04 0x2e1a10, 0x2e3078 | [`eudora_story`] |
//! | U200, U201, U203 | 918 the race girl (flag 0), 919 the bouncer (planet 7), 925 the R.Y.N.O. salesman (05) | level05 0x316ab8, 0x317470, 0x3180a0 | [`rilgar_story`] |
//! | U233 | 1105 Blarg's scientist (the Grindboots) (06) | level06 0x301070 | [`blarg_story`] |
//! | U297, U298, U299 | 1130 the commando (planet 10), 1144 the deserter (planet 9), 1283 the turret host (the Metal Detector) (08) | level08 0x302ce8, 0x305270, 0x3065d8 | [`batalia_story`] |
//! | U321 | 1290 the Pilot's Helmet pickup (09; the code is in every overlay) | level01 0x30a6d0 | [`gaspar_story`] |
//! | U329, U350 | 18 the Magneboots pickup and 1326 the Nanotech seller (G-SAV-005) (10) | level10 0x298668, 0x2e8358 | [`orxon_story`] |
//! | U360, U363, U365 | 23 the O2 Mask prop, 90 the Thruster-Pack giver, 298 the Persuader giver (11) | level11 0x2cb668, 0x2d0710, 0x2f0e40 | [`pokitaru_story`] |
//! | U394, U398, U411 | 282 (flag 1, flag 0x61), 328 the Hydro-Pack giver, 1404 the scene triggers (12) | level12 0x2e6c48, 0x2eb570 | [`hoven_story`] |
//! | U440 | 1353 Gemlik's story director (the arrival, Qwark's ambush, planet 14) (13) | level13 0x30b628 | [`gemlik_story`] |
//! | U464, U469, U474 | 851 Qwark (the PDA), 924 the scrap merchant (planet 15), 1354 the Morph-o-Ray (14) | level14 0x2fba20, 0x2fefe0, 0x305758 | [`oltanis_story`] |
//! | U503, U506, U511 | 1388 the Bolt Grabber, 1419 the broadcast director (planet 17), 1469 the help director (15) | level15 0x2ea748, 0x2eb4c0 | [`quartu_story`] |
//! | U529 | 1377 the Map-O-Matic giver (flags 0x70 / 0x71) (16) | level16 0x2e3190 | [`kalebo_story`] |
//! | U561 | 1428 the fleet's item scene (flags 2 / 0x78) (17) | level17 0x2f1790 | [`fleet_story`] |
//! | U31 | 834 Veldin's Clank (flag 8, the trip to Novalis) (00) | level00 0x2d9dc8 | [`veldin_story`] |
//! | U24 | 530 Ratchet's ship on Veldin (00; hidden behind the scenes' own ship, its canopy glass) | level00 0x2d1e80 | [`veldin_ship`] |
//! | U36, U37 | 1440 Veldin's beam drones (00; fly in, fire a crackling beam, two hits) and 1471 their beam manager (the three beam slots, the strands, sparks and draw) | level00 0x2e0b88, 0x2e1df0 | [`veldin_beamer`] |
//! | U38 | 1545 Veldin's cutscene FX driver (00; the infobot's thrusters, scene 4's dust) | level00 0x2e3800 | [`veldin_scene_fx`] |
//! | U597 | 1434 / 1435 the pieces that turn over at the boss's checkpoint (18) | level18 0x2f7ad8 | [`veldin_turnover`] |
//! | U602 | 1799 the boss's jet in the scenes of Veldin's last level (18) | level18 0x2fad28 | [`veldin_scene_jet`] |
//! | U593 | 1392 the fields beside the Trespasser locks of Veldin's last level (18; a hum and pulsing bands until the lock is solved) | level18 0x2f1e68, draw 0x2f1fd8 | [`veldin_lock_field`] |
//! | U594 | 1402 the Hydrodisplacer pools of Veldin's last level (18; two heights, their groups carried, the surface meshes) | level18 0x2f2310, draws 0x2f2970 / 0x2f2620 / 0x2f27c8 | [`veldin_pool`] |
//! | U519 | 1443 (16) / 1890 (18) the energy fans (blades and rings until their switch group is thrown) | level18 0x2fae48, draw 0x2fb018 (level16 0x2e5708) | [`energy_fan`] |
//! | U599 | 1563 the cutscene effects of Veldin's last level (18; the seat glows, the jets and blasts of scene 4, the piece's glow and beam, the flash, the puffs) | level18 0x2f88e8, draws 0x2f9780 / 0x2f9a98 / 0x2f9c20 / 0x2f9eb8, the Morph-o-Ray beam 0x2c04b8 | [`veldin_finale_fx`] |
//! | U598 | 1454 the tanks of Veldin's last level (18; a path, a turret, shells 41 that may home, treads 331) | level18 0x2f7c40, 0x2a7220, 0x2ce7d0 | [`veldin_tank`] |
//! | U511 | 1356 the dropships (16, 18; the approach with the troopers, the drop, the exit, the homing shots 50) | level18 0x2f0920, 0x2a7b90 | [`dropship`] |
//! | U504 | 638 the hover troopers (16, 18; patrols, bursts of shots 49, the dropship's passengers) | level18 0x2dc918, 0x2a76e0 | [`drop_trooper`] |
//! | — | 1510 the burning wreck of a flyer 660 / gunship 688 (01; created only) | level01 0x30ba18, draw 0x30bfa8 | [`super::burning_wreck`] |
//! | U581 | 582 the rail chooser of Veldin's last level (18; the twice-laid rails, one way live at a time; two nanotech clusters) | level18 0x2d62e8 | [`veldin_rails`] |
//! | U248 | 436 Umbris' story director (the lair, planet 8, the trip to Batalia) (07) | level07 0x2f5ba0 | [`umbris_story`] |
//! | U118 | 1005 / 1016 the item scenes: the Trespasser (02), the Hydrodisplacer (06) | level02 0x2ea210 | [`aridia_story`] |
//! | U237 | 1109 Blarg's shuttle (the station's routes, the last ride's blast, the infobot hand-off, planet 5) (06) | level06 0x302578 | [`blarg_shuttle`] |
//! | U405 | 1267 Hoven's turret mini-game (Gemlik's unlock, planet 13), its HUD 0x304218 and red screen 0x304d98 (12) | level12 0x303540 | [`hoven_turret`] |
//! | U424 | 69 Gemlik's ship (the base battle: the flight, the guns, the missiles, the vehicle record), its HUD 0x2b97f8 (the lock, the targets left, the gauge) (13) | level13 0x2bb068 | [`gemlik_ship`], [`gemlik_ship_hud`] |
//! | U434 | 388 Qwark's ship (the Gemlik base battle's boss: paths, phases, tractor beam, shield, taunts), its parts 389..401, shield 352, missile 82 and mine 83 (13) | level13 0x2eb098 | [`qwark_ship`], [`qwark_ship_parts`] |
//! | — | 458 the ridden turrets' shell (08, 12; created by code: the spawner 0x2ef2e8) | level12 0x2ef648 | [`turret_shell`] |
//! | U407 | 1274 Hoven's carrier (the turret game's target: 17 parts, five guns, the fall, scene 7) (12) | level12 0x3069d0 | [`hoven_carrier`] |
//! | — | 184 the gun shot (04, 12; created by code: the spawner 0x2d4150) | level12 0x2d4378 | [`gun_shot`] |
//! | — | 1009 the ships' laser shot (11, 13, 17; created by code: the spawner 0x3025f8, the search 0x302420) | level13 0x302780 | [`ship_laser`] |
//! | — | 295 the ships' homing missile (13, 17; created by code: the spawner 0x2e6a58) | level13 0x2e6c08 | [`ship_missile`] |
//! | — | 1034 its earlier copy (11, 13, 17; created by code: the spawner level11 0x309378) | level11 0x3094f8 | [`ship_missile`] |
//! | U384 | 1242 Pokitaru's jet (the convoy mission: the flight, the guns, the missiles, the edge bend, the ambush calls), its HUD 0x311d50 (the convoys left, the lock, the gauge) (11) | level11 0x313290 | [`pokitaru_jet`], [`pokitaru_jet_hud`] |
//! | U387 | 1264 Pokitaru's convoys (the jet mission's targets), their cars 1265 and the sludge 1524 they drop (11) | level11 0x3172c0, 0x3181f0, 0x31ab08 | [`pokitaru_convoy`] |
//! | U389 | 1319 Pokitaru's fighters (the convoys' escorts and the jet's ambushers: paths, attack runs, shots, pickups), their contrails 0x3192c8 (11) | level11 0x319838 | [`ship_fighter`] |
//! | U576 | 1843 the fleet's fighters (the same code, the floating pickups), their contrails 0x2f3b68 (17) | level17 0x2f40d8 | [`ship_fighter`] |
//! | U433 | 224 / 228 the floating ship pickups (13: 8 + 8; 17: dropped by 1843) | level13 0x2e1cc8 | [`ship_pickup_float`] |
//! | U568 | 1379 the fleet's ship (Gemlik's flight with the edge bend; the turrets' mission), its HUD 0x2eb9a8 (17) | level17 0x2ed018 | [`fleet_ship`], [`fleet_ship_hud`] |
//! | U563 | 347 the fleet's turrets (the ship mission's targets) and their bolt 1368 (17) | level17 0x2cb310, 0x2e8e08 | [`fleet_turret`] |
//! | — | 1218 / 1220 the ship pickups (missiles / health) and their parachute 1219 (11; created by code: the spawner 0x30f5a8) | level11 0x30f728, 0x310028 | [`ship_pickup`] |
//! | — | 1017 the fighters' laser shot (11, 13, 17; created by code: the spawner level11 0x308f48) | level11 0x309098 | [`fighter_shot`] |
//! | U397 | 326 Hoven's gun drones (ordinary and the turret game's attackers) and their shot 409 (12) | level12 0x2e9f68, 0x2ece00 | [`hoven_drone`] |
//! | — | 1371 the drones' rider (12, 15; created by code: 0x308670, knocked off by 0x308708) | level12 0x308918 | [`drone_rider`] |
//! | U538 | 1455 Kalebo III's hoverboard-race host (the Hologuise, skill point 0x13d422; the race itself: G-LVL-007) (16) | level16 0x2e6808 | [`kalebo_race`] |

use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

pub mod asteroid;
pub mod chain_link;
pub mod barricade;
pub mod tethered_platform;
pub mod explosive_tank;
pub mod fleet_door;
pub mod linked_cog;
pub mod quartu_belt;
pub mod rising_float;
pub mod extending_piece;
pub mod veldin_carrier;
pub mod hidden_prop;
pub mod bubble_vent;
pub mod oltanis_switchboard;
pub mod empty;
pub mod lamp;
pub mod loose_piece;
pub mod conveyor;
pub mod timed_switch;
pub mod linked_mover;
pub mod vent;
pub mod grind_mine;
pub mod rising_block;
pub mod bob_block;
pub mod blarg_shuttle;
pub mod marker;
pub mod smoke_emitter;
pub mod hydro_pad;
pub mod trespasser_lock;
pub mod lock_doors;
pub mod slider;
pub mod help_director;
pub mod path_glider;
pub mod rail_car;
pub mod kalebo_traffic;
pub mod horny_toad;
pub mod hop_gunner;
pub mod laser_fence;
pub mod help_veldin;
pub mod help_aridia;
pub mod help_kerwan;
pub mod help_eudora;
pub mod help_rilgar;
pub mod help_blarg;
pub mod help_batalia;
pub mod help_gaspar;
pub mod help_orxon;
pub mod help_hoven;
pub mod help_gemlik;
pub mod hints;
pub mod rolling_mine;
pub mod pack_biter;
pub mod hover_zapper;
pub mod flying_biter;
pub mod buzz_bomb;
pub mod area_stalker;
pub mod wave_gate;
pub mod quartu_drone;
pub mod quartu_alarm;
pub mod kill_volume;
pub mod batalia_fighter;
pub mod swing_laser;
pub mod linked_rotator;
pub mod orxon_flyers;
pub mod orxon_brawler;
pub mod gemlik_turret;
pub mod orb_holder;
pub mod kalebo_barrier;
pub mod cuboid_slider;
pub mod swing_door;
pub mod chain_anchor;
pub mod slide_door;
pub mod carriers;
pub mod falling_platform;
pub mod light_fixture;
pub mod hologram_logo;
pub mod sweep_light;
pub mod spinner_float;
pub mod board_sparkle;
pub mod hoverboard;
pub mod board_boost;
pub mod kalebo_racer;
pub mod rilgar_racer;
pub mod popup_turret;
pub mod petal_door;
pub mod anim_idler;
pub mod linked_slider;
pub mod trip_block;
pub mod switched_mover;
pub mod pressure_pad;
pub mod flame_jet;
pub mod water_laser;
pub mod orxon_vent;
pub mod pokitaru_biter;
pub mod aridia_sandshark;
pub mod aridia_flamer;
pub mod pokitaru_thrower;
pub mod air_traffic;
pub mod kerwan_mover;
pub mod pokitaru_boat;
pub mod kerwan_trooper;
pub mod batalia_runner;
pub mod kalebo_belt;
pub mod fleet_laser;
pub mod quartu_critter;
pub mod veldin_diver;
pub mod eudora_path_rider;
pub mod rilgar_rocker;
pub mod rilgar_flap;
pub mod batalia_lift;
pub mod gemlik_watch;
pub mod gaspar_hazard;
pub mod floating_pushable;
pub mod umbris_sinker;
pub mod kalebo_glow;
pub mod orxon_sparks;
pub mod blarg_doors;
pub mod wandering_light;
pub mod pokitaru_gate;
pub mod rilgar_hatch;
pub mod rilgar_spout;
pub mod gemlik_breakable;
pub mod ending_save;
pub mod umbris_lobber;
pub mod engine_trail;
pub mod path_ship;
pub mod clank_section;
pub mod giant_pad;
pub mod blarg_clank_lift;
pub mod giant_shockwave;
pub mod giant_missile;
pub mod giant_beam;
pub mod quartu_giant_mission;
pub mod riding_floats;
pub mod eudora_crank_lift;
pub mod eudora_crank_follower;
pub mod blarg_waker;
pub mod kerwan_path_spawner;
pub mod kerwan_turntable;
pub mod kerwan_riser;
pub mod kerwan_scene_fx;
pub mod kerwan_bystander;
pub mod kerwan_transport;
pub mod kerwan_layer;
pub mod pod_launcher;
pub mod pod_spawner;
pub mod kalebo_mine_drone;
pub mod kalebo_mine;
pub mod board_missile;
pub mod veldin_finale_fx;
pub mod veldin_lock_field;
pub mod veldin_pool;
pub mod veldin_rails;
pub mod drop_trooper;
pub mod dropship;
pub mod energy_fan;
pub mod veldin_scene_jet;
pub mod veldin_ship;
pub mod veldin_tank;
pub mod veldin_turnover;
pub mod veldin_scene_fx;
pub mod veldin_beamer;
pub mod pokitaru_teleporter;
pub mod drip;
pub mod water_current;
pub mod kerwan_hound;
pub mod pokitaru_commando;
pub mod pokitaru_cutaway;
pub mod veldin_boss;
pub mod veldin_cutaway;
pub mod veldin_floater;
pub mod veldin_hopper;
pub mod veldin_pads;
pub mod veldin_shots;
pub mod kerwan_train;
pub mod story_npc;
pub mod aridia_story;
pub mod kerwan_story;
pub mod eudora_story;
pub mod rilgar_story;
pub mod blarg_story;
pub mod batalia_story;
pub mod gaspar_story;
pub mod orxon_story;
pub mod pokitaru_story;
pub mod hoven_story;
pub mod gemlik_story;
pub mod oltanis_story;
pub mod quartu_story;
pub mod kalebo_story;
pub mod fleet_story;
pub mod veldin_story;
pub mod umbris_story;
pub mod hoven_turret;
pub mod gemlik_ship;
pub mod gemlik_ship_hud;
pub mod qwark_ship;
pub mod qwark_ship_parts;
pub mod turret_shell;
pub mod hoven_carrier;
pub mod gun_shot;
pub mod ship_laser;
pub mod ship_missile;
pub mod fighter_shot;
pub mod ship_pickup;
pub mod ship_pickup_float;
pub mod fleet_turret;
pub mod fleet_ship;
pub mod fleet_ship_hud;
pub mod ship_fighter;
pub mod pokitaru_convoy;
pub mod pokitaru_jet;
pub mod pokitaru_jet_hud;
pub mod hoven_drone;
pub mod drone_rider;
pub mod kalebo_race;

/// One unit's port.
#[derive(Clone, Copy, Debug)]
pub struct UnitPort {
    /// The census unit id (see docs/plan/class_census.md).
    pub unit: &'static str,
    /// The level whose overlay holds [`UnitPort::func`].
    pub level: u32,
    pub func: u32,
    /// The classes the reference level's table runs it for.
    pub classes: &'static [i16],
    pub update: fn(&mut World, MobyId),
    /// The classes whose joint points the port reads (`FUN_002645a8`): the loader fills their joint lists
    /// (`LevelPorts::needs_joint_lists`).
    pub joints: &'static [i16],
}

pub const PORTS: &[UnitPort] = &[
    UnitPort { unit: "U408", level: asteroid::REFERENCE_LEVEL, func: asteroid::UPDATE_FN, classes: &asteroid::CLASSES, update: asteroid::update, joints: &[] },
    UnitPort { unit: "U303", level: chain_link::REFERENCE_LEVEL, func: chain_link::UPDATE_FN, classes: &chain_link::CLASSES, update: chain_link::update, joints: &chain_link::JOINTS },
    UnitPort { unit: "U553", level: barricade::REFERENCE_LEVEL, func: barricade::UPDATE_FN, classes: &barricade::CLASSES, update: barricade::update, joints: &[] },
    UnitPort { unit: "U294", level: tethered_platform::REFERENCE_LEVEL, func: tethered_platform::UPDATE_FN, classes: &tethered_platform::CLASSES, update: tethered_platform::update, joints: &tethered_platform::JOINTS },
    UnitPort { unit: "U417", level: explosive_tank::REFERENCE_LEVEL, func: explosive_tank::UPDATE_FN, classes: &explosive_tank::CLASSES, update: explosive_tank::update, joints: &[] },
    UnitPort { unit: "U417 fireball", level: explosive_tank::REFERENCE_LEVEL, func: explosive_tank::FIREBALL_FN, classes: &explosive_tank::FIREBALL_CLASSES, update: explosive_tank::fireball_update, joints: &[] },
    UnitPort { unit: "U533", level: fleet_door::REFERENCE_LEVEL, func: fleet_door::UPDATE_FN, classes: &fleet_door::CLASSES, update: fleet_door::update, joints: &[] },
    UnitPort { unit: "U477", level: linked_cog::REFERENCE_LEVEL, func: linked_cog::UPDATE_FN, classes: &linked_cog::CLASSES, update: linked_cog::update, joints: &[] },
    UnitPort { unit: "U479", level: quartu_belt::REFERENCE_LEVEL, func: quartu_belt::UPDATE_FN, classes: &quartu_belt::CLASSES, update: quartu_belt::update, joints: &[] },
    UnitPort { unit: "U500", level: rising_float::REFERENCE_LEVEL, func: rising_float::UPDATE_FN, classes: &rising_float::CLASSES, update: rising_float::update, joints: &[] },
    UnitPort { unit: "U499", level: extending_piece::REFERENCE_LEVEL, func: extending_piece::UPDATE_FN, classes: &extending_piece::CLASSES, update: extending_piece::update, joints: &[] },
    UnitPort { unit: "U563", level: veldin_carrier::REFERENCE_LEVEL, func: veldin_carrier::UPDATE_FN, classes: &veldin_carrier::CLASSES, update: veldin_carrier::update, joints: &[] },
    UnitPort { unit: "U559", level: hidden_prop::REFERENCE_LEVEL, func: hidden_prop::UPDATE_FN, classes: &hidden_prop::CLASSES, update: hidden_prop::update, joints: &[] },
    UnitPort { unit: "U484", level: bubble_vent::REFERENCE_LEVEL, func: bubble_vent::UPDATE_FN, classes: &bubble_vent::CLASSES, update: bubble_vent::update, joints: &[] },
    UnitPort { unit: "U456", level: oltanis_switchboard::REFERENCE_LEVEL, func: oltanis_switchboard::UPDATE_FN, classes: &oltanis_switchboard::CLASSES, update: oltanis_switchboard::update, joints: &[] },
    // Level16 0x2cb600 is `DeleteMoby(self)`: the same effect as the markers' update (`marker`, U139), separate code.
    UnitPort { unit: "U493", level: 16, func: 0x2c_b600, classes: &[482], update: marker::update, joints: &[] },
    UnitPort { unit: "U99", level: empty::REFERENCE_LEVEL, func: empty::UPDATE_FN, classes: &empty::CLASSES, update: empty::update, joints: &[] },
    UnitPort { unit: "U27", level: lamp::REFERENCE_LEVEL, func: lamp::UPDATE_FN, classes: &lamp::CLASSES, update: lamp::update, joints: &[] },
    UnitPort { unit: "U268", level: loose_piece::REFERENCE_LEVEL, func: loose_piece::UPDATE_FN, classes: &loose_piece::CLASSES, update: loose_piece::update, joints: &loose_piece::CLASSES },
    UnitPort { unit: "U95", level: conveyor::REFERENCE_LEVEL, func: conveyor::UPDATE_FN, classes: &conveyor::CLASSES, update: conveyor::update, joints: &[] },
    UnitPort { unit: "U241", level: timed_switch::REFERENCE_LEVEL, func: timed_switch::UPDATE_FN, classes: &timed_switch::CLASSES, update: timed_switch::update, joints: &[] },
    UnitPort { unit: "U247", level: linked_mover::REFERENCE_LEVEL, func: linked_mover::UPDATE_FN, classes: &linked_mover::CLASSES, update: linked_mover::update, joints: &[] },
    UnitPort { unit: "U229", level: vent::REFERENCE_LEVEL, func: vent::UPDATE_FN, classes: &vent::CLASSES, update: vent::update, joints: &[] },
    UnitPort { unit: "U280", level: grind_mine::REFERENCE_LEVEL, func: grind_mine::UPDATE_FN, classes: &grind_mine::CLASSES, update: grind_mine::update, joints: &[] },
    UnitPort { unit: "U185", level: rising_block::REFERENCE_LEVEL, func: rising_block::UPDATE_FN, classes: &rising_block::CLASSES, update: rising_block::update, joints: &[] },
    UnitPort { unit: "U221", level: bob_block::REFERENCE_LEVEL, func: bob_block::UPDATE_FN, classes: &bob_block::CLASSES, update: bob_block::update, joints: &[] },
    UnitPort { unit: "U139", level: marker::REFERENCE_LEVEL, func: marker::UPDATE_FN, classes: &marker::CLASSES, update: marker::update, joints: &[] },
    UnitPort { unit: "U281", level: smoke_emitter::REFERENCE_LEVEL, func: smoke_emitter::UPDATE_FN, classes: &smoke_emitter::CLASSES, update: smoke_emitter::update, joints: &[] },
    UnitPort { unit: "U170", level: hydro_pad::REFERENCE_LEVEL, func: hydro_pad::UPDATE_FN, classes: &hydro_pad::CLASSES, update: hydro_pad::update, joints: &[] },
    UnitPort { unit: "U96", level: trespasser_lock::REFERENCE_LEVEL, func: trespasser_lock::UPDATE_FN, classes: &trespasser_lock::CLASSES, update: trespasser_lock::update, joints: &trespasser_lock::CLASSES },
    UnitPort { unit: "U107", level: lock_doors::REFERENCE_LEVEL, func: lock_doors::UPDATE_743, classes: &lock_doors::CLASSES_743, update: lock_doors::update_743, joints: &[] },
    UnitPort { unit: "U108", level: lock_doors::REFERENCE_LEVEL, func: lock_doors::UPDATE_744, classes: &lock_doors::CLASSES_744, update: lock_doors::update_744, joints: &[] },
    UnitPort { unit: "U365", level: lock_doors::LEVEL_1159, func: lock_doors::UPDATE_1159, classes: &lock_doors::CLASSES_1159, update: lock_doors::update_1159, joints: &[] },
    UnitPort { unit: "U204", level: slider::REFERENCE_LEVEL, func: slider::UPDATE_FN, classes: &slider::CLASSES, update: slider::update, joints: &[] },
    UnitPort { unit: "U82", level: help_director::REFERENCE_LEVEL, func: help_director::UPDATE_FN, classes: &help_director::CLASSES, update: help_director::update, joints: &[] },
    UnitPort { unit: "U36", level: path_glider::REFERENCE_LEVEL, func: path_glider::UPDATE_FN, classes: &path_glider::CLASSES, update: path_glider::update, joints: &[] },
    UnitPort { unit: "U495", level: rail_car::REFERENCE_LEVEL, func: rail_car::UPDATE_FN, classes: &rail_car::CLASSES, update: rail_car::update, joints: &[] },
    UnitPort { unit: "U523", level: kalebo_traffic::REFERENCE_LEVEL, func: kalebo_traffic::UPDATE_FN, classes: &kalebo_traffic::CLASSES, update: kalebo_traffic::update, joints: &[] },
    UnitPort { unit: "U25", level: horny_toad::REFERENCE_LEVEL, func: horny_toad::UPDATE_FN, classes: &horny_toad::CLASSES, update: horny_toad::update, joints: &horny_toad::JOINTS },
    UnitPort { unit: "U287", level: hop_gunner::REFERENCE_LEVEL, func: hop_gunner::UPDATE_FN, classes: &hop_gunner::CLASSES, update: hop_gunner::update, joints: &hop_gunner::JOINTS },
    UnitPort { unit: "U287 shot", level: hop_gunner::REFERENCE_LEVEL, func: hop_gunner::SHOT_FN, classes: &hop_gunner::SHOT_CLASSES, update: hop_gunner::shot_update, joints: &[] },
    UnitPort { unit: "U553 mine", level: rolling_mine::REFERENCE_LEVEL, func: rolling_mine::UPDATE_FN, classes: &rolling_mine::CLASSES, update: rolling_mine::update, joints: &[] },
    UnitPort { unit: "U301", level: pack_biter::REFERENCE_LEVEL, func: pack_biter::UPDATE_FN, classes: &pack_biter::CLASSES, update: pack_biter::update, joints: &[] },
    UnitPort { unit: "U268 252", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::UPDATE_FN, classes: &hover_zapper::CLASSES, update: hover_zapper::update, joints: &hover_zapper::CLASSES },
    // Draw callbacks only (no class runs them: `Callback::UnitGlow` / `Callback::UnitQuads` payloads).
    UnitPort { unit: "U268 252 glow", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U268 252 arc", level: hover_zapper::REFERENCE_LEVEL, func: hover_zapper::ARC_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U407 63", level: flying_biter::REFERENCE_LEVEL, func: flying_biter::UPDATE_FN, classes: &flying_biter::CLASSES, update: flying_biter::update, joints: &[] },
    UnitPort { unit: "U300 52", level: buzz_bomb::REFERENCE_LEVEL, func: buzz_bomb::UPDATE_FN, classes: &buzz_bomb::CLASSES, update: buzz_bomb::update, joints: &[] },
    UnitPort { unit: "U521 1445", level: area_stalker::REFERENCE_LEVEL, func: area_stalker::UPDATE_FN, classes: &area_stalker::CLASSES, update: area_stalker::update, joints: &area_stalker::CLASSES },
    UnitPort { unit: "U426 1271", level: wave_gate::REFERENCE_LEVEL, func: wave_gate::UPDATE_FN, classes: &wave_gate::CLASSES, update: wave_gate::update, joints: &[] },
    UnitPort { unit: "U183", level: laser_fence::REFERENCE_LEVEL, func: laser_fence::UPDATE_FN, classes: &laser_fence::CLASSES, update: laser_fence::update, joints: &[] },
    UnitPort { unit: "U32 help", level: help_veldin::REFERENCE_LEVEL, func: help_veldin::UPDATE_FN, classes: &help_veldin::CLASSES, update: help_veldin::update, joints: &[] },
    UnitPort { unit: "U119 help", level: help_aridia::REFERENCE_LEVEL, func: help_aridia::UPDATE_FN, classes: &help_aridia::CLASSES, update: help_aridia::update, joints: &[] },
    UnitPort { unit: "U145 help", level: help_kerwan::REFERENCE_LEVEL, func: help_kerwan::UPDATE_FN, classes: &help_kerwan::CLASSES, update: help_kerwan::update, joints: &[] },
    UnitPort { unit: "U165 help", level: help_eudora::REFERENCE_LEVEL, func: help_eudora::UPDATE_FN, classes: &help_eudora::CLASSES, update: help_eudora::update, joints: &[] },
    UnitPort { unit: "U204 help", level: help_rilgar::REFERENCE_LEVEL, func: help_rilgar::UPDATE_FN, classes: &help_rilgar::CLASSES, update: help_rilgar::update, joints: &[] },
    UnitPort { unit: "U232 help", level: help_blarg::REFERENCE_LEVEL, func: help_blarg::UPDATE_FN, classes: &help_blarg::CLASSES, update: help_blarg::update, joints: &[] },
    UnitPort { unit: "U292 help", level: help_batalia::REFERENCE_LEVEL, func: help_batalia::UPDATE_FN, classes: &help_batalia::CLASSES, update: help_batalia::update, joints: &[] },
    UnitPort { unit: "U305 help", level: help_gaspar::REFERENCE_LEVEL, func: help_gaspar::UPDATE_FN, classes: &help_gaspar::CLASSES, update: help_gaspar::update, joints: &[] },
    UnitPort { unit: "U341 help", level: help_orxon::REFERENCE_LEVEL, func: help_orxon::UPDATE_FN, classes: &help_orxon::CLASSES, update: help_orxon::update, joints: &[] },
    UnitPort { unit: "U391 help", level: help_hoven::REFERENCE_LEVEL, func: help_hoven::UPDATE_FN, classes: &help_hoven::CLASSES, update: help_hoven::update, joints: &[] },
    UnitPort { unit: "U419 help", level: help_gemlik::REFERENCE_LEVEL, func: help_gemlik::UPDATE_FN, classes: &help_gemlik::CLASSES, update: help_gemlik::update, joints: &[] },
    UnitPort { unit: "U470 77", level: quartu_drone::REFERENCE_LEVEL, func: quartu_drone::UPDATE_FN, classes: &quartu_drone::CLASSES, update: quartu_drone::update, joints: &[] },
    UnitPort { unit: "U480 408", level: quartu_alarm::REFERENCE_LEVEL, func: quartu_alarm::UPDATE_FN, classes: &quartu_alarm::CLASSES, update: quartu_alarm::update, joints: &[] },
    UnitPort { unit: "U216 1039", level: kill_volume::REFERENCE_LEVEL, func: kill_volume::UPDATE_FN, classes: &kill_volume::CLASSES, update: kill_volume::update, joints: &[] },
    UnitPort { unit: "U274 438", level: batalia_fighter::REFERENCE_LEVEL, func: batalia_fighter::UPDATE_FN, classes: &batalia_fighter::CLASSES, update: batalia_fighter::update, joints: &[] },
    UnitPort { unit: "U474 123", level: swing_laser::REFERENCE_LEVEL, func: swing_laser::UPDATE_FN, classes: &swing_laser::CLASSES, update: swing_laser::update, joints: &[] },
    UnitPort { unit: "U411 127", level: linked_rotator::REFERENCE_LEVEL, func: linked_rotator::UPDATE_FN, classes: &linked_rotator::CLASSES, update: linked_rotator::update, joints: &[] },
    UnitPort { unit: "U335 1196", level: orxon_flyers::REFERENCE_LEVEL, func: orxon_flyers::SCOUT_FN, classes: &orxon_flyers::SCOUT_CLASSES, update: orxon_flyers::update_1196, joints: &[] },
    UnitPort { unit: "U336 1199", level: orxon_flyers::REFERENCE_LEVEL, func: orxon_flyers::SWOOP_FN, classes: &orxon_flyers::SWOOP_CLASSES, update: orxon_flyers::update_1199, joints: &orxon_flyers::SWOOP_CLASSES },
    UnitPort { unit: "U337 1202", level: orxon_brawler::REFERENCE_LEVEL, func: orxon_brawler::UPDATE_FN, classes: &orxon_brawler::CLASSES, update: orxon_brawler::update, joints: &orxon_brawler::CLASSES },
    UnitPort { unit: "U407 29", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::UPDATE_FN, classes: &gemlik_turret::CLASSES, update: gemlik_turret::update, joints: &gemlik_turret::CLASSES },
    UnitPort { unit: "U407 36", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::RIDER_FN, classes: &gemlik_turret::RIDER_CLASSES, update: gemlik_turret::rider_update, joints: &gemlik_turret::RIDER_CLASSES },
    UnitPort { unit: "U407 1238", level: gemlik_turret::REFERENCE_LEVEL, func: gemlik_turret::SHOT_FN, classes: &gemlik_turret::SHOT_CLASSES, update: gemlik_turret::shot_update, joints: &[] },
    UnitPort { unit: "U215 1038", level: orb_holder::REFERENCE_LEVEL, func: orb_holder::UPDATE_FN, classes: &orb_holder::CLASSES, update: orb_holder::update, joints: &[] },
    UnitPort { unit: "U215 orb", level: orb_holder::REFERENCE_LEVEL, func: orb_holder::ORB_FN, classes: &orb_holder::ORB_CLASSES, update: orb_holder::orb_update, joints: &[] },
    UnitPort { unit: "U503 552", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::UPDATE_FN, classes: &kalebo_barrier::CLASSES, update: kalebo_barrier::post_update, joints: &[] },
    UnitPort { unit: "U502 546", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::SWITCH_FN, classes: &kalebo_barrier::SWITCH_CLASSES, update: kalebo_barrier::switch_update, joints: &[] },
    UnitPort { unit: "U514 1387", level: kalebo_barrier::REFERENCE_LEVEL, func: kalebo_barrier::WALL_FN, classes: &kalebo_barrier::WALL_CLASSES, update: kalebo_barrier::wall_update, joints: &[] },
    UnitPort { unit: "U185 843", level: cuboid_slider::REFERENCE_LEVEL, func: cuboid_slider::UPDATE_FN, classes: &cuboid_slider::CLASSES, update: cuboid_slider::update, joints: &[] },
    UnitPort { unit: "U473 93", level: swing_door::REFERENCE_LEVEL, func: swing_door::UPDATE_FN, classes: &swing_door::CLASSES, update: swing_door::update, joints: &[] },
    UnitPort { unit: "U307 1172", level: chain_anchor::REFERENCE_LEVEL, func: chain_anchor::UPDATE_FN, classes: &chain_anchor::CLASSES, update: chain_anchor::update, joints: &[] },
    UnitPort { unit: "U477 196", level: slide_door::REFERENCE_LEVEL, func: slide_door::UPDATE_FN, classes: &slide_door::CLASSES, update: slide_door::update, joints: &[] },
    UnitPort { unit: "U102 707", level: carriers::REFERENCE_LEVEL_TURNTABLE, func: carriers::TURNTABLE_FN, classes: &carriers::TURNTABLE_CLASSES, update: carriers::turntable, joints: &[] },
    UnitPort { unit: "U102 734", level: carriers::REFERENCE_LEVEL_TURNTABLE, func: carriers::TURNTABLE_FN_734, classes: &carriers::TURNTABLE_CLASSES_734, update: carriers::turntable, joints: &[] },
    UnitPort { unit: "U126 1210", level: carriers::REFERENCE_LEVEL_JOINT, func: carriers::JOINT_FN, classes: &carriers::JOINT_CLASSES, update: carriers::joint_platform, joints: &carriers::JOINT_CLASSES },
    UnitPort { unit: "U179 812", level: carriers::REFERENCE_LEVEL_PINNED, func: carriers::PINNED_FN, classes: &carriers::PINNED_CLASSES, update: carriers::pinned_platform, joints: &[] },
    UnitPort { unit: "U565 1381", level: falling_platform::REFERENCE_LEVEL, func: falling_platform::UPDATE_FN, classes: &falling_platform::CLASSES, update: falling_platform::update, joints: &[] },
    UnitPort { unit: "U207 1511", level: light_fixture::REFERENCE_LEVEL, func: light_fixture::UPDATE_FN, classes: &light_fixture::CLASSES, update: light_fixture::update, joints: &[] },
    UnitPort { unit: "U472 1511", level: light_fixture::REFERENCE_LEVEL_14, func: light_fixture::UPDATE_FN_14, classes: &light_fixture::CLASSES, update: light_fixture::update, joints: &[] },
    UnitPort { unit: "U514 1143", level: hologram_logo::REFERENCE_LEVEL, func: hologram_logo::UPDATE_FN, classes: &hologram_logo::CLASSES, update: hologram_logo::update, joints: &hologram_logo::CLASSES },
    UnitPort { unit: "U180 823", level: sweep_light::REFERENCE_LEVEL, func: sweep_light::UPDATE_FN, classes: &sweep_light::CLASSES, update: sweep_light::update, joints: &sweep_light::CLASSES },
    UnitPort { unit: "U155 481", level: spinner_float::REFERENCE_LEVEL, func: spinner_float::UPDATE_FN, classes: &spinner_float::CLASSES, update: spinner_float::update, joints: &spinner_float::CLASSES },
    UnitPort { unit: "U203 1139", level: board_sparkle::REFERENCE_LEVEL, func: board_sparkle::UPDATE_FN, classes: &board_sparkle::CLASSES, update: board_sparkle::update, joints: &[] },
    UnitPort { unit: "U177 717", level: rilgar_racer::REFERENCE_LEVEL, func: rilgar_racer::UPDATE_FN, classes: &rilgar_racer::CLASSES, update: rilgar_racer::update, joints: &rilgar_racer::JOINTS },
    UnitPort { unit: "U171 133", level: board_boost::REFERENCE_LEVEL, func: board_boost::UPDATE_FN, classes: &board_boost::CLASSES, update: board_boost::update, joints: &[] },
    UnitPort { unit: "U503 556", level: kalebo_racer::REFERENCE_LEVEL, func: kalebo_racer::UPDATE_FN, classes: &kalebo_racer::CLASSES, update: kalebo_racer::update, joints: &kalebo_racer::CLASSES },
    UnitPort { unit: "U174 439", level: hoverboard::REFERENCE_LEVEL, func: hoverboard::UPDATE_FN, classes: &hoverboard::CLASSES, update: hoverboard::update, joints: &hoverboard::JOINTS },
    UnitPort { unit: "U440 30", level: popup_turret::REFERENCE_LEVEL, func: popup_turret::UPDATE_FN, classes: &popup_turret::CLASSES, update: popup_turret::update, joints: &[] },
    UnitPort { unit: "U440 681", level: popup_turret::REFERENCE_LEVEL, func: popup_turret::SHOT_FN, classes: &popup_turret::SHOT_CLASSES, update: popup_turret::shot_update, joints: &[] },
    UnitPort { unit: "U212 1021", level: petal_door::REFERENCE_LEVEL, func: petal_door::UPDATE_FN, classes: &petal_door::CLASSES, update: petal_door::update, joints: &[] },
    UnitPort { unit: "U390 339", level: anim_idler::REFERENCE_LEVEL, func: anim_idler::UPDATE_FN, classes: &anim_idler::CLASSES, update: anim_idler::update, joints: &[] },
    UnitPort { unit: "U248 1013", level: linked_slider::REFERENCE_LEVEL, func: linked_slider::UPDATE_FN, classes: &linked_slider::CLASSES, update: linked_slider::update, joints: &[] },
    UnitPort { unit: "U329 1015", level: trip_block::REFERENCE_LEVEL, func: trip_block::UPDATE_FN, classes: &trip_block::CLASSES, update: trip_block::update, joints: &[] },
    UnitPort { unit: "U162 1101", level: switched_mover::REFERENCE_LEVEL, func: switched_mover::UPDATE_FN, classes: &switched_mover::CLASSES, update: switched_mover::update, joints: &[] },
    UnitPort { unit: "U487 1209", level: pressure_pad::REFERENCE_LEVEL, func: pressure_pad::UPDATE_FN, classes: &pressure_pad::CLASSES, update: pressure_pad::update, joints: &pressure_pad::CLASSES },
    UnitPort { unit: "U211 911", level: flame_jet::REFERENCE_LEVEL, func: flame_jet::UPDATE_FN, classes: &flame_jet::CLASSES, update: flame_jet::update, joints: &[] },
    UnitPort { unit: "U542 669", level: water_laser::REFERENCE_LEVEL, func: water_laser::UPDATE_FN, classes: &water_laser::CLASSES, update: water_laser::update, joints: &[] },
    UnitPort { unit: "U349 1544", level: orxon_vent::REFERENCE_LEVEL, func: orxon_vent::UPDATE_FN, classes: &orxon_vent::CLASSES, update: orxon_vent::update, joints: &[] },
    UnitPort { unit: "U375 1246", level: pokitaru_biter::REFERENCE_LEVEL, func: pokitaru_biter::UPDATE_FN, classes: &pokitaru_biter::CLASSES, update: pokitaru_biter::update, joints: &[] },
    UnitPort { unit: "U95 580", level: aridia_sandshark::REFERENCE_LEVEL, func: aridia_sandshark::UPDATE_FN, classes: &aridia_sandshark::CLASSES, update: aridia_sandshark::update, joints: &[] },
    UnitPort { unit: "U101 668", level: aridia_sandshark::REFERENCE_LEVEL, func: aridia_sandshark::NEST_FN, classes: &aridia_sandshark::NEST_CLASSES, update: aridia_sandshark::nest_update, joints: &[] },
    UnitPort { unit: "U96 612", level: aridia_flamer::REFERENCE_LEVEL, func: aridia_flamer::UPDATE_FN, classes: &aridia_flamer::CLASSES, update: aridia_flamer::update, joints: &aridia_flamer::JOINTS },
    UnitPort { unit: "U373 1231", level: pokitaru_thrower::REFERENCE_LEVEL, func: pokitaru_thrower::UPDATE_FN, classes: &pokitaru_thrower::CLASSES, update: pokitaru_thrower::update, joints: &[] },
    UnitPort { unit: "U373 1297", level: pokitaru_thrower::REFERENCE_LEVEL, func: pokitaru_thrower::BALL_FN, classes: &pokitaru_thrower::BALL_CLASSES, update: pokitaru_thrower::ball_update, joints: &[] },
    UnitPort { unit: "U128", level: air_traffic::REFERENCE_LEVEL, func: air_traffic::UPDATE_FN, classes: &air_traffic::CLASSES, update: air_traffic::update, joints: &air_traffic::JOINTS },
    UnitPort { unit: "U128 235", level: air_traffic::REFERENCE_LEVEL, func: air_traffic::TRAIL_FN, classes: &air_traffic::TRAIL_CLASSES, update: air_traffic::trail_update, joints: &[] },
    UnitPort { unit: "U126", level: kerwan_mover::REFERENCE_LEVEL, func: kerwan_mover::UPDATE_FN, classes: &kerwan_mover::CLASSES, update: kerwan_mover::update, joints: &[] },
    UnitPort { unit: "U363 1075", level: pokitaru_boat::REFERENCE_LEVEL, func: pokitaru_boat::UPDATE_FN, classes: &pokitaru_boat::CLASSES, update: pokitaru_boat::update, joints: &pokitaru_boat::JOINTS },
    UnitPort { unit: "U130 574", level: kerwan_trooper::REFERENCE_LEVEL, func: kerwan_trooper::UPDATE_FN, classes: &kerwan_trooper::CLASSES, update: kerwan_trooper::update, joints: &kerwan_trooper::JOINTS },
    UnitPort { unit: "U130 833", level: kerwan_trooper::REFERENCE_LEVEL, func: kerwan_trooper::ROCKET_FN, classes: &kerwan_trooper::ROCKET_CLASSES, update: kerwan_trooper::rocket_update, joints: &[] },
    UnitPort { unit: "U280 452", level: batalia_runner::REFERENCE_LEVEL, func: batalia_runner::UPDATE_FN, classes: &batalia_runner::CLASSES, update: batalia_runner::update, joints: &batalia_runner::JOINTS },
    UnitPort { unit: "U506 471", level: kalebo_belt::REFERENCE_LEVEL, func: kalebo_belt::UPDATE_FN, classes: &kalebo_belt::CLASSES, update: kalebo_belt::update, joints: &[] },
    // Draw callback only (the belt's second quad layer: `Callback::UnitQuads` payload).
    UnitPort { unit: "U506 471 layer", level: kalebo_belt::REFERENCE_LEVEL, func: kalebo_belt::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U541 99", level: fleet_laser::REFERENCE_LEVEL, func: fleet_laser::UPDATE_FN, classes: &fleet_laser::CLASSES, update: fleet_laser::update, joints: &[] },
    UnitPort { unit: "U485 221", level: quartu_critter::REFERENCE_LEVEL, func: quartu_critter::UPDATE_FN, classes: &quartu_critter::CLASSES, update: quartu_critter::update, joints: &quartu_critter::CLASSES },
    UnitPort { unit: "U567 1355", level: veldin_diver::REFERENCE_LEVEL, func: veldin_diver::UPDATE_FN, classes: &veldin_diver::CLASSES, update: veldin_diver::update, joints: &[] },
    // Draw callback only (the group's trails: `Callback::UnitQuads` payload; the glows are `UnitGlow` of the row above).
    UnitPort { unit: "U567 1355 trail", level: veldin_diver::REFERENCE_LEVEL, func: veldin_diver::TRAIL_FN, classes: &[], update: empty::update, joints: &[] },    // Draw callback only (the teleporter pads 1135's beam `0x3094f0`: `Callback::UnitQuads` payload; the pads are the
    UnitPort { unit: "U162 617", level: eudora_path_rider::REFERENCE_LEVEL, func: eudora_path_rider::UPDATE_FN, classes: &eudora_path_rider::CLASSES, update: eudora_path_rider::update, joints: &[] },
    UnitPort { unit: "U180 810", level: rilgar_rocker::REFERENCE_LEVEL, func: rilgar_rocker::UPDATE_FN, classes: &rilgar_rocker::CLASSES, update: rilgar_rocker::update, joints: &[] },
    UnitPort { unit: "U195 895", level: rilgar_flap::REFERENCE_LEVEL, func: rilgar_flap::UPDATE_FN, classes: &rilgar_flap::CLASSES, update: rilgar_flap::update, joints: &[] },
    UnitPort { unit: "U283 467", level: batalia_lift::REFERENCE_LEVEL, func: batalia_lift::UPDATE_FN, classes: &batalia_lift::CLASSES, update: batalia_lift::update, joints: &[] },
    UnitPort { unit: "U436 1577", level: gemlik_watch::REFERENCE_LEVEL, func: gemlik_watch::UPDATE_FN, classes: &gemlik_watch::CLASSES, update: gemlik_watch::update, joints: &[] },
    // Level04 0x2ce060 is `DeleteMoby(self)`: the markers' effect (`marker`, U139), separate code.
    UnitPort { unit: "U158 484", level: 4, func: 0x2c_e060, classes: &[484], update: marker::update, joints: &[] },
    UnitPort { unit: "U313 1206", level: gaspar_hazard::REFERENCE_LEVEL, func: gaspar_hazard::UPDATE_FN, classes: &gaspar_hazard::CLASSES, update: gaspar_hazard::update, joints: &[] },
    UnitPort { unit: "U60 695", level: floating_pushable::REFERENCE_LEVEL, func: floating_pushable::UPDATE_FN, classes: &floating_pushable::CLASSES, update: floating_pushable::update, joints: &[] },
    UnitPort { unit: "U256 1080", level: umbris_sinker::REFERENCE_LEVEL, func: umbris_sinker::UPDATE_FN, classes: &umbris_sinker::CLASSES, update: umbris_sinker::update, joints: &[] },
    UnitPort { unit: "U505 470", level: kalebo_glow::REFERENCE_LEVEL, func: kalebo_glow::UPDATE_FN, classes: &kalebo_glow::CLASSES, update: kalebo_glow::update, joints: &[] },
    UnitPort { unit: "U341 1240", level: orxon_sparks::REFERENCE_LEVEL, func: orxon_sparks::UPDATE_FN, classes: &orxon_sparks::CLASSES, update: orxon_sparks::update, joints: &[] },
    UnitPort { unit: "U263 104", level: linked_mover::REFERENCE_LEVEL, func: linked_mover::PLATFORM_FN, classes: &linked_mover::PLATFORM_CLASSES, update: linked_mover::platform_update, joints: &[] },
    UnitPort { unit: "U222 1054", level: blarg_doors::REFERENCE_LEVEL, func: blarg_doors::UPDATE_FN, classes: &blarg_doors::CLASSES, update: blarg_doors::update, joints: &[] },
    UnitPort { unit: "U88 1504", level: wandering_light::REFERENCE_LEVEL, func: wandering_light::UPDATE_FN, classes: &wandering_light::CLASSES, update: wandering_light::update, joints: &[] },
    UnitPort { unit: "U376 1248", level: pokitaru_gate::REFERENCE_LEVEL, func: pokitaru_gate::UPDATE_FN, classes: &pokitaru_gate::CLASSES, update: pokitaru_gate::update, joints: &[] },
    UnitPort { unit: "U194 893", level: rilgar_hatch::REFERENCE_LEVEL, func: rilgar_hatch::UPDATE_FN, classes: &rilgar_hatch::CLASSES, update: rilgar_hatch::update, joints: &rilgar_hatch::CLASSES },
    UnitPort { unit: "U191 855", level: rilgar_spout::REFERENCE_LEVEL, func: rilgar_spout::UPDATE_FN, classes: &rilgar_spout::CLASSES, update: rilgar_spout::update, joints: &[] },
    UnitPort { unit: "U438 1805", level: gemlik_breakable::REFERENCE_LEVEL, func: gemlik_breakable::UPDATE_FN, classes: &gemlik_breakable::CLASSES, update: gemlik_breakable::update, joints: &[] },
    UnitPort { unit: "U570 1750", level: ending_save::REFERENCE_LEVEL, func: ending_save::UPDATE_FN, classes: &ending_save::CLASSES, update: ending_save::update, joints: &[] },
    UnitPort { unit: "U252 1041", level: umbris_lobber::REFERENCE_LEVEL, func: umbris_lobber::UPDATE_FN, classes: &umbris_lobber::CLASSES, update: umbris_lobber::update, joints: &umbris_lobber::CLASSES },
    UnitPort { unit: "U252 882", level: umbris_lobber::REFERENCE_LEVEL, func: umbris_lobber::SHOT_FN, classes: &umbris_lobber::SHOT_CLASSES, update: umbris_lobber::shot_update, joints: &[] },
    UnitPort { unit: "U118 1212", level: path_ship::REFERENCE_LEVEL, func: path_ship::UPDATE_FN, classes: &path_ship::CLASSES, update: path_ship::update, joints: &path_ship::CLASSES },
    UnitPort { unit: "U155 434", level: eudora_crank_lift::REFERENCE_LEVEL, func: eudora_crank_lift::UPDATE_FN, classes: &eudora_crank_lift::CLASSES, update: eudora_crank_lift::update, joints: &eudora_crank_lift::CLASSES },
    UnitPort { unit: "U154 432", level: eudora_crank_follower::REFERENCE_LEVEL, func: eudora_crank_follower::UPDATE_FN, classes: &eudora_crank_follower::CLASSES, update: eudora_crank_follower::update, joints: &[] },
    UnitPort { unit: "U225 1066", level: blarg_waker::REFERENCE_LEVEL, func: blarg_waker::UPDATE_FN, classes: &blarg_waker::CLASSES, update: blarg_waker::update, joints: &[] },
    UnitPort { unit: "U140 899", level: kerwan_path_spawner::REFERENCE_LEVEL, func: kerwan_path_spawner::UPDATE_FN, classes: &kerwan_path_spawner::CLASSES, update: kerwan_path_spawner::spawner_update, joints: &[] },
    // The movers 899 creates (created only: not in the census's placed units).
    UnitPort { unit: "U140 898", level: kerwan_path_spawner::REFERENCE_LEVEL, func: kerwan_path_spawner::MOVER_FN, classes: &kerwan_path_spawner::MOVER_CLASSES, update: kerwan_path_spawner::mover_update, joints: &[] },
    UnitPort { unit: "L03 825", level: kerwan_turntable::REFERENCE_LEVEL, func: kerwan_turntable::UPDATE_FN, classes: &kerwan_turntable::CLASSES, update: kerwan_turntable::update, joints: &[] },
    UnitPort { unit: "L03 997", level: kerwan_riser::REFERENCE_LEVEL, func: kerwan_riser::UPDATE_FN, classes: &kerwan_riser::CLASSES, update: kerwan_riser::update, joints: &[] },
    UnitPort { unit: "L03 1548", level: kerwan_scene_fx::REFERENCE_LEVEL, func: kerwan_scene_fx::UPDATE_FN, classes: &kerwan_scene_fx::CLASSES, update: kerwan_scene_fx::update, joints: &[] },
    UnitPort { unit: "L03 914", level: kerwan_bystander::REFERENCE_LEVEL, func: kerwan_bystander::UPDATE_FN, classes: &kerwan_bystander::CLASSES, update: kerwan_bystander::update, joints: &[] },
    UnitPort { unit: "L03 816", level: kerwan_transport::REFERENCE_LEVEL, func: kerwan_transport::PLATFORM_FN, classes: &kerwan_transport::PLATFORM_CLASSES, update: kerwan_transport::update_816, joints: &[] },
    UnitPort { unit: "L03 1012", level: kerwan_transport::REFERENCE_LEVEL, func: kerwan_transport::SHUTTLE_FN, classes: &kerwan_transport::SHUTTLE_CLASSES, update: kerwan_transport::update_1012, joints: &[] },
    UnitPort { unit: "L03 578", level: kerwan_layer::REFERENCE_LEVEL, func: kerwan_layer::LAYER_FN, classes: &kerwan_layer::LAYER_CLASSES, update: kerwan_layer::update_578, joints: &[] },
    // The blobs 578 drops (created only: not in the census's placed units).
    UnitPort { unit: "L03 627", level: kerwan_layer::REFERENCE_LEVEL, func: kerwan_layer::BLOB_FN, classes: &kerwan_layer::BLOB_CLASSES, update: kerwan_layer::update_627, joints: &[] },
    UnitPort { unit: "U319 1885", level: pod_launcher::REFERENCE_LEVEL, func: pod_launcher::UPDATE_FN, classes: &pod_launcher::CLASSES, update: pod_launcher::update, joints: &pod_launcher::CLASSES },
    // The pods 1885 lobs (created only: not in the census's placed units).
    UnitPort { unit: "U319 1886", level: pod_launcher::REFERENCE_LEVEL, func: pod_launcher::POD_FN, classes: &pod_launcher::POD_CLASSES, update: pod_launcher::pod_update, joints: &[] },
    UnitPort { unit: "U134 455", level: pod_spawner::REFERENCE_LEVEL, func: pod_spawner::UPDATE_FN, classes: &pod_spawner::CLASSES, update: pod_spawner::update, joints: &pod_spawner::CLASSES },
    UnitPort { unit: "U134 545", level: pod_spawner::REFERENCE_LEVEL, func: pod_spawner::POD_FN, classes: &pod_spawner::POD_CLASSES, update: pod_spawner::pod_update, joints: &[] },
    UnitPort { unit: "U581 582", level: veldin_rails::REFERENCE_LEVEL, func: veldin_rails::UPDATE_FN, classes: &veldin_rails::CLASSES, update: veldin_rails::update, joints: &[] },
    UnitPort { unit: "U597 1434", level: veldin_turnover::REFERENCE_LEVEL, func: veldin_turnover::UPDATE_FN, classes: &veldin_turnover::CLASSES, update: veldin_turnover::update, joints: &[] },
    UnitPort { unit: "U602 1799", level: veldin_scene_jet::REFERENCE_LEVEL, func: veldin_scene_jet::UPDATE_FN, classes: &veldin_scene_jet::CLASSES, update: veldin_scene_jet::update, joints: &[] },
    UnitPort { unit: "U593 1392", level: veldin_lock_field::REFERENCE_LEVEL, func: veldin_lock_field::UPDATE_FN, classes: &veldin_lock_field::CLASSES, update: veldin_lock_field::update, joints: &[] },
    // Draw callback only (1392's bands `0x2f1fd8`: `Callback::UnitQuads`).
    UnitPort { unit: "U593 1392 field", level: veldin_lock_field::REFERENCE_LEVEL, func: veldin_lock_field::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::UPDATE_FN, classes: &veldin_pool::CLASSES, update: veldin_pool::update, joints: &[] },
    // Draw callbacks only (1402's three meshes: `Callback::UnitQuads`).
    UnitPort { unit: "U594 1402 mesh 0", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[0], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402 mesh 1", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[1], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U594 1402 mesh 2", level: veldin_pool::REFERENCE_LEVEL, func: veldin_pool::DRAW_FNS[2], classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U519 1890", level: energy_fan::REFERENCE_LEVEL, func: energy_fan::UPDATE_FN, classes: &energy_fan::CLASSES, update: energy_fan::update, joints: &[] },
    // Draw callback only (the fan's blades and rings `0x2fb018`: `Callback::UnitQuads`, two groups).
    UnitPort { unit: "U519 1890 fan", level: energy_fan::REFERENCE_LEVEL, func: energy_fan::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::UPDATE_FN, classes: &veldin_finale_fx::CLASSES, update: veldin_finale_fx::update, joints: &[] },
    // Draw callbacks only (1563's flash, glow and beam: `Callback::UnitQuads`; the seat glows: `UnitGlow`).
    UnitPort { unit: "U599 1563 flash", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::FLASH_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 seat", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::SEAT_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 glow", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 beam", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U599 1563 morph beam", level: veldin_finale_fx::REFERENCE_LEVEL, func: veldin_finale_fx::MORPH_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U598 1454", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::UPDATE_FN, classes: &veldin_tank::CLASSES, update: veldin_tank::update, joints: &veldin_tank::CLASSES },
    // The tank's shells and treads (created only: not in the census's placed units).
    UnitPort { unit: "U598 41", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::SHELL_FN, classes: &veldin_tank::SHELL_CLASSES, update: veldin_tank::shell_update, joints: &[] },
    UnitPort { unit: "U598 331", level: veldin_tank::REFERENCE_LEVEL, func: veldin_tank::TREAD_FN, classes: &veldin_tank::TREAD_CLASSES, update: veldin_tank::tread_update, joints: &[] },
    UnitPort { unit: "U511 1356", level: dropship::REFERENCE_LEVEL, func: dropship::UPDATE_FN, classes: &dropship::CLASSES, update: dropship::update, joints: &dropship::CLASSES },
    // The dropship's shots (created only).
    UnitPort { unit: "U511 50", level: dropship::REFERENCE_LEVEL, func: dropship::SHOT_FN, classes: &dropship::SHOT_CLASSES, update: dropship::shot_update, joints: &[] },
    UnitPort { unit: "U504 638", level: drop_trooper::REFERENCE_LEVEL, func: drop_trooper::UPDATE_FN, classes: &drop_trooper::CLASSES, update: drop_trooper::update, joints: &drop_trooper::CLASSES },
    // The trooper's shots (created only).
    UnitPort { unit: "U504 49", level: drop_trooper::REFERENCE_LEVEL, func: drop_trooper::SHOT_FN, classes: &drop_trooper::SHOT_CLASSES, update: drop_trooper::shot_update, joints: &[] },
    // The flyers' and gunship's wreck (created only; the module sits with its level-01 ships).
    UnitPort { unit: "1510", level: super::burning_wreck::REFERENCE_LEVEL, func: super::burning_wreck::UPDATE_FN, classes: &super::burning_wreck::CLASSES, update: super::burning_wreck::update, joints: &[] },
    // The Summoner mouse 1818's glow sprites (draw only; its update is `classes::mouse`).
    UnitPort { unit: "1818 glow", level: super::mouse::REFERENCE_LEVEL, func: super::mouse::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    // The blob shadows' draw (`fun_001f4880`, `crate::shadows::blob`; draw only).
    UnitPort { unit: "blob shadow", level: 1, func: crate::shadows::BLOB_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "1510 glow", level: super::burning_wreck::REFERENCE_LEVEL, func: super::burning_wreck::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U24 530", level: veldin_ship::REFERENCE_LEVEL, func: veldin_ship::UPDATE_FN, classes: &veldin_ship::CLASSES, update: veldin_ship::update, joints: &veldin_ship::CLASSES },
    UnitPort { unit: "U36 1440", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::UPDATE_FN, classes: &veldin_beamer::CLASSES, update: veldin_beamer::update, joints: &veldin_beamer::CLASSES },
    UnitPort { unit: "U37 1471", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::MANAGER_FN, classes: &veldin_beamer::MANAGER_CLASSES, update: veldin_beamer::manager, joints: &[] },
    // Draw callbacks only (1440's eye glow `0x2e1c78`: `Callback::UnitGlow`; 1471's beams `0x2e2af0`: `UnitFrame` + `UnitQuads`).
    UnitPort { unit: "U36 1440 eye", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::EYE_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U37 1471 beams", level: veldin_beamer::REFERENCE_LEVEL, func: veldin_beamer::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U38 1545", level: veldin_scene_fx::REFERENCE_LEVEL, func: veldin_scene_fx::UPDATE_FN, classes: &veldin_scene_fx::CLASSES, update: veldin_scene_fx::update, joints: &[] },
    UnitPort { unit: "1475 board missile", level: board_missile::REFERENCE_LEVEL, func: board_missile::UPDATE_FN, classes: &board_missile::CLASSES, update: board_missile::update, joints: &[] },
    UnitPort { unit: "U509 933", level: kalebo_mine::REFERENCE_LEVEL, func: kalebo_mine::UPDATE_FN, classes: &kalebo_mine::CLASSES, update: kalebo_mine::update, joints: &[] },
    UnitPort { unit: "U521 1401", level: kalebo_mine_drone::REFERENCE_LEVEL, func: kalebo_mine_drone::UPDATE_FN, classes: &kalebo_mine_drone::CLASSES, update: kalebo_mine_drone::update, joints: &[] },
    // class port `classes::teleporter`).
    UnitPort { unit: "U85 1135 beam", level: 1, func: crate::moby_update::classes::teleporter::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U359 318", level: pokitaru_teleporter::REFERENCE_LEVEL, func: pokitaru_teleporter::UPDATE_FN, classes: &pokitaru_teleporter::CLASSES, update: pokitaru_teleporter::update, joints: &[] },
    // Draw callback only (the beam of the Pokitaru pads: `Callback::UnitQuads` payload).
    UnitPort { unit: "U359 318 beam", level: pokitaru_teleporter::REFERENCE_LEVEL, func: pokitaru_teleporter::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    // The cave drips 751 spawns (created only: not in the census's placed units).
    UnitPort { unit: "787 drip", level: drip::REFERENCE_LEVEL, func: drip::UPDATE_FN, classes: &drip::CLASSES, update: drip::update, joints: &[] },
    UnitPort { unit: "U50 613", level: water_current::REFERENCE_LEVEL, func: water_current::UPDATE_FN, classes: &water_current::CLASSES, update: water_current::update, joints: &[] },
    UnitPort { unit: "U129 573", level: kerwan_hound::REFERENCE_LEVEL, func: kerwan_hound::UPDATE_FN, classes: &kerwan_hound::CLASSES, update: kerwan_hound::update, joints: &[] },
    UnitPort { unit: "U353 114", level: pokitaru_commando::REFERENCE_LEVEL, func: pokitaru_commando::UPDATE_FN, classes: &pokitaru_commando::CLASSES, update: pokitaru_commando::update, joints: &[] },
    UnitPort { unit: "U350 65", level: pokitaru_commando::REFERENCE_LEVEL, func: pokitaru_commando::GATE_FN, classes: &pokitaru_commando::GATE_CLASSES, update: pokitaru_commando::gate_update, joints: &[] },
    UnitPort { unit: "U363 1157", level: pokitaru_cutaway::REFERENCE_LEVEL, func: pokitaru_cutaway::UPDATE_FN, classes: &pokitaru_cutaway::CLASSES, update: pokitaru_cutaway::update, joints: &[] },
    // The boss of Veldin's last arena and its partners (level 18; 2026-10-01).
    UnitPort { unit: "U564 1422", level: veldin_boss::REFERENCE_LEVEL, func: veldin_boss::UPDATE_FN, classes: &veldin_boss::CLASSES, update: veldin_boss::update, joints: &veldin_boss::JOINTS },
    // Draw callback only (the boss's glows `0x2f7880`: `Callback::UnitGlow` payload).
    UnitPort { unit: "U564 1422 glow", level: veldin_boss::REFERENCE_LEVEL, func: veldin_boss::DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U557 644", level: veldin_cutaway::REFERENCE_LEVEL, func: veldin_cutaway::UPDATE_FN, classes: &veldin_cutaway::CLASSES, update: veldin_cutaway::update, joints: &[] },
    UnitPort { unit: "U556 587", level: veldin_floater::REFERENCE_LEVEL, func: veldin_floater::UPDATE_FN, classes: &veldin_floater::CLASSES, update: veldin_floater::update, joints: &[] },
    UnitPort { unit: "U572 1906", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::UPDATE_FN, classes: &veldin_hopper::CLASSES, update: veldin_hopper::update, joints: &veldin_hopper::CLASSES },
    UnitPort { unit: "U572 1906 glow", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::GLOW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U572 1906 beam", level: veldin_hopper::REFERENCE_LEVEL, func: veldin_hopper::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U554 583", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::PAD_FN, classes: &veldin_pads::PAD_CLASSES, update: veldin_pads::pad_update, joints: &[] },
    UnitPort { unit: "U555 586", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::BUTTON_FN, classes: &veldin_pads::BUTTON_CLASSES, update: veldin_pads::button_update, joints: &[] },
    // Draw callback with a `rand` at draw time (586's countdown `0x2d8098`: `Callback::UnitFrame` payload).
    UnitPort { unit: "U555 586 countdown", level: veldin_pads::REFERENCE_LEVEL, func: veldin_pads::COUNTDOWN_FN, classes: &[], update: empty::update, joints: &[] },
    // The boss's created shots and effects (not placed: created only).
    UnitPort { unit: "564 lob", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::LOB_FN, classes: &veldin_shots::LOB_CLASSES, update: veldin_shots::lob_update, joints: &[] },
    // Draw callback only (564's target marker `0x2d5768`: `Callback::UnitQuads` payload).
    UnitPort { unit: "564 lob marker", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::LOB_MARK_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "624 ring", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::RING_FN, classes: &veldin_shots::RING_CLASSES, update: veldin_shots::ring_update, joints: &[] },
    UnitPort { unit: "628 aura", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::AURA_FN, classes: &veldin_shots::AURA_CLASSES, update: veldin_shots::aura_update, joints: &[] },
    UnitPort { unit: "628 aura bands", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::AURA_DRAW_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "983 beam", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::BEAM_FN, classes: &veldin_shots::BEAM_CLASSES, update: veldin_shots::beam_update, joints: &[] },
    UnitPort { unit: "1898 flash", level: veldin_shots::REFERENCE_LEVEL, func: veldin_shots::FLASH_FN, classes: &veldin_shots::FLASH_CLASSES, update: veldin_shots::flash_update, joints: &[] },
    UnitPort { unit: "U123 822", level: kerwan_train::REFERENCE_LEVEL, func: kerwan_train::UPDATE_FN, classes: &kerwan_train::CLASSES, update: kerwan_train::update, joints: &kerwan_train::JOINTS },
    UnitPort { unit: "U124 845", level: kerwan_train::REFERENCE_LEVEL, func: kerwan_train::LEAD_FN, classes: &kerwan_train::LEAD_CLASSES, update: kerwan_train::lead_update, joints: &[] },
    UnitPort { unit: "U323 22", level: clank_section::REFERENCE_LEVEL, func: clank_section::UPDATE_FN, classes: &clank_section::CLASSES, update: clank_section::update, joints: &[] },
    UnitPort { unit: "U500 1451", level: giant_pad::REFERENCE_LEVEL, func: giant_pad::UPDATE_FN, classes: &giant_pad::CLASSES, update: giant_pad::update, joints: &[] },
    UnitPort { unit: "U220 1061", level: blarg_clank_lift::REFERENCE_LEVEL, func: blarg_clank_lift::UPDATE_FN, classes: &blarg_clank_lift::CLASSES, update: blarg_clank_lift::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x593", level: giant_shockwave::REFERENCE_LEVEL, func: giant_shockwave::UPDATE_FN, classes: &giant_shockwave::CLASSES, update: giant_shockwave::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x100", level: giant_missile::REFERENCE_LEVEL, func: giant_missile::UPDATE_FN, classes: &giant_missile::CLASSES, update: giant_missile::update, joints: &[] },
    UnitPort { unit: "Giant Clank 0x5f3", level: giant_beam::REFERENCE_LEVEL, func: giant_beam::UPDATE_FN, classes: &giant_beam::CLASSES, update: giant_beam::update, joints: &[] },
    UnitPort { unit: "U499 1446", level: quartu_giant_mission::REFERENCE_LEVEL, func: quartu_giant_mission::UPDATE_FN, classes: &quartu_giant_mission::CLASSES, update: quartu_giant_mission::update, joints: &[] },
    UnitPort { unit: "U307 664", level: riding_floats::REFERENCE_LEVEL_664, func: riding_floats::UPDATE_FN_664, classes: &riding_floats::CLASSES_664, update: riding_floats::float_664, joints: &[] },
    UnitPort { unit: "U316 1293", level: riding_floats::REFERENCE_LEVEL_1293, func: riding_floats::UPDATE_FN_1293, classes: &riding_floats::CLASSES_1293, update: riding_floats::float_1293, joints: &[] },
    UnitPort { unit: "U255 1069", level: riding_floats::REFERENCE_LEVEL_1069, func: riding_floats::UPDATE_FN_1069, classes: &riding_floats::CLASSES_1069, update: riding_floats::float_1069, joints: &[] },
    // Story drivers (batch 6, lane story: docs/plan/progression.md `## story`).
    UnitPort { unit: "U114 786", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::SURFER_FN, classes: &aridia_story::SURFER_CLASSES, update: aridia_story::surfer_update, joints: &aridia_story::SURFER_CLASSES },
    UnitPort { unit: "U115 788", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::AGENT_FN, classes: &aridia_story::AGENT_CLASSES, update: aridia_story::agent_update, joints: &[] },
    UnitPort { unit: "U142 890", level: kerwan_story::REFERENCE_LEVEL, func: kerwan_story::HELGA_FN, classes: &kerwan_story::HELGA_CLASSES, update: kerwan_story::helga_update, joints: &[] },
    UnitPort { unit: "U145 909", level: kerwan_story::REFERENCE_LEVEL, func: kerwan_story::AL_FN, classes: &kerwan_story::AL_CLASSES, update: kerwan_story::al_update, joints: &[] },
    UnitPort { unit: "U169 1120", level: eudora_story::REFERENCE_LEVEL, func: eudora_story::SUCK_FN, classes: &eudora_story::SUCK_CLASSES, update: eudora_story::suck_cannon_update, joints: &[] },
    UnitPort { unit: "U170 1190", level: eudora_story::REFERENCE_LEVEL, func: eudora_story::INFORMANT_FN, classes: &eudora_story::INFORMANT_CLASSES, update: eudora_story::informant_update, joints: &[] },
    UnitPort { unit: "U200 918", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::GIRL_FN, classes: &rilgar_story::GIRL_CLASSES, update: rilgar_story::race_girl_update, joints: &[] },
    UnitPort { unit: "U201 919", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::BOUNCER_FN, classes: &rilgar_story::BOUNCER_CLASSES, update: rilgar_story::bouncer_update, joints: &[] },
    UnitPort { unit: "U203 925", level: rilgar_story::REFERENCE_LEVEL, func: rilgar_story::SALESMAN_FN, classes: &rilgar_story::SALESMAN_CLASSES, update: rilgar_story::salesman_update, joints: &[] },
    UnitPort { unit: "U233 1105", level: blarg_story::REFERENCE_LEVEL, func: blarg_story::SCIENTIST_FN, classes: &blarg_story::SCIENTIST_CLASSES, update: blarg_story::scientist_update, joints: &[] },
    UnitPort { unit: "U297 1130", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::COMMANDO_FN, classes: &batalia_story::COMMANDO_CLASSES, update: batalia_story::commando_update, joints: &[] },
    UnitPort { unit: "U298 1144", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::DESERTER_FN, classes: &batalia_story::DESERTER_CLASSES, update: batalia_story::deserter_update, joints: &[] },
    UnitPort { unit: "U299 1283", level: batalia_story::REFERENCE_LEVEL, func: batalia_story::WORKER_FN, classes: &batalia_story::WORKER_CLASSES, update: batalia_story::water_worker_update, joints: &[] },
    UnitPort { unit: "U321 1290", level: gaspar_story::REFERENCE_LEVEL, func: gaspar_story::UPDATE_FN, classes: &gaspar_story::CLASSES, update: gaspar_story::update, joints: &[] },
    UnitPort { unit: "U329 18", level: orxon_story::REFERENCE_LEVEL, func: orxon_story::MAGNEBOOTS_FN, classes: &orxon_story::MAGNEBOOTS_CLASSES, update: orxon_story::magneboots_update, joints: &[] },
    UnitPort { unit: "U350 1326", level: orxon_story::REFERENCE_LEVEL, func: orxon_story::NANOTECH_FN, classes: &orxon_story::NANOTECH_CLASSES, update: orxon_story::nanotech_update, joints: &[] },
    UnitPort { unit: "U360 23", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::MASK_FN, classes: &pokitaru_story::MASK_CLASSES, update: pokitaru_story::mask_update, joints: &[] },
    UnitPort { unit: "U363 90", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::THRUSTER_FN, classes: &pokitaru_story::THRUSTER_CLASSES, update: pokitaru_story::thruster_update, joints: &[] },
    UnitPort { unit: "U365 298", level: pokitaru_story::REFERENCE_LEVEL, func: pokitaru_story::PERSUADER_FN, classes: &pokitaru_story::PERSUADER_CLASSES, update: pokitaru_story::persuader_update, joints: &[] },
    UnitPort { unit: "U394 282", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::MERCHANT_FN, classes: &hoven_story::MERCHANT_CLASSES, update: hoven_story::merchant_update, joints: &hoven_story::MERCHANT_CLASSES },
    UnitPort { unit: "U411 1404", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::SCENE_FN, classes: &hoven_story::SCENE_CLASSES, update: hoven_story::scene_trigger_update, joints: &[] },
    UnitPort { unit: "U398 328", level: hoven_story::REFERENCE_LEVEL, func: hoven_story::HYDRO_FN, classes: &hoven_story::HYDRO_CLASSES, update: hoven_story::hydro_update, joints: &[] },
    UnitPort { unit: "U440 1353", level: gemlik_story::REFERENCE_LEVEL, func: gemlik_story::UPDATE_FN, classes: &gemlik_story::CLASSES, update: gemlik_story::update, joints: &[] },
    UnitPort { unit: "U464 851", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::QWARK_FN, classes: &oltanis_story::QWARK_CLASSES, update: oltanis_story::qwark_update, joints: &[] },
    UnitPort { unit: "U469 924", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::MERCHANT_FN, classes: &oltanis_story::MERCHANT_CLASSES, update: oltanis_story::merchant_update, joints: &[] },
    UnitPort { unit: "U474 1354", level: oltanis_story::REFERENCE_LEVEL, func: oltanis_story::MORPH_FN, classes: &oltanis_story::MORPH_CLASSES, update: oltanis_story::morph_update, joints: &[] },
    UnitPort { unit: "U503 1388", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::GRABBER_FN, classes: &quartu_story::GRABBER_CLASSES, update: quartu_story::grabber_update, joints: &[] },
    UnitPort { unit: "U511 1469", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::HELP_FN, classes: &quartu_story::HELP_CLASSES, update: quartu_story::help_update, joints: &[] },
    UnitPort { unit: "U506 1419", level: quartu_story::REFERENCE_LEVEL, func: quartu_story::BROADCAST_FN, classes: &quartu_story::BROADCAST_CLASSES, update: quartu_story::broadcast_update, joints: &[] },
    UnitPort { unit: "U529 1377", level: kalebo_story::REFERENCE_LEVEL, func: kalebo_story::UPDATE_FN, classes: &kalebo_story::CLASSES, update: kalebo_story::update, joints: &[] },
    UnitPort { unit: "U561 1428", level: fleet_story::REFERENCE_LEVEL, func: fleet_story::UPDATE_FN, classes: &fleet_story::CLASSES, update: fleet_story::update, joints: &[] },
    UnitPort { unit: "U31 834", level: veldin_story::REFERENCE_LEVEL, func: veldin_story::UPDATE_FN, classes: &veldin_story::CLASSES, update: veldin_story::update, joints: &[] },
    UnitPort { unit: "U248 436", level: umbris_story::REFERENCE_LEVEL, func: umbris_story::UPDATE_FN, classes: &umbris_story::CLASSES, update: umbris_story::update, joints: &[] },
    UnitPort { unit: "U118 1005", level: aridia_story::REFERENCE_LEVEL, func: aridia_story::ITEM_SCENE_FN, classes: &aridia_story::ITEM_SCENE_CLASSES, update: aridia_story::item_scene_update, joints: &[] },
    UnitPort { unit: "U405 1267", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::UPDATE_FN, classes: &hoven_turret::CLASSES, update: hoven_turret::update, joints: &hoven_turret::CLASSES },
    // Draw callbacks only (no class runs them): the HUD (`Callback::UnitFrame`) and the red screen (`Callback::UnitQuads`).
    UnitPort { unit: "U405 1267 hud", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U405 1267 tint", level: hoven_turret::REFERENCE_LEVEL, func: hoven_turret::TINT_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U237 1109", level: blarg_shuttle::REFERENCE_LEVEL, func: blarg_shuttle::UPDATE_FN, classes: &blarg_shuttle::CLASSES, update: blarg_shuttle::update, joints: &[] },
    UnitPort { unit: "U424 69", level: gemlik_ship::REFERENCE_LEVEL, func: gemlik_ship::UPDATE_FN, classes: &gemlik_ship::CLASSES, update: gemlik_ship::update, joints: &gemlik_ship::CLASSES },
    UnitPort { unit: "U424 69 hud", level: gemlik_ship::REFERENCE_LEVEL, func: gemlik_ship_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U434 388", level: qwark_ship::REFERENCE_LEVEL, func: qwark_ship::UPDATE_FN, classes: &qwark_ship::CLASSES, update: qwark_ship::update, joints: &qwark_ship::CLASSES },
    UnitPort { unit: "U434 388 beam", level: qwark_ship::REFERENCE_LEVEL, func: qwark_ship::BEAM_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "388 parts", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::PART_FN, classes: &qwark_ship_parts::PART_CLASSES, update: qwark_ship_parts::part_update, joints: &[] },
    UnitPort { unit: "388 shield 352", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::SHIELD_FN, classes: &qwark_ship_parts::SHIELD_CLASSES, update: qwark_ship_parts::shield_update, joints: &[] },
    UnitPort { unit: "388 missile 82", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::MISSILE_FN, classes: &qwark_ship_parts::MISSILE_CLASSES, update: qwark_ship_parts::missile_update, joints: &qwark_ship_parts::MISSILE_CLASSES },
    UnitPort { unit: "388 mine 83", level: qwark_ship_parts::REFERENCE_LEVEL, func: qwark_ship_parts::MINE_FN, classes: &qwark_ship_parts::MINE_CLASSES, update: qwark_ship_parts::mine_update, joints: &[] },
    UnitPort { unit: "458 shell", level: turret_shell::REFERENCE_LEVEL, func: turret_shell::UPDATE_FN, classes: &turret_shell::CLASSES, update: turret_shell::update, joints: &[] },
    UnitPort { unit: "U407 1274", level: hoven_carrier::REFERENCE_LEVEL, func: hoven_carrier::UPDATE_FN, classes: &hoven_carrier::CLASSES, update: hoven_carrier::update, joints: &hoven_carrier::JOINTS },
    UnitPort { unit: "184 shot", level: gun_shot::REFERENCE_LEVEL, func: gun_shot::UPDATE_FN, classes: &gun_shot::CLASSES, update: gun_shot::update, joints: &[] },
    UnitPort { unit: "1009 laser", level: ship_laser::REFERENCE_LEVEL, func: ship_laser::UPDATE_FN, classes: &ship_laser::CLASSES, update: ship_laser::update, joints: &[] },
    UnitPort { unit: "295 missile", level: ship_missile::REFERENCE_LEVEL, func: ship_missile::UPDATE_FN, classes: &ship_missile::CLASSES, update: ship_missile::update, joints: &ship_missile::CLASSES },
    UnitPort { unit: "1034 missile", level: ship_missile::EARLY_LEVEL, func: ship_missile::EARLY_UPDATE_FN, classes: &ship_missile::EARLY_CLASSES, update: ship_missile::update, joints: &ship_missile::EARLY_CLASSES },
    UnitPort { unit: "U389 1319", level: ship_fighter::REFERENCE_LEVEL, func: ship_fighter::UPDATE_FN, classes: &ship_fighter::CLASSES, update: ship_fighter::update, joints: &[] },
    UnitPort { unit: "U389 1319 trails", level: ship_fighter::REFERENCE_LEVEL, func: ship_fighter::TRAIL_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U576 1843", level: ship_fighter::FLEET_LEVEL, func: ship_fighter::FLEET_UPDATE_FN, classes: &ship_fighter::FLEET_CLASSES, update: ship_fighter::update, joints: &[] },
    UnitPort { unit: "U576 1843 trails", level: ship_fighter::FLEET_LEVEL, func: ship_fighter::FLEET_TRAIL_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U433 224 228", level: ship_pickup_float::REFERENCE_LEVEL, func: ship_pickup_float::UPDATE_FN, classes: &ship_pickup_float::CLASSES, update: ship_pickup_float::update, joints: &ship_pickup_float::CLASSES },
    UnitPort { unit: "U568 1379", level: fleet_ship::REFERENCE_LEVEL, func: fleet_ship::UPDATE_FN, classes: &fleet_ship::CLASSES, update: fleet_ship::update, joints: &fleet_ship::CLASSES },
    UnitPort { unit: "U568 1379 hud", level: fleet_ship::REFERENCE_LEVEL, func: fleet_ship_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U563 347", level: fleet_turret::REFERENCE_LEVEL, func: fleet_turret::UPDATE_FN, classes: &fleet_turret::CLASSES, update: fleet_turret::update, joints: &fleet_turret::CLASSES },
    UnitPort { unit: "U563 1368", level: fleet_turret::REFERENCE_LEVEL, func: fleet_turret::SHOT_FN, classes: &fleet_turret::SHOT_CLASSES, update: fleet_turret::shot_update, joints: &[] },
    UnitPort { unit: "U384 1242", level: pokitaru_jet::REFERENCE_LEVEL, func: pokitaru_jet::UPDATE_FN, classes: &pokitaru_jet::CLASSES, update: pokitaru_jet::update, joints: &pokitaru_jet::CLASSES },
    UnitPort { unit: "U384 1242 hud", level: pokitaru_jet::REFERENCE_LEVEL, func: pokitaru_jet_hud::HUD_FN, classes: &[], update: empty::update, joints: &[] },
    UnitPort { unit: "U387 1264", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::UPDATE_FN, classes: &pokitaru_convoy::CLASSES, update: pokitaru_convoy::update, joints: &[] },
    UnitPort { unit: "U387 1265 car", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::CAR_FN, classes: &pokitaru_convoy::CAR_CLASSES, update: pokitaru_convoy::car_update, joints: &[] },
    UnitPort { unit: "U387 1524 sludge", level: pokitaru_convoy::REFERENCE_LEVEL, func: pokitaru_convoy::SLUDGE_FN, classes: &pokitaru_convoy::SLUDGE_CLASSES, update: pokitaru_convoy::sludge_update, joints: &[] },
    UnitPort { unit: "1218 pickups", level: ship_pickup::REFERENCE_LEVEL, func: ship_pickup::UPDATE_FN, classes: &ship_pickup::CLASSES, update: ship_pickup::update, joints: &ship_pickup::CLASSES },
    UnitPort { unit: "1219 parachute", level: ship_pickup::REFERENCE_LEVEL, func: ship_pickup::CHUTE_FN, classes: &ship_pickup::CHUTE_CLASSES, update: ship_pickup::chute_update, joints: &ship_pickup::CHUTE_CLASSES },
    UnitPort { unit: "1017 shot", level: fighter_shot::REFERENCE_LEVEL, func: fighter_shot::UPDATE_FN, classes: &fighter_shot::CLASSES, update: fighter_shot::update, joints: &[] },
    UnitPort { unit: "U397 326", level: hoven_drone::REFERENCE_LEVEL, func: hoven_drone::UPDATE_FN, classes: &hoven_drone::CLASSES, update: hoven_drone::update, joints: &hoven_drone::JOINTS },
    UnitPort { unit: "U397 409", level: hoven_drone::REFERENCE_LEVEL, func: hoven_drone::SHOT_FN, classes: &hoven_drone::SHOT_CLASSES, update: hoven_drone::shot_update, joints: &[] },
    UnitPort { unit: "1371 rider", level: drone_rider::REFERENCE_LEVEL, func: drone_rider::UPDATE_FN, classes: &drone_rider::CLASSES, update: drone_rider::update, joints: &[] },
    UnitPort { unit: "U538 1455", level: kalebo_race::REFERENCE_LEVEL, func: kalebo_race::UPDATE_FN, classes: &kalebo_race::CLASSES, update: kalebo_race::update, joints: &kalebo_race::CLASSES },
];

/// Port indices as the `ClassUpdate::Unit` payload.
pub fn ids() -> impl Iterator<Item = u16> { 0..PORTS.len() as u16 }

/// `(*moby+0x74)(moby)` for unit port `i`.
pub fn update(w: &mut World, id: MobyId, i: u16) { (PORTS[i as usize].update)(w, id) }

/// The level data words the unit ports' class code reads and writes (a level's `$gp` / `.data` globals, e.g. the
/// rising floats' last-sound tick), by their address in the unit's reference overlay; reset with the level
/// (`Services::new`).
#[derive(Clone, Debug, Default)]
pub struct Globals {
    words: std::collections::HashMap<u32, u32>,
    /// Veldin's beam slots (level00 0x161bf8.., `veldin_beamer`).
    pub veldin_beams: veldin_beamer::Beams,
    /// Veldin's last level's cutscene effects (`veldin_finale_fx`: its level words and this tick's draws).
    pub veldin_finale: veldin_finale_fx::Fx,
    /// The pool meshes of level 18 (`veldin_pool`; read from the overlay by the engine at the level load).
    pub veldin_pools: Option<std::sync::Arc<veldin_pool::Meshes>>,
}

impl Globals {
    /// The word at `addr` (0 until written: the words these ports use start at 0 in the data).
    pub fn word(&self, addr: u32) -> u32 { self.words.get(&addr).copied().unwrap_or(0) }
    pub fn set_word(&mut self, addr: u32, v: u32) { self.words.insert(addr, v); }
    /// The word at `addr`, `init` until written (a word the overlay data starts at another value: Qwark's −1).
    pub fn word_or(&self, addr: u32, init: u32) -> u32 { self.words.get(&addr).copied().unwrap_or(init) }
}

// ---------------------------------------------------------------------------------------------------
// Small shared reads the unit ports make (each the game's own global or class-header field).

/// Ratchet's position `0x13f3d0` as `f32`.
pub fn hero_pos(w: &World) -> [f32; 4] { w.hero.pos.map(|x| f32::from_bits(x.0)) }

/// The class scale (class header +0x24) of `o_class` as `f32`.
pub fn class_scale(w: &World, o_class: i16) -> f32 { crate::moby_update::services::fl(w.class_scale(o_class)) }

/// Whether the class header of `o_class` has a collision blob (class +0x10: what the game stores into moby +0x94 to
/// turn the moby's collision back on).
pub fn class_collision(w: &World, o_class: i16) -> bool { w.classes.info(o_class).is_some_and(|i| i.has_collision) }

/// `0x277a00(amp, rate, moby, &phase, &prev)`: a vertical bob: phase += rate (`fast_add_rotations`), z −= prev,
/// prev = amp·sin(phase), z += prev. `phase` / `prev` are pvar offsets.
pub fn bob(w: &mut World, id: MobyId, amp: f32, rate: f32, phase: usize, prev: usize) {
    use crate::moby_update::creature as c;
    let a = c::add_rot(c::pf(w, id, phase), rate);
    c::set_pf(w, id, phase, a);
    let old = c::pf(w, id, prev);
    let s = a.sin() * amp;
    c::set_pf(w, id, prev, s);
    let m = w.mm(id);
    m.position[2] = (m.position[2] - old) + s;
}

/// `0x277a80(amp, rate_a, rate_b, moby, &a, &b)`: a tilt wobble: rot.x = amp·sin a·sin b, rot.y = amp·sin a·cos b,
/// then a += rate_a, b += rate_b (`fast_add_rotations`). `a` / `b` are pvar offsets. (The mines' copy of it is inline
/// in `classes::mine`.)
pub fn wobble(w: &mut World, id: MobyId, amp: f32, rate_a: f32, rate_b: f32, pa: usize, pb: usize) {
    use crate::moby_update::creature as c;
    let (a, b) = (c::pf(w, id, pa), c::pf(w, id, pb));
    let m = w.mm(id);
    m.rotation[0] = amp * a.sin() * b.sin();
    m.rotation[1] = amp * a.sin() * b.cos();
    c::set_pf(w, id, pa, c::add_rot(a, rate_a));
    c::set_pf(w, id, pb, c::add_rot(b, rate_b));
}

/// `FUN_00272078(moby)`: Ratchet's moby's light word and ambient (+0x38..+0x3f) copied onto `id`.
pub fn take_hero_light(w: &mut World, id: MobyId) {
    if let Some(h) = w.hero_moby {
        let (l, a) = (w.m(h).light, w.m(h).ambient);
        let m = w.mm(id);
        m.light = l;
        m.ambient = a;
    }
}

// ---------------------------------------------------------------------------------------------------
// Draw callbacks and the registry lookups of the unit ports.

/// The row of the unit whose reference update is `func` of level `level` (as the `Callback::UnitGlow` payload).
pub fn row(level: u32, func: u32) -> Option<u16> { PORTS.iter().position(|u| (u.level, u.func) == (level, func)).map(|i| i as u16) }

/// One call of the glow quad `0x2781d0(size, pull, point, rgba)` (`rc-engine` `fx_draw::glow_quad`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowQuad {
    pub size: f32,
    /// Toward the camera.
    pub pull: f32,
    pub point: [f32; 3],
    /// GS RGBA (R low).
    pub rgba: u32,
}

/// One `FastDrawQuadReal` quad of a unit's draw callback, in world space: corners in GS strip order, ST, GS RGBA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxQuad {
    pub corners: [[f32; 3]; 4],
    pub st: [[f32; 2]; 4],
    pub rgba: [u32; 4],
}

/// A unit draw callback's quads with their FX texture and blend (ALPHA 0x48 additive, else 0x44).
#[derive(Clone, Debug, PartialEq)]
pub struct FxQuads {
    pub fx: usize,
    pub additive: bool,
    pub quads: Vec<FxQuad>,
}

/// The quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitQuads(i)`; draw only).
pub fn fx_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Option<FxQuads> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((1, crate::moby_update::classes::teleporter::BEAM_FN)) => crate::moby_update::classes::teleporter::beam_quads(table, svc, id),
        Some((pokitaru_teleporter::REFERENCE_LEVEL, pokitaru_teleporter::BEAM_FN)) => pokitaru_teleporter::beam_quads(table, svc, id),
        Some((laser_fence::REFERENCE_LEVEL, laser_fence::UPDATE_FN)) => laser_fence::fx_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::ARC_FN)) => hover_zapper::fx_quads(table, svc, id),
        Some((quartu_drone::REFERENCE_LEVEL, quartu_drone::UPDATE_FN)) => quartu_drone::fx_quads(table, svc, id),
        Some((swing_laser::REFERENCE_LEVEL, swing_laser::UPDATE_FN)) => swing_laser::fx_quads(table, svc, id),
        Some((kalebo_barrier::REFERENCE_LEVEL, kalebo_barrier::UPDATE_FN)) => kalebo_barrier::fx_quads(table, svc, id),
        Some((sweep_light::REFERENCE_LEVEL, sweep_light::UPDATE_FN)) => sweep_light::fx_quads(table, svc, id),
        Some((light_fixture::REFERENCE_LEVEL, light_fixture::UPDATE_FN)) => light_fixture::fx_quads(table, svc, id),
        Some((water_laser::REFERENCE_LEVEL, water_laser::UPDATE_FN)) => water_laser::fx_quads(table, svc, id),
        Some((kalebo_belt::REFERENCE_LEVEL, kalebo_belt::UPDATE_FN)) => kalebo_belt::fx_quads(table, svc, id, false),
        Some((kalebo_belt::REFERENCE_LEVEL, kalebo_belt::DRAW_FN)) => kalebo_belt::fx_quads(table, svc, id, true),
        Some((fleet_laser::REFERENCE_LEVEL, fleet_laser::UPDATE_FN)) => fleet_laser::fx_quads(table, svc, id),
        Some((veldin_diver::REFERENCE_LEVEL, veldin_diver::TRAIL_FN)) => veldin_diver::fx_quads(table, svc, id),
        Some((veldin_lock_field::REFERENCE_LEVEL, veldin_lock_field::DRAW_FN)) => veldin_lock_field::fx_quads(table, svc, id),
        Some((veldin_pool::REFERENCE_LEVEL, f)) if veldin_pool::DRAW_FNS.contains(&f) => veldin_pool::fx_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::LOB_MARK_FN)) => veldin_shots::lob_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::RING_FN)) => veldin_shots::ring_quads(table, svc, id),
        Some((veldin_shots::REFERENCE_LEVEL, veldin_shots::AURA_DRAW_FN)) => veldin_shots::aura_quads(table, svc, id),
        Some((veldin_hopper::REFERENCE_LEVEL, veldin_hopper::BEAM_FN)) => veldin_hopper::beam_quads(table, svc, id),
        Some((qwark_ship::REFERENCE_LEVEL, qwark_ship::BEAM_FN)) => qwark_ship::beam_quads(table, svc, id),
        Some((ship_fighter::REFERENCE_LEVEL, ship_fighter::TRAIL_FN)) | Some((ship_fighter::FLEET_LEVEL, ship_fighter::FLEET_TRAIL_FN)) => ship_fighter::trail_quads(table, svc, id),
        _ => None,
    }
}

/// The quad groups of the draw callback unit row `i` registered for moby `id` (`Callback::UnitQuads(i)`): the rows
/// whose callback draws more than one texture or blend (1471's beams), else [`fx_quads`]'s one group.
pub fn fx_quad_groups(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<FxQuads> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::BEAM_FN)) => veldin_beamer::fx_quad_groups(svc),
        Some((energy_fan::REFERENCE_LEVEL, energy_fan::DRAW_FN)) => energy_fan::fx_quad_groups(table, svc, id),
        Some((1, crate::shadows::BLOB_FN)) => {
            let quads = svc.blobs.1.iter().filter(|b| b.0 == id).map(|(_, b)| {
                let (corners, st) = crate::shadows::blob_quad(b);
                FxQuad { corners, st, rgba: [0x4080_8080; 4] }
            }).collect();
            vec![FxQuads { fx: 0, additive: false, quads }]
        }
        Some((super::burning_wreck::REFERENCE_LEVEL, super::burning_wreck::DRAW_FN)) => super::burning_wreck::fx_quads(table, svc, id).into_iter().collect(),
        Some((veldin_finale_fx::REFERENCE_LEVEL, f)) if [veldin_finale_fx::FLASH_FN, veldin_finale_fx::GLOW_FN, veldin_finale_fx::BEAM_FN, veldin_finale_fx::MORPH_FN].contains(&f) => veldin_finale_fx::fx_quad_groups(svc, f),
        _ => fx_quads(table, svc, i, id).into_iter().collect(),
    }
}

/// The glow quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitGlow(i)`; draw only).
pub fn glow_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<GlowQuad> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((lamp::REFERENCE_LEVEL, lamp::UPDATE_FN)) => lamp::glow_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::GLOW_FN)) => hover_zapper::glow_quads(table, id),
        Some((veldin_diver::REFERENCE_LEVEL, veldin_diver::UPDATE_FN)) => veldin_diver::glow_quads(table, svc, id),
        Some((veldin_boss::REFERENCE_LEVEL, veldin_boss::DRAW_FN)) => veldin_boss::glow_quads(table, svc, id),
        Some((veldin_hopper::REFERENCE_LEVEL, veldin_hopper::GLOW_FN)) => veldin_hopper::glow_quads(table, svc, id),
        Some((path_ship::REFERENCE_LEVEL, path_ship::UPDATE_FN)) => path_ship::glow_quads(table, svc, id),
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::EYE_FN)) => veldin_beamer::glow_quads(table, svc, id),
        Some((veldin_finale_fx::REFERENCE_LEVEL, veldin_finale_fx::SEAT_FN)) => veldin_finale_fx::glow_quads(table, svc),
        Some((super::mouse::REFERENCE_LEVEL, super::mouse::GLOW_FN)) => super::mouse::glow_quads(table, id),
        _ => Vec::new(),
    }
}

/// The state part of the unit draw callback row `i` run for moby `id` by the frame's callbacks
/// (`Callback::UnitFrame(i)`, `draw_callbacks::run_frame`): its `rand` draws and what it leaves for the renderer.
pub fn frame_callback(w: &mut World, i: u16, id: MobyId) {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((veldin_pads::REFERENCE_LEVEL, veldin_pads::COUNTDOWN_FN)) => veldin_pads::countdown_draw(w, id),
        Some((hoven_turret::REFERENCE_LEVEL, hoven_turret::HUD_FN)) => hoven_turret::hud_frame(w, id),
        Some((gemlik_ship::REFERENCE_LEVEL, gemlik_ship_hud::HUD_FN)) => gemlik_ship_hud::hud_frame(w, id),
        Some((pokitaru_jet::REFERENCE_LEVEL, pokitaru_jet_hud::HUD_FN)) => pokitaru_jet_hud::hud_frame(w, id),
        Some((fleet_ship::REFERENCE_LEVEL, fleet_ship_hud::HUD_FN)) => fleet_ship_hud::hud_frame(w, id),
        Some((veldin_beamer::REFERENCE_LEVEL, veldin_beamer::BEAM_FN)) => veldin_beamer::frame(w, id),
        _ => {}
    }
}
