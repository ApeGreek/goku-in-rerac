//! **Package P5 — boots and spline riding: Magneboots, Grind Boots, the cable slide** (stub: the dispatch routes
//! here; nothing is ported yet). Owner: the P5 agent (docs/plan/hero_states.md "Packages"). Addresses level01
//! unless noted (the grind and cable code only exists in the overlays of the levels with rails: level00 is the
//! reference).
//!
//! * **Magneboots** (item 28, owned flag 0x13d4dc): surface 2 sets 0x140637 (`Hero::f0637`); standing on it with
//!   the boots switches the gravity mode `0x141403` to 1 (0x248ad8: states 0x3f / 0x71 / 0x70, or state 0 with
//!   fewer than 4 air ticks on surface 2 with the boots) and the gravity frame 0x13f5e0 follows the floor normal.
//!   `0x13f658` (`Hero::f658`) is set by the surface reaction when the ground moby is class 0xad. States: 0x3f
//!   walk (group 1, anim 0x5b), 0x70 wrench swing (group 6, anim 0x5c + step; picked by the weapon check when
//!   f658 = 1), 0x71 jump / hop (group 1; ✕ in idle / crouch with f658 = 1). The ground physics' magnetic branch
//!   ([`ground_magnet`]). Note: the port's gravity-mode rule in `Hero::input_physics_move` differs from 0x248ad8
//!   (it forces mode 1 with ≥ 4 air ticks and does not test the boots); unreachable while f0637 is only set on
//!   surface-2 floors, to fix with this package.
//! * **Grind Boots** (item 29): the grind paths (gameplay section 0x74, `rc_formats::volumes::GrindPath`),
//!   `SplineProject` / `SplineSample`, the rail contact 0x13f8bc (walk and fall take 0x28). States: 0x28 grind
//!   (group 0xf, anims 0x31 / 0x32 by stance), 0x29 grind jump (jump block, h 2.4, +4.7 on a booster 0x140638),
//!   0x2a rail-switch jump (h 2.5, anim 0x1e / 0x1f by side), 0x2b grind wrench swing (anims 0x4e / 0x4f, wrench
//!   seq 0xc / 0xd), 0x42 grind hurt (anim 0x10, curve −3). One physics case and one transitions case for all
//!   five in the game (level00 0x217970 / 0x229b70).
//! * **Cable slide** 0x74 (group 0x1a, anims 0x73 / 0x66; caught from the fall when 0x13f94c is set): the same
//!   spline follower (`SplineSample` + `SplineProject`, speed → 14 u/s at 9·dt², a spring hanging offset);
//!   identity inferred (L).
//!
//! Seams already wired (behaviour-neutral stubs): [`ground_magnet`] (ground physics), [`rail_contact`] (walk and
//! fall transitions → 0x28), [`cable_contact`] (fall → 0x74); walk 2 → 0x3f on `Hero::f658` is plain code in
//! walk.rs. 0x29 / 0x2a are jump-group entries: `Hero::jump_block_defaults` + parameters.
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 0x28..0x2b, 0x3f, 0x42, 0x70, 0x71, 0x74. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

/// The ground physics' magnetic-floor branch (0x140637 with the Magneboots): true when it replaced the rest of
/// the ground physics (wall check and gravity).
pub(super) fn ground_magnet(_h: &mut Hero, _env: &Env) -> bool { false }

/// A grind rail under / at the hero (0x13f8bc ≠ 0).
pub(super) fn rail_contact(_h: &Hero) -> bool { false }

/// A cable caught from the fall (0x13f94c ≠ 0).
pub(super) fn cable_contact(_h: &Hero) -> bool { false }
