//! Gold-weapon offers, classes 304 (the offer) and 1456–1465 (the props that go with each gold weapon):
//! `ItemOfferUpdate` level01 0x2e1ac0. Spec: `docs/plan/moby_update_catalogue.md` "In the port: teleporter pads
//! and item offers".
//!
//! Pvar+0x40 (s32) is the offer's index k into the gold-weapon table 0x1deb10 (level01 `.data`:
//! `[19, 10, 11, 16, 17, 15, 21, 20, 25, 9]`, item ids); the item is owned when `0x13e520[item]` ≠ 0.
//!
//! Per tick:
//! * **the challenge gate**: k ≠ −1, k even and the times-completed count `0x15ee20` = 0 → `DeleteMoby`, return
//!   (on a first playthrough the even offers and their props are gone after the load pass);
//! * state 0: class 304: owned → state 3; else the help-message registration `FUN_0027b480` (a free slot in the
//!   help table 0x1b1af8: pvar+4/+8/+9/+0x36/+0x38/+0x3c) → state 1 (−1 → state 3). The registration is **not
//!   ported**: the port takes the success branch (the Novalis savestate has the odd offers in state 1) and counts
//!   it. The props (1456–1465) → state 2;
//! * state 1: the offer (`FUN_0027b028`: the buy prompt, price 0x1c4544, the purchase: item owned, bolts −=
//!   price, `FUN_002aea70`, state 4) — **not ported** (counted);
//! * state 2: the prop is deleted once its gold weapon is owned;
//! * state 4: outside game mode 0 wait; else `memcard_Save(0, −1)` (not ported, counted) and state 3.

use crate::moby_runtime::MobyId;
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table.
pub const UPDATE_FN: u32 = 0x2e1ac0;

/// Classes that run [`update`].
pub const CLASSES: [i16; 11] = [304, 1456, 1457, 1458, 1459, 1460, 1461, 1462, 1463, 1464, 1465];

/// The offer class (0x130).
const OFFER: i16 = 304;

/// `0x1deb10`: offer index → gold weapon item id.
pub const GOLD_ITEMS: [i32; 10] = [19, 10, 11, 16, 17, 15, 21, 20, 25, 9];

fn owned(w: &World, k: i32) -> bool {
    let Some(&item) = usize::try_from(k).ok().and_then(|k| GOLD_ITEMS.get(k)) else { return false };
    w.svc.counters.gold_weapons.get(item as usize).is_some_and(|&b| b != 0)
}

/// `ItemOfferUpdate` (0x2e1ac0).
pub fn update(w: &mut World, id: MobyId) {
    let k = if w.m(id).pvars.len() >= 0x44 { p::i32(&w.m(id).pvars, 0x40) } else { -1 };
    if k != -1 && k & 1 == 0 && w.svc.counters.times_completed == 0 {
        w.delete_moby(id);
        return;
    }
    match w.m(id).state {
        0 => {
            let next = if w.m(id).o_class == OFFER {
                if owned(w, k) {
                    3
                } else {
                    w.svc.unported("item offer: help registration (assumed free)");
                    1
                }
            } else {
                2
            };
            w.mm(id).state = next;
        }
        1 => w.svc.unported("item offer: buy prompt"),
        2 => {
            if owned(w, k) { w.delete_moby(id); }
        }
        4 => {
            if w.svc.game_mode != 0 { return; }
            w.svc.unported("item offer: memcard save");
            w.mm(id).state = 3;
        }
        _ => {}
    }
}
