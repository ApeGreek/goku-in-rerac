//! The Gaspar help director, class 1000 (census U305; level09 `0x300888`, one instance). Read from the level09
//! decomp and disassembly (the two "unreachable" blocks are the `div` traps).
//!
//! **Pvars**: +0x00 the cuboid of the hint.
//!
//! | address | what | port |
//! |---|---|---|
//! | 0x3008a4 | tick 0x15f5cc > 0: the level word gp−0x5838 (0x1613c8; each tethered platform 1182–1189 adds one per update, `0x2c26c8`) = 0 and skill point 0x13d416 not earned → skill point, `PlayLevelSoundAtMoby(1, 0, 0)`, `ShowBanner(0x53d6, −1)`; the word := 0 | NOT ported (G-SAV-007 [deferred]: the award, its counter and the platforms' increment) |
//! | 0x3008fc | `H[0x5b]` count 0: item 11 owned (0x13d4cb) → count := 1; `H[0x5b]` time / level mask refreshed | [`update`] |
//! | 0x3009a0 | then, in cuboid +0x00 → `Help_Request(9001, 0x5b)` | [`update`] |
//! | no sound (besides the skill point's), particle, flag, save, other moby written | | n/a |

use super::hints::{in_cuboid, owned, request, touch};
use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x30_0888;
pub const REFERENCE_LEVEL: u32 = 9;
pub const CLASSES: [i16; 1] = [1000];

/// Level09 0x300888 (module doc).
pub fn update(w: &mut World, id: MobyId) {
    if w.svc.help.records.help[0x5b].count != 0 { return; }
    if owned(w, 11) { w.svc.help.records.help[0x5b].count = 1; }
    touch(w, 0x5b);
    if in_cuboid(w, id, 0) { request(w, 9001, 0x5b); }
}
