//! **Hero polish — the hero's effects on the rest of the game**: the requests the hero code makes of systems it does
//! not own (the camera, the particle system, the sound layer's delayed voices), queued during the hero update in the
//! game's order and handed over by the tick (`crate::tick`) or the particle hook. One mechanism for every hero state:
//! a state asks through the helpers here and never talks to the camera or the particle pool itself.
//!
//! * **Camera shake** ([`HeroFx::shakes`], [`shake`]): the hero code's stores into the camera's shake records
//!   0x167260 / 0x167270 (`crate::follow_camera::Shake`), e.g. the Thruster stomp's landing (0.2 along up for 40
//!   ticks, level01 0x2390a0). The tick hands them to `Camera::request_shake` right after the hero update; the
//!   camera update of the same tick applies them, as in the game.
//! * **Particles** ([`HeroFx::parts`], [`spark`], [`dust`], [`bubbles`], [`sparkle_burst`]): the hero code calls the particle
//!   spawners inside its update, and the spawners' random draws land there. The port makes those draws at the
//!   game's point (so the one `rand` stream stays in step) and queues the spawn with them; the particle hook creates
//!   the queued records, in order, right before `UpdateParts` ([`create_particles`]). Nothing else creates a
//!   particle between the hero update and `UpdateParts` (the hand items' wall / ground sparks are not ported), so the
//!   pool slots and the update order are the game's. One difference: the game makes a spawner's own draws only when
//!   `CreatePart` found a free record; the port always makes them (the pool never fills in play: 2048 records).
//! * **Delayed voices** ([`HeroFx::voices`], `0x236810` / `0x236860`): the 8-entry queue at 0x141528 (`{active,
//!   sound, timer, flags}`); SetState queues the surfacing gasps (7 / 8) there, and right after the transitions
//!   (mode 0) `0x236860` counts every entry's timer down and plays `0x236738(sound, flags)` when it runs out
//!   ([`flush`]). The swim code's own voices (`SwimEvent::Sound` / `Voice`, which it records instead of playing)
//!   go through the same flush, at the same point.

use super::swim::SwimEvent;
use super::{Hero, HeroSounds};
use crate::follow_camera::{ShakeAxis, ShakeRequest};
use crate::moby_runtime::Moby;
use crate::particles::{type25, type34, type47, type53, Particles};
use crate::rng::Rng;

/// A particle spawn the hero code made this tick, with its spawner's random draws already made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PartSpawn {
    /// `PartType25Spawn(pos, vel, variant)` 0x2825f8 (the grind / cable sparks; vel.w = its gravity); `size` =
    /// the spawner's `randf(5000, 30000)`.
    Spark { pos: [f32; 4], vel: [f32; 4], variant: bool, size: f32 },
    /// `PartType47Spawn(size, pos, vel)` 0x286cb0 (the sinking floor's sand); `alpha` = `randi(8)`, `rot` =
    /// `randi(0x100)`, the spawner's two draws.
    Dust { size: f32, pos: [f32; 4], vel: [f32; 4], alpha: u8, rot: u8 },
    /// `PartType34Spawn(size, level, pos, vel)` 0x2840e0 (bubbles), with the pop level resolved at the call and the
    /// spawner's six draws.
    Bubble { size: f32, level: f32, pos: [f32; 4], vel: [f32; 3], draws: type34::Draws },
    /// `PartType53Spawn(s12, s13, s14, pos, life, rgba, a3, t0, vel)` 0x287328 with `a3` 0 / 1 (no draw): the
    /// sparkle (the cable grab's burst `0x2a7e20`).
    Sparkle { s12: f32, s13: f32, s14: f32, pos: [f32; 4], life: i32, rgba: u32, b8: u8, t0: i8, vel: [f32; 3] },
}

/// One entry of the delayed-voice queue 0x141528 (8 bytes: s16 active, sound, timer, flags).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueuedVoice {
    pub active: bool,
    pub sound: i16,
    pub timer: i16,
    pub flags: i16,
}

/// The hero's queued effects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeroFx {
    /// Camera shake requests of this tick, in order (the last one of an axis wins, as the stores do); drained by the
    /// tick.
    pub shakes: Vec<ShakeRequest>,
    /// Particle spawns of this tick, in the game's order (cleared at the start of the hero update; created by the
    /// particle hook: [`create_particles`]).
    pub parts: Vec<PartSpawn>,
    /// 0x141528: the delayed voices.
    pub voices: [QueuedVoice; 8],
    /// The hand item's class sounds of this tick (`PlayClassSound(index, 0, item)` in its update: the wrench's hit),
    /// played by the tick right after the item's update (`super::gadgets::flush_item_sounds`).
    pub item_sounds: Vec<i32>,
    /// The length of `swim.events` when this tick's hero update started (the swim events after it are this tick's;
    /// the engine drains the list after the tick).
    pub swim_mark: usize,
}

/// A camera shake request (the writer's stores into 0x167260 for [`ShakeAxis::Up`], 0x167270 for
/// [`ShakeAxis::Forward`]): `amp` units for `ticks` ticks.
pub fn shake(h: &mut Hero, axis: ShakeAxis, amp: f32, ticks: i32) { h.fx.shakes.push(ShakeRequest { axis, amp, ticks }); }

/// `PartType25Spawn(pos, vel, variant)` 0x2825f8 from the hero code: the spawner's size draw `randf(5000, 30000)`
/// now, the record at the particle hook.
pub fn spark(h: &mut Hero, rng: &mut Rng, pos: [f32; 4], vel: [f32; 4], variant: bool) {
    let size = rng.randf(5000.0, 30000.0);
    h.fx.parts.push(PartSpawn::Spark { pos, vel, variant, size });
}

/// `PartType47Spawn(size, pos, vel)` 0x286cb0 from the hero code: the spawner's draws `randi(8)` (alpha) and
/// `randi(0x100)` (rotation) now, the record at the particle hook.
pub fn dust(h: &mut Hero, rng: &mut Rng, size: f32, pos: [f32; 4], vel: [f32; 4]) {
    let alpha = rng.randi(8) as u8;
    let rot = rng.randi(0x100) as u8;
    h.fx.parts.push(PartSpawn::Dust { size, pos, vel, alpha, rot });
}

/// `0x22b140(n, 0)`: `n` bubbles around the hero (±0.1, −0.5..0.1) drifting with 0.7 of his displacement (1.15 in
/// 0x34) ± 0.4 u/s and sinking at most 1 u/s before they rise, size `randf(4200, 7350)`, popping at the water level
/// 0x13f640 (the hero's z + 0.4 in the sinking floor 0x31). 6 draws each, then the spawner's 6. Modes 1 / 2 (at
/// Ratchet's joint points 0 / 0xe, 0x17 / 0x16) are not ported: the hero update has no joint lists.
pub fn bubbles(h: &mut Hero, rng: &mut Rng, n: i32) {
    let dt = 1.0 / 60.0;
    let p0 = crate::hero::physics::to_f32x3(h.pos);
    let d = crate::hero::physics::to_f32x3(h.disp);
    let k = if h.state == 0x34 { f32::from_bits(0x3f93_3333) } else { f32::from_bits(0x3f33_3333) };
    let level = if h.state == 0x31 { p0[2] + 0.4 } else { h.water_level.to_f32() };
    for _ in 0..n.max(0) {
        let x = p0[0] + rng.randf(-0.1, 0.1);
        let y = p0[1] + rng.randf(-0.1, 0.1);
        let z = p0[2] + rng.randf(-0.5, 0.1);
        let vx = d[0] * k + rng.randf(dt * -0.4, dt * 0.4);
        let vy = d[1] * k + rng.randf(dt * -0.4, dt * 0.4);
        let vz = d[2] * k + rng.randf(-dt, dt * 0.0);
        let size = rng.randf(4200.0, 7350.0);
        let draws = type34::Draws::draw(rng);
        h.fx.parts.push(PartSpawn::Bubble { size, level, pos: [x, y, z, 0.0], vel: [vx, vy, vz], draws });
    }
}

/// `0x2a7e20(point, n)` (L00; the cable grab): `n` pairs of type-53 sparkles flying off `point` (a random direction
/// 20°..88° up at 0.05..0.1 u/tick, spin ±1), a coloured and a white one per pair. Each pair first jitters `point`
/// itself by `randf_sym(0, 0.15)` per axis (the game writes the jitter into its argument, the hand point 0x13f930,
/// cumulatively). Draws per pair: `randf(0.1, 2)`, `randi(2)`, `rand_angle`, `randf(0.349, 1.536)`,
/// `randf(0.05, 0.1)`, three `randf_sym` (the spawns draw nothing).
pub fn sparkle_burst(h: &mut Hero, rng: &mut Rng, point: &mut [f32; 3], n: i32) {
    for _ in 0..n.max(0) {
        let s = rng.randf(0.1, 2.0);
        let i2 = rng.randi(2);
        let spin = if i2 != 0 { i2 } else { -1 };
        let a = rng.rand_angle();
        let b = rng.randf(f32::from_bits(0x3eb2_b8c2), f32::from_bits(0x3fc4_9809));
        let l = rng.randf(f32::from_bits(0x3d4c_cccd), 0.1);
        // FUN_00262c50(len, a, b): (cos a·l·cos b, sin a·l·cos b, sin b·l).
        let vel = [a.cos() * l * b.cos(), a.sin() * l * b.cos(), b.sin() * l];
        for p in point.iter_mut() { *p += rng.randf_sym(0.0, 0.15); }
        let pos = [point[0], point[1], point[2], 0.0];
        let life = crate::hero::physics::ticks(30);
        let s14 = f32::from_bits(0x3b44_9ba6);
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * 0.1, s13: s, s14, pos, life, rgba: 0x7f20_7f7f, b8: 0, t0: spin as i8, vel });
        h.fx.parts.push(PartSpawn::Sparkle { s12: s * 0.07, s13: s * 0.7, s14, pos, life, rgba: 0x7f7f_7f7f, b8: 0x20, t0: -spin as i8, vel });
    }
}

/// The particle hook's part: create the hero's queued spawns of this tick, in order (before `UpdateParts`).
pub fn create_particles(h: &Hero, sys: &mut Particles) {
    sys.hero = crate::hero::physics::to_f32x3(h.pos);
    for s in &h.fx.parts {
        match *s {
            PartSpawn::Spark { pos, vel, variant, size } => { type25::spawn(sys, pos, vel, variant, size); }
            PartSpawn::Dust { size, pos, vel, alpha, rot } => { type47::spawn(sys, size, pos, vel, alpha, rot); }
            PartSpawn::Bubble { size, level, pos, vel, draws } => { type34::spawn(sys, size, level, pos, vel, &draws); }
            PartSpawn::Sparkle { s12, s13, s14, pos, life, rgba, b8, t0, vel } => {
                type53::spawn(sys, s12.to_bits(), s13.to_bits(), s14.to_bits(), pos.map(f32::to_bits), life, rgba, b8, t0, vel.map(f32::to_bits));
            }
        }
    }
}

/// `0x236810(sound, delay, flags)`: the first free entry of the queue (none free: dropped).
pub fn queue_voice(h: &mut Hero, sound: i16, delay: i16, flags: i16) {
    if let Some(e) = h.fx.voices.iter_mut().find(|e| !e.active) {
        *e = QueuedVoice { active: true, sound, timer: delay, flags };
    }
}

/// Start of the hero update: this tick's particle spawns begin empty; the swim events so far are old ones.
pub(super) fn begin(h: &mut Hero) {
    h.fx.parts.clear();
    h.fx.swim_mark = h.swim.events.len();
}

/// After the transitions (mode 0): this tick's swim voices (`0x236738(id, 0)` played, `0x236810` queued), then
/// `0x236860`: every queue entry's timer counts down (`FastDecTimer__FRs`), and an entry whose timer runs out this
/// tick is freed and its voice played.
pub(super) fn flush(h: &mut Hero, moby: &Moby, sounds: &mut dyn HeroSounds, rng: &mut Rng) {
    let new: Vec<SwimEvent> = h.swim.events.get(h.fx.swim_mark..).map(|e| e.to_vec()).unwrap_or_default();
    h.fx.swim_mark = h.swim.events.len();
    for e in new {
        match e {
            SwimEvent::Sound(id) => { sounds.voice(moby, id, 0, rng); }
            SwimEvent::Voice(id, delay) => queue_voice(h, id as i16, delay as i16, 0),
            _ => {}
        }
    }
    if h.mode != 0 { return; }
    for k in 0..h.fx.voices.len() {
        let e = &mut h.fx.voices[k];
        // FastDecTimer: 1 at 0 (no change), else t = max(t, 1) − 1, 2 when it reaches 0.
        if e.timer == 0 { continue; }
        e.timer = e.timer.max(1) - 1;
        if e.timer > 0 { continue; }
        e.active = false;
        let (sound, flags) = (e.sound, e.flags);
        sounds.voice(moby, sound as i32, flags as u16 as u32, rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Log(Vec<(i32, u32)>);
    impl HeroSounds for Log {
        fn anim_advanced(&mut self, _: &Moby, _: &super::super::AnimView, _: &super::super::AnimView, _: &mut Rng) {}
        fn voice(&mut self, _: &Moby, index: i32, flags: u32, _: &mut Rng) -> i32 {
            self.0.push((index, flags));
            0
        }
    }

    /// The surfacing gasps: queued twice (27 and 40 ticks), played when each timer runs out; the splash voice at
    /// once.
    #[test]
    fn delayed_voices_play_when_their_timers_run_out() {
        let mut h = Hero::new();
        let moby = Moby::zeroed();
        let mut rng = Rng::new();
        let mut log = Log(Vec::new());
        begin(&mut h);
        h.swim.events.push(SwimEvent::Sound(3));
        h.swim.events.push(SwimEvent::Voice(7, 27));
        h.swim.events.push(SwimEvent::Voice(7, 40));
        flush(&mut h, &moby, &mut log, &mut rng);
        assert_eq!(log.0, vec![(3, 0)]);
        let mut at = Vec::new();
        for t in 1..60 {
            begin(&mut h);
            let n = log.0.len();
            flush(&mut h, &moby, &mut log, &mut rng);
            if log.0.len() > n { at.push((t, log.0[n])); }
        }
        // Counted down once on the queueing tick too: 26 / 39 ticks later.
        assert_eq!(at, vec![(26, (7, 0)), (39, (7, 0))]);
        assert!(h.fx.voices.iter().all(|e| !e.active));
    }

    /// The ledge climb's voice: 0x1c with the Thruster-Pack as the back item (0x1404f8 = 3) and Clank shown, on the
    /// tick the anim passes frame 10 (`0x236738(4, 0)`), queued for the physics' flush; none with the Heli-Pack.
    #[test]
    fn ledge_climb_voice_with_the_thruster() {
        use super::super::packs::SoundCmd;
        let coll = super::super::testkit::floor(100.0, 100, 106, 100, 106);
        let pad = crate::pad::PadState::default();
        let (cam_rows, cam_yaw) = super::super::testkit::cam_x();
        let env = super::super::Env { coll: &coll, pad: &pad, cam_yaw, cam_rows, mirror: false, death_z: crate::ps2v::Pf::ZERO, mobys: None, hero_moby: None, water: None, world: None };
        for (back, frame, want) in [(3, 10.5, true), (3, 11.5, false), (2, 10.5, false)] {
            let mut h = Hero::spawn([410.0, 410.0, 102.0], 0.0);
            h.state = 0x1c;
            h.back_slot.slot.id = back;
            let mut anim = super::super::RecordingAnim::default();
            anim.v.frame = frame;
            anim.v.frame_step = 1.0;
            super::super::ledge::physics(&mut h, &env, &mut anim, &mut Rng::new());
            assert_eq!(h.packs.sounds.contains(&SoundCmd::Voice { index: 4, flags: 0 }), want, "back {back} frame {frame}");
        }
    }

    #[test]
    fn full_queue_drops() {
        let mut h = Hero::new();
        for k in 0..9 { queue_voice(&mut h, k, 5, 0); }
        assert_eq!(h.fx.voices.iter().map(|e| e.sound).collect::<Vec<_>>(), (0..8).collect::<Vec<_>>());
    }
}
