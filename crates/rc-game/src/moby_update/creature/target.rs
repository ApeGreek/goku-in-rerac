//! Target acquisition `0x274b78(range, moby, &out)`: whom a creature goes for this tick.
//!
//! * Ratchet (`0x1413d0`) by default, **no target** while his movement group (`0x1413dc`) is 0x18 or his state
//!   (`0x1413d4`) is 0x72 (the cutscene hold): kind 2 then, and every field of the record is zero.
//! * A decoy nearer than `range` (the **xy** distance, `fun_001f9b80` = `VecDistance2` 0x221398; the port read it as
//!   3-D until 2026-09-28, hero_gameplay.md §18) wins over him (kind 1): the mobys of list `0x1b0cb0`
//!   of class 0xcb / 0x76c in state 3, then the 20 slots of `0x1dd580` of class 0x10e with `+0xbc ≠ 0`; each closer
//!   one replaces the previous and shrinks the range. The port has no writer of those two gadget lists, so it scans
//!   the moby table in index order for the same classes and states (the same candidates; only the order of equally
//!   distant decoys could differ). None exist on Novalis.
//! * The record: +0x00 the target's position (moby +0x10), +0x10 its Euler (+0x40), +0x20 the aim point and +0x30
//!   the body point: for Ratchet `0x13f410` / `0x13f420` (the hero block's shadow and body points), for a decoy its
//!   position 0.6 up (both); +0x40 the moby, +0x44 the kind.

use super::V;
use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

/// The target record (`out` of 0x274b78).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Target {
    pub pos: V,
    pub rot: V,
    pub aim: V,
    pub body: V,
    /// +0x40 (None: no target; every vector is then zero).
    pub moby: Option<MobyId>,
    /// +0x44: 0 Ratchet, 1 a decoy, 2 none (Ratchet in state 0x72 / group 0x18).
    pub kind: u32,
}

/// Decoy classes of list `0x1b0cb0` (need state 3) and of `0x1dd580` (need `+0xbc ≠ 0`).
pub const DECOY_STATE3: [i16; 2] = [0xcb, 0x76c];
pub const DECOY_CMD: i16 = 0x10e;

/// `0x274b78(range, id)`.
pub fn acquire(w: &World, id: MobyId, range: f32) -> Target { acquire_in(w, id, range, None) }

/// `0x274df8(range, id, &out, cuboids, 0, points, count)` with a polygon region (spline `region`, its points as a 2-D
/// polygon, [`super::region::point_in_polygon`]): as [`acquire`], but Ratchet (his feet, 0x13f3d0) and every decoy
/// must be inside the region; Ratchet outside gives kind 2 and no target. (The cuboid-list form is not used by the
/// ported classes.)
pub fn acquire_in(w: &World, id: MobyId, range: f32, region: Option<usize>) -> Target {
    let me = super::pos(w, id);
    let mut range = range;
    let (mut best, mut kind) = (w.hero_moby, 0u32);
    let inside = |p: V| region.is_none_or(|r| super::region::point_in_polygon(w, r, p));
    if w.hero.group == 0x18 || w.hero.state == 0x72 || !inside(w.hero.pos.map(|x| f32::from_bits(x.0))) {
        best = None;
        kind = 2;
    }
    let mut consider = |cand: MobyId, best: &mut Option<MobyId>, kind: &mut u32| {
        let d = super::dist2(me, w.m(cand).position);
        if d < range && inside(w.m(cand).position) {
            range = d;
            *best = Some(cand);
            *kind = 1;
        }
    };
    for (i, m) in w.table.mobys.iter().enumerate() {
        if m.state == crate::moby_runtime::state::END { break; }
        if DECOY_STATE3.contains(&m.o_class) && m.state == 3 { consider(i, &mut best, &mut kind); }
    }
    for (i, m) in w.table.mobys.iter().enumerate() {
        if m.state == crate::moby_runtime::state::END { break; }
        if m.o_class == DECOY_CMD && m.state < 0xfd && m.cmd != 0 { consider(i, &mut best, &mut kind); }
    }
    let Some(t) = best else { return Target { kind, ..Target::default() } };
    let m = w.m(t);
    let pos = m.position;
    let rot = m.rotation;
    if Some(t) == w.hero_moby {
        let f = |v: crate::hero::physics::V4| v.map(|x| f32::from_bits(x.0));
        return Target { pos, rot, aim: f(w.hero.shadow_point), body: f(w.hero.body_point), moby: Some(t), kind };
    }
    let mut body = pos;
    body[2] += 0.6;
    Target { pos, rot, aim: body, body, moby: Some(t), kind }
}
