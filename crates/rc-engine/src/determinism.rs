//! Frame-exact runs: a game tick counter that never reads the wall clock in deterministic mode, and a
//! screenshot taken at an exact frame number.
//!
//! Environment:
//! - `RC_SCREENSHOT_FRAME=N` capture the frame rendered by update N (1-based) to `RC_SCREENSHOT`, then exit.
//!   Implies `RC_DETERMINISTIC=1`. `main.rs` then does not schedule its wall-clock capture (`RC_SCREENSHOT_DELAY`).
//! - `RC_DETERMINISTIC=1` every update advances exactly one 60 Hz game tick: `Time<Real>`, `Time<Virtual>`
//!   and `Time` advance by exactly 1/60 s and `FixedUpdate` runs exactly once per update
//!   (`TimeUpdateStrategy::FixedTimesteps(1)` with a 60 Hz fixed clock), so everything driven by `Time`
//!   or `FixedUpdate` is a function of the frame number, not of how fast the machine renders.
//!
//! [`GameTicks`] is the game's 60 Hz logic tick count since level start: one per update in deterministic
//! mode; otherwise the catch-up count `floor(60·t_virtual)`, like the game's main loop at 30 fps.
//!
//! Always (independent of the mode above), for run-to-run identical frames:
//! - every `Camera3d` gets `NoIndirectDrawing` as it is spawned (`RC_INDIRECT=1` keeps Bevy's GPU indirect
//!   mode): indirect mode orders the instances of each instanced draw with GPU atomics, see
//!   [`no_indirect_drawing`];
//! - the blended (Transparent3d) phase breaks equal-distance ties by entity, see [`stable_transparent_order`].
//!
//! The capture also refuses to fire while the render world still has pipelines compiling (Bevy compiles
//! them asynchronously, so a mesh whose pipeline is not ready is silently skipped): if frame N is reached
//! with pipelines pending, it waits and says so, since that capture would not be comparable.
//!
//! Capture robustness (docs/plan/level_sweep.md H1/H2):
//! - **Offscreen target.** On macOS a window that is fully covered, on another Space or behind a
//!   full-screen app is "occluded": wgpu-hal's Metal surface then refuses to hand out a drawable
//!   (`SurfaceError::Occluded`), Bevy renders nothing to the window, and a window screenshot comes back all
//!   zeros for as long as the window stays hidden (2,000+ frames seen). With `RC_SCREENSHOT_FRAME` every
//!   camera that targets the primary window renders instead to an offscreen image of the window's physical
//!   size and swapchain format (`Bgra8UnormSrgb`), kept in sync with the window, and that image is captured:
//!   frame N no longer depends on the window being visible. The window itself stays blank.
//!   `RC_CAPTURE_WINDOW=1` captures the window as before.
//! - **Bounded retry.** A capture that still comes back blank is retried on later frames for at most
//!   [`MAX_RETRY_FRAMES`] frames past N; then whatever came back is saved, a warning names the frame, and the
//!   process exits with status 3 (the image is not frame N). A later-frame success also exits 3.
//! - **Monitor blips.** Bevy links each window to its monitor (`OnMonitor` → `HasWindows`, `linked_spawn`),
//!   so when winit briefly stops listing the monitor, `create_monitors` despawns the monitor entity and the
//!   window with it, and `exit_on_all_closed` ends the run ("Monitor removed" → "No windows are open,
//!   exiting", exit 0, no PNG). The link is dropped as soon as it is made (nothing here uses it), in every
//!   mode.
//! - **Exit before capture.** If the app exits before the capture was saved, the process exits with
//!   status 2 instead of 0.

use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::math::FloatOrd;
use bevy::prelude::*;
use bevy::render::render_phase::{sort_phase_system, ViewSortedRenderPhases};
use bevy::render::render_resource::PipelineCache;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::render::render_resource::{Extent3d, TextureFormat, TextureUsages};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{OnMonitor, PrimaryWindow, WindowRef};
use bevy::render::view::NoIndirectDrawing;
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::time::TimeUpdateStrategy;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Frames past `RC_SCREENSHOT_FRAME` a blank capture is retried for before it is saved anyway.
pub const MAX_RETRY_FRAMES: u64 = 30;

/// Game logic ticks per second (NTSC).
pub const TICK_HZ: f64 = 60.0;

/// 60 Hz game ticks since start (see the module docs); updated in `First`.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct GameTicks(pub u64);

/// Updates run so far, 1 in the first `Update`; updated in `First`.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct FrameNumber(pub u64);

/// Whether this run is frame-exact (`RC_DETERMINISTIC=1` or `RC_SCREENSHOT_FRAME` set).
#[derive(Resource, Clone, Copy, Debug)]
pub struct Deterministic(pub bool);

fn env(name: &str) -> Option<String> { std::env::var(name).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) }

pub fn screenshot_frame() -> Option<u64> { env("RC_SCREENSHOT_FRAME").and_then(|v| v.parse().ok()) }

pub fn deterministic() -> bool { screenshot_frame().is_some() || env("RC_DETERMINISTIC").as_deref() == Some("1") }

/// Pipeline cache state as seen by the render world at the end of its last frame.
#[derive(Resource, Clone, Default)]
struct PipelineStatus(Arc<[AtomicU64; 2]>);

impl PipelineStatus {
    /// (pipelines known, pipelines still waiting to be created).
    fn get(&self) -> (u64, u64) { (self.0[0].load(Ordering::Acquire), self.0[1].load(Ordering::Acquire)) }
}

#[derive(Resource)]
struct FrameShot {
    path: PathBuf,
    frame: u64,
    /// Set by the capture observer when the image came back blank.
    retry: Arc<AtomicBool>,
    /// Set by the capture observer once it has saved (or given up on) the image.
    saved: Arc<AtomicBool>,
    /// The offscreen image the cameras render to (None with `RC_CAPTURE_WINDOW=1`).
    target: Option<Handle<Image>>,
}

pub struct DeterminismPlugin;

impl Plugin for DeterminismPlugin {
    fn build(&self, app: &mut App) {
        let det = deterministic();
        app.insert_resource(Deterministic(det))
            .init_resource::<GameTicks>()
            .init_resource::<FrameNumber>()
            .add_systems(First, advance_ticks.after(bevy::time::TimeSystems));
        if env("RC_INDIRECT").as_deref() != Some("1") { app.add_observer(no_indirect_drawing); }
        app.add_observer(unlink_monitor);
        if det {
            app.insert_resource(Time::<Fixed>::from_hz(TICK_HZ)).insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
            println!("determinism: frame-exact mode (1 tick of 1/60 s per update)");
        }
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                stable_transparent_order.after(sort_phase_system::<Transparent3d>).in_set(RenderSystems::PhaseSort),
            );
        }
        let Some(frame) = screenshot_frame() else { return };
        let Some(path) = std::env::var_os("RC_SCREENSHOT") else {
            eprintln!("RC_SCREENSHOT_FRAME is set without RC_SCREENSHOT: no capture");
            return;
        };
        let target = (env("RC_CAPTURE_WINDOW").as_deref() != Some("1")).then(|| {
            let mut img = Image::new_target_texture(1024, 832, TextureFormat::Bgra8UnormSrgb, None);
            img.asset_usage = RenderAssetUsages::RENDER_WORLD;
            img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
            app.world_mut().resource_mut::<Assets<Image>>().add(img)
        });
        if target.is_some() {
            println!("determinism: capturing an offscreen copy of the window (RC_CAPTURE_WINDOW=1 captures the window)");
            app.add_observer(render_offscreen).add_systems(PostUpdate, sync_capture_size);
        }
        let status = PipelineStatus::default();
        app.insert_resource(status.clone())
            .insert_resource(FrameShot { path: path.into(), frame: frame.max(1), retry: Arc::default(), saved: Arc::default(), target })
            .add_systems(Update, frame_screenshot)
            .add_systems(Last, exit_before_capture.after(bevy::window::ExitSystems));
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(status).add_systems(Render, record_pipelines.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Bevy's indirect mode (GPU preprocessing with culling) gives every instance of a batch its output slot
/// with an `atomicAdd` in `mesh_preprocess.wgsl`, so the order of instances inside one instanced draw
/// changes from frame to frame and run to run. Where instances of one mesh overlap at exactly equal depth
/// (coplanar decal-like mobys, instances placed at the same spot) the later one wins the depth test, so
/// the pixel flips. Without indirect drawing the CPU assigns the slots in bin order. Inserted as each
/// camera is spawned (Bevy supports it only from spawn).
fn no_indirect_drawing(add: On<Add, Camera3d>, mut commands: Commands) {
    commands.entity(add.entity).insert(NoIndirectDrawing);
}

fn advance_ticks(det: Res<Deterministic>, time: Res<Time<Virtual>>, mut ticks: ResMut<GameTicks>, mut frame: ResMut<FrameNumber>) {
    frame.0 += 1;
    ticks.0 = if det.0 { frame.0 } else { (time.elapsed().as_nanos() * TICK_HZ as u128 / 1_000_000_000) as u64 };
}

/// Bevy sorts the blended (Transparent3d) phase by the view depth of each entity's origin (plus the
/// material's depth bias) with a stable sort over a retained map whose order is the order items were first
/// queued, i.e. the order their pipelines finished compiling on the async task pool. Items at equal depth
/// (every part entity of one tie / shrub / moby instance shares its transform) therefore drew in a
/// run-dependent order. Break those ties by the main-world entity, whose index follows the deterministic
/// spawn order (per instance, its parts in class order).
fn stable_transparent_order(mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>) {
    for phase in phases.values_mut() {
        phase.items.sort_by(|ka, a, kb, b| FloatOrd(a.distance).cmp(&FloatOrd(b.distance)).then_with(|| ka.1.cmp(&kb.1)));
    }
}

fn record_pipelines(cache: Res<PipelineCache>, status: Res<PipelineStatus>) {
    status.0[0].store(cache.pipelines().count() as u64, Ordering::Release);
    status.0[1].store(cache.waiting_pipelines().count() as u64, Ordering::Release);
}

/// See the module docs ("Monitor blips"): a window must not be despawned with a monitor entity.
fn unlink_monitor(add: On<Insert, OnMonitor>, mut commands: Commands) {
    commands.entity(add.entity).try_remove::<OnMonitor>();
}

/// Capture mode: a camera spawned for the primary window renders to the offscreen capture image.
fn render_offscreen(add: On<Add, Camera>, shot: Res<FrameShot>, targets: Query<&RenderTarget>, mut commands: Commands) {
    let Some(image) = &shot.target else { return };
    if matches!(targets.get(add.entity), Ok(RenderTarget::Window(WindowRef::Primary))) {
        commands.entity(add.entity).insert(RenderTarget::Image(image.clone().into()));
    }
}

/// Keeps the capture image at the primary window's physical size (the letterbox viewport and the projection
/// are computed from the window).
fn sync_capture_size(shot: Res<FrameShot>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut images: ResMut<Assets<Image>>) {
    let (Some(h), Some(w)) = (&shot.target, window) else { return };
    let size = w.physical_size();
    if size.x == 0 || size.y == 0 { return; }
    let Some(img) = images.get(h) else { return };
    if img.texture_descriptor.size.width == size.x && img.texture_descriptor.size.height == size.y { return; }
    if let Some(mut img) = images.get_mut(h) { img.resize(Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 }); }
}

/// H1: the run ends (window closed, monitor blip) before the capture was saved: exit status 2.
fn exit_before_capture(mut exits: MessageReader<AppExit>, shot: Res<FrameShot>, frame: Res<FrameNumber>) {
    if exits.read().next().is_some() && !shot.saved.load(Ordering::Acquire) {
        eprintln!("determinism: the app is exiting at frame {} before the frame-{} capture was saved; exit status 2", frame.0, shot.frame);
        std::process::exit(2);
    }
}

fn frame_screenshot(
    mut commands: Commands,
    shot: Res<FrameShot>,
    frame: Res<FrameNumber>,
    status: Res<PipelineStatus>,
    mut done: Local<bool>,
    mut warned: Local<bool>,
) {
    if shot.retry.swap(false, Ordering::AcqRel) { *done = false; }
    if *done || frame.0 < shot.frame { return; }
    let (total, waiting) = status.get();
    if waiting != 0 || total == 0 {
        if !*warned {
            eprintln!("determinism: frame {} reached with {waiting} of {total} pipelines still compiling; waiting (raise RC_SCREENSHOT_FRAME)", shot.frame);
            *warned = true;
        }
        return;
    }
    *done = true;
    let (path, n, want, retry, saved) = (shot.path.clone(), frame.0, shot.frame, shot.retry.clone(), shot.saved.clone());
    let give_up = n >= want + MAX_RETRY_FRAMES;
    let request = match &shot.target {
        Some(image) => Screenshot::image(image.clone()),
        None => Screenshot::primary_window(),
    };
    commands.spawn(request).observe(
        move |shot: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            let mut status = 0;
            match shot.image.clone().try_into_dynamic() {
                Ok(img) => {
                    let rgb = img.to_rgb8();
                    // An all-zero image is a frame that was never drawn (an occluded window: see the module docs);
                    // nothing the game draws is pure black edge to edge. Capture a later frame instead, for at most
                    // MAX_RETRY_FRAMES frames.
                    if rgb.as_raw().iter().all(|&b| b == 0) {
                        if !give_up {
                            eprintln!("determinism: frame {n} came back blank (window not presented?); capturing a later frame");
                            retry.store(true, Ordering::Release);
                            return;
                        }
                        eprintln!("determinism: WARNING frame {n} came back blank and {MAX_RETRY_FRAMES} frames of retries are used up; saving it anyway");
                    }
                    match rgb.save(&path) {
                        Ok(()) => println!("screenshot of frame {n} saved to {}", path.display()),
                        Err(e) => { eprintln!("cannot save screenshot to {}: {e}", path.display()); status = 1; }
                    }
                    if n != want {
                        eprintln!("determinism: WARNING the saved screenshot is frame {n}, not frame {want}; exit status 3");
                        status = status.max(3);
                    }
                }
                Err(e) => { eprintln!("cannot convert screenshot: {e:?}"); status = 1; }
            }
            saved.store(true, Ordering::Release);
            if status != 0 { std::process::exit(status); }
            exit.write(AppExit::Success);
        },
    );
}
