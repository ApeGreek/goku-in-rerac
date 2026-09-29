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

/// The glow quads of the draw callback unit row `i` registered for moby `id` (`Callback::UnitGlow(i)`; draw only).
pub fn glow_quads(table: &crate::moby_runtime::MobyTable, svc: &crate::moby_update::Services, i: u16, id: MobyId) -> Vec<GlowQuad> {
    match PORTS.get(i as usize).map(|u| (u.level, u.func)) {
        Some((lamp::REFERENCE_LEVEL, lamp::UPDATE_FN)) => lamp::glow_quads(table, svc, id),
        _ => Vec::new(),
    }
}
