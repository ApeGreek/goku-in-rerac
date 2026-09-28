//! Death and break effects shared by the creatures (level01 addresses; all engine code linked into every level):
//!
//! * [`death_explosion`] `0x273f50(size, light, moby, pos, sound)`: three spark pairs (type 11), two flashes, a camera
//!   shake when in view, the moby's death sound, an explosion light ([`light_spawn`], class 639).
//! * [`beam_explosion`] `SpawnBeamExplosion` 0x273310: the general explosion (damage sphere, type-15 streaks,
//!   type-11 sparks by camera distance, type-8 puffs, flashes, shake, sound, light).
//! * [`break_piece`] `BreakFxB` 0x278ad8 and [`piece_update`] `FxGroupUpdate` 0x30cd18: a body piece thrown off with a
//!   random velocity and spin, bouncing on the world, exploding (or fading, flag 1) when its timer runs out.
//! * [`light_spawn`] 0x2f3570 and [`light_update`] 0x2f3748: the explosion light moby (class 0x27f = 639).
//! * [`rate_slot`] `0x2efbf8`: a small "busy until tick" table that rate-limits the death explosions.
//!
//! The particle spawners the effects call ([`part02`], [`part04`], [`part08`], [`part15`], [`part52`]) fill the game's
//! records (`crate::particles::type02` …) and make the spawners' own draws at the game's point, and are counted in
//! `FxStats::part_spawns`. Without a particle system (unit tests) the draws are made as if a record was free. The
//! explosion light takes one of the eight point-light slots (`WritePointLight_B` 0x252750, [`crate::point_lights`]).

use super::{add, cs, len3, scale, set_len3, sub, V};
use crate::moby_runtime::{mode, MobyId};
use crate::moby_update::classes::debris::flash_spawn;
use crate::moby_update::services::{pf as to_pf, pv, pvar, HitTemplate, World};
use crate::ps2v::Pf;

/// `0x20a090` / `0x20a0a8`: the spark colours the explosions pick from (`randi(6)` each).
pub const SPARK_A: [u32; 6] = [0x4f00_8fff, 0x4f00_8fff, 0x4f00_7fff, 0x4f00_6fff, 0x2fff_ffff, 0x2fff_ffff];
pub const SPARK_B: [u32; 6] = [0x2f00_5f7f, 0x2f00_4f7f, 0x2f00_3f7f, 0x2f00_004f, 0x2f00_0000, 0x3f00_0000];

/// The explosion light class (0x27f) and its update's level01 address.
pub const LIGHT_CLASS: i16 = 0x27f;
pub const LIGHT_UPDATE_FN: u32 = 0x2f3748;
/// `FxGroupUpdate` 0x30cd18 and the level01 classes it runs (the body pieces of 577 are 1747–1749).
pub const PIECE_UPDATE_FN: u32 = 0x30cd18;
pub const PIECE_CLASSES: [i16; 13] = [1736, 1737, 1738, 1747, 1748, 1749, 1761, 1762, 1763, 1770, 1814, 1815, 1817];

pub(crate) fn frame_load(w: &World) -> (f32, f32) { (f32::from_bits(w.svc.frame_load[0].0), f32::from_bits(w.svc.frame_load[1].0)) }

/// A record of an unported particle type (the pool slot the game takes) and its count.
pub fn part_unported(w: &mut World, ty: u8) -> bool {
    *w.svc.fx.part_spawns.entry(ty).or_default() += 1;
    let Some(p) = w.particles.as_deref_mut() else { return true };
    let ok = p.create_part(ty).is_some();
    if !ok { w.svc.fx.part_failed += 1; }
    ok
}

/// A spawner call of type `ty`: `f` fills the record (and makes the spawner's draws) when there is a particle
/// system; `draws` makes the same draws when there is none. Counted in `FxStats`; false when the pool was full.
fn spawn_part(w: &mut World, ty: u8, draws: impl FnOnce(&mut crate::rng::Rng), f: impl FnOnce(&mut crate::particles::Particles, &mut crate::rng::Rng) -> Option<usize>) -> bool {
    *w.svc.fx.part_spawns.entry(ty).or_default() += 1;
    let Some(p) = w.particles.as_deref_mut() else {
        draws(w.rng);
        return true;
    };
    let ok = f(p, w.rng).is_some();
    if !ok { w.svc.fx.part_failed += 1; }
    ok
}

/// `PartType02Spawn` 0x27dc98 (the trail blob; `crate::particles::type02`): one `randf(0, 255)` with a record.
pub fn part02(w: &mut World, a: &crate::particles::type02::Spawn) -> bool {
    spawn_part(w, 2, |r| { r.randf(0.0, 255.0); }, |p, r| crate::particles::type02::spawn(p, r, a))
}

/// `PartType04Spawn` 0x27e538 (the smoke / fire puff; `crate::particles::type04`): one raw `rand()` with a record.
pub fn part04(w: &mut World, a: &crate::particles::type04::Spawn) -> bool {
    spawn_part(w, 4, |r| { r.rand(); }, |p, r| crate::particles::type04::spawn(p, r, a))
}

/// `PartType08Spawn(size, pos, vel, c1, c2, life)` 0x27f2b0 (the explosion puff): nothing for life 0; no draws.
pub fn part08(w: &mut World, size: f32, p: V, vel: V, c1: u32, c2: u32, life: i32) -> bool {
    if life == 0 { return false; }
    spawn_part(w, 8, |_| {}, |s, _| crate::particles::type08::spawn(s, size, p, vel, c1, c2, life))
}

/// `PartType15Spawn` 0x280bd0 (the streak): nothing for life 0; one raw `rand()` with a record.
pub fn part15(w: &mut World, a: &crate::particles::type15::Spawn) -> bool {
    if a.life == 0 { return false; }
    spawn_part(w, 15, |r| { r.rand(); }, |p, r| crate::particles::type15::spawn(p, r, a))
}

/// `PartType52Spawn(s1, s2, pos, c1, c2, life)` 0x287158 (the flat goo drip): one `randi(255)` with a record.
pub fn part52(w: &mut World, s1: f32, s2: f32, p: V, c1: u32, c2: u32, life: i32) -> bool {
    spawn_part(w, 52, |r| { r.randi(0xff); }, |s, r| crate::particles::type52::spawn(s, r, s1, s2, p, c1, c2, life))
}

/// `0x277b50(speed, a, b, out)`: `speed·(cos a·cos b, sin a·cos b, sin b)`.
pub fn polar(speed: f32, a: f32, b: f32) -> V { [a.cos() * speed * b.cos(), a.sin() * speed * b.cos(), b.sin() * speed, 0.0] }

/// `0x2efbf8(table, n, dur)`: the first slot (from the last) whose busy-until tick is before now gets `now + dur`;
/// returns its index, −1 when all are busy.
pub fn rate_slot(slots: &mut [i32], now: i32, dur: i32) -> i32 {
    for i in (0..slots.len()).rev() {
        if slots[i] < now {
            slots[i] = now + dur;
            return i as i32;
        }
    }
    -1
}

/// Is the sphere `(pos, r)` in view (`FastBSphereCheck(far, …) ≠ −1`)? No view: in view.
pub fn in_view(w: &World, far: f32, p: V, r: f32) -> bool {
    match w.view {
        Some(v) => !v.culled(far, [p[0], p[1], p[2], r]),
        None => true,
    }
}

/// `0x273f50(size, light, moby, pos, sound)`: the death explosion (module doc). Draws: per spark pair
/// `randf(8, 10)`, `randi(6)` ×2, two `rand_range`s, then the type-11 spawn's own; 3 per flash; the light's.
pub fn death_explosion(w: &mut World, size: f32, light: f32, moby: Option<MobyId>, p: V, sound: i32) {
    sparks_and_flashes(w, size, 400000.0, moby, p);
    if in_view(w, 10.0, p, 2.0) {
        let t = w.ticks(20);
        w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp: size * 0.1, ticks: t });
    }
    if let Some(m) = moby {
        if !w.m(m).is_deleted() && sound != -1 { w.play_sound(sound, 0, m); }
    }
    explosion_light(w, light, p);
}

/// `0x2742a8(size, light, moby, pos, −1)`: the quiet explosion of a broken prop's flag-2 pieces
/// (`crate::moby_update::classes::breakables`): the death explosion's three spark pairs (sprites 500000·size instead
/// of 400000·size) and two flashes, no camera shake and no sound; the light when `light ≠ 0`. The gold-glove colour
/// shifts it passes the colours through (`0x270fa8`, `0x270f48`) are the identity here, as in `bomb.rs`.
pub fn piece_explosion(w: &mut World, size: f32, light: f32, moby: Option<MobyId>, p: V) {
    sparks_and_flashes(w, size, 500000.0, moby, p);
    explosion_light(w, light, p);
}

/// The spark pairs and flashes 0x273f50 and 0x2742a8 share (`sprite` = the type-11 size per unit of `size`).
fn sparks_and_flashes(w: &mut World, size: f32, sprite: f32, moby: Option<MobyId>, p: V) {
    for _ in 0..3 {
        let sp = w.rng.randf(8.0, 10.0) * super::DT;
        let a = w.rng.randi(6) as usize;
        let b = w.rng.randi(6) as usize;
        let (t15, t20) = (w.ticks(15), w.ticks(20));
        let life = w.rng.rand_range(t15, t20);
        let (t25, t30) = (w.ticks(25), w.ticks(30));
        let t1 = w.rng.rand_range(t25, t30);
        w.part11(to_pf(size * sprite), to_pf(sp * size), pv(p), [Pf::ZERO; 4], SPARK_A[a], SPARK_B[b], life, t1, 0, 0);
    }
    if let Some(m) = moby {
        let t = w.ticks(20);
        flash_spawn(w, size * 4.0, m, p, [0.0; 4], t, 0x7f, 0x40, 0, 0x30);
        let t = w.ticks(0x1d);
        flash_spawn(w, size * 3.0, m, p, [0.0; 4], t, 0x60, 0x20, 0, 0x20);
    }
}

/// The explosion light of 0x273f50 / 0x2742a8: none for 0, radius 13 for a negative size.
fn explosion_light(w: &mut World, light: f32, p: V) {
    if light != 0.0 {
        let l = if light <= 0.0 { 13.0 } else { light };
        let mut t = LIGHT_DEATH;
        t.radius = [l, l, l];
        light_spawn(w, &t, p);
    }
}

/// The parameters of `SpawnBeamExplosion` 0x273310 (in the game's order).
#[derive(Clone, Copy, Debug)]
pub struct Beam {
    /// `param_1` / `param_2`: damage sphere radius and damage (flags 0x810001, type 2 / subtype 1, the moby's class).
    pub damage_r: f32,
    pub damage: f32,
    /// `param_3` / `param_4`: the two flash sizes; `param_5`: the camera distance beyond which the extra flashes show.
    pub flash: f32,
    pub flash2: f32,
    pub flash_dist: f32,
    /// `param_6`: the effect scale; `param_7`: the light radius (0 none).
    pub scale: f32,
    pub light: f32,
    /// `param_11` type-15 streaks, `param_12` type-11 spark pairs, `param_13` type-8 puffs, `param_16` debris.
    pub streaks: i32,
    pub sparks: i32,
    pub puffs: i32,
    pub debris: i32,
    /// `param_14` the moby's class sound (−1 none), `param_15` camera shake.
    pub sound: i32,
    pub shake: bool,
}

/// `SpawnBeamExplosion(…, moby, pos, …)` 0x273310 with `param_17` = −1 (throttled when the frame loads sum over 1.7)
/// and `param_18` = 0 (the normal colours). The debris burst (`param_16`, `0x2c4c20`) is not ported: counted, its
/// draws are not made (no caller in the ported classes uses it).
pub fn beam_explosion(w: &mut World, b: &Beam, moby: Option<MobyId>, p: V) {
    let (l0, l1) = frame_load(w);
    let throttle = (1.7 < l1 + l0) as i32;
    let base_z = super::DT * 8.0 * b.scale;
    if 0.0 < b.damage_r {
        if let Some(m) = moby {
            let t = HitTemplate { dir: [Pf::ZERO; 4], attacker: Some(m), flags: 0x81_0001, b18: 2, b19: 1, h1a: w.m(m).o_class as u16, damage: to_pf(b.damage), w20: 0 };
            w.sphere_mobys(to_pf(b.damage_r), pv(p), 0x10, Some(m), Some(&t));
        }
    }
    for _ in 0..b.streaks.max(0) {
        let pitch = w.rng.randf(0.2618, 1.3963);
        let ang = w.rng.rand_angle();
        let sp = w.rng.randf(7.0, 10.5);
        let mut v = polar(b.scale * sp * super::DT, ang, pitch);
        v[2] += super::DT * 3.0;
        let (t60, t120) = (w.ticks(60), w.ticks(120));
        let life = w.rng.rand_range(t60, t120) - throttle * 10;
        let a = crate::particles::type15::Spawn { size: b.scale * 40000.0, pos: p, vel: v, c1: 0x4f00_7fff, c2: 0x1f00_007f, life, split: 1, def: -1, blend: -1 };
        part15(w, &a);
    }
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    let dist = len3(sub(cam, p));
    if b.debris != 0 {
        // The random direction (3 draws) and the debris of 0x2c4c20 (not ported).
        w.svc.unported("creature fx: beam explosion debris 0x2c4c20");
    }
    let mut n = b.sparks;
    if dist < (b.sparks as f32) * 2.0 { n = (dist as i32) / 2; }
    let near = if dist < 7.0 { 7.0 - dist } else { 0.0 };
    for _ in 0..n.max(0) {
        let sp = w.rng.randf(8.0, 10.0) * super::DT;
        let a = w.rng.randi(6) as usize;
        let c = w.rng.randi(6) as usize;
        let speed = (sp - near * super::DT) * b.scale;
        let (t15, t20) = (w.ticks(15), w.ticks(20));
        let life = w.rng.rand_range(t15, t20);
        let (t30, t45) = (w.ticks(30), w.ticks(45));
        let t1 = w.rng.rand_range(t30, t45);
        let size = to_pf(b.scale * 400000.0);
        let basev = [Pf::ZERO, Pf::ZERO, to_pf(base_z), Pf::ZERO];
        w.part11(size, to_pf(speed), pv(p), basev, SPARK_A[a], SPARK_B[c], life - throttle * 3, t1 - throttle * 5, 0, 0);
        let (t5, t10) = (w.ticks(5), w.ticks(10));
        let life2 = w.rng.rand_range(t5, t10);
        let (t15, t20) = (w.ticks(15), w.ticks(20));
        let t2 = w.rng.rand_range(t15, t20);
        w.part11(size, to_pf(speed * 0.5), pv(p), basev, 0x7fff_ffff, 0x00ff_ffff, life2 - throttle * 2, t2 - throttle * 3, 0, 0);
    }
    for _ in 0..b.puffs.max(0) {
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let z = w.rng.randf(-1.0, 1.0);
        let s = w.rng.randf(0.0, 3.0);
        let vel = set_len3([x, y, z, 0.0], b.scale * s * super::DT);
        let a = w.rng.randi(6) as usize;
        let c = w.rng.randi(6) as usize;
        let (t30, t45) = (w.ticks(30), w.ticks(45));
        let life = w.rng.rand_range(t30, t45) - throttle * 5;
        part08(w, 200_000.0, p, vel, SPARK_A[a], SPARK_B[c], life);
    }
    if let Some(m) = moby {
        if 0.0 < b.flash {
            if l0 < 0.95 && b.flash_dist < dist {
                let t = w.ticks(16);
                flash_spawn(w, b.flash, m, p, [0.0; 4], t, 0x96, 0x96, 0x96, 0x20);
                let t = w.ticks(22);
                flash_spawn(w, b.flash, m, p, [0.0; 4], t, 0x7f, 0x7f, 0x50, 0x20);
            }
            let t = w.ticks(30);
            flash_spawn(w, b.flash, m, p, [0.0; 4], t, 0x7f, 0x7f, 0, 0x30);
        }
        if 0.0 < b.flash2 {
            let t = w.ticks(27);
            flash_spawn(w, b.flash2, m, p, [0.0; 4], t, 0xff, 0xff, 0xff, 0x20);
        }
    }
    if b.shake && in_view(w, 10.0, p, 2.0) {
        let amp = if dist < 20.0 { 0.4 - dist * 0.0175 } else { 0.050_000_012 };
        let t = w.ticks(25);
        w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: t });
    }
    if let (Some(m), true) = (moby, b.sound != -1) { w.play_sound(b.sound, 0, m); }
    if b.light != 0.0 && throttle == 0 && in_view(w, 100.0, p, b.light) {
        // The template's three radii are overwritten (the game writes them into 0x1b06d0 itself); param_18 = 0.
        let l = if b.light <= 0.0 { 15.0 } else { b.light };
        let mut t = LIGHT_BEAM;
        t.radius = [l, l, l];
        light_spawn(w, &t, p);
    }
}

// -------------------------------------------------------------------------------------------------
// The amoeboids' goo (0x2ef560 drips, 0x2ef770 bursts; gp block 0x1619b0..0x161a3c)

/// `0x2747a0(r, v)`: `v.xyz += (randf(−r, r), randf(−r, r), randf(−r, r))`, in that order.
fn jitter(w: &mut World, r: f32, v: &mut V) {
    for c in v.iter_mut().take(3) { *c += w.rng.randf(-r, r); }
}

/// `0x2ef560`: while the amoeboid was drawn (+0x31), two chances of a flat goo drip (type 52) at a random point within
/// 0.1 of its feet: 1 in 20 (size `randf(0, 1)`·s → `randf(1, 2)`·s, colours 0x30002028 → 0x2020) and 1 in 5 (size
/// `randf(0, 3.5)`·s → `randf(3.5, 4.5)`·s, colours 0x20002020 → 0x2030); life `randf(120, 180)` ticks; s = the
/// amoeboid's size (pvar +0x250). Per drip 6 draws plus the spawner's one.
pub fn goo_drips(w: &mut World, id: MobyId, size: f32) {
    if w.m(id).visible == 0 { return; }
    for (n, lo, hi, c1, c2) in [(20, 1.0f32, 2.0f32, 0x3000_2028u32, 0x0000_2020u32), (5, 3.5, 4.5, 0x2000_2020, 0x0000_2030)] {
        if w.rng.randi(n - 1) != 0 { continue; }
        let a = w.rng.rand_angle();
        let r = w.rng.randf(0.0, 0.1);
        let s1 = w.rng.randf(0.0, lo) * size;
        let s2 = w.rng.randf(lo, hi) * size;
        let life = w.rng.randf(120.0, 180.0) as i32;
        let (cs, sn) = cs(a);
        let q = w.m(id).position;
        let p = [cs * r + q[0], sn * r + q[1], q[2], q[3]];
        let life = w.ticks(life);
        part52(w, s1, s2, p, c1, c2, life);
    }
}

/// `0x2ef770(moby, dir)`: the goo burst of a hit / death: 20 clumps (from the feet jittered by `0.5·size`, 1 up) of 10
/// type-2 blobs each (`crate::particles::type02`). A clump flies off at `randf(1, 8)` horizontally in a random direction
/// and `randf(2, 6)` up (per second; plus `dir`·`randf(0, 0)`), its phase-B velocity falling by 20·dt² per tick of
/// `ticks(30)`; each blob moves the clump's point by `randf(±0.2)` and its phase-B velocity by `randf(±0.5)·dt` (both
/// kept for the next blob), sizes `r·0.125` / `r·0.065` (r = `randf(0.5, 1.5)`), colours tweened between
/// 0x8000eeee / 0x8000ff90 and 0xffee, phases `ticks(10)`, `ticks(30)`, `randf(5, 25)` ticks, texture `def[23]`.
pub fn goo_burst(w: &mut World, id: MobyId, dir: V, size: f32) {
    for _ in 0..20 {
        let mut p = w.m(id).position;
        jitter(w, 0.5 * size, &mut p);
        p[2] += 1.0;
        let a = w.rng.rand_angle();
        let s1 = w.rng.randf(1.0, 8.0) * super::DT;
        let s2 = w.rng.randf(2.0, 6.0) * super::DT;
        let s3 = w.rng.randf(0.0, 0.0);
        let mut v1 = scale(dir, s3 * super::DT);
        let (cs_, sn) = cs(a);
        v1[0] += cs_ * s1;
        v1[1] += sn * s1;
        v1[2] += s2;
        let mut v2 = v1;
        let t30 = w.ticks(30);
        v2[2] -= 20.0 * super::DT2 * t30 as f32;
        for _ in 0..10 {
            let r = w.rng.randf(0.5, 1.5);
            v1[3] = r * 0.125;
            v2[3] = r * 0.065;
            jitter(w, 0.2, &mut p);
            jitter(w, 0.5 * super::DT, &mut v2);
            let f = w.rng.randf(0.0, 1.0);
            let c1 = crate::particles::tween_color(f.to_bits(), 0x8000_eeee, 0x8000_ff90);
            let f = w.rng.randf(0.0, 1.0);
            let c2 = crate::particles::tween_color(f.to_bits(), 0x0000_ffee, 0x0000_ffee);
            let (ta, tb) = (w.ticks(10), w.ticks(30));
            let tc = w.svc.timing.scale(to_pf(w.rng.randf(5.0, 25.0))).to_f32() as i32;
            let a = crate::particles::type02::Spawn { pos: p, v1, v2, c1, c2, t: [ta, tb, tc], def: -1 };
            part02(w, &a);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Body pieces (BreakFxB 0x278ad8, FxGroupUpdate 0x30cd18)

/// The class bounding sphere (class header +0x30) of `o_class`: [`super::Globals::class_spheres`], else the first
/// sequence's sphere.
fn class_sphere(w: &World, o_class: i16) -> [f32; 4] {
    if let Some(s) = w.svc.creatures.class_spheres.get(&o_class) { return *s; }
    w.classes.anim(o_class).and_then(|c| c.sequence(0)).map(|q| q.header.sphere).unwrap_or([0.0; 4])
}

fn euler_rows(e: V) -> [V; 3] {
    let r = rc_formats::moby_light::rotation_rows([e[0], e[1], e[2]]);
    r.map(|row| row.map(f32::from_bits))
}

/// `MatrixMulVec3(out, v, rows)` 0x2215e0: `r0·v.x + r1·v.y + r2·v.z`.
fn apply(rows: &[V; 3], v: V) -> V {
    std::array::from_fn(|k| if k == 3 { 0.0 } else { rows[0][k] * v[0] + rows[1][k] * v[1] + rows[2][k] * v[2] })
}

/// `BreakFxB(g, parent, class, pos, rot, timer, flags, vel, spin, sphere)` 0x278ad8 with the zero vectors the
/// creatures pass (random velocity, spin and the class sphere): a piece moby of `class` at `pos`, rows from `rot`,
/// scale `class scale · parent scale / parent class scale`, the parent's light word and ambient, state 1, update
/// distance 0xff, draw distance 0x80, no collision; pvars: +0x30 timer `trunc(randf(60, 120))` (when `timer` is 0),
/// +0x34 flags (1 fade instead of exploding, 2 the other explosion), +0x38 gravity `randf(10, 15)·dt²` (when `g` is 0),
/// +0x00 velocity `(cos a, sin a)·randf(2, 4)·dt, randf(5, 8)·dt` (a = `rand_angle`), +0x10 spin `rand_vec(dt·π,
/// dt·2π)`, +0x20 the collision sphere (class sphere · scale / 1024). 8 draws.
pub fn break_piece(w: &mut World, parent: MobyId, class: i16, p: V, rot: V, timer: i32, flags: u32) -> Option<MobyId> {
    let m = w.create_moby(class)?;
    let (pscale, pclass, light, ambient) = { let q = w.m(parent); (q.scale, q.o_class, q.light, q.ambient) };
    let pcs = w.classes.info(pclass).map(|c| c.scale).unwrap_or(1.0);
    let cs_ = w.classes.info(class).map(|c| c.scale).unwrap_or(1.0);
    let t = if timer == 0 {
        let x = w.rng.randf(60.0, 120.0);
        w.svc.timing.scale(to_pf(x)).to_f32() as i32
    } else {
        timer
    };
    let g = w.rng.randf(10.0, 15.0) * super::DT2;
    let a = w.rng.rand_angle();
    let s = w.rng.randf(2.0, 4.0) * super::DT;
    let (c, sn) = cs(a);
    let vz = w.rng.randf(5.0, 8.0) * super::DT;
    let spin = w.rng.rand_vec(super::DT * std::f32::consts::PI, super::DT * 2.0 * std::f32::consts::PI);
    let sphere = class_sphere(w, class);
    let mo = w.mm(m);
    mo.visible = 1;
    mo.mode |= mode::KEEP_ROWS;
    mo.scale = cs_ * (pscale / pcs);
    mo.light = light;
    mo.ambient = ambient;
    mo.state = 1;
    mo.position = p;
    let r = euler_rows(rot);
    mo.rows[..3].copy_from_slice(&r);
    mo.update_dist = 0xff;
    mo.draw_dist = 0x80;
    mo.has_collision = false;
    let k = mo.scale * (1.0 / 1024.0);
    let pv_ = &mut mo.pvars;
    pvar::set_i32(pv_, 0x30, t);
    pvar::set_u32(pv_, 0x34, flags);
    pvar::set_ff(pv_, 0x38, g);
    pvar::set_v4f(pv_, 0, [c * s, sn * s, vz, 0.0]);
    pvar::set_v4f(pv_, 0x10, [spin[0], spin[1], spin[2], 0.0]);
    pvar::set_v4f(pv_, 0x20, [sphere[0] * k, sphere[1] * k, sphere[2] * k, sphere[3] * k]);
    w.build_matrix(m);
    Some(m)
}

/// `FxGroupUpdate` 0x30cd18: state 1 flies (spin about the sphere centre, gravity, bounce off the world keeping the
/// speed, the spin re-aimed about `vel × n`) until the timer runs out, then state 2 explodes
/// (`SpawnBeamExplosion(0, 0, r, r/2, 9, r/2, 0, piece, pos, 5, 3, 5, −1, 0, 0)`, or `0x2742a8` with flag 2) and is
/// deleted, or state 3 (flag 1) fades its alpha over `scale(10)` ticks while still flying.
pub fn piece_update(w: &mut World, id: MobyId) {
    let st = w.m(id).state;
    match st {
        1 => {
            if super::dec_timer_pvar_i32(w, id, 0x30) != 0 {
                let f = pvar::u32(&w.m(id).pvars, 0x34);
                w.mm(id).state = if f & 1 != 0 { 3 } else { 2 };
                let t = w.svc.timing.scale(to_pf(10.0)).to_f32() as i32;
                pvar::set_i32(&mut w.mm(id).pvars, 0x30, t);
                return;
            }
        }
        2 => {
            let rows = rows3(w, id);
            let c = add(apply(&rows, super::pv4(w, id, 0x20)), super::pos(w, id));
            let r = super::pf(w, id, 0x2c);
            if pvar::u32(&w.m(id).pvars, 0x34) & 2 != 0 {
                piece_explosion(w, r * 0.5, 0.0, Some(id), c);
            } else {
                let b = Beam { damage_r: 0.0, damage: 0.0, flash: r, flash2: r * 0.5, flash_dist: 9.0, scale: r * 0.5, light: 0.0, streaks: 5, sparks: 3, puffs: 5, debris: 0, sound: -1, shake: false };
                beam_explosion(w, &b, Some(id), c);
            }
            w.delete_moby(id);
            return;
        }
        3 => {
            if super::dec_timer_pvar_i32(w, id, 0x30) != 0 {
                w.delete_moby(id);
                return;
            }
            let d = w.svc.timing.scale(to_pf(10.0)).to_f32();
            let a = ((super::pi32(w, id, 0x30) as f32 / d) * 128.0) as i32;
            w.mm(id).alpha = a as u8;
        }
        0 => {}
        _ => return,
    }
    fly(w, id);
}

fn dec_s16(w: &mut World, id: MobyId, o: usize) -> i32 { super::dec_timer_pvar_s16(w, id, o) }

fn rows3(w: &World, id: MobyId) -> [V; 3] { let r = &w.m(id).rows; [r[0], r[1], r[2]] }

fn fly(w: &mut World, id: MobyId) {
    let spin_e = super::pv4(w, id, 0x10);
    let s = euler_rows(spin_e);
    let sph = super::pv4(w, id, 0x20);
    let old = rows3(w, id);
    let a = apply(&old, sph);
    let new: [V; 3] = old.map(|r| apply(&s, r));
    for k in 0..3 { w.mm(id).rows[k] = [new[k][0], new[k][1], new[k][2], old[k][3]]; }
    let b = apply(&new, sph);
    let mut vel = super::pv4(w, id, 0);
    let p = add(sub(super::pos(w, id), sub(b, a)), vel);
    super::set_pos(w, id, p);
    vel[2] -= super::pf(w, id, 0x38);
    super::set_pv4(w, id, 0, vel);
    let c = add(apply(&new, sph), p);
    let r = super::pf(w, id, 0x2c);
    let Some(o) = w.coll_sphere(pv(c), to_pf(r), 0, Some(id)) else { return };
    let n = set_len3([o.normal[0], o.normal[1], o.normal[2], 0.0], 1.0);
    let l = len3(vel);
    let d = super::dot3(vel, n);
    if d >= 0.0 || d.is_nan() { return; }
    let rf = crate::moby_update::services::reflect(pv(vel), pv(n)).map(|x| f32::from_bits(x.0));
    let v2 = add(rf, scale(n, d * 0.5));
    let v2 = set_len3(v2, l);
    super::set_pv4(w, id, 0, v2);
    // FastVecCross(out, n, vel) = vel × n.
    let ax = [v2[1] * n[2] - v2[2] * n[1], v2[2] * n[0] - v2[0] * n[2], v2[0] * n[1] - v2[1] * n[0], 0.0];
    let ax = set_len3(ax, 1.0);
    let ang = len3(spin_e);
    let q = crate::moby_update::services::axis_angle_quat(to_pf(ang), pv(ax));
    let rows = crate::moby_update::services::quat_rows(q);
    let z = Pf::ZERO;
    let m4 = [rows[0], rows[1], rows[2], [z, z, z, Pf::ONE]];
    let e = crate::moby_update::services::rows_euler(&m4).map(|x| f32::from_bits(x.0));
    super::set_pv4(w, id, 0x10, [e[0], e[1], e[2], spin_e[3]]);
}

// -------------------------------------------------------------------------------------------------
// The explosion light (class 0x27f)

/// An explosion light template (the 0x50-byte records at 0x1b0770 / 0x1b06d0 / 0x1b0720 that 0x2f3570 copies).
#[derive(Clone, Copy, Debug)]
pub struct LightTemplate {
    /// +0x00: offset (w kept).
    pub offset: V,
    /// +0x10 bytes: intensity ×100 start (the light's `+0x24`), then r, g, b triples (start, max, min, up, down per
    /// channel in the game's byte order +0x11..+0x1f).
    pub bytes: [u8; 16],
    /// +0x20 / +0x24 / +0x28: radius start / max / min; +0x2c / +0x30 radius grow / shrink per tick.
    pub radius: [f32; 3],
    pub grow: f32,
    pub shrink: f32,
    /// +0x34: life in ticks (−1 forever); +0x38: flags.
    pub life: i32,
    pub flags: u32,
    /// +0x3c draw distance byte, +0x3d/+0x3e/+0x3f hold counts, +0x42/+0x44/+0x46 periods, +0x48 radius hold,
    /// +0x49 start delay.
    pub draw: u8,
    pub holds: [u8; 3],
    pub periods: [u16; 3],
    pub period_r: u16,
    pub hold_r: u8,
    pub delay: u8,
}

/// `0x1b0770`, the death explosion's light (`0x273f50` writes the three radii).
pub const LIGHT_DEATH: LightTemplate = LightTemplate {
    offset: [0.0, 0.0, 1.3, 0.0],
    bytes: [0x00, 0x00, 0xff, 0x00, 0x13, 0x0f, 0x00, 0xff, 0x00, 0x13, 0x11, 0x00, 0x64, 0x00, 0x13, 0x13],
    radius: [13.0, 13.0, 13.0],
    grow: 0.01,
    shrink: 0.01,
    life: 45,
    flags: 0x0008_0000,
    draw: 0xff,
    holds: [0, 0, 0],
    periods: [0, 0, 0],
    period_r: 0,
    hold_r: 0,
    delay: 0,
};

/// `0x1b06d0`, `SpawnBeamExplosion`'s light (normal colours; 0x1b0720 is the param_18 variant).
pub const LIGHT_BEAM: LightTemplate = LightTemplate {
    offset: [0.0, 0.0, 0.3, 0.0],
    bytes: [0x00, 0x00, 0xff, 0x00, 0x11, 0x0b, 0x00, 0xff, 0x00, 0x11, 0x0d, 0x00, 0x64, 0x00, 0x10, 0x0d],
    radius: [15.0, 15.0, 15.0],
    grow: 0.01,
    shrink: 0.01,
    life: 60,
    flags: 0x0008_0000,
    draw: 0xff,
    holds: [0, 0, 0],
    periods: [0, 0, 0],
    period_r: 0,
    hold_r: 0,
    delay: 0,
};

/// `0x20a930`, the Bomb Glove explosion's light (copied to the stack by `0x2c3300`, spawned at the explosion).
pub const LIGHT_BOMB: LightTemplate = LightTemplate {
    offset: [0.0, 0.0, 1.0, 0.0],
    bytes: [0x00, 0x00, 0xff, 0x00, 0x80, 0x09, 0x00, 0xff, 0x00, 0x80, 0x0c, 0x00, 0x64, 0x00, 0x80, 0x0d],
    radius: [20.0, 20.0, 20.0],
    grow: 0.01,
    shrink: 0.01,
    life: 70,
    flags: 0x0008_0000,
    draw: 0xff,
    holds: [0, 0, 0],
    periods: [0, 0, 0],
    period_r: 0,
    hold_r: 0,
    delay: 3,
};

/// The light moby's pvar offsets (0x2f3748's record).
mod lp {
    pub const OFFSET: usize = 0x00;
    pub const BASE: usize = 0x10;
    pub const PARENT: usize = 0x20;
    pub const R_MAX: usize = 0x38;
    pub const R_MIN: usize = 0x3c;
    pub const R_UP: usize = 0x40;
    pub const R_DOWN: usize = 0x44;
    pub const LIFE: usize = 0x48;
    pub const FLAGS: usize = 0x4c;
    pub const SLOT: usize = 0x50;
    pub const RGB: usize = 0x54;
    pub const RADIUS: usize = 0x60;
}

/// `0x2f3570(template, pos, 0, 0)`: `CreateMoby(0x27f)` hidden (mode |= 1), update distance 0xff, +0x31 = 1, at `pos`,
/// with the template copied into its pvars; then the matrix.
pub fn light_spawn(w: &mut World, t: &LightTemplate, p: V) -> Option<MobyId> {
    let m = w.create_moby(LIGHT_CLASS)?;
    let mo = w.mm(m);
    mo.update_dist = 0xff;
    mo.visible = 1;
    mo.mode |= 1;
    mo.state = 0;
    mo.cmd = 0;
    mo.position = p;
    mo.draw_dist = t.draw as i16;
    if mo.pvars.len() < 0x80 { mo.pvars.resize(0x80, 0); }
    let pv_ = &mut mo.pvars;
    pvar::set_v4f(pv_, lp::OFFSET, t.offset);
    pvar::set_i32(pv_, lp::PARENT, 0);
    pv_[0x24..0x34].copy_from_slice(&t.bytes);
    pvar::set_ff(pv_, 0x34, t.radius[0]);
    pvar::set_ff(pv_, lp::R_MAX, t.radius[1]);
    pvar::set_ff(pv_, lp::R_MIN, t.radius[2]);
    pvar::set_ff(pv_, lp::R_UP, t.grow);
    pvar::set_ff(pv_, lp::R_DOWN, t.shrink);
    pvar::set_i32(pv_, lp::LIFE, t.life);
    pvar::set_u32(pv_, lp::FLAGS, t.flags);
    pv_[0x68] = t.holds[0];
    pv_[0x69] = t.holds[1];
    pv_[0x6a] = t.holds[2];
    pv_[0x6b] = t.hold_r;
    pvar::set_i16(pv_, 0x6c, t.periods[0] as i16);
    pvar::set_i16(pv_, 0x6e, t.periods[1] as i16);
    pvar::set_i16(pv_, 0x70, t.periods[2] as i16);
    pvar::set_i16(pv_, 0x72, t.period_r as i16);
    pvar::set_i32(pv_, 0x64, -1);
    pv_[0x1c] = t.delay;
    w.build_matrix(m);
    w.svc.creatures.lights += 1;
    Some(m)
}

/// One colour channel of the light (`0x2f3748`'s three identical blocks). `b` = the channel's bytes (start, max, min,
/// up, down; ×100), `bits` = (down, hold, repeat, holding, random). Per tick: the period timer (when non-zero) restarts
/// the ramp (down cleared, the value set to the *min byte* unscaled, a game quirk kept); with the random bit the value is
/// `randf(min, max)/100` (only without a period); else it ramps up by `up/100` to `max/100` (then holds for the hold
/// byte's ticks when the hold bit is set), down by `down/100` to `min/100` (turning up again with the repeat bit when
/// there is no period).
#[allow(clippy::too_many_arguments)]
fn channel(w: &mut World, id: MobyId, flags: &mut u32, v: &mut f32, b: [u8; 5], period: u16, timer_off: usize, hold_off: usize, bits: [u32; 5]) {
    let [_start, max, min, up, down] = b;
    let [b_down, b_hold, b_repeat, b_holding, b_random] = bits;
    if period != 0 {
        let mut t = super::pi16(w, id, timer_off);
        let r = crate::moby_update::services::fast_dec_timer_s16(&mut t);
        super::set_pi16(w, id, timer_off, t);
        if r != 0 {
            let p = w.ticks(period as i32);
            super::set_pi16(w, id, timer_off, p as i16);
            *flags &= !b_down;
            *v = min as f32;
        }
    }
    if *flags & b_random != 0 {
        if period == 0 { *v = w.rng.randf(min as f32 / 100.0, max as f32 / 100.0); }
        return;
    }
    if *flags & (b_hold | b_holding) == b_holding { *flags ^= b_holding; }
    if *flags & b_holding != 0 {
        if super::pu8(w, id, hold_off) == 0 { return; }
        let mut h = super::pu8(w, id, hold_off + 0x14);
        let r = crate::moby_update::services::fast_dec_timer_u8(&mut h);
        super::set_pu8(w, id, hold_off + 0x14, h);
        if r != 0 { *flags ^= b_holding; }
        return;
    }
    if *flags & b_down == 0 {
        *v += (up as f32 / 100.0) * super::SPEED;
        let top = max as f32 / 100.0;
        if top <= *v {
            *v = top;
            if *flags & b_hold == 0 { *flags ^= b_down; } else {
                let hold = super::pu8(w, id, hold_off);
                let t = w.ticks(hold as i32) as u8;
                super::set_pu8(w, id, hold_off + 0x14, t);
                *flags = (*flags | b_holding) ^ b_down;
            }
        }
    } else {
        *v -= (down as f32 / 100.0) * super::SPEED;
        let bot = min as f32 / 100.0;
        if *v <= bot {
            *v = bot;
            if *flags & b_repeat != 0 && period == 0 { *flags ^= b_down; }
        }
    }
}

/// `0x2f3748`: the explosion light (class 0x27f). State 0 keeps its start position, radius and colours (w = −1 slot);
/// then per tick: the start delay (+0xbc), the life timer (deleted when it runs out), the view test
/// (`FastBSphereCheck(255, pos, radius)`; out of view the light slot is freed and nothing else runs), the position
/// (follows the parent, plus the offset: fixed, `randf(0, 1)`-scaled (flag 0x100000) or `randf(±offset)` (0x300000)),
/// the radius (grow/shrink between its limits, or `randf(min, max)` with flag 0x80000) and the colour channels; then
/// its point-light slot (+0x50, −1 none): taken with `WritePointLight_B` 0x252750 when it has none, else rewritten
/// (colour, intensity +0x24 / 100, position, radius); freed on the view cull and at the end of its life
/// ([`crate::point_lights`]).
pub fn light_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { return; }
    if w.m(id).state == 0 {
        let p = super::pos(w, id);
        super::set_pv4(w, id, lp::BASE, p);
        let radius0 = super::pf(w, id, 0x34);
        super::set_pf(w, id, lp::RADIUS, radius0);
        for (k, o) in [0x25usize, 0x2a, 0x2f].iter().enumerate() {
            let v = super::pu8(w, id, *o) as f32 / 100.0;
            super::set_pf(w, id, lp::RGB + 4 * k, v);
        }
        let d = super::pu8(w, id, 0x1c);
        w.mm(id).cmd = d;
        pvar::set_i32(&mut w.mm(id).pvars, lp::SLOT, -1);
        w.mm(id).state = 1;
    }
    let c = w.m(id).cmd;
    if c != 0 {
        w.mm(id).cmd = c - 1;
        return;
    }
    if super::pi32(w, id, lp::LIFE) != -1 && super::dec_timer_pvar_i32(w, id, lp::LIFE) != 0 {
        free_slot(w, id);
        w.delete_moby(id);
        return;
    }
    let mut radius = super::pf(w, id, lp::RADIUS);
    let p = super::pos(w, id);
    if !in_view(w, 255.0, p, radius) {
        free_slot(w, id);
        return;
    }
    let mut flags = pvar::u32(&w.m(id).pvars, lp::FLAGS);
    let base = super::pv4(w, id, lp::BASE);
    if flags & 0x80_0000 == 0 {
        let off = super::pv4(w, id, lp::OFFSET);
        let o = if flags & 0x10_0000 == 0 {
            [off[0], off[1], off[2], 0.0]
        } else if flags & 0x20_0000 == 0 {
            let k = w.rng.randf(0.0, 1.0);
            scale([off[0], off[1], off[2], 0.0], k)
        } else {
            let x = w.rng.randf(-off[0], off[0]);
            let y = w.rng.randf(-off[1], off[1]);
            let z = w.rng.randf(-off[2], off[2]);
            [x, y, z, 0.0]
        };
        let np = add(base, o);
        super::set_pos(w, id, [np[0], np[1], np[2], p[3]]);
    } else {
        super::set_pos(w, id, [base[0], base[1], base[2], p[3]]);
    }
    // Radius period (+0x72, timer +0x7a): restart at the minimum, going up.
    let period_r = super::pi16(w, id, 0x72) as u16;
    let mut reset = false;
    if period_r != 0 && dec_s16(w, id, 0x7a) != 0 {
        let t = w.ticks(period_r as i32);
        super::set_pi16(w, id, 0x7a, t as i16);
        if flags & 4 != 0 {
            reset = true;
            radius = super::pf(w, id, lp::R_MIN);
            flags &= !8;
        }
    }
    // Radius (bits 1 hold, 2 holding, 4 repeat, 8 down, 0x80000 random).
    if flags & 0x8_0000 != 0 {
        if period_r == 0 || reset { radius = w.rng.randf(super::pf(w, id, lp::R_MIN), super::pf(w, id, lp::R_MAX)); }
    } else {
        if flags & 3 == 2 { flags ^= 2; }
        if flags & 2 == 0 {
            if flags & 8 == 0 {
                radius += super::pf(w, id, lp::R_UP) * super::SPEED;
                let mx = super::pf(w, id, lp::R_MAX);
                if mx <= radius {
                    radius = mx;
                    if flags & 1 == 0 { flags ^= 8; } else {
                        let h = super::pu8(w, id, 0x6b);
                        let t = w.ticks(h as i32) as u8;
                        super::set_pu8(w, id, 0x7f, t);
                        flags = (flags | 2) ^ 8;
                    }
                }
            } else {
                radius -= super::pf(w, id, lp::R_DOWN) * super::SPEED;
                let mn = super::pf(w, id, lp::R_MIN);
                if radius <= mn {
                    radius = mn;
                    if flags & 4 != 0 && period_r == 0 { flags ^= 8; }
                }
            }
        } else if super::pu8(w, id, 0x6b) != 0 {
            let mut h = super::pu8(w, id, 0x7f);
            if crate::moby_update::services::fast_dec_timer_u8(&mut h) != 0 { flags ^= 2; }
            super::set_pu8(w, id, 0x7f, h);
        }
    }
    super::set_pf(w, id, lp::RADIUS, radius);
    // Colour channels: r (bits 0x10..0x80, random 0x10000), g (0x100..0x800, 0x20000), b (0x1000..0x8000, 0x40000).
    type Chan = ([usize; 5], u16, usize, usize, [u32; 5]);
    let chans: [Chan; 3] = [
        ([0x25, 0x26, 0x27, 0x28, 0x29], super::pi16(w, id, 0x6c) as u16, 0x74, 0x68, [0x80, 0x10, 0x40, 0x20, 0x1_0000]),
        ([0x2a, 0x2b, 0x2c, 0x2d, 0x2e], super::pi16(w, id, 0x6e) as u16, 0x76, 0x69, [0x800, 0x100, 0x400, 0x200, 0x2_0000]),
        ([0x2f, 0x30, 0x31, 0x32, 0x33], super::pi16(w, id, 0x70) as u16, 0x78, 0x6a, [0x8000, 0x1000, 0x4000, 0x2000, 0x4_0000]),
    ];
    for (k, (offs, period, toff, hoff, bits)) in chans.iter().enumerate() {
        let b = offs.map(|o| super::pu8(w, id, o));
        let mut v = super::pf(w, id, lp::RGB + 4 * k);
        channel(w, id, &mut flags, &mut v, b, *period, *toff, *hoff, *bits);
        super::set_pf(w, id, lp::RGB + 4 * k, v);
    }
    pvar::set_u32(&mut w.mm(id).pvars, lp::FLAGS, flags);
    // The slot: taken or rewritten with this tick's colour, intensity, position and radius.
    let l = crate::point_lights::PointLight {
        color: [0, 1, 2].map(|k| super::pf(w, id, lp::RGB + 4 * k)),
        intensity: super::pu8(w, id, 0x24) as f32 / 100.0,
        pos: { let q = super::pos(w, id); [q[0], q[1], q[2]] },
        radius: super::pf(w, id, lp::RADIUS),
    };
    let slot = super::pi32(w, id, lp::SLOT);
    if slot == -1 {
        let load = f32::from_bits(w.svc.frame_load[1].0);
        let got = w.svc.point_lights.alloc(l, load).map_or(-1, |i| i as i32);
        pvar::set_i32(&mut w.mm(id).pvars, lp::SLOT, got);
    } else {
        w.svc.point_lights.set(slot as usize, l);
    }
}

/// `FreePointLight` of the light moby's slot (+0x50), which becomes −1.
fn free_slot(w: &mut World, id: MobyId) {
    let slot = super::pi32(w, id, lp::SLOT);
    if slot != -1 {
        w.svc.point_lights.free(slot as usize);
        pvar::set_i32(&mut w.mm(id).pvars, lp::SLOT, -1);
    }
}
