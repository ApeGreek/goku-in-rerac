//! **Port-only: the game frame and its presentation.** The PS2 drew one 512×416 buffer; the port draws the whole
//! game (world, sky, HUD, menus, canvases) into one offscreen image of that shape, [`GameFrame`], and a last camera
//! ([`PresentCamera`]) draws it into the window, scaled to fit and centred on black. Every camera that would render
//! to the primary window is pointed at the frame as it is spawned ([`redirect`]), so nothing else needs to know
//! the window's size: overlays laid out on the frame (the HUD composite, the menu layer and its snapshot, the item
//! canvases) match it at any window size, the menu snapshot copies the frame without bars, and a screenshot is the
//! frame itself.
//!
//! **Settings** ([`DisplaySettings`], the Port Options page; persisted in the port settings file,
//! `crate::render_settings::save_key`): the render resolution — "Window" (the largest 512:416 rectangle that fits the
//! window, in physical pixels: what the window showed before) or a fixed multiple of 512×416 (×1 to ×4) scaled to the
//! window — and fullscreen (borderless on the current monitor). `RC_RESOLUTION=window|1|2|3|4` and
//! `RC_FULLSCREEN=0|1` override the file at start; F11 toggles fullscreen and saves it.
//!
//! The frame's format is the window's swap-chain format (`Bgra8UnormSrgb`), so every pipeline is specialised as it
//! was for the window; it can be copied (screenshots) and sampled (the present sprite, linear filtering).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat, TextureUsages};
use bevy::render::view::Msaa;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowRef};

use crate::game_camera::{SCREEN_H, SCREEN_W};

/// The present camera's own layer (nothing else draws in it).
pub const PRESENT_LAYER: usize = 63;

/// The offscreen image every game camera renders into, and its current size (physical pixels).
#[derive(Resource, Clone, Debug)]
pub struct GameFrame {
    pub image: Handle<Image>,
    pub size: UVec2,
}

/// The render resolution: fit to the window, or a fixed multiple of the PS2's 512×416.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Window,
    Scale(u8),
}

impl Resolution {
    /// The Port Options row's values, in order.
    pub const ALL: [Resolution; 5] = [Resolution::Window, Resolution::Scale(1), Resolution::Scale(2), Resolution::Scale(3), Resolution::Scale(4)];

    pub fn index(self) -> u8 { Self::ALL.iter().position(|&r| r == self).unwrap_or(0) as u8 }
    pub fn from_index(i: u8) -> Self { Self::ALL.get(i as usize).copied().unwrap_or(Resolution::Window) }

    fn parse(v: &str) -> Option<Self> {
        match v.trim() {
            "window" => Some(Resolution::Window),
            n => n.parse::<u8>().ok().filter(|k| (1..=4).contains(k)).map(Resolution::Scale),
        }
    }

    fn key(self) -> String {
        match self {
            Resolution::Window => "window".into(),
            Resolution::Scale(k) => k.to_string(),
        }
    }
}

/// The display settings a menu can change at run time.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplaySettings {
    pub resolution: Resolution,
    pub fullscreen: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self { DisplaySettings { resolution: Resolution::Window, fullscreen: false } }
}

impl DisplaySettings {
    /// The settings file's values, then `RC_RESOLUTION` / `RC_FULLSCREEN` over them.
    pub fn startup() -> Self {
        let mut s = DisplaySettings::default();
        if let Some(r) = crate::render_settings::load_key("resolution").as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Some(f) = crate::render_settings::load_key("fullscreen") { s.fullscreen = f.trim() == "1"; }
        if let Some(r) = std::env::var("RC_RESOLUTION").ok().as_deref().and_then(Resolution::parse) { s.resolution = r; }
        if let Ok(f) = std::env::var("RC_FULLSCREEN") { s.fullscreen = f.trim() == "1"; }
        s
    }

    /// Writes both keys into the port settings file.
    pub fn save(&self) {
        crate::render_settings::save_key("resolution", &self.resolution.key());
        crate::render_settings::save_key("fullscreen", if self.fullscreen { "1" } else { "0" });
    }
}

/// The frame's size for `res` in a window of `window` physical pixels: the largest 512:416 rectangle that fits, or
/// the fixed multiple.
pub fn frame_size(res: Resolution, window: UVec2) -> UVec2 {
    match res {
        Resolution::Scale(k) => UVec2::new(SCREEN_W as u32 * k as u32, SCREEN_H as u32 * k as u32),
        Resolution::Window => {
            let scale = (window.x as f32 / SCREEN_W).min(window.y as f32 / SCREEN_H);
            UVec2::new((SCREEN_W * scale).round() as u32, (SCREEN_H * scale).round() as u32).min(window).max(UVec2::ONE)
        }
    }
}

/// The largest rectangle of the frame's aspect that fits `window` (logical units): the present sprite's size.
pub fn fit(frame: UVec2, window: Vec2) -> Vec2 {
    let (fw, fh) = (frame.x.max(1) as f32, frame.y.max(1) as f32);
    // Cross-multiplied (no aspect quotient): a window of the frame's own aspect gets exactly its size.
    if window.x * fh > window.y * fw { Vec2::new(window.y * fw / fh, window.y) } else { Vec2::new(window.x, window.x * fh / fw) }
}

/// The camera that draws the frame into the window.
#[derive(Component)]
pub struct PresentCamera;

/// The frame's sprite on the present camera.
#[derive(Component)]
struct PresentSprite;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    /// The frame exists from `build` on, so plugins built later (crate::determinism's capture) can use it.
    fn build(&self, app: &mut App) {
        let size = UVec2::new(1024, 832);
        let mut img = Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
        img.asset_usage = RenderAssetUsages::RENDER_WORLD;
        img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        img.sampler = ImageSampler::linear();
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(img);
        let settings = DisplaySettings::startup();
        println!("display: render resolution {:?}, fullscreen {} (Port Options; RC_RESOLUTION / RC_FULLSCREEN)", settings.resolution, settings.fullscreen);
        app.insert_resource(GameFrame { image, size })
            .insert_resource(settings)
            .add_observer(redirect)
            .add_systems(Startup, spawn_present)
            .add_systems(First, sync_frame)
            .add_systems(Update, (toggle_fullscreen, apply_mode, fit_present).chain());
    }
}

/// A camera spawned for the primary window renders into the frame instead (all but the present camera).
fn redirect(add: On<Add, Camera>, frame: Res<GameFrame>, targets: Query<&RenderTarget>, present: Query<(), With<PresentCamera>>, mut commands: Commands) {
    if present.contains(add.entity) { return; }
    if matches!(targets.get(add.entity), Ok(RenderTarget::Window(WindowRef::Primary))) {
        commands.entity(add.entity).insert(RenderTarget::Image(frame.image.clone().into()));
    }
}

fn spawn_present(mut commands: Commands, frame: Res<GameFrame>) {
    commands.spawn((
        Camera2d,
        Camera { order: 1000, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        RenderTarget::Window(WindowRef::Primary),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(PRESENT_LAYER),
        PresentCamera,
        Name::new("present camera (game frame → window)"),
    ));
    commands.spawn((
        Sprite { image: frame.image.clone(), custom_size: Some(Vec2::new(1024.0, 832.0)), ..default() },
        RenderLayers::layer(PRESENT_LAYER),
        PresentSprite,
        Name::new("game frame"),
    ));
}

/// Keeps the frame at the size the settings and the window ask for.
fn sync_frame(mut frame: ResMut<GameFrame>, settings: Res<DisplaySettings>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut images: ResMut<Assets<Image>>) {
    let Some(w) = window else { return };
    let ws = w.physical_size();
    if ws.x == 0 || ws.y == 0 { return; }
    let want = frame_size(settings.resolution, ws);
    if want == frame.size { return; }
    if let Some(mut img) = images.get_mut(&frame.image) { img.resize(Extent3d { width: want.x, height: want.y, depth_or_array_layers: 1 }); }
    frame.size = want;
}

/// F11: fullscreen on / off (saved).
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<DisplaySettings>) {
    if !keys.just_pressed(KeyCode::F11) { return; }
    settings.fullscreen = !settings.fullscreen;
    settings.save();
}

/// The window mode follows the setting.
fn apply_mode(settings: Res<DisplaySettings>, mut window: Option<Single<&mut Window, With<PrimaryWindow>>>) {
    let Some(w) = window.as_deref_mut() else { return };
    let want = if settings.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
    if w.mode != want { w.mode = want; }
}

/// The frame's sprite fills the window at the frame's aspect, centred (the camera clears the rest to black).
fn fit_present(frame: Res<GameFrame>, window: Option<Single<&Window, With<PrimaryWindow>>>, mut sprites: Query<&mut Sprite, With<PresentSprite>>) {
    let Some(w) = window else { return };
    let size = fit(frame.size, Vec2::new(w.width(), w.height()));
    for mut s in &mut sprites {
        if s.custom_size != Some(size) { s.custom_size = Some(size); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(frame_size(Resolution::Window, UVec2::new(1024, 832)), UVec2::new(1024, 832));
        assert_eq!(frame_size(Resolution::Window, UVec2::new(2560, 1440)), UVec2::new(1772, 1440));
        assert_eq!(frame_size(Resolution::Scale(2), UVec2::new(300, 300)), UVec2::new(1024, 832));
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1600.0, 832.0)), Vec2::new(1024.0, 832.0));
        assert_eq!(fit(UVec2::new(1024, 832), Vec2::new(1024.0, 1000.0)), Vec2::new(1024.0, 832.0));
        for r in Resolution::ALL { assert_eq!(Resolution::from_index(r.index()), r); assert_eq!(Resolution::parse(&r.key()), Some(r)); }
    }
}
