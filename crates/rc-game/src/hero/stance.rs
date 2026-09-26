//! **Package P2 (with damage) — stances and scripted walking** (stub: the dispatch routes here). Owner: P2
//! (docs/plan/hero_states.md "Packages"). Addresses level01.
//!
//! * 1 look stance (group 0, 0x1415d4 = 4): L1 / L2 held in idle, walk, stop or crouch (R&C1's first-person look;
//!   the camera side is follow_camera's). Physics = the ground case (0x23713c) with the camera-facing turn when
//!   0x1413f5 is set; transitions: release → idle, R1 / R2 → crouch 4; the anim is Ratchet's sequence
//!   `0x226f10(1)` (13 with crouch held). 0x1e is the same stance entered by mobys (four classes call SetState(0x1e)).
//! * 0x40 fidget state (fidget record 2 of `super::idle`'s table, the only record with a state) and 0x41 its end
//!   (0x241d98, the Clank-side 0x140964 blend).
//! * 0x65 / 0x66 / 0x67 walk to a point (0x140990, yaw 0x14099c; vendors, ship, cutscene marks): walk / stop /
//!   turn, `HeroWalkRunAnim` for the anim.
//!
//! No seams in the shared files: idle / walk / stop / crouch already take 1 (walk and crouch as the game does).
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 1, 0x1e, 0x40, 0x41, 0x65..0x67. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}
