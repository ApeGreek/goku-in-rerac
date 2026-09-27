//! Particle types 45 and 66, the flat splash rings (level01 spawners `PartType45Spawn` 0x286780 / `PartType66Spawn`
//! 0x289648, updates 0x2868e0 / 0x2897b0, read from the decomp): the drops' landing rings (type 35), the hero's
//! surface wake (45), the bomb scorch's ground rings (45).
//!
//! **Spawn** `(size, growth, pos[, level], rgba)`: **kind 1** (flat quad in the world XY plane), additive (0x48),
//! byte9 0x24 (near 1 u, far 64 u), RGBA (−1: `(0x14 + randi(6)) << 24 | 0x808080`), rotation = the low byte of one
//! raw `rand()`, timer 4 (45) / 4 (66), size `size·210000`, a drift of `randf(0.0015, 0.0025)` along `rand_angle`
//! (vel.z 0), +0x30 the growth; 45 keeps a water-level pointer +0x34 (0 here: the callers the port has pass none);
//! 66 picks `def[66][1]` or `def[66][0]` on the parity of `randi(0x100)`, 45 `def[45][0]`. Draws, in order: 45 —
//! (`randi(6)`), `rand()`, `rand_angle`, `randf`; 66 — (`randi(6)`), `randi(0x100)`, `rand()`, `rand_angle`, `randf`.
//!
//! **Update**: pos += vel (45: killed outside [2, 1021]³, tested after the move); rotation + 1; size += growth; the
//! timer steps and, at 0, restarts (45: 4, 66: 2) and the alpha drops by one; killed at alpha 0. Standard `f32`.

use super::{rec, Particles};
use crate::rng::Rng;

/// `PartType45Spawn(size, growth, pos, 0, rgba)` 0x286780 (no water-level pointer).
pub fn spawn45(sys: &mut Particles, rng: &mut Rng, size: f32, growth: f32, pos: [f32; 4], rgba: u32) -> Option<usize> {
    let i = sys.create_part(45)?;
    let def = sys.def_first(45);
    let rgba = if rgba == u32::MAX { ((0x14 + rng.randi(6)) as u32) << 24 | 0x80_8080 } else { rgba };
    let rot = rng.rand() as u8;
    let a = rng.rand_angle();
    let v = rng.randf(f32::from_bits(0x3ac4_9ba6), f32::from_bits(0x3b23_d70a));
    let r = &mut sys.pool.recs[i];
    rec::set_v4(r, 0x10, pos);
    rec::set_u32(r, 4, rgba);
    r[1] = 1;
    r[9] = 4 + 0x20;
    r[3] = 0x48;
    r[2] = def;
    r[8] = rot;
    rec::set_i16(r, 0xa, 4);
    rec::set_ff(r, 0xc, size * 210000.0);
    rec::set_v4(r, 0x20, [a.cos() * v, a.sin() * v, 0.0, rec::ff(r, 0x2c)]);
    rec::set_u32(r, 0x34, 0);
    rec::set_ff(r, 0x30, growth);
    Some(i)
}

/// `PartType66Spawn(size, growth, pos, rgba)` 0x289648.
pub fn spawn66(sys: &mut Particles, rng: &mut Rng, size: f32, growth: f32, pos: [f32; 4], rgba: u32) -> Option<usize> {
    let i = sys.create_part(66)?;
    let rgba = if rgba == u32::MAX { ((0x14 + rng.randi(6)) as u32) << 24 | 0x80_8080 } else { rgba };
    let pick = rng.randi(0x100);
    let def = sys.def_frame(66, if pick & 1 == 0 { 1 } else { 0 });
    let rot = rng.rand() as u8;
    let a = rng.rand_angle();
    let v = rng.randf(f32::from_bits(0x3ac4_9ba6), f32::from_bits(0x3b23_d70a));
    let r = &mut sys.pool.recs[i];
    rec::set_v4(r, 0x10, pos);
    rec::set_u32(r, 4, rgba);
    r[9] = 4 + 0x20;
    r[3] = 0x48;
    r[1] = 1;
    r[2] = def;
    r[8] = rot;
    rec::set_i16(r, 0xa, 4);
    rec::set_ff(r, 0xc, size * 210000.0);
    rec::set_v4(r, 0x20, [a.cos() * v, a.sin() * v, 0.0, rec::ff(r, 0x2c)]);
    rec::set_ff(r, 0x30, growth);
    Some(i)
}

fn step(sys: &mut Particles, i: usize, restart: i16) {
    let r = &mut sys.pool.recs[i];
    r[8] = r[8].wrapping_add(1);
    rec::set_ff(r, 0xc, rec::ff(r, 0xc) + rec::ff(r, 0x30));
    let t = rec::i16(r, 0xa) - 1;
    rec::set_i16(r, 0xa, t);
    if t == 0 {
        rec::set_i16(r, 0xa, restart);
        rec::set_u32(r, 4, rec::u32(r, 4).wrapping_sub(0x0100_0000));
    }
    if rec::u32(r, 4) & 0xff00_0000 == 0 { sys.kill_part(i); }
}

/// Update 0x2868e0 (type 45; no RNG; the water-level pointer is never set here).
pub fn update45(sys: &mut Particles, i: usize, _rng: &mut Rng) {
    let r = &mut sys.pool.recs[i];
    let (p, v) = (rec::v3(r, 0x10), rec::v3(r, 0x20));
    let p = [p[0] + v[0], p[1] + v[1], p[2] + v[2]];
    rec::set_v3(r, 0x10, p);
    if p.iter().any(|&x| !(2.0..=1021.0).contains(&x)) {
        sys.kill_part(i);
        return;
    }
    step(sys, i, 4);
}

/// Update 0x2897b0 (type 66; no RNG).
pub fn update66(sys: &mut Particles, i: usize, _rng: &mut Rng) {
    let r = &mut sys.pool.recs[i];
    let (p, v) = (rec::v3(r, 0x10), rec::v3(r, 0x20));
    rec::set_v3(r, 0x10, [p[0] + v[0], p[1] + v[1], p[2] + v[2]]);
    step(sys, i, 2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_fade_one_alpha_step_per_period() {
        let mut s = Particles::new(None, Vec::new());
        let mut rng = Rng::new();
        let i = spawn45(&mut s, &mut rng, 0.2, 5250.0, [10.0, 10.0, 10.0, 0.0], 0x0280_8080).unwrap();
        let j = spawn66(&mut s, &mut rng, 0.25, 3150.0, [10.0, 10.0, 10.0, 0.0], 0x0280_8080).unwrap();
        assert_eq!((s.pool.recs[i][1], s.pool.recs[j][1]), (1, 1));
        let mut ticks = [0; 2];
        for n in 1..100 {
            s.update_parts(&mut rng);
            for (k, &x) in [i, j].iter().enumerate() {
                if ticks[k] == 0 && s.pool.recs[x][1] & super::super::FLAG_DEAD != 0 { ticks[k] = n; }
            }
        }
        // Alpha 2: 45 loses one every 4 updates (dies on the 8th), 66 after 4 then every 2 (dies on the 6th).
        assert_eq!(ticks, [8, 6]);
    }
}
