//! Visibomb range limiters, class 832: `RcRangeLimiterUpdate` level01 0x302648, the same code on levels 1–15 and 18
//! (one instance each). A hidden controller that keeps the Visibomb's guided missile (class 172, `0xac`) inside the
//! level's allowed airspace while Ratchet steers it (hero state `0x1d`): outside every allowed area a counter runs;
//! while it runs the screen shows static (draw callback `0x302438` on list 2, the texture-effect quad grid whose
//! alpha grows with the counter over 90 ticks) and past 90 ticks the missile's flight is ended (`0x2cb788`, the
//! Visibomb's own code). Read from the level01 decomp (0x302648, the draw callback 0x302438).
//!
//! **Pvar block**: 20 area records of 4 s32 words {path (−1: none), f32 floor, f32 ceiling, cuboid (−1: none)}
//! (0x140 bytes), then P[0x50] (+0x140) the out-of-range counter.
//!
//! **Update** (update distance 0xff every tick): unless Ratchet is in state 0x1d and a class-172 moby exists, the
//! counter is 0. Otherwise the counter is incremented and the records are walked in order: the missile (its
//! position) is inside a record when (it has a path, floor < z < ceiling and the point is in the path polygon) or
//! it is in the record's cuboid; the first record that holds it zeroes the counter. Then, with the counter ≥ 1:
//! the static overlay is registered (when the draw flag gp−0x78f4 is set) and, past `ticks(90)`, the missile's
//! flight ends.
//!
//! In the port the Visibomb is not ported (no class 172 is ever created), so the controller stays at 0; the
//! overlay draw and `0x2cb788` are counted as unported when reached ([`crate::moby_update::Services::unported`]).

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x302648;
/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [832];

/// The Visibomb missile's class (`0xac`).
pub const MISSILE_CLASS: i16 = 172;
/// Ratchet's state while steering it (`0x1413d4 == 0x1d`).
pub const STEERING_STATE: i32 = 0x1d;
/// Area records.
pub const RECORDS: usize = 20;
/// The counter's pvar offset (P[0x50]).
pub const COUNTER: usize = 0x140;
/// Ticks out of range before the flight ends (`ticks(0x5a)`).
pub const LIMIT: i32 = 90;

/// The first live class-172 moby (the game walks its moby list at `0x15ffe4` for it).
fn missile(w: &World) -> Option<MobyId> { w.table.mobys.iter().position(|m| m.o_class == MISSILE_CLASS && m.state < 0x80) }

/// Whether `q` is in one of the records (module doc).
pub fn in_range(w: &World, pv: &[u8], q: [f32; 3]) -> bool {
    (0..RECORDS).any(|k| {
        let r = 0x10 * k;
        let path = p::i32(pv, r);
        if path != -1 && p::ff(pv, r + 4) < q[2] && q[2] < p::ff(pv, r + 8) && w.in_path(q, path) { return true; }
        let c = p::i32(pv, r + 0xc);
        c != -1 && w.in_cuboid(q, c)
    })
}

/// `RcRangeLimiterUpdate` 0x302648 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < COUNTER + 4 { return; }
    w.mm(id).update_dist = 0xff;
    let m = if w.hero.state == STEERING_STATE { missile(w) } else { None };
    let Some(m) = m else {
        p::set_i32(&mut w.mm(id).pvars, COUNTER, 0);
        return;
    };
    let q = { let x = w.m(m).position; [x[0], x[1], x[2]] };
    let mut n = p::i32(&w.m(id).pvars, COUNTER).wrapping_add(1);
    if in_range(w, &w.m(id).pvars, q) { n = 0; }
    p::set_i32(&mut w.mm(id).pvars, COUNTER, n);
    if n < 1 { return; }
    w.svc.unported("832 static overlay 0x302438");
    if n <= w.ticks(LIMIT) { return; }
    w.svc.unported("832 end of the Visibomb flight 0x2cb788");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moby_runtime::{Moby, MobyTable};

    #[test]
    fn idle_without_the_missile() {
        let mut m = Moby { state: 1, pvars: vec![0xff; COUNTER + 4], ..Moby::default() };
        p::set_i32(&mut m.pvars, COUNTER, 7);
        let mut t = MobyTable::new(vec![m], 4);
        let mut hero = crate::hero::Hero::new();
        hero.state = STEERING_STATE;
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
        update(&mut w, 0);
        assert_eq!((p::i32(&w.m(0).pvars, COUNTER), w.m(0).update_dist), (0, 0xff));
    }
}
