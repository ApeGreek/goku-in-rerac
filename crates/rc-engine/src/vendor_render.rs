//! What the Gadgetron vendor draws (docs/plan/interaction.md §9), from the state crate::interact_render keeps:
//!
//! * **The hologram on approach** (class 11 states 1 / 2, level01 0x2bb128): the Gadgetron logo, class 1143 (its
//!   chrome packets need the metal pass the dynamic-moby path lacks, so it is drawn here, from the child moby's
//!   position, rows and scale), its two joints turning ±0.01 rad per tick about z (`AttachManipulator` records
//!   rewritten by `FUN_00221e38`), lit with its own light word and ambient; and the class's draw callback 0x2ba9c0:
//!   the projector beam, 4 FX quads over the 9 vertices of 0x1d77e0 (x / y scaled by the hologram's size), V scrolled
//!   +0.01 per draw, turned to face the camera [M: the callback's rotation and texture are lost in the decompile; FX 0x18
//!   as the menu's cone].
//! * **Mode 5** (`DrawWorld_Mode5` 0x2b4020): the item hologram (+0x200 ammo / +0x300 weapon) in the world before
//!   the vendor, the hologram cone (`VendorDrawHologramCone` 0x2b3cc8: FX 0x18, ALPHA 0x44, V scroll), the six screens
//!   placed on the vendor's monitor joints, the popup while buying, the glass quads (FX 0x19) and then the HUD.
//!   A screen's black target, content and static are 2D primitives of the HUD pass at the screen's rectangle (the target
//!   copied 1:1 at rest, squeezed while powering on / off); the item panel's and the salesman's 3D parts are
//!   crate::screen_canvas canvases under the HUD pass: the item model (+0x100) with the class-13 backdrop, and the
//!   salesman (class 12 with the `vendor.bin` sequences), each drawn with the target's zoom-1.0 view and light set 14
//!   (the vendor's light: colour (0.9, 0.9, 0.6), direction vendor rows · (−0.42, −0.7, −0.577), ambient 0x202020).
//!
//! Differences: the world behind the menu is drawn live (the game re-uploads a still snapshot taken with the vendor
//! hidden; the port's world does not move in the menu either, but it can hide parts of the vendor where scenery is
//! nearer); the screens' 2D pass draws after the HUD's own elements (the game draws the HUD last; they do not overlap
//! on Novalis); the static's noise uses the HUD's blend (the game adds it with a fixed alpha: the same on the black
//! screens, slightly darker over text).

use crate::hud_render::{Hud2d, Hud2dHook, HudBuild, Prim, Tex};
use crate::interact_render::{ScreenDraw, VendorRt};
use crate::moby_render::{self, ExtraMobys, MobyMaterial};
use crate::screen_canvas::{CanvasId, CanvasView, Canvases};
use crate::text_render::TextState;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, JointModifier, MobyAnimClass, Rows};
use rc_formats::moby_light::{self, V4};
use rc_game::menus::vendor::{self as vendor, screens, ScreenMoby};
use rc_game::menus::MenuDraw;

/// Extra-moby slots.
const SLOT_ITEM: u32 = 0;
const SLOT_BACKDROP: u32 = 1;
const SLOT_SALESMAN: u32 = 2;
const SLOT_HOLO: u32 = 3;
const SLOT_POPUP: u32 = 4;
const SLOT_LOGO: u32 = 5;
/// Vendors per level whose logo can show at once.
const LOGOS: u32 = 3;
const SLOTS: u32 = SLOT_LOGO + LOGOS;
/// Palette entries per slot (the salesman has 92 joints).
const JOINTS: u32 = 128;
/// The hologram class and the FX textures.
const LOGO_CLASS: i16 = 1143;
const CONE_FX: usize = 0x18;
/// Light set 14 and the screen mobys' light word.
const LIGHT_SET: usize = 14;
const LIGHT_WORD: u32 = 0x0e0e;

#[derive(Resource)]
struct VendorGfx {
    extra: ExtraMobys,
    /// Entities per (slot, class) and what each slot shows.
    ents: HashMap<(u32, i16), Vec<Entity>>,
    shown: HashMap<u32, i16>,
    /// The gadget table's classes (the weapons' models).
    gadgets: std::sync::Arc<Vec<(LevelMobyClass, MobyAnimClass)>>,
    item_canvas: CanvasId,
    salesman_canvas: CanvasId,
    cone: crate::fx_draw::FxSlots,
    /// The beam's V scroll 0x161394 and the tick it last advanced.
    beam_scroll: f32,
    beam_tick: Option<u64>,
    /// The logo's joint-list targets (its manipulators on lists 0 and 1).
    logo_joints: [Option<u8>; 2],
    /// FX 0x19's size (the glass quads' texels).
    glass_size: (i32, i32),
}

pub struct VendorRenderPlugin;

impl Plugin for VendorRenderPlugin {
    fn build(&self, app: &mut App) {
        if !crate::gameplay::enabled() { return; }
        app.add_systems(PreUpdate, setup).add_systems(Update, draw.after(crate::menu_render::MenuPrims).before(HudBuild));
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    mut commands: Commands,
    level: Res<crate::Level>,
    mut canvases: ResMut<Canvases>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if *done { return; }
    *done = true;
    let lv = &level.0;
    let gadgets: Vec<(LevelMobyClass, MobyAnimClass)> = match crate::moby_attach::load_blobs() {
        Ok((_, g)) => g
            .iter()
            .filter_map(|g| moby_anim::parse_sequences(&g.blob, &g.moby.class).ok().map(|s| (g.moby.clone(), MobyAnimClass::new(&g.moby.class, s))))
            .collect(),
        Err(e) => {
            eprintln!("vendor render: no gadget classes ({e:#})");
            Vec::new()
        }
    };
    let records = vec![0u8; SLOTS as usize * moby_render::EXTRA_RECORD_SIZE];
    let extra = ExtraMobys::new(lv, records, crate::moby_anim::identity_palette(SLOTS * JOINTS), &mut buffers);
    let item_canvas = canvases.create(&mut commands, &mut images, "vendor item panel");
    let salesman_canvas = canvases.create(&mut commands, &mut images, "vendor salesman");
    let logo_joints = match crate::interact_render::class_blob(LOGO_CLASS as i32).and_then(|b| Ok((rc_formats::moby::parse_moby_class(&b)?, b))) {
        Ok((c, b)) => [0, 1].map(|l| rc_formats::gadget::joint_list(&b, &c.header, l).ok().and_then(|(_, second)| moby_anim::list_target(&second))),
        Err(_) => [None, None],
    };
    let glass_size = lv.hud.as_ref().and_then(|h| h.fx.get(screens::GLASS_FX).cloned().flatten()).map_or((64, 64), |t| (t.width as i32, t.height as i32));
    println!("vendor render: {} gadget classes, logo manipulator joints {logo_joints:?}, glass FX {glass_size:?}", gadgets.len());
    commands.insert_resource(VendorGfx {
        extra,
        ents: HashMap::default(),
        shown: HashMap::default(),
        gadgets: std::sync::Arc::new(gadgets),
        item_canvas,
        salesman_canvas,
        cone: Default::default(),
        beam_scroll: 0.0,
        beam_tick: None,
        logo_joints,
        glass_size,
    });
}

/// A slot's content this frame.
struct SlotDraw<'a> {
    class: &'a LevelMobyClass,
    rows: [[f32; 3]; 3],
    pos: [f32; 3],
    scale: f32,
    pose: Vec<Rows>,
    light: u32,
    ambient: [u8; 3],
}

fn rows_bits(r: &[[f32; 3]; 3]) -> [V4; 3] { r.map(|v| [v[0].to_bits(), v[1].to_bits(), v[2].to_bits(), 0]) }

/// A screen moby's class geometry and animation (the level's, the salesman's with its `vendor.bin` sequences, or a gadget's).
fn find_class<'a>(lv: &'a crate::level_load::LoadedLevel, gadgets: &'a [(LevelMobyClass, MobyAnimClass)], salesman: Option<&'a MobyAnimClass>, o: i16) -> Option<(&'a LevelMobyClass, &'a MobyAnimClass)> {
    let m = &lv.mobys;
    if let Some(ci) = m.classes.iter().position(|c| c.o_class == o as i32) {
        let anim = if o == vendor::salesman::CLASS { salesman? } else { &m.anim[ci] };
        return Some((&m.classes[ci], anim));
    }
    gadgets.iter().find(|(c, _)| c.o_class == o as i32).map(|(c, a)| (c, a))
}

/// A frozen pose on the first frame of seq `seq` (or 0).
fn still(anim: &MobyAnimClass, seq: u8) -> Vec<Rows> {
    let mut s = AnimState::spawn(anim);
    let seq = if anim.sequence(seq).is_some() { seq } else { 0 };
    moby_anim::hard_cut(&mut s, anim, seq, 0);
    s.speed = 0.0;
    moby_anim::evaluate_with_snapshot(anim, &s, None)
}

/// The screens' 2D primitives (module docs): per screen its black target (the canvases clear their own), content and
/// static in target pixels, mapped onto the rectangle; the popup; then the glass quads.
fn screen_prims(sd: &ScreenDraw, glyphs: &[rc_formats::font::GlyphTable; 3], frame_sizes: &[(i32, i32)], canvas: bool) -> Vec<Prim> {
    let p = sd.placed;
    let (tw, th) = ((p.full[2] - 1.0).max(1.0), (p.full[3] - 1.0).max(1.0));
    let (x0, x1) = (p.drawn[0].min(p.drawn[0] + p.drawn[2]), p.drawn[0].max(p.drawn[0] + p.drawn[2]));
    let (y0, y1) = (p.drawn[1].min(p.drawn[1] + p.drawn[3]), p.drawn[1].max(p.drawn[1] + p.drawn[3]));
    if x1 - x0 < 0.5 || y1 - y0 < 0.5 { return Vec::new(); }
    let (sx, sy) = (p.drawn[2] / tw, p.drawn[3] / th);
    let (ix0, ix1, iy0, iy1) = (x0.round() as i32, x1.round() as i32, y0.round() as i32, y1.round() as i32);
    let scissor = [ix0, ix1 - 1, iy0, iy1 - 1];
    let mut h = Hud2d::default();
    h.frame_sizes = frame_sizes.to_vec();
    let mut st = TextState::default();
    let mut local: Vec<Prim> = Vec::new();
    if !canvas {
        // The target's clear (FUN_00223470 black, copied with ALPHA 0x64 = replace): opaque black.
        local.push(Prim { tex: Tex::None, pos: [[0, 0], [screens::TARGET.0, 0], [0, screens::TARGET.1], [screens::TARGET.0, screens::TARGET.1]], uv: [[0, 0]; 4], rgba: 0x8000_0000, scissor: [0, 0, 0, 0] });
    }
    for d in &sd.content {
        match d {
            MenuDraw::Hud(x) => crate::text_render::execute(&mut h, &mut st, glyphs, std::slice::from_ref(x)),
            MenuDraw::Rect { x0, y0, x1, y1, rgba } => h.rect(y0 - 1, y1 - 1, x0 - 1, x1 - 1, *rgba),
            MenuDraw::SpriteUv { frame, x0, y0, x1, y1, u0, v0, u1, v1, alpha, .. } => {
                let rgba = ((*alpha as u32) & 0xff) << 24 | 0x007f_7f7f;
                let (px0, py0, px1, py1) = (x0 / 16, y0 / 16, x1 / 16, y1 / 16);
                let (ua, ub, va, vb) = (u0 / 16, u1 / 16, v0 / 16, v1 / 16);
                h.prims.push(Prim { tex: Tex::Frame(*frame), pos: [[px0, py0], [px1, py0], [px0, py1], [px1, py1]], uv: [[ua, va], [ub, va], [ua, vb], [ub, vb]], rgba, scissor: [0, 0, 0, 0] });
            }
            _ => {}
        }
    }
    for f in &sd.fx {
        h.strip_glyph(f.fx, f.x as i32, f.y as i32, f.w as i32, f.h as i32, f.u, f.v, f.u1 - f.u, f.v1 - f.v, f.rgba);
    }
    local.extend(h.prims);
    // The target is 512×128: texels outside it are not drawn.
    let map = |q: [i32; 2]| [(p.drawn[0] + q[0] as f32 * sx).round() as i32, (p.drawn[1] + q[1] as f32 * sy).round() as i32];
    local
        .into_iter()
        .map(|mut q| {
            q.pos = q.pos.map(map);
            q.scissor = scissor;
            q
        })
        .collect()
}

/// The asset stores the draw writes.
type DrawAssets<'w> = (
    ResMut<'w, Assets<Mesh>>,
    ResMut<'w, Assets<Image>>,
    ResMut<'w, Assets<MobyMaterial>>,
    ResMut<'w, Assets<crate::fx_draw::FxPrimMaterial>>,
    ResMut<'w, Assets<ShaderBuffer>>,
);

#[allow(clippy::too_many_arguments)]
fn draw(
    mut commands: Commands,
    gfx: Option<ResMut<VendorGfx>>,
    vr: Res<VendorRt>,
    play: Option<Res<crate::gameplay::Play>>,
    level: Res<crate::Level>,
    mut hook: ResMut<Hud2dHook>,
    mut canvases: ResMut<Canvases>,
    cams: Query<&Transform, With<crate::fly_cam::FlyCam>>,
    fog: Option<Res<crate::game_camera::GameFog>>,
    mut vis: Query<&mut Visibility>,
    mut transforms: Query<&mut Transform, Without<crate::fly_cam::FlyCam>>,
    spawn_hidden: Query<(), With<crate::moby_spawn::SpawnHidden>>,
    (mut meshes, mut images, mut materials, mut fx_materials, mut buffers): DrawAssets,
) {
    let (Some(mut gfx), Some(play)) = (gfx, play) else { return };
    let gfx = &mut *gfx;
    let lv = &level.0;
    let d = &vr.draw;
    let gadgets = gfx.gadgets.clone();
    let salesman_anim = vr.vendor.as_ref().and_then(|v| v.tables.classes.salesman.as_ref());
    // ---- The screens' 2D primitives (before the menus' fades: the fade covers them).
    if let Some(lh) = lv.hud.as_ref() {
        let sizes: Vec<(i32, i32)> = lh.frames.iter().map(|t| (t.width as i32, t.height as i32)).collect();
        let mut prims: Vec<Prim> = Vec::new();
        for sd in &d.screens {
            prims.extend(screen_prims(sd, &lh.glyphs, &sizes, sd.s == screens::ITEM || sd.s == screens::SALESMAN));
        }
        if let Some(view) = d.view {
            let (gw, gh) = gfx.glass_size;
            let vh = (gh as f32 * screens::GLASS_V1) as i32;
            for q in &d.glass {
                let c = q.corners();
                let pts: Option<Vec<[f32; 2]>> = c.iter().map(|&p| view.project(p)).collect();
                let Some(pts) = pts else { continue };
                let pos = [0, 1, 2, 3].map(|k| [pts[k][0].round() as i32, pts[k][1].round() as i32]);
                prims.push(Prim { tex: Tex::Fx(screens::GLASS_FX), pos, uv: [[0, 0], [gw, 0], [0, vh], [gw, vh]], rgba: 0x8080_8080, scissor: [0, 511, 0, 415] });
            }
        }
        if !prims.is_empty() {
            prims.extend(std::mem::take(&mut hook.prims));
            hook.prims = prims;
        }
    }
    // ---- The canvases of the item panel and the salesman.
    let canvas_of = |s: usize| d.screens.iter().find(|x| x.s == s).map(|x| {
        // The target's centre (256, 64) (`SetRenderToTextureView(…, 512, 128)`) with the frame's own focal lengths [M: the
        // zoom argument 1.0 does not reach the moby program's scales: with it both models sit at the targets' edges at a
        // third of the size the game shows; with the frame's scales they sit where the game shows them].
        let p = x.placed;
        let (tx, ty) = (crate::game_camera::TAN_HALF_FOV_X, crate::game_camera::TAN_HALF_FOV_X * crate::game_camera::NTSC_Y_RATIO);
        let (sx, sy) = (p.drawn[2] / (p.full[2] - 1.0).max(1.0), p.drawn[3] / (p.full[3] - 1.0).max(1.0));
        CanvasView { rect: p.drawn, focal: [256.0 / tx * sx, 208.0 / ty * sy], centre: [p.drawn[0] + 256.0 * sx, p.drawn[1] + 64.0 * sy], clear: Color::BLACK }
    });
    canvases.show(gfx.item_canvas, canvas_of(screens::ITEM));
    canvases.show(gfx.salesman_canvas, canvas_of(screens::SALESMAN));
    // ---- The mobys.
    let mut want: Vec<(u32, SlotDraw)> = Vec::new();
    // Light set 14 for the vendor's screen mobys.
    let bank = lv.mobys.lighting.as_ref().map(|l| {
        let mut bank = l.bank.clone();
        if let (Some((_, _, rows)), Some(layout)) = (d.vendor, vr.vendor.as_ref().and_then(|v| v.tables.layout.as_ref())) {
            let dir = vendor::to_world(&rows, [0.0; 3], layout.light_dir);
            bank.sets[LIGHT_SET] = rc_formats::tfrag_light::DirLightSet { color_a: layout.light_color, dir_a: [dir[0], dir[1], dir[2], 0.0], color_b: [0.0; 4], dir_b: [0.0; 4] };
        }
        bank
    });
    let amb = |c: u32| [(c >> 16) as u8, (c >> 8) as u8, c as u8];
    let mut screen_moby = |slot: u32, m: &ScreenMoby, seq_still: u8| {
        let Some((class, anim)) = find_class(lv, &gadgets, salesman_anim, m.class) else { return };
        let pose = match &m.anim {
            Some((s, snap)) => moby_anim::evaluate_with_snapshot(anim, s, snap.as_ref()),
            None => still(anim, seq_still),
        };
        want.push((slot, SlotDraw { class, rows: m.rows, pos: m.position, scale: class.class.header.scale, pose, light: LIGHT_WORD, ambient: amb(m.ambient) }));
    };
    for (k, m) in d.scene.item_panel.iter().enumerate() {
        let slot = if m.class == vendor::BACKDROP_CLASS { SLOT_BACKDROP } else { SLOT_ITEM };
        screen_moby(slot, m, if k == 0 { 1 } else { 0 });
    }
    if let Some(m) = &d.scene.salesman { screen_moby(SLOT_SALESMAN, m, 0); }
    if let Some(m) = &d.scene.hologram { screen_moby(SLOT_HOLO, m, 1); }
    if let Some(m) = &d.scene.popup { screen_moby(SLOT_POPUP, m, 0); }
    // The logos of the vendors near Ratchet.
    let table = &play.game.mobys;
    let spin = play.svc.interact.vendor.spin;
    let mut logos: Vec<([f32; 3], f32)> = Vec::new();
    for (id, m) in table.mobys.iter().enumerate() {
        if m.o_class != vendor::VENDOR_CLASS || m.state >= 0x80 { continue; }
        let Some((c, s, shown)) = rc_game::moby_update::classes::vendor::hologram(table, id) else { continue };
        if !shown || logos.len() as u32 >= LOGOS { continue; }
        let child = &table.mobys[c];
        let Some((class, anim)) = find_class(lv, &gadgets, None, LOGO_CLASS) else { continue };
        let mods: Vec<JointModifier> = gfx.logo_joints.iter().zip(spin).filter_map(|(j, a)| {
            let q = rc_game::moby_update::services::axis_quat(rc_game::ps2v::Pf::f(a), 2);
            j.map(|j| JointModifier { quat: q.map(|x| x.to_f32()), ..JointModifier::compose(j) })
        }).collect();
        let st = child.anim;
        let pose = moby_anim::evaluate_posed(anim, &st, play.svc.snapshots.get(c).and_then(|x| x.as_ref()), &[], &mods);
        let rows = [0, 1, 2].map(|i| [child.rows[i][0], child.rows[i][1], child.rows[i][2]]);
        let slot = SLOT_LOGO + logos.len() as u32;
        want.push((slot, SlotDraw { class, rows, pos: [child.position[0], child.position[1], child.position[2]], scale: child.scale, pose, light: child.light, ambient: [child.ambient[0], child.ambient[1], child.ambient[2]] }));
        logos.push(([m.position[0], m.position[1], m.position[2]], s));
    }
    // Records, palettes, entities.
    let mut palette = buffers.get(&gfx.extra.palette).and_then(|b| b.data.clone()).unwrap_or_default();
    let mut records = buffers.get(&gfx.extra.instances).and_then(|b| b.data.clone()).unwrap_or_default();
    let before = (palette.clone(), records.clone());
    let mut now: HashMap<u32, i16> = HashMap::default();
    for (slot, s) in &want {
        let oc = s.class.o_class as i16;
        now.insert(*slot, oc);
        let key = (*slot, oc);
        if !gfx.ents.contains_key(&key) {
            let ents = gfx.extra.spawn(&mut commands, lv, s.class, *slot, Transform::IDENTITY, "vendor screen moby", &mut meshes, &mut images, &mut materials);
            let layer = match *slot {
                SLOT_ITEM | SLOT_BACKDROP => Some(canvases.layer(gfx.item_canvas)),
                SLOT_SALESMAN => Some(canvases.layer(gfx.salesman_canvas)),
                _ => None,
            };
            for &e in &ents {
                if let Some(l) = &layer { commands.entity(e).insert(l.clone()); }
            }
            gfx.ents.insert(key, ents);
        }
        let base = slot * JOINTS;
        let at = base as usize * 64;
        for (i, b) in s.pose.iter().take(JOINTS as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() {
            if let Some(x) = palette.get_mut(at + i) { *x = b; }
        }
        let lights = bank.as_ref().map(|bank| moby_light::moby_lights(&rows_bits(&s.rows), bank, s.light, s.ambient, 0x80));
        let model = moby_render::extra_model(s.rows, s.scale, s.pos);
        let r = moby_render::extra_record(&model, lights.as_ref(), base);
        let o = *slot as usize * moby_render::EXTRA_RECORD_SIZE;
        if let Some(x) = records.get_mut(o..o + r.len()) { x.copy_from_slice(&r); }
        let t = Transform::from_matrix(model);
        for &e in &gfx.ents[&key] {
            if let Ok(mut tr) = transforms.get_mut(e) {
                if *tr != t { *tr = t; }
            }
            // The gameplay setup hides the moby entities tagged with Ratchet's index (MeshTag 0) by `SpawnHidden`.
            if spawn_hidden.contains(e) { commands.entity(e).remove::<crate::moby_spawn::SpawnHidden>(); }
        }
    }
    for (key, ents) in &gfx.ents {
        let show = now.get(&key.0) == Some(&key.1);
        let was = gfx.shown.get(&key.0) == Some(&key.1);
        if show == was && !show { continue; }
        for &e in ents {
            if let Ok(mut v) = vis.get_mut(e) { v.set_if_neq(if show { Visibility::Inherited } else { Visibility::Hidden }); }
        }
    }
    gfx.shown = now;
    if (&palette, &records) != (&before.0, &before.1) {
        if let Some(mut b) = buffers.get_mut(&gfx.extra.palette) { b.data = Some(palette); }
        if let Some(mut b) = buffers.get_mut(&gfx.extra.instances) { b.data = Some(records); }
    }
    // ---- The cone (menu) and the beams (approach).
    let cam = cams.iter().next().map(|t| crate::game_camera::game_eye(t).to_array());
    let counter = play.game.counter;
    if gfx.beam_tick != Some(counter) && !logos.is_empty() {
        gfx.beam_tick = Some(counter);
        gfx.beam_scroll += 0.01;
        if gfx.beam_scroll > 1.0 { gfx.beam_scroll -= 1.0; }
    }
    let mut groups = Vec::new();
    let layout = vr.vendor.as_ref().and_then(|v| v.tables.layout.clone()).or_else(|| vr.layout());
    if let Some(l) = layout.as_ref() {
        let quads = |cone: &vendor::layout::Cone, place: &dyn Fn([f32; 3]) -> [f32; 3], scroll: f32| {
            let mut b = crate::fx_draw::PrimBuf::default();
            for q in &cone.quads {
                let p = q.map(|i| place(cone.verts[i]));
                let st = q.map(|i| [cone.uv[i][0], cone.uv[i][1] + scroll]);
                b.quad(p, st, q.map(|i| cone.rgba[i]));
            }
            crate::fx_draw::FxGroup { fx: CONE_FX, additive: false, prims: b }
        };
        if let (Some((_, pos, rows)), Some(scroll)) = (d.vendor, d.cone) {
            groups.push(quads(&l.cone, &|v| vendor::to_world(&rows, pos, v), scroll));
        }
        for &(p, s) in &logos {
            // Facing the camera about z; x / y scaled by the hologram's size.
            let yaw = cam.map_or(0.0, |c| (c[1] - p[1]).atan2(c[0] - p[0]));
            let (sn, cs) = yaw.sin_cos();
            let place = move |v: [f32; 3]| [p[0] + (v[0] * cs - v[1] * sn) * s, p[1] + (v[0] * sn + v[1] * cs) * s, p[2] + v[2]];
            groups.push(quads(&l.beam, &place, gfx.beam_scroll));
        }
    }
    let fog = fog.map(|f| f.uniform).unwrap_or_else(|| crate::game_camera::TfragFog::new(&lv.fog));
    let tex = lv.particles.textures.as_ref().map(|t| t.fx_textures.as_slice());
    let mut a = crate::fx_draw::FxAssets { meshes: &mut meshes, images: &mut images, materials: &mut fx_materials, fx: tex, fog };
    gfx.cone.show(&mut commands, &mut vis, &mut a, groups, crate::fx_draw::LIST1_BIAS + 50.0, "vendor cone");
}
