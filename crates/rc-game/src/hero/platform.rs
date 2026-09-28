//! **Package P1 — the platform carry** `HeroPlatformUpdate` 0x249618 (level01; the same function in every
//! overlay), the first call of the move pipeline 0x233de0 (`Hero::move_collide`). Owner: the P1 agent, which also
//! owns [`super::surface`] (docs/plan/hero_states.md "Packages"). Spec: docs/plan/triggers.md §5.
//!
//! **Carriers.** Any moby with mode bit 0x20 and a platform block (pvar+0x08, `FUN_00275290`,
//! [`triggers::platform_block`]); its update publishes this tick's move there with `CarryRiders` 0x2755f8
//! (block +0x00 the Euler of the rotation change, +0x10 the displacement, +0x3c flags). The mobys run before the
//! hero in a tick, so the block always holds this tick's move. The hero reads the carriers through
//! [`Env::world`] ([`HeroWorld::carrier`]); [`Carriers`] is the snapshot `crate::tick` builds from the moby table
//! after the moby loop. Nothing here is per class: the lift 726, the elevators 703 / 715 and every other class
//! that writes the block are carried the same way.
//!
//! The ledge fields it reads and moves (0x13f820 point, 0x13f834 yaw, 0x13f848 moby) are `Hero::ledge_blk`
//! (super::ledge).
//!
//! **The function** (from the disassembly):
//! 1. The platform 0x13f6b0 ([`Carry::moby`]): grounded (air ticks 0x13f65e = 0) → the ground moby 0x13f64c
//!    (none → cleared with flag bits 0/1; a different one clears bit 0); airborne in state 0x1c or group 3 with a
//!    ledge moby 0x13f848 → that one; airborne otherwise → the last one, with the fade `f = air ticks` (the two
//!    `ticks(120)` calls there are dead: their results are never read), or none. No platform, or not a carrier →
//!    return (0x13f440 / 0x13f44c untouched).
//! 2. Attach the point (flag bit 0): once, when idle (group 0 / 0xc, momentum 0x13f4a0 = 0, grounded,
//!    0x13f546 = 0) or hanging (0x18 / 0x19): the local point 0x13f660 (`FUN_00275528`: `(p − m.pos)·rowsᵀ`, the
//!    rotation `E(rot)·rowsᵀ`) of the carried position (block flag bit 2 clear) or of the position (bit 2 set); a
//!    hanging hero records the ledge point 0x13f820 in 0x13f660 and his hang point (0.45 behind the ledge, 1.43
//!    above it, after this tick's carry) in 0x13f680. Any other state clears bit 0.
//! 3. Attach the facing (flag bit 1): once, when idle (group 0 / 0xc, yaw velocity 0x13f4d4 = 0, grounded): the
//!    local rotation 0x13f670. Otherwise bit 1 is cleared.
//! 4. The correction (grounded or hanging, `f = 0`):
//!    * not attached: the carried point `FUN_002752c0` (`R(Δrot)·((p + Δ) − m.pos) + m.pos`,
//!      [`triggers::carry_point`]) minus the position; the yaw becomes the carried yaw;
//!    * attached: the recorded local points mapped back through the platform's rows now (`FUN_002753b0`:
//!      `l·rows + m.pos`, plus `ClampLen(Δ, 1)` when Ratchet's class slot is non-zero and below the platform's
//!      (update order); with block flag bit 2 plus `Δ` and re-recorded every tick); the yaw from the local
//!      rotation (bit 1) or the carried one (bit 0 only; the look stances 1 / 0x1e with 0x1413f5 always the
//!      carried one). Bit 0 clear: the carried point as above. Hanging: the carried point, the ledge point and yaw
//!      rebuilt from the mapped points. Otherwise the local point: a correction below 1e-4 snaps the position onto
//!      it (correction 0), **above 2 drops the attachment** (bits 0 and 1, correction 0); the ledge point follows
//!      the correction; a correction that differs from last tick's applied platform step 0x13f490 by more than
//!      0.1 clears bit 0.
//!    * 0x13f6a0 ([`Carry::last`]) = the correction.
//! 5. Airborne with the last platform (`f` = air ticks): block flag bit 1 clear → the last correction decays, its
//!    height by `Approach(0, 25·dt²·f)` and its horizontal length by `Approach(0, 2·dt²·f)` (from the stored
//!    value, so the decay is linear in the air ticks); bit 1 set → the last correction unchanged. 0x13f6a0 is not
//!    rewritten.
//! 6. 0x13f440 (`Hero::platform`, which the move adds to the position and re-collides) += the correction;
//!    0x13f44c (`Hero::platform[3]`) = the yaw change of this call.
//!
//! Standard floats (`f32`, the rotations composed in `f64`); the hero block's PS2 floats are converted at the
//! boundary. The Euler round trips `MatrixToEuler(EulerToMatrix(rot))` of the game (VU sin / cos, `FastArcTan`)
//! change the yaw by a few ulps per call; the port leaves the yaw untouched when the carrier does not rotate
//! (every carrier on the disc) and composes exactly otherwise (docs/plan/hardware_fidelity_layers.md).

use super::physics::{from_f32x3, to_f32x3, vadd, Env};
use super::Hero;
use crate::moby_runtime::MobyTable;
use crate::moby_update::triggers::{self, PlatformDelta};
use crate::ps2v::Pf;

/// What the hero reads of a carrier this tick: `FUN_00275290`'s block and the moby fields the carry uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carrier {
    /// +0x10: position (after this tick's move).
    pub position: [f32; 3],
    /// +0xc0 / +0xd0 / +0xe0: rotation rows.
    pub rows: [[f32; 3]; 3],
    /// +0x22: class slot (the update-order correction of `FUN_002753b0`).
    pub class_slot: u8,
    /// The platform block (rotation change, displacement, flags).
    pub delta: PlatformDelta,
}

/// Level and moby data the hero reaches outside its own block ([`Env::world`]). Default methods: nothing there.
/// Packages that need more (grind paths, swing targets) add methods here.
pub trait HeroWorld {
    /// `FUN_00275290(m) != 0` and the carrier's fields; None when moby `id` is not a carrier.
    fn carrier(&self, id: usize) -> Option<Carrier> {
        let _ = id;
        None
    }
    /// +0x22 of Ratchet's moby 0x1413d0.
    fn hero_class_slot(&self) -> u8 { 0 }
    /// Moby `id` has mode 0x20 and its pvar record (`FUN_002711f8`, pvar+0x00) has bit 0 of the u16 at +0x1e: a
    /// ledge on its top can be grabbed (probe B's moby branch, [`super::ledge`]; `triggers::record_ledge_flag`).
    fn moby_ledge_flag(&self, id: usize) -> bool {
        let _ = id;
        false
    }
    /// The level's grind paths (gameplay section 0x74, `0x15f70c`: the rails and cables of [`super::boots`]).
    fn grind_paths(&self) -> &[rc_formats::volumes::GrindPath] { &[] }
    /// The Swingshot targets of this tick's moby run list and the camera ([`super::swingshot::Targets`]; None:
    /// no targets).
    fn swing_targets(&self) -> Option<&super::swingshot::Targets> { None }
    /// The camera as the previous tick's update left it: position 0x167240, yaw 0x167258 and pitch 0x167254
    /// (positive looking down). None: no camera (tests).
    fn camera(&self) -> Option<([f32; 3], f32, f32)> { None }
    /// The target list 0x1abe80 as the melee aim search `0x22e238` reads it ([`super::melee::MeleeTarget`]; empty: no
    /// targets).
    fn melee_targets(&self) -> &[super::melee::MeleeTarget] { &[] }
}

/// The carriers of a moby table as the moby loop left it (built once per tick, before the hero update).
#[derive(Clone, Debug, Default)]
pub struct Carriers {
    list: Vec<(usize, Carrier)>,
    hero_slot: u8,
    /// The mobys whose pvar record allows ledges on their tops ([`HeroWorld::moby_ledge_flag`]).
    ledge_mobys: Vec<usize>,
    /// The level's grind paths ([`HeroWorld::grind_paths`]; the caller sets them, `Carriers::collect` leaves them
    /// empty).
    pub grind: std::sync::Arc<Vec<rc_formats::volumes::GrindPath>>,
    /// The Swingshot targets ([`HeroWorld::swing_targets`]; the caller sets them, `Carriers::collect` leaves none).
    pub targets: super::swingshot::Targets,
    /// The melee aim search's targets ([`HeroWorld::melee_targets`]; the caller sets them, `Carriers::collect` leaves none).
    pub melee: Vec<super::melee::MeleeTarget>,
}

impl Carriers {
    /// Every moby with a platform block (`FUN_00275290`), and Ratchet's class slot.
    pub fn collect(table: &MobyTable, hero_moby: usize) -> Carriers {
        let list = table
            .mobys
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                let delta = triggers::platform_delta(m)?;
                let rows = [0, 1, 2].map(|k| [m.rows[k][0], m.rows[k][1], m.rows[k][2]]);
                Some((i, Carrier { position: [m.position[0], m.position[1], m.position[2]], rows, class_slot: m.class_slot, delta }))
            })
            .collect();
        let hero_slot = table.mobys.get(hero_moby).map_or(0, |m| m.class_slot);
        let ledge_mobys = table.mobys.iter().enumerate().filter(|(_, m)| triggers::record_ledge_flag(m)).map(|(i, _)| i).collect();
        Carriers { list, hero_slot, ledge_mobys, grind: Default::default(), targets: Default::default(), melee: Vec::new() }
    }
}

impl HeroWorld for Carriers {
    fn carrier(&self, id: usize) -> Option<Carrier> { self.list.iter().find(|(i, _)| *i == id).map(|(_, c)| *c) }
    fn hero_class_slot(&self) -> u8 { self.hero_slot }
    fn moby_ledge_flag(&self, id: usize) -> bool { self.ledge_mobys.contains(&id) }
    fn grind_paths(&self) -> &[rc_formats::volumes::GrindPath] { &self.grind }
    fn swing_targets(&self) -> Option<&super::swingshot::Targets> { Some(&self.targets) }
    fn camera(&self) -> Option<([f32; 3], f32, f32)> { Some((self.targets.camera, self.targets.cam_yaw, self.targets.cam_pitch)) }
    fn melee_targets(&self) -> &[super::melee::MeleeTarget] { &self.melee }
}

/// The carry fields of the hero block.
#[derive(Clone, Default, PartialEq)]
pub struct Carry {
    /// 0x13f6b0: the platform moby (None = 0).
    pub moby: Option<usize>,
    /// 0x13f6b4: bit 0 the point is attached (0x13f660), bit 1 the facing (0x13f670).
    pub flags: u32,
    /// 0x13f660: the attached point in the platform's local space; 0x13f670: the attached rotation (Euler) in it.
    pub local: [f32; 3],
    pub local_rot: [f32; 3],
    /// 0x13f680: the hang point in the platform's local space (states 0x18 / 0x19; the game's 0x13f680 is also
    /// the knockback magnitude / pitch / yaw, which the hanging states do not use).
    pub hang_local: [f32; 3],
    /// 0x13f6a0: the last correction (x, y, z, w).
    pub last: [f32; 4],
}

impl std::fmt::Debug for Carry {
    /// `0` while untouched (the hero digest drops new zero fields), else the fields.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Carry::default() { return write!(f, "0"); }
        f.debug_struct("Carry")
            .field("moby", &self.moby)
            .field("flags", &self.flags)
            .field("local", &self.local)
            .field("local_rot", &self.local_rot)
            .field("hang_local", &self.hang_local)
            .field("last", &self.last)
            .finish()
    }
}

// ------------------------------------------------------------------------------------------------
// Rotations: `EulerToMatrix` 0x221980 (R = X·Y·Z, row-vector convention: rows = images of the axes) and
// `MatrixToEuler` 0x2721f0 (z = atan2(m01, m00), then y, then x of what is left), in f64.

type M3 = [[f64; 3]; 3];

fn euler_matrix(e: [f32; 3]) -> M3 {
    let (sx, cx) = (e[0] as f64).sin_cos();
    let (sy, cy) = (e[1] as f64).sin_cos();
    let (sz, cz) = (e[2] as f64).sin_cos();
    let x = [[1.0, 0.0, 0.0], [0.0, cx, sx], [0.0, -sx, cx]];
    let y = [[cy, 0.0, -sy], [0.0, 1.0, 0.0], [sy, 0.0, cy]];
    let z = [[cz, sz, 0.0], [-sz, cz, 0.0], [0.0, 0.0, 1.0]];
    mul(&mul(&x, &y), &z)
}

/// `a·b` (row i of the result = row i of `a` through `b`).
fn mul(a: &M3, b: &M3) -> M3 { std::array::from_fn(|i| std::array::from_fn(|k| (0..3).map(|j| a[i][j] * b[j][k]).sum())) }

fn transpose(a: &M3) -> M3 { std::array::from_fn(|i| std::array::from_fn(|k| a[k][i])) }

fn matrix_euler(m: &M3) -> [f32; 3] {
    let z = m[0][1].atan2(m[0][0]);
    let m1 = mul(m, &euler_matrix([0.0, 0.0, -z as f32]));
    let y = (-m1[0][2]).atan2(m1[0][0]);
    let m2 = mul(&m1, &euler_matrix([0.0, -y as f32, 0.0]));
    let x = m2[1][2].atan2(m2[1][1]);
    [x as f32, y as f32, z as f32]
}

fn rows64(c: &Carrier) -> M3 { c.rows.map(|r| r.map(|x| x as f64)) }

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn len3(a: [f32; 3]) -> f32 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }

/// `FUN_002752c0(p, rot)`: where a point and a rotation riding the carrier go this tick.
fn carried(c: &Carrier, p: [f32; 3], rot: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let d = &c.delta;
    let q = add3(p, [d.displacement[0], d.displacement[1], d.displacement[2]]);
    if d.rotation == [0.0; 3] { return (q, rot); }
    let md = euler_matrix(d.rotation);
    let v = sub3(q, c.position).map(|x| x as f64);
    let r: [f32; 3] = std::array::from_fn(|k| (v[0] * md[0][k] + v[1] * md[1][k] + v[2] * md[2][k]) as f32);
    (add3(r, c.position), matrix_euler(&mul(&euler_matrix(rot), &md)))
}

/// `FUN_00275528(p, rot)`: the point and rotation in the carrier's local space.
fn to_local(c: &Carrier, p: [f32; 3], rot: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let d = sub3(p, c.position);
    let l = c.rows.map(|r| d[0] * r[0] + d[1] * r[1] + d[2] * r[2]);
    (l, matrix_euler(&mul(&euler_matrix(rot), &transpose(&rows64(c)))))
}

/// `FUN_002753b0(l, lrot)`: a local point and rotation back in the world (and the update-order / bit-2
/// corrections; with block flag bit 2 the local pair is re-recorded).
fn from_local(c: &Carrier, hero_slot: u8, l: &mut [f32; 3], lrot: &mut [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let r = &c.rows;
    let mut p: [f32; 3] = std::array::from_fn(|k| l[0] * r[0][k] + l[1] * r[1][k] + l[2] * r[2][k]);
    p = add3(p, c.position);
    let rot = matrix_euler(&mul(&euler_matrix(*lrot), &rows64(c)));
    let d = [c.delta.displacement[0], c.delta.displacement[1], c.delta.displacement[2]];
    if c.delta.flags & 4 != 0 {
        p = add3(p, d);
        (*l, *lrot) = to_local(c, p, rot);
    } else if hero_slot != 0 && hero_slot < c.class_slot {
        // ClampLen(Δ, 1) 0x2745f0.
        let n = len3(d);
        let d = if 1.0 < n { d.map(|x| x * (1.0 / n)) } else { d };
        p = add3(p, d);
    }
    (p, rot)
}

/// `Approach(0, step, &x)` on a non-negative length / signed value.
fn approach_zero(x: f32, step: f32) -> f32 {
    if x < 0.0 { (x + step).min(0.0) } else { (x - step).max(0.0) }
}

/// `HeroPlatformUpdate` 0x249618 (start of the move 0x233de0): adds the carrier's move to `Hero::platform`
/// (0x13f440) and its yaw change to `Hero::platform[3]` (0x13f44c).
pub(super) fn platform_update(h: &mut Hero, env: &Env) {
    let yaw0 = h.rot[2];
    let air = h.air_ticks;
    let mut fade = 0.0f32;
    // 1. The platform.
    if air == 0 {
        match h.ground_moby {
            None => {
                h.carry.moby = None;
                h.carry.flags &= !3;
                return;
            }
            Some(g) => {
                if h.carry.moby != Some(g) { h.carry.flags &= !1; }
                h.carry.moby = Some(g);
            }
        }
    } else if (h.state == 0x1c || h.group == 3) && h.ledge_blk.moby.is_some() {
        h.carry.moby = h.ledge_blk.moby;
    } else if h.carry.moby.is_some() {
        fade = air as f32;
    } else {
        h.carry.flags &= !3;
        return;
    }
    let Some(world) = env.world else { return };
    let Some(id) = h.carry.moby else { return };
    let Some(c) = world.carrier(id) else { return };
    let hero_slot = world.hero_class_slot();
    let pos = to_f32x3(h.pos);
    let rot = to_f32x3(h.rot);
    let hanging = matches!(h.state, 0x18 | 0x19);
    let idle_group = h.group == 0 || h.group == 0xc;
    // 2. Attach the point.
    let momentum0 = super::physics::len3(h.momentum) == Pf::ZERO;
    if (idle_group && momentum0 && air == 0 && h.f546 == 0) || hanging {
        if h.carry.flags & 1 == 0 {
            if hanging {
                let lp = h.ledge_blk.point;
                let back = fast_add_rot(h.ledge_blk.yaw, std::f32::consts::PI);
                let hang = [lp[0] + back.cos() * 0.45, lp[1] + back.sin() * 0.45, lp[2] + 1.43];
                // The ledge point is recorded as it is, the hang point after this tick's carry (the game also
                // carries the ledge point into a temporary it never reads).
                let (hang, _) = carried(&c, hang, rot);
                (h.carry.local, _) = to_local(&c, lp, rot);
                (h.carry.hang_local, _) = to_local(&c, hang, rot);
            } else if c.delta.flags & 4 == 0 {
                let (q, qr) = carried(&c, pos, rot);
                (h.carry.local, _) = to_local(&c, q, qr);
            } else {
                (h.carry.local, _) = to_local(&c, pos, rot);
            }
            h.carry.flags |= 1;
        }
    } else {
        h.carry.flags &= !1;
    }
    // 3. Attach the facing.
    if idle_group && h.yaw_vel == Pf::ZERO && air == 0 {
        if h.carry.flags & 2 == 0 {
            (_, h.carry.local_rot) = to_local(&c, pos, rot);
            h.carry.flags |= 2;
        }
    } else {
        h.carry.flags &= !2;
    }
    let attached = h.carry.flags & 3 != 0;
    let mut d: [f32; 4];
    if fade == 0.0 {
        // 4. The correction.
        if !attached {
            let (q, qr) = carried(&c, pos, rot);
            if c.delta.rotation != [0.0; 3] { h.rot[2] = Pf::f(qr[2]); }
            let v = sub3(q, pos);
            d = [v[0], v[1], v[2], 0.0];
        } else {
            let (q, qr) = carried(&c, pos, rot);
            let (mut l, mut lr) = (h.carry.local, h.carry.local_rot);
            let (p660, r660) = from_local(&c, hero_slot, &mut l, &mut lr);
            (h.carry.local, h.carry.local_rot) = (l, lr);
            let (mut l, mut lr) = (h.carry.hang_local, h.carry.local_rot);
            let (p680, _) = from_local(&c, hero_slot, &mut l, &mut lr);
            (h.carry.hang_local, h.carry.local_rot) = (l, lr);
            let carried_yaw = if c.delta.rotation != [0.0; 3] { Pf::f(qr[2]) } else { h.rot[2] };
            // The look stances 1 / 0x1e with the camera-facing flag 0x1413f5 keep the carried yaw. Those states
            // are not ported (the hero does not move in them), so the flag is not in the block yet (P2).
            let look = matches!(h.state, 1 | 0x1e) && LOOK_FACES_CAMERA;
            h.rot[2] = if look || h.carry.flags & 2 == 0 { carried_yaw } else { Pf::f(r660[2]) };
            if h.carry.flags & 1 == 0 {
                let v = sub3(q, pos);
                d = [v[0], v[1], v[2], 0.0];
            } else if hanging {
                let y = h.ledge_blk.yaw;
                let lp = [y.cos() * 0.45 + p680[0], y.sin() * 0.45 + p680[1], p680[2] - 1.43];
                let v = sub3(q, pos);
                d = [v[0], v[1], v[2], 0.0];
                h.ledge_blk.point = lp;
                h.ledge_blk.yaw = (p660[1] - p680[1]).atan2(p660[0] - p680[0]);
            } else {
                let v = sub3(p660, pos);
                let n = len3(v);
                d = [v[0], v[1], v[2], 0.0];
                if n < 1e-4 {
                    d = [0.0; 4];
                    h.pos = [Pf::f(p660[0]), Pf::f(p660[1]), Pf::f(p660[2]), h.pos[3]];
                } else if 2.0 < n {
                    d = [0.0; 4];
                    h.carry.flags &= !3;
                }
                for (p, dk) in h.ledge_blk.point.iter_mut().zip(d) { *p += dk; }
                let a = to_f32x3(h.plat_applied);
                if 0.1 < len3(sub3([d[0], d[1], d[2]], a)) { h.carry.flags &= !1; }
            }
        }
        h.carry.last = d;
    } else {
        // 5. Airborne: the last correction, decaying unless the block keeps it (flag bit 1).
        let last = h.carry.last;
        if c.delta.flags & 2 == 0 {
            let dt2 = super::physics::DT2.to_f32();
            let z = approach_zero(last[2], dt2 * 25.0 * fade);
            let l0 = (last[0] * last[0] + last[1] * last[1]).sqrt();
            let l = approach_zero(l0, (dt2 + dt2) * fade);
            let (x, y) = if l0 * l0 == 0.0 { (0.0, 0.0) } else { (last[0] * (l / l0), last[1] * (l / l0)) };
            d = [x, y, z, last[3]];
        } else {
            d = last;
        }
    }
    // 6. 0x13f440 += the correction (xyz); 0x13f44c = the yaw change.
    let w = h.platform[3];
    h.platform = vadd(h.platform, from_f32x3([d[0], d[1], d[2]]));
    h.platform[3] = w;
    h.platform[3] = super::physics::fast_subtract_rotations(h.rot[2], yaw0);
}

/// 0x1413f5 (look stance facing the camera), until the look stance is ported.
const LOOK_FACES_CAMERA: bool = false;

/// `fast_add_rotations` in f32 (wrap into (−π, π]).
fn fast_add_rot(a: f32, b: f32) -> f32 {
    let mut r = a + b;
    let pi = std::f32::consts::PI;
    if pi < r { r -= 2.0 * pi; } else if r < -pi { r += 2.0 * pi; }
    r
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::physics::V0;
    use crate::pad::PadState;
    use rc_formats::collision::Collision;

    const ID: usize = 7;

    /// One carrier, moby `ID`.
    struct One(Carrier);
    impl HeroWorld for One {
        fn carrier(&self, id: usize) -> Option<Carrier> { (id == ID).then_some(self.0) }
    }

    fn carrier(pos: [f32; 3], d: [f32; 3], rot: [f32; 3], flags: u32) -> Carrier {
        Carrier {
            position: pos,
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            class_slot: 5,
            delta: PlatformDelta { rotation: rot, displacement: [d[0], d[1], d[2], 0.0], flags },
        }
    }

    /// Ratchet standing idle on moby `ID` at `p`.
    fn idle_on(p: [f32; 3]) -> Hero {
        let mut h = Hero::spawn(p, 0.3);
        h.ground_moby = Some(ID);
        h
    }

    /// `HeroPlatformUpdate`, then what the move does with 0x13f440 (added to the position; no collision here):
    /// returns the correction.
    fn step(h: &mut Hero, c: &Carrier) -> [f32; 3] {
        let coll = Collision::default();
        let pad = PadState::default();
        let world = One(*c);
        let env = Env { coll: &coll, pad: &pad, cam_yaw: Pf::ZERO, cam_rows: [V0; 3], mirror: false, death_z: Pf::ZERO, mobys: None, hero_moby: None, water: None, world: Some(&world) };
        h.platform = V0;
        let p0 = h.pos;
        platform_update(h, &env);
        let d = to_f32x3(h.platform);
        h.plat_applied = h.platform;
        h.pos = vadd(h.pos, h.platform);
        let _ = p0;
        d
    }

    fn close(a: [f32; 3], b: [f32; 3], eps: f32) -> bool { (0..3).all(|k| (a[k] - b[k]).abs() <= eps) }

    /// Idle on a moving platform: attached in its local space (flags 3) and carried by exactly its move every
    /// tick, the offset to the platform constant. On the first tick the correction differs from last tick's
    /// applied platform step 0x13f490 (0) by more than 0.1, which clears bit 0 until the next tick re-attaches.
    #[test]
    fn idle_rider_is_pinned() {
        let mut c = carrier([10.0, 10.0, 5.0], [0.1, -0.05, -0.04], [0.0; 3], 0);
        let mut h = idle_on([11.0, 10.5, 5.0]);
        let off0 = sub3(to_f32x3(h.pos), c.position);
        for t in 0..120 {
            // The carrier's update ran first this tick: its position already includes the move.
            c.position = add3(c.position, [0.1, -0.05, -0.04]);
            let d = step(&mut h, &c);
            assert_eq!(h.carry.flags, if t == 0 { 2 } else { 3 }, "tick {t}");
            assert!(close(d, [0.1, -0.05, -0.04], 1e-4), "tick {t}: {d:?}");
            assert!(close(sub3(to_f32x3(h.pos), c.position), off0, 2e-3), "tick {t}: drifted");
        }
        assert_eq!(h.carry.moby, Some(ID));
        assert!(close([h.carry.last[0], h.carry.last[1], h.carry.last[2]], [0.1, -0.05, -0.04], 1e-4));
        // The facing is pinned too: no yaw change.
        assert_eq!(h.platform[3], Pf::ZERO);
    }

    /// Walking (group 1): not attached; every tick the platform's move (`FUN_002752c0`), which the move adds.
    #[test]
    fn walking_rider_gets_the_move() {
        let c = carrier([10.0, 10.0, 5.0], [0.0, 0.2, 0.0], [0.0; 3], 0);
        let mut h = idle_on([10.0, 10.0, 5.0]);
        h.group = 1;
        h.state = 2;
        for _ in 0..5 {
            assert!(close(step(&mut h, &c), [0.0, 0.2, 0.0], 1e-6));
            assert_eq!(h.carry.flags, 0);
        }
        // Moving momentum (0x13f4a0 ≠ 0) in the idle group does not attach either.
        h.group = 0;
        h.momentum = [Pf::f(0.1), Pf::ZERO, Pf::ZERO, Pf::ZERO];
        step(&mut h, &c);
        assert_eq!(h.carry.flags & 1, 0);
    }

    /// Jumping off: airborne, the last platform's correction keeps being added, its height shrinking by
    /// 25·dt²·(air ticks) and its horizontal length by 2·dt²·(air ticks) from the stored value; with block flag
    /// bit 1 it is kept whole. Landing on the world clears the platform.
    #[test]
    fn jumping_off_decays_the_push() {
        let dt2 = super::super::physics::DT2.to_f32();
        for keep in [false, true] {
            let c = carrier([10.0, 10.0, 5.0], [0.3, 0.0, 0.05], [0.0; 3], if keep { 2 } else { 0 });
            let mut h = idle_on([10.0, 10.0, 5.0]);
            h.group = 1;
            step(&mut h, &c);
            h.group = 4;
            h.ground_moby = None;
            for n in 1..=6i16 {
                h.air_ticks = n;
                let d = step(&mut h, &c);
                let f = n as f32;
                let want = if keep { [0.3, 0.0, 0.05] } else { [(0.3 - 2.0 * dt2 * f).max(0.0), 0.0, (0.05 - 25.0 * dt2 * f).max(0.0)] };
                assert!(close(d, want, 1e-6), "keep {keep} air {n}: {d:?} vs {want:?}");
            }
            // The stored correction is not rewritten while airborne.
            assert!(close([h.carry.last[0], h.carry.last[1], h.carry.last[2]], [0.3, 0.0, 0.05], 1e-6));
            h.air_ticks = 0;
            assert_eq!(step(&mut h, &c), [0.0; 3]);
            assert_eq!(h.carry.moby, None);
        }
    }

    /// The attachment drops when the pinned point is more than 2 units away (a teleport, a push), a correction
    /// below 1e-4 snaps onto it, and one that differs from the last applied platform step by more than 0.1 lets
    /// go of the point (bit 0) for a tick.
    #[test]
    fn drop_beyond_two_units_and_snap() {
        let c = carrier([10.0, 10.0, 5.0], [0.0; 3], [0.0; 3], 0);
        let attached = || {
            let mut h = idle_on([10.5, 10.0, 5.0]);
            step(&mut h, &c);
            assert_eq!(h.carry.flags, 3);
            h
        };
        // 5e-5 off: snapped back exactly, no correction.
        let mut h = attached();
        h.pos[0] = Pf::f(10.500_05);
        assert_eq!(step(&mut h, &c), [0.0; 3]);
        assert_eq!(h.pos[0], Pf::f(10.5));
        assert_eq!(h.carry.flags, 3);
        // 1.5 off: pulled back, and the point let go (the jump from last tick's 0 step exceeds 0.1).
        let mut h = attached();
        h.pos[0] = Pf::f(12.0);
        let d = step(&mut h, &c);
        assert!(close(d, [-1.5, 0.0, 0.0], 1e-5), "{d:?}");
        assert_eq!(h.carry.flags, 2);
        // 3 off: dropped (bits 0 and 1), no correction; the next idle tick attaches where he stands.
        let mut h = attached();
        h.pos[0] = Pf::f(13.5);
        assert_eq!(step(&mut h, &c), [0.0; 3]);
        assert_eq!(h.carry.flags, 0);
        assert_eq!(step(&mut h, &c), [0.0; 3]);
        assert_eq!(h.carry.flags, 3);
        assert!(close(h.carry.local, [3.5, 0.0, 0.0], 1e-6));
    }

    /// A carrier that turns (none on the disc; the general path): the rider turns about its centre and 0x13f44c
    /// holds the yaw change.
    #[test]
    fn turning_carrier_turns_the_rider() {
        let c = carrier([10.0, 10.0, 5.0], [0.0; 3], [0.0, 0.0, 0.1], 0);
        let mut h = idle_on([12.0, 10.0, 5.0]);
        h.group = 1;
        let yaw0 = h.rot[2].to_f32();
        let d = step(&mut h, &c);
        let want = [10.0 + 2.0 * 0.1f32.cos() - 12.0, 2.0 * 0.1f32.sin(), 0.0];
        assert!(close(d, want, 1e-5), "{d:?} vs {want:?}");
        assert!((h.rot[2].to_f32() - (yaw0 + 0.1)).abs() < 1e-5);
        assert!((h.platform[3].to_f32() - 0.1).abs() < 1e-3);
    }

    /// Not a carrier (no block, or no world): nothing.
    #[test]
    fn not_a_carrier() {
        let c = carrier([10.0, 10.0, 5.0], [1.0, 0.0, 0.0], [0.0; 3], 0);
        let mut h = idle_on([10.0, 10.0, 5.0]);
        h.ground_moby = Some(ID + 1);
        assert_eq!(step(&mut h, &c), [0.0; 3]);
        assert_eq!(h.carry.moby, Some(ID + 1));
    }

    /// The rotations: `MatrixToEuler(EulerToMatrix(e)) = e` and the local round trip through tilted rows.
    #[test]
    fn euler_round_trips() {
        for e in [[0.0f32, 0.0, 1.0], [0.2, -0.3, 2.5], [-0.5, 0.4, -3.0]] {
            let r = matrix_euler(&euler_matrix(e));
            assert!(close(r, e, 1e-5), "{e:?} → {r:?}");
        }
        let rows = euler_matrix([0.1, 0.2, 0.7]).map(|r| r.map(|x| x as f32));
        let c = Carrier { rows, ..carrier([3.0, 4.0, 5.0], [0.0; 3], [0.0; 3], 0) };
        let (mut l, mut lr) = to_local(&c, [4.0, 6.0, 5.5], [0.0, 0.0, 1.2]);
        let (p, r) = from_local(&c, 0, &mut l, &mut lr);
        assert!(close(p, [4.0, 6.0, 5.5], 1e-5) && close(r, [0.0, 0.0, 1.2], 1e-5), "{p:?} {r:?}");
    }
}
