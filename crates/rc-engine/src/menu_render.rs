//! The in-level menus in the engine: the mode system (`rc_game::menus::mode`), the quick-select ring (HUD
//! slot 3) and the mode-3 page menus (pause, Options, map, planet select), drawn through the HUD's 2D pass.
//! Spec: docs/plan/menus.md; the state machines are `rc_game::menus`.
//!
//! **Frame** (`FixedUpdate`, after `gameplay::tick`; one main-loop frame of the game, `entry` 0x259c40):
//! * mode 0: the gameplay tick has run (it is skipped in every mode whose update does not tick,
//!   [`Mode::advances_tick`]); then the ring's hero part (open / double tap) and its HUD update, the pad lock
//!   it requests (PAD+0x1cc = 2 for the next `ProcessPadInput`), and the pause triggers of 0x2aba68 (Start
//!   or pad lost → kind 0, Select|R3 → kind 10, after ≥ 8 frames in mode 0). Draws: the ring.
//! * mode 3: `UpdatePad` on the game's pad (the main loop runs it in every mode), the menu frame
//!   (`SceneController`), its draws (`PageMenuDraw`). The close's post-action returns to mode 0 (ship
//!   travel, movies, scenes and the slideshow are not ported: logged, then resumed).
//! * [`MenuMode::loop_frame`] counts main-loop frames: the scripted pad (`RC_PLAY_SCRIPT`) is indexed by it,
//!   so a script keeps its timeline across a pause (the gameplay tick counter stops in mode 3).
//!
//! **Drawing.** The frame's [`MenuDraw`] list becomes HUD primitives ([`Hud2dHook`]): panels are drawn in
//! place (translated, scissored to the panel, cleared to navy first) instead of into a render target copied
//! 1:1, which is the same image for the 1:1 blit every ported widget returns. `fun_00200e08` rectangles and
//! `fun_00200c80` lines use the corner offset of those packets (pixel x − 1). In mode 3 the HUD's own calls
//! are not drawn and it does not tick (the HUD pass belongs to the mode-0 render).
//!
//! **Snapshot** (`DownloadFrameBuffer` at the menu's first tick, a synchronous VRAM download at 0x2b4c88; every
//! later menu frame re-uploads it): the frame on which the menu is entered is still rendered as a gameplay
//! frame (as on the PS2, where the mode switch takes effect next frame), and at the end of that frame's
//! render graph (`RenderGraphSystems::Finish`, after every camera, before present) the GPU copies the main
//! target's finished output attachment (world + HUD, as the game's frame-buffer copy) into the snapshot
//! texture (`assets/shaders/menu_snapshot.wgsl`, `darken`), with the menu's black at alpha 0x30 applied on the
//! way with the GS formula on display bytes (`Cd + ((−Cd·0x30) >> 7)`, i.e. ×0.625), written through a UNORM
//! view so the stored bytes are exact. No CPU read-back: the snapshot is on the GPU before the first menu
//! frame renders, so every menu frame shows it (a function of the frame number, not of the wall clock).
//! The source is whatever the target's `ViewTargetAttachments` entry is at that point: the offscreen capture
//! image in frame-exact runs (`determinism.rs`), a Bevy `Screenshot`'s offscreen texture when one is taken of
//! the same target on that frame (it then blits it to the target after the graph), or, for a window target,
//! a texture the window's cameras render into on that frame only (a swap-chain texture cannot be sampled),
//! which is then copied back to the swap chain (`copy`). The world keeps rendering underneath (hidden).
//!
//! **Menu layer** (`PageMenuDraw` steps 1–3: snapshot, black, then the 14 class-0x472 frame mobys with
//! `DrawMobyList(m, 1)` under TEST_1 0x5360b over a Z buffer the frame's clear packet set to 0): an
//! offscreen image of the main target's size, composited by a UI node on the main camera between the world
//! and the HUD composite (whose 2D pass then draws the navy panel rects and the widgets, steps 4–6). Two
//! cameras render it: a `Camera2d` (order −3) that clears it to black at alpha 0x30/0x80 and draws the
//! darkened snapshot (a UI node), then, only in mode 3, a `Camera3d` (order −2,
//! [`MENU_3D_LAYER`], no colour clear, depth cleared to 0 = far) that draws the frame mobys. The frame mobys
//! (`rc_game::menus::pause::frame`: spawn, animation, corners, rects) are `moby_render::ExtraMobys` instances
//! lit with the menu's light set 14 (`FUN_0028c128` writes it from 0x160280 / 0x160290; light word 0x0e0e,
//! ambient 0x202020 from `SpawnHandGadgetMoby`), palettes from `MobyAnimEval` of each slot's state.
//! **Menu camera.** The game draws them from the menu camera `fun_00218d10` sets: position (256, 256, 64),
//! rows identity (looking along +x, up +z), FOV 0.63 (the default projection). Every system that picks "the"
//! `Camera3d` would also see this one, so it carries the main camera's transform, and the menu view is
//! folded into the model matrices instead: `model' = C_main · C_menu⁻¹ · model`, which puts each moby in the
//! main camera's view space exactly where the menu camera sees it (same view-space depth, so fog and the
//! projection are unchanged).
//!
//! **Not ported**: the sounds (listed in the log), streamed pictures / maps, the globe, the ring's draw
//! position between HUD slots 2 and 4 (it is drawn after the whole HUD); the frame mobys are fogged with
//! the level's fog (`fog_state` pushes it into every `MobyMaterial`), not the menu's view-context fog
//! (0..524288, F 255..0): at the frames' depth (≈ 5 units) the two differ by at most one step of F.
//!
//! **Port Options** (port-only, `rc_game::menus::pause::port`): an extra "Port Options" entry in the Options
//! list opens a page built from the game's machinery; its anti-aliasing row is synced with
//! [`RenderSettings::msaa`] around every menu tick (the resource's value is shown; a ✕ writes it back, applied
//! by `render_settings::apply` and saved to the port settings file). The row offers only the sample counts the
//! GPU supports ([`SupportedMsaa`]; ✕ skips the rest, e.g. 8x on Apple M-series).
//!
//! Environment: `RC_MENU_TRACE=1` prints menu events (open, transitions, sounds, equip requests, close);
//! `RC_SETTINGS_PAGE=0` leaves the Options page as on the disc (no Port Options entry).

use crate::gameplay::{Persistent, Play, Session};
use crate::hud_render::{Hud2d, Hud2dHook, HudBuild, Prim, Tex, H, W};
use crate::input_map::{PadFrame, Script};
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::render_settings::{self, RenderSettings, SupportedMsaa};
use crate::text_render::{self, TextState};
use anyhow::{anyhow, Context};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::camera::NormalizedRenderTarget;
use bevy::core_pipeline::FullscreenShader;
use bevy::ecs::entity::ContainsEntity;
use bevy::platform::collections::HashMap;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    CommandEncoder, CommandEncoderDescriptor, Extent3d, FragmentState, LoadOp, Operations, PipelineCache, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, VertexState,
};
use bevy::render::renderer::{RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue};
use bevy::render::storage::ShaderBuffer;
use bevy::render::texture::{GpuImage, OutputColorAttachment};
use bevy::render::view::{prepare_view_attachments, prepare_view_targets, ExtractedWindows, Msaa, ViewTargetAttachments};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::ShaderDefVal;
use bevy::transform::TransformSystems;
use bevy::ui::UiTargetCamera;
use bevy::window::PrimaryWindow;
use rc_formats::moby_light::{self, MobyLights};
use rc_formats::save_game::ItemTables;
use rc_formats::tfrag_light::{ps2, DirLightSet};
use rc_game::hud::{Draw, HudAssets};
use rc_game::menus::mode::{Mode, ModeState};
use rc_game::menus::pause::frame::{self, FrameClass, FrameMobys};
use rc_game::menus::pause::port::Setting;
use rc_game::menus::pause::{MenuEnv, PageMenu, PostAction, DARKEN};
use rc_game::menus::quick_select::{HeroGate, QuickSelect};
use rc_game::menus::{MenuAssets, MenuDraw, MenuInput, Overlay};
use rc_game::pad::button;

/// The mode globals and the main-loop frame counter (read by `gameplay::tick`).
#[derive(Resource, Default, Debug)]
pub struct MenuMode {
    pub state: ModeState,
    /// Main-loop frames so far (the index of the frame being run while `gameplay::tick` runs).
    pub loop_frame: u64,
}

#[derive(Resource)]
struct MenuRt {
    assets: MenuAssets,
    qs: Option<QuickSelect>,
    menu: Option<PageMenu>,
    draws: Vec<MenuDraw>,
    /// The mode the frame's draws belong to (the mode at the frame's start).
    render_mode: Mode,
    script: Option<Script>,
    last_counter: Option<u64>,
    /// The menu was entered this frame: the snapshot is taken of this frame's render.
    snapshot_request: bool,
    /// The snapshot texture holds the entering frame (with the menu's black 0x30 applied to its bytes).
    snapshot_ready: bool,
    /// The snapshot texture (written on the GPU, module docs).
    snapshot: Handle<Image>,
    trace: bool,
    /// The menu layer (module docs).
    layer: MenuLayer,
}

#[derive(Component)]
struct SnapshotNode;

/// Render layers of the menu layer's cameras (the HUD uses 29, the sky 1).
pub const MENU_3D_LAYER: usize = 27;
const MENU_2D_LAYER: usize = 28;

/// The UI node that composites the menu layer onto the main camera.
#[derive(Component)]
struct MenuLayerNode;

/// The mode-3 `Camera3d` of the menu layer.
#[derive(Component)]
struct MenuLayerCam;

struct MenuLayer {
    image: Handle<Image>,
    cam2d: Entity,
    node: Entity,
    cam3d: Option<Entity>,
    frame: Option<FrameRender>,
}

/// The frame mobys' render side: 14 extra instances of class 0x472 (slot i = record / palette block i).
struct FrameRender {
    extra: ExtraMobys,
    entities: Vec<Vec<Entity>>,
    /// Palette slots per moby.
    slots: u32,
    /// Light block for rotation (0, 0, 0) with light set 14 = the menu light; None with `RC_NO_LIGHT`.
    lights: Option<MobyLights>,
    /// `game_to_bevy · [s/1024 · R | p]` of a frame moby (rows from rotation 0, class scale, (256, 256, 64)).
    model: Mat4,
    /// The menu camera's Bevy transform (position (256, 256, 64), rows identity).
    camera: Mat4,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuMode>();
        if !crate::gameplay::enabled() { return; }
        app.init_resource::<SnapshotRequest>()
            .add_systems(First, |mut r: ResMut<SnapshotRequest>| r.0 = None)
            .add_systems(PreUpdate, setup)
            .add_systems(FixedUpdate, menu_frame.after(crate::gameplay::GameTick))
            .add_systems(Update, (target_main_camera, build_prims).chain().before(HudBuild))
            .add_systems(PostUpdate, menu_layer.before(TransformSystems::Propagate));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<SnapshotJob>()
            .init_resource::<SnapshotOffscreen>()
            .add_systems(RenderStartup, init_snapshot_pipelines)
            .add_systems(ExtractSchedule, extract_snapshot)
            .add_systems(
                Render,
                prepare_snapshot.in_set(RenderSystems::PrepareViews).after(prepare_view_attachments).before(prepare_view_targets),
            )
            .add_systems(RenderGraph, snapshot_copy.in_set(RenderGraphSystems::Finish));
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    mut commands: Commands,
    level: Res<crate::Level>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if *done { return; }
    *done = true;
    let Some(lh) = level.0.hud.as_ref() else {
        eprintln!("menus: no HUD data for this level: no menus");
        return;
    };
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let overlay = match crate::disc_source::level_file(&root, index, "overlay.bin").and_then(|b| Ok(Overlay::parse(&b)?)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("menus: overlay not read ({e:#}): no menus");
            return;
        }
    };
    let items = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99"))
        .and_then(|elf| Ok(ItemTables::load(&elf, &crate::disc_source::level_file(&root, index, "overlay.bin")?)?));
    let qs = match &items {
        Ok(t) => QuickSelect::load(&overlay, t),
        Err(e) => {
            eprintln!("menus: item tables not read ({e:#}): no quick select");
            None
        }
    };
    let mut menu = PageMenu::load(&overlay);
    // The frame mobys: class 0x472 from the level core.
    let frame_class = match load_frame_class(&level.0, &overlay) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("menus: no frame mobys ({e:#}): no panels are placed");
            None
        }
    };
    if let (Some(m), Some((fc, _))) = (menu.as_mut(), &frame_class) { m.frames = Some(FrameMobys::new(fc.clone(), false)); }
    // Port-only: the "Port Options" page and its entry in the Options list (RC_SETTINGS_PAGE=0: none).
    if let Some(m) = menu.as_mut().filter(|_| std::env::var("RC_SETTINGS_PAGE").map_or(true, |v| v.trim() != "0")) {
        if !m.install_port_page(&overlay) { eprintln!("menus: Port Options page not installed (Options page records missing)"); }
    }
    println!(
        "menus: quick select {} (neighbour table {:#x}), page menu {} ({} pages, {} widgets); RC_MENU_TRACE=1 logs events",
        if qs.is_some() { "ready" } else { "missing" },
        qs.as_ref().map_or(0, |q| q.tables.table_addr),
        if menu.is_some() { "ready" } else { "missing" },
        menu.as_ref().map_or(0, |m| m.pages.len()),
        menu.as_ref().map_or(0, |m| m.widgets.len()),
    );
    let script = std::env::var("RC_PLAY_SCRIPT").ok().filter(|s| !s.trim().is_empty()).and_then(|s| Script::parse(&s).ok());
    let size = window.map_or(UVec2::new(1024, 832), |w| w.physical_size()).max(UVec2::ONE);
    let mut layer_image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    layer_image.sampler = bevy::image::ImageSampler::nearest();
    let image = images.add(layer_image);
    // The snapshot: sampled through its sRGB view by the UI, written through a UNORM view (exact bytes).
    let mut snap_image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    snap_image.texture_descriptor.view_formats = &[TextureFormat::Rgba8Unorm];
    snap_image.sampler = bevy::image::ImageSampler::nearest();
    let snapshot = images.add(snap_image);
    let cam2d = commands
        .spawn((
            Camera2d,
            Camera {
                order: -3,
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, DARKEN as f32 / 128.0)),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Msaa::Off,
            Tonemapping::None,
            DebandDither::Disabled,
            RenderLayers::layer(MENU_2D_LAYER),
            Name::new("menu layer 2D camera (snapshot + black)"),
        ))
        .id();
    let node = commands
        .spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            ImageNode::new(image.clone()),
            GlobalZIndex(i32::MAX - 1),
            Visibility::Hidden,
            MenuLayerNode,
            Name::new("menu layer (snapshot, black, frame mobys)"),
        ))
        .id();
    let frame = frame_class.map(|(fc, ci)| frame_render(&mut commands, &level.0, &overlay, &fc, ci, &mut meshes, &mut images, &mut materials, &mut buffers));
    let layer = MenuLayer { image, cam2d, node, cam3d: None, frame };
    let assets = MenuAssets::new(HudAssets::new(&lh.hud, lh.glyphs, lh.messages.clone()), overlay);
    commands.insert_resource(MenuRt {
        assets,
        qs,
        menu,
        draws: Vec::new(),
        render_mode: Mode::Gameplay,
        script,
        last_counter: None,
        snapshot_request: false,
        snapshot_ready: false,
        snapshot: snapshot.clone(),
        trace: std::env::var("RC_MENU_TRACE").is_ok_and(|v| v.trim() == "1"),
        layer,
    });
    // The snapshot is the menu layer's background: a UI node of its 2D camera.
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        ImageNode::new(snapshot),
        Visibility::Hidden,
        SnapshotNode,
        UiTargetCamera(cam2d),
        Name::new("menu snapshot (frame buffer copy)"),
    ));
}

/// Class 0x472 (index into the level's moby classes) and its corner chains: the joint lists named at
/// 0x161fe0, read from the class blob in the level core.
fn load_frame_class(level: &crate::level_load::LoadedLevel, ov: &Overlay) -> anyhow::Result<(FrameClass, usize)> {
    let m = &level.mobys;
    let ci = m.classes.iter().position(|c| c.o_class == frame::O_CLASS).ok_or_else(|| anyhow!("class {:#x} is not on this level", frame::O_CLASS))?;
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let read = |name: &str| crate::disc_source::level_file(&root, index, name);
    let data = rc_formats::wad::decompress(&read("core_data.bin")?).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&read("core_index.bin")?, data.len()).context("parsing core index")?;
    let name = format!("moby_class/{:04}", frame::O_CLASS);
    let blk = core.blocks.iter().find(|b| b.name == name).ok_or_else(|| anyhow!("no {name} block"))?;
    let blob = data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow!("{name} out of range"))?;
    let header = &m.classes[ci].class.header;
    let mut chains: [Vec<u8>; 4] = Default::default();
    for (k, c) in chains.iter_mut().enumerate() {
        let id = ov.i32(frame::CORNER_LISTS_ADDR + 4 * k as u32).ok_or_else(|| anyhow!("0x161fe0 not in the overlay"))?;
        *c = rc_formats::gadget::joint_list(blob, header, usize::try_from(id)?).with_context(|| format!("joint list {id}"))?.0;
    }
    Ok((FrameClass { anim: m.anim[ci].clone(), chains, scale: header.scale }, ci))
}

/// The 14 frame-moby instances (hidden, [`MENU_3D_LAYER`]) and their light block.
#[allow(clippy::too_many_arguments)]
fn frame_render(
    commands: &mut Commands,
    level: &crate::level_load::LoadedLevel,
    ov: &Overlay,
    fc: &FrameClass,
    ci: usize,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> FrameRender {
    let class = &level.mobys.classes[ci];
    let slots = (fc.anim.joint_count as u32).max(ExtraMobys::max_skinned_joint(class) as u32 + 1).max(1);
    let n = frame::SLOTS as u32;
    let records = vec![0u8; frame::SLOTS * moby_render::EXTRA_RECORD_SIZE];
    let mut extra = ExtraMobys::new(level, records, crate::moby_anim::identity_palette(n * slots), buffers);
    let entities: Vec<Vec<Entity>> = (0..n)
        .map(|slot| {
            let es = extra.spawn(commands, level, class, slot, Transform::IDENTITY, "menu frame", meshes, images, materials);
            for &e in &es { commands.entity(e).insert((RenderLayers::layer(MENU_3D_LAYER), Visibility::Hidden)); }
            es
        })
        .collect();
    let rows = moby_light::rotation_rows([0.0; 3]);
    // FUN_0028c128: light set 14 (0x1806c0) = { colour A = *0x160290, direction A = VecScale(1.0, *0x160280), 0, 0 }.
    let q = |a: u32| -> [f32; 4] { std::array::from_fn(|k| f32::from_bits(ov.u32(a + 4 * k as u32).unwrap_or(0))) };
    let dir = q(frame::LIGHT_DIR_ADDR);
    let set = DirLightSet {
        color_a: q(frame::LIGHT_COLOR_ADDR),
        dir_a: [0, 1, 2, 3].map(|k| if k < 3 { f32::from_bits(ps2::mul(dir[k].to_bits(), ps2::ONE)) } else { dir[3] }),
        color_b: [0.0; 4],
        dir_b: [0.0; 4],
    };
    let lights = level.mobys.lighting.as_ref().map(|l| {
        let mut bank = l.bank.clone();
        bank.sets[frame::LIGHT_SET] = set;
        moby_light::moby_lights(&rows, &bank, frame::LIGHT_WORD, frame::AMBIENT, 0x80)
    });
    let rows_f32 = rows.map(|r| [0, 1, 2].map(|k| f32::from_bits(r[k])));
    let model = moby_render::extra_model(rows_f32, fc.scale, frame::CAMERA_POS);
    let camera = Transform::from_translation(crate::tfrag_render::game_to_bevy(frame::CAMERA_POS)).looking_to(Vec3::X, Vec3::Y).to_matrix();
    println!(
        "menus: frame mobys: class {:#x} ({} joints, {} sequences, corner joints {:?}), {} palette slots each; menu light {:?} / {:?}",
        frame::O_CLASS, fc.anim.joint_count, fc.anim.sequences.len(), fc.chains.iter().map(|c| c.last().copied()).collect::<Vec<_>>(), slots, set.color_a, set.dir_a
    );
    FrameRender { extra, entities, slots, lights, model, camera }
}

fn target_main_camera(mut commands: Commands, nodes: Query<Entity, (With<MenuLayerNode>, Without<UiTargetCamera>)>, cams: Query<Entity, With<crate::fly_cam::FlyCam>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

/// The anti-aliasing row's values (Off, 2x, 4x, 8x) as sample counts.
const AA_SAMPLES: [u32; 4] = [1, 2, 4, 8];

fn aa_index(m: Msaa) -> u8 { AA_SAMPLES.iter().position(|&n| n == m.samples()).unwrap_or(0) as u8 }

/// The anti-aliasing row's selectable values (bit k = `AA_SAMPLES[k]`) on this device.
fn aa_choices(s: &SupportedMsaa) -> u32 {
    AA_SAMPLES.iter().enumerate().filter(|(_, n)| s.0.contains(n)).fold(0, |m, (k, _)| m | 1 << k)
}

/// The hand item shown now (0x140408): the wrench while `0x15ed90` (wrench held) is set, else the
/// equipped hand item [M: the hand-swap state machine is not ported].
fn held_item(g: &rc_game::game_state::Global) -> i32 { if g.wrench_held != 0 { 8 } else { g.equipped[0] } }

#[allow(clippy::too_many_arguments)]
fn menu_frame(
    rt: Option<ResMut<MenuRt>>,
    mut mm: ResMut<MenuMode>,
    play: Option<ResMut<Play>>,
    gs: Option<ResMut<Persistent>>,
    sess: Option<ResMut<Session>>,
    pad: Res<PadFrame>,
    source: Res<crate::game_camera::CameraSource>,
    mut render: Option<ResMut<RenderSettings>>,
    supported: Option<Res<SupportedMsaa>>,
) {
    let (Some(mut rt), Some(mut play), Some(mut gs), Some(mut sess)) = (rt, play, gs, sess) else { return };
    let rt = &mut *rt;
    let (gs, sess) = (&mut gs.0, &mut sess.0);
    let mode = mm.state.mode;
    let counter = play.game.counter;
    if mode == Mode::Gameplay && rt.last_counter == Some(counter) { return; }
    rt.last_counter = Some(counter);
    mm.state.begin_frame();
    let frame = mm.loop_frame;
    let vsync = frame as u32;
    rt.render_mode = mode;
    rt.draws.clear();
    let trace = rt.trace;
    match mode {
        Mode::Gameplay => {
            let inp = MenuInput::from_pad(&play.game.pad, true);
            let gate = HeroGate { early_exit: sess.hp < 1, held_item: held_item(&gs.global), ..Default::default() };
            if let Some(qs) = rt.qs.as_mut() {
                let h = qs.hero(&inp, &gate, &mut gs.global, sess);
                let u = qs.update(&inp, &gate, &mut gs.global, sess, vsync);
                if u.lock_pad { play.game.pad.lock = 2; }
                if trace || h.request.is_some() || u.closed || h.opened {
                    if h.opened { println!("menus: frame {frame}: quick select opened (slots {:?})", gs.global.quick_select); }
                    if let Some(r) = h.request { println!("menus: frame {frame}: double tap: hand request item {r} (0x141408)"); }
                    if u.closed {
                        println!("menus: frame {frame}: quick select closed, selection {} → hand request {:?} (0x141408 = {}; the hand swap FUN_002307e0 is not ported)", qs.sel, u.request, sess.temp_hand);
                    }
                }
                qs.draw(&rt.assets, &gs.global, play.game.counter, &mut rt.draws);
            }
            // 0x2aba68: Start / pad lost → pause menu, Select|R3 → map (≥ 8 frames in mode 0, hero state and HP).
            let hs = play.game.hero.state;
            let allowed = mm.state.frames_in_mode >= 8 && ![0x72, 0x32, 0x1d].contains(&hs) && sess.hp != 0;
            if let (true, Some(menu)) = (allowed, rt.menu.as_mut()) {
                let kind = if inp.pressed & button::START != 0 || !inp.connected {
                    Some(0)
                } else if inp.pressed & (button::SELECT | button::R3) != 0 {
                    Some(10)
                } else {
                    None
                };
                if let Some(k) = kind {
                    menu.enter(k, gs);
                    mm.state.set(Mode::Menu);
                    rt.snapshot_request = true;
                    rt.snapshot_ready = false;
                    println!("menus: frame {frame}: enter mode 3, kind {k} (game tick frozen at {counter})");
                }
            }
        }
        Mode::Menu => {
            let input = match &rt.script {
                Some(s) => s.at(frame),
                None if *source == crate::game_camera::CameraSource::Play => pad.0,
                None => rc_game::pad::PadInput::neutral(),
            };
            let mirror = gs.options().mirror;
            play.game.pad.update(Some(&input.bytes()), mirror);
            let inp = MenuInput::from_pad(&play.game.pad, true);
            let env = MenuEnv { vsync, b13f4: 0, pal: false };
            if let Some(menu) = rt.menu.as_mut() {
                menu.set_port_choices(Setting::Msaa, supported.as_deref().map_or_else(|| aa_choices(&SupportedMsaa::default()), aa_choices));
                if let Some(r) = render.as_deref() { menu.set_port_value(Setting::Msaa, aa_index(r.msaa)); }
                let out = menu.tick(&inp, gs, &env);
                if let (Some(v), Some(r)) = (menu.port_value(Setting::Msaa), render.as_mut()) {
                    let msaa = render_settings::msaa_from_samples(AA_SAMPLES[v as usize % AA_SAMPLES.len()]);
                    if r.msaa != msaa {
                        r.msaa = msaa;
                        println!("menus: frame {frame}: Port Options: anti-aliasing {} samples", msaa.samples());
                        render_settings::save(r);
                    }
                }
                if trace && !out.sounds.is_empty() { println!("menus: frame {frame}: sounds {:?}", out.sounds); }
                if let Some((a, b)) = out.transition { println!("menus: frame {frame}: transition {a:#x} → {b:#x} (kind 1, 12 ticks)"); }
                if let Some(p) = out.entered { println!("menus: frame {frame}: page {p:#x} entered (kind {:#x})", menu.kind); }
                if out.quit { println!("menus: frame {frame}: Quit Game ○ (0x15f570 = 1: leaving the level is not ported)"); }
                if let Some(p) = out.freeze { println!("menus: frame {frame}: mode_freezeInit(3, {p:#x}) (save / load dialog not ported)"); }
                if let Some(x) = out.exit {
                    if x != PostAction::Resume { println!("menus: frame {frame}: post-action {x:?} not ported; resuming"); }
                    mm.state.set(Mode::Gameplay);
                    println!("menus: frame {frame}: menu closed, mode 0 (stub calls {:?})", menu.stub_calls);
                } else {
                    menu.draw(&rt.assets, gs, &env, &mut rt.draws);
                }
            }
        }
        _ => {}
    }
    mm.state.end_frame();
    mm.loop_frame += 1;
}

/// The frame's draws → HUD primitives; the snapshot node and its capture.
fn build_prims(
    rt: Option<ResMut<MenuRt>>,
    level: Res<crate::Level>,
    mut hook: ResMut<Hud2dHook>,
    mut snap: Query<&mut Visibility, With<SnapshotNode>>,
    cams: Query<&RenderTarget, With<crate::fly_cam::FlyCam>>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut request: ResMut<SnapshotRequest>,
) {
    let Some(mut rt) = rt else { return };
    let Some(lh) = level.0.hud.as_ref() else { return };
    let mut h = Hud2d::default();
    h.frame_sizes = rt.assets.hud.frame_sizes.clone();
    let mut st = TextState::default();
    let snapshot = convert(&rt.draws, &mut h, &mut st, &lh.glyphs);
    hook.prims = h.prims;
    hook.replace_hud = rt.render_mode == Mode::Menu;
    hook.freeze = rt.render_mode != Mode::Gameplay;
    let show = snapshot && rt.snapshot_ready;
    for mut v in &mut snap { *v = if show { Visibility::Visible } else { Visibility::Hidden }; }
    if std::mem::take(&mut rt.snapshot_request) {
        // This frame's render (still a gameplay frame) is copied on the GPU at the end of its render graph,
        // before the first menu frame renders: the snapshot is ready from that frame on.
        let target = cams.iter().next().cloned().unwrap_or_default().normalize(primary.iter().next());
        match target {
            Some(t) => {
                request.0 = Some((t, rt.snapshot.id()));
                rt.snapshot_ready = true;
            }
            None => eprintln!("menus: the main camera has no render target: no snapshot"),
        }
    }
}

/// The menu layer each frame: its cameras and node follow the mode; the frame mobys' visibility, palettes
/// and records follow `PageMenu::frames` (after this frame's menu tick).
#[allow(clippy::too_many_arguments)]
fn menu_layer(
    mut commands: Commands,
    rt: Option<ResMut<MenuRt>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    main: Query<&Transform, With<crate::fly_cam::FlyCam>>,
    mut transforms: Query<&mut Transform, Without<crate::fly_cam::FlyCam>>,
    mut cams: Query<&mut Camera>,
    mut vis: Query<&mut Visibility>,
    spawn_hidden: Query<(), With<crate::moby_spawn::SpawnHidden>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(mut rt) = rt else { return };
    let rt = &mut *rt;
    let active = rt.render_mode == Mode::Menu;
    let Some(main_t) = main.iter().next().copied() else { return };
    let layer = &mut rt.layer;
    // The layer image and the snapshot follow the main target (the window, or the capture image kept at its size).
    if let Some(size) = window.map(|w| w.physical_size()).filter(|s| s.x > 0 && s.y > 0) {
        for h in [&layer.image, &rt.snapshot] {
            let same = images.get(h).is_some_and(|i| i.texture_descriptor.size.width == size.x && i.texture_descriptor.size.height == size.y);
            if !same {
                if let Some(mut img) = images.get_mut(h) { img.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }); }
            }
        }
    }
    if let Ok(mut c) = cams.get_mut(layer.cam2d) {
        if c.is_active != active { c.is_active = active; }
    }
    if let Ok(mut v) = vis.get_mut(layer.node) {
        let want = if active { Visibility::Visible } else { Visibility::Hidden };
        if *v != want { *v = want; }
    }
    // The 3D camera exists only in mode 3 (other systems take "the" Camera3d; it carries the main transform).
    match (active && layer.frame.is_some(), layer.cam3d) {
        (true, None) => {
            layer.cam3d = Some(
                commands
                    .spawn((
                        Camera3d::default(),
                        Camera { order: -2, clear_color: ClearColorConfig::None, ..default() },
                        RenderTarget::Image(layer.image.clone().into()),
                        crate::game_camera::game_projection(),
                        Msaa::Off,
                        Tonemapping::None,
                        DebandDither::Disabled,
                        RenderLayers::layer(MENU_3D_LAYER),
                        main_t,
                        MenuLayerCam,
                        Name::new("menu layer 3D camera (frame mobys)"),
                    ))
                    .id(),
            );
        }
        (true, Some(e)) => {
            if let Ok(mut t) = transforms.get_mut(e) {
                if *t != main_t { *t = main_t; }
            }
        }
        (false, Some(e)) => {
            commands.entity(e).despawn();
            layer.cam3d = None;
        }
        (false, None) => {}
    }
    let Some(fr) = layer.frame.as_mut() else { return };
    let menu = rt.menu.as_ref();
    let frames = menu.and_then(|m| m.frames.as_ref()).and_then(|f| f.slots.as_ref().map(|s| (f, s)));
    let goodies = menu.is_some_and(|m| m.goodies);
    let fold = main_t.to_matrix() * fr.camera.inverse() * fr.model;
    let mut palette: Option<Vec<u8>> = None;
    let mut records: Option<Vec<u8>> = None;
    for i in 0..frame::SLOTS {
        let show = active && frames.is_some() && (i != 6 || goodies);
        let want = if show { Visibility::Inherited } else { Visibility::Hidden };
        for &e in &fr.entities[i] {
            // The gameplay setup hides every moby entity tagged with Ratchet's instance index (MeshTag 0 here)
            // by `SpawnHidden`, which `moby_spawn` enforces each frame: these are not his.
            if spawn_hidden.contains(e) { commands.entity(e).remove::<crate::moby_spawn::SpawnHidden>(); }
            if let Ok(mut v) = vis.get_mut(e) {
                if *v != want { *v = want; }
            }
        }
        let (true, Some((f, slots))) = (show, frames) else { continue };
        let pal = palette.get_or_insert_with(|| buffers.get(&fr.extra.palette).and_then(|b| b.data.clone()).unwrap_or_default());
        let rec = records.get_or_insert_with(|| buffers.get(&fr.extra.instances).and_then(|b| b.data.clone()).unwrap_or_default());
        // MobyAnimEval of the slot's state (the frame mobys have no pose layers and no blends).
        let m = rc_formats::moby_anim::evaluate_with_snapshot(&f.class.anim, &slots[i].state, None);
        let base = i * fr.slots as usize;
        let bytes: Vec<u8> = m.iter().take(fr.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).collect();
        if let Some(dst) = pal.get_mut(base * 64..base * 64 + bytes.len()) { dst.copy_from_slice(&bytes); }
        let r = moby_render::extra_record(&fold, fr.lights.as_ref(), base as u32);
        let at = i * moby_render::EXTRA_RECORD_SIZE;
        if let Some(dst) = rec.get_mut(at..at + r.len()) { dst.copy_from_slice(&r); }
        let t = Transform::from_matrix(fold);
        for &e in &fr.entities[i] {
            if let Ok(mut tr) = transforms.get_mut(e) {
                if *tr != t { *tr = t; }
            }
        }
    }
    for (h, data) in [(&fr.extra.palette, palette), (&fr.extra.instances, records)] {
        let Some(data) = data else { continue };
        if buffers.get(h).and_then(|b| b.data.as_ref()).is_some_and(|d| *d == data) { continue; }
        if let Some(mut b) = buffers.get_mut(h) { b.data = Some(data); }
    }
}

fn clip(a: [i32; 4], b: [i32; 4]) -> [i32; 4] { [a[0].max(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].min(b[3])] }

/// A draw translated into the panel at (ox, oy).
fn translate(d: &Draw, ox: i32, oy: i32) -> Draw {
    let mut d = d.clone();
    match &mut d {
        Draw::Sprite { x, y, .. } | Draw::Text { x, y, .. } | Draw::FxQuad { x, y, .. } => {
            *x += ox;
            *y += oy;
        }
        Draw::TextWindow { window: w, .. } => {
            w.x_min += ox as i16;
            w.x_max += ox as i16;
            w.x_anchor += ox as i16;
            w.y_min += oy as i16;
            w.y_max += oy as i16;
            w.y_start += oy as i16;
        }
        Draw::UiFrame { top, bottom, left, right, .. } => {
            *top += oy;
            *bottom += oy;
            *left += ox;
            *right += ox;
        }
    }
    d
}

/// Converts the menu draws to primitives; returns whether the snapshot is shown. The snapshot and the
/// menu's black 0x30 are the menu layer's (module docs), not primitives.
fn convert(draws: &[MenuDraw], h: &mut Hud2d, st: &mut TextState, glyphs: &[rc_formats::font::GlyphTable; 3]) -> bool {
    let full = [0, W - 1, 0, H - 1];
    let mut panel = full;
    let (mut ox, mut oy) = (0, 0);
    let mut snapshot = false;
    for d in draws {
        let start = h.prims.len();
        match d {
            MenuDraw::Hud(x) => text_render::execute(h, st, glyphs, std::slice::from_ref(&translate(x, ox, oy))),
            MenuDraw::Rect { x0, y0, x1, y1, rgba } => h.rect(y0 - 1 + oy, y1 - 1 + oy, x0 - 1 + ox, x1 - 1 + ox, *rgba),
            MenuDraw::Line { x0, y0, x1, y1, rgba } => {
                let (ax, ay, bx, by) = (x0 - 1 + ox, y0 - 1 + oy, x1 - 1 + ox, y1 - 1 + oy);
                // A 1-pixel-wide quad along the line (GS LINE, [M] for the end pixels).
                let pos = if (bx - ax).abs() >= (by - ay).abs() {
                    [[ax, ay], [bx, by], [ax, ay + 1], [bx, by + 1]]
                } else {
                    [[ax, ay], [ax + 1, ay], [bx, by], [bx + 1, by]]
                };
                h.prims.push(Prim { tex: Tex::None, pos, uv: [[0, 0]; 4], rgba: *rgba, scissor: full });
            }
            MenuDraw::SpriteUv { frame, x0, y0, x1, y1, u0, v0, u1, v1, alpha, repeat_u } => {
                let rgba = ((*alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
                let (px0, py0, px1, py1) = (x0 / 16 + ox, y0 / 16 + oy, x1 / 16 + ox, y1 / 16 + oy);
                let (ua, ub, va, vb) = (u0 / 16, u1 / 16, v0 / 16, v1 / 16);
                let tw = h.frame_sizes.get(*frame).map_or(0, |s| s.0);
                let mut quad = |xa: i32, xb: i32, ua: i32, ub: i32| {
                    h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[xa, py0], [xb, py0], [xa, py1], [xb, py1]], uv: [[ua, va], [ub, va], [ua, vb], [ub, vb]], rgba, scissor: full });
                };
                if *repeat_u && tw > 0 && ub - ua == tw {
                    // CLAMP_1 = REPEAT: split at the texture's wrap.
                    let us = ua.rem_euclid(tw);
                    let xm = px0 + ((px1 - px0) * (tw - us)) / tw;
                    quad(px0, xm, us, tw);
                    if us != 0 { quad(xm, px1, 0, us); }
                } else {
                    quad(px0, px1, ua, ub);
                }
            }
            MenuDraw::Snapshot => snapshot = true,
            // The menu layer's clear colour until the snapshot arrives, then in the snapshot's bytes.
            MenuDraw::Darken { alpha } if *alpha != DARKEN => h.rect(0, H, 0, W, ((*alpha as u32) & 0xff) << 24),
            MenuDraw::Darken { .. } => {}
            MenuDraw::PanelBegin { x, y, w, h: ph, clear } => {
                panel = [*x, x + w - 1, *y, y + ph - 1];
                (ox, oy) = (*x, *y);
                h.rect(*y, y + ph, *x, x + w, *clear);
            }
            MenuDraw::PanelEnd => {
                panel = full;
                (ox, oy) = (0, 0);
            }
            MenuDraw::Stub(_) => {}
        }
        for p in &mut h.prims[start..] { p.scissor = clip(p.scissor, panel); }
    }
    snapshot
}

// ---- Snapshot, render world (module docs, "Snapshot") ----

/// Main world: the entering frame's snapshot job (the main camera's target, the snapshot image); set by
/// `build_prims`, cleared in `First`, so exactly the entering frame's extraction sees it.
#[derive(Resource, Default)]
struct SnapshotRequest(Option<(NormalizedRenderTarget, AssetId<Image>)>);

/// Render world: this frame's snapshot job.
#[derive(Resource, Default)]
struct SnapshotJob(Option<(NormalizedRenderTarget, AssetId<Image>)>);

/// Render world: the texture a window target renders into on the snapshot frame (None otherwise, or when a
/// `Screenshot` already renders that window offscreen).
#[derive(Resource, Default)]
struct SnapshotOffscreen(Option<TextureView>);

/// Pipelines of `menu_snapshot.wgsl`, by (copy, source view is sRGB, target format).
#[derive(Resource)]
struct SnapshotPipelines {
    layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
    vertex: VertexState,
    ids: HashMap<(bool, bool, TextureFormat), CachedRenderPipelineId>,
}

impl SnapshotPipelines {
    fn id(&mut self, cache: &PipelineCache, copy: bool, src_srgb: bool, format: TextureFormat) -> CachedRenderPipelineId {
        let (layout, shader, vertex) = (&self.layout, &self.shader, &self.vertex);
        *self.ids.entry((copy, src_srgb, format)).or_insert_with(|| {
            let mut shader_defs = vec![ShaderDefVal::Int("DARKEN".into(), DARKEN)];
            if src_srgb { shader_defs.push("SRC_SRGB".into()); }
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some(if copy { "menu snapshot: window copy" } else { "menu snapshot: darken" }.into()),
                layout: vec![layout.clone()],
                vertex: vertex.clone(),
                fragment: Some(FragmentState {
                    shader: shader.clone(),
                    shader_defs,
                    entry_point: Some(if copy { "copy" } else { "darken" }.into()),
                    targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                }),
                ..default()
            })
        })
    }

    fn get<'a>(&self, cache: &'a PipelineCache, copy: bool, src_srgb: bool, format: TextureFormat) -> Option<&'a RenderPipeline> {
        self.ids.get(&(copy, src_srgb, format)).and_then(|id| cache.get_render_pipeline(*id))
    }
}

fn init_snapshot_pipelines(mut commands: Commands, assets: Res<AssetServer>, fullscreen: Res<FullscreenShader>) {
    commands.insert_resource(SnapshotPipelines {
        layout: BindGroupLayoutDescriptor::new(
            "menu snapshot",
            &BindGroupLayoutEntries::single(ShaderStages::FRAGMENT, texture_2d(TextureSampleType::Float { filterable: false })),
        ),
        shader: assets.load("shaders/menu_snapshot.wgsl"),
        vertex: fullscreen.to_vertex_state(),
        ids: HashMap::default(),
    });
}

fn extract_snapshot(mut job: ResMut<SnapshotJob>, request: Extract<Res<SnapshotRequest>>) { job.0 = request.0.clone(); }

/// Queues the pipelines every frame (so they are compiled before the menu is first entered) and, on the
/// snapshot frame of a window target, points the window's output attachment at a samplable texture.
fn prepare_snapshot(
    job: Res<SnapshotJob>,
    windows: Res<ExtractedWindows>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    mut pipes: ResMut<SnapshotPipelines>,
    mut attachments: ResMut<ViewTargetAttachments>,
    mut offscreen: ResMut<SnapshotOffscreen>,
) {
    offscreen.0 = None;
    for srgb in [false, true] { pipes.id(&cache, false, srgb, TextureFormat::Rgba8Unorm); }
    for w in windows.windows.values() {
        if let Some(f) = w.swap_chain_texture_view_format { pipes.id(&cache, true, f.is_srgb(), f); }
    }
    let Some((target @ NormalizedRenderTarget::Window(window), _)) = &job.0 else { return };
    let Some(w) = windows.windows.get(&window.entity()) else { return };
    let (Some(swap), Some(att)) = (w.swap_chain_texture_view.as_ref(), attachments.get(target)) else { return };
    // A `Screenshot` of this window already renders it into a samplable texture: that is read instead.
    if att.view.id() != swap.id() { return; }
    let format = att.view_format;
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("menu snapshot: window frame"),
        size: Extent3d { width: w.physical_width, height: w.physical_height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    attachments.insert(target.clone(), OutputColorAttachment::new(view.clone(), format));
    offscreen.0 = Some(view);
}

fn fullscreen_pass(encoder: &mut CommandEncoder, device: &RenderDevice, cache: &PipelineCache, layout: &BindGroupLayoutDescriptor, pipeline: &RenderPipeline, src: &TextureView, dst: &TextureView) {
    let bind_group = device.create_bind_group("menu snapshot", &cache.get_bind_group_layout(layout), &BindGroupEntries::single(src));
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("menu snapshot"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: dst,
            depth_slice: None,
            resolve_target: None,
            ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

/// After every camera of the frame has been submitted (`RenderGraphSystems::Finish`), before present and before
/// Bevy's screenshot copies: the target's finished frame → the snapshot texture, darkened; a window frame
/// rendered offscreen for it → back to the swap chain.
#[allow(clippy::too_many_arguments)]
fn snapshot_copy(
    job: Res<SnapshotJob>,
    offscreen: Res<SnapshotOffscreen>,
    attachments: Res<ViewTargetAttachments>,
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<GpuImage>>,
    pipes: Option<Res<SnapshotPipelines>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let (Some((target, dst)), Some(pipes)) = (&job.0, pipes) else { return };
    let Some(src) = attachments.get(target) else {
        eprintln!("menus: the main target has no output attachment this frame: no snapshot");
        return;
    };
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("menu snapshot") });
    match (images.get(*dst), pipes.get(&cache, false, src.view_format.is_srgb(), TextureFormat::Rgba8Unorm)) {
        (Some(dst), Some(pipeline)) => {
            let view = dst.texture.create_view(&TextureViewDescriptor { format: Some(TextureFormat::Rgba8Unorm), ..default() });
            fullscreen_pass(&mut encoder, &device, &cache, &pipes.layout, pipeline, &src.view, &view);
        }
        (None, _) => eprintln!("menus: the snapshot texture is not on the GPU: no snapshot"),
        (_, None) => eprintln!("menus: the snapshot pipeline is not compiled yet: no snapshot"),
    }
    // The window's frame went to our texture (not replaced by a screenshot's): present it.
    let window = match (&offscreen.0, target) {
        (Some(own), NormalizedRenderTarget::Window(w)) if own.id() == src.view.id() => windows.windows.get(&w.entity()).map(|w| (own, w)),
        _ => None,
    };
    if let Some((own, (Some(swap), Some(f)))) = window.map(|(own, w)| (own, (&w.swap_chain_texture_view, w.swap_chain_texture_view_format))) {
        match pipes.get(&cache, true, f.is_srgb(), f) {
            Some(pipeline) => fullscreen_pass(&mut encoder, &device, &cache, &pipes.layout, pipeline, own, swap),
            None => eprintln!("menus: the window copy pipeline is not compiled yet: this frame is not presented"),
        }
    }
    queue.submit([encoder.finish()]);
}
