//! Particle type 69, the drifting twinkle (level01 update `PartType69Update` 0x289df0, the same code on all 19 levels,
//! `overlay-diff`; spawner level00 `0x274948`, one code cluster on levels 00, 14, 15, 18 (`clusters.tsv` 1fa9adb9…;
//! level01 has none); read from the decomp). The census's units U436 (class 28), U33 (1440), U439 (211), U458 (1331),
//! U34 (1471) call the spawner (G-PRT-001 consumers; their classes are not ported).
//!
//! **Spawn** `(pos a0, vel a1, alpha a2, moby a3)`: position = pos, byte1 0, ALPHA 0x48 (additive), byte9 `trunc(4) +
//! 0x70` = 0x74, size `randf(8000, 50000)`, rotation `randi(256)`, texture `def[69][0]`, timer `ticks(rand_range(10,
//! 30))` (the three draws, in that order), +0x30 = 1/timer, +0x34 = alpha (s16), +0x36 = mode 0, +0x38 = 0x7f7f7f,
//! +0x3c = a3, velocity +0x20 = vel, RGBA `alpha << 24 | 0x7f7f7f`.
//!
//! **Update**: outside [2, 1021]³ → killed; mode 0: pos += vel (modes 1–3 copy a point out of the moby's pvars
//! +0xd0 / +0x1f0 / +0xe0 and mode 4 the moby's position + 0.2 up: no spawner sets them, see G-PRT-001; here they hold
//! the position and are counted in [`super::PartStats::unported_branch`]); outside the box again → killed; rotation +=
//! `trunc(randf_sym(0, 64))` (past 0xff: − 0xff; the byte wraps) — one draw a tick; then the timer fires → killed, else
//! A = `trunc(t · (1/life) · alpha)` over +0x38's colour (the twinkle fades out linearly). Native `f32`.

use super::{fast_dec_timer, rec, Particles};
use crate::rng::Rng;

pub const TYPE: u8 = 69;

/// Level00 `0x274948(pos, vel, alpha, moby)`: three draws with a record; none when the pool is full.
pub fn spawn(sys: &mut Particles, rng: &mut Rng, pos: [f32; 4], vel: [f32; 4], alpha: i32, moby: u32) -> Option<usize> {
    let i = sys.create_part(TYPE)?;
    let def = sys.def_first(TYPE);
    let size = rng.randf(8000.0, 50000.0);
    let rot = rng.randi(0x100) as u8;
    let n = rng.rand_range(10, 0x1e);
    let t = sys.time.ticks(n) as i16;
    let r = &mut sys.pool.recs[i];
    rec::set_v4(r, 0x10, pos);
    r[1] = 0;
    r[3] = 0x48;
    r[9] = 4 + 0x70;
    rec::set_ff(r, 0xc, size);
    r[8] = rot;
    r[2] = def;
    rec::set_i16(r, 10, t);
    rec::set_u32(r, 0x3c, moby);
    rec::set_i16(r, 0x34, alpha as i16);
    rec::set_u32(r, 0x38, 0x7f7f7f);
    rec::set_i16(r, 0x36, 0);
    rec::set_ff(r, 0x30, 1.0 / t as f32);
    rec::set_v4(r, 0x20, vel);
    rec::set_u32(r, 4, (alpha as u32) << 24 | 0x7f7f7f);
    Some(i)
}

fn in_box(p: [f32; 3]) -> bool { p.iter().all(|&x| (2.0..=1021.0).contains(&x)) }

/// Update 0x289df0: one draw a tick while it lives in the box.
pub fn update(sys: &mut Particles, i: usize, rng: &mut Rng) {
    if !in_box(rec::pos(&sys.pool.recs[i])) {
        sys.kill_part(i);
        return;
    }
    if rec::i16(&sys.pool.recs[i], 0x36) == 0 {
        let r = &mut sys.pool.recs[i];
        let (p, v) = (rec::v3(r, 0x10), rec::v3(r, 0x20));
        rec::set_v3(r, 0x10, [p[0] + v[0], p[1] + v[1], p[2] + v[2]]);
    } else {
        sys.stats.unported_branch += 1;
    }
    if !in_box(rec::pos(&sys.pool.recs[i])) {
        sys.kill_part(i);
        return;
    }
    let k = rng.randf_sym(0.0, 64.0) as i32;
    let r = &mut sys.pool.recs[i];
    let mut b = r[8] as i32 + k;
    if 0xff < b { b -= 0xff; }
    r[8] = b as u8;
    if fast_dec_timer(r, 10) != 0 {
        sys.kill_part(i);
        return;
    }
    let a = (rec::i16(r, 10) as f32 * rec::ff(r, 0x30) * rec::i16(r, 0x34) as f32) as i32;
    rec::set_u32(r, 4, (a as u32) << 24 | rec::u32(r, 0x38));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three spawn draws in order, a draw a tick for the twinkle, the linear fade, the life, the box.
    #[test]
    fn drifts_twinkles_fades_and_dies() {
        let mut s = Particles::new(None, Vec::new());
        let mut rng = Rng::new();
        let mut twin = rng;
        let i = spawn(&mut s, &mut rng, [10.0, 10.0, 10.0, 1.0], [0.1, 0.0, 0.0, 0.0], 0x60, 0).unwrap();
        let size = twin.randf(8000.0, 50000.0);
        let rot = twin.randi(0x100) as u8;
        let life = twin.rand_range(10, 30);
        assert_eq!(rng, twin);
        let r = &s.pool.recs[i];
        assert_eq!((rec::ff(r, 0xc), r[8], rec::i16(r, 10) as i32, r[9]), (size, rot, life, 0x74));
        assert_eq!(rec::u32(r, 4), 0x607f_7f7f);
        s.update_parts(&mut rng);
        let k = twin.randf_sym(0.0, 64.0) as i32;
        assert_eq!(rng, twin, "one draw a tick");
        let r = &s.pool.recs[i];
        assert_eq!(rec::pos(r), [10.1, 10.0, 10.0]);
        let mut b = rot as i32 + k;
        if b > 0xff { b -= 0xff; }
        assert_eq!(r[8], b as u8);
        let a = ((life - 1) as f32 * (1.0 / life as f32) * 96.0) as u32;
        assert_eq!(rec::u32(r, 4), a << 24 | 0x7f7f7f);
        let mut n = 1;
        while s.pool.count > 0 {
            s.update_parts(&mut rng);
            n += 1;
        }
        assert_eq!(n, life);
        // Out of the box: killed with no draw.
        let before = rng;
        spawn(&mut s, &mut rng, [1.0, 10.0, 10.0, 1.0], [0.0; 4], 0x60, 0).unwrap();
        let after = rng;
        s.update_parts(&mut rng);
        assert_eq!(s.pool.count, 0);
        assert_eq!(rng, after);
        assert_ne!(before, after);
    }
}
