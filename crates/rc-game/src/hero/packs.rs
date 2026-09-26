//! **Package P4 — Clank's packs: Heli-Pack and Thruster-Pack** (stub: the dispatch routes here; nothing is
//! ported yet). Owner: the P4 agent (docs/plan/hero_states.md "Packages"); P4 also owns `jump.rs` during its
//! window. Addresses level01 unless noted.
//!
//! The pack on Clank is the ready item of item slot 3 (`GetClankModule(3)` 0x22ddd8 = `0x140408 + 3·0x50` when
//! slot state `0x140404 + 3·0x50` = 2): 2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack. Ownership: the game state's
//! item table `0x13d4c0 + id` (Heli-Pack 2 = 0x13d4c2, Thruster-Pack 3 = 0x13d4c3). `0x141628` (Clank hidden)
//! disables every pack move.
//!
//! States (`ported: false` in [`super::registry::STATES`] until implemented here):
//! * 8 glide / hover (group 5, anim 0x13 curve −2): ✕ held in the air with the Heli-Pack (fall 6 above 0.6,
//!   jumps above 0.6 / 1.0 / 1.5); physics 0x2370b8 case 8 (target speed 3·dt (5·dt on 0x1404f8 = 3), TurnTo
//!   (0.025, 0.3, 720°/s), SpeedStep(15, 7)·dt², vz from 0x248f68 / 0x248b68, the pack's sound slots 0x141574 /
//!   0x141578, `HeroWallLedgeCheckA`); transitions case 8 (→ 6 after 30 ticks without ✕, → 0x18 / 0x11, the
//!   landing into 3 / 4 / 0 / 2 with 0x13f524 = 12 − timer).
//! * 10 Heli-Pack long jump, 0xf Heli-Pack high jump, 0x10 Thruster-Pack long jump, 0xd Thruster-Pack high jump
//!   (jump group; `HeroCrouchJumps` 0x242420: crouch + ✕ moving → 10 / 0x10, standing → 0xf / 0xd; an R1 tap
//!   in the first 7 ticks of 7 / 9 → 10 / 0x10); 0x22 Thruster stomp (group 0xb: R1 in the air); 0x7a pack
//!   jump wall rebound (group 10); 0x81 Thruster-Pack hover (group 1: R1 double tap on the ground, the latch
//!   0x14161a keeps idle → 0x81).
//! * The heli branch of the double-jump boost (0x2345f0 with module 3), the pack models / anims on Clank
//!   (`super::idle`'s back table rows 3 / 4, the Hydro-Pack model swap noted in swim.rs).
//!
//! Seams already wired (behaviour-neutral stubs): [`crouch_jump`] (common.rs), [`hover_latched`] (idle
//! transitions), [`fall_to_glide`] / [`fall_to_thruster`] (fall), [`jump_pack_moves`], [`jump_crouch_press`],
//! [`pack_jump_wall_hit`], [`heli_long_jump_landed`], [`thruster_fallover`], [`jump_to_glide`] (jump
//! transitions), [`pda_epilogue`] (the end of the weapon check 0x240ed8). `Hero::f524` (0x13f524, the jump
//! buffer override the glide sets) is already read by the idle / walk / fall jump tests and counted down.
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 8, 10, 0xd, 0xf, 0x10, 0x22, 0x7a, 0x81. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

/// `HeroCrouchJumps` 0x242420 after the ✕ test: the pack jumps 10 / 0x10 / 0xd / 0xf (true: handled, including
/// the Thruster lockout 0x13f508 that refuses without a state change); false → the plain jump 7.
pub(super) fn crouch_jump(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// Idle with the hover latch 0x14161a set (and not on a magnetic floor with the Magneboots) → 0x81.
pub(super) fn hover_latched(_h: &Hero) -> bool { false }

/// The fall 6's glide test (✕ held, Heli-Pack owned, the back module not the Hydro-Pack, Clank shown, no
/// lockout, height > 0.6 → 8). True when it changed the state.
pub(super) fn fall_to_glide(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// The fall 6's Thruster test (✕ + R1 combo within 9 ticks, gap 8 → 0x10). True when it changed the state.
pub(super) fn fall_to_thruster(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// The jump transitions' pack moves after the flip chain (Thruster ✕ within 8 ticks with R1 held → 0x10; an
/// R1 tap within 2 ticks in the first 7 ticks → 10 / 0x10). True when it changed the state.
pub(super) fn jump_pack_moves(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// R1 / R2 pressed within 9 ticks in the air (the pass ends after this either way): the Thruster stomp 0x22.
pub(super) fn jump_crouch_press(_h: &mut Hero, _c: &mut Ctx) {}

/// 10 / 0x10 hitting a wall (not landed, `0x13f598 < 0.9`, capsule hit, |vel| > 7·dt, the move stopped) →
/// 0x7a. True when it changed the state.
pub(super) fn pack_jump_wall_hit(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// The Heli long jump 10 once landed: SetState(3, no anim) + skid anim 5 (curve −2), momentum × 0.8.
pub(super) fn heli_long_jump_landed(_h: &mut Hero, _c: &mut Ctx) {}

/// 0x10 after 60 ticks above 1.5 → 6. True when it changed the state.
pub(super) fn thruster_fallover(_h: &mut Hero, _c: &mut Ctx) -> bool { false }

/// The jumps' glide test when no double jump was taken (Heli-Pack, ✕ held, height above 0.6 / 1.0 (7, 9) /
/// 1.5 (0xe), timer past 20 (8 for 0x10) and 0x13f7f6 → 8).
pub(super) fn jump_to_glide(_h: &mut Hero, _c: &mut Ctx) {}

/// The end of `HeroPdaGadget` 0x240ed8 when it did not change the state: the Thruster hover 0x81 (R1 double tap
/// on the ground, timer gp−0x758c) or, without the Thruster, the latch 0x14161a cleared.
pub(super) fn pda_epilogue(_h: &mut Hero, _c: &mut Ctx) {}
