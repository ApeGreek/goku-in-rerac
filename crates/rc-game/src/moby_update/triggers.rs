//! Trigger volumes and moving platforms: the shared helpers every class calls (docs/plan/triggers.md). Level01
//! addresses; the functions are engine code, identical in all 19 level overlays (decomp/names/clusters.tsv).
//!
//! **Volume tests.** A class keeps volume *indices* in its pvars (s32, −1 = none) and passes a test point:
//! Ratchet's feet `0x13f3d0` (almost every caller), the camera `0x167240` (water 751, gunship 688) or the hero
//! body point `0x13f420` (amoeboid). The test maps `p − centre` through the record's stored inverse rows
//! (`VecSub` 0x2211b8, `MatrixMulVec3` 0x2215e0: `l = d.x·inv0 + d.y·inv1 + d.z·inv2`) and bounds `l`:
//!
//! | fn | test | index guard |
//! |---|---|---|
//! | [`point_in_cuboid`] 0x274820 | `−1 ≤ l.x, l.y, l.z ≤ 1` (`c.le.s` both ends) | `== −1` |
//! | [`point_in_cylinder`] 0x2748f8 | `|l.xy| < 1` (VU `vsqrt`, 0x221318) and `−1 ≤ l.z ≤ 1` | `< 0` |
//! | [`point_in_sphere`] 0x2749b0 | `|l| < 1` (0x2212e8) | `< 0` |
//! | [`point_in_path_polygon`] 0x26e6c0 | even-odd crossing test in XY over a path's points, closed | count ≤ 0 |
//! | [`sphere_touches_shape`] 0x20ff18 | camera-grid primitive test (sphere radius only for spheres / pill caps) | — |
//! | [`box_local`] + [`in_unit_box`] | the cuboid test for records with their own box (sound instances 0x31a128 …) | — |
//!
//! The tests are stateless: no helper keeps "was inside" state. Every edge (entered / left) is kept by the
//! class that asks, in its own pvars (checkpoint 805's rising edge, the path lift's one-shot latch that clears
//! its pvar index and sets the spawn id's death bits, …): triggers.md §4 lists them.
//!
//! **Moving platforms.** A carrier is a moby with mode `0x20` whose pvar+0x08 holds the offset of its
//! *platform block* (a relative pointer fixed up by the loader; the port keeps it block-relative):
//! [`platform_block`] = `FUN_00275290`. Each tick the carrier's update calls `CarryRiders` 0x2755f8 with its
//! displacement and rotation change ([`carry_riders`]): block +0x00 = Euler of `R_oldᵀ·R_new`, +0x10 = the
//! displacement. `HeroPlatformUpdate` 0x249618 (hero code) reads the block of the moby he stands on and moves
//! him with it ([`carry_point`] is its point transform `FUN_002752c0`). [`World::hero_on_moby`] = `HeroOnMoby`
//! 0x277fb8.
//!
//! Standard `f32` throughout (the PS2 VU/FPU rounding is not modelled; boundary cases can differ by an ulp).

use crate::moby_runtime::{Moby, MobyId};
use crate::moby_update::services::{fl, Services, World};
use rc_formats::volumes::{ShapeKind, Volumes};
use std::sync::Arc;

// ---------------------------------------------------------------------------------------------------------
// Volume tests

/// The local point of `p` in a box given by its centre and world → local rows: `l = (p − c)·inv` (0x2211b8 +
/// 0x2215e0). The same computation as [`Shape::local`](rc_formats::volumes::Shape::local), for records that
/// carry their own box instead of a cuboid index: the sound-instance boxes (centre +0x40, rows +0x50;
/// `SndInstMusicBoxUpdate` 0x31a128 and the volume / one-shot / reverb boxes, audio.md §3.3).
pub fn box_local(centre: [f32; 3], inverse: &[[f32; 4]; 3], p: [f32; 3]) -> [f32; 3] {
    let d = [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]];
    std::array::from_fn(|k| inverse[0][k] * d[0] + inverse[1][k] * d[1] + inverse[2][k] * d[2])
}

/// The unit-box bound every box test applies to a local point: `−1 ≤ l ≤ 1` on all three axes (0x274820's
/// `c.le.s` pairs; the sound boxes' `fabs(l) ≤ 1` is the same set).
pub fn in_unit_box(l: [f32; 3]) -> bool { l.iter().all(|&x| (-1.0..=1.0).contains(&x)) }

/// `PointInCuboid(p, index)` 0x274820 (table 0x1600ec). Only −1 is rejected by the game; the port also
/// rejects other negative / out-of-range indices (the game would read outside the table).
pub fn point_in_cuboid(v: &Volumes, p: [f32; 3], index: i32) -> bool {
    if index == -1 { return false; }
    let Some(s) = v.shape(ShapeKind::Cuboid, index) else { return false };
    in_unit_box(s.local(p))
}

/// `PointInCylinder(p, index)` 0x2748f8 (table 0x1600fc): the unit cylinder along local z.
pub fn point_in_cylinder(v: &Volumes, p: [f32; 3], index: i32) -> bool {
    let Some(s) = v.shape(ShapeKind::Cylinder, index) else { return false };
    let l = s.local(p);
    (l[0] * l[0] + l[1] * l[1]).sqrt() < 1.0 && (-1.0..=1.0).contains(&l[2])
}

/// `PointInSphere(p, index)` 0x2749b0 (table 0x1600f4): the unit ball.
pub fn point_in_sphere(v: &Volumes, p: [f32; 3], index: i32) -> bool {
    let Some(s) = v.shape(ShapeKind::Sphere, index) else { return false };
    let l = s.local(p);
    (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt() < 1.0
}

/// `PointInPathPolygon(p, points, count)` 0x26e6c0: the path's points as a closed polygon in XY (z ignored),
/// even-odd rule. Edge k runs from point k to point k+1 (the last back to 0); it counts when `p.y` lies in
/// `(min, max]` of its y and the crossing x is left of `p.x`.
pub fn point_in_path_polygon(p: [f32; 3], points: &[[f32; 4]]) -> bool {
    let n = points.len();
    let mut inside = false;
    for k in 0..n {
        let (a, b) = (points[k], points[(k + 1) % n]);
        let spans = (a[1] < p[1] && p[1] <= b[1]) || (b[1] < p[1] && p[1] <= a[1]);
        if spans && a[0] + ((p[1] - a[1]) / (b[1] - a[1])) * (b[0] - a[0]) < p[0] { inside = !inside; }
    }
    inside
}

/// [`point_in_path_polygon`] over path `index` (`0x1b0930[index]`); false for a negative / missing index.
pub fn point_in_path(v: &Volumes, p: [f32; 3], index: i32) -> bool {
    usize::try_from(index).ok().and_then(|i| v.paths.get(i)).is_some_and(|pts| point_in_path_polygon(p, pts))
}

/// `FUN_0020ff18(r, centre, prim)`: does a sphere touch shape `index` of `kind` (the camera-collision grid's
/// primitive test, called from its lookup `FUN_0020fdb0`). Only the sphere and the pill caps use `r`: a
/// cuboid or cylinder tests the centre alone.
/// * sphere: `|centre − c| < |row0| + r` (the radius is the length of matrix row 0);
/// * cuboid / cylinder: as [`point_in_cuboid`] / [`point_in_cylinder`];
/// * pill: the cylinder, else the caps `c ± row2` with the radius at runtime +0x80
///   ([`Volumes::pill_cap_radius`]), `|centre − cap| < radius` (r not added).
pub fn sphere_touches_shape(v: &Volumes, kind: ShapeKind, index: i32, centre: [f32; 3], r: f32) -> bool {
    let Some(s) = v.shape(kind, index) else { return false };
    let len = |a: [f32; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    match kind {
        ShapeKind::Sphere => {
            let rad = len([s.matrix[0][0], s.matrix[0][1], s.matrix[0][2]]);
            len(sub(centre, s.centre())) < rad + r
        }
        ShapeKind::Cuboid => point_in_cuboid(v, centre, index),
        ShapeKind::Cylinder => point_in_cylinder(v, centre, index),
        ShapeKind::Pill => {
            let l = s.local(centre);
            if (l[0] * l[0] + l[1] * l[1]).sqrt() < 1.0 && (-1.0..=1.0).contains(&l[2]) { return true; }
            let cap_r = v.pill_cap_radius(index).unwrap_or(0.0);
            [1.0f32, -1.0].iter().any(|&z| len(sub(centre, s.world([0.0, 0.0, z]))) < cap_r)
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The level's volumes in the moby services, and the World-side calls

impl Services {
    /// The level's volume sections (`rc_formats::volumes::parse_volumes`), for [`World`]'s trigger tests.
    pub fn set_volumes(&mut self, v: Volumes) { self.volumes = Arc::new(v); }
}

impl World<'_> {
    /// `0x13f3d0`: Ratchet's position (feet), the point nearly every class tests.
    pub fn hero_point(&self) -> [f32; 3] { [fl(self.hero.pos[0]), fl(self.hero.pos[1]), fl(self.hero.pos[2])] }

    /// `0x167240`: the camera position.
    pub fn camera_point(&self) -> [f32; 3] { [fl(self.camera[0]), fl(self.camera[1]), fl(self.camera[2])] }

    /// `0x13f420`: the hero body point (`pos + R·(0, 0, 0.7)`).
    pub fn hero_body_point(&self) -> [f32; 3] { [fl(self.hero.body_point[0]), fl(self.hero.body_point[1]), fl(self.hero.body_point[2])] }

    /// `PointInCuboid(p, index)` 0x274820 on this level's cuboids.
    pub fn in_cuboid(&self, p: [f32; 3], index: i32) -> bool { point_in_cuboid(&self.svc.volumes, p, index) }

    /// `PointInCylinder(p, index)` 0x2748f8.
    pub fn in_cylinder(&self, p: [f32; 3], index: i32) -> bool { point_in_cylinder(&self.svc.volumes, p, index) }

    /// `PointInSphere(p, index)` 0x2749b0.
    pub fn in_sphere(&self, p: [f32; 3], index: i32) -> bool { point_in_sphere(&self.svc.volumes, p, index) }

    /// `PointInPathPolygon(p, 0x1b0930[index])` 0x26e6c0.
    pub fn in_path(&self, p: [f32; 3], index: i32) -> bool { point_in_path(&self.svc.volumes, p, index) }

    /// `HeroOnMoby(m)` 0x277fb8: in movement group 3 (0x1413dc) or state 0x1c (0x1413d4) the hero's attach moby
    /// `0x13f848` decides (not modelled by the hero port: never); otherwise he stands on `m`: air ticks
    /// `0x13f65e` = 0 and ground moby `0x13f64c` = m.
    pub fn hero_on_moby(&self, id: MobyId) -> bool {
        let h = self.hero;
        if h.group == 3 || h.state == 0x1c { return false; }
        h.air_ticks == 0 && h.ground_moby == Some(id)
    }
}

// ---------------------------------------------------------------------------------------------------------
// Moving platforms

/// `FUN_00275290(m)`: the platform block of a carrier: mode `0x20` and a non-zero pvar+0x08 (the block's
/// offset in the pvar block; a heap pointer in the game). None: not a carrier.
pub fn platform_block(m: &Moby) -> Option<usize> {
    if m.mode & 0x20 == 0 || m.pvars.len() < 0x0c { return None; }
    let o = i32::from_le_bytes(m.pvars[8..12].try_into().unwrap());
    usize::try_from(o).ok().filter(|&o| o != 0 && o + 0x40 <= m.pvars.len())
}

/// The platform block as the hero code reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlatformDelta {
    /// +0x00: Euler angles of this tick's rotation change `R_oldᵀ·R_new`.
    pub rotation: [f32; 3],
    /// +0x10: this tick's displacement (w copied from the caller's vector).
    pub displacement: [f32; 4],
    /// +0x3c: flags (bit 1: the hero keeps the carried velocity after leaving; bit 2: re-attach in local space
    /// every tick; `HeroPlatformUpdate`).
    pub flags: u32,
}

/// Reads a carrier's platform block (None: not a carrier).
pub fn platform_delta(m: &Moby) -> Option<PlatformDelta> {
    let o = platform_block(m)?;
    let f = |k: usize| f32::from_le_bytes(m.pvars[o + k..o + k + 4].try_into().unwrap());
    Some(PlatformDelta {
        rotation: [f(0), f(4), f(8)],
        displacement: [f(0x10), f(0x14), f(0x18), f(0x1c)],
        flags: u32::from_le_bytes(m.pvars[o + 0x3c..o + 0x40].try_into().unwrap()),
    })
}

/// `CarryRiders(block, delta, rot_old, rot_new)` 0x2755f8, writing into `pvars[block..]`: +0x00 = Euler of
/// `EulerToMatrix(rot_old)ᵀ · EulerToMatrix(rot_new)` (`FUN_002721f0`), +0x10 = `delta` (all four words).
///
/// Every caller (lift 726, elevators 703 / 715) passes its own +0x40 for both rotations, so the product is
/// `Rᵀ·R = I` and the Euler angles are (0, 0, 0): that is what the port writes (the VU product can be off the
/// identity by an ulp, i.e. angles ≲ 1e-7 rad, which nothing can see). A caller with a real rotation change
/// would need the general extraction; there is none.
pub fn carry_riders(pvars: &mut [u8], block: usize, delta: [f32; 4], rot_old: [f32; 4], rot_new: [f32; 4]) {
    debug_assert!(rot_old[..3] == rot_new[..3], "CarryRiders with a rotation change is not ported");
    for k in 0..3 { pvars[block + 4 * k..block + 4 * k + 4].copy_from_slice(&0f32.to_le_bytes()); }
    for (k, d) in delta.iter().enumerate() {
        pvars[block + 0x10 + 4 * k..block + 0x14 + 4 * k].copy_from_slice(&d.to_le_bytes());
    }
}

/// `FUN_002752c0`'s point transform: where a point riding the carrier goes this tick,
/// `R(Δrot)·((p + Δ) − c) + c` with `c` the carrier's position after its move. With Δrot = 0 (every carrier
/// on the disc) this is `p + Δ`. None: not a carrier. (The hero side of the carry, `HeroPlatformUpdate`, is
/// hero code: triggers.md §5.)
pub fn carry_point(m: &Moby, p: [f32; 3]) -> Option<[f32; 3]> {
    let d = platform_delta(m)?;
    let q = [p[0] + d.displacement[0], p[1] + d.displacement[1], p[2] + d.displacement[2]];
    if d.rotation == [0.0; 3] { return Some(q); }
    let c = [m.position[0], m.position[1], m.position[2]];
    let r = rc_formats::moby_light::rotation_rows(d.rotation).map(|row| row.map(f32::from_bits));
    let v = [q[0] - c[0], q[1] - c[1], q[2] - c[2]];
    Some(std::array::from_fn(|k| v[0] * r[0][k] + v[1] * r[1][k] + v[2] * r[2][k] + c[k]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::volumes::Shape;

    /// A shape centred at `c` with local axes `axes` (rows; half extents = their lengths) and the exact inverse
    /// rows for orthogonal axes.
    fn shape(c: [f32; 3], axes: [[f32; 3]; 3]) -> Shape {
        let mut s = Shape::default();
        for (k, &a) in axes.iter().enumerate() {
            let l2 = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
            s.matrix[k] = [a[0], a[1], a[2], 0.0];
            // inverse: column k of the inverse = a_k / |a_k|² → rows j get a_k[j] / l2 in lane k.
            for (row, &x) in s.inverse.iter_mut().zip(&a) { row[k] = x / l2; }
        }
        s.matrix[3] = [c[0], c[1], c[2], 1.0];
        s
    }

    fn volumes() -> Volumes {
        // Cuboid 0: centre (10, 20, 5), half sizes 4 (along world +y), 2 (along world −x), 3 (z).
        let cub = shape([10.0, 20.0, 5.0], [[0.0, 4.0, 0.0], [-2.0, 0.0, 0.0], [0.0, 0.0, 3.0]]);
        // Cylinder 0: radius 2 in xy, half height 1, at (0, 0, 10).
        let cyl = shape([0.0, 0.0, 10.0], [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]]);
        // Sphere 0: radius 5 at (100, 0, 0).
        let sph = shape([100.0, 0.0, 0.0], [[5.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 5.0]]);
        // Path 0: the square (0,0)-(10,0)-(10,10)-(0,10).
        let sq = vec![[0.0, 0.0, 0.0, 0.0], [10.0, 0.0, 0.0, 0.0], [10.0, 10.0, 0.0, 0.0], [0.0, 10.0, 0.0, 0.0]];
        Volumes { cuboids: vec![cub], cylinders: vec![cyl], spheres: vec![sph], paths: vec![sq], ..Default::default() }
    }

    #[test]
    fn cuboid_faces_are_inclusive() {
        let v = volumes();
        assert!(point_in_cuboid(&v, [10.0, 20.0, 5.0], 0));
        // Faces: y ± 4, x ± 2, z ± 3 are inside (c.le.s), just past them outside.
        for p in [[10.0, 24.0, 5.0], [10.0, 16.0, 5.0], [8.0, 20.0, 5.0], [12.0, 20.0, 5.0], [10.0, 20.0, 8.0], [10.0, 20.0, 2.0]] {
            assert!(point_in_cuboid(&v, p, 0), "{p:?}");
        }
        for p in [[10.0, 24.01, 5.0], [7.99, 20.0, 5.0], [10.0, 20.0, 8.01], [14.0, 20.0, 5.0]] {
            assert!(!point_in_cuboid(&v, p, 0), "{p:?}");
        }
        assert!(!point_in_cuboid(&v, [10.0, 20.0, 5.0], -1));
        let cub = &v.cuboids[0];
        assert_eq!(box_local(cub.centre(), &cub.inverse, [12.0, 24.0, 8.0]), cub.local([12.0, 24.0, 8.0]));
        assert!(in_unit_box([1.0, -1.0, 0.0]) && !in_unit_box([1.0, -1.0, 1.0001]));
        assert!(!point_in_cuboid(&v, [10.0, 20.0, 5.0], 1));
    }

    #[test]
    fn cylinder_sphere_and_path() {
        let v = volumes();
        assert!(point_in_cylinder(&v, [1.9, 0.0, 10.0], 0));
        assert!(!point_in_cylinder(&v, [2.0, 0.0, 10.0], 0), "|l.xy| < 1 is strict");
        assert!(point_in_cylinder(&v, [0.0, 0.0, 11.0], 0), "−1 ≤ l.z ≤ 1 is inclusive");
        assert!(!point_in_cylinder(&v, [0.0, 0.0, 11.01], 0));
        assert!(!point_in_cylinder(&v, [0.0, 0.0, 10.0], -2));
        assert!(point_in_sphere(&v, [104.9, 0.0, 0.0], 0));
        assert!(!point_in_sphere(&v, [105.0, 0.0, 0.0], 0), "strict");
        assert!(!point_in_sphere(&v, [104.0, 3.0, 0.0], 0));
        assert!(point_in_path(&v, [5.0, 5.0, 99.0], 0), "z ignored");
        assert!(!point_in_path(&v, [15.0, 5.0, 0.0], 0));
        assert!(!point_in_path(&v, [5.0, 5.0, 0.0], 1));
        // An edge's y range is (min, max]: y = 0 (the bottom edge's level) is outside, y = 10 inside.
        assert!(!point_in_path_polygon([5.0, 0.0, 0.0], &v.paths[0]));
        assert!(point_in_path_polygon([5.0, 10.0, 0.0], &v.paths[0]));
        assert!(!point_in_path_polygon([5.0, 5.0, 0.0], &[]));
        // A concave polygon (U shape): the notch is outside.
        let u = [[0.0, 0.0], [9.0, 0.0], [9.0, 9.0], [6.0, 9.0], [6.0, 3.0], [3.0, 3.0], [3.0, 9.0], [0.0, 9.0]].map(|p| [p[0], p[1], 0.0, 0.0]);
        assert!(!point_in_path_polygon([4.5, 6.0, 0.0], &u));
        assert!(point_in_path_polygon([1.5, 6.0, 0.0], &u));
        assert!(point_in_path_polygon([4.5, 1.5, 0.0], &u));
    }

    #[test]
    fn camera_grid_sphere_test() {
        let mut v = volumes();
        assert!(sphere_touches_shape(&v, ShapeKind::Sphere, 0, [106.0, 0.0, 0.0], 1.5));
        assert!(!sphere_touches_shape(&v, ShapeKind::Sphere, 0, [106.5, 0.0, 0.0], 1.5));
        // Cuboid / cylinder: the radius is not used.
        assert!(!sphere_touches_shape(&v, ShapeKind::Cuboid, 0, [10.0, 24.5, 5.0], 10.0));
        assert!(sphere_touches_shape(&v, ShapeKind::Cylinder, 0, [1.0, 1.0, 10.5], 0.0));
        // A pill: the cylinder plus caps at centre ± row2 with the +0x80 radius.
        v.pills = v.cylinders.clone();
        v.pill_tail = vec![0.5];
        assert!(sphere_touches_shape(&v, ShapeKind::Pill, 0, [0.0, 0.3, 11.3], 0.0));
        assert!(!sphere_touches_shape(&v, ShapeKind::Pill, 0, [0.0, 0.3, 11.6], 0.0));
        assert!(sphere_touches_shape(&v, ShapeKind::Pill, 0, [0.0, 0.0, 8.6], 0.0));
    }

    #[test]
    fn carry_block_round_trip() {
        let mut m = Moby::zeroed();
        m.pvars = vec![0; 0xd0];
        m.pvars[8..12].copy_from_slice(&0x60i32.to_le_bytes());
        assert!(platform_block(&m).is_none(), "mode 0x20 required");
        m.mode |= 0x20;
        assert_eq!(platform_block(&m), Some(0x60));
        m.position = [5.0, 5.0, 10.0, 0.0];
        let rot = [0.0, 0.0, 0.7, 0.0];
        carry_riders(&mut m.pvars, 0x60, [0.25, -0.5, 1.0, -1.0], rot, rot);
        let d = platform_delta(&m).unwrap();
        assert_eq!(d.rotation, [0.0; 3]);
        assert_eq!(d.displacement, [0.25, -0.5, 1.0, -1.0]);
        assert_eq!(carry_point(&m, [1.0, 2.0, 3.0]), Some([1.25, 1.5, 4.0]));
    }

    /// Novalis (from `extracted/`): the checkpoint / mission cuboids the classes name, tested at Ratchet's
    /// spawn; the native cuboid test agrees with the water port's PS2-float one on a grid of points.
    #[test]
    fn novalis_cuboids() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels/01/gameplay_ntsc.dec");
        let Ok(g) = std::fs::read(path) else { eprintln!("skipped: no extracted/levels/01"); return };
        let v = rc_formats::volumes::parse_volumes(&g).unwrap();
        let spawn = [162.53032, 136.39348, 60.5];
        // Checkpoint 805 (instance 906) pvar+0 = 54 contains the spawn; the mission cuboid 42 (730 / 790 / 737 /
        // 805) at (159.5, 208.4, 40) does not, but its centre is inside.
        assert!(point_in_cuboid(&v, spawn, 54));
        assert!(!point_in_cuboid(&v, spawn, 42));
        assert!(point_in_cuboid(&v, [159.51, 208.44, 40.0], 42));
        let water: Vec<rc_formats::water::Cuboid> = rc_formats::water::parse_cuboids(&g).unwrap();
        let mut disagree = 0;
        for (i, c) in water.iter().enumerate() {
            let s = &v.cuboids[i];
            assert_eq!(c.matrix, s.matrix);
            for dx in -12..=12 {
                for dz in -3..=3 {
                    let p = s.world([dx as f32 / 10.0 + 0.005, 0.37, dz as f32 / 2.5 + 0.005]);
                    if point_in_cuboid(&v, p, i as i32) != crate::water::cuboid_contains(c, p) { disagree += 1; }
                }
            }
        }
        // Off the faces the two float models agree (on a face they can round either way).
        assert_eq!(disagree, 0);
    }
}
