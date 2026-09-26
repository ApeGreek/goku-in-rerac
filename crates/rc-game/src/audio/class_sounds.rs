//! The audio layer on the game's tick (docs/plan/audio.md §3.1, §3.2): the class sounds the gameplay code plays
//! (`PlayClassSound` level01 0x2a1618 / 0x2a16c0) and the per-tick sound step, all on the game's one `rand`
//! stream ([`crate::tick::Game::rng`]) at the game's points:
//!
//! * **Moby class sounds** (crates, bolts, platforms, …): the moby loop calls `PlayClassSound` through
//!   [`crate::moby_update::services::SoundSink`]; [`ClassSoundSink`] allocates the slot at once
//!   ([`AudioSystem::play_class_sound`], `SoundSlotAlloc` 0x2a13a0), so its pitch-bend `randi` lands in the moby
//!   order. The listener is the camera 0x167240 as the previous tick's camera update left it.
//! * **Ratchet's animation triggers**: `RatchetAnimAdvance` 0x247d48 plays the class sound of the first trigger
//!   whose time its key time passed this tick ([`ratchet_trigger`]). The hero code is ported without the
//!   sound layer, so [`TriggerAnim`] wraps its [`AnimCtl`] and records the fired sounds; [`sound_step`] plays
//!   them before `sound_update`. Their pitch-bend draws therefore come after the particles' draws of the tick
//!   instead of inside the hero update (same count per tick, inferred order inside the tick).
//! * **The sound step** [`sound_step`]: after the camera, before the counter increment (level01 `FUN_002aba68`,
//!   docs/plan/trace_results_novalis.md "Second savestate"): the recorded trigger sounds, then the EE frame
//!   (`sound_update` 0x2a0638: occlusion origin = 3 draws every frame, the 6-origin batch per new occluded
//!   sound, the sound instances' plays) and its 800 samples.
//!
//! Clank (601) and the back packs (607–609) have no class sound defs on any level (class header +0x0d = 0), so
//! the triggers of their `MobyAnimAdvance` never play or draw; the hand items' triggers (wrench 71, …) are not
//! routed (the hero's item code advances them).

use super::voices::{Listener, Owner};
use super::{AudioSystem, FrameInput};
use crate::follow_camera::CameraView;
use crate::hero::{AnimCtl, AnimView, Hero};
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::services::{SoundEvent, SoundSink};
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::moby_anim::{MobyAnimClass, Rows};
use rc_formats::sound_bank::SoundOwner;
use std::cell::RefCell;

/// Class 0x472 (1138): its sounds may use slots 26..29 like the hero's.
pub const PRIVILEGED_CLASS: i16 = 0x472;

/// The listener of the sound code: the camera 0x167240 with its rows 0x167450.. (forward, left, up).
pub fn listener_of(cam: &CameraView) -> Listener {
    Listener { pos: cam.pos_f32(), rows: cam.rows_f32(), underwater: false, water_height: 0.0 }
}

/// A slot owner's position (moby +0x10) from the moby table; None when the moby is deleted (state 0xfd /
/// 0xfe) or gone.
pub fn owner_position(table: &MobyTable, id: u32) -> Option<[f32; 3]> {
    let m = table.mobys.get(id as usize)?;
    if m.state >= crate::moby_runtime::state::DELETED_STATIC { return None; }
    Some([m.position[0], m.position[1], m.position[2]])
}

impl AudioSystem {
    /// `PlayClassSound(index, flags, moby)` (0x2a1618; 0x2a16c0 with another class's table): class sound
    /// `index` of `ev.sound_class` (−1 when the class has no such def), `SoundSlotAlloc` with the moby as the
    /// owner (privileged: the hero `hero` or class 0x472), volume 0x400; the slot remembers the class-sound
    /// index (+0xe). Draws the def's pitch bend from `rng` when it gets a slot.
    pub fn play_class_sound(&mut self, ev: &SoundEvent, hero: Option<MobyId>, listener: &Listener, rng: &mut Rng) -> i32 {
        self.stats.class_sounds += 1;
        let Ok(local) = usize::try_from(ev.index) else { return -1 };
        let Some(def) = self.data.sounds.def(SoundOwner::Class(ev.sound_class as i32), local).copied() else { return -1 };
        let owner = Owner { id: ev.moby as u32, privileged: Some(ev.moby) == hero || ev.o_class == PRIVILEGED_CLASS };
        let k = self.slots.play(&def, ev.flags as u8, Some(owner), Some(ev.pos), None, 0x400, listener, rng);
        if k >= 0 {
            self.slots.slots[k as usize].class_index = ev.index as u16;
            self.stats.class_slots += 1;
        }
        k
    }
}

/// The moby loop's [`SoundSink`]: class sounds into `audio` with the listener of the tick (the previous tick's
/// camera).
pub struct ClassSoundSink<'a> {
    pub audio: &'a mut AudioSystem,
    pub listener: Listener,
    /// Ratchet's moby (privileged owner).
    pub hero: Option<MobyId>,
}

impl SoundSink for ClassSoundSink<'_> {
    fn play_class_sound(&mut self, ev: &SoundEvent, rng: &mut Rng) -> i32 { self.audio.play_class_sound(ev, self.hero, &self.listener, rng) }
}

/// The trigger check at the end of `RatchetAnimAdvance` (0x247d48): when key A and key B were the same
/// sequence before the advance and it has triggers (`+0x7e`), the first trigger word (lo16 class sound, hi16
/// time in 1/16 frames, signed) with `16·frame_a₀ + trunc(16·t₀) < time ≤ 16·frame_a₁ + trunc(16·t₁)` fires
/// (`before` / `after` = the views around the advance). Returns its class sound.
pub fn ratchet_trigger(class: &MobyAnimClass, before: &AnimView, after: &AnimView) -> Option<u16> {
    if before.seq_a != before.seq_b { return None; }
    let seq = class.sequence(after.seq_a)?;
    if seq.header.trigger_count == 0 { return None; }
    let lo = before.frame_a as i32 * 16 + (before.t * 16.0) as i32;
    let hi = after.frame_a as i32 * 16 + (after.t * 16.0) as i32;
    seq.triggers.iter().take(seq.header.trigger_count as usize).find(|&&w| {
        let time = (w as i32) >> 16;
        lo < time && time <= hi
    }).map(|&w| (w & 0xffff) as u16)
}

/// Ratchet's [`AnimCtl`] with the trigger check of his advance: every class sound [`ratchet_trigger`] fires is
/// appended to `fired` (drained by [`sound_step`]).
pub struct TriggerAnim<'a> {
    pub inner: &'a mut dyn AnimCtl,
    pub class: &'a MobyAnimClass,
    pub fired: &'a RefCell<Vec<u16>>,
}

impl AnimCtl for TriggerAnim<'_> {
    fn set_anim(&mut self, blend: Pf, seq: u8, frame: i32) { self.inner.set_anim(blend, seq, frame) }
    fn advance(&mut self, speed: Pf) {
        let before = self.inner.view();
        self.inner.advance(speed);
        if let Some(s) = ratchet_trigger(self.class, &before, &self.inner.view()) { self.fired.borrow_mut().push(s); }
    }
    fn view(&self) -> AnimView { self.inner.view() }
    fn frame_count(&self, seq: u8) -> u8 { self.inner.frame_count(seq) }
    fn set_loop(&mut self, start: i32, end: i32) { self.inner.set_loop(start, end) }
    fn clear_loop(&mut self) { self.inner.clear_loop() }
    fn eval_chains(&self, chains: &[&[u8]]) -> Vec<Rows> { self.inner.eval_chains(chains) }
    fn pose_frame(&self) -> Option<rc_formats::moby_anim::MobyFrame> { self.inner.pose_frame() }
}

/// The tick's sound step (module docs): Ratchet's triggered class sounds (`fired`, drained; owner = his moby
/// `hero_moby`), then [`AudioSystem::tick_with`] with the listener = `cam` (this tick's camera), the hero
/// position for the music boxes, the counter 0x15f5cc and the moby table resolving the slot owners. The 800
/// samples are appended to `out`.
#[allow(clippy::too_many_arguments)]
pub fn sound_step(
    audio: &mut AudioSystem,
    table: &MobyTable,
    hero: &Hero,
    hero_moby: MobyId,
    cam: &CameraView,
    rng: &mut Rng,
    counter: u64,
    fired: &mut Vec<u16>,
    out: &mut Vec<[i16; 2]>,
) {
    let listener = listener_of(cam);
    let o_class = table.mobys.get(hero_moby).map_or(0, |m| m.o_class);
    let pos = owner_position(table, hero_moby as u32).unwrap_or(listener.pos);
    for idx in fired.drain(..) {
        let ev = SoundEvent { index: idx as i32, flags: 0, moby: hero_moby, o_class, sound_class: o_class, pos, tick: counter };
        audio.play_class_sound(&ev, Some(hero_moby), &listener, rng);
    }
    let input = FrameInput { listener, hero_pos: [hero.pos[0].to_f32(), hero.pos[1].to_f32(), hero.pos[2].to_f32()] };
    audio.tick_with(&input, counter as u32, rng, &|id| owner_position(table, id), out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::moby_anim::{MobySequence, MobySequenceHeader};

    fn class_with(triggers: Vec<u32>) -> MobyAnimClass {
        let header = MobySequenceHeader { frame_count: 20, loop_sound: 0xff, trigger_count: triggers.len() as u8, ..Default::default() };
        let sequences = vec![Some(MobySequence { header, frames: vec![], triggers })];
        MobyAnimClass { joint_count: 0, skeleton: vec![], rest: vec![], parent_word: vec![], sequences }
    }

    fn view(seq_a: u8, frame_a: u8, t: f32) -> AnimView { AnimView { seq_a, seq_b: 0, frame_a, t, ..Default::default() } }

    #[test]
    fn trigger_fires_once_in_its_window() {
        // Class sound 26 at time 8 (frame 0.5), 30 at 160 (frame 10).
        let c = class_with(vec![8 << 16 | 26, 160 << 16 | 30]);
        assert_eq!(ratchet_trigger(&c, &view(0, 0, 0.0), &view(0, 0, 0.5)), Some(26));
        // Exclusive below, inclusive above.
        assert_eq!(ratchet_trigger(&c, &view(0, 0, 0.5), &view(0, 0, 0.9)), None);
        assert_eq!(ratchet_trigger(&c, &view(0, 9, 0.5), &view(0, 10, 0.0)), Some(30));
        // A blend (key A ≠ key B before the advance) never fires; a wrap back to frame 0 neither.
        assert_eq!(ratchet_trigger(&c, &view(0xff, 0, 0.0), &view(0, 0, 0.5)), None);
        assert_eq!(ratchet_trigger(&c, &view(0, 19, 0.5), &view(0, 0, 0.6)), None);
    }
}
