//! **Kalebo's cars, class 1410** (level16 `0x2e43e0` with its helpers `0x2e4900`..`0x2e4f30`, 2 placed in group 43;
//! census U558; the name is descriptive [L]). Two hovering cars shuttle between three stations (the cuboids at pvar
//! +0x80..0x88); a car shows once item 0x21 is owned, or at once at station 2. Standing on a waiting car (within 1.2
//! of its middle across), the prompt names where it goes (station 1 → 0, 0 → 1, 2 → 1) and △ sends it: the other car,
//! the one already at the destination, hides; the car eases from its station onto its path, rides it, and eases into
//! the destination station, where the hidden car reappears at the station it left. While riding, the car hums, holds
//! Ratchet's jump and edge brake, and keeps him on through the spline wall (its spline +0xa0 rewritten each tick to
//! seven points round the car). It bobs and sways all the time and carries Ratchet with every move. Read from the
//! level16 decomp and its table 0x1d96e0.
//!
//! **Pvars**: +0x08 the platform block's offset (0x20), +0x60 the placed position and +0x70 the Euler (the bob is added
//! to the moby's), +0x80 the station cuboids (3), +0x90 / +0x94 / +0x9c the paths from stations 0 / 1 / 2, +0xa0 the
//! wall spline, +0xa4 the path being ridden (a pointer in the game; the spline index here), +0xaa s16 the hum's
//! voice, +0xac s16 the station, +0xae s16 the destination, +0xb0 / +0xb4 the ride's t and its velocity, +0xb8 /
//! +0xbc / +0xc0 the bob's phases, +0xc4 / +0xc8 the path's top speed and acceleration.
//!
//! | address | what | port |
//! |---|---|---|
//! | every tick first | the old position and Euler kept; `0x261c98(m, shown)` (L01 `0x27b438`: the talked word) | [`update`] (`interact::set_talked`) |
//! | state 0 | voice −1; `0x2e4900`: the station = the cuboid within 1 (`vec_distance`) of the position; item 0x21 (0x13d4e1) not owned and not station 2 → 6, no collision, hidden; else → 1, the class collision, shown; +0x60 / +0x70 = that cuboid's centre / Euler | [`init`] |
//! | state 1 | Ratchet on it, grounded, \|(Ratchet − position)·rows\|.y < 1.2: `0x2e4a58` (the path: station 1 → +0x94, destination 0; 0 → +0x90, 1; 2 → +0x9c, 1; t = v = 0; acceleration 5·dt²/(d·n), top speed 20·dt/(d·n), d = the path's first segment, n its points); `0x2e4cd8` (`try_set_help_message(8, msg)` by destination: 0x3e89, 0x3e8a, 0x139a, 0x139c, 0x1399, else 0x139b); △ (0x13cae4 & 0x10) with the prompt owner 8 → 3, the car at the destination (`0x2e4be0`: the group 43 car whose station is it) → 2, no collision, hidden | [`update`] |
//! | state 3 | `0x257268(1, dt², dt², dt/2, &t, &v)`; +0x60 = lerp(the station's centre, the path's start, t), yaw / roll toward the start's by t (`fast_subtract_rotations`, `fast_add_rotations`); t ≥ 1 → t = v = 0, → 4 | [`update`] |
//! | state 4 | `0x2e4c40`: the spring with the path's speeds; +0x60 / +0x70 = the path at t·(n − 1) (`0x25e338` = L01 `0x277d40`); t ≥ 1 → t = v = 0, → 5 | [`update`] (`path::pose`) |
//! | state 5 | as 3 from the path's end to the destination's centre and Euler; t ≥ 1 → t = v = 0, → 1, `0x2e4b58`: the hidden car put at this car's station (its centre and Euler) and re-placed (`0x2e4900`), the station = the destination | [`update`] |
//! | state 6 | item 0x21 owned → 1, the class collision, shown | [`update`] |
//! | every tick | `0x2e4d88`: position / Euler = +0x60 / +0x70; +0xb8 += 120°·dt, z += 0.25·sin; +0xbc += 97°·dt, pitch += 5°·sin; +0xc0 += 53°·dt, roll += 5°·sin; roll and pitch clamped to ±10° | [`bob`] |
//! | states 3..5 | 0x13f544 = 0x13f542 = 2; `0x2e4f30`: the wall spline +0xa0 (of 7 points) = the table's points through the rows plus the position, 0x14162a = it; no hum → +0xaa = `PlayClassSound(0, 4)` | [`update`] (`HeroFields::jump_lockout` / `edge_brake` / `wall_spline`) |
//! | other states | the hum alive → released (owner-guarded), +0xaa = −1 | [`update`] |
//! | every tick last | `CarryRiders(+0x20, position − old, old Euler, Euler)` | [`update`] (`triggers::carry_riders`) |

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::{self as c, turn, DT, DT2};
use crate::moby_update::services::World;
use crate::moby_update::triggers;

pub const REFERENCE_LEVEL: u32 = 16;
pub const UPDATE_FN: u32 = 0x2e_43e0;
pub const CLASSES: [i16; 1] = [1410];
/// The item that brings the cars out (0x13d4c0 + 0x21).
pub const ITEM: usize = 0x21;
/// The prompt's owner.
pub const OWNER: i32 = 8;
/// The prompts by destination (`0x2e4cd8`).
pub const PROMPTS: [i32; 5] = [0x3e89, 0x3e8a, 0x139a, 0x139c, 0x1399];
pub const PROMPT_OTHER: i32 = 0x139b;
const TRIANGLE: u32 = 0x10;
/// The wall's points round the car (0x1d96e0: x, y, z, w = 1).
const WALL: [[f32; 3]; 7] = [[1.3, 1.3, 0.0], [1.3, -1.3, 0.0], [0.0, -1.65, 0.0], [-1.3, -1.3, 0.0], [-1.3, 1.3, 0.0], [0.0, 1.65, 0.0], [1.3, 1.3, 0.0]];
const TEN_DEG: f32 = 0.174_532_92;

/// Pvar offsets (module doc).
pub mod pv {
    pub const BLOCK: usize = 0x20;
    pub const POS: usize = 0x60;
    pub const ROT: usize = 0x70;
    pub const STATIONS: usize = 0x80;
    pub const PATHS: [usize; 3] = [0x90, 0x94, 0x9c];
    pub const WALL: usize = 0xa0;
    pub const PATH: usize = 0xa4;
    pub const VOICE: usize = 0xaa;
    pub const STATION: usize = 0xac;
    pub const DEST: usize = 0xae;
    pub const T: usize = 0xb0;
    pub const V: usize = 0xb4;
    pub const BOB: usize = 0xb8;
    pub const VMAX: usize = 0xc4;
    pub const ACCEL: usize = 0xc8;
    pub const SIZE: usize = 0xcc;
}

fn station_shape(w: &World, id: MobyId, k: i16) -> Option<rc_formats::volumes::Shape> {
    if !(0..3).contains(&k) { return None; }
    w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c::pi32(w, id, pv::STATIONS + 4 * k as usize)).copied()
}

fn path_points(w: &World, id: MobyId) -> Option<Vec<crate::path::Point>> {
    usize::try_from(c::pi32(w, id, pv::PATH)).ok().and_then(|i| w.svc.splines.get(i)).cloned().filter(|p| !p.is_empty())
}

/// `0x2e4900`: the station, shown or not, the placed pose (module doc).
pub fn init(w: &mut World, id: MobyId) {
    c::set_pi16(w, id, pv::STATION, -1);
    let p = w.m(id).position;
    for k in 0..3i16 {
        let Some(s) = station_shape(w, id, k) else { continue };
        let c3 = s.centre();
        if ((p[0] - c3[0]).powi(2) + (p[1] - c3[1]).powi(2) + (p[2] - c3[2]).powi(2)).sqrt() < 1.0 { c::set_pi16(w, id, pv::STATION, k); }
    }
    let station = c::pi16(w, id, pv::STATION);
    if !super::hints::owned(w, ITEM) && station != 2 {
        let m = w.mm(id);
        m.state = 6;
        m.has_collision = false;
        m.mode |= 1;
        m.visible = 0;
    } else {
        show(w, id);
    }
    if let Some(s) = station_shape(w, id, station) {
        c::set_pv4(w, id, pv::POS, s.matrix[3]);
        c::set_pv4(w, id, pv::ROT, [s.euler[0], s.euler[1], s.euler[2], s.unused_7c]);
    }
}

fn show(w: &mut World, id: MobyId) {
    let coll = super::class_collision(w, w.m(id).o_class);
    let m = w.mm(id);
    m.state = 1;
    m.has_collision = coll;
    m.mode &= !1;
    m.visible = 1;
}

/// `0x2e4a58`: the path from this station and its speeds.
fn choose_path(w: &mut World, id: MobyId) {
    let (path, dest) = match c::pi16(w, id, pv::STATION) {
        1 => (Some(c::pi32(w, id, pv::PATHS[1])), 0),
        0 => (Some(c::pi32(w, id, pv::PATHS[0])), 1),
        2 => (Some(c::pi32(w, id, pv::PATHS[2])), 1),
        _ => (None, c::pi16(w, id, pv::DEST)),
    };
    if let Some(p) = path {
        c::set_pi32(w, id, pv::PATH, p);
        c::set_pi16(w, id, pv::DEST, dest);
    }
    c::set_pf(w, id, pv::V, 0.0);
    c::set_pf(w, id, pv::T, 0.0);
    let Some(pts) = path_points(w, id) else { return };
    let pt = |k: usize| pts.get(k).copied().unwrap_or_default().map(f32::from_bits);
    let (a, b) = (pt(0), pt(1));
    let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let inv = 1.0 / (d * pts.len() as f32);
    c::set_pf(w, id, pv::ACCEL, DT2 * 5.0 * inv);
    c::set_pf(w, id, pv::VMAX, DT * 20.0 * inv);
}

/// `0x2e4be0(m, station)`: the car of this car's group at `station`.
fn car_at(w: &World, id: MobyId, station: i16) -> Option<MobyId> {
    let g = usize::from(w.m(id).group as u8);
    let list = w.svc.groups.lists.get(g).cloned().flatten()?;
    list.into_iter().map(usize::from).find(|&k| k < w.table.mobys.len() && w.m(k).pvars.len() >= pv::SIZE && c::pi16(w, k, pv::STATION) == station)
}

/// The ease of states 3 / 5: position `a → b` and yaw / roll `ea → eb` by the spring's t; true when it ends.
fn ease(w: &mut World, id: MobyId, a: [f32; 4], ea: [f32; 4], b: [f32; 4], eb: [f32; 4]) -> bool {
    let (mut t, mut v) = (c::pf(w, id, pv::T), c::pf(w, id, pv::V));
    turn::spring(1.0, DT2, DT2, DT * 0.5, &mut t, &mut v);
    c::set_pf(w, id, pv::T, t);
    c::set_pf(w, id, pv::V, v);
    c::set_pv4(w, id, pv::POS, std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t));
    for k in [1usize, 2] {
        let e = c::add_rot(c::sub_rot(eb[k], ea[k]) * t, ea[k]);
        c::set_pf(w, id, pv::ROT + 4 * k, e);
    }
    if 1.0 <= t {
        c::set_pf(w, id, pv::V, 0.0);
        c::set_pf(w, id, pv::T, 0.0);
        return true;
    }
    false
}

/// `0x2e4d88`: the placed pose plus the bob (module doc).
pub fn bob(w: &mut World, id: MobyId) {
    let (pos, rot) = (c::pv4(w, id, pv::POS), c::pv4(w, id, pv::ROT));
    let ph: [f32; 3] = std::array::from_fn(|k| c::pf(w, id, pv::BOB + 4 * k));
    let p0 = c::add_rot(ph[0], DT * 2.094_395_2);
    let p1 = c::add_rot(ph[1], DT * 1.692_969_3);
    let p2 = c::add_rot(ph[2], DT * 0.925_024_5);
    for (k, p) in [p0, p1, p2].into_iter().enumerate() { c::set_pf(w, id, pv::BOB + 4 * k, p); }
    let m = w.mm(id);
    m.position = pos;
    m.rotation = rot;
    m.position[2] += p0.sin() * 0.25;
    m.rotation[1] = c::add_rot(m.rotation[1], p1.sin() * 0.087_266_46);
    m.rotation[0] = c::add_rot(m.rotation[0], p2.sin() * 0.087_266_46);
    m.rotation[0] = m.rotation[0].clamp(-TEN_DEG, TEN_DEG);
    m.rotation[1] = m.rotation[1].clamp(-TEN_DEG, TEN_DEG);
}

/// `0x2e4f30`: the wall spline rewritten round the car, and named.
fn wall(w: &mut World, id: MobyId) {
    let s = c::pi32(w, id, pv::WALL);
    let Some(i) = usize::try_from(s).ok().filter(|&i| w.svc.splines.get(i).is_some_and(|p| p.len() == 7)) else { return };
    let (rows, pos) = (w.m(id).rows, w.m(id).position);
    let pts: Vec<[u32; 4]> = WALL
        .iter()
        .map(|l| std::array::from_fn(|k| (l[0] * rows[0][k] + l[1] * rows[1][k] + l[2] * rows[2][k] + if k == 3 { 1.0 } else { 0.0 } + pos[k]).to_bits()))
        .collect();
    w.svc.splines[i] = pts;
    w.hero_fields_mut().wall_spline = Some(s as i16);
}

/// Level16 `0x2e43e0` (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < pv::SIZE { w.mm(id).pvars.resize(pv::SIZE, 0); }
    let (old_pos, old_rot) = (w.m(id).position, w.m(id).rotation);
    let shown = (w.m(id).mode ^ 1) & 1;
    crate::moby_update::interact::set_talked(w, id, shown as u32);
    let state = w.m(id).state;
    match state {
        0 => {
            c::set_pi16(w, id, pv::VOICE, -1);
            init(w, id);
        }
        1 => {
            let on = w.hero.ground_moby == Some(id) && w.hero.air_ticks == 0;
            if on {
                let (h, m) = (super::hero_pos(w), w.m(id));
                let d = [h[0] - m.position[0], h[1] - m.position[1], h[2] - m.position[2]];
                let ly = d[0] * m.rows[1][0] + d[1] * m.rows[1][1] + d[2] * m.rows[1][2];
                if ly.abs() < 1.2 {
                    choose_path(w, id);
                    let dest = c::pi16(w, id, pv::DEST);
                    let msg = usize::try_from(dest).ok().and_then(|k| PROMPTS.get(k)).copied().unwrap_or(PROMPT_OTHER);
                    w.svc.interact.try_prompt(OWNER, msg);
                    if w.hero.loop_in.pad.pressed & TRIANGLE != 0 && w.svc.interact.prompt.owner == OWNER {
                        w.mm(id).state = 3;
                        if let Some(o) = car_at(w, id, dest) {
                            let m = w.mm(o);
                            m.state = 2;
                            m.has_collision = false;
                            m.mode |= 1;
                            m.visible = 0;
                        }
                    }
                }
            }
        }
        3 => {
            let st = station_shape(w, id, c::pi16(w, id, pv::STATION));
            let start = path_points(w, id).map(|p| crate::path::pose(&p, false, 0.0, true));
            if let (Some(s), Some((p, e))) = (st, start) {
                let a = s.matrix[3];
                let ea = [s.euler[0], s.euler[1], s.euler[2], s.unused_7c];
                if ease(w, id, a, ea, p, e) { w.mm(id).state = 4; }
            }
        }
        4 => {
            let (mut t, mut v) = (c::pf(w, id, pv::T), c::pf(w, id, pv::V));
            let (acc, vmax) = (c::pf(w, id, pv::ACCEL), c::pf(w, id, pv::VMAX));
            turn::spring(1.0, acc, acc, vmax, &mut t, &mut v);
            c::set_pf(w, id, pv::T, t);
            c::set_pf(w, id, pv::V, v);
            if let Some(pts) = path_points(w, id) {
                let (p, e) = crate::path::pose(&pts, false, t * (pts.len() as f32 - 1.0), true);
                c::set_pv4(w, id, pv::POS, p);
                c::set_pv4(w, id, pv::ROT, e);
            }
            if 1.0 <= t {
                c::set_pf(w, id, pv::T, 0.0);
                c::set_pf(w, id, pv::V, 0.0);
                w.mm(id).state = 5;
            }
        }
        5 => {
            let st = station_shape(w, id, c::pi16(w, id, pv::DEST));
            let end = path_points(w, id).map(|p| crate::path::pose(&p, false, p.len() as f32, true));
            if let (Some(s), Some((p, e))) = (st, end) {
                let b = s.matrix[3];
                let eb = [s.euler[0], s.euler[1], s.euler[2], s.unused_7c];
                if ease(w, id, p, e, b, eb) {
                    w.mm(id).state = 1;
                    arrive(w, id);
                }
            }
        }
        6 if super::hints::owned(w, ITEM) => show(w, id),
        _ => {}
    }
    bob(w, id);
    let v = c::pi16(w, id, pv::VOICE) as i32;
    if (3..=5).contains(&w.m(id).state) {
        let f = w.hero_fields_mut();
        f.edge_brake = 2;
        f.jump_lockout = 2;
        wall(w, id);
        if !w.sound_alive(v, id) {
            let s = w.play_sound(0, 4, id);
            c::set_pi16(w, id, pv::VOICE, s as i16);
        }
    } else if w.sound_alive(v, id) {
        w.release_sound(v, id);
        c::set_pi16(w, id, pv::VOICE, -1);
    }
    let m = w.mm(id);
    let disp = [m.position[0] - old_pos[0], m.position[1] - old_pos[1], m.position[2] - old_pos[2], m.position[3] - old_pos[3]];
    let rot = m.rotation;
    triggers::carry_riders(&mut m.pvars, pv::BLOCK, disp, old_rot, rot);
}

/// `0x2e4b58`: the hidden car put at this car's station; the station = the destination.
fn arrive(w: &mut World, id: MobyId) {
    let (station, dest) = (c::pi16(w, id, pv::STATION), c::pi16(w, id, pv::DEST));
    if let Some(o) = car_at(w, id, dest) {
        if let Some(s) = station_shape(w, id, station) {
            let m = w.mm(o);
            m.position = s.matrix[3];
            m.rotation = [s.euler[0], s.euler[1], s.euler[2], s.unused_7c];
        }
        init(w, o);
    }
    c::set_pi16(w, id, pv::STATION, dest);
}
