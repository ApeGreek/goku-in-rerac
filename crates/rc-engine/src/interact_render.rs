//! The "use" system in the engine (`rc_game::moby_update::interact`, docs/plan/interaction.md): the level's talk
//! tables and text installed into the moby services at load, the hand-offs the classes request (the vendor's mode
//! 5, a talker's scene or movie) consumed after each gameplay tick, the context prompt fed to the HUD (slot 12), and
//! game mode 5 itself (`rc_game::menus::vendor`) driven once per main-loop frame from crate::menu_render.
//!
//! * **Load** ([`install`], called by crate::gameplay's setup before the load pass): `TalkTables::load` on the level
//!   overlay (node lists 0x1b1af8, level ranges 0x1c4938, price records 0x1c4530), the level's messages and
//!   language, the talk slots of the created mobys (instance +0x74 → `0x179638`, the loader's
//!   `MobyUnknown74Hook`), the save values the conditions read.
//! * **After a tick** ([`after_tick`], mode 0): `OpenVendor` → mode 5 (the vendor screen, [`VendorRt`]); the
//!   talkers' `Scene` / `Movie` hand-offs stay for crate::scene_render (which plays the scene, or skips the movie,
//!   and at the end places Ratchet and signals `Interact::scene_ended`); the rest are logged. The prompt goes to the
//!   HUD ([`crate::hud_render::HudFeed`]).
//! * **Mode 5** ([`vendor_frame`]): `UpdatePad` (scripted pad by main-loop frame, as mode 3), `Vendor::frame`, its
//!   draws into the menu 2D layer, its class sounds into the audio system (on the game's stream, listener = the play
//!   camera), the HUD's bolt counter pinned and its ammo slot on the selected ammo entry; the camera cut to the
//!   vendor's front after the open's 4-frame fade ([`rc_game::menus::vendor::vendor_camera`], written into
//!   [`PlayView`]); on the exit: mode 0, the vendor's state 1, the follow camera's view back, the bolt counter into
//!   the moby services (their copy is what the next tick writes back).
//!
//! Environment: `RC_INTERACT_TRACE=1` prints the prompt owner changes and every hand-off.

use crate::gameplay::Play;
use crate::play_camera::PlayView;
use bevy::prelude::*;
use rc_formats::save_game::ItemTables;
use rc_game::follow_camera::CameraView;
use rc_game::game_state::GameState;
use rc_game::menus::mode::{Mode, ModeState};
use rc_game::menus::vendor::{vendor_camera, Vendor, VendorOut, VendorTables};
use rc_game::menus::{MenuAssets, MenuDraw, MenuInput, Overlay};
use rc_game::moby_runtime::MobyId;
use rc_game::moby_update::interact::{Handoff, TalkTables};
use rc_game::moby_update::services::SoundEvent;
use rc_game::moby_update::Services;
use rc_game::ps2v::Pf;
use std::sync::Arc;

/// The level overlay (`levels/NN/overlay.bin`).
fn overlay() -> anyhow::Result<(Vec<u8>, Overlay)> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let bytes = crate::disc_source::level_file(&root, index, "overlay.bin")?;
    let ov = Overlay::parse(&bytes)?;
    Ok((bytes, ov))
}

/// The load step (module docs): tables, text, talk slots, save values.
pub fn install(svc: &mut Services, lv: &crate::level_load::LoadedLevel, instance_to_moby: &[Option<MobyId>], state: Option<&GameState>) {
    let it = &mut svc.interact;
    match overlay().map(|(_, ov)| TalkTables::load(&ov)) {
        Ok(Some(t)) => {
            let n = t.tables.iter().filter(|x| !x.is_empty()).count();
            it.tables = Arc::new(t);
            println!("interact: talk tables: {n} node lists, level ranges {:?}", it.tables.base);
        }
        Ok(None) => eprintln!("interact: talk tables not found in the overlay: NPC talk disabled"),
        Err(e) => eprintln!("interact: overlay not read ({e:#}): NPC talk disabled"),
    }
    if let Some(h) = lv.hud.as_ref() {
        it.messages = Arc::new(h.messages.clone());
        it.lang = h.lang;
    }
    let slots: Vec<(MobyId, i32)> = lv
        .mobys
        .instances
        .iter()
        .enumerate()
        .filter(|(_, i)| i.unknown_74 != -1)
        .filter_map(|(k, i)| Some((instance_to_moby.get(k).copied().flatten()?, i.unknown_74)))
        .collect();
    println!("interact: {} talk slots (instance +0x74): {:?}", slots.len(), slots);
    it.register_talk_mobys(slots);
    if let Some(gs) = state { it.sync_game(gs); }
}

/// The vendor mode's engine state.
#[derive(Resource, Default)]
pub struct VendorRt {
    tables: Option<VendorTables>,
    items: Option<ItemTables>,
    loaded: bool,
    pub vendor: Option<Vendor>,
    moby: Option<MobyId>,
    /// The follow camera's view when the vendor opened (restored on the exit).
    saved_view: Option<CameraView>,
    trace: bool,
    last_owner: i32,
}

impl VendorRt {
    fn load(&mut self, play: &Play) {
        if self.loaded { return; }
        self.loaded = true;
        self.trace = std::env::var("RC_INTERACT_TRACE").is_ok_and(|v| v.trim() == "1");
        let root = crate::level_load::extracted_root();
        let r = (|| -> anyhow::Result<(ItemTables, VendorTables)> {
            let (bytes, ov) = overlay()?;
            let elf = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99"))?;
            let items = ItemTables::load(&elf, &bytes)?;
            let vt = VendorTables::load(&ov, items.item_defs_addr, play.svc.interact.tables.shop.clone())
                .ok_or_else(|| anyhow::anyhow!("vendor tables not found"))?;
            Ok((items, vt))
        })();
        match r {
            Ok((i, t)) => {
                println!("interact: vendor tables: {} ticker lines, item names {:?}", t.ticker.len(), &t.name[10..20]);
                self.items = Some(i);
                self.tables = Some(t);
            }
            Err(e) => eprintln!("interact: no vendor ({e:#})"),
        }
    }
}

/// The class sounds of the vendor (`PlayClassSound(n, 0, vendor)`) into the audio system.
fn play_sounds(play: &mut Play, audio: Option<&mut crate::audio_out::AudioOut>, moby: Option<MobyId>, sounds: &[u8]) {
    let Some(id) = moby else { return };
    let Some(m) = play.game.mobys.mobys.get(id) else { return };
    let ev = |i: u8| SoundEvent { index: i as i32, flags: 0, moby: id, o_class: m.o_class, sound_class: m.o_class, pos: [m.position[0], m.position[1], m.position[2]], tick: play.game.counter };
    let evs: Vec<SoundEvent> = sounds.iter().map(|&i| ev(i)).collect();
    if let Some(a) = audio {
        let listener = rc_game::audio::class_sounds::listener_of(&play.game.camera.out);
        for e in &evs { a.system().play_class_sound(e, None, &listener, &mut play.game.rng); }
    }
}

/// After a mode-0 tick: the hand-offs and the prompt (module docs).
#[allow(clippy::too_many_arguments)]
pub fn after_tick(
    vr: &mut VendorRt,
    play: &mut Play,
    gs: &mut GameState,
    mode: &mut ModeState,
    frame: u64,
    feed: Option<&mut crate::hud_render::HudFeed>,
    audio: Option<&mut crate::audio_out::AudioOut>,
) {
    vr.load(play);
    let it = &mut play.svc.interact;
    if vr.trace && it.prompt.owner != vr.last_owner {
        println!("interact: frame {frame}: prompt owner {} → {} (msg {})", vr.last_owner, it.prompt.owner, it.prompt.msg);
    }
    vr.last_owner = it.prompt.owner;
    if let Some(f) = feed {
        f.prompt = it.prompt_hud || it.talk_shown;
        if f.prompt_text != it.prompt.text { f.prompt_text = it.prompt.text.clone(); }
    }
    let mut audio = audio;
    let handoffs = std::mem::take(&mut play.svc.interact.handoffs);
    let mut keep = Vec::new();
    for h in handoffs {
        println!("interact: frame {frame}: hand-off {h:?}");
        match h {
            Handoff::OpenVendor { vendor } => {
                let Some(t) = vr.tables.clone() else { continue };
                let mut out = VendorOut::default();
                let v = Vendor::open(t, gs, vendor.is_none(), &mut out);
                println!(
                    "interact: frame {frame}: OpenVendorMenu: mode 5, {} entries {:?}, selection {}",
                    v.items.len(), v.items.iter().map(|e| (e.item, e.ammo)).collect::<Vec<_>>(), v.sel
                );
                vr.vendor = Some(v);
                vr.moby = vendor;
                vr.saved_view = None;
                play_sounds(play, audio.as_deref_mut(), vendor, &out.sounds);
                mode.set(Mode::Vendor);
            }
            // crate::scene_render's.
            h @ (Handoff::Scene { .. } | Handoff::Movie { .. }) => keep.push(h),
            Handoff::ShipMenu | Handoff::GoldUpgrade { .. } => {}
        }
    }
    play.svc.interact.handoffs = keep;
}

/// The vendor camera as a game view.
fn camera_view(m: &rc_game::moby_runtime::Moby) -> CameraView {
    let (eye, rows) = vendor_camera([m.position[0], m.position[1], m.position[2]], m.rows, m.rotation[2]);
    let v = |a: [f32; 3]| [Pf::f(a[0]), Pf::f(a[1]), Pf::f(a[2]), Pf::ZERO];
    let yaw = rows[0][1].atan2(rows[0][0]);
    CameraView { pos: v(eye), euler: [Pf::ZERO, Pf::ZERO, Pf::f(yaw), Pf::ZERO], rows: [v(rows[0]), v(rows[1]), v(rows[2])] }
}

/// One mode-5 frame (module docs). Returns the frame's draws.
#[allow(clippy::too_many_arguments)]
pub fn vendor_frame(
    vr: &mut VendorRt,
    play: &mut Play,
    gs: &mut GameState,
    sess: &mut rc_game::game_state::SessionState,
    mode: &mut ModeState,
    input: MenuInput,
    assets: &MenuAssets,
    frame: u64,
    view: Option<&mut PlayView>,
    feed: Option<&mut crate::hud_render::HudFeed>,
    audio: Option<&mut crate::audio_out::AudioOut>,
    draws: &mut Vec<MenuDraw>,
) {
    let (Some(v), Some(items)) = (vr.vendor.as_mut(), vr.items.as_ref()) else {
        mode.set(Mode::Gameplay);
        return;
    };
    let was_fading = v.pre_fade > 0;
    let bolts0 = gs.global.bolts;
    let out = v.frame(&input, gs, items, sess, assets, &mut play.game.rng);
    // The cut to the vendor's front once the open's fade is black.
    if was_fading && v.pre_fade == 0 {
        if let (Some(view), Some(m)) = (view, vr.moby.and_then(|id| play.game.mobys.mobys.get(id))) {
            vr.saved_view = Some(view.view);
            view.view = camera_view(m);
        }
    }
    v.draw(assets, gs, frame as u32, draws);
    if let Some(f) = feed {
        f.prompt = false;
        f.bolts_pinned = !out.exit;
        f.weapon = Some(v.hud_ammo(gs));
    }
    if let Some(p) = out.purchase {
        println!("interact: frame {frame}: purchase item {} ({}) for {} bolts, {} unit(s): bolts {} → {}, owned {}, quick select {:?}",
            p.0, if p.1 { "ammo" } else { "weapon" }, p.2, p.3, bolts0, gs.global.bolts, gs.global.owned[p.0], gs.global.quick_select);
    }
    if !out.sounds.is_empty() && vr.trace { println!("interact: frame {frame}: vendor sounds {:?}", out.sounds); }
    // The moby services' bolt copy is what the next tick writes back (gameplay::tick).
    play.svc.counters.bolts = gs.global.bolts;
    let moby = vr.moby;
    play_sounds(play, audio, moby, &out.sounds);
    if out.exit {
        // VendorExit 0x2ae660: mode 0, the HUD slots released, the vendor's state 1, the follow camera back.
        if let Some(id) = moby { rc_game::moby_update::classes::vendor::on_exit(&mut play.game.mobys, id); }
        println!("interact: frame {frame}: VendorExit: mode 0 (bolts {}, ammo {:?})", gs.global.bolts, &gs.global.ammo[10..20]);
        vr.vendor = None;
        vr.saved_view = None;
        mode.set(Mode::Gameplay);
    }
}

/// The HUD's feed when no vendor is open (the prompt only).
pub fn feed_idle(feed: &mut crate::hud_render::HudFeed) {
    feed.bolts_pinned = false;
    feed.weapon = None;
}

/// Marks Ratchet's entities and his items hidden while the vendor is open.
#[derive(Component)]
pub struct VendorHidden;

/// `OpenVendorMenu`'s `0x1413f5 = 1` / `FUN_002486c0`: Ratchet and his items are hidden while the vendor is open (from
/// the camera cut on), shown again by `VendorExit` (`FUN_002487a8`). The tick that would do it through `HeroSyncMoby`
/// does not run in mode 5, so the entities are hidden here (as crate::scene_render does for mode 2).
#[allow(clippy::type_complexity)]
pub fn hide_hero(
    vr: Res<VendorRt>,
    mut commands: Commands,
    hero: Query<(Entity, &Name), (With<MeshMaterial3d<crate::moby_render::MobyMaterial>>, Without<crate::moby_attach::AttachedTo>, Without<VendorHidden>)>,
    items: Query<Entity, (With<crate::moby_attach::AttachedTo>, Without<VendorHidden>)>,
    mut hidden: Query<(Entity, &mut Visibility), With<VendorHidden>>,
) {
    let want = vr.vendor.as_ref().is_some_and(|v| v.pre_fade == 0 && v.sub < 2);
    if want {
        for (e, name) in &hero {
            if name.as_str().starts_with("Ratchet (play)") { commands.entity(e).insert((VendorHidden, Visibility::Hidden)); }
        }
        for e in &items { commands.entity(e).insert((VendorHidden, Visibility::Hidden)); }
        for (_, mut v) in &mut hidden { if *v != Visibility::Hidden { *v = Visibility::Hidden; } }
    } else {
        for (e, _) in &hidden { commands.entity(e).remove::<VendorHidden>().insert(Visibility::Inherited); }
    }
}
