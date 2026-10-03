//! **Orxon's energy barriers, class 1067** (level10 `0x2da2c8`, its draw `0x2da690`; census U357; 6 placed) and **the
//! strand field they draw from, class 1073** (`0x2dcc58`, U358; 1 placed: #609). Both are deleted on level 10 while
//! Clank is the hero (0x1413f4 = 1).
//!
//! **The field 1073** keeps 4 layers × 10 strands × 20 points (positions +0x00 + layer·0x2580 + strand·0x10 +
//! point·0x140, their drifts at +0x9600 the same way, the strands' timers at +0x12c00 + layer·0x78 + strand·4, a
//! scroll phase at +0x12de0). At its start the timers are staggered (`strand·45/10` + 0 / 11 / 23 / 34 by layer; the
//! lifetime 45 and the fade point 22 are the globals 0x161d64 / 0x161d68). Each tick the phase += 2·dt (wrapped), and
//! every strand's timer counts down: running, its inner points 1..18 drift; out, the strand is laid straight along
//! y from −1.25 to 1.25 (gp−0x4ea8 2.5) and its inner points get the mean of three neighbouring `rand_vec(0, 0.4·dt)`
//! drifts, the timer 45.
//!
//! **A barrier 1067** (pvars: +0x00 the width, +0x04 the rows, +0x08 their spacing, +0x0c the field, +0x10.. a
//! shuffled strand per row, +0x30 the voice, +0x34 the fade-out timer): its start shuffles the strand list 0..9 (10
//! passes of swapping forward by `randi(9)`, the draw re-made each step), turns it a quarter, hides the moby (mode
//! 0x41: the draw is the barrier), update distance 0x60. On (1): within 80 of the camera and its sphere in view the
//! draw (list 2), the hum (sound 0, flags 4) kept alive. Switched off (2, by another class): the fade-out timer
//! counts down from `scale(0.45·60)`, the hum released, the draw.
//!
//! **The draw** (`0x2da690`): per row (the rows 1..n spacing above the barrier, the barrier's rotation scaled by
//! width/2), per layer and per segment 0..18 of the row's strand: a quad from the segment's two points, ±0.05 thick
//! (local z), ST (s, 0) (s, 1) (s + 0.25, 0) (s + 0.25, 1) with s stepping 0.25 a segment plus the phase; FX 0xe,
//! additive; the colour by the strand's age (`FastTweenColor` of (t − 22)/23 above 22, else 1 − t/22, between
//! 0x7fc04070 and 0x00ff0000), its alpha ·fade² (fade 1 within 48 of the camera, to 0 at 80); switched off: white at
//! alpha `timer·255/scale(27)`.
//!
//! Read from the level10 decomp; the tables from the overlay (0x1d9ff0 ST, 0x1da010 the thickness offsets) and gp
//! (−0x4f70.. ALPHA 0, 2, 0, 1; −0x4f70.. colours). [L] The view check's distance argument is not visible in the decomp
//! (one argument): 80 here; the fade is computed by the update (the camera of the same tick).
//!
//! | address | what | port |
//! |---|---|---|
//! | `0x2da2c8` | the barrier | [`update`] |
//! | `0x2da690` | its draw | [`fx_quads`] (`Callback::UnitQuads`) |
//! | `0x2dcc58` | the field | [`field_update`] |

use super::{FxQuad, FxQuads};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::classes::draw_callbacks::Callback;
use crate::moby_update::creature::{self as c, DT};
use crate::moby_update::services::{pvar as p, Services, World};
use crate::moby_update::story;

pub const REFERENCE_LEVEL: u32 = 10;
pub const UPDATE_FN: u32 = 0x2d_a2c8;
pub const DRAW_FN: u32 = 0x2d_a690;
pub const FIELD_FN: u32 = 0x2d_cc58;
pub const CLASSES: [i16; 1] = [1067];
pub const FIELD_CLASSES: [i16; 1] = [1073];

const FIELD_LEN: usize = 0x12de4;
const POS: usize = 0x0;
const VEL: usize = 0x9600;
const TIMERS: usize = 0x12c00;
const PHASE: usize = 0x12de0;
const LAYER: usize = 0x2580;
const POINT: usize = 0x140;
const STRAND: usize = 0x10;
const LIFE: i32 = 0x2d;
const MID: i32 = 0x16;
const LENGTH: f32 = 2.5;
const DRIFT: f32 = 0.4;
const SCROLL: f32 = 2.0;
const SEG_S: f32 = 0.25;
const OFF: f32 = 0.45;
const COLOURS: (u32, u32) = (0x7fc0_4070, 0x00ff_0000);
/// The barrier's port slot: the distance fade of this tick.
const FADE: usize = 0x38;
const LEN: usize = 0x3c;

fn gone(w: &World) -> bool { w.svc.level == 10 && w.hero.mode == 1 }

fn timer_off(layer: usize, strand: usize) -> usize { TIMERS + layer * 0x78 + strand * 4 }
fn point_off(base: usize, layer: usize, strand: usize, pt: usize) -> usize { base + layer * LAYER + strand * STRAND + pt * POINT }

/// Level10 `0x2dcc58`: the field (module doc).
pub fn field_update(w: &mut World, id: MobyId) {
    story::pvars(w, id, FIELD_LEN);
    match w.m(id).state {
        0 => {
            if gone(w) {
                w.delete_moby(id);
                return;
            }
            for (layer, base) in [0, 0xb, 0x17, 0x22].into_iter().enumerate() {
                for s in 0..10 {
                    c::set_pi32(w, id, timer_off(layer, s), (s as i32 * LIFE) / 10 + base);
                }
            }
            w.mm(id).state = 1;
        }
        1 => {
            let mut ph = c::pf(w, id, PHASE) + SCROLL * DT;
            if 1.0 < ph { ph -= 1.0; }
            c::set_pf(w, id, PHASE, ph);
            for layer in 0..4 {
                for s in 0..10 {
                    if c::dec_timer_pvar_i32(w, id, timer_off(layer, s)) == 0 {
                        for pt in 1..19 {
                            let a = c::pv4(w, id, point_off(POS, layer, s, pt));
                            let v = c::pv4(w, id, point_off(VEL, layer, s, pt));
                            c::set_pv4(w, id, point_off(POS, layer, s, pt), c::add(a, v));
                        }
                        continue;
                    }
                    let mut tmp = [[0.0f32; 4]; 20];
                    for (pt, t) in tmp.iter_mut().enumerate() {
                        let y = pt as f32 * (LENGTH / 20.0) - LENGTH * 0.5;
                        c::set_pv4(w, id, point_off(POS, layer, s, pt), [0.0, y, 0.0, 1.0]);
                        let r = w.rng.rand_vec(0.0, DRIFT * DT);
                        *t = [r[0], r[1], r[2], 0.0];
                    }
                    for pt in 1..19 {
                        let v = c::add(c::add(tmp[pt - 1], tmp[pt]), tmp[pt + 1]).map(|x| x * f32::from_bits(0x3eaa_7efa));
                        c::set_pv4(w, id, point_off(VEL, layer, s, pt), v);
                    }
                    c::set_pi32(w, id, timer_off(layer, s), LIFE);
                }
            }
        }
        _ => {}
    }
}

/// The view sphere and its registration (states 1 and 2).
fn draw(w: &mut World, id: MobyId) {
    let pos = c::pos(w, id);
    let (width, rows, sp) = (c::pf(w, id, 0), c::pi32(w, id, 4), c::pf(w, id, 8));
    let h = rows as f32 * sp * 0.5;
    let r = (h * h + width * 0.5 * width * 0.5).abs().sqrt();
    let cam = w.camera.map(|x| f32::from_bits(x.0));
    let d = c::dist3(pos, [cam[0], cam[1], cam[2], 0.0]);
    let fade = if 48.0 < d { (80.0 - d) * 0.031_25 } else { 1.0 };
    c::set_pf(w, id, FADE, fade);
    if d < 80.0 && crate::moby_update::creature::fx::in_view(w, 80.0, [pos[0], pos[1], pos[2] + h, 0.0], r) {
        if let Some(row) = super::row(REFERENCE_LEVEL, DRAW_FN) { w.svc.draw_callbacks.register2(Callback::UnitQuads(row), id); }
    }
}

/// Level10 `0x2da2c8`: the barrier (module doc).
pub fn update(w: &mut World, id: MobyId) {
    story::pvars(w, id, LEN);
    if c::pi32(w, id, 0xc) == -1 {
        w.delete_moby(id);
        return;
    }
    match w.m(id).state {
        0 => {
            if gone(w) {
                w.delete_moby(id);
                return;
            }
            let mut b: [u8; 10] = std::array::from_fn(|i| i as u8);
            for _ in 0..10 {
                let mut j = 0;
                while j < w.rng.randi(9) as usize {
                    b.swap(j, j + 1);
                    j += 1;
                }
            }
            let n = c::pi32(w, id, 4).clamp(0, 0x20) as usize;
            for (k, &v) in b.iter().take(n).enumerate() { w.mm(id).pvars[0x10 + k] = v; }
            let z = c::add_rot(w.m(id).rotation[2], std::f32::consts::FRAC_PI_2);
            let t = w.ticks((OFF * 60.0) as i32);
            let m = w.mm(id);
            m.state = 1;
            m.update_dist = 0x60;
            m.rotation[2] = z;
            m.mode |= 0x41;
            c::set_pi32(w, id, 0x30, -1);
            c::set_pi32(w, id, 0x34, t);
        }
        1 => {
            draw(w, id);
            let v = c::pi32(w, id, 0x30);
            if !w.sound_alive(v, id) {
                let s = w.play_sound(0, 4, id);
                c::set_pi32(w, id, 0x30, s);
            }
        }
        2 => {
            c::dec_timer_pvar_i32(w, id, 0x34);
            let v = c::pi32(w, id, 0x30);
            if v != -1 { w.release_sound(v, id); }
            c::set_pi32(w, id, 0x30, -1);
            draw(w, id);
        }
        _ => {}
    }
}

/// Level10 `0x2da690`: the barrier's quads (module doc).
pub fn fx_quads(table: &MobyTable, svc: &Services, id: MobyId) -> Option<FxQuads> {
    let _ = svc;
    let m = table.mobys.get(id).filter(|m| m.pvars.len() >= LEN)?;
    let q = &m.pvars;
    let field = table.mobys.get(usize::try_from(p::i32(q, 0xc)).ok()?).filter(|f| f.pvars.len() >= FIELD_LEN)?;
    let fq = &field.pvars;
    let (width, rows, sp) = (p::ff(q, 0), p::i32(q, 4).clamp(0, 0x20) as usize, p::ff(q, 8));
    let fade = p::ff(q, FADE);
    let phase = p::ff(fq, PHASE);
    let rot = crate::moby_update::services::euler_rows(crate::moby_update::services::pv(m.rotation));
    let r: [[f32; 4]; 4] = rot.map(|row| row.map(|x| f32::from_bits(x.0)));
    let k = width * 0.5;
    let off = OFF * 60.0;
    let fade_ticks = (svc.ticks(off as i32) as f32).max(1.0);
    let mut quads = Vec::new();
    for layer in 0..4 {
        let mut seg_s = 0.0f32;
        for seg in 0..19 {
            seg_s += SEG_S;
            if 1.0 < seg_s { seg_s -= 1.0; }
            let s0 = seg_s + phase;
            let s1 = SEG_S + seg_s + phase;
            for row in 0..rows {
                let strand = q[0x10 + row] as usize % 10;
                let t = p::i32(fq, timer_off(layer, strand));
                let f = if MID < t { (t - MID) as f32 / (LIFE - MID) as f32 } else { 1.0 - t as f32 / MID as f32 };
                let mut rgba = crate::particles::tween_color(f.to_bits(), COLOURS.0, COLOURS.1);
                let a = (fade * fade * (rgba >> 24) as f32) as i32 as u32;
                rgba = (rgba & 0xff_ffff) | a << 24;
                if m.state == 2 {
                    let a = ((p::i32(q, 0x34) as f32 * 255.0) / fade_ticks) as i32 as u32;
                    rgba = a << 24 | 0xff_ffff;
                }
                let z = m.position[2] + (row + 1) as f32 * sp;
                let base = [m.position[0], m.position[1], z];
                let corner = |pt: usize, dz: f32| -> [f32; 3] {
                    let l = p::v4f(fq, point_off(POS, layer, strand, pt));
                    let v = [l[0], l[1], l[2] + dz];
                    std::array::from_fn(|i| (r[0][i] * v[0] + r[1][i] * v[1] + r[2][i] * v[2]) * k + base[i])
                };
                let corners = [corner(seg, -0.05), corner(seg, 0.05), corner(seg + 1, -0.05), corner(seg + 1, 0.05)];
                quads.push(FxQuad { corners, st: [[s0, 0.0], [s0, 1.0], [s1, 0.0], [s1, 1.0]], rgba: [rgba; 4] });
            }
        }
    }
    Some(FxQuads { fx: 0xe, additive: true, quads })
}
