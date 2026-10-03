//! **The electrified water, class 655** (level15 `0x2d6810`; census U546, one placed on Quartu and one on the Fleet,
//! level17 `0x2d4e88`, the same code). The current in a stretch of water: while it is on, Ratchet in the water
//! (12 cuboids) is shocked over and over, and when he is not, sparks jump between the points of the line nearest
//! him; switches (floor switches 830 or the switches 1179) turn it off for a while, a countdown on screen. Read from
//! the level15 decomp and disassembly; native `f32`.
//!
//! **Pvars**: +0x00..+0x0c the four switches (moby indices, −1 none), +0x10 / +0x12 (s16) the on and off timers,
//! +0x14 / +0x80 the hum's two voices, +0x18 the off time (seconds), +0x1c the shocks counted (the fog saved at the
//! first), +0x20..+0x4c the 12 water cuboids, +0x50..+0x5c / +0x88 the saved fog, +0x60..+0x6c four water-level
//! cuboids, +0x70 / +0x128 the reset cuboids, +0x74..+0x7c three lamps, +0x84 the hum's moby, +0x8c the switch
//! re-arm flag, +0x90 / +0xb0 / +0xd0 three lines of up to 8 cuboids (their centres the spark points), +0xf0 / +0x100
//! / +0x110 the bolt's start, end and aim, +0x120 its point light, +0x124 the area cuboid.
//!
//! * **Out of the area cuboid** (any state but 0): the hum released, the light freed, nothing else.
//! * **0**: always updated; the switches reset (an 830's save bytes and bits cleared; a 1179 in state 2 back to 1,
//!   1.5 up, `0x2d95a0`); on `scale(2·60)`, off `ticks(200)` → 1.
//! * **1, on**: Ratchet in the water (`0x2d6770`: `0x2056e0`, or swimming (states 0x18 / 0x19) at or below the
//!   water surface under him) and in a water cuboid → **shocked**: the hum moved onto him (class 0x28f's sounds 0
//!   and 1), the level fog saved (the first time) and set white (near F `randf(128, 255)`), a bolt from his body
//!   point to a point 5..8 away in a random direction, a hit (damage 1, flags 0x10001, pushed along the class's yaw).
//!   Else a coin flip (`randi(1)`: always) → sparks: the line whose first point is nearest Ratchet, a random pair of
//!   its consecutive points, the bolt between them, a point light (radius 8, (1, 1, 0.5)) at the first. The on timer
//!   out → 2 (the fog given back after the tick's shock). The lamps flicker (`rand_range(0x40, 0x7f)`).
//! * **1 and 2**: the hum on the lamp nearest the camera (sound 0, flags 5; in 1 also sound 1); in 2 the off timer
//!   out → 1 (reset). A switch pressed (830 in state 2, or 1179 in state 2) → **3** for the off time (its sound 2).
//! * **3, off**: lamps steady 0x40, no light, no hum, the countdown on screen (`0x2d77c0`, Veldin's at (250, 350)).
//!   Standing (on the ground) on an 830 when re-armed tops the time up once it has run down a second. The time out,
//!   or Ratchet in reset cuboid +0x70 (on the ground) or +0x128 → the switches reset, → 1.
//! * **Every tick**: Ratchet in the water and in a water-level cuboid: his water level = its record's +0x38 (its
//!   centre's z).
//! * **The bolt** (`0x2d79e8` reset, `0x2d7bb0` build, draw `0x2d8710`): the Tesla Claw's chain
//!   ([`crate::hero::tesla`]): 20 points from the start toward the aim along the end's direction turned 10°, each step
//!   blending 15 % toward a random direction, waving (the same phases and sizes), pulled onto the aim over the last
//!   points; the second chain; four arcs (two at the start, two at the end); type-53 sparks at the start and the aim.
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2d6810` | 655 | [`update`] |
//! | `0x2d6770` | Ratchet in the water | [`in_water`] |
//! | `0x2d79e8` / `0x2d7bb0` | the bolt's reset and build | [`reset`], [`build`] |
//! | `0x2d8710` / `0x2d77c0` | the bolt's draw, the countdown | [`fx_quad_groups`], [`countdown_draw`] |
//! | `0x2d9570` / `0x2d95a0` | a 1179 pressed, reset | [`switch_pressed`], [`switch_reset`] |
//!
//! [L] The bolt's chain is one global set (as the game's); the gold flag of the colour reads the Tesla Claw's.

use super::{FxQuad, FxQuads};
use crate::hero::tesla::{self, Tesla, POINTS};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::blaster_shot::rotate;
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, fx, SPEED};
use crate::moby_update::services::{pv, HitTemplate, Services, World};
use crate::moby_update::story;
use crate::point_lights::PointLight;
use crate::ps2v::Pf;

pub const REFERENCE_LEVEL: u32 = 15;
pub const UPDATE_FN: u32 = 0x2d_6810;
pub const DRAW_FN: u32 = 0x2d_8710;
pub const COUNTDOWN_FN: u32 = 0x2d_77c0;
pub const CLASSES: [i16; 1] = [655];

mod o {
    pub const SWITCHES: usize = 0x00;
    pub const ON_T: usize = 0x10;
    pub const OFF_T: usize = 0x12;
    pub const VOICE0: usize = 0x14;
    pub const OFF_SECS: usize = 0x18;
    pub const SHOCKS: usize = 0x1c;
    pub const WATER: usize = 0x20;
    pub const FOG: usize = 0x50;
    pub const LEVELS: usize = 0x60;
    pub const RESET_A: usize = 0x70;
    pub const LAMPS: usize = 0x74;
    pub const VOICE1: usize = 0x80;
    pub const OWNER: usize = 0x84;
    pub const FOG_RGB: usize = 0x88;
    pub const ARMED: usize = 0x8c;
    pub const LINES: usize = 0x90;
    pub const START: usize = 0xf0;
    pub const END: usize = 0x100;
    pub const AIM: usize = 0x110;
    pub const LIGHT: usize = 0x120;
    pub const AREA: usize = 0x124;
    pub const RESET_B: usize = 0x128;
    pub const SIZE: usize = 0x12c;
}

/// gp−0x5038 / −0x5034: the on time (seconds) after a reset, the off time after a run.
const ON_SECS: f32 = 2.0;
const OFF_SECS: f32 = 0.0;
/// gp−0x5030: sparks switched off when ≠ 0 (0 here).
const NO_SPARKS: f32 = 0.0;
const FLOOR_SWITCH: i16 = 0x33e;
const SWITCH: i16 = 0x49b;
const SOUND_CLASS: i16 = 0x28f;
/// gp−0x4ff4 / −0x4ff0: the wave's amplitude at the start and the end.
const WAVE_AMP: [f32; 2] = [0.03, 0.03];
const COUNTDOWN_CENTRE: (i32, i32) = (250, 350);
const COUNTDOWN_RGBA: [u32; 2] = [0x80c0_c0c0, 0x8040_40ff];

/// The bolt (the game's globals 0x1d3750.. / 0x161cc0..): the Tesla Claw's chain record.
#[derive(Clone, Debug, Default)]
pub struct Bolt {
    pub t: Tesla,
    /// The camera the draw's frame part saw.
    pub eye: [f32; 3],
    pub gravity: [f32; 3],
}

fn bolt<'a>(w: &'a mut World<'_>) -> &'a mut Bolt { &mut w.svc.units.water_shock }
fn link(w: &World, v: i32) -> Option<MobyId> { usize::try_from(v).ok().filter(|&m| m < w.table.mobys.len()) }
fn v3(w: &World, id: MobyId, at: usize) -> [f32; 3] { [c::pf(w, id, at), c::pf(w, id, at + 4), c::pf(w, id, at + 8)] }
fn set3(w: &mut World, id: MobyId, at: usize, v: [f32; 3]) { for (k, x) in v.iter().enumerate() { c::set_pf(w, id, at + 4 * k, *x); } }
fn hero_in(w: &World, cub: i32) -> bool { w.in_cuboid(w.hero_point(), cub) }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn scale(a: [f32; 3], k: f32) -> [f32; 3] { a.map(|x| x * k) }
fn len(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
fn unit(a: [f32; 3], l: f32) -> [f32; 3] { let n = len(a); if n == 0.0 { a } else { scale(a, l / n) } }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn body(w: &World) -> [f32; 3] { let b = w.hero.body_point; [b[0].to_f32(), b[1].to_f32(), b[2].to_f32()] }

/// `0x2d9570(m)`: a switch 1179 in state 2.
pub fn switch_pressed(w: &World, m: MobyId) -> bool { w.m(m).o_class == SWITCH && w.m(m).state == 2 }

/// `0x2d95a0(m)`: a pressed 1179 back to 1, 1.5 up.
pub fn switch_reset(w: &mut World, m: MobyId) {
    if switch_pressed(w, m) {
        let mo = w.mm(m);
        mo.state = 1;
        mo.position[2] += 1.5;
    }
}

/// `0x2d6770`: Ratchet in the water (`0x2056e0`), or swimming (0x18 / 0x19) at or below the water surface under him.
pub fn in_water(w: &World) -> bool {
    if crate::help::hero_in_water(w.hero.state, w.hero.group) { return true; }
    if !(0x18..=0x19).contains(&w.hero.state) { return false; }
    let h = super::hero_pos(w);
    let p = [h[0], h[1], h[2] + 2.0, h[3]];
    let hit = w.coll_line(pv([p[0], p[1], p[2] + 0.5, p[3]]), pv([p[0], p[1], 0.01, p[3]]), 2, None);
    let (z, surface) = match hit {
        Some(o) => (o.point[2], if o.kind < 0 || o.kind & 0x1f == 0x1f { -1 } else { o.kind & 0x1f }),
        None => (0.0, -1),
    };
    surface == 0 && h[2] <= z
}

fn release(w: &mut World, id: MobyId) {
    let Some(owner) = usize::try_from(c::pi32(w, id, o::OWNER) - 1).ok() else { return };
    for v in [o::VOICE0, o::VOICE1] {
        let s = c::pi32(w, id, v);
        if s != -1 && w.sound_alive(s, owner) { w.release_sound(s, owner); }
        c::set_pi32(w, id, v, -1);
    }
}

fn free_light(w: &mut World, id: MobyId) {
    if let Ok(i) = usize::try_from(c::pi32(w, id, o::LIGHT)) { w.svc.point_lights.free(i); }
    c::set_pi32(w, id, o::LIGHT, -1);
}

fn on(w: &mut World, id: MobyId) {
    let t = w.svc.timing.scale(Pf::f(ON_SECS * 60.0)).to_i32();
    c::set_pi16(w, id, o::ON_T, t as i16);
    let t = w.ticks(200);
    c::set_pi16(w, id, o::OFF_T, t as i16);
    c::set_pi32(w, id, o::SHOCKS, 0);
    w.mm(id).state = 1;
}

/// The switches back up (state 0 and the reset): an 830's save bytes and bits cleared (state 0) or its state 1 (the
/// reset), a 1179 reset.
fn reset_switches(w: &mut World, id: MobyId, clear: bool) {
    for k in 0..4 {
        let Some(m) = link(w, c::pi32(w, id, o::SWITCHES + 4 * k)) else { continue };
        if w.m(m).o_class == FLOOR_SWITCH {
            if clear {
                let (lvl, sid) = (w.svc.level, w.m(m).spawn_id);
                w.svc.save.killed.remove(&sid);
                w.svc.save.collected.remove(&sid);
                w.svc.save.death.remove(&(lvl, sid));
                w.svc.save.death_level.remove(&sid);
            } else {
                w.mm(m).state = 1;
            }
        } else {
            switch_reset(w, m);
        }
    }
}

/// Level15 `0x2d6810` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, o::SIZE);
    if w.m(id).state != 0 && !hero_in(w, c::pi32(w, id, o::AREA)) {
        if c::pi32(w, id, o::OWNER) != 0 { release(w, id); }
        if c::pi32(w, id, o::LIGHT) != -1 { free_light(w, id); }
        return;
    }
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            reset_switches(w, id, true);
            on(w, id);
            c::set_pi32(w, id, o::LIGHT, -1);
            c::set_pi32(w, id, o::VOICE1, -1);
            c::set_pi32(w, id, o::VOICE0, -1);
        }
        1 => {
            active(w, id);
            sounds(w, id);
        }
        2 => sounds(w, id),
        3 => off(w, id),
        _ => {}
    }
    for k in 0..4 {
        let cub = c::pi32(w, id, o::LEVELS + 4 * k);
        if hero_in(w, cub) && crate::help::hero_in_water(w.hero.state, w.hero.group) {
            if let Some(s) = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, cub) {
                let z = s.matrix[3][2];
                w.hero_fields_mut().water_level = z;
            }
        }
    }
}

/// State 1's own part (module doc).
fn active(w: &mut World, id: MobyId) {
    let wet = in_water(w);
    if c::dec_timer_pvar_s16(w, id, o::ON_T) != 0 {
        let t = w.svc.timing.scale(Pf::f(OFF_SECS * 60.0)).to_i32();
        c::set_pi16(w, id, o::OFF_T, t as i16);
        w.mm(id).state = 2;
        if let Ok(owner) = usize::try_from(c::pi32(w, id, o::OWNER) - 1) {
            let s = c::pi32(w, id, o::VOICE1);
            if s != -1 && w.sound_alive(s, owner) { w.release_sound(s, owner); }
            c::set_pi32(w, id, o::VOICE1, -1);
        }
    }
    let inside = (0..12).any(|k| hero_in(w, c::pi32(w, id, o::WATER + 4 * k)));
    let lit = w.m(id).state != 2;
    for k in 0..3 {
        let v = if lit { w.rng.rand_range(0x40, 0x7f) as u8 } else { 0x40 };
        if let Some(m) = link(w, c::pi32(w, id, o::LAMPS + 4 * k)) { w.mm(m).ambient[..3].copy_from_slice(&[v; 3]); }
    }
    if (!inside || !wet) && w.rng.randi(1) == 0 && NO_SPARKS == 0.0 { sparks(w, id); }
    if inside && wet { shock(w, id); }
}

/// The sparks between the line points nearest Ratchet (module doc).
fn sparks(w: &mut World, id: MobyId) {
    let h = super::hero_pos(w);
    let centre = |w: &World, cub: i32| -> Option<[f32; 3]> {
        w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, cub).map(|s| [s.matrix[3][0], s.matrix[3][1], s.matrix[3][2]])
    };
    let dist = |p: [f32; 3]| len(sub(p, [h[0], h[1], h[2]]));
    let mut best: Option<usize> = None;
    for line in 0..3 {
        let first = c::pi32(w, id, o::LINES + 0x20 * line);
        if first == -1 { continue; }
        if let Some(b) = best {
            let (a, n) = (centre(w, c::pi32(w, id, o::LINES + 0x20 * b)), centre(w, first));
            if let (Some(a), Some(n)) = (a, n) { if dist(a) <= dist(n) { continue; } }
        }
        best = Some(line);
    }
    let Some(line) = best else { return };
    let at = |k: usize| o::LINES + 0x20 * line + 4 * k;
    let n = (1..8).find(|&k| c::pi32(w, id, at(k)) == -1).unwrap_or(8);
    if n < 2 { return; }
    let k = w.rng.rand_range(0, n as i32 - 2) as usize;
    let (Some(a), Some(b)) = (centre(w, c::pi32(w, id, at(k))), centre(w, c::pi32(w, id, at(k + 1)))) else { return };
    set3(w, id, o::START, a);
    set3(w, id, o::AIM, b);
    set3(w, id, o::END, b);
    let l = PointLight { color: [1.0, 1.0, 0.5], intensity: 0.0, pos: a, radius: 8.0 };
    match usize::try_from(c::pi32(w, id, o::LIGHT)) {
        Ok(i) => w.svc.point_lights.set(i, l),
        Err(_) => {
            let load = f32::from_bits(w.svc.frame_load[1].0);
            let got = w.svc.point_lights.alloc(l, load).map_or(-1, |i| i as i32);
            c::set_pi32(w, id, o::LIGHT, got);
        }
    }
    reset(w, id);
    build(w, id);
    register(w, id, DRAW_FN);
}

/// Ratchet shocked (module doc).
fn shock(w: &mut World, id: MobyId) {
    let Some(hero) = w.hero_moby else { return };
    if c::pi32(w, id, o::OWNER) != 0 && c::pi32(w, id, o::OWNER) - 1 != hero as i32 { release(w, id); }
    c::set_pi32(w, id, o::OWNER, hero as i32 + 1);
    let v = w.play_sound_as(0, 4, hero, SOUND_CLASS);
    c::set_pi32(w, id, o::VOICE0, v);
    let v = w.play_sound_as(1, 4, hero, SOUND_CLASS);
    c::set_pi32(w, id, o::VOICE1, v);
    if c::pi32(w, id, o::SHOCKS) == 0 {
        if let Some(g) = w.svc.water.fog {
            c::set_pf(w, id, o::FOG, g.near_dist);
            c::set_pf(w, id, o::FOG + 4, g.far_dist);
            c::set_pf(w, id, o::FOG + 8, g.near_intensity);
            c::set_pf(w, id, o::FOG + 0xc, g.far_intensity);
            for k in 0..3 { c::set_pu8(w, id, o::FOG_RGB + k, g.color[k]); }
        }
    }
    let n = c::pi32(w, id, o::SHOCKS);
    c::set_pi32(w, id, o::SHOCKS, n + 1);
    let near = w.rng.randf(128.0, 255.0);
    let t = w.counter;
    w.svc.water.store_fog(t, crate::fog_zones::FogGlobals { color: [0x7f; 3], near_dist: 0.0, far_dist: 131_072.0, near_intensity: near, far_intensity: 0.0 });
    let r = w.rng.randf(5.0, 8.0);
    let (a, b) = (w.rng.rand_angle(), w.rng.rand_angle());
    let d = fx::polar(r, a, b);
    let b0 = body(w);
    let tgt = add([d[0], d[1], d[2]], b0);
    set3(w, id, o::AIM, tgt);
    set3(w, id, o::END, tgt);
    set3(w, id, o::START, b0);
    reset(w, id);
    build(w, id);
    register(w, id, DRAW_FN);
    let yaw = w.m(id).rotation[2];
    let dir = unit([yaw.cos() * 0.2, yaw.sin() * 0.2, 0.0], 1.0);
    let tmpl = HitTemplate { dir: [Pf::f(dir[0]), Pf::f(dir[1]), Pf::ONE, Pf::b(0x45af_df66)], attacker: Some(id), flags: 0x1_0001, b18: 0, b19: 1, h1a: w.m(id).o_class as u16, damage: Pf::ONE, w20: 0 };
    w.deliver_hit(hero, &tmpl);
    if w.m(id).state == 2 { restore_fog(w, id); }
}

fn restore_fog(w: &mut World, id: MobyId) {
    let g = crate::fog_zones::FogGlobals {
        color: [c::pu8(w, id, o::FOG_RGB), c::pu8(w, id, o::FOG_RGB + 1), c::pu8(w, id, o::FOG_RGB + 2)],
        near_dist: c::pf(w, id, o::FOG),
        far_dist: c::pf(w, id, o::FOG + 4),
        near_intensity: c::pf(w, id, o::FOG + 8),
        far_intensity: c::pf(w, id, o::FOG + 0xc),
    };
    let t = w.counter;
    w.svc.water.store_fog(t, g);
}

/// States 1 and 2: the hum on the lamp nearest the camera, the off timer, the switches (module doc).
fn sounds(w: &mut World, id: MobyId) {
    let cam = crate::hero::physics::to_f32x3(w.camera);
    let mut near: Option<MobyId> = None;
    for k in 0..3 {
        let Some(m) = link(w, c::pi32(w, id, o::LAMPS + 4 * k)) else { continue };
        if let Some(n) = near {
            let d = |x: MobyId| { let p = w.m(x).position; len(sub([p[0], p[1], p[2]], cam)) };
            if d(n) <= d(m) { continue; }
        }
        near = Some(m);
    }
    let owner = c::pi32(w, id, o::OWNER) - 1;
    let pick = match (owner >= 0, near) {
        (false, n) => n,
        (true, Some(n)) => {
            let d = |x: MobyId| { let p = w.m(x).position; len(sub([p[0], p[1], p[2]], cam)) };
            if d(n) < d(owner as usize) { release(w, id); Some(n) } else { Some(owner as usize) }
        }
        (true, None) => Some(owner as usize),
    };
    c::set_pi32(w, id, o::OWNER, pick.map_or(0, |m| m as i32 + 1));
    if let Some(m) = pick {
        if !w.sound_alive(c::pi32(w, id, o::VOICE0), m) {
            let v = w.play_sound_as(0, 5, m, SOUND_CLASS);
            c::set_pi32(w, id, o::VOICE0, v);
        }
        if w.m(id).state == 1 && !w.sound_alive(c::pi32(w, id, o::VOICE1), m) {
            let v = w.play_sound_as(1, 5, m, SOUND_CLASS);
            c::set_pi32(w, id, o::VOICE1, v);
        }
    }
    if c::dec_timer_pvar_s16(w, id, o::OFF_T) != 0 && w.m(id).state == 2 { on(w, id); }
    for k in 0..4 {
        let Some(m) = link(w, c::pi32(w, id, o::SWITCHES + 4 * k)) else { continue };
        let pressed = (w.m(m).o_class == FLOOR_SWITCH && w.m(m).state == 2) || switch_pressed(w, m);
        if !pressed { continue; }
        let t = w.svc.timing.scale(Pf::f(c::pf(w, id, o::OFF_SECS) * 60.0)).to_i32();
        c::set_pi16(w, id, o::OFF_T, t as i16);
        w.mm(id).state = 3;
        w.play_sound_as(2, 0, m, SOUND_CLASS);
        c::set_pi32(w, id, o::ARMED, 0);
    }
}

/// State 3 (module doc).
fn off(w: &mut World, id: MobyId) {
    for k in 0..3 {
        if let Some(m) = link(w, c::pi32(w, id, o::LAMPS + 4 * k)) { w.mm(m).ambient[..3].copy_from_slice(&[0x40; 3]); }
    }
    if c::pi32(w, id, o::LIGHT) != -1 { free_light(w, id); }
    if c::pi16(w, id, o::OFF_T) != 0 { register(w, id, COUNTDOWN_FN); }
    if c::pi32(w, id, o::OWNER) != 0 {
        release(w, id);
        c::set_pi32(w, id, o::OWNER, 0);
    }
    let mut armed = 1;
    for k in 0..4 {
        let Some(m) = link(w, c::pi32(w, id, o::SWITCHES + 4 * k)) else { continue };
        if w.hero.ground_moby != Some(m) || w.m(m).o_class != FLOOR_SWITCH || w.hero.air_ticks != 0 { continue; }
        armed = 0;
        if c::pi32(w, id, o::ARMED) == 0 { continue; }
        let full = w.svc.timing.scale(Pf::f(c::pf(w, id, o::OFF_SECS) * 60.0)).to_i32();
        if (c::pi16(w, id, o::OFF_T) as i32) < full - w.ticks(0x3c) {
            c::set_pi16(w, id, o::OFF_T, full as i16);
            w.play_sound_as(2, 0, m, SOUND_CLASS);
        }
    }
    c::set_pi32(w, id, o::ARMED, armed);
    let out = c::dec_timer_pvar_s16(w, id, o::OFF_T) != 0;
    let back = out || {
        let a = hero_in(w, c::pi32(w, id, o::RESET_A));
        (a && w.hero.air_ticks == 0) || hero_in(w, c::pi32(w, id, o::RESET_B))
    };
    if back {
        reset_switches(w, id, false);
        on(w, id);
    }
}

fn register(w: &mut World, id: MobyId, f: u32) {
    if let Some(row) = super::row(REFERENCE_LEVEL, f) {
        w.svc.draw_callbacks.register(Callback::UnitFrame(row), id);
        if f == DRAW_FN { w.svc.draw_callbacks.register(Callback::UnitQuads(row), id); }
    }
}

/// `0x2d79e8`: the chain set up on the first bolt (straight from the start toward the end in 20 steps of 15/20).
pub fn reset(w: &mut World, id: MobyId) {
    let (s, e) = (v3(w, id, o::START), v3(w, id, o::END));
    let b = bolt(w);
    b.t.start = s;
    b.t.end = e;
    if !b.t.chain.reset { return; }
    b.t.chain.reset = false;
    b.t.chain.scroll = 0.0;
    let step = unit(sub(e, s), 15.0 / 20.0);
    b.t.chain.main[0] = s;
    for i in 0..POINTS - 1 { b.t.chain.main[i + 1] = add(b.t.chain.main[i], step); }
    b.t.chain.zoff = [0.0; POINTS + 1];
    b.t.chain.side = [[0.0; 3]; POINTS];
    b.t.chain.zoff2 = [0.0; 22];
    b.t.chain.arc_timer = [-1; 4];
    b.t.chain.phase = [0.0; 3];
}

/// `0x2d7bb0`: the chain, the arcs and the sparks (module doc).
#[allow(clippy::too_many_lines)]
pub fn build(w: &mut World, id: MobyId) {
    let gold = w.hero.weapons.gold[3] != 0;
    let (start, end, aim) = (v3(w, id, o::START), v3(w, id, o::END), v3(w, id, o::AIM));
    let grav = crate::hero::physics::to_f32x3(w.hero.gravity_dir);
    let cam = crate::hero::physics::to_f32x3(w.camera);
    let mut seg = len(sub(start, aim)) * 0.05;
    let mut ch = std::mem::take(&mut bolt(w).t.chain);
    ch.phase[0] = c::add_rot(ch.phase[0], f32::from_bits(0x3eb2_b8c2));
    ch.phase[1] = c::sub_rot(ch.phase[1], f32::from_bits(0x3f4d_87ac));
    ch.phase[2] = c::add_rot(ch.phase[2], f32::from_bits(0x3d56_7770));
    ch.color = if gold { 0x7f_2040 } else { 0x7f_2020 };
    let mut dirn = unit(sub(end, start), 1.0);
    let side = cross(grav, dirn);
    dirn = rotate(dirn, f32::from_bits(0x3e32_b8c2), side);
    let mut step = scale(dirn, seg);
    ch.main[0] = start;
    ch.second[0] = start;
    for i in 1..POINTS {
        let fi = i as f32;
        let amp = WAVE_AMP[0] + (WAVE_AMP[1] - WAVE_AMP[0]) * fi * 0.05;
        let wave = amp * c::add_rot(ch.phase[0] + fi * f32::from_bits(0x3c8e_fa35), 0.0).sin() + w.rng.randf_sym(0.0, 0.1);
        let old = ch.zoff[i];
        ch.zoff[i] = wave;
        ch.main[i][2] -= old;
        let (a, b) = (w.rng.rand_angle(), w.rng.rand_angle());
        let mut r = fx::polar(seg, a, b);
        r[2] *= 0.5;
        let d: [f32; 3] = std::array::from_fn(|k| step[k] + (r[k] - step[k]) * f32::from_bits(0x3e19_999a));
        let l = len(d);
        if l == 0.0 {
            ch.main[i] = ch.main[i - 1];
        } else {
            let d = scale(d, seg / l);
            ch.main[i] = add(ch.main[i - 1], d);
            step = d;
        }
        ch.main[i][2] += wave;
        let rel = sub(ch.main[i], ch.main[i - 1]);
        let to = sub(aim, ch.main[i]);
        let l = len(to);
        if l != 0.0 {
            let k = if i == 19 { seg = l; 0.5 } else if i < 16 { 0.1 } else { seg = l / (19 - i) as f32; 0.5 };
            let to = scale(to, seg / l);
            step = add(step, scale(sub(to, step), k));
        }
        if i < 10 {
            let p2 = c::add_rot(ch.phase[1] + fi * f32::from_bits(0x3f7e_adae), 0.0);
            let p3 = c::add_rot(ch.phase[2] + fi * f32::from_bits(0x3e56_7750), 0.0);
            let s3 = 0.8 * p3.sin();
            ch.second[i] = ch.main[i];
            let z2 = s3 * p2.sin() + w.rng.randf_sym(0.0, 0.1);
            ch.zoff2[i] = z2;
            ch.second[i][2] += z2;
            let ang = c::add_rot(c::atan(rel[0], rel[1]), std::f32::consts::FRAC_PI_2);
            let rr = 0.5 * p2.cos();
            ch.side[i - 1] = [ang.cos() * rr, ang.sin() * rr, 0.0];
            ch.second[i] = add(ch.second[i], ch.side[i - 1]);
        } else {
            ch.second[i] = ch.main[i];
            ch.second[i][2] += ch.zoff2[21 - i];
            ch.second[i] = add(ch.second[i], ch.side[20 - i]);
        }
    }
    let side_n = unit(cross(grav, dirn), 1.0);
    let up = unit(grav, 1.0);
    for k in 0..4 {
        let (base, al) = if k < 2 { (start, 0.4) } else { (end, 0.5) };
        let mut t = ch.arc_timer[k];
        let fired = crate::hero::guns::dec16(&mut t);
        ch.arc_timer[k] = t;
        if !fired {
            let mut d = [[0.0f32; 3]; 4];
            for (j, dj) in d.iter_mut().enumerate() {
                let r = sub(ch.arcs[k][j + 1], ch.arcs[k][j]);
                *dj = [r[0] + w.rng.randf_sym(0.0, 0.1), r[1] + w.rng.randf_sym(0.0, 0.1), r[2] + w.rng.randf_sym(0.0, 0.1)];
            }
            ch.arcs[k][0] = base;
            for (j, dj) in d.iter().enumerate() { ch.arcs[k][j + 1] = add(ch.arcs[k][j], *dj); }
            let tv = ch.arc_timer[k] as i32;
            let dd = (tv - [4, 2, 4, 2][k]).abs() as f32;
            ch.arc_alpha[k] = ((1.0 - dd / w.ticks(15) as f32) * 32.0) as i32 as i16;
        } else {
            ch.arc_timer[k] = w.ticks([8, 4, 8, 4][k]) as i16;
            ch.arc_alpha[k] = 0;
            let a = w.rng.randf_sym(f32::from_bits(0x3f06_0a92), f32::from_bits(0x3f86_0a92));
            let b = w.rng.randf(-f32::from_bits(0x3f86_0a92), f32::from_bits(0x3e32_b8c2));
            let mut v = scale(dirn, al);
            v = rotate(v, b, side_n);
            v = rotate(v, a, up);
            ch.arcs[k][0] = base;
            ch.arcs[k][1] = add(base, v);
            let mut sign = 1.0f32;
            for j in 0..3 {
                let cc = sub(ch.arcs[k][j + 1], cam);
                let ang = w.rng.randf(f32::from_bits(0x3e32_b8c2), f32::from_bits(0x3f75_be0b)) * sign;
                sign = -sign;
                v = rotate(v, ang, cc);
                ch.arcs[k][j + 2] = add(ch.arcs[k][j + 1], v);
            }
        }
    }
    let col = ch.color | 0x7f00_0000;
    bolt(w).t.chain = ch;
    let a = w.rng.randf_sym(f32::from_bits(0x3f06_0a92), f32::from_bits(0x3f86_0a92));
    let b = w.rng.randf(-f32::from_bits(0x3f06_0a92), f32::from_bits(0x3f86_0a92));
    let mut v = scale(dirn, 0.09 * SPEED);
    v = rotate(v, b, side_n);
    v = rotate(v, a, up);
    let mut spin = w.rng.rand_range(1, 8);
    if w.rng.randi(2) != 0 { spin = -spin; }
    let f = w.rng.randf(0.2, 0.6);
    let life = w.ticks(15);
    let vel = pv([v[0], v[1], v[2], 0.0]);
    let sp = pv([start[0], start[1], start[2], 0.0]);
    w.part53(Pf::f(f * 0.3), Pf::f(f), Pf::ZERO, sp, life, col, 0, spin as i8, vel);
    w.part53(Pf::f(f * 0.15), Pf::f(f * 0.5), Pf::ZERO, sp, life, 0x307f_7f7f, 0, -spin as i8, vel);
    let mut spin = w.rng.rand_range(1, 8);
    if w.rng.randi(2) != 0 { spin = -spin; }
    let f = w.rng.randf(0.15, 3.0);
    let life = w.ticks(12);
    w.part53(Pf::f(f * 0.1), Pf::f(f), Pf::ZERO, pv([aim[0], aim[1], aim[2], 0.0]), life, col, 0, spin as i8, pv([0.0; 4]));
}

/// The draws' frame part: the camera kept; the bolt's draw steps its scroll; the countdown's text.
pub fn frame(w: &mut World, id: MobyId) {
    if w.m(id).state == 3 {
        if w.m(id).pvars.len() >= o::SIZE {
            let n = c::pi16(w, id, o::OFF_T) as i32;
            super::veldin_pads::push_countdown(w, n, COUNTDOWN_CENTRE, COUNTDOWN_RGBA);
        }
        return;
    }
    let eye = crate::hero::physics::to_f32x3(w.camera);
    let g = crate::hero::physics::to_f32x3(w.hero.gravity_dir);
    let b = bolt(w);
    b.eye = eye;
    b.gravity = g;
    b.t.count = POINTS as i16;
    let s = &mut b.t.chain.scroll;
    *s -= 0.3 * SPEED;
    if *s <= -8.0 { *s += 8.0; }
}

/// `0x2d77c0`: the countdown (pushed by [`frame`] in state 3).
pub fn countdown_draw(w: &mut World, id: MobyId) { frame(w, id) }

/// Level15 `0x2d8710`: the bolt as the Tesla Claw's strips and glow (additive).
pub fn fx_quad_groups(_table: &MobyTable, svc: &Services, _id: MobyId) -> Vec<FxQuads> {
    let b = &svc.units.water_shock;
    let mut groups: Vec<FxQuads> = Vec::new();
    for q in tesla::beam_quads(&b.t, b.eye, b.gravity) {
        let quad = FxQuad { corners: q.corners, st: q.st, rgba: q.rgba };
        match groups.iter_mut().find(|g| g.fx == q.fx) {
            Some(g) => g.quads.push(quad),
            None => groups.push(FxQuads { fx: q.fx, additive: true, subtract: false, quads: vec![quad] }),
        }
    }
    groups
}
