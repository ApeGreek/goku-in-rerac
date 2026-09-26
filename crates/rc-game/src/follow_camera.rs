//! The gameplay follow camera, type 0 (level01 `CameraUpdate` 0x20eca8 → `UpdateAllCameras` 0x20d620 →
//! type-0 update 0x314e00; init 0x311f38 + snap 0x311890), producing the `Camera` record the renderer uses
//! (position 0x167240, rows forward/left/up 0x167450/60/70, Euler 0x167250 with the yaw 0x167258 the hero
//! reads). Spec: `docs/plan/player_controller.md` §7 and "In the port".
//!
//! **Exactness.** Every step follows the game's op order on the PS2 FPU/VU model ([`Pf`]); the trig is the
//! game's (VU0 sine polynomial, `FastArcTan`, the FPU `asin` polynomial 0x221728, the quaternion rotation
//! 0x274ac8).
//!
//! **Moby collision.** Every camera line / sphere tests the mobys after the world mesh ([`CamInput::mobys`]; the
//! flags 0x12 / 0x14 / 0x34 / 0x94 / 0x96 / 0xb4 / 4 have no bit 0x1, so the moby pass runs; 0x2 only skips the
//! primitives), with the game's `a2`: Ratchet for GetTarget 0x30f498, the snap lines 0x311890, the ceiling lines
//! 0x310a08, the sphere chain 0x312c08 / re-aim 0x3127f0 and the avoidance line; none for the snap sphere
//! (flags 4) and the raise sphere 0x3101c0; the camera moby 0x167354 for the end sphere 0x3124f0 (that moby,
//! class 0x3ef from `Camera_handleCollWithHero`, is not created by the port: none). The avoidance 0x312ef8 first
//! lists the mobys around the pivot–camera midpoint (`coll_sphere_mobys`) and switches the mesh-less ones off for
//! its queries (or nudges the offset ±1° away from the classes of `NUDGE_CLASSES`); a crate blocking its line gets
//! a hit ([`Camera::hit`]).
//!
//! Not modelled (no data for them in the port): the 15-unit `coll_sphere_mobys` of 0x3111d8 (it only feeds the
//! focus-object auto-yaw), the level's camera pass-through volumes (0x20fdb0: treated as absent), camera switches
//! / blends (a lone type-0 camera), scripted focus and auto-yaw, the shake (its timers are 0 in play) and the
//! Euler pitch/roll (0x2721f0; only the yaw feeds gameplay — pitch and roll are derived here from the rows with the
//! same FastArcTan).
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern, clippy::needless_range_loop)] // FPU compare semantics and op order are spelled out on purpose.

use crate::hero::physics::{
    dot3 as dot, fast_add_rotations as add_rot, fast_cos, fast_sin, fast_subtract_rotations as sub_rot, len3 as len,
    set_len3 as norm, to_f32x3, vadd, vscale, vsub, V0, V4,
};
use crate::collision_query::{coll_line_m, coll_sphere_m, coll_sphere_mobys, CollMoby, CollOutput, MobyScene, MobySource, QueryFlags};
use crate::hero::Hero;
use crate::pad::{fast_arctan, PadState};
use crate::ps2v::Pf;
use rc_formats::collision::Collision;

const K_PI: Pf = Pf::b(0x4049_0fdb);
const HALF_PI: Pf = Pf::b(0x3fc9_0fdb);
/// Yaw rates by option 0x15ede4 (0x162228): 1.0°, 1.3°, 1.6° per tick.
pub const YAW_RATES: [Pf; 3] = [Pf::b(0x3c8e_fa35), Pf::b(0x3cb9_dede), Pf::b(0x3ce4_c388)];
const DEG40: Pf = Pf::b(0x3f32_b8c2);
const DEG15: Pf = Pf::b(0x3e86_0a92);
const DEG70: Pf = Pf::b(0x3f9c_61aa);
const DEG1_75: Pf = Pf::b(0x3cfa_35dd);

/// Camera options (boot static values; a memory-card load may change them).
#[derive(Clone, Copy, Debug)]
pub struct CameraOptions {
    /// 0x15eddc: 1 → pitch input = −ry (stick up raises the camera).
    pub pitch_normal: bool,
    /// 0x15ede0: 1 → yaw input = +rx.
    pub yaw_normal: bool,
    /// 0x15ede4: yaw-rate index into [`YAW_RATES`] (default 1 = 1.3°/tick).
    pub yaw_rate: usize,
    /// 0x15edb4: mirror (left row negated after the update).
    pub mirror: bool,
}

impl Default for CameraOptions {
    fn default() -> Self { CameraOptions { pitch_normal: true, yaw_normal: true, yaw_rate: 1, mirror: false } }
}

/// The `Camera` record the renderer and the hero read (level01 0x167240.., boot 0x187080..).
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraView {
    /// 0x167240: position.
    pub pos: V4,
    /// 0x167250: Euler (x roll, y pitch, z yaw); **yaw 0x167258 = atan2(fwd.y, fwd.x)**.
    pub euler: V4,
    /// 0x167450 / 0x167460 / 0x167470: forward, left, up.
    pub rows: [V4; 3],
}

impl CameraView {
    pub fn yaw(&self) -> Pf { self.euler[2] }
    pub fn pos_f32(&self) -> [f32; 3] { to_f32x3(self.pos) }
    pub fn rows_f32(&self) -> [[f32; 3]; 3] { self.rows.map(to_f32x3) }
}

/// Camera globals G (0x167290..) written by the pre-steps.
#[derive(Clone, Copy, Debug, Default)]
pub struct CamGlobals {
    /// G+0x00: hero position with z springed (k 0.0075, d 0.175; vel G+0x10). Not read by type 0.
    pub hero_s: V4,
    pub hero_s_vel: Pf,
    /// G+0x20: smoothed up; G+0x30 / 0x40: up2 = −gravity, previous; G+0x50: up_s spring velocities.
    pub up_s: V4,
    pub up2: V4,
    pub up2_prev: V4,
    pub up_vel: [Pf; 3],
    /// G+0x60 previous hero position, G+0x70 delta, G+0x80 horizontal move direction (0x167310), G+0x90
    /// vertical part, G+0xa0 / 0xa4 (0x167334) / 0xa8: |delta|, horizontal speed, vertical speed.
    pub prev_hero: V4,
    pub delta: V4,
    pub move_dir: V4,
    pub delta_v: V4,
    pub delta_len: Pf,
    pub h_speed: Pf,
    pub v_speed: Pf,
    /// G+0xac..0xbc: hero yaw history (newest last).
    pub yaw_hist: [Pf; 5],
    /// G+0xd8 / 0xdc: ground-moby z / platform vertical delta (no moby platforms: 0).
    pub ground_z: Pf,
    pub plat_dz: Pf,
    /// 0x167498: ticks since the last camera switch.
    pub since_switch: i32,
}

/// The data block D (0x169590) plus the UpdateCam fields type 0 uses.
#[derive(Clone, Copy, Debug, Default)]
pub struct FollowCamera {
    /// UpdateCam +0x00/0x10/0x20 rows (forward, left, up), +0x30 position, +0x40 saved forward, +0x64 previous position.
    pub rows: [V4; 3],
    pub pos: V4,
    pub saved_fwd: V4,
    pub prev_pos: V4,
    /// D+0x00: desired point pivot + off (next tick's leash).
    pub desired: V4,
    /// D+0x10 (s16): leash enable.
    pub leash: i16,
    /// D+0x20 (s16): row blend timer.
    pub row_blend: i16,
    /// D+0x22 (s16).
    pub pitch_neg: i16,
    /// D+0x24/0x2c smoothed look height / vel; D+0x28/0x30 smoothed pivot height / vel; D+0x34/0x38 bias / vel.
    pub look_h_s: Pf,
    pub look_h_v: Pf,
    pub pivot_h_s: Pf,
    pub pivot_h_v: Pf,
    pub bias: Pf,
    pub bias_v: Pf,
    /// D+0x40 target T, D+0x50 vertical target, D+0x60/0x70 raise endpoints, D+0x80 look, D+0x90 pivot,
    /// D+0xa0 smoothed target S, D+0xb0/0xc0 spring velocities, D+0xd0/0xe0 smoothed look (+vel).
    pub target: V4,
    pub vtarget: V4,
    pub raise_a: V4,
    pub raise_b: V4,
    pub look: V4,
    pub pivot: V4,
    pub smooth: V4,
    pub vel_h: [Pf; 3],
    pub vel_v: [Pf; 3],
    pub look_s: V4,
    pub look_s_vel: [Pf; 3],
    /// D+0xf0 look height (1.5), 0xf4 override target, 0xf8 rate, 0xfc vel.
    pub look_h: Pf,
    pub look_h_tgt: Pf,
    pub look_h_rate: Pf,
    pub look_h_vel: Pf,
    /// D+0x104 / 0x105 target mode / previous; 0x106 (s16) / 0x108 blend timer / 1/length.
    pub mode: u8,
    pub mode_prev: u8,
    pub mode_t: i16,
    pub mode_inv: Pf,
    /// D+0x10c / 0x110 / 0x114 raise lerp 1/length, height, timer; 0x116 / 0x117 flags; 0x118 look blend; 0x11a.
    pub raise_inv: Pf,
    pub raise_h: Pf,
    pub raise_t: i16,
    pub look_from_s: u8,
    pub raise_frozen: u8,
    pub look_blend: i16,
    pub look_h_ovr: i16,
    /// D+0x11c/0x120 horizontal k, d; 0x124/0x128 vertical k, d.
    pub kh: Pf,
    pub dh: Pf,
    pub kv: Pf,
    pub dv: Pf,
    /// D+0x130 desired offset, D+0x140 placed offset.
    pub off: V4,
    pub placed: V4,
    /// D+0x150 / 0x154 / 0x158 placement velocities (yaw, elevation, distance).
    pub yaw_v: Pf,
    pub el_v: Pf,
    pub dist_v: Pf,
    /// D+0x15c distance (4.64), 0x160 pivot height (2.0), 0x164 / 0x168 yaw / pitch step.
    pub dist: Pf,
    pub pivot_h: Pf,
    pub yaw_step: Pf,
    pub pitch_step: Pf,
    /// D+0x16c/0x170/0x174/0x178 distance override flag/target/vel/rate; 0x16e/0x17c/0x180/0x184 pivot height.
    pub dist_ovr: i16,
    pub dist_tgt: Pf,
    pub dist_ovr_v: Pf,
    pub dist_rate: Pf,
    pub ph_ovr: i16,
    pub ph_tgt: Pf,
    pub ph_v: Pf,
    pub ph_rate: Pf,
    /// D+0x188 base distance.
    pub base_dist: Pf,
    /// D+0x1a8/0x1ac smoothed yaw input; 0x1b0/0x1b4 normalised pitch; 0x1b8 script flags; 0x1bc yaw rate;
    /// 0x1c4 scripted yaw; 0x1c8 platform yaw; 0x1cc scripted pitch.
    pub yaw_in: Pf,
    pub yaw_in_v: Pf,
    pub pitch_n: Pf,
    pub pitch_n_v: Pf,
    pub script: u32,
    pub yaw_rate: Pf,
    pub script_yaw: Pf,
    pub plat_yaw: Pf,
    pub script_pitch: Pf,
    /// D+0x1f0 end-sphere candidate; 0x200 distance reduction; 0x204 (s16) recovery timer; 0x208 (s16) end
    /// hit class; 0x20a (s16) fast recovery; 0x20c / 0x210 / 0x214 sphere scale / chain count+1 / radius step;
    /// 0x218 end flags; 0x21c blocked ticks.
    pub end_cand: V4,
    pub red: Pf,
    pub rec_t: i16,
    pub end_class: i16,
    pub fast_rec: i16,
    pub sph_scale: Pf,
    pub sph_count: Pf,
    pub sph_step: Pf,
    pub end_flags: u32,
    pub blocked: i32,
    /// D+0x224 (s16) run-toward timer, 0x226 lock, 0x228 pull-back distance, 0x22c pull-back max (6.0).
    pub toward_t: i16,
    pub toward_lock: i16,
    pub pull: Pf,
    pub pull_max: Pf,
}

/// The whole camera state: globals, the type-0 camera and the defaults snapshot D0 (0x169a90).
#[derive(Clone, Debug, Default)]
pub struct Camera {
    pub g: CamGlobals,
    pub cam: FollowCamera,
    /// D0: copy of D right after init (`BackupCurrentCam`).
    pub d0: FollowCamera,
    pub out: CameraView,
    pub opts: CameraOptions,
    /// Line / avoidance / end-sphere flags (0x15ef58 / 0x15ef5c / 0x15ef60).
    pub line_flags: u32,
    pub sph_flags: u32,
    pub end_flags: u32,
    /// Number of 30-tick blocked resets so far (diagnostics).
    pub resets: u32,
    /// The crate the blocked camera line hit during the last update (0x312ef8): the moby and the hit
    /// template's direction. The tick delivers it (FUN_0026e968).
    pub hit: Option<(usize, V4)>,
}

// ------------------------------------------------------------------------------------------------
// Math (spec §0).

/// `vlerp(a, b, t)` 0x2211e8: `a + (b − a)·t`.
fn vlerp(a: V4, b: V4, t: Pf) -> V4 { vadd(a, vscale(vsub(b, a), t)) }
/// `FastVecCross(out, a, b)` 0x2212d0 = **b × a** (`o.x = b.y·a.z − a.y·b.z`, …).
fn cross(a: V4, b: V4) -> V4 {
    [b[1] * a[2] - a[1] * b[2], b[2] * a[0] - a[2] * b[0], b[0] * a[1] - a[0] * b[1], Pf::ZERO]
}
/// Standard `a × b` (`vopmula a,b; vopmsub b,a`).
fn cross_std(a: V4, b: V4) -> V4 {
    [a[1] * b[2] - b[1] * a[2], a[2] * b[0] - b[2] * a[0], a[0] * b[1] - b[0] * a[1], Pf::ZERO]
}
/// 0x221398: 2-D distance.
fn dist2d(a: V4, b: V4) -> Pf { crate::hero::physics::dist2(a, b) }
fn i2f(i: i32) -> Pf { Pf::from_i32(i) }

/// `Cam_InterpValues(cur, tgt, k, d, max, *vel)` 0x20ce40.
pub fn interp(cur: Pf, tgt: Pf, k: Pf, d: Pf, max: Pf, vel: &mut Pf) -> Pf {
    let e = tgt - cur;
    let a = k * e;
    let b = d * *vel;
    *vel = *vel + (a - b);
    let v = *vel;
    if max != Pf::ZERO {
        if max < v { *vel = max; } else if v < -max { *vel = -max; }
    }
    let ae = e.abs();
    let v = *vel;
    if ae < v { *vel = ae; } else if v < -ae { *vel = -ae; }
    cur + *vel
}

/// `AngInterp` 0x20cf28: the same on wrapped angles.
pub fn ang_interp(cur: Pf, tgt: Pf, k: Pf, d: Pf, max: Pf, vel: &mut Pf) -> Pf {
    let e = sub_rot(tgt, cur);
    let a = k * e;
    let b = d * *vel;
    *vel = *vel + (a - b);
    let v = *vel;
    if max != Pf::ZERO {
        if max < v { *vel = max; } else if v < -max { *vel = -max; }
    }
    let ae = e.abs();
    let v = *vel;
    if ae < v { *vel = ae; } else if v < -ae { *vel = -ae; }
    add_rot(cur, *vel)
}

/// `asin` 0x221728 (FPU polynomial with the VU square root).
pub fn asin(x: Pf) -> Pf {
    const C: [Pf; 4] = [Pf::b(0x3fc9_0da4), Pf::b(0xbe59_3484), Pf::b(0x3d98_1627), Pf::b(0xbc99_6e30)];
    let (mut x, mut sgn) = (x, Pf::ONE);
    if x < Pf::ZERO { x = x.abs(); sgn = Pf::b(0xbf80_0000); }
    let s = Pf::ZERO + (Pf::ONE - x).sqrt();
    let xx = x * x;
    let acc = C[0] + Pf::ZERO;
    let acc = acc + x * C[1];
    let acc = acc + xx * C[2];
    let p = acc + (x * xx) * C[3];
    sgn * (HALF_PI - p * s)
}

/// Quaternion product 0x221d78: xyz = ((b·a.w) + (a·b.w)) + a × b; w = b.w·a.w − ((a.x·b.x + a.y·b.y) + 1·a.z·b.z).
fn qmul(a: V4, b: V4) -> V4 {
    let c = cross_std(a, b);
    let d = (a[0] * b[0] + a[1] * b[1]) + Pf::ONE * (a[2] * b[2]);
    let w = b[3] * a[3];
    [
        (b[0] * a[3] + a[0] * b[3]) + c[0],
        (b[1] * a[3] + a[1] * b[3]) + c[1],
        (b[2] * a[3] + a[2] * b[3]) + c[2],
        w - d,
    ]
}

/// `rot(v, θ, axis)` 0x274ac8: rotate `v` by θ (counter-clockwise) about `axis` with a quaternion.
pub fn rot(v: V4, theta: Pf, axis: V4) -> V4 {
    if theta.abs() < Pf::b(0x3727_c5ac) { return v; }
    let n = norm(axis, Pf::ONE);
    let h = theta * Pf::b(0x3f00_0000);
    let s = fast_sin(h);
    let q = [n[0] * s, n[1] * s, n[2] * s, fast_cos(h)];
    let m = Pf::b(0xbf80_0000);
    let c = [q[0] * m, q[1] * m, q[2] * m, q[3]];
    let p = [v[0], v[1], v[2], Pf::ZERO];
    let r = qmul(qmul(q, p), c);
    [r[0], r[1], r[2], v[3]]
}

/// `CosInterp(a, b, t)` 0x26cc38.
fn cos_interp(a: Pf, b: Pf, t: Pf) -> Pf {
    if t == Pf::ZERO { return a; }
    if t == Pf::ONE { return b; }
    let c = fast_cos(t * K_PI);
    let f1 = (Pf::ONE - c) * Pf::b(0x3f00_0000);
    let f0 = (b - a) * f1;
    a + f0
}

/// `DecTimer` 0x220ea8 (s16): 1 when already 0, 2 when it just reached 0, else 0.
fn dec_timer(t: &mut i16) -> i32 {
    if *t == 0 { return 1; }
    *t = (*t).max(1) - 1;
    if *t > 0 { 0 } else { 2 }
}

fn t(n: i32) -> i32 { n }

/// `CollLine_Fix(a, b, flags, ignore, 0)`: the world mesh, then the mobys of [`CamInput::mobys`] (`ignore` = the
/// `a2` the game passes: Ratchet for every camera line).
fn line_out(inp: &CamInput, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<CollOutput> {
    coll_line_m(inp.coll, inp.mobys, to_f32x3(a), to_f32x3(b), QueryFlags(flags), ignore)
}
/// [`line_out`]'s hit point.
fn line_hit(inp: &CamInput, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<V4> {
    line_out(inp, a, b, flags, ignore).map(|o| crate::hero::physics::from_f32x3(o.point))
}
/// Sphere 0x212960 `(r, c, flags, ignore)`, world mesh then mobys: (closest point +0x20, pushed centre +0x30).
fn sphere(inp: &CamInput, r: Pf, c: V4, flags: u32, ignore: Option<usize>) -> Option<(V4, V4)> {
    use crate::hero::physics::from_f32x3;
    coll_sphere_m(inp.coll, inp.mobys, to_f32x3(c), r.to_f32(), QueryFlags(flags), ignore)
        .map(|o| (from_f32x3(o.point), from_f32x3(o.pushed_centre.unwrap_or(o.point))))
}

/// The avoidance's view of the mobys (0x312ef8): the ones its `coll_sphere_mobys` listed and switched off
/// (`+0x94 = 0` until the end of the avoidance) report no collision.
struct MaskedMobys<'a> {
    inner: &'a dyn MobySource,
    off: &'a [usize],
}

impl MobySource for MaskedMobys<'_> {
    fn moby(&self, id: usize) -> Option<CollMoby> {
        self.inner.moby(id).map(|mut m| {
            if self.off.contains(&id) { m.collision = false; }
            m
        })
    }
    fn joints(&self, id: usize, count: usize) -> Vec<[u32; 4]> { self.inner.joints(id, count) }
}

/// The classes 0x312ef8 turns the offset away from (±1° about up_s) instead of switching them off.
const NUDGE_CLASSES: [i16; 8] = [0x72, 0xb, 0x2c, 0x9a, 0xff, 0x392, 0x452, 0x353];
/// `FUN_00273278(m)`: a moby of classes 500..=540 (the crates); the blocked camera line hits those.
fn crate_class(o_class: i16) -> bool { ((o_class as i32 - 500) as u32 & 0xffff) < 0x29 }

// ------------------------------------------------------------------------------------------------

/// What the camera reads from the hero each tick.
#[derive(Clone, Copy)]
pub struct CamInput<'a> {
    pub hero: &'a Hero,
    pub pad: &'a PadState,
    pub coll: &'a Collision,
    /// The mobys the camera's line / sphere queries test after the world mesh (the moby table as the hero's
    /// write-back 0x229f20 left it, the grid, class blobs, pose cache; None: world only).
    pub mobys: Option<&'a MobyScene<'a>>,
    /// Ratchet's moby `0x1413d0`, the `a2` (ignored moby) of most camera queries.
    pub hero_moby: Option<usize>,
}

impl Camera {
    /// Level start (0x20ef58 + reset 0x20ee80): globals from the hero, type-0 init and snap behind it.
    pub fn new(inp: &CamInput, opts: CameraOptions) -> Camera {
        let mut c = Camera { opts, ..Default::default() };
        let h = inp.hero;
        let up = norm(h.moby_rows[2], Pf::ONE);
        c.g.up_s = up;
        c.g.up2 = up;
        c.g.up2_prev = up;
        c.g.prev_hero = h.pos;
        c.g.ground_z = h.pos[2];
        c.g.yaw_hist = [h.rot[2]; 5];
        c.g.hero_s = h.pos;
        c.cam.yaw_rate = YAW_RATES[opts.yaw_rate.min(2)];
        c.reset(inp);
        c
    }

    /// Reset 0x20ee80: re-run the motion pre-step, init, backup D0, publish the position.
    pub fn reset(&mut self, inp: &CamInput) {
        self.pre_motion(inp);
        self.init(inp);
        self.d0 = self.cam;
        self.out.pos = self.cam.pos;
        self.cam.prev_pos = self.cam.pos;
        self.g.hero_s = inp.hero.pos;
    }

    /// Type-0 init 0x311f38 (not switched in: target reset + snap).
    fn init(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let d = &mut self.cam;
        d.target = h.pos;
        d.look_h = Pf::b(0x3fc0_0000);
        d.mode = 0;
        d.look_from_s = 0;
        d.dh = Pf::b(0x3e4c_cccd);
        d.kv = Pf::b(0x3c75_c28f);
        d.kh = Pf::b(0x3c75_c28f);
        d.dv = Pf::b(0x3e4c_cccd);
        d.look_h_rate = Pf::b(0x3ba3_d70a);
        d.mode_t = 0;
        d.look_blend = 0;
        d.look_h_ovr = 0;
        d.look_h_tgt = Pf::ZERO;
        d.look_h_vel = Pf::ZERO;
        d.ph_rate = Pf::b(0x3b44_9ba6);
        d.pivot_h = Pf::b(0x4000_0000);
        d.base_dist = Pf::b(0x4094_7ae1);
        d.dist = Pf::b(0x4094_7ae1);
        d.dist_rate = Pf::b(0x3b44_9ba6);
        d.yaw_step = Pf::ZERO;
        d.pitch_step = Pf::ZERO;
        d.dist_v = Pf::ZERO;
        d.yaw_v = Pf::ZERO;
        d.el_v = Pf::ZERO;
        d.dist_ovr = 0;
        d.ph_ovr = 0;
        d.dist_ovr_v = Pf::ZERO;
        d.ph_v = Pf::ZERO;
        d.leash = 1;
        d.row_blend = 0;
        d.pivot_h_v = Pf::ZERO;
        d.look_h_v = Pf::ZERO;
        d.pitch_neg = 0;
        d.bias = Pf::ZERO;
        d.bias_v = Pf::ZERO;
        d.yaw_in = Pf::ZERO;
        d.yaw_rate = YAW_RATES[self.opts.yaw_rate.min(2)];
        d.script_pitch = Pf::ZERO;
        d.pitch_n = Pf::ZERO;
        d.yaw_in_v = Pf::ZERO;
        d.pitch_n_v = Pf::ZERO;
        d.script = 0;
        d.script_yaw = Pf::ZERO;
        d.plat_yaw = Pf::ZERO;
        d.toward_t = 0;
        d.toward_lock = 0;
        d.pull = d.dist;
        d.pull_max = Pf::b(0x40c0_0000);
        d.red = Pf::ZERO;
        d.fast_rec = 0;
        d.end_class = 0;
        d.blocked = 0;
        d.end_flags = self.end_flags;
        d.rec_t = t(2000) as i16;
        d.sph_step = Pf::b(0x3dcc_cccd);
        d.sph_scale = Pf::ONE;
        d.sph_count = Pf::b(0x4100_0000);
        self.target_reset(inp);
        self.snap(inp);
        let d = &mut self.cam;
        d.off = vsub(d.pos, d.pivot);
        d.placed = d.off;
        d.desired = d.pos;
        d.end_cand = d.pos;
        let grav = h.gravity_dir;
        let k = dot(d.off, grav);
        let hz = vsub(d.off, vscale(grav, k));
        d.red = d.dist - len(hz);
        if d.red < Pf::b(0xbdcc_cccd) { d.fast_rec = 1; }
        d.pivot_h_s = d.pivot_h;
        d.look_h_s = d.look_h;
    }

    /// GetTarget 0x30f498 (flag 0 / 1): the hero, or the ground under it while airborne.
    fn get_target(&self, inp: &CamInput, flag: bool) -> (V4, i32) {
        let h = inp.hero;
        if h.f65c == 0 { return (h.pos, 1); }
        let ht = h.height;
        let up2 = self.g.up2;
        if !(Pf::b(0x41f0_0000) <= ht) && flag { return (vadd(h.pos, vscale(up2, -ht)), 1); }
        let p0 = vadd(h.pos, vscale(up2, Pf::b(0x3e4c_cccd)));
        let p1 = vadd(h.pos, vscale(up2, Pf::b(0xc1f0_0000)));
        match line_hit(inp, p0, p1, 0x12, inp.hero_moby) {
            Some(p) => (p, 0),
            None => (h.pos, -1),
        }
    }

    /// 0x30f608: target, look and pivot reset.
    fn target_reset(&mut self, inp: &CamInput) {
        let (tg, _) = self.get_target(inp, false);
        let up2 = self.g.up2;
        let up_s = self.g.up_s;
        let d = &mut self.cam;
        let v = vscale(up2, dot(tg, up2));
        d.target = vsub(tg, v);
        d.vtarget = v;
        d.target = vadd(d.target, d.vtarget);
        let a = vscale(up_s, d.look_h);
        d.look = vadd(a, d.target);
        d.look_s = d.look;
        d.look_s_vel = [Pf::ZERO; 3];
        d.mode = 0;
        d.mode_inv = Pf::ONE;
        d.mode_prev = 0;
        d.mode_t = 0;
        d.smooth = d.target;
        d.vel_h = [Pf::ZERO; 3];
        d.vel_v = [Pf::ZERO; 3];
        d.pivot = vadd(vscale(up_s, d.pivot_h), d.target);
    }

    /// Init snap 0x311890(1): behind the hero at the nominal distance and pivot height, pulled in by walls.
    fn snap(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let d = &mut self.cam;
        let lo = [-d.dist, Pf::ZERO, d.pivot_h, Pf::ZERO];
        let r = [h.moby_rows[0], h.moby_rows[1], h.moby_rows[2]];
        let mut pos = vadd(d.target, crate::hero::physics::mul_rows3(&r, lo));
        let look = vadd(d.target, vscale(r[2], d.look_h));
        let mut fl = 0x94;
        if let Some(p) = line_hit(inp, look, pos, 0x94, inp.hero_moby) {
            if dist2d(p, look) < Pf::b(0x3a83_126f) { fl = 0x96; }
        }
        if let Some(hit) = line_hit(inp, look, pos, fl, inp.hero_moby) {
            let w = vsub(hit, look);
            let l = len(w);
            if l == Pf::ZERO {
                pos = d.target;
            } else if l < Pf::b(0x3e4c_cccd) {
                let look2 = vadd(d.target, vscale(r[2], Pf::b(0x3f00_0000)));
                let mut p = look2;
                let mut rad = Pf::b(0x3f00_0000);
                let mut i = 0;
                while i < 3 {
                    if let Some((_, pc)) = sphere(inp, rad, p, 4, None) {
                        p = pc;
                        i += 1;
                        continue;
                    }
                    if i != 0 { break; }
                    rad = rad + Pf::b(0x3e80_0000);
                    i += 1;
                }
                let w = vsub(p, look2);
                let l = len(w);
                if l == Pf::ZERO {
                    pos = d.target;
                } else {
                    pos = vadd(look2, vscale(w, d.dist / l));
                    if let Some(hit) = line_hit(inp, look2, pos, fl, inp.hero_moby) {
                        let w = vsub(hit, p);
                        let l = len(w);
                        pos = if l == Pf::ZERO { d.target } else { vadd(p, vscale(w, (l - Pf::b(0x3f00_0000)) / l)) };
                    }
                }
            } else {
                pos = vadd(look, vscale(w, (l - Pf::b(0x3f00_0000)) / l));
            }
        }
        let up2 = self.g.up2;
        let hz = vsub(pos, d.target);
        let hz = vsub(hz, vscale(up2, dot(hz, up2)));
        if len(hz) < Pf::b(0x3d4c_cccd) {
            // The game picks the signs with a random bit (0x26c930(2)); the port always takes +.
            pos[0] = pos[0] + Pf::b(0x3f00_0000);
            pos[1] = pos[1] + Pf::b(0x3f00_0000);
            pos[2] = pos[2] + Pf::b(0x3f00_0000);
        }
        d.pos = pos;
        d.prev_pos = pos;
        let a = norm(up2, d.look_h);
        let look = vadd(a, d.target);
        let v = vsub(look, pos);
        let fwd = norm(v, Pf::ONE);
        let left = norm(cross(fwd, up2), Pf::ONE);
        let upr = cross(left, fwd);
        d.rows = [fwd, left, upr];
        d.saved_fwd = fwd;
    }

    /// One `CameraUpdate` (0x20eca8) after the hero update. Returns the published view.
    pub fn update(&mut self, inp: &CamInput) -> CameraView {
        self.hit = None;
        self.g.since_switch += 1;
        self.pre_flags(inp);
        self.pre_motion(inp);
        self.update_type0(inp);
        self.cam.prev_pos = self.cam.pos;
        self.out.pos = self.cam.pos;
        self.out.rows = self.cam.rows;
        let f = self.out.rows[0];
        // 0x2721f0: yaw = atan2(fwd.y, fwd.x); pitch / roll from the rows (see the module doc).
        let yaw = fast_arctan(f[0], f[1]);
        let fh = Pf::ZERO + (f[0] * f[0] + f[1] * f[1]).sqrt();
        let pitch = fast_arctan(fh, -f[2]);
        let l = self.out.rows[1];
        let roll = fast_arctan(Pf::ZERO + (l[0] * l[0] + l[1] * l[1]).sqrt(), l[2]);
        self.out.euler = [roll, pitch, yaw, Pf::ZERO];
        if self.opts.mirror { self.out.rows[1] = cross(self.out.rows[2], self.out.rows[0]); }
        self.out
    }

    /// 0x20eb40: collision flags (no hero-side surface flags on land: lines 0xb4, spheres 0x94, end 0x14).
    fn pre_flags(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let mut f = 0x14;
        if h.group == 0x11 || h.group == 0x12 || h.state == 0x73 { f = 0x34; }
        if h.group != 0x11 && h.water_level < self.out.pos[2] { f = 0x14; }
        self.end_flags = f;
        self.sph_flags = f | 0x80;
        self.line_flags = 0xb4;
    }

    /// 0x20e670: up vectors and hero motion.
    fn pre_motion(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let g = &mut self.g;
        let mut u = norm(h.gravity_dir, Pf::b(0xbf80_0000));
        g.up2_prev = g.up2;
        g.up2 = u;
        if dot(g.up_s, u) < Pf::b(0xbf7a_e148) {
            let k = Pf::b(0x3e4c_cccd);
            u[0] = u[0] + k;
            u[1] = u[1] + k;
            u[2] = u[2] + k;
        }
        let (k, dd) = (Pf::b(0x3c75_c28f), Pf::b(0x3e4c_cccd));
        for c in 0..3 { g.up_s[c] = interp(g.up_s[c], u[c], k, dd, Pf::ZERO, &mut g.up_vel[c]); }
        g.up_s = norm(g.up_s, Pf::ONE);
        g.delta = vsub(h.pos, g.prev_hero);
        g.delta_len = len(g.delta);
        g.v_speed = dot(g.delta, u);
        let v = norm(u, g.v_speed);
        g.delta_v = v;
        g.move_dir = vsub(g.delta, v);
        g.h_speed = len(g.move_dir);
        g.move_dir = vscale(g.move_dir, Pf::ONE / g.h_speed);
        g.prev_hero = h.pos;
        g.hero_s[0] = h.pos[0];
        g.hero_s[1] = h.pos[1];
        if !(h.f15d4 == 0x50 && h.state != 0x11) {
            g.hero_s[2] = interp(g.hero_s[2], h.pos[2], Pf::b(0x3bf5_c28f), Pf::b(0x3e33_3333), Pf::ZERO, &mut g.hero_s_vel);
            g.hero_s[3] = h.pos[2];
        }
        g.yaw_hist.rotate_left(1);
        g.yaw_hist[4] = h.rot[2];
        // Moby platforms: none (ground moby always 0).
        g.plat_dz = Pf::ZERO;
        g.ground_z = h.pos[2];
    }

    /// The type-0 update 0x314e00.
    fn update_type0(&mut self, inp: &CamInput) {
        self.platform_carry(inp);
        // 0x3111d8 state tweaks: on foot none fire (the moby query and focus logic need mobys). Under water
        // (group 0x11) the look height is 0.25 (written each tick; the spring-back pulls it toward 1.5 once) and
        // the pivot height aims at 0.5 at rate 0.003 (`0x313690`: +0x17c, +0x184, +0x16e = 1).
        if inp.hero.group == 0x11 {
            let d = &mut self.cam;
            d.look_h = Pf::b(0x3e80_0000);
            d.ph_ovr = 1;
            d.ph_tgt = Pf::b(0x3f00_0000);
            d.ph_rate = Pf::b(0x3b44_9ba6);
        }
        self.targets(inp);
        self.leash();
        let (yi, pi) = self.stick_read(inp);
        self.stick_apply(yi, pi);
        if self.avoidance(inp) { self.resets += 1; }
        self.placement(inp);
        self.rows();
        self.spring_back();
    }

    /// 0x30f890: platform carry (0x13f490) and the rigid follow of states 0xb / 0xc.
    fn platform_carry(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let c = h.plat_applied;
        let d = &mut self.cam;
        d.pos = vadd(d.pos, c);
        d.smooth = vadd(d.smooth, c);
        d.target = vadd(d.target, c);
        d.look_s = vadd(d.look_s, c);
        d.desired = vadd(d.desired, c);
        if h.f658 == 0 && (h.ground_moby.is_none() || self.g.since_switch > t(30)) { d.plat_yaw = h.platform[3]; }
        if h.state == 0xb || h.state == 0xc {
            d.leash = 0;
            let s = vscale(self.g.move_dir, self.g.h_speed);
            let up2 = self.g.up2;
            let ch = vsub(c, vscale(up2, dot(up2, c)));
            let s = vsub(s, ch);
            d.pos = vadd(d.pos, s);
            d.smooth = vadd(d.smooth, s);
            d.target = vadd(d.target, s);
            d.look_s = vadd(d.look_s, s);
            d.desired = vadd(d.desired, s);
        }
    }

    /// Target mode state machine 0x30fb08 (the transitions an on-foot run reaches).
    fn target_mode(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let pdz = self.g.plat_dz;
        let up2 = self.g.up2;
        let up_s = self.g.up_s;
        let d = &mut self.cam;
        let set = |d: &mut FollowCamera, m: u8, k: i32| {
            d.mode_prev = d.mode;
            d.mode = m;
            d.mode_t = k as i16;
            d.mode_inv = if k != 0 { Pf::ONE / i2f(k) } else { Pf::ONE };
        };
        match d.mode {
            0 => {
                if h.group == 2 {
                    set(d, 1, 10);
                } else if h.state == 0xb && pdz == Pf::ZERO {
                    set(d, 6, 30);
                    d.raise_t = t(35) as i16;
                    d.raise_inv = Pf::ONE / Pf::f(35.0);
                    d.raise_frozen = 0;
                    d.raise_h = Pf::ONE;
                    d.raise_b = vscale(up2, dot(up2, vadd(vscale(up_s, Pf::ONE), h.pos)));
                    d.raise_a = d.vtarget;
                } else if h.state == 0xe && pdz == Pf::ZERO {
                    set(d, 5, 30);
                    d.raise_t = t(45) as i16;
                    d.raise_inv = Pf::ONE / Pf::f(45.0);
                    d.raise_frozen = 0;
                    d.raise_h = Pf::b(0x4060_0000);
                    d.raise_b = vscale(up2, dot(up2, vadd(vscale(up_s, d.raise_h), h.pos)));
                    d.raise_a = d.vtarget;
                } else if h.group == 4 && pdz == Pf::ZERO {
                    set(d, 2, 30);
                }
            }
            1 | 2 => {
                if h.group != 2 && h.group != 4 {
                    d.mode_prev = d.mode;
                    d.mode = 0;
                    d.mode_t = 0;
                    d.mode_inv = Pf::ONE;
                }
            }
            6 if h.state != 0xb => set(d, 0, t(20)),
            3 | 5 if h.group != 4 => set(d, 0, t(20)),
            4 if h.group != 4 => set(d, if h.group == 2 { 1 } else { 0 }, t(20)),
            _ => {}
        }
    }

    /// VerticalTarget 0x3101c0: returns the vertical target V and writes the horizontal part to D+0x40.
    fn vertical_target(&mut self, inp: &CamInput) -> V4 {
        let h = inp.hero;
        let up2 = self.g.up2;
        let hero = h.pos;
        let plain = |d: &mut FollowCamera| {
            let v = vscale(up2, dot(hero, up2));
            d.target = vsub(hero, v);
            v
        };
        match self.cam.mode {
            2 => {
                if h.state == 0x11 {
                    self.cam.mode = 1;
                    self.cam.mode_t = 10;
                } else if h.jump.descending != 0 && Pf::b(0x3e4c_cccd) < h.height {
                    let a = self.view_angle(h);
                    if a < Pf::b(0xbc23_d70a) || HALF_PI < a {
                        self.cam.mode = 1;
                        self.cam.mode_t = 10;
                    }
                }
                let mut hold = false;
                if h.f65c != 0 && self.g.plat_dz == Pf::ZERO && (h.jump.descending == 0 || h.jump.land_eta >= 16) {
                    hold = true;
                } else {
                    let (g, r) = self.get_target(inp, true);
                    if r != 1 || dot(g, up2) < len(self.cam.vtarget) { hold = true; }
                }
                if !hold {
                    let (g, _) = self.get_target(inp, true);
                    let v = vscale(up2, dot(g, up2));
                    self.cam.target = vsub(g, v);
                    return v;
                }
                let p = self.g.up2_prev;
                if (0..3).any(|c| Pf::b(0x3a83_126f) < (up2[c] - p[c]).abs()) { return plain(&mut self.cam); }
                let v = self.cam.vtarget;
                self.cam.target = vsub(hero, vscale(up2, dot(hero, up2)));
                v
            }
            3 | 5 | 6 | 7 => {
                let d = &mut self.cam;
                d.vtarget = vlerp(d.raise_b, d.raise_a, i2f(d.raise_t as i32) * d.raise_inv);
                if d.raise_frozen == 0 && sphere(inp, Pf::b(0x3e4c_cccd), d.pos, 0x12, None).is_some() {
                    d.raise_b = d.vtarget;
                    d.raise_frozen = 1;
                }
                let v = d.vtarget;
                d.target = vsub(hero, vscale(up2, dot(hero, up2)));
                if dec_timer(&mut d.raise_t) != 0 {
                    if d.mode == 6 {
                        d.mode = 7;
                        d.raise_t = t(35) as i16;
                        d.raise_b = d.raise_a;
                        d.raise_a = d.vtarget;
                    } else {
                        d.mode = 4;
                        d.mode_t = 20;
                    }
                }
                v
            }
            8 => {
                let v = self.cam.vtarget;
                self.cam.target = vsub(hero, vscale(up2, dot(hero, up2)));
                v
            }
            _ => plain(&mut self.cam),
        }
    }

    /// 0x30f758: signed angle between the view and the hero's look point (left component removed).
    fn view_angle(&self, h: &Hero) -> Pf {
        let d = &self.cam;
        let p = vadd(h.pos, vscale(self.g.up_s, d.look_h));
        let v = vsub(p, d.pos);
        let left = d.rows[1];
        let v = vsub(v, vscale(left, dot(v, left)));
        let l = len(v);
        if l == Pf::ZERO { return Pf::ZERO; }
        let c = dot(v, d.rows[0]) / l;
        let a = HALF_PI - asin(c);
        if dot(v, d.rows[2]) < Pf::ZERO { -a } else { a }
    }

    /// 0x310a08: targets, look point, pivot and the ceiling lines.
    fn targets(&mut self, inp: &CamInput) {
        let h = inp.hero;
        let hp = h.pos;
        self.target_mode(inp);
        let v = self.vertical_target(inp);
        self.cam.vtarget = v;
        let up2 = self.g.up2;
        let up_s = self.g.up_s;
        let d = &mut self.cam;
        let mut sv = vscale(up2, dot(up2, d.smooth));
        let mut sh = vsub(d.smooth, sv);
        for c in 0..3 { sv[c] = interp(sv[c], d.vtarget[c], d.kv, d.dv, Pf::ZERO, &mut d.vel_v[c]); }
        for c in 0..3 { sh[c] = interp(sh[c], d.target[c], d.kh, d.dh, Pf::ZERO, &mut d.vel_h[c]); }
        d.target = vadd(d.target, d.vtarget);
        d.smooth = vadd(sh, sv);
        let lo = vscale(up_s, d.look_h);
        if d.look_from_s == 0 {
            dec_timer(&mut d.look_blend);
            if d.look_blend < 0 { d.look_blend = 0; }
        } else {
            d.look_blend += 1;
            if d.look_blend > t(90) as i16 { d.look_blend = t(90) as i16; }
        }
        let tt = i2f(d.look_blend as i32) / i2f(t(90));
        d.look = vadd(lo, vlerp(d.target, d.smooth, tt));
        // LookSmooth 0x310908 (D+0xd0; not read by the rows).
        if dec_timer(&mut d.mode_t) != 0 { d.mode_prev = d.mode; }
        let tm = i2f(d.mode_t as i32) * d.mode_inv;
        let tgt = vadd(lo, if d.mode == 1 { hp } else { d.target });
        let (k, dd) = if matches!(d.mode, 1 | 3 | 5 | 6 | 7) {
            (Pf::b(0x3d75_c28f) + (Pf::b(0x3c23_d70a) - Pf::b(0x3d75_c28f)) * tm, Pf::b(0x3ecc_cccd) + (Pf::b(0x3e4c_cccd) - Pf::b(0x3ecc_cccd)) * tm)
        } else {
            (Pf::b(0x3c23_d70a), Pf::b(0x3e4c_cccd))
        };
        for c in 0..3 { d.look_s[c] = interp(d.look_s[c], tgt[c], k, dd, Pf::ZERO, &mut d.look_s_vel[c]); }
        // Pivot and ceiling.
        let b = match h.mode { 2 => Pf::b(0x4040_0000), 1 => Pf::b(0x3ecc_cccd), _ => Pf::b(0x3f00_0000) };
        let base = vadd(hp, vscale(up_s, b));
        d.pivot = vadd(vscale(up_s, d.pivot_h), d.target);
        let hh = Pf::b(0x3e80_0000) + (d.sph_step * d.sph_scale) * (d.sph_count - Pf::ONE);
        let twenty = Pf::b(0x41a0_0000);
        let mut cc = vadd(base, vscale(up_s, twenty));
        if let Some(p) = line_hit(inp, base, cc, self.line_flags, inp.hero_moby) { cc = p; }
        let mut ff = vsub(base, vscale(up_s, twenty));
        if let Some(p) = line_hit(inp, base, ff, self.line_flags, inp.hero_moby) { ff = p; }
        let p = if len(vsub(ff, cc)) < hh + hh {
            vscale(vadd(cc, ff), Pf::b(0x3f00_0000))
        } else if len(vsub(cc, d.target)) <= d.pivot_h + hh {
            vsub(cc, vscale(up_s, hh))
        } else {
            if dot(ff, up2) < dot(d.target, up2) { ff = d.target; }
            d.pivot = vadd(vscale(up_s, d.pivot_h), ff);
            return;
        };
        d.pivot = p;
        d.ph_tgt = dot(vsub(p, d.target), up_s);
        d.ph_rate = Pf::b(0x3b44_9ba6);
        d.ph_ovr = 1;
    }

    /// Leash 0x314118.
    fn leash(&mut self) {
        let d = &mut self.cam;
        if d.leash <= 0 { return; }
        let w = vsub(d.desired, d.pivot);
        d.off = vlerp(d.off, w, Pf::b(0x3f40_0000));
        let mut l = len(d.off);
        if d.dist < l {
            l = d.dist;
            d.off = norm(d.off, l);
        } else {
            let e = d.dist - d.red;
            if l < e {
                l = e;
                d.off = norm(d.off, l);
            }
        }
        d.red = d.dist - l;
    }

    /// Stick read 0x313b88 → (yawIn, pitchIn).
    fn stick_read(&mut self, inp: &CamInput) -> (Pf, Pf) {
        let (rx, ry) = (inp.pad.rx, inp.pad.ry);
        let d = &mut self.cam;
        let mut yi = if d.script & 1 != 0 { Pf::ZERO } else if self.opts.yaw_normal { rx } else { -rx };
        if yi == Pf::ZERO {
            if d.script_yaw != Pf::ZERO { yi = d.script_yaw; }
        } else {
            d.yaw_rate = YAW_RATES[self.opts.yaw_rate.min(2)];
        }
        let pi = if d.script & 2 != 0 {
            d.script_pitch / DEG40
        } else {
            let p = if self.opts.pitch_normal { -ry } else { ry };
            if p == Pf::ZERO { d.script_pitch / DEG40 } else { p }
        };
        (yi, pi)
    }

    /// Stick apply 0x313d90.
    fn stick_apply(&mut self, yi: Pf, pi: Pf) {
        let up_s = self.g.up_s;
        let d = &mut self.cam;
        d.yaw_in = interp(d.yaw_in, yi, Pf::ONE, Pf::ONE, Pf::ZERO, &mut d.yaw_in_v);
        d.yaw_step = d.yaw_rate * d.yaw_in;
        let k = Pf::b(0x3fb6_db6e);
        let p = if pi < Pf::b(0xbe99_999a) { (pi + Pf::b(0x3e99_999a)) * k } else if Pf::b(0x3e99_999a) < pi { (pi - Pf::b(0x3e99_999a)) * k } else { Pf::ZERO };
        let cur = d.pitch_n;
        let same_side_inner = p.abs() < cur.abs() && ((p < Pf::ZERO && cur < Pf::ZERO) || (Pf::ZERO < p && Pf::ZERO < cur));
        let rate = if p == Pf::ZERO || same_side_inner { Pf::b(0x3c23_d70a) } else { Pf::b(0x3ca3_d70a) };
        d.pitch_n = interp(cur, p, Pf::ONE, Pf::ONE, rate, &mut d.pitch_n_v);
        let tgt = d.pitch_n * DEG40;
        d.off = rot(d.off, if yi == Pf::ZERO { d.plat_yaw } else { d.yaw_step }, up_s);
        let e = dot(d.off, up_s);
        let l = len(d.off);
        let ax = norm(cross(up_s, d.off), Pf::ONE);
        let el = fast_arctan(l, e);
        if el.abs() < tgt.abs() && rate == Pf::b(0x3c23_d70a) { d.pitch_n = el / DEG40; }
        if el < tgt {
            let s = add_rot(el, DEG1_75);
            d.pitch_step = if tgt < s { sub_rot(tgt, el) } else { DEG1_75 };
        } else if tgt < el {
            if d.end_class < 0 {
                d.pitch_step = Pf::ZERO;
            } else {
                let s = add_rot(el, -DEG1_75);
                d.pitch_step = if s < tgt { sub_rot(tgt, el) } else { -DEG1_75 };
            }
        } else {
            d.pitch_step = Pf::ZERO;
        }
        if tgt < Pf::ZERO { d.pitch_neg = 1; }
        d.off = rot(d.off, d.pitch_step, ax);
    }

    /// Avoidance 0x312ef8. Returns true when the 30-tick blocked reset fired.
    fn avoidance(&mut self, inp: &CamInput) -> bool {
        let h = inp.hero;
        // coll_sphere_mobys(((dist − red) + 1.5)·0.5, pivot + (0x167240 − pivot)·0.5, 1, Ratchet): the listed mobys
        // with collision but no triangle mesh (FUN_0026e7b0: blob +8 = 0) are switched off (+0x94 = 0) for the rest
        // of the avoidance, reset included, and restored at its end; the NUDGE_CLASSES instead turn the offset.
        // (Level 15 also switches off meshed ones while 0x1413f4 == 2: not modelled, not reachable on Novalis.)
        let mut off_ids: Vec<usize> = Vec::new();
        if let Some(sc) = inp.mobys {
            let d = &self.cam;
            let r = ((d.dist - d.red) + Pf::b(0x3fc0_0000)) * Pf::b(0x3f00_0000);
            let c = vlerp(d.pivot, self.out.pos, Pf::b(0x3f00_0000));
            for id in coll_sphere_mobys(sc, to_f32x3(c), r.to_f32(), QueryFlags(1), inp.hero_moby) {
                let Some(m) = sc.mobys.moby(id) else { continue };
                if !m.collision || sc.classes.get(&m.o_class).is_some_and(|b| !b.faces.is_empty()) { continue; }
                if NUDGE_CLASSES.contains(&m.o_class) { self.nudge(m.position.map(Pf)) } else { off_ids.push(id) }
            }
        }
        let masked = inp.mobys.map(|sc| MaskedMobys { inner: sc.mobys, off: &off_ids });
        let masked_scene = inp.mobys.zip(masked.as_ref()).map(|(sc, m)| MobyScene { mobys: m, grid: sc.grid, classes: sc.classes, cache: sc.cache });
        let inp = &CamInput { mobys: masked_scene.as_ref(), ..*inp };
        let mut blocked = false;
        if h.state != 0x7f && self.sphere_chain(inp) { blocked = true; }
        if self.end_sphere(inp) { blocked = true; }
        let d = &mut self.cam;
        if d.dist - d.red < Pf::b(0x3fc0_0000) {
            d.red = d.dist - Pf::b(0x3fc0_0000);
            d.rec_t = t(2000) as i16;
        }
        let t2 = t(2000) as i16;
        let (rx, ry) = (inp.pad.rx, inp.pad.ry);
        let rec = if d.fast_rec != 0 {
            true
        } else if blocked {
            false
        } else if 0 < d.rec_t && d.rec_t < t2 {
            true
        } else {
            rx < Pf::b(0x3e99_999a) && ry < Pf::b(0x3e99_999a)
        };
        if rec {
            if dec_timer(&mut d.rec_t) != 0 { d.fast_rec = 0; }
            if d.fast_rec != 0 && dec_timer(&mut d.rec_t) != 0 { d.fast_rec = 0; }
            d.red = cos_interp(Pf::ZERO, d.red, i2f(d.rec_t as i32) / i2f(t2 as i32));
        } else if t2 < d.rec_t {
            dec_timer(&mut d.rec_t);
        } else {
            d.rec_t = t2;
        }
        d.off = norm(d.off, d.dist - d.red);
        let e = vadd(d.off, d.pivot);
        if h.state != 0x7f {
            if let Some(o) = line_out(inp, d.pivot, e, self.line_flags, inp.hero_moby) {
                // InVolume (pass-through volumes): none in the port.
                // A crate in the way (FUN_00273278: classes 500..=540) is hit: FUN_0026e808(20, tmpl, Ratchet,
                // 0x800000, dir) → 0x26e968, dir = the horizontal unit vector camera 0x167240 → crate, w 5627.9
                // (delivered by the tick, [`Camera::hit`]).
                if let Some(m) = o.moby.and_then(|id| inp.mobys?.mobys.moby(id).map(|m| (id, m))).filter(|(_, m)| crate_class(m.o_class)) {
                    let p = m.1.position.map(Pf);
                    let mut dir = vsub([p[0], p[1], p[2], Pf::ZERO], self.out.pos);
                    dir[2] = Pf::ZERO;
                    dir = norm(dir, Pf::ONE);
                    dir[3] = Pf::b(0x45af_df66);
                    self.hit = Some((m.0, dir));
                }
                d.blocked += 1;
                if d.blocked >= t(30) {
                    self.reset(inp);
                    return true;
                }
            } else {
                d.blocked = 0;
            }
        } else {
            d.blocked = 0;
        }
        false
    }

    /// 0x312ef8's turn away from a listed moby of a NUDGE_CLASSES class at `mpos`: when the offset is within
    /// 25° of the moby's direction from the pivot (in the plane of up2), ±1° about up_s, away from it.
    fn nudge(&mut self, mpos: [Pf; 3]) {
        let (up2, up_s) = (self.g.up2, self.g.up_s);
        let d = &mut self.cam;
        let a = vsub([mpos[0], mpos[1], mpos[2], Pf::ZERO], d.pivot);
        let b = vscale(up2, dot(up2, a));
        let c = norm(vsub(a, b), Pf::ONE);
        let x = asin(dot(c, d.off) / (d.dist - d.red));
        let x = (HALF_PI - x).abs();
        if x < Pf::b(0x3edf_66f3) {
            let n = norm(cross(d.off, up2), Pf::ONE);
            d.off = rot(d.off, if Pf::ZERO < dot(n, a) { Pf::b(0xbc8e_fa35) } else { Pf::b(0x3c8e_fa35) }, up_s);
        }
    }

    /// SphereChain 0x312c08: the 6 spheres along pivot → offset.
    fn sphere_chain(&mut self, inp: &CamInput) -> bool {
        let rx = inp.pad.rx;
        let up2 = self.g.up2;
        let mode = inp.hero.mode;
        let flags = self.sph_flags;
        let mut p = self.cam.pivot;
        let mut dir = norm(self.cam.off, Pf::ONE);
        let mut r = Pf::b(0x3e80_0000);
        let mut acc = Pf::ZERO;
        let mut blocked = false;
        let n = (self.cam.sph_count - Pf::ONE).to_i32();
        for i in 0..n {
            if i > 0 {
                if let Some((hit, _pc)) = sphere(inp, r, p, flags, inp.hero_moby) {
                    let behind = i == 1 && dot(vsub(hit, p), self.cam.off) < Pf::ZERO;
                    if !behind {
                        let q = p;
                        if self.reaim(inp, acc, r, &mut p) {
                            blocked = true;
                            let d = &mut self.cam;
                            if mode != 2 && Pf::b(0x3d4c_cccd) < rx.abs() {
                                let u = norm(vsub(hit, q), Pf::ONE);
                                if dot(up2, u).abs() < Pf::b(0x3f40_0000) {
                                    d.rec_t = 0x884;
                                    d.red = d.red + Pf::b(0x3d99_999a) * Pf::ONE;
                                    if d.dist - d.red < Pf::b(0x3e4c_cccd) { d.red = d.dist - Pf::b(0x3e4c_cccd); }
                                }
                            }
                            d.off = norm(vsub(p, d.pivot), d.dist - d.red);
                            dir = norm(d.off, Pf::ONE);
                        }
                    }
                }
            }
            let d = &self.cam;
            let s = (d.dist - d.red) / d.dist;
            r = r + d.sph_step * d.sph_scale;
            acc = acc + r;
            p = vadd(p, vscale(dir, r * s));
        }
        blocked
    }

    /// Reaim 0x3127f0: push the sphere centre so the pivot → camera segment clears the hit point.
    fn reaim(&mut self, inp: &CamInput, acc: Pf, r: Pf, p: &mut V4) -> bool {
        let d = &self.cam;
        let hh = Pf::b(0x3e80_0000) + (d.sph_step * d.sph_scale) * (d.sph_count - Pf::ONE);
        let p0 = *p;
        let a = d.pivot;
        let bp = vadd(a, d.off);
        let eff = d.dist - d.red;
        let Some((mut hit, mut pc)) = sphere(inp, r, p0, self.sph_flags, inp.hero_moby) else { return false };
        let mut disp = V0;
        let mut ret = false;
        for pass in 0..3 {
            if pass > 0 {
                match sphere(inp, r, pc, self.sph_flags, inp.hero_moby) {
                    Some((h2, pc2)) => {
                        hit = vadd(h2, vsub(p0, pc));
                        pc = pc2;
                    }
                    None => break,
                }
            }
            let (dd, c) = seg_dist(hit, a, bp);
            let tt = len(vsub(a, c));
            let allow = (Pf::b(0x3e80_0000) + (hh - Pf::b(0x3e80_0000)) * (tt / eff)) * Pf::b(0x3f40_0000);
            if !(dd < allow) { break; }
            ret = true;
            let c2 = vadd(c, norm(vsub(c, hit), allow - dd));
            let x = vadd(a, norm(vsub(c2, a), acc));
            let push = vsub(x, p0);
            disp = if pass == 0 { push } else { vadd(disp, push) };
        }
        *p = vadd(p0, disp);
        ret
    }

    /// EndSphere 0x3124f0 + 0x312310.
    fn end_sphere(&mut self, inp: &CamInput) -> bool {
        let up2 = self.g.up2;
        let group = inp.hero.group;
        let mode = inp.hero.mode;
        let rx = inp.pad.rx;
        let d = &mut self.cam;
        let mut e = vadd(d.pivot, d.off);
        d.end_class = 2;
        let hh = Pf::b(0x3e80_0000) + (d.sph_step * d.sph_scale) * (d.sph_count - Pf::ONE);
        let mut hit = false;
        let vol = false;
        for _ in 0..3 {
            // a2 = the camera moby 0x167354 (`Camera_handleCollWithHero`), which the port does not create: none.
            let Some((pt, pc)) = sphere(inp, hh, e, d.end_flags, None) else { break };
            hit = true;
            if d.end_class != 0 {
                let n = norm(vsub(pt, e), Pf::ONE);
                let c = dot(up2, n);
                d.end_class = if Pf::b(0x3f40_0000) < c { 1 } else if c < Pf::b(0xbf40_0000) { -1 } else { 0 };
            }
            e = pc;
        }
        let l0 = len(d.off);
        d.off = vsub(e, d.pivot);
        let ret;
        if d.end_class != 0 {
            d.off = norm(d.off, d.dist - d.red);
            ret = hit;
        } else {
            let mut l = len(d.off);
            if mode != 2 && Pf::b(0x3d4c_cccd) < rx.abs() && hit {
                l = l - Pf::b(0x3d99_999a) * Pf::ONE;
                d.rec_t = 0x884;
            }
            if !vol && group != 2 && l0 < l {
                d.off = vscale(d.off, l0 / l);
                d.red = d.dist - l0;
                ret = hit;
            } else {
                if l < Pf::b(0x3e4c_cccd) {
                    d.off = if l <= Pf::b(0x38d1_b717) { norm(inp.hero.moby_rows[0], Pf::b(0xbe4c_cccd)) } else { vscale(d.off, Pf::b(0x3e4c_cccd) / l) };
                    l = Pf::b(0x3e4c_cccd);
                }
                d.red = d.dist - l;
                ret = true;
            }
        }
        d.end_cand = vadd(d.pivot, d.off);
        if d.end_class == 2 { d.end_class = 0; }
        ret
    }

    /// Placement springs and the final position 0x3148f8.
    fn placement(&mut self, inp: &CamInput) {
        let up_s = self.g.up_s;
        let up2 = self.g.up2;
        let (k, dd) = self.dist_rate(inp);
        let d = &mut self.cam;
        d.desired = vadd(d.pivot, d.off);
        let ddist = d.dist - d.red;
        let l = len(d.placed);
        let ln = interp(l, ddist, k, dd, Pf::ZERO, &mut d.dist_v);
        let e1 = dot(d.off, up_s);
        let el_d = asin(e1 / ddist);
        let e2 = dot(d.placed, up_s);
        let el_p = asin(e2 / ln);
        let el = ang_interp(el_p, el_d, Pf::b(0x3c75_c28f), Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.el_v);
        let oh = vsub(d.off, vscale(up_s, e1));
        let ph = vsub(d.placed, vscale(up_s, e2));
        let m = len(oh) * len(ph);
        let a = HALF_PI - asin(dot(oh, ph) / m);
        let mut ys = ang_interp(Pf::ZERO, a, Pf::b(0x3c75_c28f), Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.yaw_v);
        let s = dot(cross(ph, up_s), oh);
        ys = ys * if Pf::ZERO <= s { Pf::ONE } else { Pf::b(0xbf80_0000) };
        let v = rot(ph, ys, up_s);
        let v = rot(v, el, cross(up_s, v));
        d.placed = norm(v, ln);
        let base = vadd(vscale(up_s, d.pivot_h), d.smooth);
        let c = dot(d.placed, up2);
        let l2 = len(d.placed);
        if l2 != Pf::ZERO {
            let ang = HALF_PI - asin(c / l2);
            if ang.abs() < DEG15 {
                d.placed = rot(d.placed, sub_rot(DEG15, ang.abs()), cross(d.placed, up2));
                d.el_v = Pf::ZERO;
                d.dist_v = Pf::ZERO;
                d.yaw_v = Pf::ZERO;
            }
        }
        d.pos = vadd(base, d.placed);
    }

    /// DistRate 0x3146f0: the distance spring constants (faster, and pulled back, when the hero runs toward
    /// the camera).
    fn dist_rate(&mut self, inp: &CamInput) -> (Pf, Pf) {
        let g = &self.g;
        let d = &mut self.cam;
        let k14 = Pf::b(0x4164_923a);
        let mut f: Option<Pf> = None;
        if d.toward_lock == 0 {
            let lim = Pf::b(0xbe99_999a);
            if d.toward_t != 0 && g.h_speed == Pf::ZERO && dot(inp.hero.moby_rows[0], d.rows[0]) <= lim {
                f = Some((g.h_speed * k14).min(Pf::ONE));
            } else if Pf::b(0x38d1_b717) < g.h_speed.abs() && dot(g.move_dir, d.rows[0]) <= lim {
                let x = g.h_speed * k14;
                f = Some(if Pf::ONE < x { Pf::ONE } else { x });
            }
        }
        match f {
            Some(f) => {
                d.toward_t = t(120) as i16;
                let far = if inp.hero.mode == 1 { Pf::b(0x4080_0000) } else { d.pull_max };
                let x = d.base_dist + (far - d.base_dist) * f;
                if d.pull < x { d.pull = x; }
                d.dist_tgt = d.pull;
                d.dist_rate = Pf::b(0x3b44_9ba6);
                d.dist_ovr = 1;
                (Pf::b(0x3ca3_d70a) + (Pf::b(0x3d23_d70a) - Pf::b(0x3ca3_d70a)) * f, Pf::b(0x3e4c_cccd) + (Pf::b(0x3e99_999a) - Pf::b(0x3e4c_cccd)) * f)
            }
            None => {
                if d.toward_t != 0 {
                    d.toward_t = 0;
                    d.pull = d.base_dist;
                }
                (Pf::b(0x3ca3_d70a), Pf::b(0x3e4c_cccd))
            }
        }
    }

    /// Rows 0x3141e8.
    fn rows(&mut self) {
        let up_s = self.g.up_s;
        let d = &mut self.cam;
        let mut fwd = d.rows[0];
        let lh = vsub(d.look, vscale(up_s, dot(d.look, up_s)));
        let ph = vsub(d.pos, vscale(up_s, dot(d.pos, up_s)));
        let hv = vsub(lh, ph);
        let l = len(hv);
        if Pf::b(0x3d4c_cccd) <= l { fwd = vscale(hv, Pf::ONE / l); }
        let left = norm(cross(fwd, up_s), Pf::ONE);
        d.pivot_h_s = interp(d.pivot_h_s, d.pivot_h, Pf::b(0x3b83_126f), Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.pivot_h_v);
        d.look_h_s = interp(d.look_h_s, d.look_h, Pf::b(0x3b83_126f), Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.look_h_v);
        let dh = d.pivot_h_s - d.look_h_s;
        let e = dot(d.placed, up_s);
        let a = vsub(vscale(up_s, -e), hv);
        let w = vsub(d.pos, a);
        let w = vsub(w, vscale(up_s, -dh));
        let v = vsub(w, d.pos);
        let lv = len(v);
        if lv != Pf::ZERO {
            let mut pit = HALF_PI - asin(dot(fwd, v) / lv);
            let up = cross(left, fwd);
            d.rows[2] = up;
            if dot(up, v) < Pf::ZERO { pit = -pit; }
            let el = fast_arctan(len(d.off), dot(d.off, up_s));
            let mut q = el / DEG40;
            let mut tb = Pf::ZERO;
            if q < Pf::b(0xbdcc_cccd) {
                q = -q;
                if Pf::b(0x3f00_0000) < q { q = Pf::ONE - q; }
                tb = (q + q) * Pf::b(0x3e86_0a92) + Pf::ZERO;
            }
            d.bias = ang_interp(d.bias, tb, Pf::b(0x3ba3_d70a), Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.bias_v);
            let mut p = sub_rot(pit, d.bias);
            if DEG70 < p { p = DEG70; } else if p < -DEG70 { p = -DEG70; }
            fwd = norm(rot(fwd, p, left), Pf::ONE);
        }
        if d.row_blend != 0 {
            dec_timer(&mut d.row_blend);
            fwd = norm(vlerp(fwd, d.saved_fwd, i2f(d.row_blend as i32) / i2f(t(90))), Pf::ONE);
        }
        let left = norm(cross(fwd, up_s), Pf::ONE);
        let up = cross(left, fwd);
        d.rows = [fwd, left, up];
    }

    /// Parameter spring-back and resets 0x311010.
    fn spring_back(&mut self) {
        let d0 = self.d0;
        let end_flags = self.end_flags;
        let d = &mut self.cam;
        d.yaw_rate = YAW_RATES[self.opts.yaw_rate.min(2)];
        d.script_yaw = Pf::ZERO;
        d.plat_yaw = Pf::ZERO;
        d.script = 0;
        d.script_pitch = Pf::ZERO;
        let tgt = if d.dist_ovr != 0 { d.dist_tgt } else { d0.dist };
        d.dist = interp(d.dist, tgt, d.dist_rate, Pf::b(0x3e4c_cccd), d.script_yaw, &mut d.dist_ovr_v);
        let tgt = if d.ph_ovr != 0 { d.ph_tgt } else { d0.pivot_h };
        d.pivot_h = interp(d.pivot_h, tgt, d.ph_rate, Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.ph_v);
        let tgt = if d.look_h_ovr != 0 { d.look_h_tgt } else { d0.look_h };
        d.look_h = interp(d.look_h, tgt, d.look_h_rate, Pf::b(0x3e4c_cccd), Pf::ZERO, &mut d.look_h_vel);
        d.dist_ovr = 0;
        d.ph_ovr = 0;
        d.dist_tgt = Pf::ZERO;
        d.ph_tgt = Pf::ZERO;
        d.base_dist = d0.dist;
        d.look_h_ovr = 0;
        d.look_h_tgt = Pf::ZERO;
        d.leash = 1;
        d.pitch_neg = 0;
        d.look_from_s = 0;
        d.kv = Pf::b(0x3c75_c28f);
        d.dv = Pf::b(0x3e4c_cccd);
        d.kh = Pf::b(0x3c75_c28f);
        d.dh = Pf::b(0x3e4c_cccd);
        d.sph_step = Pf::b(0x3dcc_cccd);
        d.end_flags = end_flags;
        d.sph_scale = Pf::ONE;
        d.sph_count = Pf::b(0x4100_0000);
        d.pull_max = Pf::b(0x40c0_0000);
        d.toward_lock = 0;
    }
}

/// SegDist 0x272b98: distance from `q` to the segment a–b and the closest point.
fn seg_dist(q: V4, a: V4, b: V4) -> (Pf, V4) {
    let u = norm(vsub(b, a), Pf::ONE);
    let tt = -((-dot(q, u)) + dot(a, u)) / dot(u, u);
    let mut c = vadd(a, vscale(u, tt));
    let outside = (0..3).any(|k| (a[k] < c[k] && b[k] < c[k]) || (c[k] < a[k] && c[k] < b[k]));
    if outside { c = if len(vsub(q, a)) <= len(vsub(q, b)) { a } else { b }; }
    (len(vsub(q, c)), c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asin_and_rot() {
        for &x in &[0.0f32, 0.3, -0.5, 0.9, 1.0] {
            assert!((asin(Pf::f(x)).to_f32() - x.asin()).abs() < 1e-4, "asin {x}");
        }
        let v = [Pf::ONE, Pf::ZERO, Pf::ZERO, Pf::ZERO];
        let z = [Pf::ZERO, Pf::ZERO, Pf::ONE, Pf::ZERO];
        let r = rot(v, HALF_PI, z);
        assert!(r[0].to_f32().abs() < 1e-5 && (r[1].to_f32() - 1.0).abs() < 1e-5, "{r:?}");
        let c = cross(v, z); // z × x = y
        assert_eq!(c[1], Pf::ONE);
    }

    fn setup() -> (Collision, crate::hero::Hero, PadState) {
        let coll = crate::hero::testkit::floor(100.0, 98, 108, 98, 108);
        let hero = crate::hero::Hero::spawn([410.0, 410.0, 100.0], 0.0);
        (coll, hero, PadState::default())
    }

    /// Init snap 0x311890 behind a hero facing +x: pos = hero + (−4.64, 0, 2.0), looking at hero + 1.5 up.
    #[test]
    fn init_snap_behind_the_hero() {
        let (coll, hero, pad) = setup();
        let cam = Camera::new(&CamInput { hero: &hero, pad: &pad, coll: &coll, mobys: None, hero_moby: None }, CameraOptions::default());
        let p = cam.cam.pos.map(Pf::to_f32);
        assert!((p[0] - (410.0 - 4.64)).abs() < 1e-4 && p[1] == 410.0 && p[2] == 102.0, "{p:?}");
        let f = cam.cam.rows[0].map(Pf::to_f32);
        let e = [4.64f32, 0.0, -0.5];
        let l = (e[0] * e[0] + e[2] * e[2]).sqrt();
        assert!((f[0] - e[0] / l).abs() < 1e-5 && (f[2] - e[2] / l).abs() < 1e-5, "{f:?}");
    }

    /// Standing still, the camera stays put; full right stick turns the desired offset by 1.3° per tick
    /// (13° over 10 ticks) counter-clockwise, and the placed offset lags behind it.
    #[test]
    fn right_stick_yaw() {
        let (coll, hero, mut pad) = setup();
        let mut cam = Camera::new(&CamInput { hero: &hero, pad: &pad, coll: &coll, mobys: None, hero_moby: None }, CameraOptions::default());
        for _ in 0..30 { pad.update(Some(&crate::pad::PadInput::neutral().bytes()), false); cam.update(&CamInput { hero: &hero, pad: &pad, coll: &coll, mobys: None, hero_moby: None }); }
        let p0 = cam.out.pos_f32();
        assert!((p0[0] - (410.0 - 4.64)).abs() < 1e-3 && (p0[1] - 410.0).abs() < 1e-3, "{p0:?}");
        let a0 = cam.cam.off[1].to_f32().atan2(cam.cam.off[0].to_f32());
        let r = crate::pad::PadInput::neutral().rstick(1.0, 0.0);
        for _ in 0..10 { pad.update(Some(&r.bytes()), false); cam.update(&CamInput { hero: &hero, pad: &pad, coll: &coll, mobys: None, hero_moby: None }); }
        let a1 = cam.cam.off[1].to_f32().atan2(cam.cam.off[0].to_f32());
        let wrap = |d: f32| { let d = d.to_degrees().rem_euclid(360.0); if d > 180.0 { d - 360.0 } else { d } };
        let turned = wrap(a1 - a0);
        let ap = cam.cam.placed[1].to_f32().atan2(cam.cam.placed[0].to_f32());
        let placed = wrap(ap - a0);
        let yaw = cam.out.yaw().to_f32().to_degrees();
        eprintln!("desired offset turned {turned}°, placed {placed}°, view yaw {yaw}°");
        assert!((turned - 13.0).abs() < 0.05, "{turned}");
        assert!(0.0 < placed && placed < turned);
        assert!((yaw - placed).abs() < 0.5, "the view follows the placed offset");
    }
}
