//! Checkpoint triggers, class 805: `CheckpointTriggerUpdate` level01 0x300220 and the checkpoint record
//! `FUN_0029ac10` (the respawn point the death reload `0x29adc8` puts the hero on). Spec: docs/plan/triggers.md
//! §4 ("rising edge, re-armed on exit"), docs/plan/hero_states.md P2. Native `f32`.
//!
//! Pvar block (P, s32 words): P[0] the cuboid (`PointInCuboid`), P[2] flags (8 off; 2 Clank only; 4 Ratchet or
//! the disguise only; 0x10 only in the groups 0 / 1 / 9), P[3] taken, P[4] may be taken again, P[5] inside last
//! tick, P[6] the mission that must be done first (−1: none). `moby+0xbc`: initialised.
//!
//! Per tick: the first update only clears P[5]. Then, gated by the flags and P[6], the hero's feet 0x13f3d0 are
//! tested against the cuboid; entering it (or the checkpoint's spawn id already collected / dead, e.g. reached
//! another way) takes it — unless taken and not repeatable: P[3] = 1, `SetMissionDone(+0xb0)`, the record, and
//! its own spawn id's killed / collected bytes and death bits cleared. Otherwise P[3] = 0. Last, state 0 → 1 once
//! its mission is done.
//!
//! **The record `0x29ac10(point, euler)`**: the point is the checkpoint's position with z on the ground below
//! it (`GroundHeight(0.5)`) unless that is more than 2 away; the Euler angles are the checkpoint's (+0x40).
//! Every spawn id this visit killed (`0x1baea4[id] = mission + 2`) whose mission is done (or 1: no mission)
//! becomes collected (`0x1bbb04`) with both death bits set; the record goes to [`SaveBits::checkpoint`]
//! (0x1bb6b0.., [`Record`]).
//!
//! **Not ported**: `SetMissionDone` (the port has no mission writer yet: counted), the record's copy of the
//! visit state 0x1baaa0 → 0x1bb700 and the hero's light word / ambient / control mode / music (the death reload
//! restores them; the engine's respawn only places the hero).
//!
//! [`SaveBits::checkpoint`]: crate::moby_update::services::SaveBits::checkpoint

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};
use crate::moby_update::triggers::point_in_cuboid;
use crate::ps2v::Pf;

/// The checkpoint update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x300220;

/// Classes that run [`update`].
pub const CLASSES: [i16; 1] = [805];

/// The checkpoint record (0x1bb6b0 = 1, position 0x1bb6c0, Euler 0x1bb6d0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub pos: [f32; 3],
    pub rot: [f32; 3],
}

pub fn update(w: &mut World, id: MobyId) {
    let flags = p::u32(&w.m(id).pvars, 8);
    if flags & 8 != 0 { return; }
    if w.m(id).cmd == 0 {
        let m = w.mm(id);
        m.cmd = 1;
        p::set_i32(&mut m.pvars, 0x14, 0);
        return;
    }
    let level = w.svc.level;
    let gate = p::i32(&w.m(id).pvars, 0x18);
    if gate != -1 && w.missions.mission_done(level, gate as u8) != 0xff { return; }
    let (mode, group) = (w.hero.mode, w.hero.group);
    if flags & 2 != 0 && mode != 1 { return; }
    if flags & 4 != 0 && mode != 3 && mode != 0 { return; }
    if flags & 0x10 != 0 && 1 < group as u32 && group != 9 { return; }
    let feet = crate::hero::physics::to_f32x3(w.hero.pos);
    let inside = point_in_cuboid(&w.svc.volumes, feet, p::i32(&w.m(id).pvars, 0));
    let mut take = false;
    {
        let pv = &mut w.mm(id).pvars;
        if inside {
            take = p::i32(pv, 0x14) == 0;
            p::set_i32(pv, 0x14, 1);
        } else {
            p::set_i32(pv, 0x14, 0);
        }
    }
    let sid = w.m(id).spawn_id;
    if !take {
        let collected = w.svc.save.collected.get(&sid).is_some_and(|&b| b != 0);
        if collected || w.svc.save.death.contains(&(level, sid)) {
            take = true;
        } else {
            p::set_i32(&mut w.mm(id).pvars, 0xc, 0);
        }
    }
    if take {
        let pv = &w.m(id).pvars;
        if !(p::i32(pv, 0xc) != 0 && p::i32(pv, 0x10) == 0) {
            p::set_i32(&mut w.mm(id).pvars, 0xc, 1);
            // SetMissionDone(+0xb0): no mission writer in the port.
            w.svc.unported("checkpoint 805 SetMissionDone");
            let m = w.m(id);
            let (pos, rot) = ([m.position[0], m.position[1], m.position[2]], [m.rotation[0], m.rotation[1], m.rotation[2]]);
            let g = w.ground_height(Pf::b(0x3f00_0000), [Pf::f(pos[0]), Pf::f(pos[1]), Pf::f(pos[2]), Pf::f(m.position[3])], 0).to_f32();
            let z = if 2.0 < (pos[2] - g).abs() { pos[2] } else { g };
            record(w, Record { pos: [pos[0], pos[1], z], rot });
            let s = &mut w.svc.save;
            s.killed.remove(&sid);
            s.collected.remove(&sid);
            s.death.remove(&(level, sid));
            s.death_level.remove(&sid);
        }
    }
    if w.m(id).state == 0 && w.missions.mission_done(level, w.m(id).mission) == 0xff { w.mm(id).state = 1; }
}

/// `FUN_0029ac10(point, euler)`: this visit's kills whose mission is done become permanent, and the record.
fn record(w: &mut World, r: Record) {
    let level = w.svc.level;
    let kills: Vec<(i16, u8)> = w.svc.save.killed.iter().map(|(&k, &v)| (k, v)).collect();
    for (sid, b) in kills {
        if !(0..0x7ff).contains(&sid) || b == 0 { continue; }
        if b == 1 || w.missions.mission_done(level, b.wrapping_sub(2)) == 0xff {
            let s = &mut w.svc.save;
            s.collected.insert(sid, b);
            s.death_level.insert(sid);
            s.death.insert((level, sid));
        }
    }
    w.svc.save.checkpoint = Some(r);
}
