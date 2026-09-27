//! The scripted hold 0x72 (docs/plan/cutscenes.md §3.4): the state the in-level cinematics put Ratchet in while
//! the script camera is up (`CameraTriggerUpdate` 0x2fb5b0 through `HeroTeleport(…, 0x72, 1)`, `GunshipUpdate`
//! 0x2f7728 through `SetState(0x72, 0)`); they end it with `SetState(0, 1)` (`FUN_002405a0` for the trigger).
//!
//! * **Entry** (`SetState` 0x23cf98 case 0x72): group 9, 0x1413fc = 1 (no control: the weapon check and the hand
//!   items refuse), camera mode 0x1415d4 = 0, the weapon put away (`0x22efd8`), then — whatever `play` says, the
//!   case `break`s into the common `SetAnim` — the idle sequence `0x226f10(0)` blended over 10 ticks.
//! * **Physics** (`0x2370b8`): the idle group's case (0, 1, 3, 4, 0x1d, 0x1e, 0x1f, 0x72, …): the drag, the edge
//!   brake, `StickTarget` — he stands and settles, gravity applies.
//! * **Transitions** (`0x242930`): no case, only the prologue (hits, death height). Nothing leaves 0x72 but the
//!   script's own `SetState`.

use super::common::blend;
use super::physics::Env;
use super::states::Ctx;
use super::{AnimCtl, Hero};
use crate::rng::Rng;

/// The scripted hold.
pub const HOLD: i32 = 0x72;

/// SetState's entry of 0x72. `None`: SetState's epilogue follows.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, _play: bool, _old_sub: i32) -> Option<bool> {
    if id != HOLD { return None; }
    h.group = 9;
    h.items.f13fc = 1;
    h.f15d4 = 0;
    super::weapons::put_away(h);
    let seq = h.idle_seq();
    h.set_anim(c.anim, c.rng, blend(10), seq, 0);
    None
}

/// 0x2370b8's idle case.
pub(super) fn physics(h: &mut Hero, env: &Env, anim: &mut dyn AnimCtl, rng: &mut Rng) -> bool {
    if h.state != HOLD { return false; }
    h.phys_ground(env, anim, rng);
    true
}

/// 0x242930 has no case for 0x72.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}
