//! The hand item of the hero (slot 0 of the item slots at `0x1403e0`): creation `HeroItemsCreate` 0x22f3c0
//! (hand part), placement `HeroItemsAttach` 0x22fec0 (hand part), the slot loop `0x231088` (slot 0), the
//! hand swap `UpdateWrenchSelected` 0x2307e0 (`hand 0`), the delete `0x2305e8`, and the per-tick call of
//! the item moby's update (the wrench: [`super::melee::wrench_update`], 0x2be1c0). Spec:
//! `docs/plan/player_controller.md` "Melee and item swap".
//!
//! **Where the item moby lives.** In the game `CreateMoby` puts the hand item in a dynamic moby slot; the
//! port keeps it here, in the hero block's slot record ([`HandSlot::item`]), with its own animation state,
//! rows and position, and the engine draws it (`rc-engine` `moby_attach.rs`). So the dynamic-slot indices
//! the game would use for the hero's items are not taken from the moby table (inferred impact: none on the
//! mobys the scheduler creates, which reuse free slots by index).
//!
//! Globals that are not in the hero block but that the swap reads or writes (request 0x141408, the saved hand
//! item 0x141660, the previous item 0x15ed8c, the wrench flag 0x15ed90) are in [`ItemGlobals`]; the engine
//! copies them from / to the persistent state and the session around each tick.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::services::HitTemplate;
use crate::pad::{button, PadState};
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame};

use super::anim::AnimCtl;
use super::physics::{ticks, V4};
use super::Hero;

/// Item ids (item definition records at 0x179f40, stride 0x4c).
pub mod item {
    pub const WRENCH: i32 = 8;
    pub const BOMB_GLOVE: i32 = 10;
}

/// Ratchet's joint lists the attach matrices are built from (`FUN_0022a940`: 0x208c70), indexed by the
/// item definition's attach word.
pub const HERO_LISTS: [usize; 9] = [0, 1, 2, 3, 4, 5, 6, 29, 30];

/// The fields of an item definition record (0x179f40 + 0x4c·id) the hand code reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemDef {
    /// +0x08: slot type (0 hand, 1 feet, 2 head, 3 back, 4/5 extras, −1 none).
    pub slot: i32,
    /// +0x0c: attach word (index into [`HERO_LISTS`]).
    pub attach: i32,
    /// +0x10: o_class of the item moby.
    pub o_class: i32,
    /// +0x18: byte copied to 0x1413fb at creation; also selects the Euler route of the attach when
    /// `iGpffff8a90` ≠ 0 (never on foot).
    pub b18: u8,
}

/// The weapon fields of an item definition (0x179f40 + 0x4c·id) the weapon draw `0x22ee08` reads
/// (`super::weapons::draw_weapon`); the engine hands them to the hero (`super::weapons::Weapons::defs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaponDef {
    /// +0x18 (the word) ≠ 0: the arm's blend is 10 ticks, else 11.
    pub w18: i32,
    /// +0x24 / +0x28 / +0x2c: the weapon's sequences of Ratchet (standing (a full `SetAnim`), moving and crouched
    /// (the arm layer)); −1 none.
    pub anims: [i32; 3],
    /// +0x30: copied to 0x1413fa (the arm stays up).
    pub w30: i32,
}

impl Default for WeaponDef {
    fn default() -> Self { WeaponDef { w18: 0, anims: [-1; 3], w30: 0 } }
}

/// Class data of one item moby class (a gadget class).
#[derive(Clone, Debug)]
pub struct ItemClass {
    pub o_class: i16,
    pub anim: MobyAnimClass,
    /// Class scale (`CreateMoby` copies class +0x24 to moby +0x2c).
    pub scale: f32,
    /// First byte list of each of the class's joint lists (root-to-joint chains), by list index.
    pub chains: Vec<Vec<u8>>,
}

/// What the item code needs from the level data, built by the engine at load.
#[derive(Clone, Debug, Default)]
pub struct ItemData {
    /// The 37 item definitions.
    pub defs: Vec<ItemDef>,
    /// Ratchet's chains for [`HERO_LISTS`] (by index into it; empty when his class lacks the list).
    pub hero_chains: Vec<Vec<u8>>,
    /// The item classes that can be created (the gadget classes).
    pub classes: Vec<ItemClass>,
}

impl ItemData {
    pub fn class(&self, o_class: i16) -> Option<&ItemClass> { self.classes.iter().find(|c| c.o_class == o_class) }
    pub fn def(&self, id: i32) -> ItemDef { self.defs.get(id.max(0) as usize).copied().unwrap_or_default() }
}

/// The globals outside the hero block that the hand swap uses (synced by the engine).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemGlobals {
    /// 0x141408: hand request (the quick-select ring / `GiveItem` write it; `SessionState::temp_hand`).
    pub request: i32,
    /// 0x141660: saved hand item (`equipped[0]`).
    pub saved: i32,
    /// 0x15ed8c: previous hand item (`last_hand_item`).
    pub previous: i32,
    /// 0x15ed90: the hand shows the wrench (`wrench_held`).
    pub wrench_flag: i32,
}

/// The item moby of the hand slot (moby fields the hand code uses).
#[derive(Clone, Debug)]
pub struct HandItem {
    /// +0xa6.
    pub o_class: i16,
    /// +0x20: the item's own state byte (wrench: 0 in hand, 10/11 thrown).
    pub mstate: u8,
    /// +0x50..: animation (MobyAnimAdvance / MobyAnimBlend on the class) and its blend snapshot.
    pub anim: AnimState,
    pub snapshot: Option<MobyFrame>,
    /// +0x2c class scale; +0x10 position; +0xc0 rows (PS2 float bits).
    pub scale: f32,
    pub position: [f32; 3],
    pub rows: [moby_anim::V4; 3],
    /// Wrench pvars used by the ported part of its update: +0x6c hit timer.
    pub hit_timer: i32,
    /// The thrown wrench's flight (its pvars +0x40..+0x7e and rotation while detached; [`super::comet`]).
    pub flight: super::comet::Flight,
}

/// Slot 0 of the item slot records at 0x1403e0 (stride 0x50).
#[derive(Clone, Debug, Default)]
pub struct HandSlot {
    /// +0x00: the item moby (0 = none).
    pub item: Option<HandItem>,
    /// +0x10: fire button mask (□ 0x80 for the wrench, ○ 0x20 otherwise).
    pub fire_mask: u32,
    /// +0x14: frame after which the next item may be created (`0x1403f4 < 0x15f3f8`).
    pub create_after: i32,
    /// +0x18 (s16): swap lockout timer (2 while swinging, 10 on a jump attack).
    pub swap_timer: i16,
    /// +0x1a: "detached" flag; +0x1b: put-away tick counter.
    pub detached: u8,
    pub putaway_ticks: u8,
    /// +0x1c: swap state (2 = the wrench update ran this tick in hand); +0x1d: timer reload.
    pub swap: u8,
    pub reload: u8,
    /// +0x20: ticks in state 2 (3 after a swap starts).
    pub ticks_ready: i32,
    /// +0x24: 2 = ready, 3 = being put away, 0 = none.
    pub state: i32,
    /// +0x28: the item id held (0x140408).
    pub id: i32,
    /// −0x20 (0x1403c0): the hand point `HeroItemsAttach` keeps for a detached item (`W.r3` of its attach list;
    /// the thrown wrench flies back to it). Native `f32`.
    pub hand_point: [f32; 3],
}

/// The hand-related hero-block fields besides the slot.
#[derive(Clone, Debug, Default)]
pub struct HeroItems {
    pub slot: HandSlot,
    /// 0x141424: target hand item (0 = none).
    pub target: i32,
    /// 0x141440: item to restore after the wrench; 0x14145c: restore pending.
    pub restore: i32,
    pub restore_pending: i32,
    /// 0x1413f6 / 0x1413f7 / 0x1413fc / 0x1413fe / 0x1413fb: hero item flags (0 in the on-foot states).
    pub f13f6: u8,
    pub f13f7: u8,
    pub f13fc: u8,
    pub f13fe: u8,
    pub f13fb: u8,
    /// 0x13f52a / 0x13f52c: item-specific timers set on a swap (`0x230720` / `0x2306c0`); 0x14161e.
    pub f52a: i16,
    pub f52c: i16,
    pub f161e: u8,
    /// A `MobyAnimBlend(hand, seq, frame, ticks)` the state code made (SetState runs without the class
    /// data); applied at the start of [`items_update`], before anything reads the item's animation.
    pub pending_blend: Option<(u8, i32, i32)>,
    /// Hit sounds the wrench update played (`PlayClassSound(FUN_002bda88(), 0, wrench)`, queued in
    /// `super::fx::HeroFx::item_sounds`): counted.
    pub hit_sounds: u32,
}

/// Hit delivery for the hero's attacks: the moby part of `CollLine_Fix` with a hit template and
/// `coll_sphere_mobys` 0x214468 (both on the port's stand-ins for moby collision).
pub trait HitSink {
    /// `coll_sphere_mobys(r, centre, flags, ignore, tmpl)`: records for the listed mobys; returns the first
    /// listed moby, if any.
    fn sphere(&mut self, table: &mut MobyTable, r: Pf, centre: V4, flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<MobyId>;
    /// `CollLine_Fix(a, b, flags, ignore, tmpl)`: `Some(moby)` for a hit (the moby, if one, gets the hit
    /// through `0x26e968`), `None` for no hit.
    fn line(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<Option<MobyId>>;
    /// A hit record for `target` (`FUN_0026eaa8` / `0x26e968`: unless its current record has a larger damage).
    fn deliver(&mut self, _table: &mut MobyTable, _target: MobyId, _tmpl: &HitTemplate) {}
    /// `CreateMoby(o_class)` 0x263390 from the hero's code (the bomb glove's bomb, class 121): a dynamic slot of the
    /// table with the class's init (the moby system's class data). None: no moby system / no free slot.
    fn create_moby(&mut self, _table: &mut MobyTable, _o_class: i16, _counter: u64) -> Option<MobyId> { None }
    /// `DeleteMoby` 0x2636c0 from the hero's code (and its grid removal where the sink has a grid).
    fn delete_moby(&mut self, table: &mut MobyTable, id: MobyId, counter: u64) { table.delete(id, counter); }
}

/// A sink that hits nothing (tests, no moby system).
pub struct NoHits;

impl HitSink for NoHits {
    fn sphere(&mut self, _: &mut MobyTable, _: Pf, _: V4, _: u32, _: Option<MobyId>, _: &HitTemplate) -> Option<MobyId> { None }
    fn line(&mut self, _: &mut MobyTable, _: V4, _: V4, _: u32, _: Option<MobyId>, _: &HitTemplate) -> Option<Option<MobyId>> { None }
}

/// Everything [`items_update`] reads besides the hero.
pub struct ItemEnv<'a> {
    pub data: &'a ItemData,
    pub pad: &'a PadState,
    /// `0x15f3f8`: the frame counter the creation gate compares with (the port passes the tick counter).
    pub frame: i32,
    /// Ratchet's moby (`0x1413d0`).
    pub hero_moby: MobyId,
    /// The level collision (world mesh) for the items' own lines (the thrown wrench's ground height and bounce;
    /// None: they hit nothing).
    pub coll: Option<&'a rc_formats::collision::Collision>,    /// The camera as the previous tick's update left it: position 0x167240 and forward 0x167450 (the Bomb Glove's
    /// first-person aim). None: no camera.
    pub camera: Option<([f32; 3], [f32; 3])>,
}

/// `FUN_0022de10(slot)` for the hand: gloves (10, 17, 20, 25) take Ratchet's hand pose instead of an
/// animation of their own.
pub fn is_glove(id: i32) -> bool { matches!(id, 10 | 17 | 20 | 25) }

impl HeroItems {
    /// `FUN_0022dda0(0)`: the hand moby when the slot is ready (state 2).
    pub fn ready_item(&self) -> Option<&HandItem> { if self.slot.state == 2 { self.slot.item.as_ref() } else { None } }
}

/// The hand-item part of the hero update (`HeroItemsUpdate` 0x231268 → create 0x22f3c0, attach 0x22fec0,
/// slot loop 0x231088), after the transitions and the write-back.
#[allow(clippy::too_many_arguments)]
pub fn items_update(hero: &mut Hero, g: &mut ItemGlobals, table: &mut MobyTable, anim: &dyn AnimCtl, rng: &mut Rng, env: &ItemEnv, hits: &mut dyn HitSink) {
    super::melee::jump_attack_shockwave(hero, table, env, hits);
    // 0x22f068 (the weapon arm's upkeep) runs in HeroItemsUpdate before the slots.
    super::weapons::arm_upkeep(hero, anim);
    apply_pending_blend(hero, env.data);
    create_hand(hero, g, env);
    attach_hand(hero, table, anim, env);
    slot_loop(hero, g, table, anim, rng, env, hits);
}

/// `MobyAnimBlend(item, seq, frame, n)` on a hand item (the item updates of super::gadgets).
pub(super) fn blend_item(it: &mut HandItem, data: &ItemData, seq: u8, frame: i32, n: i32) { blend(it, data, seq, frame, n) }

fn blend(it: &mut HandItem, data: &ItemData, seq: u8, frame: i32, n: i32) {
    if let Some(c) = data.class(it.o_class) { moby_anim::set_sequence(&mut it.anim, &c.anim, seq, frame, n, &mut it.snapshot); }
}

fn apply_pending_blend(hero: &mut Hero, data: &ItemData) {
    if let Some((seq, frame, n)) = hero.items.pending_blend.take() {
        if let Some(it) = hero.items.slot.item.as_mut() { blend(it, data, seq, frame, n); }
    }
}

/// `HeroItemsCreate` 0x22f3c0, hand part: when the slot is empty, the gate frame has passed and no request is
/// pending: item = target 0x141424, else the wrench while 0x15ed90, else the saved hand item, else the
/// wrench; `CreateMoby(def.o_class)`, state 2, fire mask □ (wrench, which also blends to sequence 1 over 1
/// tick) or ○. (The joint-modifier records 0x140c40.. it clears and the class decompression are not needed.)
fn create_hand(hero: &mut Hero, g: &ItemGlobals, env: &ItemEnv) {
    let it = &mut hero.items;
    if it.slot.item.is_some() || !(it.slot.create_after < env.frame) || !(g.request == 0 || g.request == 0x24) { return; }
    let mut id = it.target;
    if id == 0 {
        id = item::WRENCH;
        if g.wrench_flag == 0 {
            id = g.saved;
            if id == 0 { id = item::WRENCH; }
        }
    }
    let def = env.data.def(id);
    it.slot.id = id;
    let Some(class) = env.data.class(def.o_class as i16) else { return };
    let mut m = HandItem {
        o_class: class.o_class,
        mstate: 0,
        anim: AnimState::spawn(&class.anim),
        snapshot: None,
        scale: class.scale,
        position: [0.0; 3],
        rows: [[0; 4]; 3],
        hit_timer: 0,
        flight: Default::default(),
    };
    it.slot.state = 2;
    if id == item::WRENCH {
        it.slot.fire_mask = button::SQUARE;
        moby_anim::set_sequence(&mut m.anim, &class.anim, 1, 0, 1, &mut m.snapshot);
    } else {
        it.slot.fire_mask = button::CIRCLE;
    }
    it.f13fb = def.b18;
    it.slot.item = Some(m);
}

/// Ratchet's hand joints a glove's joints 1.. copy (the table at 0x17aa40 + 4, −1 terminated).
pub const GLOVE_JOINTS: [u8; 16] = [55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70];

/// `HeroItemPoseFromRatchet(table, out, class, 0, ratchet)` 0x22a9c8: one keyframe for the item class —
/// joint 0 the identity quaternion, joint k+1 Ratchet's local quaternion of `table[k]`; an inherited scale
/// record (flag 0x80) for each of those joints whose scale is not (1, 1, 1); a translation record where
/// Ratchet's local translation differs from the item's rest translation of joint k+1. Ratchet's local pose
/// comes from [`AnimCtl::pose_frame`] (the snapshot encoding: quaternions ×32768, scales ×4096, integer
/// translations, the same conversions `FUN_00221678` makes; inferred to round alike).
pub fn glove_frame(hero: &MobyFrame, class: &MobyAnimClass, table: &[u8]) -> MobyFrame {
    use rc_formats::moby_anim::{MobyFrameHeader, ScaleRec, TransRec};
    let n = table.len() + 1;
    let mut payload = Vec::new();
    for c in [0i16, 0, 0, 0x7fff] { payload.extend_from_slice(&c.to_le_bytes()); }
    for &j in table { for c in hero.quat_at(j as usize) { payload.extend_from_slice(&c.to_le_bytes()); } }
    let mut scales = Vec::new();
    for (k, &j) in table.iter().enumerate() {
        if let Some(r) = hero.scales.iter().find(|r| r.joint == j && r.inherited()) {
            if r.scale != [0x1000; 3] { scales.push(ScaleRec { scale: r.scale, joint: (k + 1) as u8, flags: 0x80 }); }
        }
    }
    let mut trans = Vec::new();
    for (k, &j) in table.iter().enumerate() {
        let t = hero.trans.iter().find(|r| r.joint == j).map(|r| r.trans);
        let Some(t) = t else { continue };
        let rest = class.rest.get(k + 1).map(|r| r.map(|c| c as i16)).unwrap_or([0; 3]);
        if t != rest { trans.push(TransRec { trans: t, joint: (k + 1) as u8, pad: 0 }); }
    }
    let quat_bytes = 8 * n;
    for r in &scales {
        for c in r.scale { payload.extend_from_slice(&c.to_le_bytes()); }
        payload.extend_from_slice(&[r.joint, r.flags]);
    }
    for r in &trans {
        for c in r.trans { payload.extend_from_slice(&c.to_le_bytes()); }
        payload.extend_from_slice(&[r.joint, r.pad]);
    }
    let qwc = ((payload.len() + 15) >> 4) as u16;
    payload.resize(qwc as usize * 16, 0);
    let header = MobyFrameHeader {
        rate: 0.0,
        time: 0,
        qwc,
        quat_bytes: quat_bytes as u16,
        scale_count: scales.len() as u16,
        trans_offset: (quat_bytes + 8 * scales.len()) as u16,
        trans_count: trans.len() as u16,
    };
    let quats = (0..n).map(|j| { let o = 8 * j; [0, 1, 2, 3].map(|c| i16::from_le_bytes([payload[o + 2 * c], payload[o + 2 * c + 1]])) }).collect();
    MobyFrame { header, quats, scales, trans, payload }
}

/// `HeroItemsAttach` 0x22fec0, hand part: position = `W.r3` of the attach list (`FUN_0022a940`'s matrices
/// from Ratchet's current pose); not a glove: `MobyAnimAdvance`, rows = `W.r0..r2`, columns normalised;
/// a glove: rows = W, no advance, no normalisation, and its pose is Ratchet's hand ([`glove_frame`]; frame
/// A = B = that keyframe, t = 0: the port shows it as the snapshot key).
fn attach_hand(hero: &mut Hero, table: &MobyTable, anim: &dyn AnimCtl, env: &ItemEnv) {
    let id = hero.items.slot.id;
    let detached = hero.items.slot.detached != 0;
    let Some(it) = hero.items.slot.item.as_mut() else { return };
    let def = env.data.def(id);
    let list = (def.attach.max(0) as usize).min(HERO_LISTS.len() - 1);
    let Some(chain) = env.data.hero_chains.get(list).filter(|c| !c.is_empty()) else { return };
    let Some(p) = anim.eval_chains_with(&[chain.as_slice()], &hero.weapons.layers).into_iter().next() else { return };
    let r = &table.mobys[env.hero_moby];
    let host_rows: [moby_anim::V4; 3] = [0, 1, 2].map(|i| r.rows[i].map(f32::to_bits));
    let w = moby_anim::attach_matrix(&p, &host_rows, [r.position[0], r.position[1], r.position[2]], r.scale);
    let hp = [w[3][0], w[3][1], w[3][2]];
    if detached {
        // A detached item (the thrown wrench): the hand point 0x1403c0 only, `MobyAnimAdvance`; the moby keeps its
        // own position and rotation.
        if let Some(c) = env.data.class(it.o_class) { moby_anim::advance(&mut it.anim, &c.anim); }
        hero.items.slot.hand_point = hp;
        return;
    }
    it.position = hp;
    let glove = is_glove(id);
    if !glove {
        if let Some(c) = env.data.class(it.o_class) { moby_anim::advance(&mut it.anim, &c.anim); }
    }
    let mut rows: [moby_anim::V4; 3] = [0, 1, 2].map(|i| w[i].map(f32::to_bits));
    if !glove { moby_anim::normalise_columns(&mut rows); }
    it.rows = rows;
    if glove {
        if let (Some(f), Some(c)) = (anim.pose_frame(), env.data.class(it.o_class)) {
            it.snapshot = Some(glove_frame(&f, &c.anim, &GLOVE_JOINTS));
            it.anim.seq_a = moby_anim::SNAPSHOT_SEQ;
            it.anim.frame_a = 0;
            it.anim.frame_b = 0;
            it.anim.t = 0.0;
        }
    }
    hero.items.slot.hand_point = hp;
}

/// The slot loop `0x231088` for slot 0.
fn slot_loop(hero: &mut Hero, g: &mut ItemGlobals, table: &mut MobyTable, anim: &dyn AnimCtl, rng: &mut Rng, env: &ItemEnv, hits: &mut dyn HitSink) {
    if hero.items.slot.item.is_none() {
        update_hand_selected(hero, g, rng, env);
        return;
    }
    match hero.items.slot.state {
        2 => {
            hero.items.slot.ticks_ready += 1;
            update_hand_selected(hero, g, rng, env);
            // (slot 0: the joint-modifier records 0x140ce0.. are refreshed by FUN_00227050: not ported.)
            if let Some(it) = hero.items.slot.item.as_mut() {
                if it.anim.flags & 2 != 0 && it.anim.seq_b == 0 { blend(it, env.data, 1, 0, 2); }
            }
        }
        3 => {
            hero.items.slot.putaway_ticks = hero.items.slot.putaway_ticks.wrapping_add(1);
            // Slot 0 deletes on the first tick in state 3 whether or not the put-away anim wrapped.
            delete_hand(hero, env.frame + 2);
            return;
        }
        _ => {}
    }
    // The item moby's update `(*moby+0x74)(moby)`: the hand item's row of super::gadgets::HAND_ITEMS (the
    // wrench's here; one that needs the hero's context runs right after the slot loop, gadgets::after_items).
    super::gadgets::slot_item_update(hero, table, anim, env, hits, rng);
}

/// `FUN_002305e8(0, frame)`: the slot is emptied (its update would run once more with state 3, which the
/// wrench ignores), the item moby deleted, the creation gate set to `frame`.
pub fn delete_hand(hero: &mut Hero, frame: i32) {
    let s = &mut hero.items.slot;
    s.create_after = frame;
    s.state = 0;
    s.id = 0;
    s.item = None;
}

/// `UpdateWrenchSelected(0)` 0x2307e0 for the hand: the swap. □ while holding another item → the wrench; ○
/// while holding the wrench → the saved item; a pending request 0x141408 → that item. A change (unless the
/// slot's lockout timer runs) starts the put-away: 0x140400 = 3, the fidget timer `rand_range(50, 90)`, the
/// wrench flag, the request cleared, the item blends to sequence 2 over 2 ticks, slot state 3.
pub fn update_hand_selected(hero: &mut Hero, g: &mut ItemGlobals, rng: &mut Rng, env: &ItemEnv) {
    let pressed = env.pad.pressed;
    let it = &mut hero.items;
    let held = it.slot.id;
    if held != 8 && held != 0x17 && held != 9 && it.slot.swap == 2 { it.slot.swap = 0; }
    if it.f13f7 != 0 && held != 8 { it.f13f6 = 1; }
    if it.f13f7 != 0 && held == 8 {
        if g.request == 0x1f { g.request = 0; }
        if g.request != 0x18 && g.request != 0x24 { return; }
    }
    if it.f13fc != 0 && (it.f13f6 == 0 || held == 8) {
        it.f13f6 = 0;
        return;
    }
    let mut changed = false;
    if !(pressed & button::SQUARE == 0 && it.f13f6 == 0) && held != 8 {
        if it.f13f6 != 0 && held != 0 && held != 0x1f { it.restore = held; }
        it.target = 8;
        changed = true;
    }
    if pressed & button::CIRCLE != 0 && it.f13f7 == 0 && held != g.saved && g.saved != 0 {
        it.target = g.saved;
        changed = true;
    }
    if it.restore_pending != 0 {
        it.restore_pending = 0;
        let mut v = it.restore;
        if v != 0 && v != held {
            if v == 0x26 { v = 0; }
            it.restore = 0;
            g.saved = v;
            it.target = v;
            changed = true;
        }
    }
    // 0x141345 (Drone Device) and request 0x1f (FUN_00230770 path) are not reachable on foot here.
    let r = g.request;
    if r != 0 {
        if r == it.target {
            g.request = 0;
        } else {
            if r == 0x26 {
                it.target = 0;
                g.saved = 0;
            } else {
                if r != 0x24 && held != 8 { g.previous = g.saved; }
                it.target = r;
                if r != 0x24 { g.saved = r; }
            }
            changed = true;
        }
    }
    if it.slot.swap == 2 { changed = false; }
    if it.slot.swap == 1 && it.slot.swap_timer == 0 { it.slot.swap_timer = it.slot.reload as i16; }
    if crate::moby_update::services::fast_dec_timer_s16(&mut it.slot.swap_timer) == 0 { changed = false; }
    if !changed { return; }
    it.slot.ticks_ready = 3;
    // FUN_0022b8e8: clears the head-look angles 0x140354/58 and the idle secondaries' targets 0x140374...
    hero.idle.clear_look();
    hero.fidget_timer = rng.rand_range(ticks(50), ticks(90));
    let it = &mut hero.items;
    let t = it.target;
    if matches!(t, 9 | 0xb | 0xd | 0xe | 0xf | 0x10 | 0x12 | 0x13 | 0x16 | 0x17 | 0x1a) { it.f52a = ticks(0x46) as i16; }
    if matches!(t, 9 | 0xb | 0xd | 0x10 | 0x17) {
        it.f161e = 3;
        it.f52c = ticks(0x46) as i16;
    }
    // 0x1413f8 (weapon out) is 0 here: FUN_0022efd8 not reached.
    g.wrench_flag = (t == item::WRENCH) as i32;
    it.slot.detached = 0;
    g.request = 0;
    it.f13f6 = 0;
    if let Some(m) = it.slot.item.as_mut() { blend(m, env.data, 2, 0, 2); }
    it.slot.state = 3;
}
