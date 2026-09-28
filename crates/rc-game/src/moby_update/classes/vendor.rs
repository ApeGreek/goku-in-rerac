//! The Gadgetron vendor, class 11 (every level has one): its update, level01 0x2bb128 (not a function in the
//! Ghidra project: read from the disassembly, docs/plan/interaction.md §4). Native `f32`.
//!
//! Pvar block (V): +0x0c the hologram moby (class 1143; port: moby index + 1), +0x10 / +0x50 its two manipulator
//! records, +0x90 the hologram's scale 0..1. Globals: the glow phase 0x1613a4 and the manipulator phases
//! 0x16139c / 0x1613a0 ([`Globals`]).
//!
//! Every tick: glow phase += 0.05, wrapped past 3.14 to −3.14; glow `+0x90 = ((int)(sin φ · 48) + 96) · 0x010101 |
//! 0x80000000`. Then by state:
//! * **0** (init): state 1, V+0 = V+4 = 0; `CreateMoby(1143)` → V+0xc, its mode |= 2, draw distance 64, position =
//!   the vendor's + (0, 0, 2.95), +0x73 = 32, V+0x90 = 0, `AttachManipulator(child, i, V+0x10 + 0x40·i)` for i = 0, 1
//!   (not ported: counted), `MobyBuildMatrix(child)`.
//! * **1** (far): V+0x90 −= 0.1 while > 0; ≤ 0 hides the hologram (mode |= 1). Every 8th tick (0x15f5cc & 7 = 0):
//!   XY distance to the hero (`VecDistance2`) ≤ 16 and |Δz| ≤ 8 → state 2, seq 1 (blend 10) unless already on it,
//!   the hologram shown. Then the common tail.
//! * **2** (near): V+0x90 += 0.1 while < 1. Every 8th tick: XY > 18 or |Δz| > 10 → state 1, seq 0 (blend 10) unless
//!   on it, return. The common tail, then the prompt rule ([`interact::vendor_rule`]: XY ≤ 4, |Δz| ≤ 2, facing
//!   ≤ π/2, hero group 0 / 1 or state 3, not 0x1d / 0x32, control mode 0; opening also needs key A = seq 1) →
//!   `try_set_help_message(1, 21475 "△ Activate Gadgetron Vendor")`; △ (0x13cae4 & 0x10) with the lease →
//!   `OpenVendorMenu(vendor)` ([`Handoff::OpenVendor`]), state 3.
//! * **3** (vendor open; `VendorExit` sets 1 again): V+0x90 = 0, the hologram's scale 0.
//! * **Tail** (states 1 and 2): `RegisterDrawCallback2(0x2ba9c0, vendor)` (the hologram cone, not ported: counted),
//!   the manipulators' z rotations `FUN_00221e38(V+0x20, 2, a)` / `(V+0x60, 2, b)` (counted), a += 0.01, b −= 0.01,
//!   the hologram's scale = V+0x90 · class scale · 2.5, `MobyBuildMatrix(child)`.

use crate::moby_runtime::MobyId;
use crate::moby_update::interact::{self, add_rot, owner, Handoff, HeroView};
use crate::moby_update::services::{pvar as p, World};

/// The update address in the level01 class table (not a Ghidra function).
pub const UPDATE_FN: u32 = 0x2bb128;
pub const CLASSES: [i16; 1] = [11];
/// The hologram (`CreateMoby(0x477)`).
pub const HOLOGRAM: i16 = 1143;
/// 21475 "△ Activate Gadgetron Vendor".
pub const PROMPT: i32 = 21475;
/// V+0x0c, V+0x90.
const CHILD: usize = 0x0c;
const SCALE: usize = 0x90;
const PVAR_SIZE: usize = 0x94;
/// Whether the hologram moby is un-hidden when Ratchet comes near (the game: yes). The port keeps its mode bit set
/// and the engine draws it (crate::vendor_render in rc-engine: its chrome packets and the beam callback 0x2ba9c0) from
/// [`hologram`], since the dynamic-moby path has no metal pass.
const SHOW_HOLOGRAM: bool = false;

/// The hologram as the game draws it: (child moby, scale V+0x90 · class scale · 2.5 applied, shown), with the
/// manipulator phases (0x16139c / 0x1613a0). Shown while the vendor is in state 1 or 2 with V+0x90 > 0.
pub fn hologram(table: &crate::moby_runtime::MobyTable, id: MobyId) -> Option<(MobyId, f32, bool)> {
    let m = table.mobys.get(id)?;
    let v = p::i32(&m.pvars, CHILD);
    let c = (v > 0).then(|| (v - 1) as usize).filter(|&c| c < table.mobys.len())?;
    let s = if m.pvars.len() >= PVAR_SIZE { p::ff(&m.pvars, SCALE) } else { 0.0 };
    Some((c, s, (m.state == 1 || m.state == 2) && s > 0.0))
}

/// The vendor globals (0x1613a4 glow phase, 0x16139c / 0x1613a0 manipulator phases).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Globals {
    pub glow: f32,
    pub spin: [f32; 2],
}

fn child(w: &World, id: MobyId) -> Option<MobyId> {
    let v = p::i32(&w.m(id).pvars, CHILD);
    (v > 0).then(|| (v - 1) as usize).filter(|&c| c < w.table.mobys.len())
}

/// The vendor update (0x2bb128).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < PVAR_SIZE { w.mm(id).pvars.resize(PVAR_SIZE, 0); }
    // The glow.
    let g = &mut w.svc.interact.vendor;
    g.glow += 0.05;
    // 0x4048f5c3 (3.14, not π).
    let lim = f32::from_bits(0x4048_f5c3);
    if g.glow > lim { g.glow = -lim; }
    let k = ((g.glow.sin() * 48.0) as i32 + 96) as u32;
    w.mm(id).glow = k.wrapping_mul(0x0001_0101) | 0x8000_0000;
    let st = w.m(id).state;
    match st {
        0 => {
            w.mm(id).state = 1;
            p::set_i32(&mut w.mm(id).pvars, 0, 0);
            p::set_i32(&mut w.mm(id).pvars, 4, 0);
            let c = w.create_moby(HOLOGRAM);
            p::set_i32(&mut w.mm(id).pvars, CHILD, c.map_or(0, |c| c as i32 + 1));
            if let Some(c) = c {
                let pos = w.m(id).position;
                let m = w.mm(c);
                m.mode |= 2 | if SHOW_HOLOGRAM { 0 } else { 1 };
                m.draw_dist = 64;
                m.position = [pos[0], pos[1], pos[2] + 2.95, pos[3]];
                m.b73 = 32;
                p::set_ff(&mut w.mm(id).pvars, SCALE, 0.0);
                w.svc.unported("vendor: hologram manipulators (AttachManipulator)");
                w.build_matrix(c);
            }
        }
        1 | 2 => {
            let s = p::ff(&w.m(id).pvars, SCALE);
            if st == 1 {
                let s = if s > 0.0 { s - 0.1 } else { s };
                p::set_ff(&mut w.mm(id).pvars, SCALE, s);
                if s <= 0.0 {
                    if let Some(c) = child(w, id) { w.mm(c).mode |= 1; }
                }
            } else if s < 1.0 {
                p::set_ff(&mut w.mm(id).pvars, SCALE, s + 0.1);
            }
            let h = HeroView::of(w.hero);
            let pos = w.m(id).position;
            let (dx, dy) = (pos[0] - h.pos[0], pos[1] - h.pos[1]);
            let xy = (dx * dx + dy * dy).sqrt();
            let dz = (pos[2] - h.pos[2]).abs();
            if w.counter & 7 == 0 {
                if st == 1 && xy <= interact::VENDOR_NEAR.0 && dz <= interact::VENDOR_NEAR.1 {
                    w.mm(id).state = 2;
                    if w.m(id).anim.seq_b != 1 { w.anim_blend(id, 1, 0, 10); }
                    // The game shows the hologram here; its glass / additive draw (with the cone callback 0x2ba9c0)
                    // is not ported and the opaque moby path draws it as a black ball, so the port keeps it hidden.
                    if let Some(c) = child(w, id) {
                        if SHOW_HOLOGRAM { w.mm(c).mode &= !1; } else { w.svc.unported("vendor: hologram draw (kept hidden)"); }
                    }
                } else if st == 2 && (interact::VENDOR_FAR.0 < xy || interact::VENDOR_FAR.1 < dz) {
                    w.mm(id).state = 1;
                    if w.m(id).anim.seq_b != 0 { w.anim_blend(id, 0, 0, 10); }
                    return;
                }
            }
            tail(w, id);
            if st == 2 {
                let settled = w.m(id).anim.seq_a == 1;
                let (show, can_use) = interact::vendor_rule(&h, [pos[0], pos[1], pos[2]], settled);
                let lease = show && w.svc.interact.try_prompt(owner::VENDOR, PROMPT) != 0;
                if w.svc.interact.triangle() && can_use && lease {
                    w.svc.interact.handoffs.push(Handoff::OpenVendor { vendor: Some(id) });
                    // OpenVendorMenu's `SetState(100, 1)` inside the moby loop: Ratchet's update of this tick already
                    // runs state 100 (nothing moves; he is hidden from the next frame on).
                    crate::cinematic::hero_state(w, 100, true);
                    w.mm(id).state = 3;
                }
            }
        }
        3 => {
            p::set_ff(&mut w.mm(id).pvars, SCALE, 0.0);
            if let Some(c) = child(w, id) {
                w.mm(c).scale = 0.0;
                w.build_matrix(c);
            }
        }
        _ => {}
    }
}

/// The common tail of states 1 and 2 (module docs).
fn tail(w: &mut World, id: MobyId) {
    w.svc.unported("vendor: hologram cone draw callback 0x2ba9c0");
    w.svc.unported("vendor: hologram manipulator spin");
    let g = &mut w.svc.interact.vendor;
    g.spin[0] = add_rot(g.spin[0], 0.01);
    g.spin[1] = add_rot(g.spin[1], -0.01);
    let s = p::ff(&w.m(id).pvars, SCALE);
    if let Some(c) = child(w, id) {
        let cs = w.class_scale(HOLOGRAM).to_f32();
        w.mm(c).scale = s * cs * 2.5;
        w.build_matrix(c);
    }
}

/// `VendorExit` 0x2ae660's write to the vendor: state 1.
pub fn on_exit(table: &mut crate::moby_runtime::MobyTable, id: MobyId) {
    if let Some(m) = table.mobys.get_mut(id) { m.state = 1; }
}
