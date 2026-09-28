//! The weapons that act on creatures through the reaction layer (docs/plan/hero_gameplay.md §10), headless on
//! Novalis (and Rilgar for the interface): the Suck Cannon, the Taunter. Harness as `hero_guns_novalis.rs` (the level
//! as the engine ticks it; `GiveItem(id, equip)` on a first arrival's game state), plus the hit sink's moby world
//! (`HitSink::world`) the reaction calls run on. Every run twice: identical. Skipped when `extracted/` is absent; the
//! numbers are distilled from the runs themselves.

use rc_formats::moby_anim::{parse_sequence, parse_sequences, MobyAnimClass, MobySequence};
use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_formats::{collision, gadget, gameplay, level, moby_spawn};
use rc_game::game_state::{GameState, SessionState};
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::items::{ItemClass, ItemData, ItemDef, WeaponDef, HERO_LISTS};
use rc_game::hero::Hero;
use rc_game::moby_runtime::{mode, MobyTable, Seq0Info};
use rc_game::moby_update::scheduler::{self, class_info, load_level_mobys, Scheduler};
use rc_game::moby_update::services::{ServiceHits, SharedServices, World};
use rc_game::moby_update::{ClassTable, Services};
use rc_game::pad::{button, PadInput};
use rc_game::particles::Particles;
use rc_game::rng::Rng;
use rc_game::tick::{Game, GameOptions, TickHooks};
use std::sync::Arc;

struct Lv {
    level: u32,
    reactions: std::collections::HashMap<i16, rc_game::moby_update::creature::react::Table>,
    mesh: collision::Collision,
    instances: Vec<gameplay::MobyInstance>,
    pvars: Vec<Option<Vec<u8>>>,
    tests: Vec<moby_spawn::SpawnTest>,
    splines: Vec<Vec<[f32; 4]>>,
    gp: Vec<u8>,
    classes: ClassTable,
    spawnable: usize,
    death_z: f32,
    coll_blobs: Vec<(i32, rc_formats::moby_collision::MobyCollision)>,
    ratchet: MobyAnimClass,
    seconds: Vec<Vec<u8>>,
    items: ItemData,
    weapon_defs: Vec<WeaponDef>,
    tables: ItemTables,
    state: GameState,
    session: SessionState,
}

fn load() -> Option<Lv> { load_level(1) }

fn load_level(lvl: u32) -> Option<Lv> {
    let root = rc_formats::test_data::root();
    let dir = root.join(format!("levels/{lvl:02}"));
    let data = rc_formats::test_data::core_data(lvl)?;
    let idx = std::fs::read(dir.join("core_index.bin")).ok()?;
    let gp = rc_formats::test_data::gameplay(lvl)?;
    let settings = rc_formats::test_data::gameplay_section(lvl, "level_settings")?;
    let core = level::parse_level_core(&idx, data.len()).unwrap();
    let mesh = collision::parse_collision(&core, &data).unwrap();
    let instances = gameplay::parse_moby_instances(&gp).unwrap();
    let tests = moby_spawn::loader_spawns(&instances, &mut moby_spawn::SpawnSave::default());
    let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = gameplay::parse_pvars_spawned(&gp, &spawned).unwrap();
    let splines = gameplay::parse_splines(&gp).unwrap();
    let rd = |o: usize| i32::from_le_bytes(gp[o..o + 4].try_into().unwrap());
    let spawnable = rd(rd(0x44) as usize + 4) as usize;
    let death_z = f32::from_le_bytes(settings[0x28..0x2c].try_into().unwrap());
    let mut classes = ClassTable::default();
    for (slot, e) in core.moby_classes.iter().enumerate() {
        let oc = e.o_class as i16;
        let parsed = rc_formats::test_data::core_block(lvl, &format!("moby_class/{:04}", e.o_class)).and_then(|b| rc_formats::moby::parse_moby_class(&b).ok().map(|c| (b, c)));
        if let Some((blob, c)) = parsed {
            let anim = MobyAnimClass::new(&c, parse_sequences(&blob, &c).unwrap_or_default());
            let mut info = class_info(&c, slot as u8, scheduler::port_update_fn(oc));
            info.seq0 = anim.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
            classes.classes.insert(oc, (info, Some(anim)));
        } else {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: scheduler::port_update_fn(oc), ..Default::default() };
            classes.classes.entry(oc).or_insert((info, None));
        }
    }
    let coll_blobs = rc_formats::moby_collision::parse_level(&core, &data).unwrap();
    let ratchet_blob = rc_formats::test_data::core_block(lvl, "moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(&ratchet_blob).unwrap();
    let seqs: Vec<Option<MobySequence>> = (0..256)
        .map(|i| rc_formats::test_data::core_block(lvl, &format!("ratchet_seq/{i:03}")).and_then(|b| parse_sequence(&b, 0).ok()))
        .collect();
    let ratchet = MobyAnimClass::new(&class, seqs);
    let seconds = (0..64).map(|l| gadget::joint_list(&ratchet_blob, &class.header, l).map(|(_, b)| b).unwrap_or_default()).collect();
    let elf = std::fs::read(root.join("boot/SCUS_971.99")).ok()?;
    let ov0 = std::fs::read(root.join("levels/00/overlay.bin")).ok()?;
    let ov1 = std::fs::read(dir.join("overlay.bin")).ok()?;
    let tables = ItemTables::load(&elf, &ov1).unwrap();
    let sections = rc_formats::font::parse_overlay_sections(&ov1).unwrap();
    let (n, sz) = (rc_formats::save_game::ITEM_COUNT, rc_formats::save_game::ITEM_DEF_SIZE);
    let raw = rc_formats::font::read_overlay(&sections, tables.item_defs_addr, n * sz).unwrap().to_vec();
    let w = |i: usize, o: usize| i32::from_le_bytes(raw[i * sz + o..i * sz + o + 4].try_into().unwrap());
    let defs = (0..n).map(|i| ItemDef { slot: w(i, 8), attach: w(i, 0xc), o_class: w(i, 0x10), b18: raw[i * sz + 0x18] }).collect();
    let weapon_defs = (0..n).map(|i| WeaponDef { w18: w(i, 0x18), anims: [w(i, 0x24), w(i, 0x28), w(i, 0x2c)], w30: w(i, 0x30) }).collect();
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(&ratchet_blob, &class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let item_classes = gadget::parse_gadget_classes(&core, &data)
        .unwrap()
        .iter()
        .map(|g| {
            let c = &g.moby.class;
            let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
            ItemClass { o_class: g.moby.o_class as i16, anim: MobyAnimClass::new(c, parse_sequences(&g.blob, c).unwrap_or_default()), scale: c.header.scale, chains }
        })
        .collect();
    let items = ItemData { defs, hero_chains, classes: item_classes };
    let lump = SaveGameLump::parse(&std::fs::read(root.join("global/save_game.bin")).ok()?).unwrap();
    let ct = ChunkTables::from_boot_elf(&elf).unwrap();
    let mut state = GameState::new_game(ct, &lump.template).unwrap();
    let mut session = SessionState::default();
    state.apply_level_start(0, &ItemTables::load(&elf, &ov0).unwrap(), &mut session);
    state.on_veldin_clank_init(&mut session);
    state.apply_transition(lvl as i32);
    state.apply_level_start(lvl as i32, &tables, &mut session);
    let ov_target = rc_formats::level_overlay::LevelOverlay::parse(&ov1).ok()?;
    let ov_ref = rc_formats::level_overlay::LevelOverlay::parse(&std::fs::read(root.join("levels/01/overlay.bin")).ok()?).ok()?;
    let reactions = rc_game::moby_update::creature::react::tables_from_overlay(&ov_target, &ov_ref);
    Some(Lv { level: lvl, reactions, mesh, instances, pvars, tests, splines, gp: gp.to_vec(), classes, spawnable, death_z, coll_blobs, ratchet, seconds, items, weapon_defs, tables, state, session })
}

/// The hero's hit sink over the moby loop's services, as the engine's `CellHits` (with the guns' probe and class type).
struct CellHits<'a, 'b> {
    svc: &'a std::cell::RefCell<&'b mut Services>,
    classes: &'a ClassTable,
    coll: &'a collision::Collision,
    parts: &'a std::cell::RefCell<&'b mut Particles>,
}

impl CellHits<'_, '_> {
    fn with<R>(&mut self, f: impl FnOnce(&mut ServiceHits) -> R) -> R {
        let mut s = self.svc.borrow_mut();
        f(&mut ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) })
    }
}

type V4 = rc_game::hero::physics::V4;
type Tmpl = rc_game::moby_update::services::HitTemplate;

impl rc_game::hero::items::HitSink for CellHits<'_, '_> {
    fn sphere(&mut self, table: &mut MobyTable, r: rc_game::ps2v::Pf, centre: V4, flags: u32, ignore: Option<usize>, tmpl: &Tmpl) -> Option<usize> { self.with(|h| h.sphere(table, r, centre, flags, ignore, tmpl)) }
    fn line(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>, tmpl: &Tmpl) -> Option<Option<usize>> { self.with(|h| h.line(table, a, b, flags, ignore, tmpl)) }
    fn deliver(&mut self, table: &mut MobyTable, target: usize, tmpl: &Tmpl) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::deliver_hit_in(table, &mut s.hits, target, tmpl);
    }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<usize> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::create_from_hero(table, &mut s, self.classes, o_class, counter)
    }
    fn delete_moby(&mut self, table: &mut MobyTable, id: usize, counter: u64) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::delete_from_hero(table, &mut s, id, counter);
    }
    fn probe(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<Option<[f32; 3]>> { self.with(|h| h.probe(table, a, b, flags, ignore)) }
    fn probe_moby(&mut self, table: &mut MobyTable, a: V4, b: V4, flags: u32, ignore: Option<usize>) -> Option<Option<rc_game::hero::items::Probe>> { self.with(|h| h.probe_moby(table, a, b, flags, ignore)) }
    fn class_type(&self, o_class: i16) -> Option<u8> { use rc_game::moby_update::services::ClassData; self.classes.info(o_class).map(|i| i.ty) }
    fn light_alloc(&mut self, l: rc_game::point_lights::PointLight) -> i32 { self.with(|h| h.light_alloc(l)) }
    fn light_get(&mut self, slot: i32) -> Option<rc_game::point_lights::PointLight> { self.with(|h| h.light_get(slot)) }
    fn light_set(&mut self, slot: i32, l: rc_game::point_lights::PointLight) { self.with(|h| h.light_set(slot, l)) }
    fn light_free(&mut self, slot: i32) { self.with(|h| h.light_free(slot)) }
    fn world(&mut self, table: &mut MobyTable, hero: &Hero, rng: &mut Rng, counter: u64, f: &mut dyn FnMut(&mut World)) -> bool {
        let mut s = self.svc.borrow_mut();
        let mut p = self.parts.borrow_mut();
        let mut w = World::new(table, hero, rng, self.classes, &mut s, counter);
        w.coll = Some(self.coll);
        w.particles = Some(&mut **p);
        f(&mut w);
        true
    }
}

/// One tick of a run.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    state: i32,
    pos: [f32; 3],
    /// The hand item: id, its state (+0x20), its key B sequence.
    hand: (i32, u8, u8),
    out: u8,
    /// The watched mobys: (state, position, suck record state, mode).
    watched: Vec<(u8, [f32; 3], Option<i16>, u16)>,
    /// The reaction globals: held, coming, slots in use.
    suck: (i32, i32, usize),
    /// The creatures' damage-record lure (+0x18) of the watched mobys.
    lures: Vec<i32>,
    bursts: u32,
    fired: u32,
    lure_calls: u32,
    /// Hits in the log this tick for the watched mobys (a hit record present).
    hit: Vec<bool>,
    markers: Vec<(usize, u32)>,
    fp: u8,
    mouth: Option<[f32; 3]>,
    /// The vortex: drawn this tick, node strengths, live strands; the vacuum's takes.
    vortex: (bool, [i16; 10], usize),
    vacuumed: u32,
    /// The Taunter's rings drawn this tick and on; its whistle sounding.
    rings: (bool, usize),
    whistle: bool,
    rng: u32,
}

/// What a run gives and watches.
struct Setup<'a> {
    item: i32,
    at: ([f32; 3], f32),
    /// Moby instances to watch (their states).
    watch: &'a [usize],
    /// How long a hand-item class sound plays (the stand-in sound layer's `SoundIsAlive`; 0: no sound layer).
    sound_ticks: u64,
}

/// A stand-in sound layer: every hand-item class sound plays for `ticks` ticks (the Taunter's whistles, the Suck
/// Cannon's suction), so `SoundIsAlive` answers as an audio layer would; nothing is heard.
struct FakeSounds {
    ticks: u64,
    now: u64,
    started: Vec<u64>,
}

impl rc_game::hero::HeroSounds for FakeSounds {
    fn anim_advanced(&mut self, _: &rc_game::moby_runtime::Moby, _: &rc_game::hero::anim::AnimView, _: &rc_game::hero::anim::AnimView, _: &mut Rng) {}
    fn item_sound(&mut self, _: i16, _: [f32; 3], _: i32, _: u32, _: &mut Rng) -> i32 {
        if self.ticks == 0 { return -1; }
        self.started.push(self.now);
        self.started.len() as i32 - 1
    }
    fn alive(&mut self, slot: i32) -> bool { self.started.get(slot as usize).is_some_and(|&t| self.now < t + self.ticks) }
}

fn give(lv: &Lv, item: i32) -> (GameState, SessionState) {
    let (mut gs, mut session) = (lv.state.clone(), lv.session);
    gs.give_item(item as usize, true, &lv.tables, &mut session);
    (gs, session)
}

fn run(lv: &Lv, s: &Setup, input: &dyn Fn(u32) -> PadInput, ticks: u32) -> Vec<Row> {
    let (gs, session) = give(lv, s.item);
    let classes = Arc::new(ClassTable { classes: lv.classes.classes.clone() });
    let mut ct = ClassTable { classes: lv.classes.classes.clone() };
    let statics = load_level_mobys(&lv.instances, &mut ct, &lv.pvars, &lv.tests);
    let watch: Vec<usize> = s.watch.iter().map(|w| statics.moby_to_instance.iter().position(|&i| i == *w).expect("watched moby")).collect();
    let mut mobys = statics.mobys.clone();
    let hero_idx = mobys.iter().position(|m| m.o_class == 0).expect("Ratchet");
    let (p, yaw) = s.at;
    mobys[hero_idx].position = [p[0], p[1], p[2], 1.0];
    mobys[hero_idx].rotation = [0.0, 0.0, yaw, 0.0];
    let mut table = MobyTable::new(mobys, lv.spawnable);
    table.mobys[hero_idx].mode |= mode::NO_UPDATE;
    let mut game = Game::new(&lv.mesh, table, hero_idx, GameOptions::default(), lv.death_z);
    game.hero.idle.level = lv.level as _;
    game.hero.set_joint_targets(&lv.seconds);
    game.item_data = Some(lv.items.clone());
    game.hero.weapons.defs = lv.weapon_defs.clone();
    game.hero.owned.0 = gs.global.owned;
    game.hero.weapons.ammo = gs.global.ammo;
    for (i, r) in lv.tables.records.iter().enumerate() { game.hero.weapons.uses_ammo[i] = r.has_ammo(); }
    game.item_globals.wrench_flag = 1;
    game.item_globals.request = session.temp_hand;
    game.item_globals.saved = session.temp_hand;
    let mut svc = Services::new();
    svc.level = lv.level;
    svc.creatures.react.tables = lv.reactions.clone();
    svc.set_splines(&lv.splines);
    svc.groups = statics.groups(&lv.gp);
    svc.set_moby_collision(lv.coll_blobs.clone());
    svc.build_grid(&mut game.mobys);
    let mut particles = Particles::new(None, Vec::new());
    let mut sched = Scheduler::new();
    {
        let hero: Hero = game.hero.clone();
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &*classes, &mut svc, 0);
        w.camera = game.camera.out.pos;
        w.coll = Some(&lv.mesh);
        w.particles = Some(&mut particles);
        sched.load_pass(&mut w);
    }
    game.finish_load();
    let mut anim = RatchetAnim::new(&lv.ratchet);
    anim.arm_joints = rc_game::hero::weapons::ARM_LISTS.map(|l| lv.seconds[l as usize].clone());
    let svc_cell = std::cell::RefCell::new(&mut svc);
    let parts_cell = std::cell::RefCell::new(&mut particles);
    let mut rows = Vec::new();
    let mut fake = FakeSounds { ticks: s.sound_ticks, now: 0, started: Vec::new() };
    for t in 0..ticks {
        let classes_ref: &ClassTable = &classes;
        let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &rc_game::follow_camera::CameraView, coll: &collision::Collision, counter: u64| {
            let mut s = svc_cell.borrow_mut();
            let mut p = parts_cell.borrow_mut();
            let mut w = World::new(table, hero, rng, classes_ref, &mut s, counter);
            w.camera = cam.pos;
            w.coll = Some(coll);
            w.particles = Some(&mut **p);
            sched.tick(&mut w);
        };
        let mut parts = |hero: &Hero, _: &rc_game::follow_camera::CameraView, rng: &mut Rng, _: u64| {
            let mut p = parts_cell.borrow_mut();
            rc_game::hero::fx::create_particles(hero, &mut p);
            p.update_parts(rng);
        };
        let mut world = SharedServices { svc: &svc_cell, classes: classes.clone() };
        let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
        game.hero.idle.counter = game.counter as i32;
        // The render's +0x31 ("drawn last frame", MobyProc's; the R.Y.N.O.'s search reads it): the watched mobys are
        // in view in these runs.
        for &i in &watch { if game.mobys.mobys[i].state < 0x80 { game.mobys.mobys[i].visible = 1; } }
        let mut hits = CellHits { svc: &svc_cell, classes: &classes, coll: &lv.mesh, parts: &parts_cell };
        fake.now = game.counter;
        let r = game.tick_with_hero_sounds(Some(&input(t).bytes()), &lv.mesh, &mut anim.ctl(&lv.ratchet), &mut hooks, &mut hits, None, &mut fake);
        assert_eq!(r.hero, rc_game::hero::HeroTick::Ran, "hero stopped in state {:#x} at tick {t}", game.hero.state);
        let h = &game.hero;
        let tick = game.counter - 1;
        let svc = svc_cell.borrow();
        let g = &svc.creatures.react;
        let rec_state = |m: &rc_game::moby_runtime::Moby| -> Option<i16> {
            let r = match m.o_class { 577 => 0x60, 572 | 866 => 0xc0, 270 => 0xe0, _ => return None };
            (m.pvars.len() >= r + 0x6a).then(|| i16::from_le_bytes([m.pvars[r + 0x68], m.pvars[r + 0x69]]))
        };
        rows.push(Row {
            state: h.state,
            pos: h.position(),
            hand: h.items.slot.item.as_ref().map_or((h.items.slot.id, 0xff, 0xff), |m| (h.items.slot.id, m.mstate, m.anim.seq_b)),
            out: h.f13f8,
            watched: watch.iter().map(|&i| { let m = &game.mobys.mobys[i]; (m.state, [m.position[0], m.position[1], m.position[2]], rec_state(m), m.mode) }).collect(),
            suck: (g.held, g.coming, g.slots.iter().filter(|&&s| s != 0).count()),
            lures: watch.iter().map(|&i| { let m = &game.mobys.mobys[i]; if m.pvars.len() >= 0x3c { i32::from_le_bytes(m.pvars[0x38..0x3c].try_into().unwrap()) } else { 0 } }).collect(),
            bursts: g.bursts,
            fired: h.weapons.reactive.suck.fired,
            lure_calls: h.weapons.reactive.taunter.lures,
            hit: watch.iter().map(|&i| game.mobys.mobys[i].hit_slot != 0xff).collect(),
            markers: h.weapons.markers.of_tick(tick).iter().map(|m| (m.fx, m.rgba)).collect(),
            fp: h.f13f5,
            mouth: g.cannon.map(|c| c.mouth),
            vortex: (h.weapons.reactive.suck.vortex.drawn == Some(tick), h.weapons.reactive.suck.vortex.alpha, h.weapons.reactive.suck.vortex.strands.iter().filter(|x| x.part.is_some()).count()),
            vacuumed: h.weapons.reactive.suck.vacuumed,
            rings: (h.weapons.reactive.taunter.rings.drawn == Some(tick), h.weapons.reactive.taunter.rings.rings.iter().filter(|r| r.on).count()),
            whistle: h.fx.item_loop_alive[rc_game::hero::fx::LOOP_ITEM],
            rng: game.rng.state,
        });
    }
    rows
}

/// Crate 376 (class 500 below the spawn plateau).
const CRATE: usize = 376;

/// The critters 577 in the pit north-west of the spawn (instances of the table's mobys 590 / 592), and a place `d`
/// units from `id`'s moby in direction `from`, facing it.
fn facing_moby(lv: &Lv, inst: usize, d: f32, from: f32) -> ([f32; 3], f32) {
    let c = lv.instances[inst].position;
    let p = [c[0] + d * from.cos(), c[1] + d * from.sin(), c[2] + 0.5];
    (p, (c[1] - p[1]).atan2(c[0] - p[0]))
}

/// The instance of the table's moby `id` (the moby loader's order).
fn instance_of(lv: &Lv, id: usize) -> usize {
    let mut ct = ClassTable { classes: lv.classes.classes.clone() };
    let statics = load_level_mobys(&lv.instances, &mut ct, &lv.pvars, &lv.tests);
    statics.moby_to_instance[id]
}




fn dump(rows: &[Row]) {
    if std::env::var("RC_REACT_DUMP").is_err() { return; }
    for (t, r) in rows.iter().enumerate() { eprintln!("{t:4} {r:?}"); }
}


// ------------------------------------------------------------------------------------------------------------------
// The Suck Cannon

const HIDDEN: u16 = rc_game::moby_runtime::mode::HIDDEN;

/// The distinct values of `f` over the rows, in order of first appearance.
fn seen<T: PartialEq + Copy>(rows: &[Row], f: impl Fn(&Row) -> T) -> Vec<T> {
    let mut v = Vec::new();
    for r in rows { let x = f(r); if v.last() != Some(&x) && !v.contains(&x) { v.push(x); } }
    v
}

fn suck_then_fire(t: u32) -> PadInput {
    if (40..=200).contains(&t) || (260..=262).contains(&t) || (330..=336).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() }
}

/// Novalis, the critters 577 in the pit: ○ held pulls the one in the cone through the reaction table (its suck record
/// 1 approach → 2 rise → 3 pulled → 4 swallowed, the class in its held state 7) during the first hold, hides it and
/// counts it held; the next press (after the release) fires it (record 5), and it bursts where it hits (the burst,
/// deleted).
#[test]
fn novalis_suck_cannon_pulls_and_fires_a_critter() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[9].o_class, 849, "item 9 is the Suck Cannon (class 849)");
    assert_eq!(lv.reactions.get(&577), Some(&rc_game::moby_update::creature::react::Table::Critter));
    let (a, b) = (instance_of(&lv, 590), instance_of(&lv, 592));
    let at = facing_moby(&lv, b, 6.0, std::f32::consts::PI);
    let s = Setup { item: 9, at, watch: &[a, b], sound_ticks: 0 };
    let rows = run(&lv, &s, &suck_then_fire, 480);
    dump(&rows);
    let held = rows.iter().position(|r| r.suck.0 == 1).expect("nothing swallowed");
    let k = rows[held].watched.iter().position(|w| w.2 == Some(4)).expect("which one");
    let recs = seen(&rows[..held + 1], |r| r.watched[k].2);
    let order: Vec<i16> = recs.iter().flatten().copied().filter(|&x| x != 0 && x != 7).collect();
    assert_eq!(order, [1, 2, 3, 4], "record states {recs:?}");
    assert_eq!(rows[held].watched[k].0, 7, "577 in its held state 7");
    // Swallowed at scale 0; its next carried tick hides it and stops its update (mode 1 | 2).
    assert!(rows[held + 1].watched[k].3 & (HIDDEN | 2) == HIDDEN | 2, "hidden inside the cannon");
    assert_eq!(rows[held].suck, (1, 0, 1));
    let fired = rows.iter().position(|r| r.fired == 1).expect("never fired");
    assert!(held < fired && (260..265).contains(&fired), "fired at {fired} by the press at 260");
    assert_eq!(rows[fired].suck, (0, 0, 0), "the slot emptied");
    assert!(rows[fired].watched[k].2.is_some_and(|x| x >= 5), "record {:?}", rows[fired].watched[k].2);
    let burst = rows.iter().position(|r| r.bursts == 1).expect("no burst");
    assert!(rows[burst].watched[k].0 >= 0xfd, "deleted after the burst");
    assert_eq!(rows, run(&lv, &s, &suck_then_fire, 480), "deterministic");
}

/// Rilgar (level 05), a small amoeboid 866 awake by the river: the same interface through the amoeboids' table (held
/// state 0xe, record at +0xc0).
#[test]
fn rilgar_suck_cannon_takes_a_small_amoeboid() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.reactions.get(&866), Some(&rc_game::moby_update::creature::react::Table::Amoeboid));
    assert_eq!(lv.reactions.get(&865), None, "865 keeps the default table");
    let at = facing_moby(&lv, 1262, 5.0, -std::f32::consts::FRAC_PI_2);
    let s = Setup { item: 9, at, watch: &[1262], sound_ticks: 0 };
    let input = |t: u32| if (30..=160).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 200);
    dump(&rows);
    let held = rows.iter().position(|r| r.suck.0 == 1).expect("the amoeboid was not swallowed");
    assert_eq!((rows[held].watched[0].0, rows[held].watched[0].2), (0xe, Some(4)));
    assert!(seen(&rows, |r| r.watched[0].2).contains(&Some(3)), "pulled");
    assert_eq!(rows, run(&lv, &s, &input, 200), "deterministic");
}

// ------------------------------------------------------------------------------------------------------------------
// The Taunter

/// Novalis, the critters in the pit: ○ whistles (the item's sequence 3) and lures the critters in front (their damage
/// record's +0x18 = the lure, which each clears on its next update after turning it into its alert).
#[test]
fn novalis_taunter_lures_the_critters() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    assert_eq!(lv.items.defs[14].o_class, 175, "item 14 is the Taunter (class 175)");
    let (a, b) = (instance_of(&lv, 590), instance_of(&lv, 592));
    let at = facing_moby(&lv, b, 6.0, std::f32::consts::PI);
    let s = Setup { item: 14, at, watch: &[a, b], sound_ticks: 0 };
    let input = |t: u32| if t == 60 { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 120);
    dump(&rows);
    let t = rows.iter().position(|r| r.lure_calls > 0).expect("no lure");
    assert!((60..63).contains(&t), "whistle at {t}");
    assert!(rows[t].lures.iter().all(|&l| l != 0), "both critters lured: {:?}", rows[t].lures);
    assert!(rows[t + 1].lures.iter().all(|&l| l == 0), "read and cleared by the critters");
    assert_eq!(rows[t].hand.2, 3, "the whistle sequence 3");
    assert_eq!(rows, run(&lv, &s, &input, 120), "deterministic");
}

/// The crates below the spawn plateau (the table's mobys 375, 377, 376 (class 500) and 533 (501)), all within 12 in
/// front of Ratchet: the whistle counts them and knocks the first (`randi(max(count of the last whistle, 1))` = 0 on the
/// first one): a hit record of damage 1 that breaks it.
#[test]
fn novalis_taunter_knocks_a_crate() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let c = lv.instances[CRATE].position;
    let at = ([c[0], c[1] + 6.0, c[2]], -std::f32::consts::FRAC_PI_2);
    let crates: Vec<usize> = [375, 377, 376, 533].iter().map(|&m| instance_of(&lv, m)).collect();
    let s = Setup { item: 14, at, watch: &crates, sound_ticks: 0 };
    let input = |t: u32| if t == 60 { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 120);
    dump(&rows);
    let t = rows.iter().position(|r| r.lure_calls > 0).expect("no whistle");
    assert_eq!(rows[t].hit, [true, false, false, false], "the first crate in the list got the whistle's hit");
    assert!(rows[t + 10].watched[0].0 != rows[0].watched[0].0, "it broke: {:?}", rows[t + 10].watched[0]);
    assert!(rows[t + 10].watched[1..].iter().zip(&rows[0].watched[1..]).all(|(a, b)| a.0 == b.0), "the others stand");
    assert_eq!(rows, run(&lv, &s, &input, 120), "deterministic");
}

// ------------------------------------------------------------------------------------------------------------------
// Holding (the user's report: "it sucks once for a millisecond")

/// Novalis, the pit: ○ held for 360 ticks keeps the cannon sucking (state 3) the whole time, the vortex drawn every
/// tick with its strands (type-23 puffs) running down it; released, it collapses and fades out.
#[test]
fn novalis_suck_cannon_holds_continuously() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let (a, b) = (instance_of(&lv, 590), instance_of(&lv, 592));
    let at = facing_moby(&lv, b, 6.0, std::f32::consts::PI);
    let s = Setup { item: 9, at, watch: &[a, b], sound_ticks: 0 };
    let input = |t: u32| if (40..=400).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 460);
    dump(&rows);
    let start = rows.iter().position(|r| r.hand.1 == 3).expect("never sucking");
    assert!(start < 70, "sucking from {start}");
    assert!(rows[start..=400].iter().all(|r| r.hand.1 == 3), "the whole hold in state 3");
    // The first state-3 row is the tick that entered it (state 2's update ran); from the next the vortex every tick.
    assert!(rows[start + 1..=400].iter().all(|r| r.vortex.0), "the vortex drawn every tick of the hold");
    assert!(rows[start + 20..=400].iter().all(|r| r.vortex.1[0] >= 0x40), "node 0 at full strength");
    assert!(rows[start + 40..=400].iter().all(|r| r.vortex.2 > 0), "strands running");
    let most = rows.iter().map(|r| r.vortex.2).max().unwrap();
    assert!(most >= 20, "strands at most {most}");
    assert!(rows[400].vacuumed > 0, "the vacuum took the swallowed critters' bolts");
    // Released: the fade (−4 a tick from 64) ends the draw within 20 ticks.
    assert!(rows[401].vortex.0 && !rows[430].vortex.0, "fades out");
    assert_eq!(rows, run(&lv, &s, &input, 460), "deterministic");
}

/// Rilgar, two small amoeboids in a line in front of Ratchet: with ○ held the one 14.2 away (the reach is 15) is
/// pulled and swallowed first, then the nearer one: held 1, then 2, the cannon sucking throughout.
#[test]
fn rilgar_suck_cannon_far_reach_and_two_in_sequence() {
    let Some(lv) = load_level(5) else { eprintln!("skipped: no extracted/"); return };
    let at = ([167.55005, 305.2638, 26.5], (319.1 - 305.26f32).atan2(164.2 - 167.55));
    let s = Setup { item: 9, at, watch: &[1261, 1266], sound_ticks: 0 };
    let input = |t: u32| if (30..=300).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 320);
    dump(&rows);
    let first = rows.iter().position(|r| r.watched[1].2.is_some_and(|x| x != 0)).expect("the far one was not taken");
    let d = {
        let (p, h) = (rows[first].watched[1].1, rows[first].pos);
        ((p[0] - h[0]).powi(2) + (p[1] - h[1]).powi(2)).sqrt()
    };
    assert!(d > 13.0, "taken at {d} from Ratchet");
    let one = rows.iter().position(|r| r.suck.0 == 1).expect("none swallowed");
    let two = rows.iter().position(|r| r.suck.0 == 2).expect("the second was not swallowed");
    assert!(one < two);
    assert_eq!(rows[one].watched[1].2, Some(4), "the far one first");
    assert_eq!(rows[two].watched[0].2, Some(4));
    assert!(rows[one..two].iter().all(|r| r.hand.1 == 3), "sucking between the two");
    assert_eq!(rows, run(&lv, &s, &input, 320), "deterministic");
}

/// The Taunter held for 240 ticks (the stand-in sound layer plays each whistle for 90): whistle after whistle, the
/// lure every 4th frame while one sounds, the rings flowing while the whistle sequence plays — for the whole hold.
#[test]
fn novalis_taunter_lures_for_the_whole_hold() {
    let Some(lv) = load() else { eprintln!("skipped: no extracted/"); return };
    let (a, b) = (instance_of(&lv, 590), instance_of(&lv, 592));
    let at = facing_moby(&lv, b, 6.0, std::f32::consts::PI);
    let s = Setup { item: 14, at, watch: &[a, b], sound_ticks: 90 };
    let input = |t: u32| if (60..=300).contains(&t) { PadInput::neutral().press(button::CIRCLE) } else { PadInput::neutral() };
    let rows = run(&lv, &s, &input, 340);
    dump(&rows);
    let first = rows.iter().position(|r| r.lure_calls > 0).expect("no whistle");
    assert!((60..64).contains(&first), "first whistle at {first}");
    // Lures in every 30-tick window of the hold.
    for w in (first..300 - 30).step_by(30) {
        assert!(rows[w + 30].lure_calls > rows[w].lure_calls, "no lure in {w}..{}", w + 30);
    }
    let sounding = rows[first..=300].iter().filter(|r| r.whistle).count();
    assert!(sounding > 200, "the whistles sound {sounding} of the 240 ticks");
    assert!(rows[first + 5..first + 60].iter().all(|r| r.rings.0 && r.rings.1 > 0), "rings out");
    let most = rows[first..first + 60].iter().map(|r| r.rings.1).max().unwrap();
    assert_eq!(most, 3, "a ring every 10 ticks, 30 ticks each");
    assert_eq!(rows, run(&lv, &s, &input, 340), "deterministic");
}
