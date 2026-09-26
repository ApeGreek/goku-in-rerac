//! **Package P6 (with the Swingshot) — the weapon check's other hand items** (stub). `HeroPdaGadget` 0x240ed8 is a
//! switch on the hand item (`0x140408`); the wrench (item 8) is ported in [`super::melee`]. This module takes the
//! other cases: the Swingshot 12 (→ 0x24 / 0x2c, super::swingshot), the Hologuise 0x1f (the 18-tick timer
//! 0x14162e that swaps to the disguise body), the PDA / vendor 0x20 (`OpenVendorMenu`), 0x15 (0x22ee08), the
//! gadget draw 0x20 state; and the holster check 0x2405f8 of the transitions' prologue. Owner: P6.
//!
//! Contract with [`super::packs::pda_epilogue`]: the game runs the Thruster hover test at the end of 0x240ed8
//! whenever the state did not change; [`pda_item`] must call it on that path (the wrench path already does).
#![allow(dead_code)]

use super::states::Ctx;
use super::Hero;

/// `HeroPdaGadget` 0x240ed8 for a ready hand item other than the wrench (the slot checks have passed). `t0` is
/// the state timer at entry; returns true when the state changed (`timer < t0`).
pub(super) fn pda_item(h: &mut Hero, _c: &mut Ctx, t0: i32) -> bool { h.timer < t0 }

/// The holster check 0x2405f8 (after the water checks). True when it changed the state.
pub(super) fn holster_check(_h: &mut Hero, _c: &mut Ctx) -> bool { false }
