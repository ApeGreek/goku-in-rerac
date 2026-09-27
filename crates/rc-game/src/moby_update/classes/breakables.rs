//! Novalis's breakable props (docs/plan/creatures.md §1, "World props"): the breakable rocks 704 (`BreakableRockUpdate`
//! level01 0x2f9810), the walls the gunship's shells break 709 / 710 / 711 (`ProjectileBreakableUpdate` 0x2f9d80), the
//! big wall 729 (`BreakableWallUpdate` 0x2fa800), the pipe 778 (`BreakablePropUpdate` 0x2ff860) and the spray it leaves,
//! class 779 (0x2ffb28). Read from the disassembly; standard `f32`; the draws, sounds and order are the game's. The
//! level-table survey finds these functions on level 01 only, for exactly these classes.
//!
//! Every one bursts through two engine pieces that exist in the port: the burning rock bits 696–698
//! (`SpawnDebrisMoby` 0x2f8530 = `gunship::spawn_ember`, their update `0x2f8718`) and type-22 smoke puffs
//! (`PartType22Spawn` 0x281f30, `crate::particles::type22`: rising grey puffs; [`puff`] makes the record and its
//! draw, or only the draw without a particle system).
//!
//! * **704**: a hit with flags 0x10000 and damage > 0 (the wrench): class sound 0 (flags 0x10), 200 puffs (size
//!   `randf(0.5, 1)·210000`, life `ticks(rand_range(30, 90))`, colours 0x5f787878 / 0x181818) in a box 2 × 0.6 × 2
//!   along the rock's yaw, rising at `randf(0.5, 1.5)·dt`, 40 rock bits (`randf(0.04, 0.09)` scale, life 60–180)
//!   thrown at `randf(1, 5)·dt`; `SetDeathBits(m, 0, −1)` (its bolts) and deleted.
//! * **709–711**: deleted once its spawn id is collected or dead; a hit (mask 0x830000) whose attacker is a gunship
//!   shell (class 0x2ae = 686) bursts it into 20 rock bits (life 180–300) on a ring of radius 2–3, sets its death
//!   bits and deletes it; any other record is dropped.
//! * **729**: deleted once its spawn id is collected or dead; a hit with flags 0x800000: sound 0, 500 puffs (life
//!   120–240) and 100 bits over 8 × 2 × 8 along its yaw, the death bits, `0x13d396 = 1` (not kept: no reader in
//!   the port), deleted.
//! * **778**: a wrench hit (0x10000, damage > 0): `randi(P[1] − P[0] + 1)` (an empty delay loop), sound 0, 150 puffs
//!   from 1.2 above it, then the spray 779 in its place (update distance 0x20, draw distance 0xff, drawn, its light,
//!   pvar[0] = −1) and deleted.
//! * **779**: its loop sound 4 (kept alive), two puffs a tick rising out along its rows.
//! * **754** (pots, `BreakablePotUpdate` 0x2fd9a0) / **1813** (boxes, 0x30d0f0): state 0 → 1; a wrench hit (0x10000,
//!   damage > 0) → 2; state 2: sound 0, `BreakFxA` 0x2787a0 (`BoltBurst(m, 4, 7, Ratchet within 7 (xy) ? 2 : 0, −1)`),
//!   the pieces (`BreakFxB` 0x278ad8 = `fx::break_piece`: a pot four 1817 shards at 90° steps, 0.57·s out and
//!   1.2·s up (s = scale / class scale), turned 50° more; a box one 1815 in its place), `BreakFxC` 0x278e20 (the
//!   remains 1816 in its place, scaled by s, its light, update distance 0, draw distance 0xff, mode 0), deleted.
//!   (The pieces 1815 / 1817 are `FxGroupUpdate`, ported with the creatures; 1816's update is empty.)

use crate::moby_runtime::MobyId;
use crate::moby_update::classes::crate_::set_death_bits;
use crate::moby_update::classes::gunship::spawn_ember;
use crate::moby_update::creature::projectile;
use crate::moby_update::services::{pvar as p, World};
use crate::ps2v::Pf;

pub const ROCK_FN: u32 = 0x2f9810;
pub const ROCK_CLASSES: [i16; 1] = [704];
pub const SHELL_WALL_FN: u32 = 0x2f9d80;
pub const SHELL_WALL_CLASSES: [i16; 3] = [709, 710, 711];
pub const WALL_FN: u32 = 0x2fa800;
pub const WALL_CLASSES: [i16; 1] = [729];
pub const PIPE_FN: u32 = 0x2ff860;
pub const PIPE_CLASSES: [i16; 1] = [778];
pub const SPRAY_FN: u32 = 0x2ffb28;
pub const SPRAY_CLASSES: [i16; 1] = [779];
pub const POT_FN: u32 = 0x2fd9a0;
pub const POT_CLASSES: [i16; 1] = [754];
pub const BOX_FN: u32 = 0x30d0f0;
pub const BOX_CLASSES: [i16; 1] = [1813];
/// 1816's update 0x30d200 is `jr ra` (the remains the pots and boxes leave: an update function, so not mode 2).
pub const REMAINS_FN: u32 = 0x30d200;
pub const REMAINS_CLASSES: [i16; 1] = [1816];

/// `0x15ed6c`: dt.
const DT: f32 = 1.0 / 60.0;
/// 0x161c50 / 0x161c60 / 0x161c70: the rock-bit classes.
const BITS: [i16; 3] = [696, 697, 698];
/// The gunship shell.
const SHELL: i16 = 0x2ae;

type V = [f32; 4];

/// `FastVecNormalize(len, v, v)`: `v` scaled to length `len` (zero stays zero).
fn to_len(v: V, len: f32) -> V {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0.0 { return v; }
    let k = len / l;
    [v[0] * k, v[1] * k, v[2] * k, v[3]]
}

fn pos(w: &World, id: MobyId) -> V { w.m(id).position }

/// A point `(cos yaw·a, sin yaw·b, c) + m.pos` (the game's VU0 sine / cosine, here `std`).
fn along_yaw(w: &World, id: MobyId, a: f32, b: f32, c: f32) -> V {
    let (s, co) = w.m(id).rotation[2].sin_cos();
    let q = pos(w, id);
    [co * a + q[0], s * b + q[1], c + q[2], q[3]]
}

/// The spawn id collected (`0x1bbb04`) or dead this level (`0x14c190`).
fn spawn_id_done(w: &World, id: MobyId) -> bool {
    let s = w.m(id).spawn_id;
    w.svc.save.collected.get(&s).is_some_and(|&b| b != 0) || w.svc.save.death.contains(&(w.svc.level, s))
}

/// The death bits the walls set by hand (`0x14c190[level]` and `0x1ba950`), without `SetDeathBits`' bolts.
fn mark_dead(w: &mut World, id: MobyId) {
    let (s, lvl) = (w.m(id).spawn_id, w.svc.level);
    w.svc.save.death.insert((lvl, s));
    w.svc.save.death_level.insert(s);
}

/// `PartType22Spawn(size, pos, vel, c1, c2, life)`: the type-22 record (one `rand()` with it), or without a particle
/// system the draw alone.
fn puff(w: &mut World, size: f32, pos: V, vel: V, c1: u32, c2: u32, life: i32) {
    projectile::part22(w, &crate::particles::type22::Spawn { size, pos, vel, c1, c2, life });
}

/// A burst of `n` rock bits: per bit the throw direction `randf(−1, 1)`×3, the offset (`randf(±ox)`, `randf(±oy)`
/// along the yaw, `randf(z0, z1)` up), the speed `randf(1, 5)·dt`, the class `randi(3)`, the scale `randf(s0, s1)`,
/// the life `rand_range(60, 180)`, then `SpawnDebrisMoby(scale, 1, 1, 0.75, …, keep 0)`.
fn bits(w: &mut World, id: MobyId, n: usize, ox: f32, oy: f32, z: (f32, f32), scale: (f32, f32)) {
    for _ in 0..n {
        let d = [w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), 0.0];
        let a = w.rng.randf(-ox, ox);
        let b = w.rng.randf(-oy, oy);
        let c = w.rng.randf(z.0, z.1);
        let at = along_yaw(w, id, a, b, c);
        let v = to_len(d, w.rng.randf(1.0, 5.0) * DT);
        let class = BITS[w.rng.randi(3) as usize];
        let s = w.rng.randf(scale.0, scale.1);
        let life = w.rng.rand_range(0x3c, 0xb4);
        spawn_ember(w, s, 1.0, 1.0, 0.75, at, v, class, life, 0);
    }
}

/// A burst of `n` smoke puffs (704 / 729): direction `randf(−1, 1)`×3, offset along the yaw (`randf(±ox)`,
/// `randf(±oy)`, `randf(0, zt)`), speed `randf(s0, s1)·dt`, size `randf(z0, z1)·210000`, life `ticks(rand_range(l0,
/// l1))`, then the spawner.
#[allow(clippy::too_many_arguments)]
fn puffs(w: &mut World, id: MobyId, n: usize, ox: f32, oy: f32, zt: f32, speed: (f32, f32), size: (f32, f32), life: (i32, i32)) {
    for _ in 0..n {
        let d = [w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), w.rng.randf(-1.0, 1.0), 0.0];
        let a = w.rng.randf(-ox, ox);
        let b = w.rng.randf(-oy, oy);
        let c = w.rng.randf(0.0, zt);
        let at = along_yaw(w, id, a, b, c);
        let v = to_len(d, w.rng.randf(speed.0, speed.1) * DT);
        let size = w.rng.randf(size.0, size.1) * 210000.0;
        let l = w.rng.rand_range(life.0, life.1);
        let life = w.ticks(l);
        puff(w, size, at, v, 0x5f78_7878, 0x18_1818, life);
    }
}

/// `BreakableRockUpdate` (0x2f9810).
pub fn rock_update(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x1_0000, false);
    w.mm(id).hit_slot = 0xff;
    if !hit.is_some_and(|h| Pf::ZERO < h.damage) { return; }
    w.play_sound(0, 0x10, id);
    puffs(w, id, 200, 1.0, 0.3, 2.0, (0.5, 1.5), (0.5, 1.0), (0x1e, 0x5a));
    bits(w, id, 40, 1.2, 0.4, (0.5, 2.0), (0.04, 0.09));
    set_death_bits(w, id, 0, -1);
    w.delete_moby(id);
}

/// `ProjectileBreakableUpdate` (0x2f9d80), 709–711.
pub fn shell_wall_update(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x83_0000, false);
    if spawn_id_done(w, id) { w.delete_moby(id); }
    let by_shell = hit.and_then(|h| h.attacker).is_some_and(|a| w.table.mobys.get(a).is_some_and(|m| m.o_class == SHELL));
    if !by_shell {
        w.mm(id).hit_slot = 0xff;
        return;
    }
    for _ in 0..20 {
        let d = [w.rng.randf(-DT, DT), w.rng.randf(-DT, DT), w.rng.randf(-DT, DT), 0.0];
        let r = w.rng.randf(2.0, 3.0);
        let ang = w.rng.rand_angle();
        let q = pos(w, id);
        let at = [ang.cos() * r + q[0], ang.sin() * r + q[1], q[2], q[3]];
        let v = to_len(d, w.rng.randf(DT * 5.0, DT * 20.0));
        let class = BITS[w.rng.randi(3) as usize];
        let life = w.rng.rand_range(0xb4, 300);
        spawn_ember(w, 0.05, 1.0, 1.0, 0.75, at, v, class, life, 0);
    }
    mark_dead(w, id);
    w.delete_moby(id);
}

/// `BreakableWallUpdate` (0x2fa800), 729.
pub fn wall_update(w: &mut World, id: MobyId) {
    if spawn_id_done(w, id) {
        w.delete_moby(id);
        return;
    }
    if w.get_hit(id, 0x80_0000, false).is_none() { return; }
    w.play_sound(0, 0, id);
    puffs(w, id, 500, 4.0, 1.0, 8.0, (1.5, 3.5), (1.5, 3.5), (0x78, 0xf0));
    bits(w, id, 100, 4.0, 1.0, (1.0, 8.0), (0.05, 0.15));
    mark_dead(w, id);
    // (The game also sets the byte 0x13d396 = 1; nothing in the port reads it.)
    w.delete_moby(id);
}

/// `BreakablePropUpdate` (0x2ff860), 778.
pub fn pipe_update(w: &mut World, id: MobyId) {
    let hit = w.get_hit(id, 0x1_0000, false);
    w.mm(id).hit_slot = 0xff;
    if !hit.is_some_and(|h| Pf::ZERO < h.damage) { return; }
    let (lo, hi) = (p::i32(&w.m(id).pvars, 0), p::i32(&w.m(id).pvars, 4));
    w.rng.randi(hi - lo + 1); // the count of an empty delay loop
    w.play_sound(0, 0, id);
    for _ in 0..150 {
        let q = pos(w, id);
        let at = [q[0] - 0.2, q[1], q[2] + 1.2, q[3]];
        let a0 = w.rng.rand_angle();
        let s0 = w.rng.randf(1.5, 3.0);
        let vx = a0.cos() * s0 * DT;
        let a1 = w.rng.rand_angle();
        let s1 = w.rng.randf(1.5, 3.0);
        let vy = a1.sin() * s1 * DT;
        let vz = w.rng.randf(0.0, 3.0) * DT;
        let size = w.rng.randf(125000.0, 175000.0);
        let l = w.rng.rand_range(0x2d, 0x3c);
        let life = w.ticks(l);
        puff(w, size, at, [vx, vy, vz, 0.0], 0x207f_7f7f, 0x27_2727, life);
    }
    // FUN_002ffa90: the spray 779 in its place.
    if let Some(s) = w.create_moby(779) {
        let (pos, rot, light, amb) = { let m = w.m(id); (m.position, m.rotation, m.light, m.ambient) };
        let m = w.mm(s);
        m.update_dist = 0x20;
        m.draw_dist = 0xff;
        m.visible = 1;
        m.state = 0;
        m.light = light;
        m.ambient = amb;
        m.position = pos;
        m.rotation = rot;
        if m.pvars.len() < 4 { m.pvars.resize(0x80, 0); }
        p::set_i32(&mut m.pvars, 0, -1);
        w.build_matrix(s);
    }
    w.delete_moby(id);
}

/// 779's update (0x2ffb28): the loop sound 4 kept alive (`SoundIsAlive` ‖ `PlayClassSound(0, 4)`), two puffs a tick
/// (speeds `randf(0.8, 1.2)·3·dt` out and `randf(0.8, 1.2)·2·dt` up along the moby's rows, size `randf(125000,
/// 175000)`, life `ticks(rand_range(45, 60))`).
pub fn spray_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 4 { return; }
    let v = p::i32(&w.m(id).pvars, 0);
    if !w.sound_alive(v, id) {
        let s = w.play_sound(0, 4, id);
        p::set_i32(&mut w.mm(id).pvars, 0, s);
    }
    for _ in 0..2 {
        let out = w.rng.randf(0.8, 1.2) * DT * 3.0;
        let up = w.rng.randf(0.8, 1.2) * (DT + DT);
        // (out, 0, up) turned by the moby's rows; the nozzle (−0.2, 0, 1.2) from its position (not turned).
        let r = w.m(id).rows;
        let v: V = std::array::from_fn(|k| if k == 3 { 0.0 } else { r[0][k] * out + r[2][k] * up });
        let q = pos(w, id);
        let at = [q[0] - 0.2, q[1], q[2] + 1.2, q[3]];
        let size = w.rng.randf(125000.0, 175000.0);
        let l = w.rng.rand_range(0x2d, 0x3c);
        let life = w.ticks(l);
        puff(w, size, at, v, 0x207f_7f7f, 0x27_2727, life);
    }
}

/// `BreakFxC(m, class)` 0x278e20: a remains moby of `class` in `m`'s place.
fn remains(w: &mut World, id: MobyId, class: i16) {
    let (pos, rot, light, amb, scale, oc) = { let m = w.m(id); (m.position, m.rotation, m.light, m.ambient, m.scale, m.o_class) };
    let cs = w.class_scale(oc).to_f32();
    let Some(r) = w.create_moby(class) else { return };
    let m = w.mm(r);
    m.draw_dist = 0xff;
    m.visible = 1;
    m.scale *= scale / cs;
    m.position = pos;
    m.rotation = rot;
    m.update_dist = 0;
    m.light = light;
    m.ambient = amb;
    m.mode = 0;
    w.build_matrix(r);
}

/// `BreakFxA(m)` 0x2787a0: the bolts.
fn bolts(w: &mut World, id: MobyId) {
    let p = w.m(id).position;
    let h = crate::hero::physics::to_f32x3(w.hero.pos);
    let d = ((p[0] - h[0]).powi(2) + (p[1] - h[1]).powi(2)).sqrt();
    crate::moby_update::classes::crate_::bolt_burst(w, id, 4, 7, if d < 7.0 { 2 } else { 0 }, -1);
}

/// The two-step break of 754 / 1813 (state 0 → 1, a wrench hit → 2, the break in state 2); `pieces` spawns the
/// class's pieces.
fn break_on_hit(w: &mut World, id: MobyId, pieces: fn(&mut World, MobyId)) {
    let hit = w.get_hit(id, 0x1_0000, false);
    match w.m(id).state {
        0 => w.mm(id).state = 1,
        1 => {
            if hit.is_some_and(|h| Pf::ZERO < h.damage) { w.mm(id).state = 2; }
        }
        2 => {
            w.play_sound(0, 0, id);
            bolts(w, id);
            pieces(w, id);
            remains(w, id, 0x718);
            w.delete_moby(id);
        }
        _ => {}
    }
}

/// `BreakablePotUpdate` (0x2fd9a0), 754.
pub fn pot_update(w: &mut World, id: MobyId) {
    break_on_hit(w, id, |w, id| {
        let (p, oc, scale) = { let m = w.m(id); (m.position, m.o_class, m.scale) };
        let s = scale / w.class_scale(oc).to_f32();
        for i in 0..4 {
            let a = i as f32 * 1.570_796_4;
            let rot = [0.0, 0.0, add_rot(a, f32::from_bits(0x3f5f_66f3)), 0.0];
            let at = [a.cos() * -0.57 * s + p[0], a.sin() * -0.57 * s + p[1], s * 1.2 + p[2], p[3]];
            crate::moby_update::creature::fx::break_piece(w, id, 0x719, at, rot, 0, 0);
        }
    });
}

/// 1813's update (0x30d0f0).
pub fn box_update(w: &mut World, id: MobyId) {
    break_on_hit(w, id, |w, id| {
        let (p, r) = { let m = w.m(id); (m.position, m.rotation) };
        crate::moby_update::creature::fx::break_piece(w, id, 0x717, p, r, 0, 0);
    });
}

/// 1816's update (0x30d200): nothing.
pub fn remains_update(_w: &mut World, _id: MobyId) {}

/// `fast_add_rotations` 0x221ff8: `a + b` wrapped once into [−π, π).
fn add_rot(a: f32, b: f32) -> f32 {
    use std::f32::consts::PI;
    let s = a + b;
    if s >= PI || s.is_nan() { (s - PI) - PI } else if s < -PI { (s + PI) + PI } else { s }
}
