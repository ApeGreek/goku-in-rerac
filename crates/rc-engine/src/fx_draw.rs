//! Draw-callback effects: the GS primitives the game sends straight from a moby's draw callback
//! (`RegisterDrawCallback` 0x21afe0, list 1, drawn after the mobys and before the particles; list 2 after the
//! particles; the registrations are `rc_game::moby_update::classes::draw_callbacks`). One material for all of them
//! ([`FxPrimMaterial`], `fx_prim.wgsl`: `FastDrawQuadReal` quads and `DrawEnvOverlayMesh` strips with an FX texture,
//! MODULATE, fog, ALPHA 0x44 / 0x48 on the frame's display bytes through crate::display_blend) and one vertex builder
//! ([`PrimBuf`]); the fire / smoke fields 760 (crate::water_render, list 2) and the callbacks here use them.
//!
//! **The nanotech glow** (class 806, `0x301c00`, list 1; [`nanotech_prims`]). The cluster registers it every tick it
//! is in view (`rc_game::moby_update::classes::pickup`). With p = the cluster's position + (0, 0, 0.5 + bob) and the
//! camera-facing basis F = unit(camera − p), R = unit(U_crate × F), U = F × R (U_crate = the crate's third rotation
//! row), it draws, TEST_1 0x53001 (Z tested, never written), bilinear, CLAMP repeat:
//! 1. **the sphere**: a 290-vertex hemisphere of radius 0.3 (table `verts`, x ≥ 0) turned to face the camera
//!    (`F·x + R·y + U·z + p`), three tri-strips (vertices 0..148, 146..255, 254..290), per-vertex ST and RGBA
//!    0x50804040 (a violet tint at alpha 0x50), FX 21 (a cloudy blue map), ALPHA from the gp words (0x44): the dark
//!    translucent ball;
//! 2. **the halo**: 32 quads of a ring (inner corners at 0.3 × the unit circle, outer at 0.3·1.125 × 1.3), FX 11 (a
//!    radial glow) along s = 0.5 from t = 0.5 inside to t = 1 outside, colour 0x802020 with alpha
//!    `trunc(bob/0.06·64 + 128)` (64..192, the bob's pulse) on the inner corners and 0 outside, ALPHA 0x48: the
//!    blue-violet glow around the ball;
//! 3. **on the crate only** (state 1): the crate glass's sheen, four quads of the crate box (±0.5, z 0.124..0.891) in
//!    the crate's frame, FX 21 at `0x107f7f7f` (alpha 0x10), ALPHA 0x44, sphere-mapped: per corner the view vector
//!    scaled to length 2, `s = x/2 + 0.5`, `t = z/2 + 0.5` (each `fmod 8`).
//!
//! The tables are the overlay's (read per level through the function's relocations, [`NanotechTables::parse`]); the
//! glowing dotted rings around the ball are not this callback: they are the orbs' type-62 trail particles.
//!
//! **The ship glass** (`0x2a70a8`, the boot's `0x2327a0`) is registered (the cutscene FX driver, list 1, with the
//! ship's joint-0 matrix) but **not drawn**: its tables (gp arrays per class 530..533: points and normals in joint-0
//! space, quads, colour 0x50807060 / 0x30807060; FX 0x15 sphere-mapped, ALPHA 0x44) put the glass of the arrival
//! ship 530 as a bowl under the hull with the port's joint-0 pose, which the PCSX2 frames do not show; the frame
//! the game uses is not settled (docs/plan/particles.md "Draw callbacks").

use crate::game_camera::{game_eye, GameFog, TfragFog};
use anyhow::Result;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use rc_formats::water as wf;
use rc_game::moby_update::classes::draw_callbacks::Callback;

const SHADER_PATH: &str = "shaders/fx_prim.wgsl";
/// Transparent3d sort bias of the list-1 callback draws: after the mobys, before the particles (1e6).
pub const LIST1_BIAS: f32 = 9.0e5;
/// The list-2 callback draws here (after the particles and the fire fields' band 2e6).
pub const LIST2_BIAS: f32 = 3.0e6;

/// Static uniform of one draw group.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct FxPrimParams {
    /// x = 1: ALPHA 0x48 (additive `Cs·As + Cd`), 0: ALPHA 0x44 (`(Cs − Cd)·As + Cd`).
    pub misc: Vec4,
}

impl FxPrimParams {
    pub fn blend(additive: bool) -> Self { FxPrimParams { misc: Vec4::new(additive as u32 as f32, 0.0, 0.0, 0.0) } }
}

/// One draw group of a callback: an FX texture, the fog, one of the two ALPHA equations, its Transparent3d order.
/// Drawn by crate::display_blend's effect pass (the entity carries `DisplayEffect`).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FxPrimMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub fog: TfragFog,
    #[uniform(3)]
    pub params: FxPrimParams,
    /// Transparent3d sort bias (the callback list's band + the draw's position in it).
    pub order: f32,
}

impl Material for FxPrimMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend }
    fn depth_bias(&self) -> f32 { self.order }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        // TEST_1 0x53001 / 0x51001 (ATST NEVER, AFAIL FB_ONLY: colour, never Z; ZTST GEQUAL).
        crate::gs_state::GsPass::BlendNoZ.specialize(descriptor);
        crate::display_blend::specialize(descriptor);
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?];
        Ok(())
    }
}

/// An FX texture (`GetEffectTex`) as an image of raw GS bytes; TEX1 0xff9000000260 (bilinear, no mips), CLAMP 0
/// (repeat).
pub fn fx_image(images: &mut Assets<Image>, t: &rc_formats::texture::Texture) -> Handle<Image> {
    let mut img = Image::new_uninit(
        Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(t.rgba.clone());
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    images.add(img)
}

/// Mesh data of one draw group being built: game-space points, (s, t), GS RGBA bytes.
#[derive(Default)]
pub struct PrimBuf {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    color: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl PrimBuf {
    fn vertex(&mut self, p: [f32; 3], st: [f32; 2], c: u32) -> u32 {
        let b = self.pos.len() as u32;
        self.pos.push(crate::tfrag_render::game_to_bevy(p).to_array());
        self.uv.push(st);
        self.color.push([(c & 0xff) as f32, (c >> 8 & 0xff) as f32, (c >> 16 & 0xff) as f32, (c >> 24) as f32].map(|x| x / 128.0));
        b
    }

    /// One `FastDrawQuadReal` quad: corners in GS strip order (triangles 0 1 2, 1 2 3). A quad whose four alphas are
    /// 0 adds nothing under either equation and is skipped.
    pub fn quad(&mut self, p: [[f32; 3]; 4], st: [[f32; 2]; 4], rgba: [u32; 4]) {
        if rgba.iter().all(|c| c >> 24 == 0) { return; }
        let b = self.pos.len() as u32;
        for k in 0..4 { self.vertex(p[k], st[k], rgba[k]); }
        self.idx.extend([b, b + 1, b + 2, b + 1, b + 2, b + 3]);
    }

    /// One tri-strip (`DrawEnvOverlayMesh`): triangle k = vertices k, k + 1, k + 2.
    pub fn strip(&mut self, v: impl IntoIterator<Item = ([f32; 3], [f32; 2], u32)>) {
        let b = self.pos.len() as u32;
        for (p, st, c) in v { self.vertex(p, st, c); }
        let n = self.pos.len() as u32 - b;
        for k in 0..n.saturating_sub(2) { self.idx.extend([b + k, b + k + 1, b + k + 2]); }
    }

    pub fn is_empty(&self) -> bool { self.idx.is_empty() }

    pub fn write(self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.color);
        mesh.insert_indices(Indices::U32(self.idx));
    }
}

/// One draw of a callback this tick: FX texture, equation and primitives.
pub struct FxGroup {
    pub fx: usize,
    pub additive: bool,
    pub prims: PrimBuf,
}

/// One slot's entity, mesh, material and whether it is shown.
type FxSlot = (Entity, Handle<Mesh>, Handle<FxPrimMaterial>, bool);

/// The persistent entities of the callback draws: slot k = the k-th draw of the list this tick (Bevy's mesh
/// allocator keeps no slab for an empty mesh, so an entity is made when its slot first has primitives).
#[derive(Default)]
pub struct FxSlots {
    slots: Vec<Option<FxSlot>>,
    fx: Vec<Option<Handle<Image>>>,
}

/// The assets the slots need.
pub struct FxAssets<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub images: &'a mut Assets<Image>,
    pub materials: &'a mut Assets<FxPrimMaterial>,
    pub fx: Option<&'a [Option<rc_formats::texture::Texture>]>,
    pub fog: TfragFog,
}

impl FxSlots {
    /// FX image `i`, made on first use.
    fn image(&mut self, a: &mut FxAssets, i: usize) -> Option<Handle<Image>> {
        let t = a.fx?.get(i)?.as_ref()?;
        if self.fx.len() <= i { self.fx.resize(i + 1, None); }
        if self.fx[i].is_none() { self.fx[i] = Some(fx_image(a.images, t)); }
        self.fx[i].clone()
    }

    /// Shows `groups` in slots 0.. (bias `band + slot`) and hides the rest.
    pub fn show(&mut self, commands: &mut Commands, vis: &mut Query<&mut Visibility>, a: &mut FxAssets, groups: Vec<FxGroup>, band: f32, name: &str) {
        let n = groups.len();
        for (k, g) in groups.into_iter().enumerate() {
            let img = self.image(a, g.fx);
            if self.slots.len() <= k { self.slots.push(None); }
            let show = !g.prims.is_empty() && img.is_some();
            if show {
                let img = img.expect("checked");
                let params = FxPrimParams::blend(g.additive);
                match &mut self.slots[k] {
                    Some((_, mesh, mat, _)) => {
                        if let Some(mut m) = a.meshes.get_mut(&*mesh) { g.prims.write(&mut m); }
                        let need = a.materials.get(&*mat).is_none_or(|m| m.texture != img || m.params.misc != params.misc || m.fog != a.fog);
                        if need {
                            if let Some(mut m) = a.materials.get_mut(&*mat) {
                                m.texture = img;
                                m.params = params;
                                m.fog = a.fog;
                            }
                        }
                    }
                    None => {
                        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                        g.prims.write(&mut m);
                        let mesh = a.meshes.add(m);
                        let mat = a.materials.add(FxPrimMaterial { texture: img, fog: a.fog, params, order: band + k as f32 });
                        let e = commands
                            .spawn((
                                Mesh3d(mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                Transform::IDENTITY,
                                NoFrustumCulling,
                                Visibility::Inherited,
                                crate::display_blend::DisplayEffect,
                                Name::new(format!("{name} draw {k}")),
                            ))
                            .id();
                        self.slots[k] = Some((e, mesh, mat, true));
                        continue;
                    }
                }
            }
            set_visible(vis, &mut self.slots[k], show);
        }
        for s in self.slots.iter_mut().skip(n) { set_visible(vis, s, false); }
    }
}

fn set_visible(vis: &mut Query<&mut Visibility>, s: &mut Option<FxSlot>, show: bool) {
    let Some((e, _, _, shown)) = s else { return };
    if *shown == show { return; }
    *shown = show;
    if let Ok(mut v) = vis.get_mut(*e) { *v = if show { Visibility::Inherited } else { Visibility::Hidden }; }
}

// ---------------------------------------------------------------------------------------------------
// Level tables

/// The draw-callback tables of the level's overlay.
#[derive(Clone, Debug, Default)]
pub struct LevelFx {
    pub nanotech: Option<NanotechTables>,
}

impl LevelFx {
    pub fn parse(ov: &wf::Overlay, level: u32) -> LevelFx {
        let nanotech = NanotechTables::parse(ov, level).unwrap_or_else(|e| {
            eprintln!("fx: nanotech glow tables: {e}");
            None
        });
        LevelFx { nanotech }
    }
}

/// `0x301c00` per level (the one function, hash-identical in the 19 overlays; `tools/ghidra/names/clusters.tsv`).
const NANOTECH_GLOW_FN: [(u32, u32); 19] = [
    (0, 0x2d_9810), (1, 0x30_1c00), (2, 0x2e_4280), (3, 0x2d_2b20), (4, 0x2d_b9a0), (5, 0x30_b820), (6, 0x2e_80c0),
    (7, 0x30_4180), (8, 0x2f_b390), (9, 0x2f_aac0), (10, 0x2c_bc00), (11, 0x30_1f58), (12, 0x2f_9a70), (13, 0x2f_c650),
    (14, 0x2f_5b70), (15, 0x2d_d048), (16, 0x2d_83c0), (17, 0x2d_b4f0), (18, 0x2e_39a0),
];
/// Its size in bytes.
const NANOTECH_GLOW_SIZE: u32 = 1464;
/// The level overlays' gp.
const GP: u32 = 0x16_6c00;

/// A data reference of a function, in instruction order: a `lui`/`addiu` address, a gp-relative load, a
/// `lui`-based load. Identical code in every overlay gives the same list with each level's own addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reloc {
    Addr(u32),
    Gp(u32),
    Mem(u32),
}

fn relocs(ov: &wf::Overlay, start: u32, size: u32) -> Result<Vec<Reloc>> {
    let mut hi = [None::<u32>; 32];
    let mut out = Vec::new();
    for a in (start..start + size).step_by(4) {
        let w = ov.u32(a)?;
        let (op, rs, rt) = (w >> 26, (w >> 21 & 31) as usize, (w >> 16 & 31) as usize);
        let simm = (w & 0xffff) as u16 as i16 as i32;
        match op {
            0x0f => hi[rt] = Some((w & 0xffff) << 16),
            0x09 => {
                if let Some(h) = hi[rs] { out.push(Reloc::Addr(h.wrapping_add_signed(simm))); }
            }
            // Loads and stores (lb .. lw, sb .. sw, lwc1, swc1, ld, sd).
            0x20 | 0x21 | 0x23 | 0x24 | 0x25 | 0x2b | 0x31 | 0x37 | 0x39 | 0x3f => {
                if rs == 28 {
                    out.push(Reloc::Gp(GP.wrapping_add_signed(simm)));
                } else if let Some(h) = hi[rs] {
                    out.push(Reloc::Mem(h.wrapping_add_signed(simm)));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The nanotech glow's tables and constants (module doc).
#[derive(Clone, Debug, PartialEq)]
pub struct NanotechTables {
    /// The hemisphere: 290 × (x, y, z) (radius 0.3, x ≥ 0 toward the camera), ST and RGBA per vertex.
    pub verts: Vec<[f32; 3]>,
    pub st: Vec<[f32; 2]>,
    pub rgba: Vec<u32>,
    /// The sphere's FX texture (gp word) and its ALPHA (A, B, C, D, FIX gp words; 0x44 on every level).
    pub sphere_fx: u32,
    pub sphere_additive: bool,
    /// The halo: 96 points (32 inner of length 1, 64 outer of length 1.3), 32 quads of 4 point indices, the scale
    /// (0.3) and the outer corners' extra factor (1.125), the colour (RGB), the alpha offset `0x161e80`.
    pub halo_pts: Vec<[f32; 3]>,
    pub halo_quads: Vec<[u16; 4]>,
    pub halo_scale: f32,
    pub halo_outer: f32,
    pub halo_rgb: u32,
    pub halo_alpha_add: i32,
    /// The bob's amplitude (0.06): the pulse is bob / this.
    pub bob_amp: f32,
    /// The crate glass: 16 points, 4 quads, its FX texture, the view vector's length for the sphere map (2).
    pub glass_pts: Vec<[f32; 3]>,
    pub glass_quads: Vec<[u16; 4]>,
    pub glass_fx: u32,
    pub glass_env: f32,
    /// A vector added to the centre (`0x161e60`, zero on Novalis).
    pub offset: [f32; 3],
}

/// The halo's FX texture (`GetEffectTex(0xb)`, a literal), the glass colour, the sphere map's wrap.
const HALO_FX: usize = 0xb;
const GLASS_RGBA: u32 = 0x107f_7f7f;
const GLASS_WRAP: f32 = 8.0;

impl NanotechTables {
    /// Reads the tables through the function's references (their order is the code's, the same on every level).
    pub fn parse(ov: &wf::Overlay, level: u32) -> Result<Option<NanotechTables>> {
        let Some(&(_, f)) = NANOTECH_GLOW_FN.iter().find(|t| t.0 == level) else { return Ok(None) };
        let r = relocs(ov, f, NANOTECH_GLOW_SIZE)?;
        if r.len() != 47 { anyhow::bail!("0x{f:x}: {} data references, expected 47", r.len()); }
        let addr = |i: usize| -> Result<u32> {
            match r[i] {
                Reloc::Addr(a) | Reloc::Gp(a) | Reloc::Mem(a) => Ok(a),
            }
        };
        let (fv, uv) = (|a: u32| -> Result<f32> { Ok(ov.f32(a)?) }, |a: u32| -> Result<u32> { Ok(ov.u32(a)?) });
        let v3 = |a: u32| -> Result<[f32; 3]> { Ok([fv(a)?, fv(a + 4)?, fv(a + 8)?]) };
        let shorts = |a: u32, n: usize| -> Result<Vec<[u16; 4]>> {
            (0..n).map(|q| -> Result<[u16; 4]> { let b = ov.read(a + 8 * q as u32, 8)?; Ok(std::array::from_fn(|k| u16::from_le_bytes([b[2 * k], b[2 * k + 1]]))) }).collect()
        };
        // Reference ordinals (level01 0x301c00): 0 gp bob amplitude, 2 gp sphere FX, 3..7 gp ALPHA B, A, C, D, FIX,
        // 8 the centre offset, 13 the vertices, 15 RGBA, 16 ST, 21 the alpha offset, 22 gp halo RGB, 23 halo quads,
        // 24 halo points, 26 / 27 gp halo scale / outer factor, 31 gp glass FX, 36 glass points, 38 glass quads, 43 gp
        // the sphere map's length.
        let (verts_a, rgba_a, st_a) = (addr(13)?, addr(15)?, addr(16)?);
        let n = 290u32;
        let verts = (0..n).map(|i| v3(verts_a + 16 * i)).collect::<Result<Vec<_>>>()?;
        let rgba = (0..n).map(|i| uv(rgba_a + 4 * i)).collect::<Result<Vec<_>>>()?;
        let st = (0..n).map(|i| -> Result<[f32; 2]> { Ok([fv(st_a + 8 * i)?, fv(st_a + 8 * i + 4)?]) }).collect::<Result<Vec<_>>>()?;
        let (a_b, a_a, a_c, a_d) = (uv(addr(3)?)?, uv(addr(4)?)?, uv(addr(5)?)?, uv(addr(6)?)?);
        let sphere_additive = match (a_a, a_b, a_c, a_d) {
            (0, 1, 0, 1) => false,
            (0, 2, 0, 1) => true,
            other => anyhow::bail!("sphere ALPHA {other:?} is neither 0x44 nor 0x48"),
        };
        let halo_quads = shorts(addr(23)?, 32)?;
        let glass_quads = shorts(addr(38)?, 4)?;
        let (hp, gp_) = (addr(24)?, addr(36)?);
        let halo_n = halo_quads.iter().flatten().map(|&i| i as u32 + 1).max().unwrap_or(0);
        let glass_n = glass_quads.iter().flatten().map(|&i| i as u32 + 1).max().unwrap_or(0);
        Ok(Some(NanotechTables {
            verts,
            st,
            rgba,
            sphere_fx: uv(addr(2)?)?,
            sphere_additive,
            halo_pts: (0..halo_n).map(|i| v3(hp + 16 * i)).collect::<Result<Vec<_>>>()?,
            halo_quads,
            halo_scale: fv(addr(26)?)?,
            halo_outer: fv(addr(27)?)?,
            halo_rgb: uv(addr(22)?)? & 0xff_ffff,
            halo_alpha_add: uv(addr(21)?)? as i32,
            bob_amp: fv(addr(0)?)?,
            glass_pts: (0..glass_n).map(|i| v3(gp_ + 16 * i)).collect::<Result<Vec<_>>>()?,
            glass_quads,
            glass_fx: uv(addr(31)?)?,
            glass_env: fv(addr(43)?)?,
            offset: v3(addr(8)?)?,
        }))
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn scale3(a: [f32; 3], s: f32) -> [f32; 3] { [a[0] * s, a[1] * s, a[2] * s] }
/// `FastVecCross(out, a, b)` 0x2212d0: b × a.
fn cross_ba(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [b[1] * a[2] - b[2] * a[1], b[2] * a[0] - b[0] * a[2], b[0] * a[1] - b[1] * a[0]] }
fn unit3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l == 0.0 { v } else { scale3(v, 1.0 / l) }
}
/// `M·(x, y, z, 1)` with rows `m[0..3]` and translation `t`.
fn xform(m: &[[f32; 3]; 3], t: [f32; 3], v: [f32; 3]) -> [f32; 3] { add3(add3(add3(scale3(m[0], v[0]), scale3(m[1], v[1])), scale3(m[2], v[2])), t) }
/// `FUN_00222170(v, m)`: v − m·trunc(v/m) (VU0 `vdiv`, `ftoi0`, `itof0`).
fn wrap(v: f32, m: f32) -> f32 { v - m * (v / m).trunc() }

/// The three draws of callback `0x301c00` for one cluster (module doc).
pub fn nanotech_prims(t: &NanotechTables, g: &rc_game::moby_update::classes::pickup::NanotechGlow, cam: [f32; 3]) -> Vec<FxGroup> {
    let p = add3([g.pos[0], g.pos[1], g.pos[2] + 0.5 + g.bob], t.offset);
    let f = unit3(sub3(cam, p));
    let r = unit3(cross_ba(f, g.crate_rows[2]));
    let u = cross_ba(r, f);
    let basis = [f, r, u];
    let mut out = Vec::with_capacity(3);
    // 1. The sphere: three strips over the turned hemisphere.
    let mut sphere = PrimBuf::default();
    for (a, b) in [(0usize, 0x94usize), (146, 146 + 0x6d), (254, 254 + 0x24)] {
        sphere.strip((a..b.min(t.verts.len())).map(|i| (xform(&basis, p, t.verts[i]), t.st[i], t.rgba[i])));
    }
    out.push(FxGroup { fx: t.sphere_fx as usize, additive: t.sphere_additive, prims: sphere });
    // 2. The halo ring, pulsing with the bob.
    let pulse = g.bob / t.bob_amp;
    let a = ((pulse * 64.0 + 128.0) as i32).wrapping_add(t.halo_alpha_add) as u32;
    let c = a << 24 | t.halo_rgb;
    let mut halo = PrimBuf::default();
    for q in &t.halo_quads {
        let pts: [[f32; 3]; 4] = std::array::from_fn(|k| {
            let s = if k < 2 { t.halo_scale } else { t.halo_scale * t.halo_outer };
            xform(&basis, p, scale3(t.halo_pts.get(q[k] as usize).copied().unwrap_or_default(), s))
        });
        halo.quad(pts, [[0.5, 0.5], [0.5, 0.5], [0.5, 1.0], [0.5, 1.0]], [c, c, 0, 0]);
    }
    out.push(FxGroup { fx: HALO_FX, additive: true, prims: halo });
    // 3. The crate glass's sheen, while on the crate.
    let mut glass = PrimBuf::default();
    if g.on_crate {
        let at = [p[0], p[1], g.pos[2]];
        for q in &t.glass_quads {
            let pts: [[f32; 3]; 4] = std::array::from_fn(|k| xform(&g.crate_rows, at, t.glass_pts.get(q[k] as usize).copied().unwrap_or_default()));
            let st = pts.map(|w| {
                let e = sub3(w, cam);
                let l = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
                let e = if l == 0.0 { e } else { scale3(e, t.glass_env / l) };
                [wrap(e[0] * 0.5 + 0.5, GLASS_WRAP), wrap(e[2] * 0.5 + 0.5, GLASS_WRAP)]
            });
            glass.quad(pts, st, [GLASS_RGBA; 4]);
        }
    }
    out.push(FxGroup { fx: t.glass_fx as usize, additive: false, prims: glass });
    out
}

// ---------------------------------------------------------------------------------------------------
// Plugin

pub struct FxDrawPlugin;

impl Plugin for FxDrawPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FxPrimMaterial>::default())
            .init_resource::<FxDraw>()
            .add_systems(PostUpdate, draw_list1.before(bevy::asset::AssetEventSystems));
    }
}

/// The list-1 callback draws' state.
#[derive(Resource, Default)]
struct FxDraw {
    slots: FxSlots,
    slots2: FxSlots,
    /// The tick counter the last draw used (redrawn once per tick, like the other callbacks).
    drawn: Option<u64>,
}

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// The callbacks of this tick that draw here (the nanotech glow), per list in registration order.
#[allow(clippy::too_many_arguments)]
fn draw_list1(
    mut commands: Commands,
    mut state: ResMut<FxDraw>,
    play: Option<Res<crate::gameplay::Play>>,
    cams: MainCamera,
    fog: Option<Res<GameFog>>,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FxPrimMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(cam_t) = cams.iter().next() else { return };
    let counter = play.as_deref().map(|p| p.game.counter);
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    if counter == state.drawn && counter.is_some() { return; }
    state.drawn = counter;
    let cam = game_eye(cam_t).to_array();
    let (mut g1, mut g2) = (Vec::new(), Vec::new());
    if let Some(p) = play.as_deref() {
        let cbs = &p.svc.draw_callbacks;
        for (list, out) in [(&cbs.list1, &mut g1), (&cbs.list2, &mut g2)] {
            for &(cb, id) in list {
                match cb {
                    Callback::NanotechGlow => {
                        let (Some(t), Some(g)) = (level.0.water.fx.nanotech.as_ref(), rc_game::moby_update::classes::pickup::nanotech_glow(&p.game.mobys, id)) else { continue };
                        out.extend(nanotech_prims(t, &g, cam));
                    }
                    // Not drawn (module doc): the glass's joint-0 frame is unverified.
                    Callback::ShipGlass => {}
                    // Drawn by crate::water_render.
                    Callback::FireField760 => {}
                }
            }
        }
    }
    let tex = level.0.particles.textures.as_ref().map(|t| t.fx_textures.as_slice());
    let mut a = FxAssets { meshes: &mut meshes, images: &mut images, materials: &mut materials, fx: tex, fog };
    let st = &mut *state;
    st.slots.show(&mut commands, &mut vis, &mut a, g1, LIST1_BIAS, "list-1 callback");
    st.slots2.show(&mut commands, &mut vis, &mut a, g2, LIST2_BIAS, "list-2 callback");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_and_quads_make_the_game_triangles() {
        let mut b = PrimBuf::default();
        b.strip((0..5).map(|i| ([i as f32, 0.0, 0.0], [0.0, 0.0], 0x8080_8080)));
        assert_eq!(b.idx, [0, 1, 2, 1, 2, 3, 2, 3, 4]);
        b.quad([[0.0; 3]; 4], [[0.0; 2]; 4], [0; 4]);
        assert_eq!(b.pos.len(), 5, "an all-transparent quad is skipped");
        b.quad([[0.0; 3]; 4], [[0.0; 2]; 4], [0x1000_0000, 0, 0, 0]);
        assert_eq!(&b.idx[9..], [5, 6, 7, 6, 7, 8]);
        assert_eq!(b.color[5], [0.0, 0.0, 0.0, 0.125]);
    }

    #[test]
    fn wrap_is_the_vu_remainder() {
        assert_eq!(wrap(9.5, 8.0), 1.5);
        assert_eq!(wrap(-0.25, 8.0), -0.25);
        assert_eq!(wrap(0.75, 8.0), 0.75);
    }
}
