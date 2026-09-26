//! Ground walking for creatures (level01 `SeedJumpPattern` 0x26d930, the step `0x26d9a8`, the move `0x26d8b0` /
//! `0x26d610` and the ledge probe `0x26d1d0`).
//!
//! The walker record `J` (f32 words; the class's pvar, e.g. critter 577 at +0x180, amoeboid at +0x170):
//!
//! | word | seed | meaning |
//! |---|---|---|
//! | 0 | 0x280 (int) | collision radius ×1024 |
//! | 1 | 1.0 | step-up height (the sphere / line start above the feet) |
//! | 2 / 3 | 0.5 / 0.5 | ledge probe up / down |
//! | 4 | 0 | current speed (per tick) |
//! | 6 | 0 | vertical speed (gravity 9·dt² per tick while bit 0 of the flags is clear) |
//! | 7 | 0.3 | acceleration (× distance to the target when within 1) |
//! | 8 | 0.3 | braking fraction (within 10 ticks of the target) |
//! | 9 | 2.7 | top speed |
//! | 10 | 1.57 | — |
//! | 11 | 0.1 | "arrived" distance (result bit 4) |
//! | 12 | (pvar) | height of the origin above the ground (probe depth) |
//! | 14 | (pvar) | flags: 1 no gravity, 4 move along word 19 instead of the yaw, 8 walk off ledges, 0x10 / 0x20 the moby passes |
//! | 19 | — | heading used with flag 4 |
//!
//! **Step** (`0x26d9a8(k, moby, J, target, &out)`): speed += `min(acc·k·d, acc·k)` (d = xy distance to `target`),
//! braked by `J8·speed` when `d < 10·speed`, clamped to `±J9·k`; the move = (cos yaw, sin yaw)·speed plus the vertical
//! speed. The ledge probe (a line from `up` above to `down` below the stepped point, flags 0x22) decides: no ground →
//! result 0x22 (or 8 with flag 8), a slope steeper than 0.5 rad → one more probe 0.57 ahead (the far side may be
//! walkable). Blocked ledges (bit 1) do not move and reset the vertical speed. Otherwise the move (`0x26d610`): a line
//! guard for moves longer than the radius, up to six sphere push-outs (flags 4) at the step-up height, then a ground
//! line from the step-up height to `J12 + 0.05` below (flags 2): ground (a world face) → bit 2 and the vertical speed
//! reset to −0.01·[0x15ed60]; steep → bit 4; the origin eased up 30 % of the way when below `J12`. Walls → bit 0 (and
//! bit 1 too when less than 20 % of the move survived); a move that changed the height by more than the ledge limits is
//! undone (0x22). `out` receives the move actually made. Bit 4 is also set when within `J11` of the target.
//!
//! Note: the classes pass a *direction* (2·(cos yaw, sin yaw, 0)) as the target point (577, 572), so `d` is the
//! distance to a point near the world origin: in the game those creatures always accelerate to full speed and never
//! "arrive". The port passes the same values and gets the same result.

use super::{cs, dot3, len2, len3, scale, set_len3, sub, V};
use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pv, pvar, World};

/// `SeedJumpPattern(J)` 0x26d930.
pub fn seed(p: &mut [u8], j: usize) {
    pvar::set_i32(p, j, 0x280);
    pvar::set_u32(p, j + 0x2c, 0x3dcc_cccd);
    pvar::set_u32(p, j + 0x4, 0x3f80_0000);
    pvar::set_u32(p, j + 0xc, 0x3f00_0000);
    pvar::set_u32(p, j + 0x24, 0x402c_cccd);
    pvar::set_u32(p, j + 0x1c, 0x3e99_999a);
    pvar::set_u32(p, j + 0x28, 0x3fc8_f5c3);
    pvar::set_u32(p, j + 0x8, 0x3f00_0000);
    pvar::set_u32(p, j + 0x14, 0);
    pvar::set_u32(p, j + 0x10, 0);
    pvar::set_u32(p, j + 0x18, 0);
    pvar::set_u32(p, j + 0x20, 0x3e99_999a);
}

/// Result bits of [`step`].
pub mod res {
    pub const BLOCKED: u32 = 1;
    pub const STOPPED: u32 = 2;
    pub const ARRIVED: u32 = 4;
    pub const LEDGE_ALLOWED: u32 = 8;
    pub const NO_GROUND: u32 = 0x22;
}

fn w32(w: &World, id: MobyId, j: usize, k: usize) -> f32 { super::pf(w, id, j + 4 * k) }
fn set_w32(w: &mut World, id: MobyId, j: usize, k: usize, x: f32) { super::set_pf(w, id, j + 4 * k, x) }
fn flags(w: &World, id: MobyId, j: usize) -> u32 { pvar::u32(&w.m(id).pvars, j + 0x38) }

/// `0x26d1d0(up, down, p, moby, skip_mobys)`: `CollLine_Fix((p.xy, p.z + up), (p.xy, max(p.z − down, 0)), 0x22, moby)`;
/// a moby-primitive hit (negative kind) counts as none, and so does any moby hit with `skip_mobys`.
fn ledge_probe(w: &World, id: MobyId, up: f32, down: f32, p: V, skip_mobys: bool) -> Option<crate::collision_query::CollOutput> {
    let a = [p[0], p[1], p[2] + up, p[3]];
    let b = [p[0], p[1], (p[2] - down).max(0.0), p[3]];
    let o = w.coll_line(pv(a), pv(b), 0x22, Some(id))?;
    if o.kind < 0 { return None; }
    if skip_mobys && o.moby.is_some() { return None; }
    Some(o)
}

/// The slope of a hit normal from vertical: `FastArcTan(n.z, |n.xy|)`.
fn slope(o: &crate::collision_query::CollOutput) -> f32 {
    let n = [o.normal[0], o.normal[1], o.normal[2], 0.0];
    super::atan(n[2], len2(n))
}

/// `0x26d610(up, radius, down, moby, &mv, flags)`: the move with collision (module doc). Returns bit 0 walls, bit 1
/// ground under the feet, bit 2 steep ground; `mv` becomes the displacement made.
pub fn move_collide(w: &mut World, id: MobyId, up: f32, radius: f32, down: f32, mv: &mut V, flags: u32) -> u32 {
    let sflags = (flags & 0x10) << 1;
    let old = super::pos(w, id);
    let mut p = super::add(old, *mv);
    super::set_pos(w, id, p);
    let mut r = 0u32;
    if radius < len3(*mv) {
        let mut a = old;
        a[2] += up;
        let s = set_len3(*mv, radius);
        let mut b = super::add(s, p);
        b[2] += up;
        if let Some(o) = w.coll_line(pv(a), pv(b), sflags, Some(id)) {
            let h = [o.point[0], o.point[1], o.point[2], 0.0];
            p = sub(h, s);
            r = 1;
            p[2] -= up;
            super::set_pos(w, id, p);
        }
    }
    for _ in 0..6 {
        let mut c = p;
        c[2] += up;
        let Some(o) = w.coll_sphere(pv(c), crate::moby_update::services::pf(radius), sflags | 4, Some(id)) else { break };
        let Some(pc) = o.pushed_centre else { break };
        p = [pc[0], pc[1], pc[2], p[3]];
        r |= 1;
        p[2] -= up;
        super::set_pos(w, id, p);
    }
    let zlo = p[2] - down;
    let mut a = p;
    a[2] += up;
    let mut b = p;
    b[2] = zlo - 0.05;
    let mut out = r;
    if let Some(o) = w.coll_line(pv(a), pv(b), sflags | 2, Some(id)) {
        if o.kind > 0 {
            out = r | 2;
            if 0.5 < slope(&o) { out |= 4; }
            if zlo < o.point[2] {
                p[2] += (o.point[2] - zlo) * 0.3;
                super::set_pos(w, id, p);
            }
        }
    }
    *mv = sub(p, old);
    out
}

/// `0x26d9a8(k, moby, J, target, &out)`: one walking step (module doc). Returns the result bits ([`res`]); `out` gets
/// the move made (unchanged when the step was refused).
pub fn step(w: &mut World, id: MobyId, j: usize, k: f32, target: V, out: &mut V) -> u32 {
    let mut r = 0u32;
    let acc = w32(w, id, j, 7) * k;
    let vmax = w32(w, id, j, 9) * k;
    let fl = flags(w, id, j);
    let p0 = super::pos(w, id);
    let d = len2(sub(target, p0));
    let mut a = acc * d;
    if acc < a { a = acc; }
    let mut speed = w32(w, id, j, 4) + a;
    if d < speed * 10.0 { speed -= w32(w, id, j, 8) * speed; }
    if vmax < speed { speed = vmax; } else if speed < -vmax { speed = -vmax; }
    set_w32(w, id, j, 4, speed);
    let heading = if fl & 4 != 0 { w32(w, id, j, 19) } else { super::yaw(w, id) };
    let (c, s) = cs(heading);
    let mut mv: V = [c * speed, s * speed, 0.0, 0.0];
    if fl & 1 == 0 {
        mv[2] = w32(w, id, j, 6);
        set_w32(w, id, j, 6, mv[2] - super::DT2 * 9.0);
    }
    let zc = p0[2] + 0.04;
    let depth = w32(w, id, j, 12);
    let (up, down) = (w32(w, id, j, 2), w32(w, id, j, 3));
    let mut q = super::add(p0, mv);
    q[2] = p0[2] - depth;
    let skip = fl & 0x20 != 0;
    let mut stop;
    match ledge_probe(w, id, up, down, q, skip) {
        None => {
            r = if fl & 8 != 0 { 8 } else { res::NO_GROUND };
            stop = r & 2 != 0;
        }
        Some(o) => {
            let gz = o.point[2];
            stop = false;
            if 0.5 < slope(&o) && 0.1 < len2(mv) {
                let mut q2 = super::add(set_len3(mv, 0.57), p0);
                q2[2] = p0[2] - depth;
                let far = ledge_probe(w, id, up, down, q2, skip);
                // LAB_0026dca0: 0x22, stopping only when the stepped ground is at or above the feet.
                let ledge_rule = |r: &mut u32| -> bool { *r = 0x22; if zc <= gz { true } else { *r = 8; false } };
                match far {
                    None => {
                        r = 0x22;
                        stop = if fl & 8 != 0 { ledge_rule(&mut r) } else { true };
                    }
                    Some(o2) => {
                        if 0.5 <= slope(&o2) {
                            r = 0x22;
                            stop = if fl & 8 != 0 { if zc <= o2.point[2] { true } else { ledge_rule(&mut r) } } else { true };
                        }
                    }
                }
            }
        }
    }
    if !stop {
        let l = len2(mv);
        let dirn = if l == 0.0 { [0.0; 4] } else { scale(mv, 1.0 / l) };
        let before = p0;
        let up1 = w32(w, id, j, 1);
        let radius = super::pi32(w, id, j) as f32 * (1.0 / 1024.0);
        let m = move_collide(w, id, up1, radius, depth, &mut mv, fl);
        if m & 2 != 0 { set_w32(w, id, j, 6, super::SPEED * -0.01); }
        if m & 1 != 0 {
            r |= 1;
            if mv[0] * dirn[0] + mv[1] * dirn[1] < l * 0.2 { r |= 3; }
            let g = super::ground::ground(w, super::pos(w, id), 0.5, 0).z;
            if up < before[2] - g || down < g - before[2] {
                r |= 0x22;
                super::set_pos(w, id, before);
            }
        }
        *out = mv;
    } else if fl & 1 == 0 {
        set_w32(w, id, j, 6, super::SPEED * -0.01);
    }
    let _ = dot3;
    if d < w32(w, id, j, 11) { r |= 4; }
    r
}
