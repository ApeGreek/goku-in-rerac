//! **Package P3 — ledges and the wall jump** (stub: the dispatch routes here; nothing is ported yet).
//! Owner: the P3 agent (docs/plan/hero_states.md "Packages"). Addresses level01 unless noted.
//!
//! States (all `ported: false` in [`super::registry::STATES`] until this module implements them):
//! * 0x18 ledge grab (group 3, anim 0x20; from the fall 6, the jumps and the glide 8 when `0x13f838` is set),
//!   0x19 hang (anim 0x21), 0x1a / 0x1b shimmy left / right (anims 0x24 / 0x25), 0x1c climb / jump up
//!   (jump group, h 2.0, takeoff 10, anim 0x22), 0x11 wall jump (jump group, h 3.3, takeoff 9, anim 0x23;
//!   ✕ within 7 ticks while the wall window `0x13f504` is open).
//! * Probes: `HeroWallLedgeCheckA` 0x22c9a0 (end of the jump / glide physics: [`wall_ledge_probe_a`]),
//!   `HeroWallLedgeCheckB` 0x22d090 (every tick after the surface reaction: [`wall_ledge_probe_b`]),
//!   `HeroWallLedgeCheckC` 0x20c758 (the hang / shimmy transitions). They set 0x13f504 (wall window,
//!   `Hero::ledge`), 0x13f838 (ledge found, `Hero::f838`), 0x13f820 / 0x13f834 (ledge point / yaw), 0x13f848 (the
//!   moby the ledge belongs to: the platform carry follows it in group 3 and state 0x1c, super::platform).
//! * Physics 0x2370b8 cases 0x18 / 0x19 (turn to the ledge yaw + π, pull to 0x13f820, `ClampLen(4·dt)`) and
//!   0x1a / 0x1b (the shimmy along two probes, speed from the anim table 0x1c4130); transitions 0x242930 cases
//!   0x18 → 0x19 after frame 13.5, 0x19 (✕ → 0x1c, stick back + ✕ → 6, the shimmy probes, R1 → 6 with 0x13f500 =
//!   10), 0x1a / 0x1b.
//!
//! Seams already wired in the shared files (behaviour-neutral stubs until ported): the fall 6 and the jumps take
//! 0x18 when `Hero::f838 != 0` (air.rs, jump.rs), the jump transitions call [`wall_jump`], the jump physics
//! calls [`wall_ledge_probe_a`], `hero_update` calls [`wall_ledge_probe_b`]. 0x11 and 0x1c are jump-group
//! states: build their entry with `Hero::jump_block_defaults` + their parameters, and run
//! `Hero::phys_jump` / `Hero::tr_jump` (jump.rs) for their physics / transitions.
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 0x11, 0x18..0x1c. `None`: continue with SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

/// `HeroWallLedgeCheckA` 0x22c9a0, at the end of the jump-group physics (and of the glide 8's).
pub(super) fn wall_ledge_probe_a(_h: &mut Hero, _env: &Env) {}

/// `HeroWallLedgeCheckB` 0x22d090, every hero tick after the surface reaction 0x22cd48.
pub(super) fn wall_ledge_probe_b(_h: &mut Hero, _env: &Env) {}

/// The wall jump test of the jump transitions (0x13f504 open, not landed, 7 / 9 / 0x11, ✕ within 7 ticks →
/// 0x11). True when it changed the state.
pub(super) fn wall_jump(_h: &mut Hero, _c: &mut Ctx) -> bool { false }
