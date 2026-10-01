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
pub mod marker;
pub mod smoke_emitter;
pub mod hydro_pad;
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
}

impl Globals {
    /// The word at `addr` (0 until written: the words these ports use start at 0 in the data).
    pub fn word(&self, addr: u32) -> u32 { self.words.get(&addr).copied().unwrap_or(0) }
    pub fn set_word(&mut self, addr: u32, v: u32) { self.words.insert(addr, v); }
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
        Some((laser_fence::REFERENCE_LEVEL, laser_fence::UPDATE_FN)) => laser_fence::fx_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::ARC_FN)) => hover_zapper::fx_quads(table, svc, id),
        Some((quartu_drone::REFERENCE_LEVEL, quartu_drone::UPDATE_FN)) => quartu_drone::fx_quads(table, svc, id),
        Some((swing_laser::REFERENCE_LEVEL, swing_laser::UPDATE_FN)) => swing_laser::fx_quads(table, svc, id),
        Some((kalebo_barrier::REFERENCE_LEVEL, kalebo_barrier::UPDATE_FN)) => kalebo_barrier::fx_quads(table, svc, id),
        Some((sweep_light::REFERENCE_LEVEL, sweep_light::UPDATE_FN)) => sweep_light::fx_quads(table, svc, id),
        Some((light_fixture::REFERENCE_LEVEL, light_fixture::UPDATE_FN)) => light_fixture::fx_quads(table, svc, id),
        Some((water_laser::REFERENCE_LEVEL, water_laser::UPDATE_FN)) => water_laser::fx_quads(table, svc, id),
        _ => None,
    }
}

/// The glow quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitGlow(i)`; draw only).
pub fn glow_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<GlowQuad> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((lamp::REFERENCE_LEVEL, lamp::UPDATE_FN)) => lamp::glow_quads(table, svc, id),
        Some((hover_zapper::REFERENCE_LEVEL, hover_zapper::GLOW_FN)) => hover_zapper::glow_quads(table, id),
        _ => Vec::new(),
    }
}
