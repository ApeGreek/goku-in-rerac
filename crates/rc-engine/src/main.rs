//! Runtime entry point: loads one level's terrain from `extracted/` and shows it with a fly camera.
//!
//! Environment:
//! - `RC_LEVEL`      level index (default 1 = Novalis)
//! - `RC_EXTRACTED`  extraction root (default `<workspace>/extracted`)
//! - `RC_SCREENSHOT` if set, save a PNG of the window there after the scene has rendered, then exit
//! - `RC_SCREENSHOT_DELAY` seconds to wait before that capture (default 3); with `RC_SCREENSHOT_FRAME`
//!   the capture is frame-exact instead (crate::determinism) and this wall-clock one is not scheduled
//! - `RC_CAM`        optional camera override in game coordinates: `eye_x,eye_y,eye_z,target_x,target_y,target_z`
//! - `RC_GS_ALPHA=0` the previous alpha mapping (one blended draw without Z for any As != 0x80), crate::gs_state
//! - `RC_FOG_ZONES=0`, `RC_UNDERWATER=1`, `RC_CAM_PATH` fog zones / underwater / camera walk-through, crate::fog_state
//! - `RC_PLAY=0`, `RC_PLAY_SCRIPT`, `RC_PLAY_TRACE=1` the game tick / scripted pad / per-tick trace, crate::gameplay
//! - `RC_PLAY_FLY=1` start on the fly camera with the game ticking; `RC_DEBUG_HIT=moby@tick,...` debug hits
//!   (flags 0x10000, damage 1) before those ticks' moby loop, crate::gameplay
//! - `RC_SCENE=0` / `RC_SCENE=<k>`, `RC_SUBTITLES=0` in-engine scenes (Novalis arrival = scene 5), crate::scene_render
//! - `RC_HUD=0`, `RC_HUD_DEMO=1`, `RC_HUD_TEXT`, `RC_HUD_HELP=<id>`, `RC_LANG` the HUD and its demos, crate::hud_render
//! - `RC_MSAA=0|2|4|8` world-camera multisampling at start (default 0 = off, like the GS), crate::render_settings
//! - `RC_SETTINGS_FILE=<path>` the port-settings file (`0` or empty: none), crate::render_settings;
//!   `RC_SETTINGS_PAGE=0` no "Port Options" page in the Options menu, crate::menu_render

mod audio_out;
mod determinism;
mod disc_source;
mod fly_cam;
mod fog_state;
mod game_camera;
mod gameplay;
mod gs_state;
mod hud_render;
mod input_map;
mod level_load;
mod menu_render;
mod moby_anim;
mod moby_attach;
mod moby_light;
mod moby_lod;
mod moby_render;
mod moby_spawn;
mod occlusion;
mod particle_render;
mod play_camera;
mod render_settings;
mod scene_render;
mod shrub_billboard;
mod shrub_light;
mod shrub_render;
mod sky_render;
mod sky_stars;
mod text_render;
mod tfrag_light;
mod tfrag_lod;
mod tfrag_render;
mod tie_light;
mod tie_lod;
mod tie_render;
mod water_render;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::path::PathBuf;
use std::time::Duration;

use fly_cam::{FlyCam, FlyCamPlugin};
use level_load::LoadedLevel;
use tfrag_render::{game_to_bevy, TfragMaterial};

#[derive(Resource)]
struct Level(LoadedLevel);

#[derive(Resource)]
struct ScreenshotRequest {
    path: PathBuf,
    /// Minimum wall time and frame count before capturing, so the shader has compiled and the meshes have been uploaded.
    delay: Duration,
    min_frames: u32,
}

fn main() -> anyhow::Result<()> {
    let root = level_load::extracted_root();
    let index = level_load::level_index();
    let mut level = level_load::load_level(&root, index)?;
    // The loader's ship and the load-time update pass (crate::moby_spawn; RC_SPAWN_RULES=0 skips it).
    let spawn = moby_spawn::apply_load_pass(&root, index, &mut level)?;
    // Fog zones and the underwater test (crate::fog_state).
    let fog_state = fog_state::FogState::new(fog_state::load(&root, index)?, &level.fog);
    let t = &level.timings;
    let bg = level.background;
    println!(
        "level {index:02} from {}: {} tfrags, {} textures; load {:.1} ms (read {:.1}, WAD {:.1}, core {:.1}, tfrags {:.1}, textures {:.1})",
        root.display(), level.tfrags.len(), level.textures.len(), ms(t.total()),
        ms(t.read), ms(t.decompress), ms(t.parse_core), ms(t.parse_tfrags), ms(t.parse_textures)
    );
    println!("background (GS clear colour): {bg:?}");
    println!("{}", fly_cam::CONTROLS);

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                // 2x the NTSC 512x416 GS draw buffer (docs/plan/render_pipeline.md).
                primary_window: Some(Window { title: "randcre".into(), resolution: (1024u32, 832u32).into(), ..default() }),
                ..default()
            })
            .set(AssetPlugin { file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(), ..default() }),
    )
    // Frame-exact ticks / capture and the deterministic phase order, before anything spawns a camera.
    .add_plugins(determinism::DeterminismPlugin)
    // MSAA of the world cameras (default off; `RenderSettings` can change it at run time).
    .add_plugins(render_settings::RenderSettingsPlugin)
    .add_plugins((MaterialPlugin::<TfragMaterial>::default(), FlyCamPlugin, game_camera::GameCameraPlugin { fog: level.fog }))
    .add_plugins(occlusion::OcclusionPlugin)
    .add_plugins(tfrag_lod::TfragLodPlugin)
    .add_plugins(moby_render::MobyRenderPlugin)
    .add_plugins(moby_spawn::MobySpawnPlugin)
    .add_plugins(moby_attach::MobyAttachPlugin)
    .insert_resource(spawn)
    .add_plugins(tie_render::TieRenderPlugin)
    .add_plugins(shrub_render::ShrubRenderPlugin)
    .add_plugins(sky_render::SkyRenderPlugin)
    .add_plugins(particle_render::ParticlePlugin)
    .add_plugins(water_render::WaterPlugin)
    .add_plugins(sky_stars::SkyStarsPlugin)
    .add_plugins(fog_state::FogStatePlugin)
    .add_plugins(hud_render::HudPlugin)
    // The game tick (hero, pad, follow camera) driving Ratchet and the view; RC_PLAY=0 keeps the fly camera only.
    .add_plugins(gameplay::GameplayPlugin)
    .add_plugins(menu_render::MenuPlugin)
    // In-engine scenes (mode 2) and the Novalis arrival (crate::scene_render; RC_SCENE=0 / RC_SCENE=<k>).
    .add_plugins(scene_render::SceneRenderPlugin)
    // Sound slots, 989snd, music and the software SPU2 mixed per game tick (crate::audio_out; RC_AUDIO=0 off).
    .add_plugins(audio_out::AudioOutPlugin::new(level.audio.take()))
    .insert_resource(fog_state)
    // The GS frame clear: level settings +0x00..+0x08 via `set_background_color` (game_camera::level_background).
    // srgb_u8: the bytes are display-encoded like every GS colour; the sRGB view target stores them back exactly.
    .insert_resource(ClearColor(Color::srgb_u8(bg[0], bg[1], bg[2])))
    .insert_resource(Level(level))
    .add_systems(Startup, setup)
    .add_systems(Update, report_fps);

    if let Some(path) = std::env::var_os("RC_SCREENSHOT").filter(|_| determinism::screenshot_frame().is_none()) {
        let secs = std::env::var("RC_SCREENSHOT_DELAY").ok().and_then(|s| s.parse().ok()).unwrap_or(3.0);
        app.insert_resource(ScreenshotRequest { path: path.into(), delay: Duration::from_secs_f32(secs), min_frames: 30 })
            .add_systems(Update, take_screenshot);
    }
    app.run();
    Ok(())
}

fn ms(d: Duration) -> f64 { d.as_secs_f64() * 1e3 }

fn setup(
    mut commands: Commands,
    level: Res<Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TfragMaterial>>,
    mut buffers: ResMut<Assets<bevy::render::storage::ShaderBuffer>>,
) {
    let level = &level.0;
    let s = tfrag_render::spawn_tfrags(&mut commands, level, &mut meshes, &mut images, &mut materials, &mut buffers);
    println!(
        "tfrags: {} tfrags, {} LOD-0 triangles, {} vertices, {} meshes ({} with the GS alpha-test split), {} images; mesh build {:.1} ms, asset upload {:.1} ms",
        s.tfrags, s.triangles, s.vertices, s.meshes, s.blended_meshes, s.images, ms(s.mesh_build), ms(s.upload)
    );

    // Camera framing from the tfrag bounding spheres. They are stored in integer position units
    // (x1024 like origin + local), not world units: on Novalis the raw centroid is ~(166138, 189596, 59301)
    // while the LOD-0 vertices span (22..322, 45..388, 30..112) after /1024.
    let spheres: Vec<[f32; 4]> = level.tfrags.iter().map(|t| t.header.bsphere.map(|v| v / 1024.0)).collect();
    let n = spheres.len().max(1) as f32;
    let c = spheres.iter().fold(Vec3::ZERO, |a, s| a + Vec3::new(s[0], s[1], s[2])) / n;
    let extent = spheres.iter().map(|s| (Vec3::new(s[0], s[1], s[2]) - c).length() + s[3]).fold(0.0f32, f32::max);
    println!(
        "bounds (game units): sphere centroid {:.1?}, extent {:.1}; vertex AABB {:.1?} .. {:.1?}",
        c.to_array(), extent, s.min, s.max
    );
    let (eye, target) = match std::env::var("RC_CAM").ok().and_then(|v| parse_cam(&v)) {
        Some((e, t)) => (game_to_bevy(e), game_to_bevy(t)),
        // Up (game +Z) and back (game -Y) from the centroid, looking at it.
        None => (game_to_bevy((c + Vec3::new(0.0, -0.35 * extent, 0.25 * extent)).to_array()), game_to_bevy(c.to_array())),
    };
    let transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        game_camera::game_projection(),
        // No tonemapping or dithering: the shader already produces the final display value.
        Tonemapping::None,
        DebandDither::Disabled,
        render_settings::WorldCamera,
        FlyCam::from_transform(&transform, (extent / 20.0).clamp(5.0, 200.0)),
        transform,
    ));
}

/// `ex,ey,ez,tx,ty,tz` in game coordinates.
fn parse_cam(s: &str) -> Option<([f32; 3], [f32; 3])> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
    (v.len() == 6).then(|| ([v[0], v[1], v[2]], [v[3], v[4], v[5]]))
}

fn report_fps(time: Res<Time>, mut acc: Local<(f32, u32)>) {
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 5.0 {
        println!("fps: {:.1}", acc.1 as f32 / acc.0);
        *acc = (0.0, 0);
    }
}

fn take_screenshot(
    mut commands: Commands,
    req: Res<ScreenshotRequest>,
    time: Res<Time<Real>>,
    mut frames: Local<u32>,
    mut requested: Local<bool>,
) {
    *frames += 1;
    if *requested || *frames < req.min_frames || time.elapsed() < req.delay { return; }
    *requested = true;
    let path = req.path.clone();
    commands.spawn(Screenshot::primary_window()).observe(move |shot: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
        match shot.image.clone().try_into_dynamic() {
            Ok(img) => match img.to_rgb8().save(&path) {
                Ok(()) => println!("screenshot saved to {}", path.display()),
                Err(e) => eprintln!("cannot save screenshot to {}: {e}", path.display()),
            },
            Err(e) => eprintln!("cannot convert screenshot: {e:?}"),
        }
        exit.write(AppExit::Success);
    });
}
