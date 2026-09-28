//! The game's per-frame draw-callback lists: `RegisterDrawCallback` 0x21afe0 (list 1, drained by 0x21b030 after
//! the mobys and before the particles) and `RegisterDrawCallback2` 0x21b198 (list 2, drained by 0x21b1e8 after the
//! particles). A moby update registers `(fn, moby)` during the tick; the frame render calls them in registration
//! order; `0x2ab920` clears both lists at the start of the next tick. docs/plan/world_animation.md §0, §3.
//!
//! Most callbacks only draw (the water strips and ripples of `rc-engine`'s water renderer). Some also change game
//! state in the draw, with `rand` draws on the one shared stream: the fire / smoke fields 760 (`0x2fe080`:
//! flame timers, respawns). The port keeps the registrations here ([`DrawCallbacks`], in `Services`) and runs the state
//! part of every registered callback in [`run_frame`] at the start of the next tick's moby loop
//! (`Scheduler::tick`): on the PS2 that is the end of the frame render, and nothing between it and the moby loop
//! draws from the stream (the pad read and the free-slot pass do not), so the draws land at the game's place.
//! The renderer draws the current tick's registrations with the state as the last [`run_frame`] left it (one frame
//! behind the PS2's draw-time update: the fade and the scroll lag by one step, invisible at 60 fps).
//!
//! Frame pacing: the PS2 drains the lists once per rendered frame, so on a lag frame with a catch-up tick (two
//! ticks, one frame) it runs the last tick's callbacks once. The port runs them once per tick, which keeps the
//! effect's speed independent of the frame rate (identical in the deterministic one-tick-per-frame mode).

use crate::moby_runtime::MobyId;
use crate::moby_update::services::World;

/// The callbacks the port knows (the game's function pointers, by what they are).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Callback {
    /// The fire / smoke field 760 (level01 `0x2fe080`, list 2): [`super::fire_field`].
    FireField760,
    /// The nanotech cluster 806's glow (level01 `0x301c00`, list 1; draw only): [`super::pickup::nanotech_glow`].
    NanotechGlow,
    /// The ships' canopy glass (level01 `0x2a70a8`, the boot's `0x2327a0`; draw only, apart from its cross-fade timer):
    /// registered by the cutscene FX driver on list 1 with the matrix of the ship's joint list 0 (its canopy joint) ([`DrawCallbacks::matrices`]).
    ShipGlass,
    /// The ripple patches of the level's ripple module (751's `0x2fd0c0` on level 01, the patch managers' callbacks on
    /// 05 / 07 / 11 / 12 / 13: `FUN_002b91c8(table, n)`; draw only, apart from the per-patch UV advance the renderer
    /// makes): `crate::water::managers`.
    RipplePatches,
    /// The Gadgetron vendor 11's beam and glow points (level01 `0x2ba9c0`, list 2; draw only): registered by its update
    /// in states 1 and 2 ([`super::vendor`]). `rc-engine` draws the four glow points in `fx_draw` (through the shared glow
    /// quad `0x2781d0`) and the beam in `vendor_render`.
    VendorBeam,
    /// A sea / liquid surface of `crate::water::sea::PORTS[i]` (draw only; list 1 or the after-ties list by port).
    Sea(u8),
}

/// The lists (registration order).
#[derive(Clone, Debug, Default)]
pub struct DrawCallbacks {
    /// `0x21afe0` (after the mobys, before the particles).
    pub list1: Vec<(Callback, MobyId)>,
    /// `0x21b198` (after the particles).
    pub list2: Vec<(Callback, MobyId)>,
    /// `0x16e100` (count `0x15f42c`): drained by `DrawWorld` after the ties and before the shrubs (`RunDrawCallbacks_2`
    /// 0x21b0a8; registered by the levels' own copies of the register function, e.g. level05 `0x228110`).
    pub ties: Vec<(Callback, MobyId)>,
    /// For the draw-only callbacks that draw in a joint's frame (the ship glass: `0x264508(m, 0, M)`): the matrix
    /// (rows x, y, z, point) of the registering moby this tick, taken where the port has the pose (a scene actor's).
    pub matrices: std::collections::HashMap<MobyId, [[f32; 4]; 4]>,
}

impl DrawCallbacks {
    /// `RegisterDrawCallback` 0x21afe0 (64 entries; the game drops registrations past the end).
    pub fn register(&mut self, cb: Callback, id: MobyId) {
        if self.list1.len() < LIST_LEN { self.list1.push((cb, id)); }
    }

    /// [`Self::register`] with the moby's joint-list-0 matrix (`0x264508(m, 0, M)`) for the draw.
    pub fn register_with_matrix(&mut self, cb: Callback, id: MobyId, m: [[f32; 4]; 4]) {
        if self.list1.len() < LIST_LEN {
            self.list1.push((cb, id));
            self.matrices.insert(id, m);
        }
    }

    /// `RegisterDrawCallback2` 0x21b198 (64 entries).
    pub fn register2(&mut self, cb: Callback, id: MobyId) {
        if self.list2.len() < LIST_LEN { self.list2.push((cb, id)); }
    }

    /// The after-ties list's register function (level05 `0x228110`, 64 entries).
    pub fn register_ties(&mut self, cb: Callback, id: MobyId) {
        if self.ties.len() < LIST_LEN { self.ties.push((cb, id)); }
    }
}

/// Entries per list (`0x21afe0` / `0x21b198` refuse the 65th).
pub const LIST_LEN: usize = 64;

/// The frame render's callbacks of the last tick, then the lists cleared (`0x2ab920`): the after-ties list, list 1, then
/// list 2, each in registration order. Called first thing in the moby loop (see the module doc).
pub fn run_frame(w: &mut World) {
    let lists = std::mem::take(&mut w.svc.draw_callbacks);
    for (cb, id) in lists.ties.into_iter().chain(lists.list1).chain(lists.list2) {
        if w.table.mobys.get(id).is_none_or(|m| m.state >= 0x80) { continue; }
        match cb {
            Callback::FireField760 => super::fire_field::draw_callback(w, id),
            // Draw only: no game state, no `rand` (crate `rc-engine` fx_draw).
            Callback::NanotechGlow | Callback::ShipGlass | Callback::RipplePatches | Callback::VendorBeam | Callback::Sea(_) => {}
        }
    }
}
