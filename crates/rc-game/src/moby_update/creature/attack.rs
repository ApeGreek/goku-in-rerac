//! Hits a creature deals and the group alert (level01 `0x26eaa8`, `0x26e090`).
//!
//! A creature attacks Ratchet the way every moby does: a hit record for his moby in the hit log (`0x178580`), which
//! the tick hands to the hero (`MobySystem::hit_message`) and P2's intake (`crate::hero::damage::hit_intake`) turns
//! into the hurt state, the knockback and the health loss. Nothing here touches the hero block.

use super::V;
use crate::moby_runtime::MobyId;
use crate::moby_update::services::{HitRecord, World};
use crate::ps2v::Pf;

/// `0x26eaa8(damage, target, attacker, flags, &pos, &dir)`: a hit record for `target` in the next log slot
/// (`0x1742d4`), unless the target's current record (+0xa4, for it) carries more damage (`damage < old` skips).
/// The record: +0x00 `pos`, +0x10 `dir`, +0x20 attacker, +0x24 flags, +0x2c damage, +0x30 = 1 when `|dir| > 0.0001`
/// (Ratchet's intake then pushes along `dir`), +0x34 target, +0x38 0. The type bytes +0x28..+0x2b are not written by
/// the game (whatever the slot held); the port writes 0.
pub fn hit_moby(w: &mut World, target: MobyId, attacker: MobyId, damage: f32, flags: u32, pos: V, dir: V) {
    let slot = w.m(target).hit_slot;
    if slot != 0xff {
        if let Some(old) = w.svc.hits.records.get(slot as usize) {
            if old.target == target && damage < f32::from_bits(old.damage.0) { return; }
        }
    }
    let next = w.svc.hits.next;
    let p = |v: V| v.map(Pf::f);
    w.svc.hits.records[next as usize] = HitRecord {
        pos: p(pos),
        dir: p(dir),
        attacker: Some(attacker),
        flags,
        b28: 0,
        b29: 0,
        h2a: 0,
        damage: Pf::f(damage),
        w30: (super::len3(dir) > 0.0001) as u32,
        target,
        prim: 0,
    };
    w.mm(target).hit_slot = next;
    w.svc.hits.next = (next + 1) & 0x3f;
}

/// `0x26e090(group, v)`: `+0xbc = v` on every member of moby group `group` (`0x1abcc0[group]`): the creatures'
/// "alert the others" (577 sets 1 when it notices Ratchet or is hit, 2 from its thrown state).
pub fn group_command(w: &mut World, group: i8, v: u8) {
    if group < 0 { return; }
    let Some(Some(list)) = w.svc.groups.lists.get(group as usize) else { return };
    let list = list.clone();
    for e in list {
        if let Some(m) = w.table.mobys.get_mut((e & 0x7fff) as usize) { m.cmd = v; }
    }
}

/// `coll_sphere_mobys(r, centre, 0x10, attacker, 0)` then `0x26f8f8(damage, push, push_z, attacker, &centre, list, n,
/// ignore, flags, type, subtype)`: every moby the sphere lists (but `ignore`) gets a hit (`0x26e968`) pushing it away
/// from `centre` — dir `(cos a·push, sin a·push, push_z)` with `a` the heading from `centre` to it and the exact-push
/// marker w = 5627.925 — from `attacker` (its class at +0x1a), `flags` at +0x14 and +0x20, the type bytes, `damage`.
/// The chicken 270's gold burst uses it (the Bomb Glove's and the Devastator's blasts call the same code with their
/// own values). Returns the number of mobys listed.
#[allow(clippy::too_many_arguments)]
pub fn area_hit(w: &mut World, r: f32, centre: V, attacker: MobyId, damage: f32, push: f32, push_z: f32, ignore: Option<MobyId>, flags: u32, b18: u8, b19: u8) -> usize {
    use crate::moby_update::services::{pf, pv, sphere_mobys_in};
    let list = sphere_mobys_in(w.table, w.svc, w.classes, pf(r), pv(centre), 0x10, Some(attacker), None);
    area_push(w, &list, centre, attacker, damage, push, push_z, ignore, flags, b18, b19);
    list.len()
}

/// `0x26f8f8(damage, push, push_z, attacker, &centre, list, n, ignore, flags, type, subtype)` on a list a caller made
/// itself (the Glove of Doom's bots list with their own sphere, `coll_sphere_mobys(1, pos + 0.5 up, 0x15)`, and push
/// from their position): each listed moby but `ignore` gets the hit of [`area_hit`].
#[allow(clippy::too_many_arguments)]
pub fn area_push(w: &mut World, list: &[MobyId], centre: V, attacker: MobyId, damage: f32, push: f32, push_z: f32, ignore: Option<MobyId>, flags: u32, b18: u8, b19: u8) {
    use crate::moby_update::services::{pf, HitTemplate};
    let class = w.m(attacker).o_class as u16;
    for &t in list {
        if Some(t) == ignore { continue; }
        let q = w.m(t).position;
        let a = super::atan(q[0] - centre[0], q[1] - centre[1]);
        let tmpl = HitTemplate { dir: [pf(a.cos() * push), pf(a.sin() * push), pf(push_z), Pf::b(0x45af_df66)], attacker: Some(attacker), flags, b18, b19, h1a: class, damage: pf(damage), w20: flags };
        w.deliver_hit(t, &tmpl);
    }
}
