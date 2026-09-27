//! In-engine scenes (game mode 2) in the engine: `rc_game::scene_player` driven once per 60 Hz frame, its
//! actors drawn as extra mobys, its camera, fade, subtitles, HUD hiding and audio requests applied, and the
//! Novalis first-arrival trigger. Spec: docs/plan/cutscenes_transitions.md §3, §4.3–4.4 ("In the port").
//!
//! * **Trigger** (`FixedUpdate`, before the gameplay tick): the classes' `DialogStreamStart(k)` calls of the last
//!   tick (`rc_game::cinematic::EngineRequest::StartScene`, and the talkers' `Handoff::Scene` of
//!   `rc_game::moby_update::interact`). The mission NPC 730/790 (`rc_game::moby_update::classes::mission_npc`)
//!   runs its state 0 in the load pass, so its state-1 branch runs in the first gameplay tick: mission byte ≠ 0xff,
//!   mode 0 and global flag 0x13d397 (`GameState::global.flags[15]`, mirrored into
//!   `Cinematic::arrival_seen`) clear → `DialogStreamStart(5)`, flag := 1; later its mission starts scenes 3 and 4.
//!   The port starts the scene on the frame after the tick that asked for it (the tick itself ran whole), sets the
//!   mode (`MenuMode` → `Mode::Cutscene`). `RC_SCENE` only concerns the arrival scene: `RC_SCENE=0` drops the
//!   arrival request (the classes go on), `RC_SCENE=<k>` plays scene k of the current level after gameplay tick 1
//!   instead of it; every other hand-off (talkers, the mission's scenes 3 / 4) always plays. A scene that cannot be
//!   loaded is reported as ended (skipped) to its talker.
//! * **Movies** (`EngineRequest::StartMovie`, `Handoff::Movie`: `DialogStreamUpdate(n)` → `StartPssMovie`): the
//!   decoder is not chosen (decision U10), so [`play_movie`] is a stub that logs the file and returns at once, as
//!   a skipped movie does (`MovieExitToGameplay` 0x2ad2b8: mode 0, the talker's dialogue refreshed).
//! * **The other requests** of the moby loop: `SetMissionDone` (the level's mission bytes and the saved game), the
//!   ship hidden / shown (`FUN_002a2450` / `0x2a2480`), `UnlockPlanet` and the save (logged, not ported).
//! * **While it runs** the gameplay tick is suspended ([`crate::gameplay::GameTick`] gets a `run_if`): the
//!   hero stays in his spawn idle (the game puts him in state 100, zero velocity) and is hidden with his
//!   items (`FUN_002486c0`: hero, hand, back, Clank mode |= 1), the HUD is hidden (draw mask 0x7f,
//!   [`crate::hud_render::SceneLayer`]).
//! * **Actors**: `CreateMoby(class)` per chunk-0 actor record, drawn through one [`ExtraMobys`] (record +
//!   palette per actor). The streamed sequence goes into the actor's own copy of its class animation at slot
//!   = the class's sequence count (`class+0xc`, then `+0xc++`), re-pointed at every chunk; the pose is
//!   `MobyAnimEval` of (slot, frame f, slot, f + 1, t). Rows = identity (no rotation track; `CreateMoby`
//!   leaves the rotation 0), scale = the class scale, lights = Ratchet's light word and ambient (+0x38 copied
//!   from the hero). A class missing from the level core that is a spaceship class (530..=533) is loaded from
//!   the global `spaceships` file (entry class − 530) like crate::moby_spawn's ship.
//! * **Camera**: [`ActiveScene::camera`] (`SceneCamera`: eye, forward/left/up rows, tan(hfov/2)) replaces the
//!   play camera's transform and projection tangent after `play_camera::apply` (same `RunFixedMainLoop`
//!   slot); when the scene ends `play_camera::apply` restores 0.63 by itself.
//! * **Fade**: a full-screen [`SceneFade`] pass on the main camera after the UI pass and the underwater tint
//!   (`assets/shaders/fade.wgsl`, GS integer blend), alpha `⌊black·128⌋`.
//! * **Subtitles** (`fun_001f4be0`, option 0x15ee40, on unless `RC_SUBTITLES=0`): `FontSetWindow(200, 0x208,
//!   0x28, 0x1d8, 0x100, h − 0x38, 0x12, 7)` measure, y = h − 0x3c (raised when the box would pass h − 0x14),
//!   `DrawUIFrame(y ∓ (height/2 + 5), 0x100 − (width/2 + 10), width/2 + 0x10a, 0x60)`, then the text in
//!   0x80b0b0b0 (regular font), appended to the 2D pass.
//! * **Audio**: the player's requests become `rc_game::audio::scene` commands (speech VAG from
//!   `levels/NN/speech/KK_<lang>.bin`), applied by the same frame's audio frame (`crate::audio_out`'s scene
//!   sound step, which runs after this system while the tick is suspended: EE frame when `world_runs`).
//! * **End** (`FUN_002ac608`): Ratchet `SetState(0, 1)` (queued on the hero-block channel for the next tick); for a
//!   talker's scene Ratchet is put in front of it, facing it (`Interact::scene_end_place`, 0x16cd26), and its
//!   dialogue refreshed (`Interact::scene_ended`).
//! * **Letterbox** (`DrawScreenFade` 0x21b7d8) and the HUD while `0x15f404` (`creature::Globals::cutscene`: the
//!   camera trigger, the gunship, the troopers, the bolt crank) is set in gameplay: black bars top and bottom grow by
//!   one pixel per frame to 24 and shrink the same way after, drawn after the HUD; `HudDraw` draws nothing
//!   meanwhile.
//!
//! Not modelled: the world freeze during the blocking fades (mobys keep their generic advance), particles
//! (they are stepped from the suspended gameplay tick), the FX driver class 1546 (scene 5's ship trail,
//! `0x30c190`: not ported), classes 74/203 mode 0x80.

use crate::fly_cam::FlyCam;
use crate::game_camera::{CameraSource, GameProjection, NTSC_Y_RATIO};
use crate::gameplay::{GameTick, Persistent, Play};
use crate::hud_render::{Hud2d, Prim, SceneLayer};
use crate::input_map::PadFrame;
use crate::moby_attach::AttachedTo;
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::tfrag_render::game_to_bevy;
use anyhow::{anyhow, Context, Result};
use bevy::camera::visibility::VisibilitySystems;
use bevy::core_pipeline::fullscreen_material::{fullscreen_material_system, FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::font::{Font, GlyphTable};
use rc_formats::level::{ClassEntry, TextureEntry};
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass};
use rc_formats::moby_light::{self as light, V4};
use rc_formats::scene::Scene;
use rc_formats::texture::{LevelTexture, TextureSource, TextureTable};
use rc_game::audio::scene::{self as scene_audio, SceneAudioCmd};
use rc_game::hud::text;
use rc_game::menus::mode::Mode;
use rc_game::moby_runtime::MobyId;
use rc_game::pad::PadState;
use rc_game::scene_player::{AudioRequest, Frame, SceneCamera, SceneContext, ScenePlayer, SceneTick, REGION};
use std::sync::Arc;

/// Global flag 0x13d397 = `flags[0x13d397 − 0x13d388]`.
pub const ARRIVAL_FLAG: usize = 0x13d397 - 0x13d388;

/// `RC_SCENE`: None = the game's triggers, Some(None) = never, Some(Some(k)) = force scene k.
fn scene_env() -> Option<Option<usize>> {
    let v = std::env::var("RC_SCENE").ok()?;
    let v = v.trim();
    if v.is_empty() { return None; }
    Some(if v == "0" { None } else { v.parse().ok() })
}

/// The mode-2 black quad.
#[derive(Component, ExtractComponent, Clone, Copy, Default, PartialEq, ShaderType)]
pub struct SceneFade {
    /// GS RGBA bytes (A: 0x80 = 1.0).
    pub rgba: UVec4,
}

impl FullscreenMaterial for SceneFade {
    fn fragment_shader() -> ShaderRef { "shaders/fade.wgsl".into() }

    /// Last: after the UI pass (HUD, subtitles) and the underwater tint (docs: fade quad after `DrawWorld`).
    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system
            .after(bevy::core_pipeline::Core3dSystems::PostProcess)
            .after(bevy::ui_render::ui_pass)
            .after(fullscreen_material_system::<crate::fog_state::UnderwaterTint>)
            .before(bevy::core_pipeline::upscaling::upscaling)
    }
}

/// Marks entities hidden while a scene runs (the hero and his items).
#[derive(Component)]
struct SceneHidden;

struct Actor {
    anim: MobyAnimClass,
    /// Extra sequence slot (class+0xc at spawn).
    slot: u8,
    /// The chunk whose sequence the slot points at.
    chunk: Option<usize>,
    scale: f32,
    base: u32,
    slots: u32,
    entities: Vec<Entity>,
}

/// The running scene, published for the play camera, the HUD and the reports.
#[derive(Resource, Default)]
pub struct ActiveScene {
    /// The scene camera of this frame (None = the play camera).
    pub camera: Option<SceneCamera>,
    /// Black coverage to draw over this frame (0..1).
    pub black: f32,
    pub running: bool,
    /// The last frame's output (reports / tests).
    pub last: Option<SceneTick>,
}

#[derive(Resource)]
struct SceneRuntime {
    mode: Option<Option<usize>>,
    player: Option<ScenePlayer>,
    triggered: bool,
    pad: PadState,
    actors: Vec<Actor>,
    extra: Option<ExtraMobys>,
    palette_len: u32,
    light_word: u32,
    ambient: [u8; 3],
    language: usize,
    subtitles: bool,
    glyphs: Option<[GlyphTable; 3]>,
    frames: u32,
    uploaded: Option<u32>,
    /// Actor records / palette of the last frame (uploaded in PostUpdate).
    records: Vec<u8>,
    palette: Vec<u8>,
    /// Scenes the classes asked for that wait for the running one: (scene, arrival flag, talker).
    pending: std::collections::VecDeque<(usize, bool, Option<MobyId>)>,
    /// The talker whose scene runs (`0x179588`).
    talker: Option<MobyId>,
    /// `DrawScreenFade`'s bar height 0x15f408 (pixels, 0..=24).
    letterbox: i32,
}

pub struct SceneRenderPlugin;

impl Plugin for SceneRenderPlugin {
    fn build(&self, app: &mut App) {
        let mode = scene_env();
        let subtitles = !std::env::var("RC_SUBTITLES").is_ok_and(|v| v.trim() == "0");
        app.add_plugins(FullscreenMaterialPlugin::<SceneFade>::default())
            .init_resource::<ActiveScene>()
            .insert_resource(SceneRuntime {
                mode,
                player: None,
                triggered: false,
                pad: PadState::default(),
                actors: Vec::new(),
                extra: None,
                palette_len: 0,
                light_word: 0,
                ambient: [0x38; 3],
                language: crate::hud_render::language() as usize,
                subtitles,
                glyphs: None,
                frames: 0,
                uploaded: None,
                records: Vec::new(),
                palette: Vec::new(),
                pending: Default::default(),
                talker: None,
                letterbox: 0,
            })
            // The gameplay tick is suspended while a scene runs (mode 2 runs CutsceneModeUpdate instead).
            .configure_sets(FixedUpdate, GameTick.run_if(|a: Res<ActiveScene>| !a.running))
            .add_systems(FixedUpdate, scene_frame.before(GameTick))
            .add_systems(RunFixedMainLoop, apply_camera.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop).after(crate::play_camera::apply))
            .add_systems(Update, subtitle_layer.before(crate::hud_render::HudBuild))
            .add_systems(PostUpdate, (upload, fade_pass))
            .add_systems(PostUpdate, force_hidden.after(moby_render::update_moby_occlusion).before(VisibilitySystems::VisibilityPropagate));
        match mode {
            Some(None) => println!("scene: RC_SCENE=0: the arrival scene is not played (other scenes are)"),
            Some(Some(k)) => println!("scene: RC_SCENE={k}: scene {k} of this level starts after gameplay tick 1"),
            None => {}
        }
    }
}

/// The scene k of the current level, from `level_header.bin` and `scene/KK_ntsc.bin`.
fn load_scene(k: usize) -> Result<Scene> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let header = crate::disc_source::level_file(&root, index, "level_header.bin")?;
    let h = rc_formats::toc::parse_level_header(&header)?;
    let region = crate::disc_source::level_file(&root, index, &format!("scene/{k:02}_{}.bin", REGION.name()))?;
    Ok(Scene::load(&h, &region, k, REGION)?)
}

/// `levels/NN/speech/KK_<lang>.bin` (None when the scene has no speech in that language).
fn load_speech(k: usize, language: usize) -> Option<Arc<[u8]>> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let lang = rc_formats::toc::SCENE_LANGUAGES.get(language)?;
    crate::disc_source::level_file(&root, index, &format!("speech/{k:02}_{lang}.bin")).ok().map(Arc::from)
}

/// Gameplay ticks run since the level load: `0x15f5cc` is 1 after the load (`Game::finish_load`, the load's
/// `0x15f5cc++`) and counts up once per tick.
fn ticks_since_load(counter: u64) -> u64 { counter.saturating_sub(1) }

/// `RC_DEBUG_KILL=t` (dev check, not a game option): before gameplay tick index `t` the three mobys the mission NPC
/// 790 waits for (its links +0x14 / +0x18 / +0x1c) are deleted, to reach its bridge cutaway without a fight.
fn debug_kills(p: &mut Play) {
    static KILL: std::sync::OnceLock<Option<u64>> = std::sync::OnceLock::new();
    let Some(t) = *KILL.get_or_init(|| std::env::var("RC_DEBUG_KILL").ok()?.trim().parse().ok()) else { return };
    if p.game.counter != t + 1 { return; }
    let Some(npc) = p.game.mobys.mobys.iter().position(|m| m.o_class == 790 && !m.is_deleted()) else { return };
    let pv = p.game.mobys.mobys[npc].pvars.clone();
    for o in [0x14usize, 0x18, 0x1c] {
        let Some(id) = pv.get(o..o + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap())).and_then(|v| usize::try_from(v).ok()) else { continue };
        if id >= p.game.mobys.mobys.len() || p.game.mobys.mobys[id].is_deleted() { continue; }
        let c = p.game.counter;
        p.game.mobys.delete(id, c);
        println!("scene: RC_DEBUG_KILL: moby {id} (class {}) deleted before tick {t}", p.game.mobys.mobys[id].o_class);
    }
}

/// `RC_SCENE=<k>`: scene k after gameplay tick 1 (once).
fn forced(rt: &SceneRuntime, play: Option<&Play>) -> Option<usize> {
    if rt.triggered { return None; }
    let play = play?;
    if ticks_since_load(play.game.counter) < 1 { return None; }
    rt.mode.flatten()
}

/// The moby loop's requests of the last tick (module docs): scenes queued in `rt.pending`, the rest applied.
fn take_requests(rt: &mut SceneRuntime, play: &mut Play, state: Option<&mut Persistent>) {
    use rc_game::cinematic::EngineRequest as R;
    use rc_game::moby_update::interact::Handoff;
    let level = crate::level_load::level_index() as usize;
    let mut state = state;
    // Global flag 0x13d397 as the saved game has it (the NPC sets it with its request; the engine only ever raises it).
    if state.as_deref().is_some_and(|s| s.0.global.flags[ARRIVAL_FLAG] != 0) { play.svc.cinematic.arrival_seen = true; }
    let mut scenes: Vec<(usize, bool, Option<MobyId>)> = Vec::new();
    let mut movies: Vec<(i32, Option<MobyId>)> = Vec::new();
    for r in std::mem::take(&mut play.svc.cinematic.requests) {
        match r {
            R::StartScene { scene, arrival } => scenes.push((scene, arrival, None)),
            R::StartMovie { movie } => movies.push((movie, None)),
            R::MissionDone { mission } => {
                if let Some(b) = play.missions.done.get_mut(mission as usize) { *b = 0xff; }
                if let Some(b) = state.as_deref_mut().and_then(|s| s.0.levels.get_mut(level)).and_then(|l| l.missions.get_mut(mission as usize)) { *b = 0xff; }
                println!("scene: SetMissionDone({mission}) on level {level}");
            }
            R::UnlockPlanet { planet } => println!("scene: UnlockPlanet({planet}) + ShowPlanetBanner({planet}) (not ported)"),
            R::Save => println!("scene: memcard_Save (not ported)"),
            R::ShipHidden(h) => {
                if let Some(id) = play.ship_moby() {
                    let m = &mut play.game.mobys.mobys[id];
                    let was = m.mode & 3 == 3;
                    if h { m.mode |= 3 } else { m.mode &= !3 }
                    m.has_collision = !h;
                    if was != h { println!("scene: the ship {}", if h { "hidden" } else { "shown" }); }
                }
            }
        }
    }
    // The talkers' scene / movie hand-offs (the other hand-offs stay for their owner).
    let mut keep = Vec::new();
    for h in std::mem::take(&mut play.svc.interact.handoffs) {
        match h {
            Handoff::Scene { scene, npc } => scenes.push((scene.max(0) as usize, false, npc)),
            Handoff::Movie { movie, npc } => movies.push((movie, npc)),
            other => keep.push(other),
        }
    }
    play.svc.interact.handoffs = keep;
    for (movie, npc) in movies {
        play_movie(movie);
        // MovieExitToGameplay: the talker's dialogue continues.
        if npc.is_some() {
            play.svc.interact.talker = npc;
            play.svc.interact.scene_ended = true;
        }
    }
    for (scene, arrival, npc) in scenes {
        // RC_SCENE only replaces the arrival scene; every other hand-off plays.
        if arrival && rt.mode.is_some() {
            println!("scene: DialogStreamStart({scene}) (the arrival) dropped: RC_SCENE is set");
            continue;
        }
        if arrival {
            if let Some(s) = state.as_deref_mut() { s.0.global.flags[ARRIVAL_FLAG] = 1; }
        }
        rt.pending.push_back((scene, arrival, npc));
    }
}

/// **The movie hook** (`StartPssMovie` 0x2ad0c0 → `MovieModeUpdate` 0x2ad498): in-level movie `n` is `mpegs[2 + n]`
/// (NTSC; PAL `21 + n`), raw PSS in Tier 0 at `global/mpegs/NNN.bin`. No decoder yet (decision U10, see
/// docs/plan/cutscenes.md §5): the stub logs the file and returns at once, which is what the game does when the
/// movie is skipped (Start + L1 L2 R1 R2, or Start alone after the game is beaten / from the replay menu). A
/// decoder plugs in here: play the video full-screen at 30 fps with its SShd ADPCM audio, honour that skip rule,
/// then return. Returns whether a movie played.
pub fn play_movie(n: i32) -> bool {
    let index = 2 + n;
    let path = crate::level_load::extracted_root().join(format!("global/mpegs/{index:03}.bin"));
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    println!("movie: DialogStreamUpdate({n}) → mpegs[{index}] ({}, {size} bytes): no decoder (U10), skipped", path.display());
    false
}

/// Moby part entities that are not hero items (Ratchet's play entities are found by name among them).
type HeroEntities<'w, 's> = Query<'w, 's, (Entity, &'static Name), (With<MeshMaterial3d<MobyMaterial>>, Without<AttachedTo>)>;

#[allow(clippy::too_many_arguments)]
fn scene_frame(
    mut commands: Commands,
    mut rt: ResMut<SceneRuntime>,
    mut active: ResMut<ActiveScene>,
    mut level: ResMut<crate::Level>,
    mut play: Option<ResMut<Play>>,
    mut state: Option<ResMut<Persistent>>,
    mut menu: Option<ResMut<crate::menu_render::MenuMode>>,
    pad: Option<Res<PadFrame>>,
    frame: Res<crate::determinism::FrameNumber>,
    hero: HeroEntities,
    items: Query<Entity, With<AttachedTo>>,
    hidden: Query<Entity, With<SceneHidden>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let rt = &mut *rt;
    // Clean-up on the frame after the last scene frame: gameplay resumes this very frame.
    if rt.player.as_ref().is_some_and(|p| p.done()) {
        rt.player = None;
        for a in rt.actors.drain(..) { for e in a.entities { commands.entity(e).despawn(); } }
        for e in &hidden { commands.entity(e).remove::<SceneHidden>().insert(Visibility::Inherited); }
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Gameplay); }
        *active = ActiveScene::default();
        // FUN_002ac608: SetState(0, 1) (the next tick runs it), the talker's teleport and dialogue refresh.
        if let Some(p) = play.as_mut() {
            let counter = p.game.counter;
            let mut f = rc_game::moby_update::services::HeroFields::of(&p.game.hero);
            if let Some(npc) = rt.talker.take() {
                if let Some((pos, yaw)) = p.svc.interact.scene_end_place.take() {
                    f.clear_motion();
                    f.pose = Some(rc_game::moby_update::services::HeroPose { pos, yaw, target_yaw: yaw });
                }
                p.svc.interact.talker = Some(npc);
                p.svc.interact.scene_ended = true;
            }
            f.call(rc_game::moby_update::services::HeroCall::SetState { id: 0, play: true });
            p.svc.hero_writes = Some((counter, f));
        }
        println!("scene: control returns to gameplay at frame {} (hero SetState(0, 1) on the next tick)", rt.frames);
        return;
    }
    if let Some(p) = play.as_mut() {
        debug_kills(p);
        take_requests(rt, p, state.as_deref_mut());
    }
    if rt.player.is_none() {
        let is_forced = forced(rt, play.as_deref()).is_some();
        let (k, natural, talker) = if let Some(k) = forced(rt, play.as_deref()) {
            (k, false, None)
        } else if let Some(r) = rt.pending.pop_front() {
            r
        } else {
            return;
        };
        rt.triggered = true;
        rt.talker = talker;
        let scene = match load_scene(k) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                warn!("scene: scene {k} not loaded: {e:#}");
                // Reported as ended (a skipped scene) so a talker's dialogue goes on.
                if let (Some(npc), Some(p)) = (rt.talker.take(), play.as_mut()) {
                    p.svc.interact.talker = Some(npc);
                    p.svc.interact.scene_ended = true;
                }
                return;
            }
        };
        let gs = state.as_deref().map(|s| &s.0.global);
        let ctx = SceneContext {
            game_beaten: gs.is_some_and(|g| g.game_beaten != 0),
            completes: gs.map_or(0, |g| g.completes),
            replay: false,
            level: crate::level_load::level_index() as i32,
            language: rt.language,
            subtitles: rt.subtitles,
        };
        println!(
            "scene: app frame {}: DialogStreamStart({k}) after gameplay tick {} ({}): {} ticks, {} chunks, actors {:?}, cuts {:?}; subtitles {} (game option 0x15ee40 = {}), language {}",
            frame.0, play.as_ref().map_or(0, |p| ticks_since_load(p.game.counter)), if natural { "mission NPC 730/790, flag 0x13d397 := 1" } else if is_forced { "RC_SCENE" } else if talker.is_some() { "a talker's hand-off" } else { "a class" },
            scene.end_tick(), scene.chunks.len(), scene.actor_classes(), scene.cut_ticks(), ctx.subtitles, gs.map_or(-1, |g| g.subtitles as i32), ctx.language
        );
        if let Err(e) = spawn_actors(rt, &mut commands, &mut level.0, &scene, &mut meshes, &mut images, &mut materials, &mut buffers) {
            warn!("scene: actors not drawn: {e:#}");
        }
        rt.glyphs = level.0.hud.as_ref().map(|h| h.glyphs);
        let player = ScenePlayer::start(scene, ctx);
        post_audio(rt, &player, &player.start_audio());
        rt.player = Some(player);
        rt.frames = 0;
        if let Some(m) = menu.as_mut() { m.state.set(Mode::Cutscene); }
        // FUN_002486c0: the hero and his items are hidden.
        for (e, name) in &hero {
            if name.as_str().starts_with("Ratchet (play)") { commands.entity(e).insert((SceneHidden, Visibility::Hidden)); }
        }
        for e in &items { commands.entity(e).insert((SceneHidden, Visibility::Hidden)); }
        active.running = true;
    }

    // One frame of the player.
    if let Some(p) = &pad { rt.pad.update(Some(&p.0.bytes()), false); }
    let Some(mut player) = rt.player.take() else { return };
    let out = player.tick(&rt.pad);
    rt.frames += 1;
    post_audio(rt, &player, &out.audio);
    if out.frame == Frame::Playing && (out.scene_tick == 1 || out.camera.is_some_and(|c| c.cut)) {
        println!("scene: app frame {} (scene frame {}) = scene tick {}{}", frame.0, rt.frames, out.scene_tick, if out.scene_tick > 1 { " (camera cut)" } else { "" });
    }
    if let Some(e) = out.end {
        println!("scene: ended after {} frames ({}); tan(hfov/2) {} , music resumes in {} ticks", rt.frames, if e.skipped { "skipped" } else { "played through" }, e.tan_half_fov, e.music_resume_after);
    }
    if !out.actors.is_empty() { pose_actors(rt, &player, &out, &level.0); }
    active.camera = out.camera;
    active.black = out.black;
    active.last = Some(out);
    rt.player = Some(player);
}

fn post_audio(rt: &SceneRuntime, player: &ScenePlayer, reqs: &[AudioRequest]) {
    for r in reqs {
        let cmd = match r {
            AudioRequest::PauseMusic => SceneAudioCmd::PauseMusic,
            AudioRequest::Speech { scene, language, .. } => match load_speech(*scene, *language) {
                Some(vag) => SceneAudioCmd::Speech { vag },
                None => {
                    warn!("scene: no speech for scene {} language {} ({})", player.scene_id(), language, rt.language);
                    continue;
                }
            },
            AudioRequest::StopSpeech => SceneAudioCmd::StopSpeech,
            AudioRequest::ResumeMusic { after } => SceneAudioCmd::ResumeMusic { after: *after },
        };
        scene_audio::post(cmd);
    }
}

/// Registers a spaceship class (530..=533) from the global `spaceships` file (entry class − 530), texture
/// appended to the moby texture table, as crate::moby_spawn does for the loader's ship.
fn load_ship_class(level: &mut crate::level_load::LoadedLevel, o_class: i32) -> Result<(LevelMobyClass, MobyAnimClass)> {
    let root = crate::level_load::extracted_root();
    let file = crate::disc_source::read(&root, &format!("global/spaceships/{:03}.bin", o_class - 530))?;
    let s = rc_formats::moby_spawn::parse_spaceship(&file)?;
    let class = rc_formats::moby::parse_moby_class(s.class).context("parsing the ship class")?;
    let seqs = moby_anim::parse_sequences(s.class, &class).context("parsing the ship sequences")?;
    let tex = level.textures.iter().filter(|t| t.table == TextureTable::Moby).map(|t| t.index + 1).max().unwrap_or(0);
    let slot = u8::try_from(tex).ok().filter(|&t| t != 0xff).context("moby texture table full")?;
    let mut textures = [0xff; 16];
    textures[0] = slot;
    let desc = TextureEntry { data_offset: 0, width: 256, height: 256, ty: 4, palette: 0, mipmap: 0, pad: 0 };
    level.textures.push(LevelTexture { table: TextureTable::Moby, index: tex, source: TextureSource::Entry(desc), texture: s.texture });
    let anim = MobyAnimClass::new(&class, seqs);
    Ok((LevelMobyClass { o_class, entry: ClassEntry { offset_in_asset_wad: 0, o_class, unknown_8: 0, unknown_c: 0, textures }, class }, anim))
}

#[allow(clippy::too_many_arguments)]
fn spawn_actors(
    rt: &mut SceneRuntime,
    commands: &mut Commands,
    level: &mut crate::level_load::LoadedLevel,
    scene: &Scene,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Result<()> {
    // +0x38: Ratchet's light word and ambient.
    if let Some(i) = level.mobys.instances.iter().find(|i| i.o_class == 0) { (rt.light_word, rt.ambient) = (i.light_word(), i.ambient_rgb()); }
    let mut specs = Vec::new();
    let mut palette_len = 0u32;
    for a in &scene.chunks[0].actors {
        let (class, anim) = match level.mobys.classes.iter().position(|c| c.o_class == a.class) {
            Some(ci) => (level.mobys.classes[ci].clone(), level.mobys.anim[ci].clone()),
            None if (530..=533).contains(&a.class) => load_ship_class(level, a.class)?,
            None => return Err(anyhow!("actor class {} is not on this level", a.class)),
        };
        let mut anim = anim;
        let slot = class.class.header.sequence_count;
        if anim.sequences.len() <= slot as usize { anim.sequences.resize(slot as usize + 1, None); }
        let slots = (anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(&class) as u32 + 1).max(1);
        specs.push((class, anim, slot, palette_len, slots));
        palette_len += slots;
    }
    let n = specs.len();
    let mut extra = ExtraMobys::new(level, vec![0; n.max(1) * moby_render::EXTRA_RECORD_SIZE], crate::moby_anim::identity_palette(palette_len), buffers);
    for (k, (class, anim, slot, base, slots)) in specs.into_iter().enumerate() {
        let entities = extra.spawn(commands, level, &class, k as u32, Transform::IDENTITY, &format!("scene actor {k}"), meshes, images, materials);
        for &e in &entities { commands.entity(e).insert(Visibility::Hidden); }
        println!("scene: actor {k}: class {} ({} joints, streamed sequence in slot {slot}), {} entities", class.o_class, anim.joint_count, entities.len());
        rt.actors.push(Actor { scale: class.class.header.scale, anim, slot, chunk: None, base, slots, entities });
    }
    rt.extra = Some(extra);
    rt.palette_len = palette_len;
    Ok(())
}

/// The actors' records and palettes for this frame's poses.
fn pose_actors(rt: &mut SceneRuntime, player: &ScenePlayer, out: &SceneTick, level: &crate::level_load::LoadedLevel) {
    let scene = player.scene();
    let mut palette = crate::moby_anim::identity_palette(rt.palette_len);
    let mut records = Vec::with_capacity(rt.actors.len() * moby_render::EXTRA_RECORD_SIZE);
    let rows: [V4; 3] = [[1f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|r| [r[0].to_bits(), r[1].to_bits(), r[2].to_bits(), 0]);
    let lighting = level.mobys.lighting.as_ref();
    for (a, pose) in rt.actors.iter_mut().zip(&out.actors) {
        if a.chunk != Some(pose.chunk) {
            // FUN_00259288: the slot is re-pointed at this chunk's sequence.
            a.anim.sequences[a.slot as usize] = Some(scene.chunks[pose.chunk].actors[pose.actor].sequence.clone());
            a.chunk = Some(pose.chunk);
        }
        let s = AnimState { seq_a: a.slot, frame_a: pose.frame_a, seq_b: a.slot, frame_b: pose.frame_b, t: pose.t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: true };
        let f = moby_anim::evaluate(&a.anim, &s);
        let at = a.base as usize * 64;
        for (k, b) in f.iter().take(a.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() { palette[at + k] = b; }
        let lights = lighting.map(|l| light::moby_lights(&rows, &l.bank, rt.light_word, rt.ambient, 0x80));
        let model = moby_render::extra_model([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], a.scale, pose.position);
        records.extend_from_slice(&moby_render::extra_record(&model, lights.as_ref(), a.base));
    }
    rt.records = records;
    rt.palette = palette;
}

/// Records, palettes, transforms and visibility of the actors after a frame.
fn upload(mut rt: ResMut<SceneRuntime>, active: Res<ActiveScene>, mut buffers: ResMut<Assets<ShaderBuffer>>, mut q: Query<(&mut Transform, &mut Visibility), Without<Camera3d>>) {
    let rt = &mut *rt;
    if rt.uploaded == Some(rt.frames) || rt.records.is_empty() { return; }
    rt.uploaded = Some(rt.frames);
    let Some(extra) = &rt.extra else { return };
    if let Some(mut b) = buffers.get_mut(&extra.palette) { b.data = Some(rt.palette.clone()); }
    if let Some(mut b) = buffers.get_mut(&extra.instances) { b.data = Some(rt.records.clone()); }
    let shown = active.last.as_ref().is_some_and(|t| !t.actors.is_empty());
    for (k, a) in rt.actors.iter().enumerate() {
        let pos = active.last.as_ref().and_then(|t| t.actors.get(k)).map_or([0.0; 3], |p| p.position);
        let t = Transform::from_matrix(moby_render::extra_model([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], a.scale, pos));
        for &e in &a.entities {
            if let Ok((mut tr, mut vis)) = q.get_mut(e) {
                *tr = t;
                let want = if shown { Visibility::Inherited } else { Visibility::Hidden };
                if *vis != want { *vis = want; }
            }
        }
    }
}

/// The scene camera over the play camera (after `play_camera::apply`, before any reader of the transform).
fn apply_camera(active: Res<ActiveScene>, source: Option<Res<CameraSource>>, mut cams: Query<(&mut Transform, &mut Projection), With<FlyCam>>) {
    let Some(c) = active.camera else { return };
    if source.is_some_and(|s| *s == CameraSource::Fly) { return; }
    let [fwd, _left, up] = c.rows;
    let t = Transform::from_translation(game_to_bevy(c.eye)).looking_to(game_to_bevy(fwd), game_to_bevy(up));
    for (mut tf, mut proj) in &mut cams {
        if *tf != t { *tf = t; }
        if let Projection::Custom(p) = &mut *proj {
            if let Some(g) = p.get_mut::<GameProjection>() {
                if g.tan_x != c.tan_half_fov {
                    g.tan_x = c.tan_half_fov;
                    g.tan_y = c.tan_half_fov * NTSC_Y_RATIO;
                }
            }
        }
    }
}

/// The fade quad component on the main camera.
fn fade_pass(mut commands: Commands, active: Res<ActiveScene>, cams: Query<(Entity, Option<&SceneFade>), With<FlyCam>>) {
    let alpha = (active.black.clamp(0.0, 1.0) * 128.0) as u32;
    let want = (active.running && alpha > 0).then(|| SceneFade { rgba: UVec4::new(0, 0, 0, alpha) });
    for (e, have) in &cams {
        match (want, have) {
            (Some(w), Some(h)) if *h == w => {}
            (Some(w), _) => { commands.entity(e).insert(w); }
            (None, Some(_)) => { commands.entity(e).remove::<SceneFade>(); }
            (None, None) => {}
        }
    }
}

/// HUD hidden and the subtitle box (module docs) into the 2D pass; in gameplay, the letterbox of `0x15f404`.
fn subtitle_layer(mut rt: ResMut<SceneRuntime>, active: Res<ActiveScene>, play: Option<Res<Play>>, mut layer: ResMut<SceneLayer>) {
    let flag = !active.running && play.as_ref().is_some_and(|p| p.svc.creatures.cutscene);
    // DrawScreenFade 0x21b7d8 (part of the HUD layer, so never in mode 2): the bars grow while 0x15f404 is set.
    if !active.running {
        if flag {
            if rt.letterbox < LETTERBOX_MAX { rt.letterbox += 1; }
        } else if rt.letterbox > 0 {
            rt.letterbox -= 1;
        }
    }
    // HudDraw 0x24fb50 draws no slot while 0x15f404 is set.
    layer.hide_hud = active.running || flag;
    layer.prims.clear();
    if !active.running && rt.letterbox > 0 { layer.prims = letterbox_prims(rt.letterbox); }
    let (Some(glyphs), Some(line)) = (rt.glyphs.as_ref(), active.last.as_ref().and_then(|t| t.subtitle.as_ref())) else { return };
    if !active.running { return; }
    layer.prims = subtitle_prims(glyphs, &line.text);
}

/// `DrawScreenFade`'s largest bar height (0x15f408 < 0x18).
pub const LETTERBOX_MAX: i32 = 24;

/// The two opaque black bars of `DrawScreenFade`: `h` pixels at the top and the bottom of the frame (one triangle
/// strip, colour 0x80000000, no blending).
pub fn letterbox_prims(h: i32) -> Vec<Prim> {
    const W: i32 = crate::hud_render::W;
    const H: i32 = crate::hud_render::H;
    let mut out = Hud2d::default();
    out.rect(0, h, 0, W, 0x8000_0000);
    out.rect(H - h, H, 0, W, 0x8000_0000);
    out.prims
}

/// `fun_001f4be0`'s box and text for one line.
pub fn subtitle_prims(glyphs: &[GlyphTable; 3], text_bytes: &[u8]) -> Vec<Prim> {
    use crate::text_render::{draw_ui_frame, font_print_window, TextState};
    const H: i16 = crate::hud_render::H as i16;
    const RGBA: u32 = 0x80b0_b0b0;
    let g = &glyphs[Font::Regular as usize];
    let mut out = Hud2d::default();
    let mut st = TextState::default();
    let mut win = text::Window::new(0xc8, 0x208, 0x28, 0x1d8, 0x100, H - 0x38, 0x12, text::CENTRE_LINES | text::CENTRE_BLOCK | text::MEASURE_ONLY);
    font_print_window(&mut out, &mut st, g, Font::Regular, &mut win, RGBA, text_bytes, -1);
    let hh = (win.height >> 1) as i32 + 5;
    let hw = (win.max_width >> 1) as i32;
    let mut y = H as i32 - 0x3c;
    if (H as i32 - 0x14) < y + hh { y = H as i32 - ((win.height >> 1) as i32 + 0x19); }
    win.y_start = y as i16;
    out.prims.clear();
    draw_ui_frame(&mut out, y - hh, y + hh, 0x100 - (hw + 10), hw + 0x10a, 0x60);
    win.flags &= !text::MEASURE_ONLY;
    font_print_window(&mut out, &mut st, g, Font::Regular, &mut win, RGBA, text_bytes, -1);
    out.prims
}

/// Hidden wins over the occlusion system's visibility writes (as crate::moby_spawn::force_hidden).
fn force_hidden(mut q: Query<&mut Visibility, With<SceneHidden>>) {
    for mut v in &mut q {
        if *v != Visibility::Hidden { *v = Visibility::Hidden; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_formats::font::{Glyph, GLYPHS};

    /// `DrawScreenFade`: two opaque black bars of `h` pixels, top and bottom, full width.
    #[test]
    fn letterbox_bars() {
        let p = letterbox_prims(LETTERBOX_MAX);
        let r: Vec<[i32; 4]> = p.iter().map(|q| q.rect()).collect();
        assert_eq!(r, [[0, 0, crate::hud_render::W, 24], [0, crate::hud_render::H - 24, crate::hud_render::W, 24]]);
        assert!(p.iter().all(|q| q.rgba == 0x8000_0000 && q.tex == crate::hud_render::Tex::None));
    }

    /// `fun_001f4be0`: one 60-pixel line → box of half width 30 + 10 around x = 256, 9 + 5 above and below
    /// y = 416 − 0x3c, text centred on the same line.
    #[test]
    fn subtitle_box_geometry() {
        let mut t = [Glyph::default(); GLYPHS];
        t[b'A' as usize] = Glyph { u: 0, v: 0, y_off: 0, advance: 12 };
        let glyphs = [t; 3];
        let prims = subtitle_prims(&glyphs, b"AAAAA");
        assert!(!prims.is_empty());
        // The first primitives are the DrawUIFrame box; its bounds are the union of their rectangles.
        let frame: Vec<[i32; 4]> = prims.iter().filter(|p| p.tex == crate::hud_render::Tex::None).map(|p| p.rect()).collect();
        assert!(!frame.is_empty());
        let (x0, y0) = (frame.iter().map(|r| r[0]).min().unwrap(), frame.iter().map(|r| r[1]).min().unwrap());
        let (x1, y1) = (frame.iter().map(|r| r[0] + r[2]).max().unwrap(), frame.iter().map(|r| r[1] + r[3]).max().unwrap());
        assert_eq!(((x0 + x1) / 2, (y0 + y1) / 2), (256, 356), "box {x0}..{x1} x {y0}..{y1}");
        assert!(prims.iter().any(|p| p.tex != crate::hud_render::Tex::None), "text drawn");
    }
}
