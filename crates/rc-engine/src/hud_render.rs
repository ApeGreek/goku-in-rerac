//! The 2D pass: HUD sprites, text quads and rectangles as the game sends them to the GS (PATH2 DIRECT packets,
//! no VU1 program), drawn after all 3D. Spec: docs/plan/hud_text.md §2; the calls come from
//! [`rc_game::hud::HudState`] through [`crate::text_render`].
//!
//! **Primitives** ([`Hud2d`]): `HudSprite` 0x2500e0 (SPRITE, PRIM 0x156, UV (0,0)..(tw,th) stretched over w×h,
//! RGBAQ = `alpha << 24 | 0x7f7f7f`), its 180° / 90° TRISTRIP variants 0x250468 / 0x2506c8 (PRIM 0x154),
//! `DrawTexturedQuad` 0x21be90 (TRISTRIP, glyphs and FX textures, UV = (u, v)..(u + tw, v + th)) and
//! `DrawRectOverlay` 0x21bce0 (untextured TRISTRIP 0x144). All are flat-shaded, blended, at Z 0xfffff0.
//!
//! **Mapping.** A corner at game pixel (x, y) is sent as `X = x·16 + OFX − 8` with `OFX = (2048 − 256)·16`, i.e.
//! at window pixel `x − 0.5`. The GS samples pixel *X* at its integer position, the GPU at `X + 0.5`, so the
//! corner lands exactly on GPU coordinate `x` of a 512×416 target and a w-wide primitive covers pixels
//! `x .. x + w − 1`, as on the GS. UVs are texel·16 (FST): the vertex UV here is in texels; at pixel `px` the
//! interpolated U is `u0 + (px + 0.5 − x)·Δu/Δx`, the GS's value.
//!
//! **Pixel pipeline** (`assets/shaders/hud.wgsl`). Texture: TEX1 bilinear + CLAMP inherited from the AA blit's
//! A+D block (the HUD packets set neither), done by hand on an atlas of every HUD frame and FX texture (raw GS
//! bytes, alpha 0x80 = 1.0): sample position quantised to 1/16 texel (UV is 12.4 fixed point), four texels
//! clamped to the texture's own rectangle, weights `k/16`, result truncated [M]. MODULATE: `C = Ct·Cf >> 7`,
//! `A = At·Af >> 7` (clamped to 0xff), so RGB 0x7f gives `tex × 127/128`. Blend ALPHA_1 0x44
//! `(Cs − Cd)·As/128 + Cd` ([`GsPass::Hud`]: TEST_1 0x5380b passes RGB either way and Z is constant, so one
//! blended draw is exact). Untextured rectangles use the vertex RGBA. Scissor (`FontPrintWindow`) per
//! primitive.
//!
//! **Where it draws.** The primitives render in submission (painter's) order into an offscreen 512×416
//! `Rgba16Float` target on a dedicated `Camera2d` (order −10, [`HUD_LAYER`]), at the game's own resolution:
//! the GS's bilinear filter works on 512×416 pixels, so filtering at the window's 2× resolution would be a
//! different (smoother) image. Blending there runs in display bytes / 255 without clamping, premultiplied
//! (the target keeps `Σ Cs·As` in RGB and the coverage in A), which is exact for HUD-on-HUD overlaps and
//! keeps As > 0x80 (orb glow) as the GS computes it over opaque HUD pixels. The result is composited onto the
//! main camera by a Bevy UI node ([`HudComposite`], `UiMaterial`) filling the camera's letterboxed 512×416
//! viewport (`game_camera::letterbox`), sampled **nearest** (integer pixel doubling at 1024×832), in the UI
//! pass: after every 3D pass and before the underwater tint (`fog_state::UnderwaterTint`, scheduled after
//! `ui_pass`), which is the game's order (the tint is in the `0x15f3f4 & 0x40` pass after the HUD's `& 0x80`).
//! The composite mixes in linear light on the sRGB target (exact where the HUD coverage is 0 or 1; the GS
//! mixes display bytes), the difference the world passes also accept (gs_state.rs).
//!
//! Environment: `RC_HUD=0` disables the HUD; `RC_HUD_DEMO=1` sets bolts to 1234 at tick 60 and drops HP to
//! 3 at tick 180; `RC_HUD_TEXT="…"` shows it as a banner (`ShowBanner` path, 180 ticks) from tick 1, or with
//! `RC_HUD_TEXT_WINDOW=1` as `FontPrintWindow` text (regular font) centred in a `DrawUIFrame` at y = 100;
//! `RC_HUD_HELP=<id>` opens the help box with that level message at tick 1; `RC_LANG` = game language
//! (0 En, 2 Fr, 3 De, 4 Es, 5 It).

use crate::gs_state::GsPass;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{ClearColorConfig, Hdr, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat};
use bevy::render::view::Msaa;
use bevy::shader::{ShaderDefVal, ShaderRef};
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d};
use bevy::ui::UiTargetCamera;
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialKey, UiMaterialPlugin};
use rc_formats::font::GlyphTable;
use rc_formats::hud::Hud;
use rc_formats::strings::Message;
use rc_formats::texture::Texture;
use rc_game::hud::{Draw, HudAssets, HudState, Inputs, Rot};
use std::path::Path;

const SHADER_PATH: &str = "shaders/hud.wgsl";
/// Render layer of the offscreen HUD camera and its mesh.
pub const HUD_LAYER: usize = 29;
/// The GS draw buffer (NTSC).
pub const W: i32 = 512;
pub const H: i32 = 416;
/// Atlas width in texels.
const ATLAS_W: u32 = 1024;

/// Per vertex: texel UV.
pub const ATTRIBUTE_UV: MeshVertexAttribute = MeshVertexAttribute::new("HudUv", 0x4855_4401, VertexFormat::Float32x2);
/// Per vertex: RGBA bytes (R low).
pub const ATTRIBUTE_RGBA: MeshVertexAttribute = MeshVertexAttribute::new("HudRgba", 0x4855_4402, VertexFormat::Uint32);
/// Per vertex: atlas x | y << 16, texture w | h << 16, flags (1 = textured), 0.
pub const ATTRIBUTE_TEX: MeshVertexAttribute = MeshVertexAttribute::new("HudTex", 0x4855_4403, VertexFormat::Uint32x4);
/// Per vertex: scissor x0, x1, y0, y1 (inclusive pixels).
pub const ATTRIBUTE_SCISSOR: MeshVertexAttribute = MeshVertexAttribute::new("HudScissor", 0x4855_4404, VertexFormat::Uint32x4);

/// A texture a primitive samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tex {
    None,
    /// HUD frame (`GetFrameTex`).
    Frame(usize),
    /// FX texture n (`GetEffectTex(n)`: 1..3 fonts, 4 the Gadgetron logo).
    Fx(usize),
}

/// One GS primitive: four corners in strip order (v0 v1 v2 v3: triangles 012, 123), game pixels, with texel UVs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prim {
    pub tex: Tex,
    pub pos: [[i32; 2]; 4],
    pub uv: [[i32; 2]; 4],
    pub rgba: u32,
    /// x0, x1, y0, y1, inclusive.
    pub scissor: [i32; 4],
}

impl Prim {
    /// Bounding rectangle `[x, y, w, h]` (tests).
    #[allow(dead_code)]
    pub fn rect(&self) -> [i32; 4] {
        let (x0, x1) = (self.pos.iter().map(|p| p[0]).min().unwrap(), self.pos.iter().map(|p| p[0]).max().unwrap());
        let (y0, y1) = (self.pos.iter().map(|p| p[1]).min().unwrap(), self.pos.iter().map(|p| p[1]).max().unwrap());
        [x0, y0, x1 - x0, y1 - y0]
    }
}

const FULL_SCISSOR: [i32; 4] = [0, W - 1, 0, H - 1];

/// The frame's 2D primitive list, in draw order.
#[derive(Clone, Debug)]
pub struct Hud2d {
    pub prims: Vec<Prim>,
    scissor: [i32; 4],
    /// Texture size of every HUD frame.
    pub frame_sizes: Vec<(i32, i32)>,
}

impl Default for Hud2d {
    fn default() -> Self { Hud2d { prims: Vec::new(), scissor: FULL_SCISSOR, frame_sizes: Vec::new() } }
}

impl Hud2d {
    pub fn clear(&mut self) {
        self.prims.clear();
        self.scissor = FULL_SCISSOR;
    }

    /// `VU1_setScissor(x0, x1, y0, y1)`.
    pub fn set_scissor(&mut self, x0: i32, x1: i32, y0: i32, y1: i32) { self.scissor = [x0, x1, y0, y1]; }
    pub fn reset_scissor(&mut self) { self.scissor = FULL_SCISSOR; }

    fn push(&mut self, tex: Tex, pos: [[i32; 2]; 4], uv: [[i32; 2]; 4], rgba: u32) {
        self.prims.push(Prim { tex, pos, uv, rgba, scissor: self.scissor });
    }

    /// `HudSprite(frame, x, y, w, h, alpha)` and its rotated variants.
    #[allow(clippy::too_many_arguments)]
    pub fn sprite(&mut self, frame: usize, x: i32, y: i32, w: i32, h: i32, alpha: i32, rot: Rot) {
        let (tw, th) = self.frame_sizes.get(frame).copied().unwrap_or((0, 0));
        let rgba = ((alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
        let (x1, y1) = (x + w, y + h);
        let (pos, uv) = match rot {
            Rot::None => ([[x, y], [x1, y], [x, y1], [x1, y1]], [[0, 0], [tw, 0], [0, th], [tw, th]]),
            Rot::R180 => ([[x, y1], [x, y], [x1, y1], [x1, y]], [[tw, 0], [tw, th], [0, 0], [0, th]]),
            Rot::R90 => ([[x, y], [x1, y], [x, y1], [x1, y1]], [[tw, 0], [tw, th], [0, 0], [0, th]]),
        };
        self.push(Tex::Frame(frame), pos, uv, rgba);
    }

    /// `DrawTexturedQuad(x, y, w, h, u, v, tw, th, rgba, GetEffectTex(fx))`.
    #[allow(clippy::too_many_arguments)]
    pub fn strip_glyph(&mut self, fx: usize, x: i32, y: i32, w: i32, h: i32, u: i32, v: i32, tw: i32, th: i32, rgba: u32) {
        let (x1, y1, u1, v1) = (x + w, y + h, u + tw, v + th);
        self.push(Tex::Fx(fx), [[x, y], [x1, y], [x, y1], [x1, y1]], [[u, v], [u1, v], [u, v1], [u1, v1]], rgba);
    }

    /// `DrawRectOverlay(top, bottom, left, right, rgba)`: pixels `left..right−1` × `top..bottom−1`.
    pub fn rect(&mut self, top: i32, bottom: i32, left: i32, right: i32, rgba: u32) {
        self.push(Tex::None, [[left, top], [right, top], [left, bottom], [right, bottom]], [[0, 0]; 4], rgba);
    }
}

/// The level's HUD data.
pub struct LevelHud {
    pub hud: Hud,
    /// Every frame with raw GS alpha.
    pub frames: Vec<Texture>,
    /// FX textures (raw alpha), index = `GetEffectTex` argument.
    pub fx: Vec<Option<Texture>>,
    pub glyphs: [GlyphTable; 3],
    pub glyph_addrs: [u32; 3],
    pub messages: Vec<Message>,
    pub lang: u32,
}

/// `RC_LANG`, default English.
pub fn language() -> u32 { std::env::var("RC_LANG").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(rc_formats::strings::lang::ENGLISH) }

/// Reads `hud_header`, the banks, the overlay's glyph tables, the FX textures and the level text.
pub fn load(root: &Path, index: u32, core: &rc_formats::level::LevelCore, core_index: &[u8], core_data: &[u8], gameplay: &[u8]) -> Result<LevelHud> {
    use rc_formats::{font, hud, particle_tex, strings, wad};
    let read = |name: &str| crate::disc_source::level_file(root, index, name);
    let header = read("hud_header.bin")?;
    let h = hud::parse_header(&header)?;
    let mut banks: [Vec<u8>; hud::BANKS] = Default::default();
    for (b, bank) in banks.iter_mut().enumerate() {
        if h.bank_size[b] != 0 { *bank = wad::decompress(&read(&format!("hud_bank_{b}.bin"))?).with_context(|| format!("hud bank {b}"))?; }
    }
    let hud = hud::parse_hud(&header, std::array::from_fn(|b| banks[b].as_slice())).context("parsing hud")?;
    let frames = (0..hud.frames.len()).map(|i| hud.decode_frame_raw(i)).collect::<rc_formats::buf::Result<Vec<_>>>()?;
    let (glyphs, glyph_addrs) = font::parse_glyph_tables(&read("overlay.bin")?).context("glyph tables")?;
    let fx_entries = particle_tex::parse_particle_textures(core, core_index, core_data).context("fx textures")?.fx_entries;
    let fx_bank = core.blocks.iter().find(|b| b.name == "fx_bank").map(|b| &core_data[b.offset..b.offset + b.size]).unwrap_or(&[]);
    let fx = fx_entries
        .iter()
        .map(|e| {
            if !e.present() { return Ok(None); }
            let px = fx_bank.get(e.texture as usize..).context("fx pixels")?;
            let clut = fx_bank.get(e.palette as usize..).context("fx palette")?;
            Ok(Some(hud::decode_indexed8_raw(px, e.width as u32, e.height as u32, clut)?))
        })
        .collect::<Result<Vec<_>>>()?;
    let lang = language();
    let messages = strings::parse_strings(gameplay, lang).context("level text")?;
    Ok(LevelHud { hud, frames, fx, glyphs, glyph_addrs, messages, lang })
}

/// Rectangles of every texture in the atlas.
struct Atlas {
    rgba: Vec<u8>,
    height: u32,
    frames: Vec<[u32; 4]>,
    fx: Vec<Option<[u32; 4]>>,
}

/// Shelf-packs the frames and FX textures (tallest first) into one raw-byte image.
fn build_atlas(frames: &[Texture], fx: &[Option<Texture>]) -> Atlas {
    let mut items: Vec<(usize, &Texture)> = frames.iter().enumerate().chain(fx.iter().enumerate().filter_map(|(i, t)| t.as_ref().map(|t| (frames.len() + i, t)))).collect();
    items.sort_by_key(|(i, t)| (std::cmp::Reverse(t.height), std::cmp::Reverse(t.width), *i));
    let mut rects = vec![[0u32; 4]; frames.len() + fx.len()];
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for (i, t) in &items {
        if x + t.width > ATLAS_W {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        rects[*i] = [x, y, t.width, t.height];
        x += t.width;
        shelf = shelf.max(t.height);
    }
    let height = (y + shelf).max(1);
    let mut rgba = vec![0u8; (ATLAS_W * height * 4) as usize];
    for (i, t) in &items {
        let [rx, ry, w, _] = rects[*i];
        for row in 0..t.height {
            let src = &t.rgba[(row * w * 4) as usize..((row + 1) * w * 4) as usize];
            let dst = (((ry + row) * ATLAS_W + rx) * 4) as usize;
            rgba[dst..dst + src.len()].copy_from_slice(src);
        }
    }
    let fx_rects = (0..fx.len()).map(|i| fx[i].as_ref().map(|_| rects[frames.len() + i])).collect();
    rects.truncate(frames.len());
    Atlas { rgba, height, frames: rects, fx: fx_rects }
}

/// The 2D primitives' material: the atlas.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HudMaterial {
    #[texture(0)]
    pub atlas: Handle<Image>,
}

impl Material2d for HudMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode2d { AlphaMode2d::Blend }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, layout: &MeshVertexBufferLayoutRef, _key: Material2dKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_UV.at_shader_location(1),
            ATTRIBUTE_RGBA.at_shader_location(2),
            ATTRIBUTE_TEX.at_shader_location(3),
            ATTRIBUTE_SCISSOR.at_shader_location(4),
        ])?];
        descriptor.vertex.shader_defs.push(ShaderDefVal::Bool("HUD_PRIMS".into(), true));
        if let Some(f) = descriptor.fragment.as_mut() { f.shader_defs.push(ShaderDefVal::Bool("HUD_PRIMS".into(), true)); }
        descriptor.primitive.cull_mode = None;
        GsPass::Hud.specialize(descriptor);
        Ok(())
    }
}

/// The UI node that puts the 512×416 HUD image on the main camera (nearest, premultiplied → straight alpha).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HudComposite {
    #[texture(0)]
    pub image: Handle<Image>,
}

impl UiMaterial for HudComposite {
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        if let Some(f) = descriptor.fragment.as_mut() {
            f.entry_point = Some("composite".into());
            f.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
        }
        descriptor.vertex.shader_defs.push(ShaderDefVal::Bool("HUD_COMPOSITE".into(), true));
    }
}

/// Demo / debug requests from the environment.
#[derive(Clone, Debug, Default)]
struct HudEnv {
    demo: bool,
    text: Option<Vec<u8>>,
    text_window: bool,
    help: Option<i32>,
}

impl HudEnv {
    fn read() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        HudEnv {
            demo: var("RC_HUD_DEMO").as_deref() == Some("1"),
            text: var("RC_HUD_TEXT").map(|s| s.into_bytes()),
            text_window: var("RC_HUD_TEXT_WINDOW").as_deref() == Some("1"),
            help: var("RC_HUD_HELP").and_then(|v| v.trim().parse().ok()),
        }
    }
}

#[derive(Resource)]
struct HudRuntime {
    state: HudState,
    env: HudEnv,
    glyphs: [GlyphTable; 3],
    atlas_frames: Vec<[u32; 4]>,
    atlas_fx: Vec<Option<[u32; 4]>>,
    mesh: Handle<Mesh>,
    ticks_done: u64,
    draws: Vec<Draw>,
    hud2d: Hud2d,
    game: Inputs,
}

#[derive(Component)]
struct HudMesh;

#[derive(Component)]
struct HudCompositeNode;

pub struct HudPlugin;

/// Other 2D layers drawn through this pass (crate::menu_render: the quick-select ring, which is HUD slot 3,
/// and the mode-3 page menus). Set before [`HudBuild`] each frame.
#[derive(Resource, Default)]
pub struct Hud2dHook {
    /// Appended after the HUD's own primitives.
    pub prims: Vec<Prim>,
    /// The HUD's own calls are not drawn (mode 3 draws no HUD).
    pub replace_hud: bool,
    /// The HUD update loop does not run (it is part of the mode-0 render only).
    pub freeze: bool,
}

/// The mode-2 scene layer (crate::scene_render), separate from [`Hud2dHook`] (which the menus rewrite every
/// frame): while a scene runs the draw mask is 0x7f, so the HUD neither updates nor draws, and the subtitle
/// box primitives (`fun_001f4be0`) are appended last. Set before [`HudBuild`] each frame.
#[derive(Resource, Default)]
pub struct SceneLayer {
    pub hide_hud: bool,
    pub prims: Vec<Prim>,
}

/// The system set that ticks the HUD and builds the primitive mesh.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HudBuild;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hud2dHook>().init_resource::<SceneLayer>();
        if std::env::var("RC_HUD").is_ok_and(|v| v.trim() == "0") { return; }
        app.add_plugins((Material2dPlugin::<HudMaterial>::default(), UiMaterialPlugin::<HudComposite>::default()))
            .add_systems(Startup, setup)
            .add_systems(Update, (target_main_camera, tick_and_build).chain().in_set(HudBuild));
    }
}

fn setup(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HudMaterial>>,
    mut composites: ResMut<Assets<HudComposite>>,
) {
    let Some(lh) = level.0.hud.as_ref() else {
        eprintln!("hud: no HUD data for this level");
        return;
    };
    let atlas = build_atlas(&lh.frames, &lh.fx);
    let mut img = Image::new(
        Extent3d { width: ATLAS_W, height: atlas.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        atlas.rgba,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    let atlas_handle = images.add(img);
    let target = images.add(Image::new_target_texture(W as u32, H as u32, TextureFormat::Rgba16Float, None));

    commands.spawn((
        Camera2d,
        Camera { order: -10, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(target.clone().into()),
        Hdr,
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        RenderLayers::layer(HUD_LAYER),
        Name::new("hud camera (512x416 offscreen)"),
    ));
    let mesh = meshes.add(empty_mesh());
    commands.spawn((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(HudMaterial { atlas: atlas_handle })),
        Transform::IDENTITY,
        NoFrustumCulling,
        RenderLayers::layer(HUD_LAYER),
        HudMesh,
        Name::new("hud primitives"),
    ));
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        MaterialNode(composites.add(HudComposite { image: target })),
        GlobalZIndex(i32::MAX),
        HudCompositeNode,
        Name::new("hud composite"),
    ));

    let assets = HudAssets::new(&lh.hud, lh.glyphs, lh.messages.clone());
    let frame_sizes = assets.frame_sizes.clone();
    let env = HudEnv::read();
    println!(
        "hud: {} icons, {} frames, {} fx textures, glyph tables {:#x}/{:#x}/{:#x}, {} messages (language {}), atlas {}x{}{}{}{}",
        lh.hud.icons.len() - 1, lh.frames.len(), lh.fx.iter().flatten().count(), lh.glyph_addrs[0], lh.glyph_addrs[1], lh.glyph_addrs[2],
        lh.messages.len(), lh.lang, ATLAS_W, atlas.height,
        if env.demo { "; RC_HUD_DEMO: bolts 1234 at tick 60, HP 3 at tick 180" } else { "" },
        env.text.as_ref().map_or(String::new(), |t| format!("; RC_HUD_TEXT {:?}{}", String::from_utf8_lossy(t), if env.text_window { " (window)" } else { "" })),
        env.help.map_or(String::new(), |id| format!("; RC_HUD_HELP {id}: {}", rc_formats::strings::display(rc_formats::strings::lookup(&lh.messages, id)))),
    );
    commands.insert_resource(HudRuntime {
        state: HudState::new(assets),
        env,
        glyphs: lh.glyphs,
        atlas_frames: atlas.frames,
        atlas_fx: atlas.fx,
        mesh,
        ticks_done: 0,
        draws: Vec::new(),
        hud2d: Hud2d { frame_sizes, ..default() },
        // Until the first frame reads the game state (Persistent / Session / HeldWeapon): 4/4, no bolts, no slot.
        game: Inputs { hp: 4, max_hp: 4, bolts: 0, weapon: None, lang: lh.lang },
    });
}

/// The composite node goes to the main (fly) camera's UI pass.
fn target_main_camera(mut commands: Commands, nodes: Query<Entity, (With<HudCompositeNode>, Without<UiTargetCamera>)>, cams: Query<Entity, With<crate::fly_cam::FlyCam>>) {
    let Some(cam) = cams.iter().next() else { return };
    for n in &nodes { commands.entity(n).insert(UiTargetCamera(cam)); }
}

/// Runs the HUD for every 60 Hz tick since the last frame, then rebuilds the primitive mesh.
/// The game state the HUD shows (crate::gameplay's resources).
type GameInputs<'w> = (Option<Res<'w, crate::gameplay::Persistent>>, Option<Res<'w, crate::gameplay::Session>>, Option<Res<'w, crate::gameplay::HeldWeapon>>);

#[allow(clippy::too_many_arguments)]
fn tick_and_build(
    rt: Option<ResMut<HudRuntime>>,
    ticks: Res<crate::determinism::GameTicks>,
    hook: Res<Hud2dHook>,
    scene: Res<SceneLayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    (state, session, held): GameInputs,
) {
    let Some(mut rt) = rt else { return };
    let rt = &mut *rt;
    // The game's values (bolts 0x15ed98, max HP 0x15eda0, HP 0x1415f8, the held item's ammo slot), unless the
    // RC_HUD_DEMO values drive it.
    if !rt.env.demo {
        if let Some(gs) = &state {
            rt.game.bolts = gs.0.global.bolts;
            rt.game.max_hp = gs.0.global.max_hp;
        }
        if let Some(s) = &session { rt.game.hp = s.0.hp; }
        rt.game.weapon = held.as_ref().and_then(|h| h.0);
    }
    let target = ticks.0;
    if hook.freeze || scene.hide_hud { rt.ticks_done = target; }
    // Catch up at most a second of ticks per frame.
    if target > rt.ticks_done + 60 { rt.ticks_done = target - 60; }
    while rt.ticks_done < target {
        rt.ticks_done += 1;
        let t = rt.ticks_done;
        if t == 1 {
            if let Some(text) = rt.env.text.clone().filter(|_| !rt.env.text_window) { rt.state.show_banner(&text, None); }
            if let Some(id) = rt.env.help { rt.state.help_request(id); }
        }
        if rt.env.demo {
            if t == 60 { rt.game.bolts = 1234; }
            if t == 180 { rt.game.hp = 3; }
        }
        rt.draws = rt.state.tick(rt.game);
        if let Some(text) = rt.env.text.clone().filter(|_| rt.env.text_window) { window_text_demo(&rt.state, &rt.glyphs, &text, &mut rt.draws); }
    }
    rt.hud2d.clear();
    let mut st = crate::text_render::TextState::default();
    if !hook.replace_hud && !scene.hide_hud { crate::text_render::execute(&mut rt.hud2d, &mut st, &rt.glyphs, &rt.draws); }
    rt.hud2d.prims.extend(hook.prims.iter().copied());
    rt.hud2d.prims.extend(scene.prims.iter().copied());
    let _ = meshes.insert(&rt.mesh, build_mesh(&rt.hud2d, &rt.atlas_frames, &rt.atlas_fx));
}

/// `RC_HUD_TEXT_WINDOW=1`: the text in a `DrawUIFrame` sized from a `FontPrintWindow` measure (regular font,
/// window x 44..468, anchor 256, centred block on y = 100, line height 16), as the help box sizes itself.
fn window_text_demo(state: &HudState, glyphs: &[GlyphTable; 3], text: &[u8], out: &mut Vec<Draw>) {
    use rc_formats::font::Font;
    use rc_game::hud::text;
    let font = Font::Regular;
    let mut win = text::Window::new(0, 416, 0x2c, 0x1d4, 0x100, 100, 0x10, text::CENTRE_LINES | text::CENTRE_BLOCK | text::MEASURE_ONLY);
    text::layout(&mut win, text, -1, &glyphs[font as usize], true);
    let (hw, hh) = ((win.max_width >> 1) as i32 + 10, (win.height >> 1) as i32 + 5);
    let _ = state;
    out.push(Draw::UiFrame { top: 100 - hh, bottom: 100 + hh, left: 256 - hw, right: 256 + hw, alpha: 0x60 });
    win.flags = text::CENTRE_LINES | text::CENTRE_BLOCK;
    out.push(Draw::TextWindow { font, window: win, rgba: 0x80f0_f0f0, text: text.to_vec() });
}

fn empty_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 3])
        .with_inserted_attribute(ATTRIBUTE_UV, vec![[0.0f32; 2]; 3])
        .with_inserted_attribute(ATTRIBUTE_RGBA, VertexAttributeValues::Uint32(vec![0; 3]))
        .with_inserted_attribute(ATTRIBUTE_TEX, VertexAttributeValues::Uint32x4(vec![[0; 4]; 3]))
        .with_inserted_attribute(ATTRIBUTE_SCISSOR, VertexAttributeValues::Uint32x4(vec![[1, 0, 1, 0]; 3]))
        .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
}

/// The primitives as one triangle list in submission order (the GPU blends triangles of one draw in order).
fn build_mesh(h: &Hud2d, frames: &[[u32; 4]], fx: &[Option<[u32; 4]>]) -> Mesh {
    if h.prims.is_empty() { return empty_mesh(); }
    let n = h.prims.len() * 4;
    let (mut pos, mut uv, mut rgba, mut tex, mut sc, mut idx) =
        (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n / 4 * 6));
    for p in &h.prims {
        let rect = match p.tex {
            Tex::None => None,
            Tex::Frame(i) => frames.get(i).copied(),
            Tex::Fx(i) => fx.get(i).copied().flatten(),
        };
        let t = match rect {
            Some([x, y, w, hh]) => [x | y << 16, w | hh << 16, 1, 0],
            None => [0, 0x0001_0001, 0, 0],
        };
        let s = p.scissor.map(|v| v.clamp(-1, 0xffff) as u32);
        let base = pos.len() as u32;
        for k in 0..4 {
            pos.push([p.pos[k][0] as f32, p.pos[k][1] as f32, 0.0]);
            uv.push([p.uv[k][0] as f32, p.uv[k][1] as f32]);
            rgba.push(p.rgba);
            tex.push(t);
            sc.push([s[0], s[1], s[2], s[3]]);
        }
        idx.extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(ATTRIBUTE_UV, uv)
        .with_inserted_attribute(ATTRIBUTE_RGBA, VertexAttributeValues::Uint32(rgba))
        .with_inserted_attribute(ATTRIBUTE_TEX, VertexAttributeValues::Uint32x4(tex))
        .with_inserted_attribute(ATTRIBUTE_SCISSOR, VertexAttributeValues::Uint32x4(sc))
        .with_inserted_indices(Indices::U32(idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_variants_map_uv_like_the_gs_packets() {
        let mut h = Hud2d { frame_sizes: vec![(32, 32)], ..default() };
        h.sprite(0, 10, 20, 64, 32, 0x80, Rot::None);
        h.sprite(0, 10, 20, 32, 32, 0x40, Rot::R180);
        h.sprite(0, 10, 20, 32, 32, 0x1ff, Rot::R90);
        let p = &h.prims;
        assert_eq!((p[0].rect(), p[0].rgba), ([10, 20, 64, 32], 0x807f_7f7f));
        // 180°: the top-left corner shows texel (tw, th).
        assert_eq!((p[1].pos[1], p[1].uv[1]), ([10, 20], [32, 32]));
        assert_eq!((p[1].pos[2], p[1].uv[2]), ([42, 52], [0, 0]));
        // RGBAQ keeps the low alpha byte.
        assert_eq!(p[2].rgba, 0xff7f_7f7f);
        h.rect(5, 9, 1, 3, 0x6004_0404);
        assert_eq!(h.prims[3].rect(), [1, 5, 2, 4]);
    }

    #[test]
    fn atlas_packs_without_overlap() {
        let tex = |w: u32, h: u32, v: u8| Texture { width: w, height: h, rgba: vec![v; (w * h * 4) as usize] };
        let frames = vec![tex(32, 32, 1), tex(256, 256, 2), tex(1024, 8, 3), tex(64, 16, 4)];
        let fx = vec![None, Some(tex(256, 128, 5))];
        let a = build_atlas(&frames, &fx);
        let rects: Vec<[u32; 4]> = a.frames.iter().copied().chain(a.fx.iter().flatten().copied()).collect();
        for (i, r) in rects.iter().enumerate() {
            assert!(r[0] + r[2] <= ATLAS_W && r[1] + r[3] <= a.height);
            for s in &rects[i + 1..] {
                assert!(r[0] + r[2] <= s[0] || s[0] + s[2] <= r[0] || r[1] + r[3] <= s[1] || s[1] + s[3] <= r[1], "{r:?} overlaps {s:?}");
            }
        }
        let [x, y, ..] = a.fx[1].unwrap();
        assert_eq!(a.rgba[((y * ATLAS_W + x) * 4) as usize], 5);
    }
}
