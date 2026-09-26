//! **Package P1 — the platform carry** (stub: [`platform_update`] is called at the start of the move, as in the
//! game; it does nothing yet). Owner: the P1 agent, which also owns [`super::surface`] (docs/plan/hero_states.md
//! "Packages"). Addresses level01.
//!
//! `HeroPlatformUpdate` 0x249618, the first call of the move pipeline 0x233de0 (`HeroMovePipeline`): pick the
//! platform 0x13f6b0 (the ground moby 0x13f64c when grounded; the ledge moby 0x13f848 in group 3 and state 0x1c;
//! while airborne the last one, faded over `ticks(120)` of air); return unless it is a carrier
//! (`moby_update::triggers::platform_block`, 0x275290); record Ratchet's point in its local space when idle
//! (groups 0 / 0xc, no momentum, grounded, 0x13f546 = 0) or hanging (0x18 / 0x19), flag bits 0x13f6b4; each tick
//! the carried point (`triggers::carry_point` / `platform_delta`) or the recorded local point through the
//! platform's rows; the correction (> 2 drops the attachment, < 1e-4 snaps) into 0x13f440 (`Hero::platform`,
//! which the ported move already adds and re-collides) and the yaw change into 0x13f44c; airborne without block
//! flag bit 1 the last delta decays (25·dt² vertical, 2·dt² horizontal). Spec: docs/plan/triggers.md §5.
//! Consumers waiting for it: the path lift 726 (`moby_update/classes/path_platform.rs`), elevators 703 / 715.
#![allow(dead_code)]

use super::physics::Env;
use super::Hero;

/// `HeroPlatformUpdate` 0x249618 (start of the move 0x233de0): adds the carrier's move to `Hero::platform`.
pub(super) fn platform_update(_h: &mut Hero, _env: &Env) {}
