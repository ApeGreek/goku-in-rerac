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
//!
//! **The scene body 99 / 100** (level01 `SetState` case 99 / 100, `0x2370b8` case 99 / 100; no case in `0x242930`):
//! the state a mode-2 scene (`SetState(100, 2)`, docs/plan/cutscenes.md §7) and the vendor (`OpenVendorMenu`:
//! `SetState(100, 1)`) put Ratchet in; they end it with `SetState(0, 1)`.
//! * **Entry**: group 0x18, 0x1413fc = 1 (no control), the weapon put away (`0x22efd8`); with `play`, the idle sequence
//!   `0x226f10(0)` on the eased curve 1 (`SetAnim(−1, …)`, 11 ticks).
//! * **Physics**: the velocity 0x13f430 zeroed (no gravity, no drag): he stays where the scene found him.
//! * **Transitions**: none. The gameplay Ratchet is never seen in this state: the scene hides him and his items
//!   (`FUN_002486c0`) and draws its own Ratchet actor; the vendor hides him too.

use super::common::blend;
use super::physics::Env;
use super::states::Ctx;
use super::{AnimCtl, Hero};
use crate::rng::Rng;

/// The scripted hold.
pub const HOLD: i32 = 0x72;

/// The scene body (99 is the same case).
pub const SCENE: i32 = 100;

/// SetState's entry of 0x72, 99 and 100. `None`: SetState's epilogue follows.
pub(super) fn entry(h: &mut Hero, c: &mut Ctx, id: i32, play: bool, _old_sub: i32) -> Option<bool> {
    if id == SCENE || id == SCENE - 1 {
        h.group = 0x18;
        h.items.f13fc = 1;
        super::weapons::put_away(h);
        if play {
            let seq = h.idle_seq();
            h.set_anim(c.anim, c.rng, crate::ps2v::Pf::b(0xbf80_0000), seq, 0);
        }
        return None;
    }
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
    if h.state == SCENE || h.state == SCENE - 1 {
        h.vel = super::physics::V0;
        return true;
    }
    if h.state != HOLD { return false; }
    h.phys_ground(env, anim, rng);
    true
}

/// 0x242930 has no case for 0x72.
pub(super) fn transitions(_h: &mut Hero, _c: &mut Ctx) {}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::super::HeroTick;
    use super::*;
    use crate::pad::PadInput;

    /// `SetState(100, 2)` (a scene): group 0x18, no control, the idle sequence on the eased curve; the hero update
    /// runs (nothing unported) and keeps him still whatever the pad does; `SetState(0, 1)` gives him back.
    #[test]
    fn scene_body_holds_ratchet_still() {
        let coll = floor(100.0, 100, 106, 100, 106);
        let mut r = Runner::new([410.0, 410.0, 100.0], 0.0);
        r.run(&coll, PadInput::neutral(), 5);
        assert!(r.with_ctx(&coll, |h, c| h.set_state(c, SCENE, true)));
        assert_eq!((r.hero.state, r.hero.group, r.hero.items.f13fc), (SCENE, 0x18, 1));
        assert_eq!(r.anim.calls.last().map(|c| (c.0, c.2)), Some((crate::ps2v::Pf::b(0xbf80_0000), 0)));
        let at = r.hero.position();
        for _ in 0..60 {
            assert_eq!(r.tick(&coll, PadInput::neutral().stick(0.0, -1.0).press(crate::pad::button::CROSS)), HeroTick::Ran);
        }
        assert_eq!(r.hero.state, SCENE, "no transition leaves the scene body");
        let p = r.hero.position();
        assert!((0..3).all(|k| (p[k] - at[k]).abs() < 1e-4), "moved {at:?} → {p:?}");
        assert!(r.with_ctx(&coll, |h, c| h.set_state(c, 0, true)));
        r.run(&coll, PadInput::neutral().stick(0.0, -1.0), 30);
        assert_eq!(r.hero.state, 2, "control is back");
    }
}
