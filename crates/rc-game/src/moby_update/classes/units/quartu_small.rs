//! Quartu's small classes (level 15): **the bomb droppers 1394** (`0x2eabd8`, census U551, 2 placed) with **their
//! bombs 1257** (spawner `0x2e7ed8`, update `0x2e7f68`; created by code only, so the census never saw it) and **the
//! scene thrusters 1560** (`0x2edb20`, U560, one placed). Read from the level15 decomp and disassembly; native `f32`.
//!
//! * **1394**: updated always; every `scale(+0x04·60)` ticks at first, then every `scale(3.5·60)` (gp−0x4b84), a bomb
//!   1257 at its position.
//! * **1257**: falls (z velocity −20·dt² a tick); its sphere (0.5) touching something that is not a carrier, a hit
//!   (mask 0x830000) from another class, its line landing on Ratchet or landing faster than 9.75·dt: it blows up.
//!   Landing on a carrier (`0x275290`) it rides it (`0x2752c0`, the velocity the carried move) until the carrier
//!   is gone (it blows up) or the ground under it is no carrier's (it falls again). The blast: the beam explosion
//!   (flashes 2 / 1, light 7, 3 streaks, 3 sparks, 5 puffs, a debris piece), class 0x79's sound 0, hits within 2
//!   (damage 1, push 1 / 1, flags 0x830001, type 2 / 1), gone.
//! * **1560**: updated always; in a scene (game mode 2) 1..4 or 6, the thrusters (`0x252ef0` = L01 `0x278450`) on
//!   actor 4 (scene 1), 2 (scenes 2..4) or 1 (scene 6).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2eabd8` / `0x2e7ed8` | 1394, the bomb's spawn | [`dropper_update`], [`spawn_bomb`] |
//! | `0x2e7f68` | 1257 | [`bomb_update`] |
//! | `0x2edb20` | 1560 | [`thrusters_update`] |
//!
//! [L] The blast's position argument is 0 in the call; the bomb's position is used.

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, attack, fx};
use crate::moby_update::services::{pv, World};
use crate::moby_update::{story, triggers};
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 15;
pub const DROPPER_FN: u32 = 0x2e_abd8;
pub const DROPPER_CLASSES: [i16; 1] = [1394];
pub const BOMB_FN: u32 = 0x2e_7f68;
pub const BOMB_CLASSES: [i16; 1] = [1257];
pub const THRUSTERS_FN: u32 = 0x2e_db20;
pub const THRUSTERS_CLASSES: [i16; 1] = [1560];

/// gp−0x4b84: the drop interval (seconds).
const INTERVAL: f32 = 3.5;
/// gp−0x4c44: the bombs' gravity (· dt²).
const GRAVITY: f32 = 20.0;
const BLAST: fx::Beam = fx::Beam { damage_r: 0.0, damage: 0.0, flash: 2.0, flash2: 1.0, flash_dist: 4.0, scale: 1.0, light: 7.0, streaks: 3, sparks: 3, puffs: 5, debris: 1, sound: -1, shake: false };
const SOUND_CLASS: i16 = 0x79;

fn seconds(w: &World, s: f32) -> i32 { w.svc.timing.scale(Pf::f(s * 60.0)).to_i32() }

/// Level15 `0x2eabd8` (module doc).
pub fn dropper_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 8);
    let st = w.m(id).state;
    match st {
        0 => {
            let t = seconds(w, c::pf(w, id, 4));
            c::set_pi32(w, id, 0, t);
            let m = w.mm(id);
            m.update_dist = 0xff;
            m.state = 1;
        }
        1 if c::dec_timer_pvar_i32(w, id, 0) != 0 => {
            let t = seconds(w, INTERVAL);
            c::set_pi32(w, id, 0, t);
            let p = c::pos(w, id);
            spawn_bomb(w, p);
        }
        _ => {}
    }
}

/// Level15 `0x2e7ed8`: a bomb 1257 at `p` (draw distance 0x7f, always updated, drawn, state 0, its velocity and
/// carrier 0, collision on).
pub fn spawn_bomb(w: &mut World, p: c::V) -> Option<MobyId> {
    let id = w.create_moby(BOMB_CLASSES[0])?;
    story::pvars(w, id, 0x20);
    let coll = super::class_collision(w, BOMB_CLASSES[0]);
    let m = w.mm(id);
    m.draw_dist = 0x7f;
    m.update_dist = 0xff;
    m.visible = 1;
    m.state = 0;
    m.cmd = 0;
    m.pvars[..0x20].fill(0);
    m.position = p;
    m.has_collision = coll;
    w.build_matrix(id);
    Some(id)
}

fn carrier(w: &World, id: MobyId) -> Option<MobyId> {
    let v = c::pi32(w, id, 0x10);
    usize::try_from(v - 1).ok().filter(|&m| v > 0 && m < w.table.mobys.len())
}
fn is_carrier(w: &World, m: Option<MobyId>) -> bool { m.is_some_and(|m| triggers::carrier(w.m(m)).is_some()) }

/// Level15 `0x2e7f68` (module doc).
pub fn bomb_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, 0x20);
    let old = c::pos(w, id);
    let p = pv(old);
    if let Some(h) = w.coll_sphere(p, Pf::f(0.5), 1, Some(id)) {
        if !is_carrier(w, h.moby) { w.mm(id).cmd = 1; }
    }
    if let Some(a) = w.get_hit(id, 0x83_0000, false).and_then(|h| h.attacker) {
        if w.m(a).o_class != w.m(id).o_class { w.mm(id).cmd = 1; }
    }
    w.mm(id).hit_slot = 0xff;
    match w.m(id).state {
        0 => {
            let dt2 = crate::moby_update::creature::DT2;
            let vz = c::pf(w, id, 8) - GRAVITY * dt2;
            c::set_pf(w, id, 8, vz);
            let v = c::pv4(w, id, 0);
            let np = [old[0] + v[0], old[1] + v[1], old[2] + v[2], old[3]];
            w.mm(id).position = np;
            if let Some(h) = w.coll_line(p, pv(np), 0x10, Some(id)) {
                let at = [h.point[0], h.point[1], h.point[2], np[3]];
                if is_carrier(w, h.moby) {
                    c::set_pi32(w, id, 0x10, h.moby.map_or(0, |m| m as i32 + 1));
                    w.mm(id).state = 1;
                    c::set_pf(w, id, 8, 0.0);
                    w.mm(id).position = at;
                } else if h.moby.is_some() && h.moby == w.hero_moby || vz < crate::moby_update::creature::DT * -9.75 {
                    w.mm(id).cmd = 1;
                    w.mm(id).position = at;
                }
            }
        }
        1 => {
            let g = w.coll_line(pv([old[0], old[1], old[2] + 0.5, old[3]]), pv([old[0], old[1], 0.01, old[3]]), 2, None);
            let mut riding = false;
            if let Some(cm) = carrier(w, id) {
                if w.m(cm).state >= 0xfd {
                    w.mm(id).cmd = 1;
                    c::set_pi32(w, id, 0x10, 0);
                } else if g.as_ref().is_some_and(|g| g.moby.is_some()) && is_carrier(w, g.and_then(|g| g.moby)) {
                    riding = true;
                    if let Some(cv) = triggers::carrier(w.m(cm)) {
                        let rot = w.m(id).rotation;
                        let (q, r) = triggers::carried(&cv, [old[0], old[1], old[2]], [rot[0], rot[1], rot[2]]);
                        let m = w.mm(id);
                        m.position = [q[0], q[1], q[2], old[3]];
                        m.rotation = [r[0], r[1], r[2], rot[3]];
                        c::set_pv4(w, id, 0, [q[0] - old[0], q[1] - old[1], q[2] - old[2], 0.0]);
                    }
                }
            }
            if !riding {
                w.mm(id).state = 0;
                c::set_pf(w, id, 8, 0.0);
                c::set_pi32(w, id, 0x10, 0);
            }
        }
        _ => {}
    }
    if w.m(id).cmd != 0 { blow(w, id); }
}

fn blow(w: &mut World, id: MobyId) {
    let p = c::pos(w, id);
    fx::beam_explosion(w, &BLAST, Some(id), p);
    w.play_sound_as(0, 0, id, SOUND_CLASS);
    attack::area_hit(w, 2.0, p, id, 1.0, 1.0, 1.0, None, 0x83_0001, 2, 1);
    w.delete_moby(id);
}

/// Level15 `0x2edb20` (module doc).
pub fn thrusters_update(w: &mut World, id: MobyId) {
    match w.m(id).state {
        0 => {
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0xff;
        }
        1 => {
            if w.svc.game_mode != 2 { return; }
            let Some(scene) = w.svc.cinematic.scene.clone() else { return };
            let k = match scene.id {
                1 => 4,
                2..=4 => 2,
                6 => 1,
                _ => return,
            };
            if let Some(a) = scene.actors.get(k) { crate::moby_update::classes::cutscene_fx::infobot_thrusters(w, a); }
        }
        _ => {}
    }
}
