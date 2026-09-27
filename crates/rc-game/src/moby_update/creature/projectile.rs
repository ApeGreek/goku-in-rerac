//! Enemy projectiles: the engine pieces every shot uses (level01 addresses; the particle spawns are engine code in all
//! 19 overlays, `tools/ghidra/names/clusters.tsv`), and the pool records of the particle types the enemies spawn that
//! the particle system does not simulate yet.
//!
//! **Shots.** A projectile is a moby whose update moves it by its pvar velocity and sweeps the step: the segment
//! last → new position through `CollLine_Fix` with a hit template (the shooter ignored), then, on a miss, a small
//! sphere through `coll_sphere_mobys` with the same template ([`sweep`]). The template writes the hit record into the
//! moby that was hit (`services::line_hit_in` / `sphere_mobys_in`, `mode & 0x4000` mobys only); Ratchet's record goes
//! through the tick to P2's intake like any other hit. [`template`] is the record the enemy shots build on their stack
//! (`dir` = the push, `attacker` = the projectile moby, the type bytes, the damage). The class code (459's shot 722,
//! the gunship's shell 686) keeps its own flight rule and effects.
//!
//! **Particle records without a port.** Types 16 (smoke `PartType16Spawn` 0x280f30), 22 (jet exhaust
//! `PartType22Spawn` 0x281f30) and 26 (glow sprite on a moby `PartType26Spawn` 0x282b00) are taken from the pool and
//! make the spawn's own draws at the game's point ([`part`]); the particle system kills a type it does not simulate
//! on its first update (so the update's own draws are missing: `docs/plan/creatures.md` §6). The callers make the
//! draws for the arguments (colours, lives) themselves, in the game's order. Types 4, 8 and 15 are simulated: the
//! callers use `fx::part04` / `part08` / `part15`.

use super::V;
use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pf as to_pf, pv, HitTemplate, World};

/// A particle type without a simulation (module doc) and its spawn rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// `PartType16Spawn`: nothing for life 0; the frame-load throttle (`randi(3)` / `randi(2)` / `randi(1)` over
    /// 0.9 / 0.95 / 1.0 of 0x15f5d0, a 0 drops the spawn); a record, then `rand()`.
    T16 { life: i32 },
    /// `PartType22Spawn`: a record, then `rand()`.
    T22,
    /// `PartType26Spawn`: a record, then `rand()`.
    T26,
}

impl Part {
    pub const fn ty(self) -> u8 {
        match self {
            Part::T16 { .. } => 16,
            Part::T22 => 22,
            Part::T26 => 26,
        }
    }
}

/// The spawn of `p` (module doc): true when a record was taken.
pub fn part(w: &mut World, p: Part) -> bool {
    if let Part::T16 { life } = p {
        if life == 0 { return false; }
        let l0 = f32::from_bits(w.svc.frame_load[0].0);
        for (k, n) in [(0.9f32, 3), (0.95, 2), (1.0, 1)] {
            if k < l0 && w.rng.randi(n) == 0 { return false; }
        }
    }
    let ok = super::fx::part_unported(w, p.ty());
    if ok { w.rng.rand(); }
    ok
}

/// The hit template the enemy projectiles build (`+0x00` push, `+0x10` attacker, `+0x14` flags, `+0x18`/`+0x19` type
/// / subtype, `+0x1a` class, `+0x1c` damage, `+0x20`).
pub fn template(w: &World, shot: MobyId, dir: V, flags: u32, kind: (u8, u8), damage: f32, w20: u32) -> HitTemplate {
    HitTemplate { dir: pv(dir), attacker: Some(shot), flags, b18: kind.0, b19: kind.1, h1a: w.m(shot).o_class as u16, damage: to_pf(damage), w20 }
}

/// Is `p` inside the world box [2, 1021]³ (the bound every flight tests)?
pub fn in_world(p: V) -> bool { (0..3).all(|k| (2.0..=1021.0).contains(&p[k])) }

/// The sweep of one step `from → to` (module doc): `CollLine_Fix(from, to, flags, ignore, tmpl)`, the hit point on
/// a hit; else `coll_sphere_mobys(r, to, 0x10, ignore, tmpl)`, `to` when it touched a moby; `None` when the step is
/// clear.
pub fn sweep(w: &mut World, from: V, to: V, flags: u32, r: f32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<V> {
    let h = crate::moby_update::services::line_hit_in(w.table, w.svc, w.classes, w.coll, pv(from), pv(to), flags, ignore, tmpl);
    if let Some(h) = h { return Some([h.point[0], h.point[1], h.point[2], to[3]]); }
    (w.sphere_mobys(to_pf(r), pv(to), 0x10, ignore, Some(tmpl)) > 0).then_some(to)
}
