//! Level-code water: strip meshes (classes 676, 678, 761, 1225) and ripple patches (class 751), drawn in the
//! game's frame position. Spec: docs/plan/world_animation.md §1–§3; tables: `rc_formats::water`; state and
//! the exact float arithmetic: `rc_game::water`. `RC_WATER=0` disables all of it; `RC_WATER_STATS=1` prints
//! the ripple module's state once per second of ticks.
//!
//! **Frame position.** The game's water is drawn by per-frame draw callbacks that the moby updates register
//! on list 0x21afe0 (moby instance order: 676, 678, 751, 761, 1225 on Novalis); the list is drained after the
//! mobys and before the particles. Here every water draw is a Transparent3d item at the world origin with
//! `depth_bias = 5e5 + slot`, `slot` = the callback order (per class, per strip / patch, layer 1 then layer 2):
//! after every moby item (Opaque3d / AlphaMask3d run first, moby Transparent3d items have bias 0) and before
//! the particles (bias 1e6), and among themselves exactly in callback order (all share the origin distance).
//!
//! **GS state → Bevy.** Both passes: `ALPHA = FIX << 32 | 0x64` = `Cd + (Cs − Cd)·FIX/128`, TEST 0x5360b with
//! vertex A = 0 (As = 0 fails, AFAIL RGB_ONLY: colour, never Z), ZTST GEQUAL, CLAMP 0 (repeat), TEX1 bilinear
//! without mips, MODULATE, FGE. The blend is `GsPass::BlendNoZ` (SrcAlpha / OneMinusSrcAlpha, no depth write,
//! GreaterEqual) with FIX/128 **baked into the fragment alpha** (not `BlendFactor::Constant`: Bevy's material
//! draw has no per-draw blend constant). The GPU mixes in linear light, the GS on display bytes (as for every
//! blended pass, gs_state.rs).
//!
//! **Strips.** One static mesh per strip (the GS tri-strip as a `TriangleStrip`), two materials (layer 1 / 2).
//! Per rendered frame (`FUN_002b96e0` runs in the draw callback, so once per frame on which a tick ran):
//! `ScrollState::tick`, then the layer offset, the eight wobble offsets `A·(sin θ, cos θ)` for this phase
//! (VU0 sine on the CPU, `rc_game::water::wobble_table`; each vertex carries its `hash & 7` from the raw bits
//! of x and y, computed at load) and the class's z blend go to the material uniforms. The z blend: 761 writes
//! `bob(t)` in its update (per tick, before the draw); 1225 writes it in its callback after drawing (one frame
//! late); 676/678 keep the stored 0 (z = z1). The FastBSphereCheck(400) cull is not reproduced (the GPU clips).
//!
//! **Ripples.** Per tick, the 751 update: zone activation against the camera, random drops, the sim clock.
//! With the game tick (crate::gameplay) the moby scheduler runs it at 751's place in the moby order
//! ([`WaterState::ripple_update`], its load-pass init [`WaterState::ripple_init`]) on the game's one `rand`
//! stream with the game camera 0x167240; with `RC_PLAY=0` this plugin's `FixedUpdate` runs it every tick on a
//! stream of its own (`srand(1234)`) with the fly camera. Per rendered frame, per patch
//! with a mask: the four-corner frustum test (`FUN_002b91c8`; approximated with the port's projection, the
//! game tests clip codes against 0x167200), then `advance_uv` (only for drawn patches, as in the game), the
//! 17×17 vertex grid with grey and sphere-map UV from the drawn height buffer, and one mesh of the active
//! 4×4 sub-blocks (each the game's 46-vertex strip, degenerate joins dropped). Water pass (FX +0x18, FIX
//! +0x1d, the shared animated UV) then env pass (FX +0x14, FIX +0x1c, the sphere-map UV), per patch.
//! The game emits water/env per sub-block; sub-blocks do not overlap, so per patch is the same image.
//!
//! **Buffers.** The materials never change after setup (except the fog uniform, on a fog change), so Bevy
//! does not re-prepare them: the per-frame values live in two storage buffers every water material binds,
//! rewritten on each drawn frame. `frame` holds one [`FRAME_RECORD`] per callback slot (z blend w, scroll, the
//! eight wobble offsets); `ripple` one record per patch (the sub-block mask, the 17×17 positions, sphere-map
//! UVs and grey, then the 46 water UVs), which the patches' one static mesh (all 16 sub-blocks, each vertex
//! tagged with its sub-block, strip index and grid vertex) reads; sub-blocks outside the mask are dropped by the
//! vertex shader. The triangles, their order and their vertex values are those of a mesh of the active
//! sub-blocks.
//!
//! Not drawn: class 760 foam/mist (quad geometry and element timers not traced; the scroll formulas are in
//! `rc_game::water`), 1848 (env overlay), drips (787) and the hero's splashes (no hero in the water yet).

use crate::game_camera::{game_eye, GameFog, GameProjection, TfragFog};
use crate::gs_state::GsPass;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::camera::CameraProjection;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::water::{self as wf, Cuboid, RippleTables, StripAnim, StripClass, StripDescriptor, SUB_STRIP_LEN};
use rc_game::ps2v::Pf;
use rc_game::rng::{Rng, LEVEL_SEED};
use rc_game::water::{self as ww, RippleSim, ScrollState, VG};
use std::path::Path;

const SHADER_PATH: &str = "shaders/water.wgsl";
/// Transparent3d sort bias of the water draws (see the module doc).
const WATER_BIAS: f32 = 5.0e5;
/// The ripple class.
const RIPPLE_CLASS: u16 = 751;

pub const ATTRIBUTE_WATER_Z1: MeshVertexAttribute = MeshVertexAttribute::new("WaterZ1", 0x5741_5431, VertexFormat::Float32);
pub const ATTRIBUTE_WATER_UV: MeshVertexAttribute = MeshVertexAttribute::new("WaterUv", 0x5741_5432, VertexFormat::Float32x4);
pub const ATTRIBUTE_WATER_TAG: MeshVertexAttribute = MeshVertexAttribute::new("WaterTag", 0x5741_5433, VertexFormat::Uint32x2);

/// `RC_WATER=0` turns the water off.
pub fn enabled() -> bool { !std::env::var("RC_WATER").is_ok_and(|v| v.trim() == "0") }

/// Level data for the water.
#[derive(Default)]
pub struct LevelWater {
    pub strips: Vec<StripClass>,
    pub ripples: Option<RippleTables>,
    pub cuboids: Vec<Cuboid>,
    /// 751 pvar: cuboid index per zone.
    pub zone_cuboids: Vec<i32>,
    /// Directional light set 0, light A direction x / y (0x180350 / 0x180354).
    pub light_xy: [f32; 2],
    /// Water classes in moby instance order = draw-callback order.
    pub order: Vec<u16>,
}

/// Reads the overlay tables, the cuboids and the 751 pvar.
pub fn load(root: &Path, index: u32, gameplay: &[u8]) -> Result<LevelWater> {
    if !enabled() { return Ok(LevelWater::default()); }
    let ov_bytes = crate::disc_source::level_file(root, index, "overlay.bin")?;
    let ov = wf::Overlay::parse(&ov_bytes).context("parsing the level overlay")?;
    let strips = wf::parse_strip_classes(&ov, index)?;
    let ripples = wf::parse_ripple_tables(&ov, index)?;
    if strips.is_empty() && ripples.is_none() { return Ok(LevelWater::default()); }
    let cuboids = wf::parse_cuboids(gameplay)?;
    let instances = rc_formats::gameplay::parse_moby_instances(gameplay)?;
    let pvars = rc_formats::gameplay::parse_pvars(gameplay)?;
    let mut zone_cuboids = Vec::new();
    if let Some(rt) = &ripples {
        if let Some(p) = instances.iter().find(|m| m.o_class == RIPPLE_CLASS as i32).and_then(|m| m.pvar(&pvars)) {
            zone_cuboids = wf::ripple_zone_cuboids(p, rt.zones.len())?;
        }
    }
    let bank = rc_formats::tfrag_light::parse_light_bank(gameplay)?;
    let light_xy = [bank.sets[0].dir_a[0], bank.sets[0].dir_a[1]];
    let mut order = Vec::new();
    for m in &instances {
        let c = m.o_class as u16;
        let ours = strips.iter().any(|s| s.class == c) || (c == RIPPLE_CLASS && ripples.is_some());
        if ours && !order.contains(&c) { order.push(c); }
    }
    Ok(LevelWater { strips, ripples, cuboids, zone_cuboids, light_xy, order })
}

/// Static uniform of one water draw.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct WaterParams {
    /// x = FIX / 128, y = kind (0/1 strip layer, 2 ripple water, 3 ripple env).
    pub misc: Vec4,
    /// x = callback slot (record in `frame`), y = first word of the patch's `ripple` record, z = offset of its
    /// water UVs in the record (words).
    pub ids: UVec4,
}

/// Bytes per callback slot in the `frame` buffer: (w, scroll x, scroll y, 0), then the eight wobble offsets
/// two per vec4 (index = hash & 7).
const FRAME_RECORD: usize = 80;
/// Words of a patch's `ripple` record: mask + 3 pad, per grid vertex (x, y, z, env u, env v, RGBA), then the
/// 46 water UVs.
const RIPPLE_HEAD: usize = 4;
const RIPPLE_VERTEX: usize = 6;
const RIPPLE_UV_AT: usize = RIPPLE_HEAD + VG * VG * RIPPLE_VERTEX;
const RIPPLE_WORDS: usize = RIPPLE_UV_AT + SUB_STRIP_LEN * 2;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub fog: TfragFog,
    #[uniform(3)]
    pub params: WaterParams,
    /// Per-frame values of every slot (see [`FRAME_RECORD`]).
    #[storage(4, read_only, visibility(vertex))]
    pub frame: Handle<ShaderBuffer>,
    /// Per-frame ripple patch data (see [`RIPPLE_WORDS`]).
    #[storage(5, read_only, visibility(vertex))]
    pub ripple: Handle<ShaderBuffer>,
    /// Callback-order slot (Transparent3d position).
    pub slot: u32,
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { AlphaMode::Blend }
    fn depth_bias(&self) -> f32 { WATER_BIAS + self.slot as f32 }
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        GsPass::BlendNoZ.specialize(descriptor);
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_WATER_Z1.at_shader_location(1),
            ATTRIBUTE_WATER_UV.at_shader_location(2),
            ATTRIBUTE_WATER_TAG.at_shader_location(3),
        ])?];
        Ok(())
    }
}

struct StripDraw {
    desc: StripDescriptor,
    scroll: ScrollState,
    /// Callback slots of layer 1 / 2.
    slots: [u32; 2],
}

struct ClassDraw {
    class: u16,
    anim: StripAnim,
    /// The class's z blend (+0x5c of every strip; they are always written together).
    w: Pf,
    strips: Vec<StripDraw>,
}

struct PatchDraw {
    entities: [Entity; 2],
    visible: bool,
}

/// The live water state.
#[derive(Resource)]
pub struct WaterState {
    classes: Vec<ClassDraw>,
    /// Position of the ripple class in the callback order (between the strip classes).
    ripple_at: usize,
    pub ripple: Option<RippleSim>,
    patches: Vec<PatchDraw>,
    cuboids: Vec<Cuboid>,
    /// The frame counter 0x15f5cc as the updates see it.
    counter: u32,
    ticks: u64,
    drawn_ticks: u64,
    stats: bool,
    drops: u32,
    steps: u32,
    /// 751 updates run (for the stats line).
    ripple_ticks: u64,
    /// 751's init inputs (tables, zone cuboids, light x/y), for the load-pass init on the game's stream.
    ripple_inputs: Option<(RippleTables, Vec<i32>, [f32; 2])>,
    /// The stream of the `RC_PLAY=0` path (`srand(1234)`); unused once [`external`](Self::external).
    fallback_rng: Rng,
    /// The moby scheduler runs 751 (crate::gameplay): this plugin's tick leaves the ripples alone.
    pub external: bool,
    /// Every water material (for a fog change) and the fog they hold.
    materials: Vec<Handle<WaterMaterial>>,
    fog: TfragFog,
    /// The `frame` / `ripple` buffers and their CPU copies.
    frame: Handle<ShaderBuffer>,
    frame_bytes: Vec<u8>,
    ripple_buf: Handle<ShaderBuffer>,
    ripple_bytes: Vec<u8>,
}

impl WaterState {
    /// `(*moby+0x74)` of the ripple manager 751 (0x2fd0e8) as the scheduler calls it: state 0 is the load-pass
    /// init (`0x2b7a48` + the 751 init, drawing its two drops per patch from `rng`), which rebuilds the
    /// simulation; later states are the per-tick update ([`ripple_update`](Self::ripple_update)).
    pub fn ripple_init(&mut self, rng: &mut Rng) {
        let Some((t, zones, light)) = self.ripple_inputs.as_ref() else { return };
        self.ripple = Some(RippleSim::new(t, zones.clone(), *light, rng));
        self.external = true;
    }

    /// One tick of the 751 update with the game camera `cam` (0x167240), drawing from `rng`.
    pub fn ripple_update(&mut self, cam: [f32; 3], rng: &mut Rng) {
        let Some(sim) = self.ripple.as_mut() else { return };
        let info = sim.tick(cam, &self.cuboids, rng);
        self.drops += info.drops;
        self.steps += info.stepped as u32;
        self.ripple_ticks += 1;
        if self.stats && self.ripple_ticks.is_multiple_of(60) {
            let active: Vec<usize> = sim.patches.iter().enumerate().filter(|(_, p)| p.mask != 0).map(|(i, _)| i).collect();
            println!(
                "water: 751 update {} zone {} active patches {active:?} steps {} drops {} clock {:?} rng {:#010x}",
                self.ripple_ticks, info.zone, self.steps, self.drops, sim.clock, rng.state
            );
        }
    }

    /// The ripple manager exists on this level.
    pub fn has_ripples(&self) -> bool { self.ripple_inputs.is_some() }
}

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        if !enabled() { return; }
        if !crate::determinism::deterministic() && !app.world().contains_resource::<Time<Fixed>>() {
            app.insert_resource(Time::<Fixed>::from_hz(crate::determinism::TICK_HZ));
        }
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, tick)
            .add_systems(PostUpdate, draw.before(bevy::asset::AssetEventSystems));
    }
}

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

fn fx_image(images: &mut Assets<Image>, t: &rc_formats::texture::Texture) -> Handle<Image> {
    let mut img = Image::new_uninit(
        Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.data = Some(t.rgba.clone());
    // TEX1 0xff9000000260: bilinear, no mips; CLAMP 0: repeat.
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    images.add(img)
}

fn strip_mesh(d: &StripDescriptor) -> Mesh {
    let pos: Vec<[f32; 3]> = d.vertices.iter().map(|v| [v[0], v[1], v[2]]).collect();
    let z1: Vec<f32> = d.vertices.iter().map(|v| v[3]).collect();
    let uv: Vec<[f32; 4]> = d.uvs.iter().map(|u| [u[0], u[1], u[0], u[1]]).collect();
    let tag: Vec<[u32; 2]> = d.vertices.iter().map(|v| [d.rgba, ww::wobble_hash(v[0], v[1]) & 7]).collect();
    Mesh::new(PrimitiveTopology::TriangleStrip, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(ATTRIBUTE_WATER_Z1, z1)
        .with_inserted_attribute(ATTRIBUTE_WATER_UV, uv)
        .with_inserted_attribute(ATTRIBUTE_WATER_TAG, VertexAttributeValues::Uint32x2(tag))
}

fn params(fix: u8, kind: u32, slot: u32, patch: usize) -> WaterParams {
    WaterParams {
        misc: Vec4::new(fix as f32 / 128.0, kind as f32, 0.0, 0.0),
        ids: UVec4::new(slot, (patch * RIPPLE_WORDS) as u32, RIPPLE_UV_AT as u32, 0),
    }
}

/// Writes one slot's `frame` record.
fn write_frame(bytes: &mut [u8], slot: u32, w: f32, scroll: [f32; 2], wobble: &[[f32; 2]; 8]) {
    let at = slot as usize * FRAME_RECORD;
    let Some(rec) = bytes.get_mut(at..at + FRAME_RECORD) else { return };
    let head = [w, scroll[0], scroll[1], 0.0];
    for (dst, v) in rec.as_chunks_mut::<4>().0.iter_mut().zip(head.into_iter().chain(wobble.iter().flatten().copied())) { *dst = v.to_le_bytes(); }
}

/// The static mesh every ripple patch draws: the 16 sub-blocks' GS strips as triangles without the degenerate
/// joins (`strip_order`: grid offsets of the 46 strip vertices). Vertex tag = (sub-block | strip index << 8,
/// grid vertex); the other attributes are unused (the vertex shader reads the patch's `ripple` record).
fn patch_mesh(order: &[u16]) -> Mesh {
    let (mut tag, mut idx) = (Vec::new(), Vec::new());
    for i in 0..16u32 {
        let base = ((i & 3) * 4 + (i >> 2) * 4 * VG as u32) as usize;
        let first = tag.len() as u32;
        for (k, &o) in order.iter().enumerate() { tag.push([i | (k as u32) << 8, (base + o as usize) as u32]); }
        // The GS strip's triangles (k, k+1, k+2), without the degenerate joins.
        for k in 0..order.len().saturating_sub(2) {
            let (a, b, c) = (order[k], order[k + 1], order[k + 2]);
            if a == b || b == c || a == c { continue; }
            idx.extend([first + k as u32, first + k as u32 + 1, first + k as u32 + 2]);
        }
    }
    let n = tag.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; n])
        .with_inserted_attribute(ATTRIBUTE_WATER_Z1, vec![0.0f32; n])
        .with_inserted_attribute(ATTRIBUTE_WATER_UV, vec![[0.0f32; 4]; n])
        .with_inserted_attribute(ATTRIBUTE_WATER_TAG, VertexAttributeValues::Uint32x2(tag))
        .with_inserted_indices(Indices::U32(idx))
}

/// Writes patch `p`'s `ripple` record: the mask, the 17×17 vertices and the 46 water UVs.
fn write_patch(bytes: &mut [u8], p: usize, mask: u16, v: &ww::PatchVerts, water_uv: &[[f32; 2]; SUB_STRIP_LEN]) {
    let at = p * RIPPLE_WORDS * 4;
    let Some(rec) = bytes.get_mut(at..at + RIPPLE_WORDS * 4) else { return };
    let mut w = rec.as_chunks_mut::<4>().0.iter_mut();
    let mut put = |x: u32| if let Some(d) = w.next() { *d = x.to_le_bytes() };
    for x in [mask as u32, 0, 0, 0] { put(x); }
    for g in 0..VG * VG {
        let (pos, uv) = (v.pos[g], v.env_uv[g]);
        for x in [pos[0], pos[1], pos[2], uv[0], uv[1]] { put(x.to_bits()); }
        put(v.rgba[g]);
    }
    for uv in water_uv { put(uv[0].to_bits()); put(uv[1].to_bits()); }
}

fn setup(
    mut commands: Commands,
    level: Res<crate::Level>,
    fog: Option<Res<GameFog>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let lw = &level.0.water;
    if lw.order.is_empty() { return; }
    let Some(tex) = level.0.particles.textures.as_ref() else {
        eprintln!("water: no FX textures, not drawn");
        return;
    };
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    let mut fx: Vec<Option<Handle<Image>>> = vec![None; tex.fx_textures.len()];
    let mut image = |i: i32, images: &mut Assets<Image>| -> Option<Handle<Image>> {
        let i = usize::try_from(i).ok()?;
        if fx.get(i)?.is_none() { fx[i] = Some(fx_image(images, tex.fx_textures[i].as_ref()?)); }
        fx[i].clone()
    };

    let mut fallback_rng = Rng::new();
    fallback_rng.srand(LEVEL_SEED);
    let mut slot = 0u32;
    let mut classes = Vec::new();
    let mut patches = Vec::new();
    let mut ripple = None;
    let mut ripple_at = usize::MAX;
    let spawn = |commands: &mut Commands, mesh: Handle<Mesh>, mat: Handle<WaterMaterial>, vis: Visibility, name: String| {
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::IDENTITY, NoFrustumCulling, vis, Name::new(name))).id()
    };
    // The two per-frame buffers, sized for every slot / patch the setup below can create (a slot per strip
    // layer, two per patch). Contents are written once the layout is known.
    let n_slots: usize = lw.strips.iter().map(|c| c.strips.len() * 2).sum::<usize>() + lw.ripples.as_ref().map_or(0, |r| r.patches.len() * 2);
    let n_patches = lw.ripples.as_ref().map_or(0, |r| r.patches.len());
    let mut frame_bytes = vec![0u8; n_slots.max(1) * FRAME_RECORD];
    let ripple_bytes = vec![0u8; n_patches.max(1) * RIPPLE_WORDS * 4];
    let frame = buffers.add(ShaderBuffer::new(&frame_bytes, RenderAssetUsages::default()));
    let ripple_buf = buffers.add(ShaderBuffer::new(&ripple_bytes, RenderAssetUsages::default()));
    let mut all_mats = Vec::new();
    let mut new_mat = |materials: &mut Assets<WaterMaterial>, texture: Handle<Image>, params: WaterParams, slot: u32| {
        let h = materials.add(WaterMaterial { texture, fog, params, frame: frame.clone(), ripple: ripple_buf.clone(), slot });
        all_mats.push(h.clone());
        h
    };
    for &class in &lw.order {
        if class == RIPPLE_CLASS {
            let Some(rt) = lw.ripples.as_ref() else { continue };
            ripple_at = classes.len();
            // Level load: the load-time moby pass runs 751's init (`0x2fd0e8` state 0). With the game tick the
            // scheduler's load pass rebuilds it on the game's stream (WaterState::ripple_init).
            let sim = RippleSim::new(rt, lw.zone_cuboids.clone(), lw.light_xy, &mut fallback_rng);
            let mesh = meshes.add(patch_mesh(&sim.strip_order));
            for (i, p) in sim.patches.iter().enumerate() {
                let (Some(tw), Some(te)) = (image(p.rec.fx_water, &mut images), image(p.rec.fx_env, &mut images)) else {
                    patches.push(None);
                    continue;
                };
                let mw = new_mat(&mut materials, tw, params(p.rec.fix_water, 2, slot, i), slot);
                let me = new_mat(&mut materials, te, params(p.rec.fix_env, 3, slot + 1, i), slot + 1);
                slot += 2;
                let ew = spawn(&mut commands, mesh.clone(), mw, Visibility::Hidden, format!("ripple patch {i} water"));
                let ee = spawn(&mut commands, mesh.clone(), me, Visibility::Hidden, format!("ripple patch {i} env"));
                patches.push(Some(PatchDraw { entities: [ew, ee], visible: false }));
            }
            ripple = Some(sim);
            continue;
        }
        let Some(sc) = lw.strips.iter().find(|s| s.class == class) else { continue };
        let mut descs: Vec<StripDescriptor> = sc.strips.clone();
        let w = Pf::f(descs.first().map_or(0.0, |d| d.z_blend));
        if sc.anim == StripAnim::Bob761 {
            // Load pass: 761's first update runs its init.
            let mut verts: Vec<Vec<[f32; 4]>> = descs.iter().map(|d| d.vertices.clone()).collect();
            ww::init_761(&mut verts);
            for (d, v) in descs.iter_mut().zip(verts) { d.vertices = v; }
        }
        let mut strips = Vec::new();
        for (k, d) in descs.into_iter().enumerate() {
            let mesh = meshes.add(strip_mesh(&d));
            let mut slots = Vec::new();
            for layer in 0..2 {
                let Some(t) = image(d.fx[layer], &mut images) else { continue };
                let m = new_mat(&mut materials, t, params(d.fix[layer], layer as u32, slot, 0), slot);
                // Until the first draw: the class's z blend, no scroll, no wobble.
                write_frame(&mut frame_bytes, slot, w.to_f32(), [0.0; 2], &[[0.0; 2]; 8]);
                slots.push(slot);
                slot += 1;
                spawn(&mut commands, mesh.clone(), m, Visibility::Inherited, format!("water {class} strip {k} layer {}", layer + 1));
            }
            let Ok(slots) = <[u32; 2]>::try_from(slots) else { continue };
            strips.push(StripDraw { scroll: ScrollState::new(&d), desc: d, slots });
        }
        classes.push(ClassDraw { class, anim: sc.anim, w, strips });
    }
    println!(
        "water: draw order {:?}; strips {:?}; ripple patches {} (zones {:?}, cuboids {:?}); {} draws",
        lw.order,
        classes.iter().map(|c| (c.class, c.strips.len())).collect::<Vec<_>>(),
        ripple.as_ref().map_or(0, |r: &RippleSim| r.patches.iter().filter(|p| p.rec.centre != [0.0; 3]).count()),
        lw.ripples.as_ref().map(|r| r.zones.iter().map(|z| (z.first_patch, z.patch_count)).collect::<Vec<_>>()),
        lw.zone_cuboids,
        slot
    );
    let patches = patches.into_iter().map(|p| p.unwrap_or(PatchDraw { entities: [Entity::PLACEHOLDER; 2], visible: false })).collect();
    if let Some(mut b) = buffers.get_mut(&frame) { b.data = Some(frame_bytes.clone()); }
    commands.insert_resource(WaterState {
        classes,
        ripple_at,
        ripple,
        patches,
        cuboids: lw.cuboids.clone(),
        counter: 0,
        ticks: 0,
        drawn_ticks: 0,
        stats: std::env::var("RC_WATER_STATS").is_ok_and(|v| v.trim() == "1"),
        drops: 0,
        steps: 0,
        ripple_ticks: 0,
        ripple_inputs: lw.ripples.clone().map(|t| (t, lw.zone_cuboids.clone(), lw.light_xy)),
        fallback_rng,
        external: false,
        materials: all_mats,
        fog,
        frame,
        frame_bytes,
        ripple_buf,
        ripple_bytes,
    });
}

/// One 60 Hz tick: the 761 z pulse and the 751 update (zone activation, drops, clock).
fn tick(state: Option<ResMut<WaterState>>, cams: MainCamera) {
    let Some(mut st) = state else { return };
    let st = &mut *st;
    st.counter = st.counter.wrapping_add(1);
    let t = st.counter;
    for c in st.classes.iter_mut().filter(|c| c.anim == StripAnim::Bob761) { c.w = ww::bob(t); }
    let cam = cams.iter().next().map_or([0.0; 3], |t| game_eye(t).to_array());
    if !st.external {
        let mut rng = st.fallback_rng;
        st.ripple_update(cam, &mut rng);
        st.fallback_rng = rng;
    }
    st.ticks += 1;
}

/// Clip-space outcode of a game-space point (bits: 1 left, 2 right, 4 bottom, 8 top, 0x20 behind).
fn outcode(clip_from_world: &Mat4, p: [f32; 3]) -> u32 {
    let c = *clip_from_world * crate::tfrag_render::game_to_bevy(p).extend(1.0);
    let mut o = 0;
    if c.x < -c.w { o |= 1; }
    if c.x > c.w { o |= 2; }
    if c.y < -c.w { o |= 4; }
    if c.y > c.w { o |= 8; }
    if c.w <= 0.0 { o |= 0x20; }
    o
}

/// The draw callbacks of this frame, in list order (runs only on frames that followed a tick).
#[allow(clippy::too_many_arguments)]
fn draw(
    state: Option<ResMut<WaterState>>,
    cams: MainCamera,
    fog: Option<Res<GameFog>>,
    level: Res<crate::Level>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(mut st) = state else { return };
    let st = &mut *st;
    if st.ticks == st.drawn_ticks { return; }
    st.drawn_ticks = st.ticks;
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| TfragFog::new(&level.0.fog));
    // The fog uniform is the only material field that changes: written only when the frame's fog differs.
    if fog != st.fog {
        st.fog = fog;
        for h in &st.materials {
            if let Some(mut m) = materials.get_mut(h) { m.fog = fog; }
        }
    }
    let Some(cam_t) = cams.iter().next() else { return };
    let cam = game_eye(cam_t).to_array();
    let clip_from_world = GameProjection::default().get_clip_from_view() * cam_t.to_matrix().inverse();

    let n = st.classes.len();
    for ci in 0..=n {
        if ci == st.ripple_at {
            draw_ripples(st, cam, &clip_from_world, &mut buffers, &mut vis);
        }
        let Some(c) = st.classes.get_mut(ci) else { continue };
        let w = c.w;
        for s in &mut c.strips {
            s.scroll.tick(&s.desc);
            let table = ww::wobble_table(s.scroll.phase, s.desc.wobble_amp);
            for (layer, &slot) in s.slots.iter().enumerate() {
                let off = if layer == 0 { s.scroll.s1 } else { s.scroll.s2 };
                write_frame(&mut st.frame_bytes, slot, w.to_f32(), [off[0].to_f32(), off[1].to_f32()], &table);
            }
        }
        // 1225's callback writes the z blend after drawing (0x309bf8).
        if c.anim == StripAnim::BobAfterDraw1225 { c.w = ww::bob(st.counter); }
    }
    if let Some(mut b) = buffers.get_mut(&st.frame) { b.data = Some(st.frame_bytes.clone()); }
}

#[allow(clippy::too_many_arguments)]
fn draw_ripples(
    st: &mut WaterState,
    cam: [f32; 3],
    clip_from_world: &Mat4,
    buffers: &mut Assets<ShaderBuffer>,
    vis: &mut Query<&mut Visibility>,
) {
    let Some(sim) = st.ripple.as_mut() else { return };
    let mut wrote = false;
    for p in 0..sim.patches.len() {
        let Some(pd) = st.patches.get_mut(p) else { continue };
        if pd.entities[0] == Entity::PLACEHOLDER { continue; }
        let pa = &sim.patches[p];
        let mut show = pa.mask != 0;
        if show {
            let [x, y, z] = pa.centre.map(Pf::to_f32);
            let codes = [[x - 8.0, y - 8.0], [x + 8.0, y - 8.0], [x - 8.0, y + 8.0], [x + 8.0, y + 8.0]].map(|[a, b]| outcode(clip_from_world, [a, b, z]));
            let all = codes.iter().fold(0x2f, |a, &c| a & c);
            show = all == 0;
        }
        if show {
            let mask = pa.mask;
            let uv = sim.advance_uv(p);
            let verts = sim.patch_verts(p, cam);
            write_patch(&mut st.ripple_bytes, p, mask, &verts, &uv);
            wrote = true;
        }
        if show != pd.visible {
            pd.visible = show;
            for e in pd.entities {
                if let Ok(mut v) = vis.get_mut(e) { *v = if show { Visibility::Inherited } else { Visibility::Hidden }; }
            }
        }
    }
    if wrote {
        if let Some(mut b) = buffers.get_mut(&st.ripple_buf) { b.data = Some(st.ripple_bytes.clone()); }
    }
}
