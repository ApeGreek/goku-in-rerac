//! Teleporter pads, class 1135: `TeleporterPadUpdate` level01 0x308bd8 with the arm placement `FUN_003092d0`
//! and the hide `FUN_003097e8`. Spec: `docs/plan/moby_update_catalogue.md` "In the port: teleporter pads and
//! item offers". Native `f32`.
//!
//! Pvar block (P, s32 words): P[0] +0x00 the partner pad (moby link), P[1] +0x04 activation cuboid (−1: none),
//! P[2] +0x08 mission (−1: none), P[5..8] +0x14 the three arm mobys (class 315; port: `moby index + 1`, 0 = none),
//! P[8] +0x20 arm spread 0..1, P[9] +0x24 arm tilt 0..1, P[10]/P[11] their approach velocities, P[13] +0x34 the
//! beam fade, P[14] +0x38 starts active, **P[15] +0x3c challenge-mode only**, P[16]/P[17] music.
//!
//! Per tick:
//! 1. `bVar2` = the spawn id's collected flag `0x1bbb04[id]` or its persistent death bit (`0x14c190 + L·0x100`);
//! 2. in a state ≠ 0 with a cuboid P[1] and not `bVar2`: the hero in the cuboid activates the pad
//!    (`FUN_00308b28`, `FUN_00309838`, state 5 → 1) — **not ported** (counted; no Novalis pad has a cuboid);
//! 3. **the challenge gate**: P[15] ≠ 0 and the times-completed count `0x15ee20` = 0 → `+0x94 = 0` (no
//!    collision), mode |= 0x41 (hidden, no animation), return before the state switch: the pad stays in state 0
//!    and never creates its arms. P[15] ≠ 0 with the count set: collision from the class, mode &= ~0x41;
//! 4. the states:
//!    * **0**: update distance 0xff; three `CreateMoby(315)` arms with draw distance 0x40, `+0x31 = 1`, the
//!      hero moby's light word and ambient (+0x38 u64), the pad's mode, position and rotation, yaw + k·2π/3;
//!      state 6 when P[14] ≠ 0 or `bVar2`, else state 5 and `FUN_003097e8` (pad and arms hidden, pad collision
//!      off); then the arm placement `FUN_003092d0`;
//!    * **6**: a group command `FUN_0026e0e0(group, 1)` (not ported, counted; Novalis pads have no group), then
//!      state 1 and on into 1;
//!    * **1**: idle until the mission P[2] (if any) is done (`0x14c050` byte = 0xff); with a partner, the hero
//!      standing on the pad (0x13f64c, control mode 0x1413f4 ≠ 2, grounded) gets the prompt `FUN_00309430` and
//!      △ starts the teleport — **not ported** (counted when reached; the port's hero never records a ground
//!      moby);
//!    * **5**: waits for the cuboid (above); **2, 3, 4, 7, 8** (the teleport: arms, beam, `HeroTeleport` to the
//!      partner, music): **not ported** (counted).
//!
//! **Arm placement** `FUN_003092d0`, per arm: position = pad + (0, 0, −0.4) (gp−0x4c88), rotation y =
//! −20° (gp−0x4c80), then position += row 0 of that rotation scaled to `P[8]·2.4 − 1.2`, rotation y =
//! `P[9]·(−π/3) − 20°`, `MobyBuildMatrix`.

#![allow(clippy::neg_cmp_op_on_partial_ord)] // the angle wrap tests `!(s < π)` as the game's `c.lt.s` does.

use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::services::{pvar as p, World};

/// The pad update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x308bd8;

/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [1135];

/// The arm class (`CreateMoby(0x13b)`).
pub const ARM_CLASS: i16 = 315;

const PI: f32 = std::f32::consts::PI;
/// π/180 (0x3c8efa35).
const DEG: f32 = 0.017_453_292;

/// `fast_add_rotations` 0x221ff8.
fn add_rot(a: f32, b: f32) -> f32 {
    let s = a + b;
    let below = s < -PI;
    if !(s < PI) { return (s - PI) - PI; }
    if below { return (s + PI) + PI; }
    s
}

fn arm(pv: &[u8], k: usize) -> Option<MobyId> { let v = p::i32(pv, 0x14 + 4 * k); (v > 0).then(|| (v - 1) as usize) }

/// `TeleporterPadUpdate` (0x308bd8).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x48 { return; }
    let spawn_id = w.m(id).spawn_id;
    let level = w.svc.level;
    let done = w.svc.save.collected.get(&spawn_id).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(level, spawn_id));
    let pv = w.m(id).pvars.clone();
    let st = w.m(id).state;
    if st != 0 && p::i32(&pv, 4) != -1 && !done {
        w.svc.unported("teleporter: activation cuboid");
    }
    if p::i32(&pv, 0x3c) != 0 {
        if w.svc.counters.times_completed == 0 {
            let m = w.mm(id);
            m.has_collision = false;
            m.mode |= 0x41;
            return;
        }
        let coll = w.classes.info(w.m(id).o_class).is_some_and(|c| c.has_collision);
        let m = w.mm(id);
        m.has_collision = coll;
        m.mode &= !0x41;
    }
    match st {
        0 => {
            w.mm(id).update_dist = 0xff;
            let hero = w.hero_moby.map(|h| (w.m(h).light, w.m(h).ambient));
            for k in 0..3 {
                let Some(a) = w.create_moby(ARM_CLASS) else {
                    p::set_i32(&mut w.mm(id).pvars, 0x14 + 4 * k, 0);
                    continue;
                };
                let (pad_mode, pos, rot) = { let m = w.m(id); (m.mode, m.position, m.rotation) };
                let am = w.mm(a);
                am.draw_dist = 0x40;
                am.visible = 1;
                if let Some((light, ambient)) = hero {
                    am.light = light;
                    am.ambient = ambient;
                }
                am.mode = pad_mode;
                am.position = pos;
                am.rotation = rot;
                am.rotation[2] = add_rot(am.rotation[2], k as f32 * 2.094_395_2);
                p::set_i32(&mut w.mm(id).pvars, 0x14 + 4 * k, a as i32 + 1);
            }
            if p::i32(&pv, 0x38) != 0 || done {
                w.mm(id).state = 6;
            } else {
                w.mm(id).state = 5;
                hide(w, id);
            }
            place_arms(w, id);
        }
        6 | 1 => {
            if st == 6 {
                if w.m(id).group >= 0 { w.svc.unported("teleporter: group command"); }
                w.mm(id).state = 1;
            }
            let mission = p::i32(&pv, 8);
            if mission >= 0 && w.missions.mission_done(level, mission as u8) != 0xff { return; }
            if p::i32(&pv, 0) != -1 && w.hero.mode != 2 && w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0 {
                w.svc.unported("teleporter: on-pad prompt / teleport");
            }
        }
        5 => {}
        2 | 3 | 4 | 7 | 8 => w.svc.unported("teleporter: teleport states"),
        _ => {}
    }
}

/// `FUN_003097e8`: pad `+0x31 = 0`, mode |= 1, `+0x94 = 0`; each arm mode |= 1, `+0x31 = 0`.
fn hide(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let m = w.mm(id);
    m.visible = 0;
    m.mode |= mode::HIDDEN;
    m.has_collision = false;
    for k in 0..3 {
        if let Some(a) = arm(&pv, k).filter(|&a| a < w.table.mobys.len()) {
            let am = w.mm(a);
            am.mode |= mode::HIDDEN;
            am.visible = 0;
        }
    }
}

/// `FUN_003092d0`: the three arms around the pad (module doc).
fn place_arms(w: &mut World, id: MobyId) {
    let pv = w.m(id).pvars.clone();
    let (spread, tilt) = (p::ff(&pv, 0x20), p::ff(&pv, 0x24));
    let pad = w.m(id).position;
    for k in 0..3 {
        let Some(a) = arm(&pv, k).filter(|&a| a < w.table.mobys.len()) else { continue };
        let am = w.mm(a);
        am.position = pad;
        am.position[2] += f32::from_bits(0xbecc_cccd); // gp−0x4c88: −0.4
        am.rotation[1] = -20.0 * DEG; // gp−0x4c80: −20 (degrees)
        let d = spread * 2.4 - 1.2;
        let r0 = rc_formats::moby_light::rotation_rows([am.rotation[0], am.rotation[1], am.rotation[2]])[0].map(f32::from_bits);
        let l = (r0[0] * r0[0] + r0[1] * r0[1] + r0[2] * r0[2]).sqrt();
        if l != 0.0 {
            for (c, r) in am.position.iter_mut().zip(r0).take(3) { *c += r * (d / l); }
        }
        am.rotation[1] = tilt * -std::f32::consts::FRAC_PI_3 - 20.0 * DEG;
        w.build_matrix(a);
    }
}
