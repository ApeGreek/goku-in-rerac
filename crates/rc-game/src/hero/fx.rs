//! **Hero polish — the hero's effects on the rest of the game**: the requests the hero code makes of systems it does
//! not own, queued during the hero update in the game's order and handed over by the tick (`crate::tick`).
//!
//! * **Camera shake** ([`HeroFx::shakes`]): the hero code's stores into the camera's shake records 0x167260 /
//!   0x167270 (`crate::follow_camera::Shake`), e.g. the Thruster stomp's landing (0.2 along up for 40 ticks,
//!   level01 0x2390a0). The tick hands them to [`crate::follow_camera::Camera::request_shake`] right after the hero
//!   update; the camera update of the same tick applies them, as in the game.

use crate::follow_camera::{ShakeAxis, ShakeRequest};

/// The hero's queued effects (drained by the tick).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeroFx {
    /// Camera shake requests of this tick, in order (the last one of an axis wins, as the stores do).
    pub shakes: Vec<ShakeRequest>,
}

/// A camera shake request (the writer's stores into 0x167260 for [`ShakeAxis::Up`], 0x167270 for
/// [`ShakeAxis::Forward`]): `amp` units for `ticks` ticks.
pub fn shake(h: &mut super::Hero, axis: ShakeAxis, amp: f32, ticks: i32) { h.fx.shakes.push(ShakeRequest { axis, amp, ticks }); }
