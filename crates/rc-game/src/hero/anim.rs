//! Ratchet's animation interface: the calls the hero code makes (`SetAnim` 0x247a90, Ratchet's own
//! advance 0x247d48) and the moby fields it reads back, as a trait the engine binds to
//! `rc_formats::moby_anim` (`docs/plan/player_controller.md` §6, `docs/plan/moby_animation.md` §4).
//!
//! **`SetAnim(blend, seq, frame)` (0x247a90), what the binding must do:**
//! * if `0x13fde8 & 2`: t (+0x54) = 0 first (not used by the on-foot states);
//! * always snapshot the current pose into key A (`fun_0020ede8(moby, slot | 0x300)`, seq A = 0xff) —
//!   unlike the generic `fun_00212f90`, there is no `t > 0.025` test;
//! * frame B (+0x51) = `frame` (not clamped here), seq B (+0x53) = `seq` (0x31 ↔ 0x32 swapped with the
//!   mirror option 0x15edb5), `update_moby_animation_state`;
//! * playback speed `0x13fde0` = 1; t = 0; `blend > 0`: rate `0x13fde4` = 1 / blend (a blend over `blend`
//!   ticks), curve `0x13fdf0` = −1; `blend ≤ 0`: rate 1, eased curve `(int)(−blend) − 1` (t table at
//!   `0x17c270 + 100·curve`, length from the gp table −0x7520), curve position `0x13fdf4` = 0;
//! * loop-sound byte (+0x7c) from the target sequence header; `0x13fdec` = 1 ("anim set this tick").
//!
//! **Ratchet's advance (0x247d48)** differs from `MobyAnimAdvance`: during a blend t += `0x13fde4` (or the
//! next value of the eased curve table), on the same sequence t += `0x13fde0 · 0x13fde4`; t in (0.99, 1.01)
//! snaps to 1 and |t| < 0.01 to 0; each key step sets `0x13fde8 |= 1`, a wrap `|= 2` (bits cleared at the
//! start of every advance), and the new rate `0x13fde4` is the new key A frame's own rate word.
//!
//! The hero owns `0x13fde0` (playback speed, [`crate::hero::Hero::anim_speed`]) and passes it to
//! [`AnimCtl::advance`] once per tick (step 1 of the hero update); the rate `0x13fde4`, the curve state
//! `0x13fdf0/f4` and the flags `0x13fde8` live in the binding.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::ps2v::Pf;

/// The animation fields of Ratchet's moby that the hero code reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimView {
    /// +0x52 / +0x53: key A / key B sequence (A = 0xff while blending from a snapshot).
    pub seq_a: u8,
    pub seq_b: u8,
    /// +0x50 / +0x51: key A / key B frame index.
    pub frame_a: u8,
    pub frame_b: u8,
    /// +0x54: blend A → B.
    pub t: f32,
    /// `0x13fde8` (Ratchet's own, not moby+0x70): bit 0 crossed a key this tick, bit 1 wrapped.
    pub flags: u8,
    /// `0x13fde4`: t increment per tick (1/blend during a blend, then the key's rate).
    pub rate: f32,
    /// `0x13fdf8`: Ratchet's current key time (`0x263920`, frame index + t), written by the advance.
    pub frame: f32,
    /// Frame count of sequence B (sequence header +0x10).
    pub frame_count_b: u8,
    /// Rate word of frame B (`*(moby+0x6c)`).
    pub frame_b_rate: f32,
    /// `0x13fdfc`: how far the readout moved this tick (`new − old` when it did not go back, else the t
    /// increment of the advance); `0x231f18(f)` "passed frame f" tests `f < frame && frame − f ≤ this`.
    pub frame_step: f32,
}

impl AnimView {
    /// `0x13fdec`: set by `SetAnim`, then `seq_a != seq_b` after every advance (a blend is running).
    pub fn blending(&self) -> bool { self.seq_a != self.seq_b }
}

/// What the hero needs from the animation system. The engine implements it on Ratchet's moby with
/// `rc_formats::moby_anim`; [`RecordingAnim`] is a data-free stand-in for tests.
pub trait AnimCtl {
    /// `SetAnim(blend, seq, frame)` (0x247a90), see the module doc. `blend` > 0: blend ticks (the game
    /// passes `(float)ticks(n)`); ≤ 0: eased curve `−blend`.
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32);
    /// Ratchet's advance 0x247d48 for one tick at playback speed `speed` (`0x13fde0`). The generic
    /// `MobyAnimAdvance` is not run on Ratchet (mode 2).
    fn advance(&mut self, speed: Pf);
    /// The moby's current animation fields.
    fn view(&self) -> AnimView;
    /// Frame count of sequence `seq` of Ratchet's class (0 when absent).
    fn frame_count(&self, seq: u8) -> u8;
    /// `0x247cb8(start, end)`: loop key B between `start` and `end` (re-entry at rate 1/3) until cleared;
    /// ignored unless `start ≥ 0` and `end` < the frame count of sequence B.
    fn set_loop(&mut self, start: i32, end: i32);
    /// `0x247d00()`: clear the loop range (also done by every `set_anim`).
    fn clear_loop(&mut self);
    /// The pose matrices `P_j` of the last joints of `chains` (root-to-joint byte lists of Ratchet's joint
    /// lists) in Ratchet's current pose (`fun_00210850` / `fun_002109b8`,
    /// `rc_formats::moby_anim::evaluate_chains`). Empty when there is no class data.
    fn eval_chains(&self, _chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> { Vec::new() }
    /// Ratchet's current local pose as one keyframe (the decode `MobyAnimDecodeLocalPose` 0x269938 does, in
    /// the frame encoding of the pose snapshot, `rc_formats::moby_anim::snapshot`). None without class data.
    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { None }
}

/// A data-free [`AnimCtl`]: records every `set_anim` and fakes the fields the hero reads (the blend lands
/// on key B after `blend` ticks; sequences are treated as endless, so `flags` never reports a wrap).
#[derive(Clone, Debug, Default)]
pub struct RecordingAnim {
    pub calls: Vec<(Pf, u8, i32)>,
    pub v: AnimView,
    blend_left: f32,
}

impl AnimCtl for RecordingAnim {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) {
        self.calls.push((blend, seq, frame));
        self.v.seq_a = 0xff;
        self.v.seq_b = seq;
        self.v.frame_b = frame as u8;
        self.v.t = 0.0;
        if self.v.frame_count_b == 0 { self.v.frame_count_b = 30; }
        if self.v.frame_b_rate == 0.0 { self.v.frame_b_rate = 1.0; }
        self.blend_left = if Pf::ZERO < blend { blend.to_f32() } else { 1.0 };
        self.v.rate = if Pf::ZERO < blend { 1.0 / blend.to_f32() } else { 1.0 };
    }
    fn advance(&mut self, speed: Pf) {
        self.v.flags = 0;
        if self.v.seq_a != self.v.seq_b {
            self.blend_left -= 1.0;
            self.v.t += self.v.rate;
            if self.blend_left <= 0.0 {
                self.v.seq_a = self.v.seq_b;
                self.v.frame_a = self.v.frame_b;
                self.v.t = 0.0;
                self.v.flags = 1;
            }
        } else {
            self.v.frame += speed.to_f32();
        }
    }
    fn view(&self) -> AnimView { self.v }
    fn frame_count(&self, _seq: u8) -> u8 { 30 }
    fn set_loop(&mut self, _start: i32, _end: i32) {}
    fn clear_loop(&mut self) {}
}

// ------------------------------------------------------------------------------------------------
// Ratchet's own animation path on `rc_formats::moby_anim` data.

use rc_formats::moby_anim::{snapshot, AnimState, MobyAnimClass, MobyFrame, SNAPSHOT_SEQ};

/// Eased blend curves (t per tick) at 0x17c270 + 100·(n − 1): −1 = 11 ticks, −2 = 16, −3 = 3.
pub const CURVES: [&[u32]; 3] = [
    &[0x3e19_999a, 0x3e94_7ae1, 0x3ed7_0a3d, 0x3f07_ae14, 0x3f21_47ae, 0x3f38_51ec, 0x3f4c_cccd, 0x3f5e_b852, 0x3f6e_147b, 0x3f78_51ec, 0x3f80_0000],
    &[
        0x3dcc_cccd, 0x3e4c_cccd, 0x3e94_7ae1, 0x3ec2_8f5c, 0x3eeb_851f, 0x3f0a_3d71, 0x3f1c_28f6, 0x3f2e_147b, 0x3f3d_70a4, 0x3f4c_cccd,
        0x3f59_999a, 0x3f66_6666, 0x3f70_a3d7, 0x3f78_51ec, 0x3f7d_70a4, 0x3f80_0000,
    ],
    &[0x3f00_0000, 0x3f40_0000, 0x3f80_0000],
];

/// Ratchet's animation state: the moby's anim fields (+0x50..+0x70 as [`AnimState`]; its `rate`/`speed`
/// are not used by this path) plus Ratchet's globals `0x13fde4` (rate), `0x13fde8` (flags), `0x13fdf0/f4`
/// (curve, position), `0x13fdf8` (frame readout), `0x13fe00/04` (loop range) and the snapshot frame.
#[derive(Clone, Debug)]
pub struct RatchetAnim {
    pub state: AnimState,
    pub snapshot: Option<MobyFrame>,
    pub rate: Pf,
    pub flags: u8,
    pub curve: i32,
    pub curve_pos: usize,
    pub frame: Pf,
    /// `0x13fdfc`: see [`AnimView::frame_step`].
    pub frame_step: Pf,
    pub loop_range: Option<(i32, i32)>,
    /// Option 0x15edb5: swap sequences 0x31 ↔ 0x32.
    pub mirror: bool,
}

impl RatchetAnim {
    /// Spawned on sequence 0 frame 0 (what `InitMobyInstance` leaves).
    pub fn new(class: &MobyAnimClass) -> RatchetAnim {
        RatchetAnim { state: AnimState::spawn(class), snapshot: None, rate: Pf::ONE, flags: 0, curve: -1, curve_pos: 0, frame: Pf::ZERO, frame_step: Pf::ZERO, loop_range: None, mirror: false }
    }

    /// Bind to Ratchet's class for one tick of hero code.
    pub fn ctl<'a>(&'a mut self, class: &'a MobyAnimClass) -> RatchetAnimCtl<'a> { RatchetAnimCtl { a: self, class } }

    fn key_time(f: Option<&MobyFrame>) -> Pf { Pf::from_i32(f.map(|f| f.header.time as i32).unwrap_or(0)) }
}

/// [`AnimCtl`] on [`RatchetAnim`] with the class data (`SetAnim` 0x247a90, advance 0x247d48).
pub struct RatchetAnimCtl<'a> {
    pub a: &'a mut RatchetAnim,
    pub class: &'a MobyAnimClass,
}

impl RatchetAnimCtl<'_> {
    fn frame_readout(&self) -> Pf {
        let s = &self.a.state;
        let c = self.class;
        let fb = c.frame(s.seq_b, s.frame_b);
        let fa = if s.seq_a == SNAPSHOT_SEQ { fb } else { c.frame(s.seq_a, s.frame_a) };
        let t = Pf::f(s.t);
        let k = Pf::b(0x3d80_0000); // 0.0625
        let ta = RatchetAnim::key_time(fa);
        if t == Pf::ZERO { return ta * k; }
        if s.seq_a == s.seq_b && s.frame_a <= s.frame_b {
            let tb = RatchetAnim::key_time(fb);
            return (ta * (Pf::ONE - t) + tb * t) * k;
        }
        ta * k + t
    }
}

impl AnimCtl for RatchetAnimCtl<'_> {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) {
        let a = &mut *self.a;
        if a.flags & 2 != 0 { a.state.t = 0.0; }
        let seq = if a.mirror && seq == 0x31 { 0x32 } else if a.mirror && seq == 0x32 { 0x31 } else { seq };
        if let Some(f) = snapshot(self.class, &a.state, a.snapshot.as_ref()) {
            a.snapshot = Some(f);
            a.state.seq_a = SNAPSHOT_SEQ;
            a.state.frame_a = 0;
        }
        a.state.frame_b = frame as u8;
        a.state.seq_b = seq;
        if let Some(q) = self.class.sequence(seq) { a.state.trigger_count = q.header.trigger_count; }
        if Pf::ZERO < blend {
            a.rate = Pf::ONE / blend;
            a.state.t = 0.0;
            a.curve = -1;
        } else {
            a.rate = Pf::ONE;
            a.curve = (-blend).to_i32() - 1;
            a.curve_pos = 0;
            a.state.t = 0.0;
        }
        a.loop_range = None;
    }

    fn advance(&mut self, speed: Pf) {
        let class = self.class;
        let a = &mut *self.a;
        a.flags &= !3;
        let t0 = Pf::f(a.state.t);
        let mut t = if a.state.seq_a == a.state.seq_b {
            t0 + speed * a.rate
        } else if a.curve >= 0 {
            let c = CURVES.get(a.curve as usize).copied().unwrap_or(&[0x3f80_0000]);
            let v = Pf(c[a.curve_pos.min(c.len() - 1)]);
            a.curve_pos += 1;
            v
        } else {
            t0 + a.rate
        };
        if Pf::b(0x3f7d_70a4) < t && t < Pf::b(0x3f81_47ae) { t = Pf::ONE; }
        if Pf::b(0xbc23_d70a) < t && t < Pf::b(0x3c23_d70a) { t = Pf::ZERO; }
        let dt = t - t0;
        let mut guard = 0;
        while Pf::ONE <= t && guard < 256 {
            guard += 1;
            let s = &mut a.state;
            if s.seq_a != s.seq_b {
                s.seq_a = s.seq_b;
                if let Some(q) = class.sequence(s.seq_b) { s.trigger_count = q.header.trigger_count; }
            }
            a.curve = -1;
            a.flags |= 1;
            s.frame_a = s.frame_b;
            s.frame_b = s.frame_b.wrapping_add(1);
            if let Some((start, end)) = a.loop_range {
                if end < s.frame_b as i32 {
                    a.rate = Pf::b(0x3eaa_aaab);
                    t = Pf::ZERO;
                    s.frame_b = start as u8;
                    continue;
                }
            }
            let fc = class.sequence(s.seq_b).map(|q| q.header.frame_count).unwrap_or(1);
            if s.frame_b >= fc {
                s.frame_b = 0;
                a.flags |= 2;
            }
            t = t - Pf::ONE;
            t = t / a.rate;
            a.rate = class.frame(s.seq_a, s.frame_a).map(|f| Pf::f(f.header.rate)).unwrap_or(Pf::ONE);
            t = t * a.rate;
        }
        a.state.t = t.to_f32();
        a.state.flags = a.flags;
        a.state.rate = a.rate.to_f32();
        a.state.speed = speed.to_f32();
        let f = self.frame_readout();
        let old = self.a.frame;
        self.a.frame_step = if old <= f { f - old } else { dt };
        self.a.frame = f;
    }

    fn view(&self) -> AnimView {
        let s = &self.a.state;
        let fb = self.class.frame(s.seq_b, s.frame_b);
        AnimView {
            seq_a: s.seq_a,
            seq_b: s.seq_b,
            frame_a: s.frame_a,
            frame_b: s.frame_b,
            t: s.t,
            flags: self.a.flags,
            rate: self.a.rate.to_f32(),
            frame: self.a.frame.to_f32(),
            frame_count_b: self.class.sequence(s.seq_b).map(|q| q.header.frame_count).unwrap_or(0),
            frame_b_rate: fb.map(|f| f.header.rate).unwrap_or(1.0),
            frame_step: self.a.frame_step.to_f32(),
        }
    }

    fn frame_count(&self, seq: u8) -> u8 { self.class.sequence(seq).map(|q| q.header.frame_count).unwrap_or(0) }

    fn set_loop(&mut self, start: i32, end: i32) {
        let fc = self.frame_count(self.a.state.seq_b) as i32;
        if start >= 0 && end < fc { self.a.loop_range = Some((start, end)); }
    }

    fn clear_loop(&mut self) { self.a.loop_range = None; }

    fn eval_chains(&self, chains: &[&[u8]]) -> Vec<rc_formats::moby_anim::Rows> {
        rc_formats::moby_anim::evaluate_chains(self.class, &self.a.state, self.a.snapshot.as_ref(), chains)
    }

    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> {
        snapshot(self.class, &self.a.state, self.a.snapshot.as_ref())
    }
}
