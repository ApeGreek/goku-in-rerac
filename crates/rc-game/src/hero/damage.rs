//! **Package P2 — damage, knockback and death** (stub: the dispatch routes here; nothing is ported yet). Owner:
//! the P2 agent (docs/plan/hero_states.md "Packages"), which also owns [`super::stance`]. Addresses level01
//! unless noted (level00's hit dispatcher is 0x210ce8).
//!
//! * **Hit intake** 0x231580 (the first call of the transitions, [`hit_intake`]): the moby hit record of
//!   Ratchet's moby (+0xa4 slot → 0x178110 + 0x40·slot: attacker, flags, damage, direction), the invulnerability
//!   timer 0x13f510 (`Hero::f510`, 77 ticks after a hit), the hurt state by group (0x16 on foot, 0x75 on the water
//!   surface, 0x76 under water, 0x42 on a grind rail (super::boots), 0x80 / 0x82 for the hazard classes 0x4eb /
//!   0x558 / 0x28f, the other bodies' 0x46 / 0x56 / 0x5d), the knockback `0x210c80(speed, up, dir)` into
//!   0x13f680 / 684 / 688 (`Hero::knock`, read by the ported push generator 0x233850), `HeroTakeDamage` (health
//!   0x1415f8), the hit flash 0x13f53e (`Hero::f53e`).
//! * States: 0x16 hurt (group 7, anim 0x10 curve −3), 0x75 / 0x76 water hurts (→ 0x37 / 0x34, or 0x6a without
//!   health), 0x3d death (health < 1, group 0x14, anim 0x45), 0x77 death fall (below the level's death height;
//!   shares 6's entry: gravity 24·dt², the random spin 0x13fca0 / 0x13fca4 — two RNG draws), 0x79 pit fall
//!   (surfaces 8 / 0xc via 0x14063a), 0x3c burn bounce (surface 1: damage 1 and a 5.5 jump; jump group), 0x7c burn
//!   death, 0x7f sinking death (surface 0xd), 0x80 / 0x82 hazard deaths. Every death ends in 0x2319b0 (deaths
//!   counter, fade to black, `Hero::fell_out` / 0x141401 = 1) when the sequence wraps or after its timer.
//! * The engine currently respawns when the hero *enters* 0x77 / 0x3d (gameplay.rs, owned by the engine side):
//!   when this package ports them, agree the handover (respawn on 0x141401 instead) with the engine owner.
//! * Also in this package: move the class-sound draws of Ratchet's animation triggers into the hero update
//!   (`hero_update_with_sounds` / [`super::HeroSounds`] is the hook; the engine still replays them in the sound
//!   step: docs/plan/hero_states.md P2).
#![allow(dead_code)]

use super::anim::AnimCtl;
use super::physics::Env;
use super::states::Ctx;
use super::Hero;
use crate::rng::Rng;

/// SetState entry of 0x16, 0x3c, 0x3d, 0x75, 0x76, 0x77, 0x79, 0x7c, 0x7f, 0x80, 0x82. `None`: continue with
/// SetState's epilogue.
pub(super) fn entry(_h: &mut Hero, _c: &mut Ctx, _id: i32, _play: bool, _old_sub: i32) -> Option<bool> { None }

/// Per-state physics; false = not ported (the hero freezes).
pub(super) fn physics(_h: &mut Hero, _env: &Env, _anim: &mut dyn AnimCtl, _rng: &mut Rng) -> bool { false }

/// Per-state transitions.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

/// The hit intake 0x231580 at the top of the transitions; true when it set a hurt state (the pass ends).
pub(super) fn hit_intake(_h: &mut Hero, _c: &mut Ctx) -> bool { false }
