//! **Package P6 — the Swingshot** (stub: the dispatch routes here; nothing is ported yet). Owner: the P6 agent
//! (docs/plan/hero_states.md "Packages"), which also owns [`super::gadgets`] (the non-wrench branches of the
//! weapon check 0x240ed8, where the Swingshot fires). Addresses level00 (the Swingshot states exist on every
//! level but Novalis).
//!
//! * Item 12 in hand, a target moby in range (the target search of the weapon check). Two target kinds:
//!   **pull** targets (group 0xd, target moby 0x13fcb4): 0x24 fire (turn to the target, anim 0x2f, loop 10..14),
//!   0x25 pull (fly to the target, anim 0x30), 0x26 arrive / let go (distance kept in 0x13fccc), then 0 or 6;
//!   **swing** targets (group 0xe, target moby 0x13fce0, its pvar +0x78 record: +0x04 id, +0x0c / +0x10 / +0x14
//!   rope length, +0x24 / +0x28 / +0x2c spring k, d, max): 0x2c swing (anim 0x34 / 0x35, gravity 27·dt², rope
//!   length 0x13fcf4, yaw to the target 0x13fcfc), release → 0x2d (group 2, the fall after a swing: anim 0xb
//!   blend 15; shares the fall transitions and physics case with 6).
//! * The Euler x/y straightening 0x236520 skips 0x25 / 0x26 / 0x2c / 0x2d (the body tilts on the rope).
//!
//! No seams in the shared files: 0x24 / 0x2c are entered from [`super::gadgets::pda_item`]; 0x2d reuses
//! `Hero::phys_fall` / `Hero::tr_fall` (air.rs) plus its own branches.
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 0x24..0x26, 0x2c, 0x2d. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}
