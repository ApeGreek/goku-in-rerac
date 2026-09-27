//! Class 815, the enemy spawner (`EnemySpawnerUpdate` 0x3021b8, level01 only). Spec `docs/plan/creatures.md`
//! "Spawner 815".
//!
//! Pvars (0x40): +0x00 moby group (−1 none), +0x08 spawns so far. State 0 → 1; in state 1, while its group has no
//! member left that is alive and not in state 0xc (`MobyGroupCount(g, 0xc)` 0x26e008) and fewer than
//! `gp−0x4d70` = 5 spawns were made, it re-creates a member of the group at its own place (`0x302328`: the member
//! reset as an amoeboid — size +0x250 = 1, health 3, walker radius, range 30 — and thrown out away from Ratchet by the
//! knockback record +0x60, state 9).
//!
//! **On Novalis all six spawners have group −1**: they only make the 0 → 1 step (verified from the pvars). The
//! re-creation `0x302328` / `0x26e150` is therefore not ported (counted if a spawner with a group ever runs).

use crate::moby_runtime::MobyId;
use crate::moby_update::creature as c;
use crate::moby_update::services::World;

pub const UPDATE_FN: u32 = 0x3021b8;
pub const CLASSES: [i16; 1] = [815];
/// `gp−0x4d70` (0x161e90): the spawn limit.
pub const MAX_SPAWNS: i32 = 5;

/// `MobyGroupCount(g, skip_state)` 0x26e008: the members of group `g` that are alive (state < 0x80) and not in
/// `skip_state` (−1: count every live member).
pub fn group_count(w: &World, g: i32, skip_state: i32) -> i32 {
    let Some(Some(list)) = usize::try_from(g).ok().and_then(|g| w.svc.groups.lists.get(g)) else { return 0 };
    list.iter()
        .filter_map(|&e| w.table.mobys.get((e & 0x7fff) as usize))
        .filter(|m| m.state < 0x80 && (skip_state == -1 || m.state as i32 != skip_state))
        .count() as i32
}

/// `EnemySpawnerUpdate` 0x3021b8.
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0xc { return; }
    match w.m(id).state {
        0 => w.mm(id).state = 1,
        1 => {
            let g = c::pi32(w, id, 0);
            if g < 0 || group_count(w, g, 0xc) != 0 || MAX_SPAWNS <= c::pi32(w, id, 8) { return; }
            let n = c::pi32(w, id, 8) + 1;
            c::set_pi32(w, id, 8, n);
            w.svc.unported("enemy spawner 815: group re-creation 0x302328");
        }
        _ => {}
    }
}
