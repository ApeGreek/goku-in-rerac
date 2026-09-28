//! **Package P6 — the hand items, one mechanism.** The game's per-tick hand-0 logic, general over every hand
//! item (docs/plan/hero_states.md "P6"):
//!
//! 1. The request `0x141408` (the quick-select ring, the double-tap △, `GiveItem`, the debug give) is taken by
//!    `UpdateWrenchSelected(0)` 0x2307e0 in the slot loop, which puts the held item away and creates the new one
//!    (`items::update_hand_selected`, `items::create_hand`): the item held is `0x140408` = the slot's id.
//! 2. The weapon check `HeroPdaGadget` 0x240ed8 (the first call of the transitions after the hit intake) runs the
//!    held item's **fire** case: [`HandItemKind::fire`] from [`HAND_ITEMS`] ([`pda_item`]), then — when the state
//!    did not change — the Thruster hover epilogue ([`super::packs::pda_epilogue`]). The early returns before
//!    the switch (0x1413fc for a non-wrench, not Ratchet, no hand moby, the slot not ready) skip the epilogue, as
//!    the game does (level00 0x227fa0 / level01 0x240ed8 disassembly).
//! 3. The item moby's own update `(*moby+0x74)(moby)` in the slot loop 0x231088: [`HandItemKind::update`] —
//!    [`ItemUpdate::Slot`] runs inside `items::slot_loop` (the wrench's hit window), [`ItemUpdate::Hero`] needs the
//!    hero's context (SetState, the collision lines: the Swingshot's hook) and runs right after the slot loop
//!    ([`after_items`], called by the tick), which is the same point of the frame.
//!
//! A new gadget or weapon adds one row to [`HAND_ITEMS`] (its fire case and / or its item update); nothing else in
//! the hero or the tick changes.
//!
//! Ported rows: the wrench (item 8, fire in [`super::melee`] ahead of the table, update `melee::wrench_update`, its
//! thrown flight [`super::comet`]), the Swingshot (item 12, [`super::swingshot`]) and the Bomb Glove (item 10, the
//! throw gloves' fire case and the glove's update: [`super::weapons`]) and the Pyrocitor (item 16, no fire case: its
//! update fires it, [`super::pyrocitor`]); the guns (docs/plan/hero_gameplay.md §9, shared parts in [`super::guns`]):
//! the Blaster (item 15, case 0xf draws it, [`super::blaster`]), the R.Y.N.O. (23, [`super::ryno`]), the Devastator
//! (11, [`super::devastator`]) and the Tesla Claw (19, [`super::tesla`]), their updates firing. The game's other
//! cases, not ported yet: the other throw weapons 0x11 / 0x14 / 0x18 / 0x19 (→ 0x23 / `0x22ee08`: their updates),
//! 0x12 (→ 0x20), 0x15, the Hologuise
//! 0x1f (the 18-tick timer 0x14162e), the PDA 0x20 (`OpenVendorMenu`); the holster check 0x2405f8.

use super::items::{HitSink, ItemData, ItemEnv};
use super::states::Ctx;
use super::Hero;
use crate::moby_runtime::MobyTable;

/// How the port runs a hand item's moby update.
#[derive(Clone, Copy)]
pub enum ItemUpdate {
    /// Not ported (the item only follows the hand).
    None,
    /// Inside the slot loop (no hero context needed).
    Slot(fn(&mut Hero, &mut MobyTable, &dyn super::anim::AnimCtl, &ItemEnv, &mut dyn HitSink, &mut crate::rng::Rng)),
    /// Right after the slot loop, with the hero's context.
    Hero(fn(&mut Hero, &mut Ctx, &ItemData)),
}

/// One hand item: its case of the weapon check and its moby update.
#[derive(Clone, Copy)]
pub struct HandItemKind {
    /// Item id (`0x140408`).
    pub id: i32,
    pub name: &'static str,
    /// The weapon check's case for the item (None: not ported; only the epilogue runs).
    pub fire: Option<fn(&mut Hero, &mut Ctx)>,
    pub update: ItemUpdate,
}

/// The hand items the port knows (see the module doc).
pub static HAND_ITEMS: [HandItemKind; 10] = [
    HandItemKind { id: super::items::item::WRENCH, name: "wrench", fire: None, update: ItemUpdate::Slot(super::melee::wrench_update) },
    HandItemKind { id: super::swingshot::SWINGSHOT, name: "Swingshot", fire: Some(super::swingshot::fire), update: ItemUpdate::Hero(super::swingshot::item_update) },
    // The throw gloves' case of the weapon check (0x23 / the arm) and the Bomb Glove's update 0x2d8330 (super::weapons).
    HandItemKind { id: super::items::item::BOMB_GLOVE, name: "Bomb Glove", fire: Some(super::weapons::fire), update: ItemUpdate::Slot(super::weapons::glove_update) },
    // No weapon-check case: the Pyrocitor's update 0x2cd458 fires it (super::pyrocitor).
    HandItemKind { id: super::pyrocitor::PYROCITOR, name: "Pyrocitor", fire: None, update: ItemUpdate::Slot(super::pyrocitor::update) },
    // The weapon check's case 0xf draws it; its update 0x2ca610 fires (super::blaster).
    HandItemKind { id: super::blaster::BLASTER, name: "Blaster", fire: Some(super::blaster::fire), update: ItemUpdate::Slot(super::blaster::update) },
    // No weapon-check case: its update 0x2e4e60 fires the salvo (super::ryno).
    HandItemKind { id: super::ryno::RYNO, name: "R.Y.N.O.", fire: None, update: ItemUpdate::Slot(super::ryno::update) },
    // No weapon-check case: its update 0x2c7d68 fires (super::devastator).
    HandItemKind { id: super::devastator::DEVASTATOR, name: "Devastator", fire: None, update: ItemUpdate::Slot(super::devastator::update) },
    // No weapon-check case: its update 0x2ce448 fires the beam (super::tesla).
    HandItemKind { id: super::tesla::TESLA, name: "Tesla Claw", fire: None, update: ItemUpdate::Slot(super::tesla::update) },
    // No weapon-check case: its update 0x303000 pulls and fires (super::suck_cannon, through creature::react).
    HandItemKind { id: super::suck_cannon::SUCK_CANNON, name: "Suck Cannon", fire: None, update: ItemUpdate::Slot(super::suck_cannon::update) },
    // No weapon-check case: its update 0x2ccb78 whistles and lures (super::taunter).
    HandItemKind { id: super::taunter::TAUNTER, name: "Taunter", fire: None, update: ItemUpdate::Slot(super::taunter::update) },
];

/// The row of item `id`.
pub fn hand_item(id: i32) -> Option<&'static HandItemKind> { HAND_ITEMS.iter().find(|k| k.id == id) }

/// The hand-item fields the mechanism keeps between the slot loop and [`after_items`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gadgets {
    /// The slot loop reached the item's update and it is an [`ItemUpdate::Hero`] one: [`after_items`] runs it.
    pub pending: Option<i32>,
}

/// `HeroPdaGadget` 0x240ed8 for a ready hand item other than the wrench (the slot checks have passed). `t0` is
/// the state timer at entry; returns true when the state changed (`timer < t0`).
pub(super) fn pda_item(h: &mut Hero, c: &mut Ctx, t0: i32) -> bool {
    if let Some(f) = hand_item(h.items.slot.id).and_then(|k| k.fire) { f(h, c); }
    if h.timer == t0 { super::packs::pda_epilogue(h, c); }
    h.timer < t0
}

/// The holster check 0x2405f8 (after the water checks). True when it changed the state. Not ported: it acts on a
/// weapon drawn in the weapon stances (0x1413f8), which no ported item sets.
pub(super) fn holster_check(h: &mut Hero, c: &mut Ctx) -> bool { super::weapons::holster_check(h, c) }

/// The item moby's update in the slot loop 0x231088 (`(*moby+0x74)(moby)`), for the hand item of the slot.
pub(super) fn slot_item_update(hero: &mut Hero, table: &mut MobyTable, anim: &dyn super::anim::AnimCtl, env: &ItemEnv, hits: &mut dyn HitSink, rng: &mut crate::rng::Rng) {
    let id = hero.items.slot.id;
    match hand_item(id).map(|k| k.update) {
        Some(ItemUpdate::Slot(f)) => {
            // The wrench's update belongs to its class (0x47), as before the table.
            if id != super::items::item::WRENCH || hero.items.slot.item.as_ref().is_some_and(|it| it.o_class == 0x47) { f(hero, table, anim, env, hits, rng); }
        }
        Some(ItemUpdate::Hero(_)) => hero.gadgets.pending = Some(id),
        _ => {}
    }
}

/// Right after the hand's slot loop (`HeroItemsUpdate` 0x231268, same point of the frame): the pending item
/// update that needs the hero's context, and the item fields of an item that is gone. `data`: the hand classes.
pub fn after_items(hero: &mut Hero, c: &mut Ctx, data: &ItemData) {
    let pending = hero.gadgets.pending.take();
    if let Some(ItemUpdate::Hero(f)) = pending.and_then(hand_item).map(|k| k.update) { f(hero, c, data); }
    // The SetState an item update made inside the slot loop (the Bomb Glove's 1 → 0x1e).
    super::weapons::after_items(hero, c);
    if hero.items.slot.item.is_none() || hero.items.slot.id != super::swingshot::SWINGSHOT { super::swingshot::item_gone(hero); }
}

/// The hand item's class sounds of this tick (`PlayClassSound(index, 0, item)` inside its update: the wrench's hit,
/// the Swingshot's fire / hit / pull), played through the hero's sound layer right after the item's update (the
/// tick calls it after [`after_items`]), in the order the update made them. No hand moby: dropped.
///
/// Then Ratchet's own sounds of the item update (`fx.item_voices`, `moby` = Ratchet: the catch's voice, the thrown
/// wrench's whoosh loop and its release, the bomb glove's throw voice), in order.
pub fn flush_item_sounds(h: &mut Hero, moby: &crate::moby_runtime::Moby, sounds: &mut dyn super::HeroSounds, rng: &mut crate::rng::Rng) {
    let mut list = std::mem::take(&mut h.fx.item_sounds);
    list.append(&mut h.swing.item.sounds);
    if let Some(item) = h.items.slot.item.as_ref() {
        let (o_class, pos) = (item.o_class, item.position);
        for index in list { sounds.item_sound(o_class, pos, index, 0, rng); }
    }
    let item = h.items.slot.item.as_ref().map(|it| (it.o_class, it.position));
    for cmd in std::mem::take(&mut h.fx.item_voices) {
        match cmd {
            super::packs::SoundCmd::Loop { n, sound } => h.packs.loops[n] = sounds.voice(moby, sound, 4, rng),
            super::packs::SoundCmd::Voice { index, flags } => { sounds.voice(moby, index, flags, rng); }
            super::packs::SoundCmd::Release { slot } => sounds.release(moby, slot),
            super::packs::SoundCmd::ItemLoop { n, index, flags } => {
                let alive = h.fx.item_loops[n].is_some_and(|s| sounds.alive(s));
                if let (false, Some((o_class, pos))) = (alive, item) {
                    let s = sounds.item_sound(o_class, pos, index, flags, rng);
                    h.fx.item_loops[n] = (s >= 0).then_some(s);
                }
            }
            super::packs::SoundCmd::ItemRelease { n } => {
                if let Some(s) = h.fx.item_loops[n].take() { sounds.release(moby, s); }
            }
            super::packs::SoundCmd::MobySound { moby: id, o_class, pos, index, flags } => { sounds.moby_sound(id, o_class, pos, index, flags, rng); }
        }
    }
    for n in 0..h.fx.item_loops.len() { h.fx.item_loop_alive[n] = h.fx.item_loops[n].is_some_and(|s| sounds.alive(s)); }
}
