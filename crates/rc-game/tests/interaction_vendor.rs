//! The "use" system and the Gadgetron vendor on Novalis, headless (docs/plan/interaction.md): the level's mobys
//! through the loader and the scheduler (the vendor class 11, the gold-weapon offers 304 and the talking NPC 774 on
//! the talk system), the context prompt, the hand-off to game mode 5, and the vendor screen's purchase flow
//! (`rc_game::menus::vendor`) on the game state of a first Novalis arrival. Skipped without `extracted/`.
//!
//! Script (as the engine check, docs/plan/interaction.md §7): Ratchet placed 3 units in front of the vendor, facing
//! it; tick 100 △ opens the vendor; in mode 5, frame 80 ✕ (the Pyrocitor, the middle entry) and frame 100 ✕
//! ("Purchase?" yes); frame 140 △ leaves; back in mode 0 the prompt returns.

use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_formats::{collision, gameplay, level, strings};
use rc_game::game_state::{GameState, SessionState};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::Hero;
use rc_game::hud::HudAssets;
use rc_game::menus::vendor::{Vendor, VendorOut, VendorTables};
use rc_game::menus::{MenuAssets, MenuInput, Overlay};
use rc_game::moby_runtime::{mode, MobyTable, Seq0Info};
use rc_game::moby_update::interact::{owner, Handoff, TalkTables};
use rc_game::moby_update::scheduler::{self, class_info, load_static_mobys, Groups, Scheduler};
use rc_game::moby_update::services::{SharedServices, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::pad::{button, PadInput, PadState};
use rc_game::rng::Rng;
use rc_game::tick::{Game, GameOptions, TickHooks};
use std::sync::Arc;

/// Ratchet 3 units in front of the vendor (169.95, 140.44, 60; yaw −2.2089), facing it.
const HERO_AT: [f32; 4] = [168.16, 138.03, 60.0, 0.9327];

struct Data {
    mesh: collision::Collision,
    instances: Vec<gameplay::MobyInstance>,
    pvars: Vec<Option<Vec<u8>>>,
    splines: Vec<Vec<[f32; 4]>>,
    gp: Vec<u8>,
    classes: ClassTable,
    spawnable: usize,
    death_z: f32,
    coll_blobs: Vec<(i32, rc_formats::moby_collision::MobyCollision)>,
    ratchet: rc_formats::moby_anim::MobyAnimClass,
    overlay: Overlay,
    items: ItemTables,
    messages: Vec<strings::Message>,
    glyphs: [rc_formats::font::GlyphTable; 3],
    state: GameState,
    session: SessionState,
}

fn load() -> Option<Data> {
    use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
    let root = rc_formats::test_data::root();
    let dir = root.join("levels/01");
    let data = rc_formats::test_data::core_data(1)?;
    let idx = std::fs::read(dir.join("core_index.bin")).ok()?;
    let gp = rc_formats::test_data::gameplay(1)?;
    let settings = rc_formats::test_data::gameplay_section(1, "level_settings")?;
    let core = level::parse_level_core(&idx, data.len()).unwrap();
    let mesh = collision::parse_collision(&core, &data).unwrap();
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let pvars = gameplay::parse_pvars(&gp).unwrap();
    let splines = gameplay::parse_splines(&gp).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4) as usize;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    let mut classes = ClassTable::default();
    for (slot, e) in core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let Some(blob) = rc_formats::test_data::core_block(1, &format!("moby_class/{:04}", e.o_class)) else { continue };
        let Ok(c) = rc_formats::moby::parse_moby_class(&blob) else { continue };
        let anim = MobyAnimClass::new(&c, parse_sequences(&blob, &c).unwrap_or_default());
        let mut info = class_info(&c, slot as u8, scheduler::port_update_fn(oc));
        info.seq0 = anim.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
        classes.classes.insert(oc, (info, Some(anim)));
    }
    let coll_blobs = rc_formats::moby_collision::parse_level(&core, &data).unwrap();
    let blob = rc_formats::test_data::core_block(1, "moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(&blob).unwrap();
    let seqs: Vec<Option<MobySequence>> = (0..256)
        .map(|i| rc_formats::test_data::core_block(1, &format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(&b, 0).ok()))
        .collect();
    let ratchet = MobyAnimClass::new(&class, seqs);
    // The game state of a first Novalis arrival (game_state.md §4), with 5000 bolts.
    let elf = std::fs::read(root.join("boot/SCUS_971.99")).ok()?;
    let lump = SaveGameLump::parse(&std::fs::read(root.join("global/save_game.bin")).ok()?).unwrap();
    let ov0 = std::fs::read(root.join("levels/00/overlay.bin")).ok()?;
    let ov1 = std::fs::read(dir.join("overlay.bin")).ok()?;
    let tables = ChunkTables::from_boot_elf(&elf).unwrap();
    let mut state = GameState::new_game(tables, &lump.template).unwrap();
    let mut session = SessionState::default();
    state.apply_level_start(0, &ItemTables::load(&elf, &ov0).unwrap(), &mut session);
    state.on_veldin_clank_init(&mut session);
    state.apply_transition(1);
    let items = ItemTables::load(&elf, &ov1).unwrap();
    state.apply_level_start(1, &items, &mut session);
    state.global.bolts = 5000;
    let messages = strings::parse_strings(&gp, 0).unwrap();
    let (glyphs, _) = rc_formats::font::parse_glyph_tables(&ov1).unwrap();
    let overlay = Overlay::parse(&ov1).unwrap();
    Some(Data { mesh, instances, pvars, splines, gp: gp.to_vec(), classes, spawnable, death_z, coll_blobs, ratchet, overlay, items, messages, glyphs, state, session })
}

/// One frame's record: (tick counter, game mode 0 / 5, prompt owner, vendor class state, bolts, vendor substate).
type Rec = (u64, u8, i32, u8, i32, i8);

struct Run {
    recs: Vec<Rec>,
    handoffs: Vec<(usize, Handoff)>,
    purchases: Vec<(usize, (usize, bool, i32, i32))>,
    sounds: Vec<(usize, Vec<u8>)>,
    state: GameState,
    talk_slots: usize,
}

/// Runs `frames` main-loop frames: gameplay ticks in mode 0 (pad `tick_input(frame)`), vendor frames in mode 5
/// (pad `vendor_input(frame since the open)`).
fn run(d: &Data, frames: usize) -> Run {
    let classes = Arc::new(ClassTable { classes: d.classes.classes.clone() });
    let mut ct = ClassTable { classes: d.classes.classes.clone() };
    let mut statics = load_static_mobys(&d.instances, &mut ct, &d.pvars);
    let hero_idx = statics.iter().position(|m| m.o_class == 0).expect("Ratchet");
    let vendor = statics.iter().position(|m| m.o_class == 11).expect("the vendor 11");
    statics[hero_idx].position = [HERO_AT[0], HERO_AT[1], HERO_AT[2], 0.0];
    statics[hero_idx].rotation[2] = HERO_AT[3];
    let mut table = MobyTable::new(statics, d.spawnable);
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut game = Game::new(&d.mesh, table, hero_idx, GameOptions::default(), d.death_z);
    game.hero.idle.level = 1;
    let mut svc = Services::new();
    svc.level = 1;
    svc.set_splines(&d.splines);
    svc.groups = Groups::parse(&d.gp, &|i| (i < d.instances.len()).then_some(i));
    svc.set_moby_collision(d.coll_blobs.clone());
    svc.build_grid(&mut game.mobys);
    let mut gs = d.state.clone();
    let mut session = d.session;
    svc.counters.bolts = gs.global.bolts;
    svc.counters.gold_weapons = gs.global.gold_weapons.to_vec();
    // The "use" system's load step (the engine's interact_render::install).
    let tables = TalkTables::load(&d.overlay).expect("talk tables");
    let shop = tables.shop.clone();
    svc.interact.tables = Arc::new(tables);
    svc.interact.messages = Arc::new(d.messages.clone());
    svc.interact.register_talk_mobys(d.instances.iter().enumerate().map(|(i, x)| (i, x.unknown_74)));
    let talk_slots = svc.interact.talk_slots.len();
    svc.interact.sync_game(&gs);
    let vt = VendorTables::load(&d.overlay, d.items.item_defs_addr, shop).expect("vendor tables");
    let assets = MenuAssets::new(HudAssets { icons: Vec::new(), frame_sizes: Vec::new(), glyphs: d.glyphs, messages: d.messages.clone() }, d.overlay.clone());
    let mut sched = Scheduler::new();
    {
        let hero: Hero = game.hero.clone();
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &*classes, &mut svc, 0);
        w.camera = game.camera.out.pos;
        w.coll = Some(&d.mesh);
        sched.load_pass(&mut w);
    }
    game.finish_load();
    let mut anim = RatchetAnim::new(&d.ratchet);
    let svc_cell = std::cell::RefCell::new(&mut svc);
    let mut out = Run { recs: Vec::new(), handoffs: Vec::new(), purchases: Vec::new(), sounds: Vec::new(), state: GameState::zeroed(ChunkTables { global: Vec::new(), level: Vec::new() }), talk_slots };
    let mut v: Option<(Vendor, usize)> = None;
    let mut pad = PadState::default();
    for f in 0..frames {
        if let Some((vend, opened)) = v.as_mut() {
            // Mode 5: UpdatePad, then VendorModeUpdate.
            let k = f - *opened;
            let input = match k {
                80 | 100 => PadInput::neutral().press(button::CROSS),
                140 => PadInput::neutral().press(button::TRIANGLE),
                _ => PadInput::neutral(),
            };
            pad.update(Some(&input.bytes()), false);
            let o = vend.frame(&MenuInput::from_pad(&pad, true), &mut gs, &d.items, &mut session, &assets, &mut game.rng);
            svc_cell.borrow_mut().counters.bolts = gs.global.bolts;
            if let Some(p) = o.purchase { out.purchases.push((f, p)); }
            if !o.sounds.is_empty() { out.sounds.push((f, o.sounds.clone())); }
            let sub = vend.sub as i8;
            out.recs.push((game.counter, 5, svc_cell.borrow().interact.prompt.owner, game.mobys.mobys[vendor].state, gs.global.bolts, sub));
            if o.exit {
                rc_game::moby_update::classes::vendor::on_exit(&mut game.mobys, vendor);
                v = None;
            }
            continue;
        }
        let input = if f == 100 { PadInput::neutral().press(button::TRIANGLE) } else { PadInput::neutral() };
        game.hero.owned.0 = gs.global.owned;
        svc_cell.borrow_mut().interact.sync_game(&gs);
        let classes_ref: &ClassTable = &classes;
        let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &rc_game::follow_camera::CameraView, coll: &collision::Collision, counter: u64| {
            let mut s = svc_cell.borrow_mut();
            let mut w = World::new(table, hero, rng, classes_ref, &mut s, counter);
            w.camera = cam.pos;
            w.coll = Some(coll);
            w.svc.interact.begin_tick(hero.loop_in.pad.pressed, hero.state);
            sched.tick(&mut w);
        };
        let mut parts = |_: &Hero, _: &rc_game::follow_camera::CameraView, _: &mut Rng, _: u64| {};
        let mut world = SharedServices { svc: &svc_cell, classes: classes.clone() };
        let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
        game.hero.idle.counter = game.counter as i32;
        game.tick(Some(&input.bytes()), &d.mesh, &mut anim.ctl(&d.ratchet), &mut hooks);
        let mut s = svc_cell.borrow_mut();
        s.interact.apply_writes(&mut gs);
        gs.global.bolts = s.counters.bolts;
        for h in std::mem::take(&mut s.interact.handoffs) {
            if let Handoff::OpenVendor { vendor: vm } = h {
                let mut o = VendorOut::default();
                v = Some((Vendor::open(vt.clone(), &gs, vm.is_none(), &mut o), f));
                out.sounds.push((f, o.sounds));
            }
            out.handoffs.push((f, h));
        }
        pad = game.pad.clone();
        out.recs.push((game.counter, 0, s.interact.prompt.owner, game.mobys.mobys[vendor].state, gs.global.bolts, -1));
    }
    out.state = gs;
    out
}

#[test]
fn novalis_vendor_prompt_open_buy_close() {
    let Some(d) = load() else { eprintln!("skipped: no extracted/"); return; };
    let r = run(&d, 400);
    eprintln!("talk slots {}; hand-offs {:?}; purchases {:?}; sounds {:?}", r.talk_slots, r.handoffs, r.purchases, r.sounds);
    // Novalis' talk slots: the ten gold-weapon offers and the Water Pump Worker.
    assert_eq!(r.talk_slots, 11);
    // The prompt: the vendor owns the lease from the first near check on (the class goes near on a tick with
    // counter & 7 = 0, then the rule holds), and the vendor goes near (state 2).
    let first = r.recs.iter().position(|x| x.2 == owner::VENDOR).expect("no prompt");
    assert!(first < 20, "prompt from frame {first}");
    assert!(r.recs[first..100].iter().all(|x| x.2 == owner::VENDOR && x.3 == 2), "prompt held until △");
    // △ at frame 100 → OpenVendorMenu, the vendor's state 3, mode 5 from the next frame.
    assert!(matches!(r.handoffs.as_slice(), [(100, Handoff::OpenVendor { vendor: Some(_) })]), "{:?}", r.handoffs);
    assert_eq!(r.recs[100].3, 3);
    assert_eq!(r.recs[101].1, 5);
    assert_eq!(r.sounds[0], (100, vec![3]), "open: sound 3");
    // Fly-in: 4 fade frames + 40 frames, then the screens (sound 4).
    assert!(r.sounds.iter().any(|(f, s)| *f == 101 + 4 + 39 && s == &vec![4]), "{:?}", r.sounds);
    // The Pyrocitor for 2500: bolts 5000 → 2500, owned, in quick select slot 1, the stock byte marked owned.
    assert_eq!(r.purchases.len(), 1, "{:?}", r.purchases);
    let (_, p) = r.purchases[0];
    assert_eq!(p, (16, false, 2500, 1));
    let g = &r.state.global;
    assert_eq!(g.bolts, 2500);
    assert_eq!(g.owned[16], 1);
    assert_eq!(g.quick_select[..2], [10, 16]);
    assert!(g.vendor.contains(&(16 | 0x40)), "{:?}", g.vendor);
    assert!(g.ammo[16] > 0, "the Pyrocitor comes with ammo");
    assert!(r.sounds.iter().any(|(_, s)| s.contains(&7)), "purchase sound 7");
    // △ leaves: 8 frames, then 40 (substate 2), VendorExit: the vendor's state 1, mode 0, the prompt again.
    let back = r.recs.iter().rposition(|x| x.1 == 5).unwrap() + 1;
    assert!(back < 400);
    assert_eq!(r.recs[back].1, 0);
    assert!(r.recs[back..].iter().any(|x| x.2 == owner::VENDOR), "the prompt returns after the exit");
    assert!(r.recs[back..].iter().all(|x| x.4 == 2500), "the bolts stay spent: the tick writes back the services' copy");
}

/// Determinism: two headless runs give the same per-frame records and the same end state.
#[test]
fn novalis_vendor_is_deterministic() {
    let Some(d) = load() else { eprintln!("skipped: no extracted/"); return; };
    let (a, b) = (run(&d, 300), run(&d, 300));
    assert_eq!(a.recs, b.recs);
    assert_eq!(a.purchases, b.purchases);
    assert_eq!(a.state.global.quick_select, b.state.global.quick_select);
}
