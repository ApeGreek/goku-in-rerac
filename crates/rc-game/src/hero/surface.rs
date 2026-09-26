//! **Package P1 — surfaces, slopes and sliding** (the surface reaction is ported for the flags the on-foot and
//! water code read; the surface-driven states are stubs). Owner: the P1 agent, which also owns
//! [`super::platform`] (docs/plan/hero_states.md "Packages"). Addresses level01; the superset of the surface
//! rules is level00's reaction 0x20b960 (each overlay compiles its own: level01 lacks surfaces 1, 3 and 7).
//!
//! The ground probe 0x232dc0 stores the hit's surface id in 0x140630 (`Hero::surface_id`); the reaction
//! `HeroSurfaceReaction` 0x22cd48 turns it into the per-tick flags 0x140630..0x14063f and, for some surfaces, a
//! state:
//! | surface | flag | effect |
//! |---|---|---|
//! | 0 water | 0x140634 | depth 0x1415f4, wading 0x1413f9 (ported; super::swim) |
//! | 1 hot floor | 0x140635 | 0x3c burn bounce / 0x7c burn death (super::damage); Clank 0x7d (level00) |
//! | 2 magnetic | 0x140637 | 0x13f658 on the class-0xad ground moby (Magneboots, super::boots) |
//! | 3 sinking liquid | 0x140636 | 0x68 sink to the level 0x13f644, ✕ → 0x69, 0x7b without health (level00) |
//! | 4 sinking floor | 0x140633 | 0x31 (group 0x10) while grounded outside groups 7 / 0x10 / 0x14 |
//! | 7 slippery | 0x140632 | the walk 2 ↔ 0x2f and its anims 0x37 / 0x6d, jumps keep the speed (level00) |
//! | 8, 0xc pit | 0x14063a | 0x79 pit fall from the ground states (super::damage) |
//! | 9 | 0x14063e | read by the jump transitions (level00) |
//! | 0xb | 0x14063b | 0x7b (level00: sinking-liquid contact) |
//! | 0xd deadly liquid | 0x14063c | 0x7f sinking death (super::damage) |
//! | 0xe shallow water | 0x140634 | water level = ground + 0.2 (ported) |
//! Level 0xd also turns a body-sphere contact with collision type 0xb into 0x7b.
//!
//! Slopes and sliding: steeper than 50° is not ground (the ported probe), so the hero falls (6) and the capsule
//! slides him down; the edge brake 0x236a68, the steep-wall stop 0x232820 and the slope ratio 0x13f4bc are
//! ported (physics.rs). What this package adds: the slippery walk 0x2f, the sinking floors 0x31 / 0x68 / 0x69 /
//! 0x7b, the surface rules above, and a data check per level that every surface id its faces use is handled by
//! that level's own reaction (so the superset does not change a level that lacks a rule).
//!
//! Seams already wired: [`walk_surface`] and [`walk_tail`] in the walk transitions; the reaction takes the
//! SetState context (it calls SetState in the game).
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::ps2v::Pf;
use crate::rng::Rng;

/// SetState entry of 0x2f, 0x31, 0x68, 0x69, 0x7b. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

/// The walk transitions' slippery-floor part (level00 0x229b70 case 2): the 0x2f anims, then 2 / 0x7e ↔ 0x2f by
/// 0x140632. Runs before the 2 ↔ 0x73 switch.
pub(super) fn walk_surface(_h: &mut Hero, _c: &mut Ctx) {}

/// The tail of the walk transitions for 0x2f (the stop and the playback speed; 2 and 0x73 have their own).
pub(super) fn walk_tail(_h: &mut Hero, _c: &mut Ctx) {}

impl Hero {
    /// Surface reaction `0x22cd48`, the part that sets flags the on-foot code reads: clears the per-tick
    /// surface flags, then water (surface 0: depth 0x1415f4 and the wading flag 0x1413f9 for depths in
    /// (0.25, 0.85)), 0xe (shallow water) and 2 (0x140637). The state changes it can make (0x7f, 0x31, 0x7b; level00
    /// also 0x3c / 0x7c / 0x7d / 0x68) lead to states the port does not implement and are not taken. `env`,
    /// `anim` and `rng` are the SetState context the full reaction needs.
    pub fn surface_reaction(&mut self, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) {
        let _ = (env, anim, rng);
        let sid = self.surface_id;
        self.surface_id = -1;
        self.f13f9 = 0;
        self.f658 = 0;
        self.f0632 = 0;
        self.f0634 = 0;
        self.f0637 = 0;
        self.f063a = 0;
        if sid == -1 { return; }
        if sid == 8 || sid == 0xc { self.f063a = 1; }
        if sid == 2 && (self.f65c == 0 || self.height < Pf::b(0x3e99_999a)) { self.f0637 = 1; }
        if sid == 0xe {
            self.f0634 = 1;
            self.water_level = self.ground_z + Pf::b(0x3e4c_cccd);
            self.f15f4 = Pf::b(0x3e4c_cccd);
        }
        if sid == 0 {
            self.f15f4 = self.water_level - self.ground_z;
            if self.f15f4 < Pf::b(0x3f59_999a) && Pf::b(0x3e80_0000) < self.f15f4 { self.f13f9 = 1; }
            self.f0634 = 1;
        }
    }
}
