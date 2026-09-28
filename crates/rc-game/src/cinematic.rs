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
//! | `FadeToBlack(n)` 0x21b438 | [`fade_to_black`] | n blocking black-quad frames over the last image |
//! | `ShowBanner(msg, t)` 0x2789e0 / `ShowPlanetBanner(p)` 0x277c38 | [`show_banner`] / [`show_planet_banner`] | the HUD banner ([`Cinematic::banner`]) |
//! | `UnlockPlanet(p)` 0x2756d0 | [`unlock_planet`] | the saved game's planet bits and map order (+ banner) |
//! | `memcard_Save(0, −1)` | [`save`] | logged: the in-memory game state is the save |
//! | `SetMissionDone(m)` 0x265080 | [`set_mission_done`] | the level's mission byte done (the live bytes and the saved game) |
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
    /// `FadeToBlack(n)` 0x21b438 called from a class in gameplay (the gold bolt's pickup: `ticks(10)`): the game draws
    /// n black quads over the last image, one per vsync, inside the tick (blocking), then the tick goes on. The engine
    /// holds the next `frames` frames (tick suspended, the last view) under the growing coverage.
    FadeToBlack { frames: i32 },
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
    /// The running mode-2 scene as the moby loop reads it (0x16cd10 / 0x16cd14 / 0x16ce58), published by the engine's
    /// scene player before each scene tick's moby loop; None outside scenes.
    pub scene: Option<crate::scene_player::SceneState>,
    /// 0x162070: the point the cutscene FX driver 1546 last read on the arrival ship (its trail's reference).
    pub fx_ship_point: [f32; 4],
    /// The banner buffer's last `ShowBanner(msg, ticks)` of the moby loop (0x179598 / 0x15f640: one banner, the last
    /// call wins), for the HUD; `seq` counts the calls.
    pub banner: BannerCall,
}

/// `ShowBanner(msg, ticks)` 0x2789e0 as the HUD takes it (`HudState::show_banner_msg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BannerCall {
    pub seq: u32,
    /// The level message id (`msg_string`).
    pub msg: i32,
    /// The countdown 0x15f640 (`ticks(180)` for `ShowBanner(msg, −1)`).
    pub ticks: i32,
}

/// `ShowPlanetBanner(p)` 0x277c38: the message of planet `clamp(p, 0, 18)` from the table at level01 0x20a0c0 (the
/// same ids in every overlay), shown `ticks(1180)`: "Infobot for Planet Aridia acquired" (1009), … .
pub const PLANET_BANNERS: [i32; 19] = [0, 6, 1009, 1010, 3014, 6011, 4006, 5012, 7000, 8007, 8008, 10009, 10010, 12005, 13001, 14001, 15009, 15010, 17001];
/// `ShowPlanetBanner`'s countdown (`ticks(0x49c)`).
pub const PLANET_BANNER_TICKS: i32 = 0x49c;

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
    // `EnvNearestSamplePoint(hero)`: the env sample point near the destination (reverb, music track; crate::audio).
    if let Some(s) = w.sound.as_deref_mut() { s.hero_teleported(pos); }
}

/// `DialogStreamStart(k)`.
pub fn start_scene(w: &mut World, scene: usize, arrival: bool) { w.svc.cinematic.requests.push(EngineRequest::StartScene { scene, arrival }); }

/// `DialogStreamUpdate(n)`.
pub fn start_movie(w: &mut World, movie: i32) { w.svc.cinematic.requests.push(EngineRequest::StartMovie { movie }); }

/// `FadeToBlack(n)` from a class (see [`EngineRequest::FadeToBlack`]).
pub fn fade_to_black(w: &mut World, frames: i32) { w.svc.cinematic.requests.push(EngineRequest::FadeToBlack { frames }); }

/// `memcard_Save(0, −1)`: the in-memory game state already holds every write; the engine logs the request (no
/// memory-card writer yet).
pub fn save(w: &mut World) { w.svc.cinematic.requests.push(EngineRequest::Save); }

/// `SetMissionDone(m)` 0x265080: `*(0x14c050 + level·16 + m) = 0xff` unless `m` is 0xff (no other effect). The one
/// writer of the mission bytes for every class (checkpoint 805, talker 774, mission NPC 730 / 790, infobot 750, bolt
/// crank 280; the per-level classes of docs/plan/level_scripting.md): the engine applies it after the tick to the live
/// bytes the classes read ([`crate::moby_update::services::LevelMissions::done`]) and to the saved game, so a class
/// later in the same moby loop still reads the old byte (one tick later than the game's direct store) [L: no reader
/// of the same tick depends on it on the ported levels].
pub fn set_mission_done(w: &mut World, mission: u8) {
    if mission != 0xff { w.svc.cinematic.requests.push(EngineRequest::MissionDone { mission }); }
}

/// `ShowBanner(msg, ticks)` 0x2789e0 (`ticks` already scaled; the caller passes `ticks(180)` for −1).
pub fn show_banner(w: &mut World, msg: i32, ticks: i32) {
    let b = &mut w.svc.cinematic.banner;
    *b = BannerCall { seq: b.seq.wrapping_add(1), msg, ticks };
}

/// `ShowPlanetBanner(p)` 0x277c38.
pub fn show_planet_banner(w: &mut World, planet: i32) {
    let t = w.ticks(PLANET_BANNER_TICKS);
    show_banner(w, PLANET_BANNERS[planet.clamp(0, 18) as usize], t);
}

/// `UnlockPlanet(p)` 0x2756d0: when planet `p` is still locked (0x13dd40[p], the moby loop's mirror of the saved
/// game), it is unlocked and appended to the galaxy-map order 0x13d510 (`GameState::unlock_planet`, through the
/// saved-game write channel), and its banner shown unless `p` is the current level.
pub fn unlock_planet(w: &mut World, planet: i32) {
    let Ok(p) = usize::try_from(planet) else { return };
    if p >= 20 { return; }
    let u = &mut w.svc.interact.game.planet_unlocked;
    if u.len() < 20 { u.resize(20, 0); }
    if u[p] != 0 { return; }
    u[p] = 1;
    w.svc.interact.writes.push(crate::moby_update::interact::GameWrite::UnlockPlanet(p));
    if planet as u32 != w.svc.level { show_planet_banner(w, planet); }
}

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
