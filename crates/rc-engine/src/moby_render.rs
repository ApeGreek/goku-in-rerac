//! Moby (object/character) instances: class meshes placed at their gameplay-file position and rotation,
//! skinned with the animated joint palette (crate::moby_anim) and lit per vertex on the GPU with the
//! game's moby lighting (crate::moby_light). Notes: docs/plan/moby_render_notes.md,
//! docs/plan/moby_animation.md ("In the port").
//!
//! Placement. The game draws a moby with the VU1 matrix `V · [s·r0; s·r1; s·r2; 1024·(p − cam)]`
//! (MobyProc `fun_00211808`, docs/plan/moby_skinning_lighting.md §2): packed model integers (the skinned
//! vertex position) go to world units ×1024 through the rotation rows r_i (moby+0xc0.., built from the
//! instance's Euler angles by `fun_0020def8` / VU0 28259, R = Rz·Ry·Rx), the scale s = class scale ×
//! instance scale (moby+0x2c), and the position p (moby+0x10). So world = s/1024 · R · packed + p. The
//! mesh holds the packed positions in game axes; the per-instance storage record holds that affine map
//! followed by `game_to_bevy`, and the shader applies it to the skinned, truncated position.
//!
//! Per frame (`update_moby_occlusion`), MobyProc's decisions for every placed instance, in its order:
//! the occlusion word (moby +0x36/+0x37, `LevelOcclusion::moby`, level01 0x26ab2c..0x26ab58; instances whose
//! gameplay `occlusion` word is not 0 carry bit 1023 and always pass), then crate::moby_lod: the
//! draw-distance / near / frustum sphere culls, the LOD, the distance alpha fade and the metal (shine) gate.
//! Each instance has four entity groups — high LOD, high LOD fading (TEST_1 AREF 0x08), low LOD, low LOD
//! fading — plus its metal entities; exactly one group (or none) is `Visibility::Inherited`, switched only
//! when the pick changes, so the draw count stays that of one LOD. A moby MobyProc does not draw gets no
//! animation job, so its pose is not evaluated (`AnimInstance::visible` = drawn, the game's +0x31);
//! `MobyAnimAdvance` still ticks it.
//!
//! GPU layout (see moby.wgsl, moby_metal.wgsl):
//! * one mesh per (class, LOD, texture): position (packed i16 as f32), UV, a `Uint32x4` skin word
//!   (azimuth | elevation << 8 | joint count << 16; joints; weights 10 bits each; RGBA multiplier) and a `u32`
//!   class vertex id (for the CPU-colour mode). Texture −1 packets use the reserved 8×8 0x80808080 texture
//!   (GS block 0x3ffb, docs/plan/moby_untextured.md): a 1×1 grey image, so the pixel is the vertex colour.
//!   A low LOD drawn with job joint count 0 (class byte 9 = 0) gets joint count 0 = the identity;
//! * one mesh per (class, metal texture −2 chrome / −3 glass) from the metal packets: position and skin word;
//! * one entity per (instance, part, GS pass) with `MeshTag(instance index)`, so every instance of a class
//!   part is one instanced draw; `NoFrustumCulling` because animated poses leave the bind-pose AABB;
//! * storage: `MobyInst` per gameplay instance (model matrix, light block, palette base, colour mode),
//!   the palette (`mat4x4` per joint slot, rewritten after each 60 Hz tick), the boot-ELF normal table
//!   (256 × (cos, sin)), the CPU colour table (only filled with `RC_MOBY_CPU_LIGHT=1`) and `MobyLod` per
//!   instance (vertex alpha, tint flag, shine alpha, sphere-map basis; rewritten per frame).
//!
//! Winding is not normalised and the GS does not cull, so both faces are drawn. GS state per moby is
//! ALPHA_1 0x8000000044 and TEST_1 0x5360b (MobyProc 0x1dedc0), 0x5308b while the distance fade is below
//! 0x80: a part whose As can differ from 0x80 is drawn twice with the AREF split (crate::gs_state); textures
//! repeat (CLAMP_1 = 0 on every Novalis moby ad-gif), bilinear, no mips. The metal pass: crate::moby_lod,
//! moby_metal.wgsl.

use crate::gs_state::{self, AlphaRange, GsPass};
use crate::level_load::LoadedLevel;
use crate::moby_anim::{self, AnimInstance, MobyAnim};
use crate::moby_light::{self, GpuLights, MobyLighting};
use crate::moby_lod::{self, ProcInput, ProcPick};
use crate::tfrag_render::game_to_bevy;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, VisibilitySystems};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshTag, MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology, VertexAttributeValues, VertexFormat};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;
use rc_formats::gameplay::{self, MobyInstance};
use rc_formats::level::LevelCore;
use rc_formats::moby::{self, LevelMobyClass, MobySubmesh};
use rc_formats::moby_anim::{AnimState, MobyAnimClass};
use rc_formats::moby_light::MobyLights;
use rc_formats::occlusion::OcclBits;
use rc_formats::texture::{Texture, TextureTable};
use rc_formats::tfrag_light::ps2;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

const SHADER_PATH: &str = "shaders/moby.wgsl";
const METAL_SHADER_PATH: &str = "shaders/moby_metal.wgsl";

/// Per vertex: azimuth | elevation << 8 | skin count << 16; joints j0 | j1 << 8 | j2 << 16;
/// weights w0 | w1 << 10 | w2 << 20 (/256); RGBA multiplier bytes (0x80 = 1.0).
pub const ATTRIBUTE_MOBY_SKIN: MeshVertexAttribute = MeshVertexAttribute::new("MobySkin", 0x4d4f_4231, VertexFormat::Uint32x4);
/// Per vertex: index of the (packet, vertex) in the class's high-LOD list (CPU colour table index).
pub const ATTRIBUTE_MOBY_VID: MeshVertexAttribute = MeshVertexAttribute::new("MobyVid", 0x4d4f_4232, VertexFormat::Uint32);

/// Colour modes in `MobyInst.misc.y`.
const MODE_GPU_LIGHT: u32 = 0;
const MODE_CPU_TABLE: u32 = 1;
const MODE_UNLIT: u32 = 2;

/// Texture key of texture −1 packets: the reserved 8×8 texture at GS block 0x3ffb that `init_once` fills with
/// 0x80808080 (docs/plan/moby_untextured.md).
const GREY: usize = usize::MAX;
/// Metal packet textures (TEX0 data_lo): the level's chrome and glass maps.
const TEX_CHROME: i32 = -2;
const TEX_GLASS: i32 = -3;

/// Level mobys after load: classes, animation data, instances and their light blocks.
pub struct LevelMobys {
    pub classes: Vec<LevelMobyClass>,
    /// Per class: sequences, skeleton, rest pose (crate::moby_anim).
    pub anim: Vec<MobyAnimClass>,
    pub instances: Vec<MobyInstance>,
    /// Per instance: placement, or None when the class has no geometry in this level.
    pub placed: Vec<Option<Placed>>,
    pub lighting: Option<MobyLighting>,
    /// `RC_MOBY_CPU_LIGHT=1` (or `RC_MOBY_LIGHT_CHECK=1`): distinct bit-exact bind-pose colour sets
    /// (class, per packet per vertex RGBA); `Placed::colors` indexes it.
    pub colors: Vec<(usize, Vec<Vec<[u8; 4]>>)>,
    pub cpu_light: bool,
    /// The metal pass's environment maps: chrome (128×128) and glass (64×64) PSMT8 from gs_ram at the core
    /// header's `chrome_map_*` / `glass_map_*` byte offsets (the TEX0 words 0x19e6c0 / 0x19e6d8 the level
    /// loader builds, L01 0x258128).
    pub env_maps: [Option<Texture>; 2],
    pub parse_time: Duration,
    pub light_time: Duration,
}

#[derive(Clone, Copy)]
pub struct Placed {
    pub class: usize,
    /// moby+0xc0..0xe0 as f32 (row i = image of model axis i, game space).
    pub rows: [[f32; 3]; 3],
    /// moby+0x2c = class scale × instance scale.
    pub scale: f32,
    /// The MobyProc light block (None with `RC_NO_LIGHT`).
    pub lights: Option<MobyLights>,
    /// CPU colour set (index into `LevelMobys::colors`), when computed.
    pub colors: Option<usize>,
}

fn env_on(name: &str) -> bool { std::env::var(name).is_ok_and(|v| v.trim() == "1") }

/// Chrome (128×128) and glass (64×64) maps: TEX0 TBP0 = (gs base + `*_texture`) >> 8, CBP = (gs base +
/// `*_palette`) >> 8, both byte offsets into the gs_ram lump (0x1d308000 / 0x19304000 give the sizes).
fn decode_env_maps(core: &LevelCore, gs_ram: &[u8]) -> [Option<Texture>; 2] {
    let h = &core.header;
    let one = |tex: i32, pal: i32, side: u32| -> Option<Texture> {
        let (t, p) = (usize::try_from(tex).ok()?, usize::try_from(pal).ok()?);
        let px = gs_ram.get(t..t + (side * side) as usize)?;
        let clut = gs_ram.get(p..p + 1024)?;
        rc_formats::texture::decode_indexed8(px, side, side, clut).ok()
    };
    [one(h.chrome_map_texture, h.chrome_map_palette, 128), one(h.glass_map_texture, h.glass_map_palette, 64)]
}

/// Parses the classes, their sequences and the instances, and builds every instance's light block.
pub fn load_mobys(root: &Path, core: &LevelCore, core_data: &[u8], gameplay_file: &[u8]) -> Result<LevelMobys> {
    let t0 = Instant::now();
    let classes = moby::parse_level_mobys(core, core_data).context("parsing moby classes")?;
    let anim = moby_anim::load_anim_classes(core, core_data, &classes);
    let instances = gameplay::parse_moby_instances(gameplay_file).context("parsing moby instances")?;
    let gs_ram = crate::disc_source::level_file(root, crate::level_load::level_index(), "gs_ram.bin").context("reading gs_ram for the moby env maps")?;
    let env_maps = decode_env_maps(core, &gs_ram);
    let parse_time = t0.elapsed();

    let t0 = Instant::now();
    let lighting: Option<MobyLighting> = moby_light::load(root, gameplay_file)?;
    let cpu_light = env_on("RC_MOBY_CPU_LIGHT");
    let check = env_on("RC_MOBY_LIGHT_CHECK");
    let by_class: HashMap<i32, usize> = classes.iter().enumerate().map(|(i, c)| (c.o_class, i)).collect();
    let mut colors = Vec::new();
    let mut shared: HashMap<(usize, [u32; 9], u32, [u8; 3]), usize> = HashMap::new();
    let mut check_sum = (0usize, 0i32, 0usize, 0usize);
    let placed = instances
        .iter()
        .map(|m| {
            let &ci = by_class.get(&m.o_class)?;
            let class = &classes[ci];
            let rows = moby_light::instance_rows(m);
            let lights = lighting.as_ref().map(|l| moby_light::instance_lights(l, &rows, m));
            let set = (cpu_light || check).then(|| {
                let key_rows: [u32; 9] = std::array::from_fn(|i| rows[i / 3][i % 3]);
                let key = (ci, key_rows, m.light_word(), m.ambient_rgb());
                *shared.entry(key).or_insert_with(|| {
                    let exact = moby_light::light_instance(lighting.as_ref(), class, lights.as_ref());
                    if let (true, Some(l), Some(k)) = (check, lighting.as_ref(), lights.as_ref()) {
                        let (n, max, over1, any) = moby_light::light_check(l, class, k, &exact);
                        check_sum = (check_sum.0 + n, check_sum.1.max(max), check_sum.2 + over1, check_sum.3 + any);
                    }
                    colors.push((ci, exact));
                    colors.len() - 1
                })
            });
            let scale = f32::from_bits(ps2::mul(class.class.header.scale.to_bits(), m.scale.to_bits()));
            Some(Placed { class: ci, rows: rows.map(|r| [0, 1, 2].map(|k| f32::from_bits(r[k]))), scale, lights, colors: set })
        })
        .collect();
    if check {
        println!(
            "moby light check (identity palette, {} distinct colour sets): {} vertices, max per-channel |GPU f32 replica − CPU exact| = {}, \
             {} vertices differ by > 1, {} differ at all",
            colors.len(), check_sum.0, check_sum.1, check_sum.2, check_sum.3
        );
    }
    Ok(LevelMobys { classes, anim, instances, placed, lighting, colors, cpu_light, env_maps, parse_time, light_time: t0.elapsed() })
}

/// Moby material: texture × vertex colour (GS MODULATE), tfrag fog; skinning and lighting in the vertex
/// shader from the storage buffers (shared by every moby material).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct MobyMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// The moby VU1 program writes F with the same fog lanes as the tfrag one (docs/plan/moby_render_notes.md).
    #[uniform(2)]
    pub fog: crate::game_camera::TfragFog,
    /// `MobyInst` per gameplay instance, indexed by `MeshTag`.
    #[storage(3, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    /// Joint palette, `mat4x4` per slot (crate::moby_anim).
    #[storage(4, read_only, visibility(vertex))]
    pub palette: Handle<ShaderBuffer>,
    /// Boot-ELF normal table: 256 × vec2 (cos, sin).
    #[storage(5, read_only, visibility(vertex))]
    pub normal_table: Handle<ShaderBuffer>,
    /// CPU bit-exact colours (`RC_MOBY_CPU_LIGHT=1`), packed RGBA8.
    #[storage(6, read_only, visibility(vertex))]
    pub cpu_colors: Handle<ShaderBuffer>,
    /// `MobyLod` per instance (crate::moby_lod), indexed by `MeshTag`.
    #[storage(7, read_only, visibility(vertex))]
    pub lods: Handle<ShaderBuffer>,
    /// This draw's GS state (crate::gs_state; same TEST_1 as tfrags, AREF 0x08 while fading).
    pub pass: GsPass,
}

impl From<&MobyMaterial> for GsPass {
    fn from(m: &MobyMaterial) -> Self { m.pass }
}

impl Material for MobyMaterial {
    fn vertex_shader() -> ShaderRef { SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    /// No culling: the GS draws both faces and the index stream's winding is inconsistent.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ATTRIBUTE_MOBY_SKIN.at_shader_location(2),
            ATTRIBUTE_MOBY_VID.at_shader_location(3),
        ])?];
        Ok(())
    }
}

/// Metal (shine) pass material: the chrome or glass map × the metal vertex colour, sphere-mapped in the
/// vertex shader (moby_metal.wgsl), with the moby's records, palette and `MobyLod` buffers.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(GsPass)]
pub struct MobyMetalMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[uniform(2)]
    pub fog: crate::game_camera::TfragFog,
    #[storage(3, read_only, visibility(vertex))]
    pub instances: Handle<ShaderBuffer>,
    #[storage(4, read_only, visibility(vertex))]
    pub palette: Handle<ShaderBuffer>,
    #[storage(5, read_only, visibility(vertex))]
    pub normal_table: Handle<ShaderBuffer>,
    #[storage(6, read_only, visibility(vertex))]
    pub lods: Handle<ShaderBuffer>,
    pub pass: GsPass,
}

impl From<&MobyMetalMaterial> for GsPass {
    fn from(m: &MobyMetalMaterial) -> Self { m.pass }
}

impl Material for MobyMetalMaterial {
    fn vertex_shader() -> ShaderRef { METAL_SHADER_PATH.into() }
    fn fragment_shader() -> ShaderRef { METAL_SHADER_PATH.into() }
    fn alpha_mode(&self) -> AlphaMode { self.pass.alpha_mode() }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        key.bind_group_data.specialize(descriptor);
        descriptor.vertex.buffers =
            vec![layout.0.get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0), ATTRIBUTE_MOBY_SKIN.at_shader_location(2)])?];
        Ok(())
    }
}

/// The metal pass's GS passes: its As = At·shine >> 7 spans 0..=0x80, so both halves of the TEST_1 0x5360b
/// split; the Z-writing half runs in Transparent3d (`LateTested`) so it follows the moby's own draws.
fn metal_passes(texel: AlphaRange) -> Vec<GsPass> {
    gs_state::draws(gs_state::AREF_WORLD, texel, AlphaRange { min: 0, max: 0x80 })
        .into_iter()
        .map(|p| match p { GsPass::OpaqueTested { aref } => GsPass::LateTested { aref }, p => p })
        .collect()
}

pub struct MobyRenderPlugin;

impl Plugin for MobyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin::<MobyMaterial>::default(), MaterialPlugin::<MobyMetalMaterial>::default(), moby_anim::MobyAnimPlugin))
            .add_systems(Startup, spawn_system)
            .add_systems(
                PostUpdate,
                update_moby_occlusion
                    .after(crate::occlusion::OcclusionSet)
                    .before(VisibilitySystems::VisibilityPropagate)
                    .before(bevy::asset::AssetEventSystems),
            )
            .add_systems(PostUpdate, push_metal_fog.after(crate::fog_state::FogSet))
            // After moby_attach's PostUpdate upload of the extra records (read here).
            .add_systems(Last, update_extra_metal);
    }
}

fn spawn_system(
    mut commands: Commands,
    level: Res<crate::Level>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut metal_materials: ResMut<Assets<MobyMetalMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let s = spawn_mobys(&mut commands, &level.0, &mut meshes, &mut images, &mut materials, &mut metal_materials, &mut buffers);
    let m = &level.0.mobys;
    let mode = if m.cpu_light { "CPU bit-exact colours, bind pose (RC_MOBY_CPU_LIGHT=1)" } else if m.lighting.is_none() { "unlit (RC_NO_LIGHT)" } else { "GPU skinning + lighting" };
    println!(
        "mobys: {} instances in the gameplay file, {} placed ({} without class geometry), {} classes parsed, {} used; \
         {} high-LOD triangles ({} of them texture -1, drawn with the grey 0x80 texture), {} low-LOD triangles, {} metal triangles \
         ({} chrome + {} glass), skipped {} on an unused class slot; {} meshes, {} entities ({} per LOD group high/high fading/low/low fading, \
         {} metal), <= {} instanced draws, {} images; {mode}",
        m.instances.len(), s.placed, s.no_class, m.classes.len(), s.classes_used,
        s.triangles, s.grey, s.low_triangles, s.chrome + s.glass, s.chrome, s.glass, s.untextured, s.meshes, s.entities,
        format_args!("{}/{}/{}/{}", s.group_entities[0], s.group_entities[1], s.group_entities[2], s.group_entities[3]),
        s.metal_entities, s.batches, s.images
    );
    println!(
        "mobys: parse {:.1} ms, light blocks {:.1} ms, mesh build {:.1} ms; {} animated instances ({} static: 1 sequence of <= 1 frame, {} classes without sequence 0), \
         palette {} matrices; {} instance positions outside the tfrag sphere bounds; {} vertices skinned to a joint past the class joint count; \
         env maps: chrome {}, glass {}; LOD {} (RC_MOBY_LOD), metal pass {} (RC_MOBY_METAL)",
        m.parse_time.as_secs_f64() * 1e3, m.light_time.as_secs_f64() * 1e3, s.build.as_secs_f64() * 1e3,
        s.animated, s.static_, s.no_seq0, s.palette, s.outside, s.joint_past_count,
        m.env_maps[0].is_some(), m.env_maps[1].is_some(),
        if moby_lod::lod_enabled() && !m.cpu_light { "on" } else { "off" }, if moby_lod::metal_enabled() { "on" } else { "off" }
    );
}

#[derive(Default)]
struct Stats {
    placed: usize,
    no_class: usize,
    classes_used: usize,
    triangles: usize,
    grey: usize,
    low_triangles: usize,
    chrome: usize,
    glass: usize,
    untextured: usize,
    meshes: usize,
    entities: usize,
    group_entities: [usize; 4],
    metal_entities: usize,
    batches: usize,
    images: usize,
    outside: usize,
    animated: usize,
    static_: usize,
    no_seq0: usize,
    palette: u32,
    joint_past_count: usize,
    build: Duration,
}

/// One mesh of a class: texture key (moby texture table index or [`GREY`]), mesh, range of its vertices'
/// multiplier alphas.
struct Part {
    texture: usize,
    mesh: Handle<Mesh>,
    /// The vertex alpha Af is ambient alpha (the `MobyLod` alpha) × multiplier alpha >> 7.
    mult_alpha: AlphaRange,
    triangles: usize,
}

/// A metal mesh: its texture (−2 chrome / −3 glass) and triangle count.
struct MetalPart {
    kind: i32,
    mesh: Handle<Mesh>,
    triangles: usize,
}

/// Texture key of a regular triangle: the moby texture table index, [`GREY`] for −1, None for an unused class
/// slot (0xff) or −2/−3 (metal packets only).
fn part_texture(class: &LevelMobyClass, texture: i32) -> Option<usize> {
    match texture {
        -1 => Some(GREY),
        t if t >= 0 => class.texture_table_index(t),
        _ => None,
    }
}

/// Class meshes of one packet list (high or low LOD), one per texture, plus the highest joint index any vertex
/// skins to. `identity`: the list is drawn with job joint count 0 (the low LOD of a class with class byte 9 =
/// 0): `MobyAnimEval` then only writes the identity at palette slot 0, so every vertex gets joint count 0
/// (moby.wgsl: the identity). The vertex id is the (packet, vertex) index in `lod` (the CPU colour table only
/// covers the high LOD, and `RC_MOBY_CPU_LIGHT=1` keeps every instance on it).
fn build_parts(class: &LevelMobyClass, lod: &[MobySubmesh], identity: bool, meshes: &mut Assets<Mesh>) -> (Vec<Part>, u8) {
    #[derive(Default)]
    struct B { pos: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, skin: Vec<[u32; 4]>, vid: Vec<u32>, idx: Vec<u32>, remap: HashMap<(usize, u32), u32>, alpha: Option<AlphaRange>, tris: usize }
    let mults = moby_light::vertex_multipliers(lod);
    let mut base = 0u32;
    let vid_base: Vec<u32> = lod.iter().map(|s| { let b = base; base += s.vertices.len() as u32; b }).collect();
    let mut max_joint = 0u8;
    let mut by_tex: BTreeMap<usize, B> = BTreeMap::new();
    for (si, sub) in lod.iter().enumerate() {
        for t in &sub.triangles {
            let Some(tex) = part_texture(class, t.texture) else { continue };
            let b = by_tex.entry(tex).or_default();
            b.tris += 1;
            for vi in [t.a, t.b, t.c] {
                let v = *b.remap.entry((si, vi)).or_insert_with(|| {
                    let vx = &sub.vertices[vi as usize];
                    b.pos.push(vx.packed_position().map(|c| c as f32));
                    // ST 4.12 (the VU converts with itof12; q = 1.0 from STCOL), GS-normalised coordinates.
                    b.uv.push([vx.st[0] as f32 / 4096.0, vx.st[1] as f32 / 4096.0]);
                    let sk = vx.skin;
                    let n = sk.count.max(1) as usize;
                    if !identity { max_joint = max_joint.max(*sk.joints[..n].iter().max().unwrap()); }
                    let m = mults[si][vi as usize];
                    b.alpha = Some(b.alpha.map_or(AlphaRange::of([m[3]]), |r| r.union(AlphaRange::of([m[3]]))));
                    let count = if identity { 0 } else { sk.count as u32 };
                    b.skin.push([
                        vx.normal_azimuth as u32 | (vx.normal_elevation as u32) << 8 | count << 16,
                        sk.joints[0] as u32 | (sk.joints[1] as u32) << 8 | (sk.joints[2] as u32) << 16,
                        sk.weights[0] as u32 | (sk.weights[1] as u32) << 10 | (sk.weights[2] as u32) << 20,
                        u32::from_le_bytes(m),
                    ]);
                    b.vid.push(vid_base[si] + vi);
                    (b.pos.len() - 1) as u32
                });
                b.idx.push(v);
            }
        }
    }
    let parts = by_tex
        .into_iter()
        .map(|(texture, b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uv)
                .with_inserted_attribute(ATTRIBUTE_MOBY_SKIN, VertexAttributeValues::Uint32x4(b.skin))
                .with_inserted_attribute(ATTRIBUTE_MOBY_VID, VertexAttributeValues::Uint32(b.vid))
                .with_inserted_indices(Indices::U32(b.idx));
            Part { texture, mesh: meshes.add(mesh), mult_alpha: b.alpha.unwrap_or(AlphaRange::OPAQUE), triangles: b.tris }
        })
        .collect();
    (parts, max_joint)
}

/// The class's metal packets as one mesh per texture (−2 / −3), plus the highest joint they skin to. Metal
/// vertices carry their skin inline (up to 3 palette joints, weights /256) and no ST.
fn build_metal_parts(class: &LevelMobyClass, meshes: &mut Assets<Mesh>) -> (Vec<MetalPart>, u8) {
    #[derive(Default)]
    struct B { pos: Vec<[f32; 3]>, skin: Vec<[u32; 4]>, idx: Vec<u32>, remap: HashMap<(usize, u32), u32>, tris: usize }
    let mut by_kind: BTreeMap<i32, B> = BTreeMap::new();
    let mut max_joint = 0u8;
    for (si, sub) in class.class.metal.iter().enumerate() {
        for t in &sub.triangles {
            if t.texture != TEX_CHROME && t.texture != TEX_GLASS { continue; }
            let b = by_kind.entry(t.texture).or_default();
            b.tris += 1;
            for vi in [t.a, t.b, t.c] {
                let v = *b.remap.entry((si, vi)).or_insert_with(|| {
                    let vx = &sub.vertices[vi as usize];
                    b.pos.push(vx.packed_position().map(|c| c as f32));
                    let sk = vx.skin;
                    max_joint = max_joint.max(*sk.joints[..sk.count.max(1) as usize].iter().max().unwrap());
                    b.skin.push([
                        vx.normal_azimuth as u32 | (vx.normal_elevation as u32) << 8 | (sk.count as u32) << 16,
                        sk.joints[0] as u32 | (sk.joints[1] as u32) << 8 | (sk.joints[2] as u32) << 16,
                        sk.weights[0] as u32 | (sk.weights[1] as u32) << 10 | (sk.weights[2] as u32) << 20,
                        0,
                    ]);
                    (b.pos.len() - 1) as u32
                });
                b.idx.push(v);
            }
        }
    }
    let parts = by_kind
        .into_iter()
        .map(|(kind, b)| {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.pos)
                .with_inserted_attribute(ATTRIBUTE_MOBY_SKIN, VertexAttributeValues::Uint32x4(b.skin))
                .with_inserted_indices(Indices::U32(b.idx));
            MetalPart { kind, mesh: meshes.add(mesh), triangles: b.tris }
        })
        .collect();
    (parts, max_joint)
}

/// `game_to_bevy` as a matrix (columns = images of game x, y, z).
fn axes() -> Mat3 { Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y) }

/// `MobyInst` (moby.wgsl): model (16 f32), light rows (3 vec4), light colours (3 vec4), −|K|, ambient,
/// misc (u32: palette base, colour mode, CPU colour base, 0). 208 bytes.
fn write_record(out: &mut Vec<u8>, model: &Mat4, g: &GpuLights, misc: [u32; 4]) {
    for v in model.to_cols_array() { out.extend_from_slice(&v.to_le_bytes()); }
    for r in g.rows.iter().chain(&g.colors).chain([&g.neg_k, &g.ambient]) {
        for v in r { out.extend_from_slice(&v.to_le_bytes()); }
    }
    for v in misc { out.extend_from_slice(&v.to_le_bytes()); }
}
const RECORD_SIZE: usize = 208;

/// A moby texture as an image (repeat, bilinear), with the raw GS alpha like tfrag_render; [`GREY`] is the
/// reserved 0x80808080 texture (clamp, as CLAMP_1 = 5 for −1 blocks; the texture is uniform anyway).
fn moby_image(level: &LoadedLevel, tex: usize, images: &mut Assets<Image>) -> (Handle<Image>, AlphaRange) {
    let (w, h, mut rgba, mode) = if tex == GREY {
        (1, 1, vec![0x80, 0x80, 0x80, 0xff], ImageAddressMode::ClampToEdge)
    } else {
        match level.textures.iter().find(|t| t.table == TextureTable::Moby && t.index == tex).map(|t| &t.texture) {
            Some(t) => (t.width, t.height, t.rgba.clone(), ImageAddressMode::Repeat),
            None => {
                warn!("moby texture {tex} has no decoded texture; drawing magenta");
                (1, 1, vec![255, 0, 255, 0xff], ImageAddressMode::Repeat)
            }
        }
    };
    (images.add(gs_image(w, h, &mut rgba, mode)), AlphaRange::of(rgba.iter().skip(3).step_by(4).copied()))
}

/// `texture::scale_alpha` maps a < 0x80 to 2a and a >= 0x80 to 0xff; undo it (exact for a <= 0x80) and make
/// a bilinear, unmipped `Rgba8Unorm` image.
fn gs_image(w: u32, h: u32, rgba: &mut [u8], mode: ImageAddressMode) -> Image {
    for a in rgba.iter_mut().skip(3).step_by(4) {
        *a = if *a == 0xff { 0x80 } else { *a / 2 };
    }
    let mut img = Image::new(Extent3d { width: w, height: h, depth_or_array_layers: 1 }, TextureDimension::D2, rgba.to_vec(), TextureFormat::Rgba8Unorm, RenderAssetUsages::RENDER_WORLD);
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

/// The chrome (−2) or glass (−3) map: TEX1 MXL 0 / MMAG linear, CLAMP_1 = 5 (`fun_00202d78`). A level whose
/// map does not decode gets a magenta 1×1.
fn metal_image(level: &LoadedLevel, kind: i32, images: &mut Assets<Image>) -> (Handle<Image>, AlphaRange) {
    let (w, h, mut rgba) = match &level.mobys.env_maps[if kind == TEX_CHROME { 0 } else { 1 }] {
        Some(t) => (t.width, t.height, t.rgba.clone()),
        None => {
            warn!("moby metal texture {kind}: no env map decoded; drawing magenta");
            (1, 1, vec![255, 0, 255, 0xff])
        }
    };
    let img = gs_image(w, h, &mut rgba, ImageAddressMode::ClampToEdge);
    (images.add(img), AlphaRange::of(rgba.iter().skip(3).step_by(4).copied()))
}

/// Per class: (high parts, highest high joint), low parts, (metal parts, highest metal joint).
type ClassDraws = ((Vec<Part>, u8), Vec<Part>, (Vec<MetalPart>, u8));

/// Builds the class meshes and storage buffers and spawns the entities of every instance (module doc).
fn spawn_mobys(
    commands: &mut Commands,
    level: &LoadedLevel,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    metal_materials: &mut Assets<MobyMetalMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Stats {
    let t0 = Instant::now();
    let m = &level.mobys;
    let mut st = Stats::default();
    let lod_on = moby_lod::lod_enabled() && !m.cpu_light;

    // Class meshes, only for classes something instances, in class order (asset ids follow it).
    let used: BTreeSet<usize> = m.placed.iter().flatten().map(|p| p.class).collect();
    st.classes_used = used.len();
    let mut parts: HashMap<usize, ClassDraws> = HashMap::new();
    for &ci in &used {
        let c = &m.classes[ci];
        let high = build_parts(c, &c.class.high_lod, false, meshes);
        let low = if lod_on && !c.class.low_lod.is_empty() { build_parts(c, &c.class.low_lod, c.class.header.low_lod_joint_count == 0, meshes).0 } else { Vec::new() };
        let metal = build_metal_parts(c, meshes);
        st.meshes += high.0.len() + low.len() + metal.0.len();
        parts.insert(ci, (high, low, metal));
    }

    // CPU colour table: every colour set flattened (packet by packet), when computed.
    let mut cpu_bytes = Vec::new();
    let set_base: Vec<u32> = m
        .colors
        .iter()
        .map(|(_, c)| {
            let b = (cpu_bytes.len() / 4) as u32;
            for rgba in c.iter().flatten() { cpu_bytes.extend_from_slice(rgba); }
            b
        })
        .collect();
    if cpu_bytes.is_empty() { cpu_bytes = vec![0; 16]; }

    // Instance records (index = gameplay instance index = MeshTag), palette slots and animation states.
    let animate = moby_anim::anim_enabled() && !m.cpu_light;
    let a = axes();
    let mut rec = Vec::with_capacity(m.instances.len() * RECORD_SIZE);
    let mut anims = Vec::new();
    let mut anim_index = vec![None; m.instances.len()];
    let mut palette_len = 0u32;
    for (ii, (inst, p)) in m.instances.iter().zip(&m.placed).enumerate() {
        let Some(p) = p else { write_record(&mut rec, &Mat4::ZERO, &GpuLights::default(), [0; 4]); continue };
        let r = Mat3::from_cols(Vec3::from(p.rows[0]), Vec3::from(p.rows[1]), Vec3::from(p.rows[2]));
        let model = Mat4::from_mat3_translation(a * r * (p.scale / 1024.0), game_to_bevy(inst.position));
        let ac = &m.anim[p.class];
        let (_, _, (_, metal_joint)) = &parts[&p.class];
        let max_joint = parts[&p.class].0 .1.max(*metal_joint);
        let slots = (ac.joint_count as u32).max(max_joint as u32 + 1).max(1);
        if (max_joint as usize) >= ac.joint_count.max(1) { st.joint_past_count += 1; }
        let (mode, cpu_base) = match (m.cpu_light, p.colors, &p.lights) {
            (true, Some(c), _) => (MODE_CPU_TABLE, set_base[c]),
            (_, _, None) => (MODE_UNLIT, 0),
            _ => (MODE_GPU_LIGHT, 0),
        };
        let g = p.lights.as_ref().map(GpuLights::new).unwrap_or_default();
        write_record(&mut rec, &model, &g, [palette_len, mode, cpu_base, 0]);
        let state = AnimState::spawn(ac);
        if ac.sequence(0).is_none() && ac.joint_count > 0 { st.no_seq0 += 1; }
        if state.speed == 0.0 { st.static_ += 1; } else { st.animated += 1; }
        anim_index[ii] = Some(anims.len());
        anims.push(AnimInstance { class: p.class, base: palette_len, slots, state, visible: true });
        palette_len += slots;
    }
    st.palette = palette_len;
    // MAIN_WORLD too: the scheduler-driven instances' records are rewritten per tick (MobyOcclusion::drive).
    let inst_buffer = buffers.add(ShaderBuffer::new(&rec, RenderAssetUsages::default()));
    let palette = buffers.add(ShaderBuffer::new(&moby_anim::identity_palette(palette_len), RenderAssetUsages::default()));
    let table_bytes: Vec<u8> = match &m.lighting {
        Some(l) => l.table.0.iter().flatten().flat_map(|w| w.to_le_bytes()).collect(),
        None => vec![0; 256 * 8],
    };
    let normal_table = buffers.add(ShaderBuffer::new(&table_bytes, RenderAssetUsages::RENDER_WORLD));
    let cpu_colors = buffers.add(ShaderBuffer::new(&cpu_bytes, RenderAssetUsages::RENDER_WORLD));
    // MAIN_WORLD too: rewritten per frame at the same size (update_moby_occlusion).
    let lod_bytes = moby_lod::default_lod_records(m.instances.len());
    let lods = buffers.add(ShaderBuffer::new(&lod_bytes, RenderAssetUsages::default()));
    if m.cpu_light && moby_anim::anim_enabled() { println!("mobys: RC_MOBY_CPU_LIGHT=1 draws the bind pose (the CPU colours are for the identity palette)"); }
    commands.insert_resource(MobyAnim::new(animate, anims, palette.clone(), palette_len));

    let fog = crate::game_camera::TfragFog::new(&level.fog);
    let mut mats: HashMap<(usize, GsPass), Handle<MobyMaterial>> = HashMap::new();
    let mut metal_mats: HashMap<(i32, GsPass), Handle<MobyMetalMaterial>> = HashMap::new();
    let mut imgs: HashMap<usize, (Handle<Image>, AlphaRange)> = HashMap::new();
    let mut metal_imgs: HashMap<i32, (Handle<Image>, AlphaRange)> = HashMap::new();

    // Sanity bounds: the tfrag bounding spheres (integer units, /1024), padded by 50 units.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in &level.tfrags {
        let s = t.header.bsphere.map(|v| v / 1024.0);
        lo = lo.min(Vec3::new(s[0], s[1], s[2]) - s[3]);
        hi = hi.max(Vec3::new(s[0], s[1], s[2]) + s[3]);
    }
    let (lo, hi) = (lo - 50.0, hi + 50.0);

    let mut batches = HashSet::new();
    let mut draws: Vec<InstanceDraws> = Vec::with_capacity(m.instances.len());
    for (ii, (inst, p)) in m.instances.iter().zip(&m.placed).enumerate() {
        let Some(p) = p else { st.no_class += 1; draws.push(InstanceDraws::default()); continue };
        st.placed += 1;
        let class = &m.classes[p.class];
        let pos = Vec3::from(inst.position);
        if pos.cmplt(lo).any() || pos.cmpgt(hi).any() { st.outside += 1; }
        let r = Mat3::from_cols(Vec3::from(p.rows[0]), Vec3::from(p.rows[1]), Vec3::from(p.rows[2]));
        let transform = Transform::from_matrix(Mat4::from_mat3_translation(a * r * (p.scale / 1024.0), game_to_bevy(inst.position)));
        let ((high, _), low, (metal, _)) = &parts[&p.class];
        let mut d = InstanceDraws {
            input: Some(ProcInput {
                position: inst.position,
                rows: p.rows,
                scale: p.scale,
                draw_distance: inst.draw_distance as i16 as i32,
                lod_trans: class.class.header.lod_trans,
                shine_distance: if class.class.header.metal_count > 0 { moby_lod::SHINE_DISTANCE } else { 0 },
                alpha: 0x80,
            }),
            class: p.class,
            class_sphere: class.class.header.bsphere,
            ..default()
        };
        // Groups: 0 high, 1 high fading, 2 low, 3 low fading.
        for (g, list) in [(0usize, high), (1, high), (2, low), (3, low)] {
            if g >= 1 && !lod_on { break; }
            for part in list.iter() {
                if g == 0 { st.triangles += part.triangles; if part.texture == GREY { st.grey += part.triangles; } }
                if g == 2 { st.low_triangles += part.triangles; }
                let (image, texel_alpha) = imgs.entry(part.texture).or_insert_with(|| moby_image(level, part.texture, images)).clone();
                let passes = if g % 2 == 0 {
                    gs_state::draws(gs_state::AREF_WORLD, texel_alpha, part.mult_alpha)
                } else {
                    // Vertex alpha (fade·moby+0x23 >> 7 = 0..0x7f) × multiplier >> 7.
                    let fade = AlphaRange { min: 0, max: ((0x7f * part.mult_alpha.max as u32) >> 7) as u8 };
                    gs_state::draws(moby_lod::AREF_FADE, texel_alpha, fade)
                };
                for pass in passes {
                    let mat = mats
                        .entry((part.texture, pass))
                        .or_insert_with(|| {
                            materials.add(MobyMaterial {
                                texture: image.clone(),
                                fog,
                                instances: inst_buffer.clone(),
                                palette: palette.clone(),
                                normal_table: normal_table.clone(),
                                cpu_colors: cpu_colors.clone(),
                                lods: lods.clone(),
                                pass,
                            })
                        })
                        .clone();
                    batches.insert((part.mesh.id(), mat.id().untyped()));
                    let e = commands.spawn((
                        Mesh3d(part.mesh.clone()),
                        MeshMaterial3d(mat),
                        transform,
                        MeshTag(ii as u32),
                        NoFrustumCulling,
                        if g == 0 { Visibility::Inherited } else { Visibility::Hidden },
                        Name::new(format!("moby {ii} class {} group {g} tex {} {pass:?}", inst.o_class, part.texture as isize)),
                    ));
                    d.groups[g].push(e.id());
                    st.group_entities[g] += 1;
                    st.entities += 1;
                }
            }
        }
        for part in metal.iter() {
            if part.kind == TEX_CHROME { st.chrome += part.triangles } else { st.glass += part.triangles }
            let (image, texel_alpha) = metal_imgs.entry(part.kind).or_insert_with(|| metal_image(level, part.kind, images)).clone();
            for pass in metal_passes(texel_alpha) {
                let mat = metal_mats
                    .entry((part.kind, pass))
                    .or_insert_with(|| {
                        metal_materials.add(MobyMetalMaterial {
                            texture: image.clone(),
                            fog,
                            instances: inst_buffer.clone(),
                            palette: palette.clone(),
                            normal_table: normal_table.clone(),
                            lods: lods.clone(),
                            pass,
                        })
                    })
                    .clone();
                batches.insert((part.mesh.id(), mat.id().untyped()));
                let e = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(mat),
                    transform,
                    MeshTag(ii as u32),
                    NoFrustumCulling,
                    Visibility::Hidden,
                    Name::new(format!("moby {ii} class {} metal {} {pass:?}", inst.o_class, part.kind)),
                ));
                d.metal.push(e.id());
                st.metal_entities += 1;
                st.entities += 1;
            }
        }
        // Triangles on an unused class slot (0xff): not drawn (none on the disc, docs/plan/moby_untextured.md).
        for sub in &class.class.high_lod {
            st.untextured += sub.triangles.iter().filter(|t| t.texture >= 0 && class.texture_table_index(t.texture).is_none()).count();
        }
        draws.push(d);
    }
    let shown = draws.iter().map(|d| d.input.map(|_| Shown { group: 0, metal: false })).collect();
    let n_inst = m.instances.len();
    commands.insert_resource(MobyOcclusion {
        records: rec,
        instances: inst_buffer.clone(),
        driven_hidden: vec![false; n_inst],
        records_dirty: false,
        bits: level.occlusion.objects.moby.clone(),
        anim_index,
        draws,
        shown,
        lods,
        last_lod: lod_bytes,
        lod_on,
        tint: moby_lod::tint_enabled(),
        metal_on: moby_lod::metal_enabled(),
        last_hist: None,
        last_print: f32::MIN,
    });
    st.batches = batches.len();
    st.images = imgs.len() + metal_imgs.len();
    st.build = t0.elapsed();
    st
}

/// One placed instance's entities and MobyProc inputs.
#[derive(Default)]
struct InstanceDraws {
    /// None: the class has no geometry (nothing spawned).
    input: Option<ProcInput>,
    class: usize,
    /// Class header 0x30 (packed units): the sphere when a sequence is missing.
    class_sphere: [f32; 4],
    /// High, high fading, low, low fading.
    groups: [Vec<Entity>; 4],
    metal: Vec<Entity>,
}

/// What an instance shows: its entity group and whether the metal pass is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    group: u8,
    metal: bool,
}

/// Per static gameplay instance: its occlusion word, its entities, its animation slot and what it shows; the
/// `MobyLod` buffer and the frame's pick statistics.
#[derive(Resource)]
pub struct MobyOcclusion {
    /// The `MobyInst` records (CPU copy of `instances`) and a pending rewrite (scheduler-driven instances).
    records: Vec<u8>,
    instances: Handle<ShaderBuffer>,
    /// Per instance: hidden by the moby loop (deleted, or mode & 1): MobyProc skips it.
    driven_hidden: Vec<bool>,
    records_dirty: bool,
    bits: Vec<OcclBits>,
    anim_index: Vec<Option<usize>>,
    draws: Vec<InstanceDraws>,
    shown: Vec<Option<Shown>>,
    lods: Handle<ShaderBuffer>,
    last_lod: Vec<u8>,
    lod_on: bool,
    tint: bool,
    metal_on: bool,
    last_hist: Option<[usize; 9]>,
    last_print: f32,
}

/// `MobyProc` for every placed instance (module doc): the occlusion test (after the "dead" check, which no
/// static instance fails in the port), then crate::moby_lod. Changes `Visibility` only when an instance's
/// pick changes, and rewrites the `MobyLod` buffer when it changes.
#[allow(clippy::too_many_arguments)]
pub fn update_moby_occlusion(
    state: Option<ResMut<MobyOcclusion>>,
    occl: Option<ResMut<crate::occlusion::OcclusionFrame>>,
    mut anim: Option<ResMut<MobyAnim>>,
    level: Res<crate::Level>,
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut vis: Query<&mut Visibility>,
    time: Res<Time<Real>>,
) {
    let Some(mut state) = state else { return };
    let mask = occl.as_ref().map(|o| o.mask);
    let cam = cams.iter().next().map(|t| {
        let [fwd, left, up] = crate::game_camera::game_rows(t);
        (crate::game_camera::game_eye(t), moby_lod::camera_rows(fwd, left, up))
    });
    let classes = &level.0.mobys.anim;
    let s = &mut *state;
    let mut lod_bytes = vec![0u8; s.last_lod.len()];
    // hist: occluded, culled (distance, near, frustum), high, high fading, low, low fading, metal.
    let mut hist = [0usize; 9];
    for (ii, d) in s.draws.iter().enumerate() {
        let rec = &mut lod_bytes[ii * moby_lod::LOD_RECORD_SIZE..(ii + 1) * moby_lod::LOD_RECORD_SIZE];
        let Some(inp) = d.input else { moby_lod::write_lod_record(rec, 0x80, 0, 0, &[[0.0; 3]; 3]); continue };
        let k = s.anim_index[ii];
        let (want, pick, sphere) = if s.driven_hidden[ii] {
            (None, None, None)
        } else if mask.as_ref().is_some_and(|m| !s.bits[ii].visible(m)) {
            hist[0] += 1;
            (None, None, None)
        } else if let Some((eye, rows)) = cam {
            let seq = match k.zip(anim.as_ref()) {
                Some((k, a)) => moby_lod::seq_sphere(&classes[d.class], &a.instances[k].state, d.class_sphere),
                None => d.class_sphere,
            };
            let sphere = moby_lod::world_sphere(&inp, seq);
            let v = moby_lod::view_centre(sphere, eye, &rows);
            let result = if s.lod_on {
                moby_lod::moby_proc(v, sphere[3], &inp)
            } else {
                Ok(ProcPick { low_lod: false, alpha: 0x80, fading: false, shine: moby_lod::shine_alpha(moby_lod::sphere_depth(v, sphere[3]), inp.shine_distance) })
            };
            match result {
                Err(c) => {
                    hist[match c { moby_lod::Cull::DrawDistance => 1, moby_lod::Cull::Near => 2, moby_lod::Cull::Frustum => 3 }] += 1;
                    (None, None, None)
                }
                Ok(p) => {
                    let group = (p.low_lod as u8) * 2 + p.fading as u8;
                    let hidden = k.zip(anim.as_ref()).is_some_and(|(k, a)| a.hidden[k]);
                    let metal = s.metal_on && p.shine > 0 && !d.metal.is_empty() && !hidden;
                    hist[4 + group as usize] += 1;
                    if metal { hist[8] += 1; }
                    (Some(Shown { group, metal }), Some(p), Some((sphere, eye, rows)))
                }
            }
        } else {
            (Some(Shown { group: 0, metal: false }), None, None)
        };
        let (alpha, flags, shine, e) = match (pick, want) {
            (Some(p), Some(w)) => {
                let e = match (w.metal, sphere) {
                    (true, Some((sphere, eye, rows))) => moby_lod::shine_basis(sphere, eye, &rows, &inp.rows),
                    _ => [[0.0; 3]; 3],
                };
                (p.alpha, if s.tint && p.low_lod { moby_lod::FLAG_TINT } else { 0 }, if w.metal { p.shine } else { 0 }, e)
            }
            _ => (0x80, 0, 0, [[0.0; 3]; 3]),
        };
        moby_lod::write_lod_record(rec, alpha, flags, shine, &e);

        let was = s.shown[ii];
        if want == was { continue; }
        s.shown[ii] = want;
        let set = |ents: &[Entity], on: bool, vis: &mut Query<&mut Visibility>| {
            for &e in ents {
                if let Ok(mut v) = vis.get_mut(e) { *v = if on { Visibility::Inherited } else { Visibility::Hidden }; }
            }
        };
        for (g, ents) in d.groups.iter().enumerate() {
            let (on, before) = (want.is_some_and(|w| w.group as usize == g), was.is_some_and(|w| w.group as usize == g));
            if on != before { set(ents, on, &mut vis); }
        }
        let (on, before) = (want.is_some_and(|w| w.metal), was.is_some_and(|w| w.metal));
        if on != before { set(&d.metal, on, &mut vis); }
        if want.is_some() != was.is_some() {
            if let (Some(a), Some(k)) = (anim.as_mut(), k) {
                a.instances[k].visible = want.is_some();
                if want.is_some() { a.pose_dirty = true; }
            }
        }
    }
    if lod_bytes != s.last_lod {
        if let Some(mut buf) = buffers.get_mut(&s.lods) { buf.data = Some(lod_bytes.clone()); }
        s.last_lod = lod_bytes;
    }
    if std::mem::take(&mut s.records_dirty) {
        if let Some(mut buf) = buffers.get_mut(&s.instances) { buf.data = Some(s.records.clone()); }
    }
    if let Some(mut o) = occl {
        o.mobys = crate::occlusion::CullCounts { occluded: hist[0], culled: hist[1] + hist[2] + hist[3], drawn: hist[4..8].iter().sum() };
    }
    let now = time.elapsed_secs();
    if s.last_hist != Some(hist) && now - s.last_print >= 1.0 && cam.is_some() {
        s.last_print = now;
        s.last_hist = Some(hist);
        println!(
            "moby LODs: occluded {}, culled {} (draw distance {}, near {}, frustum {}), high {} (+{} fading), low {} (+{} fading), metal pass {}",
            hist[0], hist[1] + hist[2] + hist[3], hist[1], hist[2], hist[3], hist[4], hist[5], hist[6], hist[7], hist[8]
        );
    }
}

/// Keeps the metal materials' fog uniform equal to the frame's (crate::fog_state pushes the other materials).
fn push_metal_fog(fog: Option<Res<crate::game_camera::GameFog>>, mut materials: ResMut<Assets<MobyMetalMaterial>>) {
    let Some(fog) = fog else { return };
    let want = fog.uniform;
    let stale: Vec<AssetId<MobyMetalMaterial>> = materials.iter().filter(|(_, m)| m.fog != want).map(|(id, _)| id).collect();
    for id in stale {
        if let Some(mut m) = materials.get_mut(id) { m.fog = want; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type P = [f64; 3];
    fn sub(a: P, b: P) -> P { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
    fn cross(a: P, b: P) -> P { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
    fn dot(a: P, b: P) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

    /// Two 2D triangles share interior area (touching edges or corners do not count): no separating axis
    /// among the six edge normals.
    fn overlap_2d(a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]) -> bool {
        for t in [a, b] {
            for i in 0..3 {
                let (p, q) = (t[i], t[(i + 1) % 3]);
                let n = [q[1] - p[1], p[0] - q[0]];
                let proj = |s: &[[f64; 2]; 3]| {
                    let v = s.map(|x| x[0] * n[0] + x[1] * n[1]);
                    (v[0].min(v[1]).min(v[2]), v[0].max(v[1]).max(v[2]))
                };
                let ((a0, a1), (b0, b1)) = (proj(a), proj(b));
                if a1 <= b0 || b1 <= a0 { return false; }
            }
        }
        true
    }

    /// Bind-pose (packed position) triangle pairs of one class that lie in one plane (every vertex of each
    /// within `tol` packed units of the other's plane) and overlap with positive area, from different
    /// texture batches (texture −1 is the grey batch). Returns (pairs, distinct (earlier batch, later batch)
    /// edges in game draw order, whether those edges admit one batch order (no cycle)).
    fn cross_batch_overlaps(class: &LevelMobyClass, tol: f64) -> (usize, Vec<(usize, usize)>, bool) {
        let mut tris: Vec<(usize, [P; 3])> = Vec::new();
        for sub in &class.class.high_lod {
            for t in &sub.triangles {
                let Some(tex) = part_texture(class, t.texture) else { continue };
                let p = [t.a, t.b, t.c].map(|i| sub.vertices[i as usize].packed_position().map(|c| c as f64));
                tris.push((tex, p));
            }
        }
        let on_plane = |t: &[P; 3], s: &[P; 3]| {
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let len = dot(n, n).sqrt();
            len > 0.0 && s.iter().all(|&v| (dot(n, sub(v, t[0])) / len).abs() <= tol).then_some(n).is_some()
        };
        let (mut pairs, mut edges) = (0, std::collections::BTreeSet::new());
        for i in 0..tris.len() {
            for j in i + 1..tris.len() {
                let ((ba, ta), (bb, tb)) = (&tris[i], &tris[j]);
                if ba == bb || !on_plane(ta, tb) || !on_plane(tb, ta) { continue; }
                let n = cross(sub(ta[1], ta[0]), sub(ta[2], ta[0]));
                let k = (0..3).max_by(|&x, &y| n[x].abs().total_cmp(&n[y].abs())).unwrap();
                let (u, v) = ((k + 1) % 3, (k + 2) % 3);
                let flat = |t: &[P; 3]| t.map(|p| [p[u], p[v]]);
                if overlap_2d(&flat(ta), &flat(tb)) {
                    pairs += 1;
                    edges.insert((*ba, *bb)); // j is later in the game's packet order: its batch wins under GEQUAL
                }
            }
        }
        let edges: Vec<(usize, usize)> = edges.into_iter().collect();
        let acyclic = !edges.iter().any(|&(a, b)| edges.contains(&(b, a))) && {
            // Kahn's algorithm over the batch graph.
            let nodes: std::collections::BTreeSet<usize> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
            let mut indeg: HashMap<usize, usize> = nodes.iter().map(|&n| (n, 0)).collect();
            for &(_, b) in &edges { *indeg.get_mut(&b).unwrap() += 1; }
            let mut ready: Vec<usize> = indeg.iter().filter(|e| *e.1 == 0).map(|e| *e.0).collect();
            let mut seen = 0;
            while let Some(n) = ready.pop() {
                seen += 1;
                for &(a, b) in &edges {
                    if a == n { let d = indeg.get_mut(&b).unwrap(); *d -= 1; if *d == 0 { ready.push(b); } }
                }
            }
            seen == nodes.len()
        };
        (pairs, edges, acyclic)
    }

    /// Measurement for the within-class draw order question (gs_state.rs "Packet order"): how many
    /// coplanar, overlapping triangle pairs of Ratchet (class 0) and class 577 come from different texture
    /// batches. Needs the disc data; skipped without it.
    #[test]
    fn moby_cross_batch_coplanar_overlaps() {
        let root = crate::level_load::extracted_root();
        let (Ok(index), Ok(data)) = (crate::disc_source::level_file(&root, 1, "core_index.bin"), crate::disc_source::level_file(&root, 1, "core_data.bin")) else {
            eprintln!("skipped: no level 01 data");
            return;
        };
        let data = rc_formats::wad::decompress(&data).unwrap();
        let core = rc_formats::level::parse_level_core(&index, data.len()).unwrap();
        let classes = moby::parse_level_mobys(&core, &data).unwrap();
        for o in [0, 577] {
            let c = classes.iter().find(|c| c.o_class == o).expect("class on Novalis");
            for tol in [0.0, 0.5, 2.0] {
                let (pairs, edges, acyclic) = cross_batch_overlaps(c, tol);
                println!("moby class {o}: tol {tol}: {pairs} coplanar overlapping cross-batch pairs, batch edges (earlier -> later) {edges:?}, one batch order possible: {acyclic}");
                // Measured 2026-09-26: none, so the class's batch order cannot change a pixel (gs_state.rs).
                assert_eq!(pairs, 0);
            }
        }
        // Level-wide (tol 0): 3 of 169 classes (725: 6 pairs, 731: 4, 790: 4), one batch edge each.
        let others: Vec<(i32, usize)> =
            classes.iter().map(|c| (c.o_class, cross_batch_overlaps(c, 0.0).0)).filter(|&(_, n)| n > 0).collect();
        println!("classes with cross-batch coplanar overlaps: {others:?} of {}", classes.len());
    }
}

// ===================================================================================================
// Extra (runtime-created) moby instances — crate::moby_attach. Separate from the gameplay instances above:
// the caller owns an instance-record buffer and a palette buffer of its own (both kept in the main world so
// it can rewrite them), and every part entity carries `MeshTag(slot)` into that record buffer. Everything
// else (meshes, textures, GS passes, shaders) is the gameplay path's. They are always drawn (high LOD, α 0x80);
// a class with metal packets also gets its shine pass (update_extra_metal).

/// Size of one `MobyInst` record (moby.wgsl).
pub const EXTRA_RECORD_SIZE: usize = RECORD_SIZE;

/// The record of one extra instance: `model` = game_to_bevy · [s/1024 · R | p], the light block (None =
/// unlit), the palette base.
pub fn extra_record(model: &Mat4, lights: Option<&MobyLights>, palette_base: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(RECORD_SIZE);
    let mode = if lights.is_some() { MODE_GPU_LIGHT } else { MODE_UNLIT };
    write_record(&mut out, model, &lights.map(GpuLights::new).unwrap_or_default(), [palette_base, mode, 0, 0]);
    out
}

/// `game_to_bevy · [s/1024 · R | p]` for rotation rows `rows` (row i = image of model axis i, game space).
pub fn extra_model(rows: [[f32; 3]; 3], scale: f32, position: [f32; 3]) -> Mat4 {
    let r = Mat3::from_cols(Vec3::from(rows[0]), Vec3::from(rows[1]), Vec3::from(rows[2]));
    Mat4::from_mat3_translation(axes() * r * (scale / 1024.0), game_to_bevy(position))
}

/// Shared state for spawning extra instances: the storage buffers and the per-texture images / materials.
pub struct ExtraMobys {
    pub instances: Handle<ShaderBuffer>,
    pub palette: Handle<ShaderBuffer>,
    normal_table: Handle<ShaderBuffer>,
    cpu_colors: Handle<ShaderBuffer>,
    /// `MobyLod` per slot (α 0x80; shine alpha and E written by update_extra_metal).
    lods: Handle<ShaderBuffer>,
    fog: crate::game_camera::TfragFog,
    images: HashMap<usize, (Handle<Image>, AlphaRange)>,
    materials: HashMap<(usize, GsPass), Handle<MobyMaterial>>,
    metal_images: HashMap<i32, (Handle<Image>, AlphaRange)>,
}

/// A metal entity of an extra instance: where update_extra_metal reads its placement and writes its shine.
#[derive(Component)]
pub struct ExtraMetal {
    slot: u32,
    records: Handle<ShaderBuffer>,
    lods: Handle<ShaderBuffer>,
    /// moby+0x2c (the item's class scale).
    scale: f32,
    /// Class header 0x30 (packed units): the sphere, as the item's sequence state is not visible here.
    sphere: [f32; 4],
}

impl ExtraMobys {
    /// `records` / `palette` = the initial buffer contents (their sizes are fixed from then on).
    pub fn new(level: &LoadedLevel, records: Vec<u8>, palette: Vec<u8>, buffers: &mut Assets<ShaderBuffer>) -> Self {
        let table_bytes: Vec<u8> = match &level.mobys.lighting {
            Some(l) => l.table.0.iter().flatten().flat_map(|w| w.to_le_bytes()).collect(),
            None => vec![0; 256 * 8],
        };
        let slots = records.len() / RECORD_SIZE;
        ExtraMobys {
            instances: buffers.add(ShaderBuffer::new(&records, RenderAssetUsages::default())),
            palette: buffers.add(ShaderBuffer::new(&palette, RenderAssetUsages::default())),
            normal_table: buffers.add(ShaderBuffer::new(&table_bytes, RenderAssetUsages::RENDER_WORLD)),
            cpu_colors: buffers.add(ShaderBuffer::new(&[0u8; 16], RenderAssetUsages::RENDER_WORLD)),
            lods: buffers.add(ShaderBuffer::new(&moby_lod::default_lod_records(slots), RenderAssetUsages::default())),
            fog: crate::game_camera::TfragFog::new(&level.fog),
            images: HashMap::new(),
            materials: HashMap::new(),
            metal_images: HashMap::new(),
        }
    }

    /// The highest joint index any high-LOD or metal vertex of `class` skins to (palette slots needed: this + 1).
    pub fn max_skinned_joint(class: &LevelMobyClass) -> u8 {
        class.class.high_lod.iter().chain(&class.class.metal).flat_map(|s| &s.vertices).map(|v| *v.skin.joints[..v.skin.count.max(1) as usize].iter().max().unwrap()).max().unwrap_or(0)
    }

    /// Builds `class`'s meshes (as for a gameplay instance) and spawns one entity per (texture, GS pass),
    /// tagged `MeshTag(slot)`, at `transform`, plus its metal entities. Returns the entities.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        commands: &mut Commands,
        level: &LoadedLevel,
        class: &LevelMobyClass,
        slot: u32,
        transform: Transform,
        name: &str,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        materials: &mut Assets<MobyMaterial>,
    ) -> Vec<Entity> {
        let (parts, _) = build_parts(class, &class.class.high_lod, false, meshes);
        let mut out = Vec::new();
        for part in &parts {
            let (image, texel_alpha) = self.images.entry(part.texture).or_insert_with(|| moby_image(level, part.texture, images)).clone();
            for pass in gs_state::draws(gs_state::AREF_WORLD, texel_alpha, part.mult_alpha) {
                let mat = self
                    .materials
                    .entry((part.texture, pass))
                    .or_insert_with(|| {
                        materials.add(MobyMaterial {
                            texture: image.clone(),
                            fog: self.fog,
                            instances: self.instances.clone(),
                            palette: self.palette.clone(),
                            normal_table: self.normal_table.clone(),
                            cpu_colors: self.cpu_colors.clone(),
                            lods: self.lods.clone(),
                            pass,
                        })
                    })
                    .clone();
                let e = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(mat),
                    transform,
                    MeshTag(slot),
                    NoFrustumCulling,
                    Name::new(format!("{name} class {} tex {} {pass:?}", class.o_class, part.texture as isize)),
                ));
                out.push(e.id());
            }
        }
        // The shine pass. The caller only lends the regular material store, so the metal materials are added
        // by a queued command.
        if moby_lod::metal_enabled() {
            let (metal, _) = build_metal_parts(class, meshes);
            for part in &metal {
                let (image, texel_alpha) = self.metal_images.entry(part.kind).or_insert_with(|| metal_image(level, part.kind, images)).clone();
                for pass in metal_passes(texel_alpha) {
                    let mat = MobyMetalMaterial {
                        texture: image.clone(),
                        fog: self.fog,
                        instances: self.instances.clone(),
                        palette: self.palette.clone(),
                        normal_table: self.normal_table.clone(),
                        lods: self.lods.clone(),
                        pass,
                    };
                    let e = commands
                        .spawn((
                            Mesh3d(part.mesh.clone()),
                            transform,
                            MeshTag(slot),
                            NoFrustumCulling,
                            ExtraMetal { slot, records: self.instances.clone(), lods: self.lods.clone(), scale: class.class.header.scale, sphere: class.class.header.bsphere },
                            Name::new(format!("{name} class {} metal {} {pass:?}", class.o_class, part.kind)),
                        ))
                        .id();
                    commands.queue(move |world: &mut World| {
                        let h = world.resource_mut::<Assets<MobyMetalMaterial>>().add(mat);
                        if let Ok(mut ent) = world.get_entity_mut(e) { ent.insert(MeshMaterial3d(h)); }
                    });
                    out.push(e);
                }
            }
        }
        out
    }
}

/// The shine gate and basis of every extra instance with metal entities, from its record's model matrix
/// (`game_to_bevy · [s/1024 · R | p]`, written by moby_attach) and the class sphere. The draw-distance, LOD
/// and fade tests are not applied to extras.
fn update_extra_metal(
    q: Query<&ExtraMetal>,
    cams: Query<&Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(t) = cams.iter().next() else { return };
    let [fwd, left, up] = crate::game_camera::game_rows(t);
    let (eye, cam) = (crate::game_camera::game_eye(t), moby_lod::camera_rows(fwd, left, up));
    let mut done = HashSet::new();
    let mut writes: Vec<(Handle<ShaderBuffer>, u32, [u8; moby_lod::LOD_RECORD_SIZE])> = Vec::new();
    for x in &q {
        if !done.insert((x.lods.id(), x.slot)) { continue; }
        let Some(data) = buffers.get(&x.records).and_then(|b| b.data.as_ref()) else { continue };
        let at = x.slot as usize * RECORD_SIZE;
        let Some(bytes) = data.get(at..at + 64) else { continue };
        let f: [f32; 16] = std::array::from_fn(|i| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()));
        let k = x.scale / 1024.0;
        if k == 0.0 { continue; }
        let to_game = |v: [f32; 3]| [v[0], -v[2], v[1]];
        let rows = [0, 1, 2].map(|c| to_game([f[c * 4], f[c * 4 + 1], f[c * 4 + 2]]).map(|v| v / k));
        let inp = ProcInput {
            position: to_game([f[12], f[13], f[14]]),
            rows,
            scale: x.scale,
            draw_distance: moby_lod::DRAW_DISTANCE_CAP,
            lod_trans: 0xff,
            shine_distance: moby_lod::SHINE_DISTANCE,
            alpha: 0x80,
        };
        let sphere = moby_lod::world_sphere(&inp, x.sphere);
        let v = moby_lod::view_centre(sphere, eye, &cam);
        let shine = moby_lod::shine_alpha(moby_lod::sphere_depth(v, sphere[3]), inp.shine_distance);
        let e = moby_lod::shine_basis(sphere, eye, &cam, &rows);
        let mut rec = [0u8; moby_lod::LOD_RECORD_SIZE];
        moby_lod::write_lod_record(&mut rec, 0x80, 0, shine, &e);
        writes.push((x.lods.clone(), x.slot, rec));
    }
    for (h, slot, rec) in writes {
        let at = slot as usize * moby_lod::LOD_RECORD_SIZE;
        let range = at..at + moby_lod::LOD_RECORD_SIZE;
        if buffers.get(&h).and_then(|b| b.data.as_ref()).and_then(|d| d.get(range.clone())).is_none_or(|d| d == rec) { continue; }
        let Some(mut buf) = buffers.get_mut(&h) else { continue };
        if let Some(data) = buf.data.as_mut().and_then(|d| d.get_mut(range)) { data.copy_from_slice(&rec); }
    }
}

impl MobyOcclusion {
    /// The `MobyAnim` instance of gameplay instance `ii` (None without geometry).
    pub fn anim_index(&self, ii: usize) -> Option<usize> { self.anim_index.get(ii).copied().flatten() }
}

// ---------------------------------------------------------------------------------------------------
// Scheduler-driven static instances (crate::gameplay): the moby loop moves / turns / hides the static
// instances of the ported classes (bolts, crates, grass) every tick, and reads back their +0x31.

impl MobyOcclusion {
    /// +0x31 of static instance `ii` for the moby loop: MobyProc drew it this frame (not hidden by the moby
    /// loop, occlusion and the draw-distance / near / frustum culls passed). Spawn-hidden instances
    /// (crate::moby_spawn) are the caller's to exclude.
    pub fn drawn(&self, ii: usize) -> bool { self.shown.get(ii).is_some_and(|s| s.is_some()) }

    /// The placement the moby loop gave static instance `ii` this tick: position (+0x10), rotation rows
    /// (+0xc0.., row i = image of model axis i), scale (+0x2c), the light block for those rows (None: keep
    /// the loaded one) and hidden (deleted or mode & 1). Rewrites its `MobyInst` record (uploaded by
    /// `update_moby_occlusion` when anything changed) and MobyProc's inputs.
    pub fn drive(&mut self, ii: usize, position: [f32; 3], rows: [[f32; 3]; 3], scale: f32, lights: Option<&MobyLights>, hidden: bool) {
        let Some(d) = self.draws.get_mut(ii) else { return };
        if let Some(inp) = d.input.as_mut() {
            inp.position = position;
            inp.rows = rows;
            inp.scale = scale;
        }
        self.driven_hidden[ii] = hidden;
        let mut rec = Vec::with_capacity(RECORD_SIZE);
        let model = extra_model(rows, scale, position);
        write_record(&mut rec, &model, &lights.map(GpuLights::new).unwrap_or_default(), [0; 4]);
        let at = ii * RECORD_SIZE;
        let n = if lights.is_some() { 192 } else { 64 };
        let Some(dst) = self.records.get_mut(at..at + n) else { return };
        if dst != &rec[..n] {
            dst.copy_from_slice(&rec[..n]);
            self.records_dirty = true;
        }
    }
}
