//! The in-level cinematic layer: the calls the moby classes make into the camera, the hero and the scene / movie
//! players while they stage a cutaway (docs/plan/cutscenes.md §3–§5). The game has **no script system** for these:
//! each class calls the same few engine functions directly from its update:
//!
//! | game function | here | effect |
//! |---|---|---|
//! | `CameraScript(pos, euler, mode, ticks, collide)` 0x316ef8 | [`camera_script`] | the script camera (type 5) switched in ([`crate::follow_camera::script`]) |
//! | `0x316dd0` / `0x316e28` | [`camera_targets`] | its target position / Euler |
//! | `0x316e88(a, b)` | [`camera_curve`] | mode 3's distance curve |
//! | `CameraScript2(kind)` 0x317070 | [`camera_script2`] | back to the follow camera (cut or blend) |
//! | `0x15f404 = 1 / 0` | [`letterbox`] | the letterbox bars (`DrawScreenFade` 0x21b7d8) and the HUD hidden (`HudDraw` 0x24fb50); the creature layer's `Globals::cutscene` *is* this global |
//! | `SetState(s, play)` 0x23cf98 | [`hero_state`] | Ratchet's state (0x72: held, no control) |
//! | `HeroTeleport(pos, euler, state, reset_cam)` 0x2368e0 | [`hero_teleport`] | Ratchet placed, motion cleared, state set |
//! | `DialogStreamStart(k)` 0x2ac330 | [`start_scene`] | a mode-2 scene (the engine's scene player) |
//! | `DialogStreamUpdate(n)` 0x2acf50 | [`start_movie`] | an in-level PSS movie (`StartPssMovie` 0x2ad0c0, mode 1) |
//!
//! The camera calls are queued here and applied by the tick right after the moby loop ([`Cinematic::calls`],
//! `crate::tick`); the hero calls go through the hero-block channel ([`crate::moby_update::services::HeroFields`]);
//! the scene / movie / game-state requests are for the engine ([`Cinematic::requests`]). The creature layer's
//! `ScriptRequest`s (gunship 688) are turned into the same calls ([`from_creature`]).

use crate::moby_runtime::MobyId;
use crate::moby_update::creature::ScriptRequest;
use crate::moby_update::services::{HeroCall, HeroPose, World};

/// A camera call of this tick's moby loop, applied to the camera by the tick in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CinematicCall {
    /// `CameraScript(pos, euler, mode, ticks, collide)`.
    CameraScript { pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool },
    /// `0x316dd0` (position) / `0x316e28` (Euler).
    CameraTargets { pos: Option<[f32; 3]>, euler: Option<[f32; 3]> },
    /// `0x316e88(a, b)`: mode 3's distance curve (D+0x124 / D+0x128).
    CameraCurve { a: f32, b: f32 },
    /// `CameraScript2(kind)` on level `level`.
    CameraRelease { kind: u8, level: u32 },
    /// `HeroTeleport(…, reset_cam = 1)`'s `CameraResetBehindHero` 0x20ee80 (after the hero calls of the tick).
    CameraResetBehindHero,
    /// A creature script's `SetState` (the gunship queues it with its camera call): run with the hero calls.
    HeroState { state: i32, play: bool },
}

/// What the engine has to do for the moby loop (outside the gameplay tick).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineRequest {
    /// `DialogStreamStart(k)`: scene k of this level (mode 2). `arrival`: the first-arrival trigger set global flag
    /// 0x13d397 with it.
    StartScene { scene: usize, arrival: bool },
    /// `DialogStreamUpdate(n)` → `StartPssMovie(mpegs[2 + n] NTSC / [21 + n] PAL)`.
    StartMovie { movie: i32 },
    /// `SetMissionDone(m)` (`0x14c050 + level·16 + m = 0xff`).
    MissionDone { mission: u8 },
    /// `UnlockPlanet(p)` + `ShowPlanetBanner(p)`.
    UnlockPlanet { planet: i32 },
    /// `FUN_002a2450` / `FUN_002a2480`: the ship moby 0x13e030 hidden (mode |= 3, no collision) / shown.
    ShipHidden(bool),
    /// `memcard_Save(0, −1)`.
    Save,
}

/// The cinematic layer's state in the moby services.
#[derive(Clone, Debug, Default)]
pub struct Cinematic {
    /// The camera calls of this tick's moby loop.
    pub calls: Vec<CinematicCall>,
    /// The engine requests, drained by the engine after the tick.
    pub requests: Vec<EngineRequest>,
    /// Global flag 0x13d397 (the Novalis first-arrival scene was started), mirrored from the saved game by the
    /// engine before each tick and set by the mission NPC.
    pub arrival_seen: bool,
    /// Every creature-layer request the tick has taken ([`take_calls`]), oldest first, the last [`LOG_LEN`] kept:
    /// the record of the hand-off for reports and tests.
    pub creature_log: std::collections::VecDeque<ScriptRequest>,
}

/// Entries kept in [`Cinematic::creature_log`].
pub const LOG_LEN: usize = 64;

/// `CameraScript(pos, euler, mode, ticks, collide)`.
pub fn camera_script(w: &mut World, pos: [f32; 3], euler: [f32; 3], mode: u8, ticks: i32, collide: bool) {
    w.svc.cinematic.calls.push(CinematicCall::CameraScript { pos, euler, mode, ticks, collide });
}

/// `0x316dd0` / `0x316e28`.
pub fn camera_targets(w: &mut World, pos: Option<[f32; 3]>, euler: Option<[f32; 3]>) {
    w.svc.cinematic.calls.push(CinematicCall::CameraTargets { pos, euler });
}

/// `0x316e88(a, b)`.
pub fn camera_curve(w: &mut World, a: f32, b: f32) { w.svc.cinematic.calls.push(CinematicCall::CameraCurve { a, b }); }

/// `CameraScript2(kind)`.
pub fn camera_script2(w: &mut World, kind: u8) {
    let level = w.svc.level;
    w.svc.cinematic.calls.push(CinematicCall::CameraRelease { kind, level });
}

/// `0x15f404 = on` (the letterbox and HUD flag; the same global the creature layer's `cutscene` field holds).
pub fn letterbox(w: &mut World, on: bool) { w.svc.creatures.cutscene = on; }

/// `SetState(state, play)` on Ratchet.
pub fn hero_state(w: &mut World, state: i32, play: bool) { w.hero_fields_mut().call(HeroCall::SetState { id: state, play }); }

/// `HeroTeleport(pos, euler, state, reset_cam)` 0x2368e0: position and Euler stored, the motion block cleared,
/// `SetState(state, 1)` (unless −1), and with `reset_cam` the follow camera reset behind him. Only the yaw of the
/// Euler is kept (the port's hero block has no pitch / roll of his own; every Novalis caller passes 0 for them).
pub fn hero_teleport(w: &mut World, pos: [f32; 3], euler: [f32; 3], state: i32, reset_cam: bool) {
    let f = w.hero_fields_mut();
    f.clear_motion();
    f.pose = Some(HeroPose { pos, yaw: euler[2], target_yaw: euler[2] });
    if state != -1 { f.call(HeroCall::SetState { id: state, play: true }); }
    if reset_cam { w.svc.cinematic.calls.push(CinematicCall::CameraResetBehindHero); }
}

/// `DialogStreamStart(k)`.
pub fn start_scene(w: &mut World, scene: usize, arrival: bool) { w.svc.cinematic.requests.push(EngineRequest::StartScene { scene, arrival }); }

/// `DialogStreamUpdate(n)`.
pub fn start_movie(w: &mut World, movie: i32) { w.svc.cinematic.requests.push(EngineRequest::StartMovie { movie }); }

/// A creature layer script request as the camera / hero calls the game makes for it (gunship 688:
/// `SetState(0x72, 0)` then `CameraScript(centre, euler, 0, 0, 0)`; at the end `CameraScript2(0)` then
/// `SetState(0, 1)`).
pub fn from_creature(r: &ScriptRequest, level: u32) -> [CinematicCall; 2] {
    match *r {
        ScriptRequest::Start { hero_state, centre, euler, .. } => [
            CinematicCall::HeroState { state: hero_state as i32, play: false },
            CinematicCall::CameraScript { pos: centre, euler, mode: 0, ticks: 0, collide: false },
        ],
        ScriptRequest::End { .. } => [CinematicCall::CameraRelease { kind: 0, level }, CinematicCall::HeroState { state: 0, play: true }],
    }
}

/// This tick's camera calls for the tick: the classes' own, then the creature layer's requests
/// ([`from_creature`]); both queues are emptied.
pub fn take_calls(svc: &mut crate::moby_update::Services) -> Vec<CinematicCall> {
    let mut v = std::mem::take(&mut svc.cinematic.calls);
    let level = svc.level;
    for r in std::mem::take(&mut svc.creatures.scripts) {
        v.extend(from_creature(&r, level));
        let log = &mut svc.cinematic.creature_log;
        if log.len() == LOG_LEN { log.pop_front(); }
        log.push_back(r);
    }
    v
}

/// The tick's application of the camera calls (right after the moby loop, before the hero update).
pub fn apply_camera_calls(cam: &mut crate::follow_camera::Camera, calls: &[CinematicCall], inp: &crate::follow_camera::CamInput) {
    for c in calls {
        match *c {
            CinematicCall::CameraScript { pos, euler, mode, ticks, collide } => {
                cam.camera_script(pos, euler, mode, ticks, collide, crate::hero::physics::to_f32x3(inp.hero.pos))
            }
            CinematicCall::CameraCurve { a, b } => cam.camera_script_curve(a, b),
            CinematicCall::CameraTargets { pos, euler } => cam.camera_script_targets(pos, euler),
            CinematicCall::CameraRelease { kind, level } => cam.camera_script2(kind, level),
            // CameraResetBehindHero 0x20ee80 on the follow camera (the hero's pose is this tick's: the class stores
            // were applied just before).
            CinematicCall::CameraResetBehindHero => cam.reset(inp),
            CinematicCall::HeroState { .. } => {}
        }
    }
}

/// The creature scripts' `SetState` calls, with the hero calls of the tick.
pub fn run_hero_calls(h: &mut crate::hero::Hero, calls: &[CinematicCall], c: &mut crate::hero::states::Ctx) {
    for call in calls {
        if let CinematicCall::HeroState { state, play } = *call { h.set_state(c, state, play); }
    }
}

/// The moby that issued a creature request (reports).
pub fn creature_moby(r: &ScriptRequest) -> MobyId {
    match *r {
        ScriptRequest::Start { moby, .. } | ScriptRequest::End { moby } => moby,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gunship_requests_map_to_the_game_calls() {
        let s = ScriptRequest::Start { moby: 3, hero_state: 0x72, cuboid: 23, centre: [1.0, 2.0, 3.0], euler: [0.0, 0.02, 1.39], ticks: 510 };
        let c = from_creature(&s, 1);
        assert_eq!(c[0], CinematicCall::HeroState { state: 0x72, play: false });
        assert_eq!(c[1], CinematicCall::CameraScript { pos: [1.0, 2.0, 3.0], euler: [0.0, 0.02, 1.39], mode: 0, ticks: 0, collide: false });
        let e = from_creature(&ScriptRequest::End { moby: 3 }, 1);
        assert_eq!(e, [CinematicCall::CameraRelease { kind: 0, level: 1 }, CinematicCall::HeroState { state: 0, play: true }]);
        assert_eq!(creature_moby(&s), 3);
    }
}
