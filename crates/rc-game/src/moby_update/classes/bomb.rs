//! **The Bomb Glove's bomb** (class 121, level01 `0x2c3300`) and **its fireballs** (class 122, `0x2c4d88`), plus
//! the glove-side writers of a bomb's pvars: `0x2c2640` (the bomb created in the glove, [`create_held`]) and
//! `0x2c27a8` (its release, [`release`]). The glove's own update is `crate::hero::weapons` (hand item 10).
//!
//! **The bomb** (moby +0x20): **0 held** — the glove places it every tick (collision off, +0x98 = 1), a pulsing
//! glow; hidden while the first-person camera is up (0x1413f5, the hero's `f13f5`); deleted when the hand no
//! longer holds the glove. **1 flying** — the glow fades from red to grey over its 30-tick timer (+0x6a), it spins
//! (x 120°/s, y 450°/s), moves by its velocity (+0x00) under gravity 11 u/s² (1.1 under water), and tests its path
//! (`CollLine_Fix(old, new, 0x10, bomb, tmpl)`: template flags 0x830000 (| 1 after its first 10 ticks), damage 2,
//! so a crate on the path gets its hit): a hit explodes it — except Ratchet or the glove in its first 10 ticks;
//! water (surface 0) while falling: it sinks at 1.5 u/s until its 300-tick fuse (+0x54) runs out; the fuse ends in
//! the air too. **The explosion** (moby +0xbc = 1): the sphere 0.5 lists the mobys (`coll_sphere_mobys`, flags
//! 0x10) and hits them (`0x26f8f8`: damage 2, flags 0x830000) → state **2**; the fireballs (10 low ones, 4 + 1 high,
//! the last toward the camera), the type-11 smoke rings (1..4 by the camera distance), the flashes (class 1192:
//! two more when the camera is farther than 9), the class sound 0, the camera shake along up `0.4 − 0.0175·d`
//! (0.05 beyond 20) for 25 ticks. **2 exploding**: hidden, a sphere growing from 0.5 to 2.5 over 15 ticks hits
//! every moby it touches (damage 2, 0x830000), then the bomb is deleted.
//!
//! **Fireball** (class 122): moves by its velocity, gravity 14.6 u/s², spins by its two rates, shrinks to 0 over
//! the last quarter of its life (random 60..120 / 60..90 ticks), deleted when it runs out or leaves the positive
//! octant.
//!
//! Native `f32`. **Not ported** (their draws are not made, so the one `rand` stream diverges from the PS2 after an
//! explosion; counted in `Services::fx.unported`): the flight's trail particles (every 8th tick, `0x27dc98`), the
//! under-water bubbles and splash (`0x2840e0`, `0x2b82a8`), the fireballs' smoke (`0x27e538`), the scorch / spark
//! particles near the ground (`0x288d90`, `0x280bd0`), the enemy alert `0x2f3570`, the aim-preview `0x2c2be0`, the
//! pass-through classes of the hit test (update `0x160770`), the gold-glove colour shifts (`0x270fa8`, `0x270f48`:
//! identity without the gold glove).

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{self as sv, pvar as p, HitTemplate, World};
use crate::ps2v::Pf;
use std::f32::consts::PI;

pub const UPDATE_FN: u32 = 0x2c3300;
pub const FIREBALL_UPDATE_FN: u32 = 0x2c4d88;
/// The bomb (`CreateMoby(0x79)`) and its fireball (`0x7a`).
pub const CLASSES: [i16; 1] = [121];
pub const FIREBALL_CLASSES: [i16; 1] = [122];
pub const BOMB_CLASS: i16 = 121;
pub const FIREBALL_CLASS: i16 = 122;
/// The explosion's flash class (`0x309a68`: `CreateMoby(0x4a8)`, the shared `FlashUpdate`).
pub const FLASH_CLASS: i16 = 1192;
/// The glove (hand item 10's class).
pub const GLOVE_CLASS: i16 = 0xc0;

/// Bomb states (+0x20).
pub const HELD: u8 = 0;
pub const FLYING: u8 = 1;
pub const EXPLODING: u8 = 2;

/// Pvar offsets of the bomb (the game's layout; the owner +0x50 holds 1 for "the glove" instead of its pointer).
pub mod pv {
    pub const VEL: usize = 0x00;
    pub const OWNER: usize = 0x50;
    pub const LIFE: usize = 0x54;
    pub const FLAG56: usize = 0x56;
    pub const SPIN_X: usize = 0x58;
    pub const SPIN_Y: usize = 0x5c;
    pub const TARGET: usize = 0x60;
    pub const WATER: usize = 0x68;
    pub const FLASH: usize = 0x6a;
    pub const YAW: usize = 0x6c;
}

fn dt() -> f32 { sv::DT.to_f32() }
fn dt2() -> f32 { sv::DT2.to_f32() }
fn wrap(a: f32) -> f32 {
    let mut x = a;
    if x >= PI { x -= 2.0 * PI; }
    if x < -PI { x += 2.0 * PI; }
    x
}
fn sub3(a: [f32; 4], b: [f32; 4]) -> [f32; 4] { [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0] }
fn add3(a: [f32; 4], b: [f32; 4]) -> [f32; 4] { [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3]] }
fn len3(a: [f32; 4]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn setlen(a: [f32; 4], l: f32) -> [f32; 4] {
    let n = len3(a);
    if n == 0.0 { [0.0; 4] } else { [a[0] * l / n, a[1] * l / n, a[2] * l / n, 0.0] }
}
fn dot3(a: [f32; 4], b: [f32; 4]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn pv4(a: [f32; 4]) -> [Pf; 4] { a.map(Pf::f) }

/// `FUN_002c2640(glove, point)`: a bomb in the glove at `point` (the table's `CreateMoby(0x79)` is the caller's;
/// this writes the new moby): update and draw distances 0xff, visible, state 0, owner the glove, the timers 0, the
/// yaw of Ratchet's facing, a random rotation (3 draws `randf(−π, π)`) and spin rates (2 draws `randf(60°, 180°)·dt`).
pub fn init_held(m: &mut crate::moby_runtime::Moby, rng: &mut crate::rng::Rng, point: [f32; 3], facing_yaw: f32) {
    let r = [rng.randf(-PI, PI), rng.randf(-PI, PI), rng.randf(-PI, PI)];
    m.update_dist = 0xff;
    m.draw_dist = 0xff;
    m.visible = 1;
    m.state = HELD;
    if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
    let pv = &mut m.pvars;
    p::set_i32(pv, pv::OWNER, 1);
    p::set_i16(pv, pv::FLAG56, 0);
    p::set_i16(pv, pv::FLASH, 0);
    p::set_i32(pv, pv::TARGET, 0);
    p::set_ff(pv, pv::YAW, facing_yaw);
    p::set_ff(pv, 0x64, 0.0);
    m.position = [point[0], point[1], point[2], m.position[3]];
    p::set_v4f(pv, pv::VEL, [0.0; 4]);
    m.rotation = [r[0], r[1], r[2], 0.0];
    let a = rng.randf(dt() * (PI / 3.0), dt() * PI);
    let b = rng.randf(dt() * (PI / 3.0), dt() * PI);
    p::set_ff(pv, pv::SPIN_X, a);
    p::set_ff(pv, pv::SPIN_Y, b);
}

/// `FUN_002c27a8(point, bomb)`: the release — the flash timer 30, state 1, the velocity the glove's aim left in the
/// pvars (plus Ratchet's velocity while grinding: `grind_vel`), the 300-tick fuse, at `point`; a wall between
/// Ratchet (`hero_pos`, at the point's height) and the point explodes it there at once. Returns the line to test
/// `(from, to)`: the caller runs it (it needs the collision) and passes the hit point to [`release_hit`].
pub fn release(m: &mut crate::moby_runtime::Moby, point: [f32; 3], grind_vel: Option<[f32; 3]>) {
    if m.pvars.len() < 0x80 { m.pvars.resize(0x80, 0); }
    let pv = &mut m.pvars;
    p::set_i16(pv, pv::FLASH, 30);
    m.state = FLYING;
    if let Some(g) = grind_vel {
        let v = p::v4f(pv, pv::VEL);
        p::set_v4f(pv, pv::VEL, [v[0] + g[0], v[1] + g[1], v[2] + g[2], v[3]]);
    }
    p::set_i16(pv, pv::LIFE, 300);
    m.position = [point[0], point[1], point[2], m.position[3]];
}

/// The release's wall test hit: explode at `hit` (+0xbc = 1).
pub fn release_hit(m: &mut crate::moby_runtime::Moby, hit: [f32; 3]) {
    m.cmd = 1;
    m.position = [hit[0], hit[1], hit[2], m.position[3]];
}

/// The bomb's template: `dir` (xy of the unit velocity, z 1), flags 0x830000 (| 1 once 10 ticks old), damage 2,
/// +0x18 = 2 / 1, +0x1a = the class.
fn template(id: MobyId, vel: [f32; 4], life: i16) -> HitTemplate {
    let n = setlen(vel, 1.0);
    let mut flags = 0x83_0000;
    if (life as i32) < 300 - 10 { flags |= 1; }
    HitTemplate { dir: [Pf::f(n[0]), Pf::f(n[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 }
}

/// `0x2c3300`.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x80 { w.mm(id).pvars.resize(0x80, 0); }
    let hand_glove = w.hero.items.slot.item.as_ref().is_some_and(|m| m.o_class == GLOVE_CLASS);
    let st = w.m(id).state;
    if (!hand_glove && st == HELD) || (w.hero.mode != 0 && st == HELD) {
        w.delete_moby(id);
        return;
    }
    // Hidden while held in first person (0x1413f5), shown otherwise.
    {
        let fp = w.hero.f13f5 != 0;
        let m = w.mm(id);
        if fp && m.state == HELD { m.mode |= 0x41; } else { m.mode &= !0x41; }
    }
    match st {
        FLYING => fly(w, id),
        EXPLODING => exploding(w, id),
        _ => {
            // Held: the glow pulse (+0x6a counts up).
            let m = w.mm(id);
            let c = p::i16(&m.pvars, pv::FLASH).wrapping_add(1);
            p::set_i16(&mut m.pvars, pv::FLASH, c);
            let a = sv::normalize_angle(Pf::f(c as f32 * 0.068)).to_f32();
            let v = ((a.sin() * 96.0) as i32 + 0x9f) as u32 & 0xff;
            m.glow = 0xff00_0000 | v << 16 | v << 8 | v;
        }
    }
}

/// State 1.
fn fly(w: &mut World, id: MobyId) {
    // The glow: (255, 64, 64) → (128, 128, 128) over the 30-tick timer.
    {
        let m = w.mm(id);
        let mut t = p::i16(&m.pvars, pv::FLASH);
        sv::fast_dec_timer_s16(&mut t);
        p::set_i16(&mut m.pvars, pv::FLASH, t);
        let k = t as f32 / 30.0;
        let r = ((128.0 - 255.0) * k + 255.0) as i32 as u32;
        let g = ((128.0 - 64.0) * k + 64.0) as i32 as u32;
        let b = ((128.0 - 64.0) * k + 64.0) as i32 as u32;
        m.glow = 0xff00_0000 | (b & 0xff) << 16 | (g & 0xff) << 8 | (r & 0xff);
    }
    if p::i16(&w.m(id).pvars, pv::WATER) == 0 && w.counter & 7 == 0 { w.svc.unported("bomb trail particles 0x27dc98"); }
    let (old, vel, water, life);
    {
        let m = w.mm(id);
        m.rotation[0] = wrap(m.rotation[0] + dt() * 2.094_395_2);
        m.rotation[1] = wrap(m.rotation[1] + dt() * 7.853_981_5);
        old = m.position;
        let v = p::v4f(&m.pvars, pv::VEL);
        m.position = add3(m.position, v);
        let wt = p::i16(&m.pvars, pv::WATER);
        let mut v2 = v;
        if wt == 0 { v2[2] -= dt2() * 11.0; } else { v2[2] -= dt2() * 11.0 / 10.0; }
        p::set_v4f(&mut m.pvars, pv::VEL, v2);
        vel = v2;
        water = wt;
        life = p::i16(&m.pvars, pv::LIFE);
        if wt != 0 { p::set_i16(&mut m.pvars, pv::WATER, wt.wrapping_add(1)); }
    }
    if water != 0 { w.svc.unported("bomb bubbles 0x2840e0"); }
    let tmpl = template(id, vel, life);
    let pos = w.m(id).position;
    let hit = sv::line_hit_in(w.table, w.svc, w.classes, w.coll, pv4(old), pv4(pos), 0x10, Some(id), &tmpl);
    let mut drift = [0.0f32; 4];
    let mut normal = [0.0f32; 4];
    match hit {
        None => {
            let m = w.mm(id);
            let mut t = p::i16(&m.pvars, pv::LIFE);
            if sv::fast_dec_timer_s16(&mut t) != 0 {
                m.cmd = 1;
                drift = vel;
            }
            p::set_i16(&mut m.pvars, pv::LIFE, t);
        }
        Some(h) => {
            let water_face = h.moby.is_none() && h.surface_id() == 0;
            if water == 0 && water_face && vel[2] < 0.0 {
                // Into water: the splash, sink slowly (the ripple and the splash particles are not ported).
                w.svc.unported("bomb splash 0x2b82a8 / 0x2845a8");
                let m = w.mm(id);
                p::set_i16(&mut m.pvars, pv::WATER, 1);
                p::set_v4f(&mut m.pvars, pv::VEL, [0.0, 0.0, dt() * -1.5, 0.0]);
            } else if !(water == 0 && water_face) {
                let young = (life as i32) >= 300 - 10;
                let own = h.moby.is_some() && (h.moby == w.hero_moby);
                if !(own && young) {
                    let m = w.mm(id);
                    m.cmd = 1;
                    m.position = [h.point[0], h.point[1], h.point[2], m.position[3]];
                    let n = [h.normal[0], h.normal[1], h.normal[2], 0.0];
                    let nn = setlen(n, 1.0);
                    let refl = sub3(vel, [nn[0] * 2.0 * dot3(vel, nn), nn[1] * 2.0 * dot3(vel, nn), nn[2] * 2.0 * dot3(vel, nn), 0.0]);
                    drift = setlen(refl, dt() + dt());
                    normal = nn;
                }
            }
        }
    }
    if w.m(id).cmd != 0 { explode(w, id, drift, normal); }
}

/// The smoke rings' colours (`0x20a980`, `0x20a998`: `randi(6)` each).
const RING_C1: [u32; 6] = [0x4f00_8fff, 0x4f00_8fff, 0x4f00_7fff, 0x4f00_6fff, 0x2fff_ffff, 0x2fff_ffff];
const RING_C2: [u32; 6] = [0x2f00_5f7f, 0x2f00_4f7f, 0x2f00_3f7f, 0x2f00_004f, 0x2f00_0000, 0x3f00_0000];

/// The explosion (+0xbc ≠ 0; `drift` = the reflected direction ·2·dt (or the velocity at the fuse's end),
/// `normal` = the unit normal of the face it hit).
fn explode(w: &mut World, id: MobyId, drift: [f32; 4], normal: [f32; 4]) {
    let k = 1.0f32;
    let drift = drift.map(|x| x * k);
    let pos = w.m(id).position;
    let cam = w.camera.map(|x| x.to_f32());
    let to_cam = sub3(cam, pos);
    let dcam = len3(to_cam);
    let water = p::i16(&w.m(id).pvars, pv::WATER);
    let dry = water < 20;
    if dry {
        // The sphere lists the mobys; 0x26f8f8 hits them (damage 2, flags 0x830000).
        let tmpl = HitTemplate { dir: [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 };
        w.sphere_mobys(Pf::f(k * 0.5), pv4(pos), 0x10, Some(id), Some(&tmpl));
        w.mm(id).state = EXPLODING;
    }
    // The fireballs spreading along the face: 10 low ones (type 0), then 4 high ones (type 1).
    let spread = |w: &mut World, lo: f32, hi: f32| -> [f32; 4] {
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let mut v = [x, y, 0.0, 0.0];
        let d = dot3(v, normal);
        v = sub3(v, [normal[0] * d, normal[1] * d, normal[2] * d, 0.0]);
        let u = w.rng.randf(0.0, 1.0);
        v = add3(v, [normal[0] * u, normal[1] * u, normal[2] * u, 0.0]);
        let s = w.rng.randf(lo, hi);
        add3(setlen(v, k * s * dt()), drift)
    };
    for _ in 0..10 {
        let v = spread(w, 3.5, 6.5);
        let life = w.rng.rand_range(60, 120);
        fireball(w, pos, v, life, 0);
    }
    if dry {
        for _ in 0..4 {
            let v = spread(w, 6.5, 10.0);
            let life = w.rng.rand_range(60, 90);
            fireball(w, pos, v, life, 1);
        }
        // One toward the camera.
        let x = w.rng.randf(-1.0, 1.0);
        let y = w.rng.randf(-1.0, 1.0);
        let z = w.rng.randf(-1.0, 1.0);
        let mut c = to_cam;
        c[2] += dcam * 0.5;
        let mut v = add3(setlen([x, y, z, 0.0], (dcam / 5.0) * dt()), setlen(c, (dcam + dcam) * dt()));
        let l = len3(v);
        if l > dt() * 10.0 { v = setlen(v, dt() * 10.0); }
        let life = w.rng.rand_range(60, 90);
        fireball(w, pos, v, life, 1);
        // The smoke rings (type 11), 1..4 by the camera distance, slower when the camera is close.
        let n = if dcam < 6.0 { dcam as i32 + 1 } else { 4 };
        let slow = if dcam < 7.0 { 7.0 - dcam } else { 0.0 };
        for _ in 0..n {
            let s = w.rng.randf(8.0, 10.0);
            let speed = k * s * dt() - slow * dt();
            let c1 = RING_C1[w.rng.randi(6) as usize];
            let c2 = RING_C2[w.rng.randi(6) as usize];
            let life = w.rng.rand_range(15, 20);
            let t1 = w.rng.rand_range(25, 30);
            w.part11(Pf::f(k * 400_000.0), Pf::f(speed), pv4(pos), pv4(drift), c1, c2, life, t1, 0, 0);
        }
        // The flashes (class 1192).
        if dcam > 9.0 {
            flash(w, k * 4.0, id, pos, drift, 15, 0x7f, 0x7f, 0x7f, 0x20);
            flash(w, k * 4.0, id, pos, drift, 24, 0x7f, 0x20, 0, 0x20);
        }
        flash(w, k * 4.0, id, pos, drift, 20, 0x7f, 0x3f, 0, 0x30);
        flash(w, k * 3.5, id, pos, drift, 27, 0x60, 0x10, 0, 0x40);
        flash(w, k * 3.0, id, pos, drift, 29, 0x20, 0, 0, 0x20);
        let m = w.mm(id);
        p::set_ff(&mut m.pvars, pv::VEL, 0.0);
    } else {
        w.svc.unported("bomb under-water debris 0x2840e0");
        let m = w.mm(id);
        p::set_ff(&mut m.pvars, pv::VEL, 0.0);
    }
    w.mm(id).cmd = 0;
    if dry {
        w.play_sound(0, 0, id);
    } else {
        w.svc.unported("bomb under-water sound 0x2a16c0(0x16)");
    }
    if water != 0 { w.svc.unported("bomb ground scorch 0x288d90 / 0x280bd0"); }
    // The camera shake along up (0x167260) for 25 ticks.
    let amp = if dcam < 20.0 { 0.4 - dcam * 0.0175 } else { f32::from_bits(0x3d4c_ccd0) };
    let t = w.ticks(25);
    w.shake_camera(crate::follow_camera::ShakeRequest { axis: crate::follow_camera::ShakeAxis::Up, amp, ticks: t });
    w.svc.unported("bomb alert 0x2f3570");
    if w.m(id).state != EXPLODING {
        w.delete_moby(id);
        return;
    }
    exploding(w, id);
}

/// `FUN_002c4c20(pos, vel, life, type, gold)`: a fireball (class 122): 2 spin draws `randf(360°, 720°)·dt`, and for
/// type 0 a smaller size (`randf(0.5, 0.75)`).
fn fireball(w: &mut World, pos: [f32; 4], vel: [f32; 4], life: i32, ty: u8) {
    let Some(m) = w.create_moby(FIREBALL_CLASS) else { return };
    let a = w.rng.randf(dt() * 2.0 * PI, dt() * 4.0 * PI);
    let b = w.rng.randf(dt() * 2.0 * PI, dt() * 4.0 * PI);
    let shrink = if ty == 0 { Some(w.rng.randf(0.5, 0.75)) } else { None };
    let mo = w.mm(m);
    mo.visible = 1;
    mo.alpha = 0x80;
    mo.draw_dist = 0xff;
    mo.update_dist = 0xff;
    mo.ambient = [0x7f, 0x7f, 0x7f, mo.ambient[3]];
    mo.position = [pos[0], pos[1], pos[2], mo.position[3]];
    if mo.pvars.len() < 0x20 { mo.pvars.resize(0x20, 0); }
    p::set_v4f(&mut mo.pvars, 0, vel);
    p::set_ff(&mut mo.pvars, 0x10, a);
    p::set_ff(&mut mo.pvars, 0x14, b);
    p::set_i16(&mut mo.pvars, 0x1a, life as i16);
    p::set_i16(&mut mo.pvars, 0x18, life as i16);
    if let Some(s) = shrink { mo.scale *= s; }
    let sc = mo.scale;
    p::set_ff(&mut mo.pvars, 0x1c, sc);
    mo.cmd = ty;
    w.build_matrix(m);
}

/// `FUN_00309a68(size, parent, pos, vec, T, r, g, b, a)`: a flash of class 1192 (3 draws for its rotation), the
/// shared `FlashUpdate` (`super::debris::flash_update`) growing it to `size` × its class scale over `T` ticks.
#[allow(clippy::too_many_arguments)]
fn flash(w: &mut World, size: f32, parent: MobyId, pos: [f32; 4], vec: [f32; 4], t: i32, r: u8, g: u8, b: u8, a: u8) {
    let Some(m) = w.create_moby(FLASH_CLASS) else { return };
    let e = [w.rng.randf(-PI, PI), w.rng.randf(-PI, PI), w.rng.randf(-PI, PI), 0.0];
    let mo = w.mm(m);
    mo.state = 0;
    mo.alpha = a;
    mo.visible = 1;
    mo.update_dist = 0xff;
    mo.draw_dist = 0xff;
    mo.ambient = [r, g, b, 0];
    if mo.pvars.len() < 0x20 { mo.pvars.resize(0x20, 0); }
    let full = mo.scale * size;
    p::set_ff(&mut mo.pvars, 0x18, full);
    mo.scale = 0.0;
    p::set_i32(&mut mo.pvars, 0x10, parent as i32 + 1);
    mo.position = [pos[0], pos[1], pos[2], mo.position[3]];
    p::set_v4f(&mut mo.pvars, 0, vec);
    mo.rotation = e;
    p::set_i16(&mut mo.pvars, 0x1e, a as i16);
    p::set_i32(&mut mo.pvars, 0x14, t);
    p::set_i16(&mut mo.pvars, 0x1c, t as i16);
    w.build_matrix(m);
}

/// `0x2c4d88`: the fireball.
pub fn fireball_update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x20 { w.mm(id).pvars.resize(0x20, 0); }
    if w.m(id).cmd & 1 != 0 { w.svc.unported("fireball smoke 0x27e538"); }
    let m = w.mm(id);
    let v = p::v4f(&m.pvars, 0);
    m.rotation[0] = wrap(m.rotation[0] + p::ff(&m.pvars, 0x10));
    m.rotation[1] = wrap(m.rotation[1] + p::ff(&m.pvars, 0x14));
    m.position = add3(m.position, v);
    p::set_ff(&mut m.pvars, 8, v[2] - dt2() * 14.6);
    if m.position[0] < 0.0 || m.position[1] < 0.0 || m.position[2] < 0.0 {
        w.delete_moby(id);
        return;
    }
    let life0 = p::i16(&m.pvars, 0x1a) as i32;
    let q = if life0 < 0 { (life0 + 3) >> 2 } else { life0 >> 2 };
    let timer = p::i16(&m.pvars, 0x18);
    if (timer as i32) < q {
        m.scale = p::ff(&m.pvars, 0x1c) * timer as f32 / q as f32;
    }
    let mut t = timer;
    let done = sv::fast_dec_timer_s16(&mut t) != 0;
    p::set_i16(&mut m.pvars, 0x18, t);
    if done { w.delete_moby(id); }
}

/// State 2: the growing sphere.
fn exploding(w: &mut World, id: MobyId) {
    let k = 1.0f32;
    let pos = w.m(id).position;
    let c = w.m(id).cmd;
    {
        let m = w.mm(id);
        m.mode |= 0x41;
    }
    let r = 0.5 + ((k * 3.0 - 0.5) - 0.5) * (c as f32 / 15.0);
    let tmpl = HitTemplate { dir: [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x83_0000, b18: 2, b19: 1, h1a: BOMB_CLASS as u16, damage: Pf::f(2.0), w20: 1 };
    w.sphere_mobys(Pf::f(r), pv4(pos), 0x10, Some(id), Some(&tmpl));
    let m = w.mm(id);
    m.cmd = m.cmd.wrapping_add(1);
    if m.cmd as i32 > 15 { w.delete_moby(id); }
}

/// `CreateMoby(o_class)` 0x263390 from the hero's code (the glove's bomb): [`World::create_moby`]'s work on the
/// moby system's parts (the hero's hit sink holds them, not a `World`).
pub fn create_from_hero(table: &mut crate::moby_runtime::MobyTable, svc: &mut sv::Services, classes: &dyn sv::ClassData, o_class: i16, counter: u64) -> Option<MobyId> {
    let info = classes.info(o_class);
    let id = table.create(o_class, info.as_ref(), counter)?;
    if svc.snapshots.len() <= id { svc.snapshots.resize(id + 1, None); }
    svc.snapshots[id] = None;
    Some(id)
}

/// `DeleteMoby` 0x2636c0 from the hero's code: [`World::delete_moby`]'s work (state, reuse tick, grid removal).
pub fn delete_from_hero(table: &mut crate::moby_runtime::MobyTable, svc: &mut sv::Services, id: MobyId, counter: u64) {
    table.delete(id, counter);
    std::sync::Arc::make_mut(&mut svc.grid).remove(&mut table.mobys[id]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Hero;
    use crate::moby_runtime::{ClassInfo, Moby, MobyTable};
    use crate::moby_update::scheduler::{self, Scheduler};
    use crate::moby_update::services::{ClassTable, Services};
    use crate::rng::Rng;

    fn classes() -> ClassTable {
        let mut t = ClassTable::default();
        for (slot, oc) in [(1u8, BOMB_CLASS), (2, FIREBALL_CLASS), (3, FLASH_CLASS)] {
            let info = ClassInfo { slot, update_fn: scheduler::port_update_fn(oc), scale: 1.0, ..Default::default() };
            t.classes.insert(oc, (info, None));
        }
        t
    }

    /// A released bomb whose fuse runs out in the air: it explodes (state 2: hidden, the 10 + 5 fireballs and the 3
    /// flashes of a close camera), grows its sphere for 15 ticks and is deleted; the fireballs fall and run out.
    #[test]
    fn fuse_explosion_and_fireballs() {
        let mut h = Moby::init_instance(0, 0, Some(&ClassInfo { scale: 1.0, ..Default::default() }));
        h.mode |= crate::moby_runtime::mode::NO_UPDATE;
        let mut table = MobyTable::new(vec![h], 64);
        let ct = classes();
        let mut hero = Hero::new();
        hero.pos = crate::hero::physics::v4(100.0, 100.0, 50.0);
        let mut rng = Rng::new();
        let mut svc = Services::new();
        let mut sched = Scheduler::new();
        let b = table.create(BOMB_CLASS, ct.classes.get(&BOMB_CLASS).map(|c| &c.0), 0).unwrap();
        init_held(&mut table.mobys[b], &mut rng, [102.0, 100.0, 51.0], 0.0);
        p::set_v4f(&mut table.mobys[b].pvars, pv::VEL, [0.1, 0.0, 0.0, 0.0]);
        release(&mut table.mobys[b], [102.0, 100.0, 51.0], None);
        p::set_i16(&mut table.mobys[b].pvars, pv::LIFE, 3);
        let mut states = Vec::new();
        let mut fireballs = 0;
        let mut flashes = 0;
        for counter in 0..200u64 {
            table.free_slot_pass(counter);
            let mut w = World::new(&mut table, &hero, &mut rng, &ct, &mut svc, counter);
            sched.tick(&mut w);
            states.push(table.mobys[b].state);
            fireballs = fireballs.max(table.mobys.iter().filter(|m| m.o_class == FIREBALL_CLASS && m.state < 0xfd).count());
            flashes = flashes.max(table.mobys.iter().filter(|m| m.o_class == FLASH_CLASS && m.state < 0xfd).count());
        }
        let boom = states.iter().position(|&s| s == EXPLODING).expect("never exploded");
        let gone = states.iter().position(|&s| s >= 0xfd).expect("never deleted");
        assert!(boom <= 4, "exploded at {boom}");
        assert_eq!(gone - boom, 15, "15 ticks of the growing sphere");
        assert_eq!(fireballs, 15);
        assert_eq!(flashes, 3);
        assert_eq!(svc.camera_shakes.len(), 1);
        assert!(table.mobys.iter().all(|m| m.o_class != FIREBALL_CLASS || m.state >= 0xfd), "fireballs outlived their timers");
    }
}
