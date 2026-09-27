//! The game tick in the engine: the ported on-foot hero, pad and follow camera (`rc_game::tick`,
//! docs/plan/player_controller.md §10 and "Engine wiring") driving Ratchet's instance and the view.
//!
//! * **Load** (`setup`, first `PreUpdate`), the level loader and load pass of `rc_game::moby_update`
//!   (docs/plan/moby_update_catalogue.md "In the port"): the class table (slot = index in the level core's
//!   class list, update function = the Rust port's level-table address or an external update's), the loader's
//!   **spawn test** on every instance (`rc_formats::moby_spawn::loader_spawns` with the level's save bytes from
//!   [`Persistent`]: mission bytes, killed bits; the rejected instances are not created and their entities are
//!   hidden), the static mobys from `scheduler::load_level_mobys` (the created instances in order: moby index =
//!   `0x1acc00[instance]`, pvar moby links remapped), `MobyTable::new(statics, spawnable count)` (gameplay moby
//!   section +4), the ship via `CreateMoby` in the first dynamic slot (hidden, mode |= 3, while the mission NPC's
//!   mission is open: the first arrival); `Game::new` (**`srand(1234)`**, hero init: Ratchet ground-snapped,
//!   mode 2, the fidget-timer draw; the camera snapped behind); the services (level, splines, the groups of
//!   gameplay +0x48 through instance → moby); then the **load pass** `Scheduler::load_pass` at counter 0 on the
//!   game's stream (the class-27 emitters see no view: culled, as the game's zero view before the first render),
//!   and the load's `0x15f5cc++` (`Game::finish_load`: the first tick runs at counter 1): the ported classes'
//!   state-0 inits (bolts, crates, grass), the external updates (class-27 emitters 0x2bd100 of
//!   crate::particle_render, water 751 0x2fd0e8 of crate::water_render), one `MobyAnimAdvance` of every
//!   non-mode-2 moby. crate::moby_spawn stays the source for the unported classes only (459, 572-family, 577,
//!   666, 730/790, 750, 1818, the ship): per moby the anim / position / mode come either from the table (a
//!   class with a ported update) or from its `SpawnState`, never both. Ratchet's animation is `RatchetAnim`
//!   on his class (sequences = the level's `ratchet_seq` table).
//! * **Tick** (`FixedUpdate`, 60 Hz): `Game::tick_with_sound` = pad → free-slot pass → mobys (hook: `Scheduler::tick`
//!   with the game camera of the previous tick, the level collision, the moby collision of `Services`, the
//!   particles, the render view of the last frame for `FastBSphereCheck`, the external updates) → hero (its
//!   queries test the mobys: `TickHooks::world` = `SharedServices`, docs/plan/collision_queries.md §7; then
//!   `MobyBuildMatrix` on Ratchet, re-registering him in the moby grid) → particles (hook: `UpdateParts` on the
//!   game's stream with the tick's camera as 0x167240, then the glints `FUN_00220928`) → camera (its queries
//!   test the mobys too; a crate blocking its line gets a hit) → sound step → counter; the sky stars draw
//!   after it (render phase). **One `rand` stream** (`Game::rng`): load pass → per tick the mobys in run order
//!   (incl. the 751 drops, the class-27 spawns and the class sounds' pitch bends) → hero → particles → camera →
//!   sound → counter → sky stars. Ticks per
//!   rendered frame follow the main loop's rule (`rc_game::tick::ticks_for_frame`: at most 2, the second only
//!   when the frame took longer than one field); in frame-exact mode exactly 1 per update.
//! * **Moby rendering after each tick.** Static instances of a ported class: position, rows, scale, light
//!   block (for the new rows / ambient) and hide (deleted or mode & 0x81) go to their `MobyInst` record and
//!   MobyProc inputs (`MobyOcclusion::drive`), the anim state and snapshot to their `MobyAnim` instance
//!   (`MobyAnim::drive`, never advanced there). Mobys the scheduler creates (dropped bolts, debris, flashes,
//!   icons) live in the dynamic slots and are drawn as extra instances ([`DynMobys`]: one record and palette
//!   range per dynamic slot, entities spawned per (slot, class) on first use and shown / hidden by the slot's
//!   state and MobyProc's draw-distance / near / frustum culls; high LOD, no fade). +0x31 is written back
//!   after drawing (`write_visible`): 1 when drawn, 0 on every skip (occlusion, culls, hidden, spawn-hidden).
//!   The moby sounds go to `rc_game::audio` through the moby loop's `SoundSink`
//!   (`audio::class_sounds::ClassSoundSink`: slot and pitch-bend draw at the moby's place in the loop, listener =
//!   the previous tick's camera) and are also counted per (class, index). The bolt counter goes to
//!   [`Persistent`] after each tick.
//! * **Sound step** (crate::audio_out): after the camera, before the counter increment, on the game's stream
//!   (`Game::tick_with_hero_sounds`, `audio::class_sounds::sound_step`): `sound_update` (the occlusion origin's 3
//!   draws every tick, the pitch bends of the sound instances' plays) and the frame's 800 samples. Ratchet's own
//!   sounds play inside the hero update (`class_sounds::HeroClassSounds`: his animation triggers right after his
//!   advance, his hurt / death voices after the transitions). Without audio (`RC_AUDIO=0`, or no sound data) the
//!   tick runs without a sound layer.
//! * **Idle and back items**: the hero gets the back items' classes (pack 607 and Clank 601,
//!   `Hero::set_back_classes`: created on the first hero update, then advanced and driven by the idle code:
//!   the back table, Clank's fidgets and blink), the level (`idle.level`, 0x15ed84) and, before every tick,
//!   the tick counter (`idle.counter`, 0x15f5cc); crate::moby_attach draws the back items from `Hero::back`.
//! * **After each tick**: Ratchet's moby (+0x10 position, +0xc0 rows, written back by the hero) is drawn
//!   by an extra moby instance of his class (crate::moby_render "Extra" block: a record + palette of its
//!   own, re-lit with his light word / ambient for the new rows); his gameplay-instance entities are hidden
//!   (`SpawnHidden`), since the static record cannot move and the static occlusion word of his spawn cell
//!   would hide him elsewhere. His `RatchetAnim` state and snapshot are written into his `MobyAnim` instance
//!   (with `skip_advance`, so the generic `MobyAnimAdvance` never touches him: mode 2), and his rows /
//!   position into crate::moby_attach, which then places the wrench, pack and Clank on the new pose (the
//!   pack's and Clank's animation from `Hero::back`).
//!   The camera goes to crate::play_camera.
//! * **Hits and death** (`rc_game::hero::damage`, docs/plan/hero_states.md P2): the tick hands Ratchet's hit
//!   message to the hero (the moby hit log, `MobySystem::hit_message`); after the tick the damage events go to
//!   the game state (hits 0x15eea8 / 0x13df88[level], deaths 0x15eeac / 0x13dfd8[level], the killer's mission
//!   deaths `LevelMissions::hero_death`). **Respawn on the game's death flag** 0x141401 (`Hero::fell_out`, raised
//!   by the death sequence 0x2319b0 at the end of every death state, or by x/y outside 2..1022), never on
//!   entering a state: no catch-up tick after it, then the death reload's hero side — the hero init (HP = max
//!   HP), at the checkpoint record (class 805's `0x29ac10`: position and Euler, camera snapped behind) when one was
//!   reached, else at the level's uid-0 moby. The level's mobys are not reloaded (the game's `LoadLevelCoreData(0,
//!   1)` is not reproduced); tick counter and RNG continue. `R` respawns on demand (e.g. out of a frozen
//!   unported state).
//!
//! * **Game state** (both modes, before the app runs): the persistent state and the session of a direct
//!   boot into this level, from the port of docs/plan/game_state.md (`rc_game::game_state`): new game from
//!   the disc's save template, Veldin's level start and Clank init, then for level N ≥ 1 the transition and
//!   N's level start (Novalis: HP 4/4, wrench held, bomb glove owned with 10 ammo, quick select [10, 0, …]).
//!   Published as the [`Persistent`] and [`Session`] resources (menus, HUD, scheduler); the tick reads its
//!   options (`Options::game_options`: mirror, camera yaw/pitch sense and rate), the mirrored-animation
//!   option and the HP (hero +0x15f8). Respawns redo the hero init (`SessionState::hero_init`).
//!
//! Environment: `RC_PLAY=0` disables all of this (fly camera only); `RC_PLAY_SCRIPT` feeds scripted pad input
//! (crate::input_map::Script); `RC_PLAY_TRACE=1` prints one line per tick (state, position, anim, camera, the
//! `rand` state, the moby loop's run count, bolts, free slots, dynamic mobys drawn / live, particles, grass on
//! sequence 1); `RC_DEBUG_HIT=moby@tick,...` delivers a hit (flags 0x10000, damage 1: breaks a crate) to those
//! mobys before the moby loop of those ticks (debug; the wrench itself now hits through the hero's hit sink);
//! `hero@tick` hits Ratchet (flags 1, damage 1, no attacker: the hit intake's knockback straight back, one HP).
//! The trace line ends with `| hp <health> inv <0x13f510>`.
//! * **Hand items and melee** (docs/plan/player_controller.md §12): the tick runs with `Game::item_data` (item
//!   definitions from the overlay's item table, the gadget classes, Ratchet's joint lists), the hand-swap globals
//!   synced with `Session::temp_hand` (the quick-select request) and `Persistent` (`equipped[0]`, `last_hand_item`,
//!   `wrench_held`), and a hit sink on the services' hit log ([`CellHits`]); after the tick the HUD's weapon slot
//!   ([`HeldWeapon`]) is derived from the held item.
//!
//! `RC_PLAY_FLY=1` starts on the fly camera (`RC_CAM`) with the game ticking (Tab switches as usual).

use crate::determinism::Deterministic;
use crate::fly_cam::FlyCam;
use crate::game_camera::CameraSource;
use crate::input_map::{self, PadFrame, Script};
use crate::moby_anim::MobyAnim;
use crate::moby_attach::{AttachedTo, MobyAttach};
use crate::moby_render::{self, ExtraMobys, MobyMaterial, MobyOcclusion};
use crate::moby_spawn::{MobySpawn, SpawnHidden};
use crate::particle_render::ParticleSim;
use crate::play_camera::{self, PlayView};
use crate::water_render::WaterState;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::collision::Collision;
use rc_formats::save_game::{ChunkTables, ItemTables, SaveGameLump};
use rc_game::game_state::{GameState, SessionState};
use rc_formats::moby_anim::{self, MobyAnimClass};
use rc_formats::moby_light::{self as light, V4};
use rc_game::follow_camera::CameraView;
use rc_game::hero::anim::RatchetAnim;
use rc_game::hero::damage::DamageEvent;
use rc_game::hero::items::{HitSink, ItemClass, ItemData, ItemDef, ItemGlobals, HERO_LISTS};
use rc_game::audio::class_sounds::{self, ClassSoundSink, HeroClassSounds};
use rc_game::hero::{Hero, HeroTick};
use rc_game::moby_runtime::{Moby, MobyId, MobyTable, Seq0Info};
use rc_game::moby_update::scheduler::{self, Scheduler};
use rc_game::moby_update::services::{ExternalUpdates, HitTemplate, LevelMissions, ServiceHits, SharedServices, World};
use rc_game::moby_update::{ClassData, ClassTable, Services};
use rc_game::pad::PadInput;
use rc_game::particles::{type06, BSphereView, Particles};
use rc_game::rng::Rng;
use rc_game::tick::{ticks_for_frame, Game, GameOptions, TickHooks, FIELD_RCNT1};
use std::cell::RefCell;
use std::collections::HashMap;

/// Level-table address of the class-27 emitter update (Novalis only), crate::particle_render.
const EMITTER_UPDATE: u32 = 0x2bd100;
/// Level-table address of the ripple manager 751's update, crate::water_render.
const RIPPLE_UPDATE: u32 = 0x2fd0e8;

/// `RC_GIVE_ITEMS=<id>,...` (debug): the item ids to own from the start (decimal or `0x` hex; ids outside the
/// item table are ignored).
fn give_items() -> Option<Vec<usize>> {
    let v = std::env::var("RC_GIVE_ITEMS").ok()?;
    let ids: Vec<usize> = v
        .split(',')
        .filter_map(|t| { let t = t.trim(); t.strip_prefix("0x").map_or_else(|| t.parse().ok(), |h| usize::from_str_radix(h, 16).ok()) })
        .filter(|&i| i < rc_formats::save_game::ITEM_COUNT)
        .collect();
    (!ids.is_empty()).then_some(ids)
}

/// `RC_PLAY` (default on; `0` = fly camera only).
pub fn enabled() -> bool { !std::env::var("RC_PLAY").is_ok_and(|v| v.trim() == "0") }

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
        match load_game_state(&root, index) {
            Ok((mut gs, sess)) => {
                // RC_GIVE_HYDROPACK=1: own the Hydro-Pack (item 4) from the start (debug).
                if std::env::var("RC_GIVE_HYDROPACK").is_ok_and(|v| v.trim() == "1") {
                    gs.global.owned[rc_game::hero::swim::ITEM_HYDRO_PACK] = 1;
                    println!("game state: RC_GIVE_HYDROPACK=1: Hydro-Pack owned");
                }
                // RC_GIVE_ITEMS=<id>,<id>,...: own those items (decimal or 0x hex ids, docs/plan/hero_states.md §0.1)
                // from the start (debug); the last back item among them (2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack)
                // is the saved back item (equipped[3], 0x14166c), so Clank wears it.
                // RC_GIVE_BOLTS=<n>: start with n bolts (debug; e.g. to buy at the vendor).
                if let Some(n) = std::env::var("RC_GIVE_BOLTS").ok().and_then(|v| v.trim().parse::<i32>().ok()) {
                    gs.global.bolts = n;
                    println!("game state: RC_GIVE_BOLTS: {n} bolts");
                }
                if let Some(ids) = give_items() {
                    for &id in &ids { gs.global.owned[id] = 1; }
                    if let Some(&b) = ids.iter().rev().find(|&&i| matches!(i, 2..=4)) { gs.global.equipped[3] = b as i32; }
                    println!("game state: RC_GIVE_ITEMS: items {ids:?} owned, back item {}", gs.global.equipped[3]);
                }
                let g = &gs.global;
                println!(
                    "game state: direct boot into level {index:02} (new game → Veldin start → transition): level {}, HP {}/{}, wrench held {}, \
                     equipped {:?}, quick select {:?}, bolts {}; options {:?}",
                    g.level, sess.hp, g.max_hp, g.wrench_held, &g.equipped[..1], g.quick_select, g.bolts, gs.options()
                );
                app.insert_resource(Persistent(gs)).insert_resource(Session(sess));
            }
            Err(e) => eprintln!("game state: not built ({e:#}); the tick uses the boot option defaults"),
        }
        if !enabled() {
            println!("gameplay: RC_PLAY=0: fly camera only, no game tick");
            return;
        }
        let script = std::env::var("RC_PLAY_SCRIPT").ok().filter(|s| !s.trim().is_empty()).map(|s| match Script::parse(&s) {
            Ok(sc) => sc,
            Err(e) => panic!("RC_PLAY_SCRIPT: {e}"),
        });
        println!("{}", input_map::CONTROLS);
        if let Some(s) = &script { println!("gameplay: RC_PLAY_SCRIPT drives the pad for ticks 0..={} (keyboard and gamepad ignored)", s.end()); }
        // RC_PLAY_FLY=1: the game ticks but the view starts on the fly camera (RC_CAM), for looking at mobys.
        let fly = std::env::var("RC_PLAY_FLY").is_ok_and(|v| v.trim() == "1");
        app.insert_resource(if fly { CameraSource::Fly } else { CameraSource::Play })
            .insert_resource(PlayScript(script))
            .init_resource::<PadFrame>()
            .init_resource::<TickBudget>()
            .add_systems(
                PreUpdate,
                (setup, input_map::sample_system, set_budget, keys).chain().after(bevy::input::InputSystems),
            )
            .add_systems(FixedUpdate, tick.in_set(GameTick).before(crate::particle_render::tick))
            .add_systems(RunFixedMainLoop, play_camera::apply.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop))
            .add_systems(
                PostUpdate,
                (
                    upload.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate).before(bevy::asset::AssetEventSystems),
                    write_visible.after(moby_render::update_moby_occlusion).after(upload),
                ),
            );
    }
}

#[derive(Resource)]
struct PlayScript(Option<Script>);

/// The set of the gameplay tick system (crate::menu_render runs its frame after it).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameTick;

/// The persistent game state (what the memory card saves), `rc_game::game_state`.
#[derive(Resource)]
pub struct Persistent(pub GameState);

/// The session-only state (hero HP, temporary items, Clank hidden, tick scale).
#[derive(Resource)]
pub struct Session(pub SessionState);

/// The HUD's weapon slot input after the last tick (`Inputs::weapon`, rule 0x24f9c0): the held item
/// (0x140408) with its ammo and max ammo when its item record has an ammo HUD and the hero is on foot.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct HeldWeapon(pub Option<(u16, i32, i32)>);

/// Per item: has an ammo HUD (record +8 ≠ 0) and the max ammo (record +0xe), from the item records; and the item
/// definitions' weapon fields (`0x22ee08`), by id.
#[derive(Resource, Clone, Debug, Default)]
struct AmmoTable(Vec<(bool, u16)>, Vec<rc_game::hero::items::WeaponDef>);

/// The hand-item data, the ammo table (uses ammo, max) and the weapon fields of the item definitions.
type ItemTablesOut = (ItemData, Vec<(bool, u16)>, Vec<rc_game::hero::items::WeaponDef>);

/// The level's hand-item data (item definitions from the overlay, the gadget classes, Ratchet's joint lists)
/// for `rc_game::hero::items`: the definitions at the overlay's item table (L01 0x179f40, found through
/// `GiveItem`), the classes from the gadget table (decompressed as `select_world_object_resource_tables`
/// would), their joint lists; with the ammo table (uses ammo, max) and the weapon fields of the definitions.
fn item_data(lv: &crate::level_load::LoadedLevel) -> anyhow::Result<ItemTablesOut> {
    use anyhow::Context;
    use rc_formats::gadget;
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let elf = crate::disc_source::read_path(&root, &root.join("boot/SCUS_971.99")).context("boot ELF")?;
    let overlay = crate::disc_source::level_file(&root, index, "overlay.bin").context("level overlay")?;
    let tables = ItemTables::load(&elf, &overlay)?;
    let sections = rc_formats::font::parse_overlay_sections(&overlay)?;
    let n = rc_formats::save_game::ITEM_COUNT;
    let sz = rc_formats::save_game::ITEM_DEF_SIZE;
    let raw = rc_formats::font::read_overlay(&sections, tables.item_defs_addr, n * sz).context("item definitions")?;
    let w = |i: usize, o: usize| i32::from_le_bytes(raw[i * sz + o..i * sz + o + 4].try_into().unwrap());
    let defs = (0..n).map(|i| ItemDef { slot: w(i, 8), attach: w(i, 0xc), o_class: w(i, 0x10), b18: raw[i * sz + 0x18] }).collect();
    // The weapon draw's fields (+0x18 word, the three sequences +0x24..+0x2c, +0x30): `0x22ee08`.
    let weapon_defs = (0..n).map(|i| rc_game::hero::items::WeaponDef { w18: w(i, 0x18), anims: [w(i, 0x24), w(i, 0x28), w(i, 0x2c)], w30: w(i, 0x30) }).collect();
    let (ratchet_blob, gadgets) = crate::moby_attach::load_blobs()?;
    let rc = lv.mobys.classes.iter().find(|c| c.o_class == gadget::RATCHET_O_CLASS).context("no class 0")?;
    let hero_chains = HERO_LISTS.iter().map(|&l| gadget::joint_list(&ratchet_blob, &rc.class.header, l).map(|(a, _)| a).unwrap_or_default()).collect();
    let mut classes = Vec::new();
    for g in &gadgets {
        let c = &g.moby.class;
        let seqs = rc_formats::moby_anim::parse_sequences(&g.blob, c).with_context(|| format!("gadget {} sequences", g.moby.o_class))?;
        let chains = (0..16).map_while(|l| gadget::joint_list(&g.blob, &c.header, l).ok().map(|(a, _)| a)).collect();
        classes.push(ItemClass { o_class: g.moby.o_class as i16, anim: rc_formats::moby_anim::MobyAnimClass::new(c, seqs), scale: c.header.scale, chains });
    }
    let ammo = tables.records.iter().map(|r| (r.has_ammo(), u16::from_le_bytes([r.0[0xe], r.0[0xf]]))).collect();
    Ok((ItemData { defs, hero_chains, classes }, ammo, weapon_defs))
}

/// The level water tables the hero's ground probe reads (`0x26ed38`): the class-751 ripple patches of
/// crate::water_render (borrowed per query: the moby hook runs 751 on the same state earlier in the tick).
struct HeroWater<'w, 'r>(&'w RefCell<Option<&'r mut WaterState>>);

impl rc_game::hero::swim::WaterQuery for HeroWater<'_, '_> {
    fn water_height(&self, p: [f32; 3]) -> Option<f32> {
        self.0.borrow().as_ref()?.ripple.as_ref()?.water_height(p)
    }
}

/// The moby system hook plus the water tables.
struct HeroWorld<'a, 'b, 'w, 'r> {
    world: SharedServices<'a, 'b>,
    water: HeroWater<'w, 'r>,
}

impl rc_game::tick::MobySystem for HeroWorld<'_, '_, '_, '_> {
    fn scene(&mut self, table: &MobyTable) -> Option<rc_game::collision_query::OwnedScene> { self.world.scene(table) }
    fn build_matrix(&mut self, table: &mut MobyTable, id: MobyId) { self.world.build_matrix(table, id) }
    fn deliver_hit(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate) { self.world.deliver_hit(table, target, tmpl) }
    fn water(&self) -> Option<&dyn rc_game::hero::swim::WaterQuery> { Some(&self.water) }
    fn hit_message(&self, table: &MobyTable, target: MobyId) -> Option<rc_game::moby_update::services::HitRecord> { self.world.hit_message(table, target) }
    fn take_hero_writes(&mut self) -> Option<rc_game::moby_update::services::HeroFields> { self.world.take_hero_writes() }
    fn take_camera_shakes(&mut self) -> Vec<rc_game::follow_camera::ShakeRequest> { self.world.take_camera_shakes() }
    fn take_cinematic(&mut self) -> Vec<rc_game::cinematic::CinematicCall> { self.world.take_cinematic() }
    fn run_list(&self, table: &MobyTable, camera: [rc_game::ps2v::Pf; 4]) -> Option<Vec<MobyId>> { self.world.run_list(table, camera) }
    fn volumes(&self) -> Option<std::sync::Arc<rc_formats::volumes::Volumes>> { self.world.volumes() }
}

/// The hero's hit sink: the moby system's hit log and moby collision (borrowed per call: the moby hook borrows
/// the services too) and the level collision.
struct CellHits<'a, 'b> {
    svc: &'a RefCell<&'b mut Services>,
    classes: &'a dyn ClassData,
    coll: &'a Collision,
}

impl HitSink for CellHits<'_, '_> {
    fn sphere(&mut self, table: &mut MobyTable, r: rc_game::ps2v::Pf, centre: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<MobyId> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.sphere(table, r, centre, flags, ignore, tmpl)
    }
    fn line(&mut self, table: &mut MobyTable, a: [rc_game::ps2v::Pf; 4], b: [rc_game::ps2v::Pf; 4], flags: u32, ignore: Option<MobyId>, tmpl: &HitTemplate) -> Option<Option<MobyId>> {
        let mut s = self.svc.borrow_mut();
        ServiceHits { svc: &mut s, classes: self.classes, coll: Some(self.coll) }.line(table, a, b, flags, ignore, tmpl)
    }
    fn deliver(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::services::deliver_hit_in(table, &mut s.hits, target, tmpl);
    }
    fn create_moby(&mut self, table: &mut MobyTable, o_class: i16, counter: u64) -> Option<MobyId> {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::create_from_hero(table, &mut s, self.classes, o_class, counter)
    }
    fn delete_moby(&mut self, table: &mut MobyTable, id: MobyId, counter: u64) {
        let mut s = self.svc.borrow_mut();
        rc_game::moby_update::classes::bomb::delete_from_hero(table, &mut s, id, counter);
    }
}

/// The level's moby class collision blobs (class header +0x10), from the core (re-read here: the level loader
/// keeps no raw class blobs).
fn moby_collision_blobs(index: u32) -> anyhow::Result<Vec<(i32, rc_formats::moby_collision::MobyCollision)>> {
    use anyhow::Context;
    let root = crate::level_load::extracted_root();
    let idx = crate::disc_source::level_file(&root, index, "core_index.bin").context("core index")?;
    let data = rc_data::level_core_data(&root, index).context("decompressing core data")?;
    let core = rc_formats::level::parse_level_core(&idx, data.len()).context("parsing core index")?;
    Ok(rc_formats::moby_collision::parse_level(&core, &data)?)
}

/// The hand-swap globals from the saved game and the session (0x141408 request, 0x141660 saved hand item,
/// 0x15ed8c previous, 0x15ed90 wrench flag).
fn item_globals(state: Option<&GameState>, session: Option<&SessionState>) -> ItemGlobals {
    let mut g = ItemGlobals { wrench_flag: 1, ..Default::default() };
    if let Some(gs) = state {
        g.saved = gs.global.equipped[0];
        g.previous = gs.global.last_hand_item;
        g.wrench_flag = gs.global.wrench_held;
    }
    if let Some(s) = session { g.request = s.temp_hand; }
    g
}

/// The state of a direct boot into level `index`: new game, Veldin's level start and Clank init, then
/// (index ≥ 1) the transition and this level's start (docs/plan/game_state.md §4).
fn load_game_state(root: &std::path::Path, index: u32) -> anyhow::Result<(GameState, SessionState)> {
    use anyhow::Context;
    let elf = crate::disc_source::read_path(root, &root.join("boot/SCUS_971.99")).context("boot ELF")?;
    let lump = SaveGameLump::parse(&crate::disc_source::read(root, "global/save_game.bin").context("save_game lump")?)?;
    let items = |l: u32| -> anyhow::Result<ItemTables> {
        Ok(ItemTables::load(&elf, &crate::disc_source::level_file(root, l, "overlay.bin").with_context(|| format!("level {l:02} overlay"))?)?)
    };
    let mut gs = GameState::new_game(ChunkTables::from_boot_elf(&elf)?, &lump.template)?;
    let mut sess = SessionState::default();
    gs.apply_level_start(0, &items(0)?, &mut sess);
    if index >= 1 {
        gs.on_veldin_clank_init(&mut sess);
        gs.apply_transition(index as i32);
        gs.apply_level_start(index as i32, &items(index)?, &mut sess);
    }
    // RC_GIVE_ITEMS (debug): the last hand item among the ids (slot type 0, not the wrench, e.g. the Swingshot 12)
    // is requested into the hand, as GiveItem's equip does (the session's temp hand item, 0x141408).
    if let Some(ids) = give_items() {
        let t = items(index)?;
        if let Some(&h) = ids.iter().rev().find(|&&i| i != 8 && t.slot_type.get(i) == Some(&0)) {
            sess.temp_hand = h as i32;
            println!("game state: RC_GIVE_ITEMS: item {h} requested into the hand");
        }
    }
    Ok((gs, sess))
}

/// Gameplay ticks still allowed this rendered frame (the main loop's catch-up rule).
#[derive(Resource, Default)]
struct TickBudget(u32);

/// The running game and Ratchet's draw state.
#[derive(Resource)]
pub struct Play {
    pub game: Game,
    pub ratchet: RatchetAnim,
    /// Ratchet's moby (`0x1acc00[his gameplay instance]`) and animation instance.
    hero_id: MobyId,
    hero_k: Option<usize>,
    /// His class (index into `LevelMobys::classes` / `anim`), moby+0x2c scale, palette slots.
    class: usize,
    scale: f32,
    slots: u32,
    light_word: u32,
    ambient: [u8; 3],
    /// His moby as the level places it, for respawns.
    spawn_moby: Moby,
    extra: ExtraMobys,
    entities: Vec<Entity>,
    uploaded: Option<u64>,
    /// The moby loop: scheduler state, the class table (shared with the hero's per-tick collision scene), the
    /// services it owns.
    pub sched: Scheduler,
    pub classes: std::sync::Arc<ClassTable>,
    pub svc: Services,
    /// Static mobys of a class with a ported update (drawn from the table) with their gameplay instance and
    /// `MobyAnim` instance, and
    /// the light block inputs they were last lit with.
    driven: Vec<(MobyId, usize, Option<usize>)>,
    /// Static moby → gameplay instance (the renderer's records, entities and occlusion are per instance).
    moby_to_instance: Vec<usize>,
    /// The level's mission state (0x15fc88, 0x14c050, the deaths 0x14ee90): kept across respawns (the death
    /// reload keeps the deaths; the death sequence bumps the killer's mission, `DamageEvent::Died`).
    pub missions: LevelMissions,
    lit: HashMap<MobyId, ([u32; 9], [u8; 3], u32)>,
    /// The loader's ship: (its table id, its gameplay-instance index), drawn by the static path.
    ship: Option<(MobyId, usize)>,
    /// Class-27 emitter moby → `Particles::owners` index.
    emitters: HashMap<MobyId, usize>,
    level: u32,
    dynamic: DynMobys,
    /// Moby sounds queued so far (drained from `Services::sounds` every tick), per (class, index).
    sounds: HashMap<(i16, i32), u64>,
    /// `RC_DEBUG_HIT=moby@tick,...` (`hero@tick` = Ratchet): hits delivered to those mobys at those ticks.
    debug_hits: Vec<(MobyId, u64)>,
    /// Option 0x15edb5 (mirrored animation), for respawns.
    mirror_anim: bool,
    /// The hand-item data (also given to every respawned `Game`).
    item_data: Option<ItemData>,
    /// The back items' classes (the packs by back item id, Clank 601), given to every (respawned) hero.
    back_classes: Option<BackPacks>,
    trace: bool,
    respawn: bool,
    frozen_hint: bool,
    /// Ratchet's entities hidden (his moby's mode bit 1: the first-person view hides him, `HeroSyncMoby`).
    ratchet_hidden: bool,
}

impl Play {
    /// The loader's ship in the moby table (`0x13e030`; crate::scene_render hides / shows it for the mission NPC).
    pub fn ship_moby(&self) -> Option<MobyId> { self.ship.map(|s| s.0) }
}

/// The external updates' level-table address for `o_class` (`ExternalUpdates::update_fn`, also used to build
/// the class table before the externals exist).
fn external_update_fn(level: u32, ripples: bool, o_class: i16) -> Option<u32> {
    match o_class {
        27 if level == 1 => Some(EMITTER_UPDATE),
        751 if ripples => Some(RIPPLE_UPDATE),
        _ => None,
    }
}

/// Class updates ported outside `rc_game::moby_update`, run by the scheduler at their place in the moby order
/// on the game's stream: the class-27 emitters (`type06::emitter_update_live` 0x2bd100 on the emitter's live
/// moby, which the Blarg flyers 660 move every tick, into the world's
/// particles, with the view of the last rendered frame) and the ripple manager 751 (state 0: the load-pass
/// init, else the update with the game camera 0x167240).
struct Externals<'a> {
    level: u32,
    emitters: &'a HashMap<MobyId, usize>,
    view: Option<&'a BSphereView>,
    water: Option<&'a mut WaterState>,
}

impl ExternalUpdates for Externals<'_> {
    fn update_fn(&self, o_class: i16) -> Option<u32> { external_update_fn(self.level, self.water.as_ref().is_some_and(|w| w.has_ripples()), o_class) }

    fn update(&mut self, addr: u32, id: MobyId, table: &mut MobyTable, rng: &mut Rng, camera: [rc_game::ps2v::Pf; 4], counter: u64, particles: Option<&mut Particles>) {
        match addr {
            EMITTER_UPDATE => {
                if let (Some(p), Some(&o)) = (particles, self.emitters.get(&id)) { type06::emitter_update_live(p, rng, o, self.view, self.level, &table.mobys[id]); }
            }
            RIPPLE_UPDATE => {
                let Some(w) = self.water.as_deref_mut() else { return };
                let m = &mut table.mobys[id];
                if m.state == 0 {
                    // 0x2fd0e8 state 0: the init, then state 1, update distance 0xff (always in the run list),
                    // z + 0.5.
                    w.ripple_init(rng);
                    m.state = 1;
                    m.update_dist = 0xff;
                    m.position[2] += 0.5;
                } else {
                    w.ripple_update_with([camera[0].to_f32(), camera[1].to_f32(), camera[2].to_f32()], rng, counter, particles);
                }
            }
            _ => {}
        }
    }
}

/// The class table of the loaded level: every class of the core's class list (slot = its index there,
/// `MobyClassesRelocate`), its `InitMobyInstance` view (a class without a blob: no header), the update address
/// (a Rust port, an external update, else none → mode 2) and its sequences. A class loaded from elsewhere (the ship from the spaceships file)
/// gets the next slot (inferred).
fn class_table(lv: &crate::level_load::LoadedLevel, ext: &dyn Fn(i16) -> Option<u32>) -> ClassTable {
    let m = &lv.mobys;
    let mut t = ClassTable::default();
    let mut slot_of: HashMap<i32, usize> = lv.moby_class_order.iter().enumerate().rev().map(|(s, &o)| (o, s)).collect();
    let mut next = lv.moby_class_order.len();
    for (c, a) in m.classes.iter().zip(&m.anim) {
        let slot = *slot_of.entry(c.o_class).or_insert_with(|| { next += 1; next - 1 });
        let oc = c.o_class as i16;
        let mut info = scheduler::class_info(&c.class, slot as u8, scheduler::port_update_fn(oc).or_else(|| ext(oc)));
        info.seq0 = a.sequence(0).map(|q| Seq0Info { frame_count: q.header.frame_count, loop_sound_bit7: q.header.loop_sound & 0x80 != 0 });
        t.classes.insert(oc, (info, Some(a.clone())));
    }
    // Slots without a class blob (no header, e.g. the class-27 emitters and 751): InitMobyInstance keeps the
    // slot's update function and sets mode |= 5.
    for (slot, &o) in lv.moby_class_order.iter().enumerate() {
        let oc = o as i16;
        t.classes.entry(oc).or_insert_with(|| {
            let info = rc_game::moby_runtime::ClassInfo { slot: slot as u8, no_header: true, update_fn: scheduler::port_update_fn(oc).or_else(|| ext(oc)), ..Default::default() };
            (info, None)
        });
    }
    t
}

/// The joint lists of the classes whose update reads joint points (`rc_game::moby_update::classes::needs_joint_lists`,
/// `Services::joint_lists`), from their blobs in the level core (as `menu_render::load_frame_class`).
fn class_joint_lists(lv: &crate::level_load::LoadedLevel) -> anyhow::Result<HashMap<i16, Vec<Vec<u8>>>> {
    use anyhow::{anyhow, Context};
    let wanted: Vec<&rc_formats::moby::LevelMobyClass> =
        lv.mobys.classes.iter().filter(|c| rc_game::moby_update::classes::needs_joint_lists(c.o_class as i16)).collect();
    let mut out = HashMap::new();
    if wanted.is_empty() { return Ok(out); }
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let read = |name: &str| crate::disc_source::level_file(&root, index, name);
    let data = rc_data::level_core_data(&root, index).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&read("core_index.bin")?, data.len()).context("parsing core index")?;
    for c in wanted {
        let name = format!("moby_class/{:04}", c.o_class);
        let blk = core.blocks.iter().find(|b| b.name == name).ok_or_else(|| anyhow!("no {name} block"))?;
        let blob = data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow!("{name} out of range"))?;
        let lists = (0..16).map_while(|l| rc_formats::gadget::joint_list(blob, &c.class.header, l).ok().map(|(a, _)| a)).collect();
        out.insert(c.o_class as i16, lists);
    }
    Ok(out)
}

fn rows_bits(rows: &[[f32; 4]; 4]) -> [V4; 3] { [0, 1, 2].map(|i| rows[i].map(f32::to_bits)) }
fn rows3(rows: &[[f32; 4]; 4]) -> [[f32; 3]; 3] { [0, 1, 2].map(|i| [rows[i][0], rows[i][1], rows[i][2]]) }
fn pos3(m: &Moby) -> [f32; 3] { [m.position[0], m.position[1], m.position[2]] }

/// `(gameplay +0x44 section) + 4`: the spawnable moby count = the dynamic slots.
fn spawnable_count(gameplay: &[u8]) -> usize {
    let rd = |o: usize| gameplay.get(o..o + 4).map(|b| i32::from_le_bytes(b.try_into().unwrap()));
    rd(rc_formats::gameplay::MOBY_INSTANCES_POINTER).and_then(|sec| rd(sec as usize + 4)).map_or(0, |n| n.max(0) as usize)
}

/// The loader's moby table: the spawn test on the instances before the ship (`moby_spawn::loader_spawns` with
/// the level's save bytes from `state`), the created ones as the static mobys (`load_level_mobys`, pvar moby
/// links through the instance → moby map), the spawnable count of dynamic slots, the ship created in the first
/// of them (hidden while the level's mission NPC has its mission open: `moby_spawn::ship_hidden_on_arrival`).
/// Returns the table, the statics' maps and the ship's id.
fn moby_table(lv: &crate::level_load::LoadedLevel, classes: &mut ClassTable, ship: Option<usize>, state: Option<&GameState>, level: u32) -> anyhow::Result<(MobyTable, scheduler::LevelStatics, Option<MobyId>)> {
    use rc_formats::moby_spawn as ms;
    let m = &lv.mobys;
    let n_inst = ship.unwrap_or(m.instances.len());
    let insts = &m.instances[..n_inst];
    let save = state.and_then(|s| s.levels.get(level as usize)).map(scheduler::spawn_save).unwrap_or_default();
    let tests = ms::loader_spawns(insts, &mut save.clone());
    let spawned: Vec<bool> = tests.iter().map(|t| t.spawn).collect();
    let pvars = rc_formats::gameplay::parse_pvars_spawned(&lv.gameplay, &spawned)?;
    let statics = scheduler::load_level_mobys(insts, classes, &pvars, &tests);
    let mut table = MobyTable::new(statics.mobys.clone(), spawnable_count(&lv.gameplay).max(1));
    let mut ship_id = None;
    if let Some(s) = ship {
        let inst = &m.instances[s];
        let info = classes.info(inst.o_class as i16);
        if let Some(id) = table.create(inst.o_class as i16, info.as_ref(), 0) {
            let mo = &mut table.mobys[id];
            mo.position = [inst.position[0], inst.position[1], inst.position[2], 0.0];
            mo.rotation = [inst.rotation[0], inst.rotation[1], inst.rotation[2], 0.0];
            for (k, r) in crate::moby_light::instance_rows(inst).iter().enumerate() { mo.rows[k] = r.map(f32::from_bits); }
            mo.draw_dist = inst.draw_distance as i16;
            mo.update_dist = inst.update_distance as u8;
            if ms::ship_hidden_on_arrival(insts, &tests, &save.missions) {
                // MissionNpcUpdate state 1 → FUN_002a2450: mode |= 3, +0x94 = 0.
                mo.mode |= 3;
                mo.has_collision = false;
            }
            ship_id = Some(id);
        }
    }
    Ok((table, statics, ship_id))
}

/// The dynamic slots drawn as extra instances (see the module doc): record `s` and palette range
/// `s·pal_slots` belong to table slot `first_dynamic + s`.
pub struct DynMobys {
    extra: ExtraMobys,
    pal_slots: u32,
    /// o_class → index into `LevelMobys::classes`.
    class_ix: HashMap<i16, usize>,
    /// The class index each slot shows.
    shown: Vec<Option<usize>>,
    records: Vec<u8>,
    palette: Vec<u8>,
    /// +0x31 per slot (the stand-in MobyProc's result of the last upload).
    visible: Vec<u8>,
    /// Live / drawn this frame (for the trace).
    live: usize,
    drawn: usize,
}

impl DynMobys {
    fn new(lv: &crate::level_load::LoadedLevel, slots: usize, buffers: &mut Assets<ShaderBuffer>) -> Self {
        let m = &lv.mobys;
        // Palette matrices per slot: the most any class other than Ratchet needs.
        let pal_slots = m.classes.iter().zip(&m.anim).filter(|(c, _)| c.o_class != 0)
            .map(|(c, a)| (a.joint_count as u32).max(ExtraMobys::max_skinned_joint(c) as u32 + 1)).max().unwrap_or(1).max(1);
        let records = vec![0; slots.max(1) * moby_render::EXTRA_RECORD_SIZE];
        let palette = crate::moby_anim::identity_palette(slots.max(1) as u32 * pal_slots);
        let extra = ExtraMobys::new(lv, records.clone(), palette.clone(), buffers);
        DynMobys {
            extra,
            pal_slots,
            class_ix: m.classes.iter().enumerate().map(|(i, c)| (c.o_class as i16, i)).collect(),
            shown: vec![None; slots],
            records,
            palette,
            visible: vec![0; slots],
            live: 0,
            drawn: 0,
        }
    }
}

/// The gameplay instances' moby entities (not the attachments).
type GameplayEntities<'w, 's> = Query<'w, 's, (Entity, &'static MeshTag), (With<MeshMaterial3d<MobyMaterial>>, Without<AttachedTo>)>;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    mut commands: Commands,
    level: Res<crate::Level>,
    spawn: Option<Res<MobySpawn>>,
    state: Option<Res<Persistent>>,
    session: Option<Res<Session>>,
    occl: Option<ResMut<MobyOcclusion>>,
    mut anim: Option<ResMut<MobyAnim>>,
    mut particles: Option<ResMut<ParticleSim>>,
    mut water: Option<ResMut<WaterState>>,
    gameplay_entities: GameplayEntities,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if *done { return; }
    let Some(mut occl) = occl else { return };
    *done = true;
    let lv = &level.0;
    let m = &lv.mobys;
    let Some(coll) = lv.collision.as_ref() else {
        eprintln!("gameplay: no collision mesh: no game tick");
        return;
    };
    let Some(hero_ii) = m.instances.iter().zip(&m.placed).position(|(i, p)| i.o_class == 0 && p.is_some()) else {
        eprintln!("gameplay: no placed class-0 (Ratchet) instance on this level: no game tick");
        return;
    };
    let placed = m.placed[hero_ii].unwrap();
    let class = &m.anim[placed.class];
    let level_index = lv.particles.level;

    // The loader: class table, static mobys, dynamic slots, the ship.
    let ripples = water.as_ref().is_some_and(|w| w.has_ripples());
    let mut classes = class_table(lv, &|oc| external_update_fn(level_index, ripples, oc));
    let ship_ii = spawn.as_ref().and_then(|s| s.ship);
    let (mut table, statics, ship_id) = match moby_table(lv, &mut classes, ship_ii, state.as_ref().map(|s| &s.0), level_index) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("gameplay: moby table not built ({e:#}): no game tick");
            return;
        }
    };
    let Some(hero_id) = statics.instance_to_moby.get(hero_ii).copied().flatten() else {
        eprintln!("gameplay: Ratchet's instance {hero_ii} was not created by the spawn test: no game tick");
        return;
    };
    if table.hero() != Some(hero_id) { warn!("gameplay: MobyTable::hero() = {:?}, using moby {hero_id} (instance {hero_ii})", table.hero()); }
    // RC_HERO_AT=x,y,z[,yaw] (debug): Ratchet's moby placed there before the hero init (which ground-snaps it).
    if let Some(v) = std::env::var("RC_HERO_AT").ok().map(|v| v.split(',').filter_map(|t| t.trim().parse::<f32>().ok()).collect::<Vec<_>>()) {
        if v.len() >= 3 {
            let m = &mut table.mobys[hero_id];
            m.position = [v[0], v[1], v[2], m.position[3]];
            if let Some(&y) = v.get(3) { m.rotation[2] = y; }
            println!("gameplay: RC_HERO_AT: Ratchet placed at {:?}, yaw {}", &v[..3], m.rotation[2]);
        }
    }
    let spawn_moby = table.mobys[hero_id].clone();
    let opts = state.as_ref().map(|s| s.0.options());
    let options = opts.map_or(GameOptions::default(), |o| o.game_options());
    // srand(1234), hero init (ground snap, mode 2, the fidget-timer draw), the camera behind him.
    let mut game = Game::new(coll, table, hero_id, options, lv.death_z);
    if let Some(s) = &session { game.hero.health = s.0.hp; }
    // The hand items (wrench, bomb glove, …): created by the hero update from the first tick on.
    let item_data = match item_data(lv) {
        Ok((d, ammo, weapon_defs)) => {
            println!(
                "gameplay: hand items: {} item definitions, {} gadget classes (wrench 71: {}, bomb glove def {:?})",
                d.defs.len(), d.classes.len(), d.class(71).is_some(), d.defs.get(10)
            );
            commands.insert_resource(AmmoTable(ammo, weapon_defs));
            Some(d)
        }
        Err(e) => {
            eprintln!("gameplay: no hand items ({e:#}): the wrench is not created and □ does nothing");
            None
        }
    };
    game.item_data = item_data.clone();
    commands.insert_resource(HeldWeapon::default());
    let mut ratchet = RatchetAnim::new(class);
    let mirror_anim = opts.is_some_and(|o| o.mirror_anim);
    ratchet.mirror = mirror_anim;
    // The back items (pack 607 of back item 2, Clank 601) and the level for the idle code.
    let back_classes = back_classes(lv, item_data.as_ref());
    match &back_classes {
        None => eprintln!("gameplay: no pack / Clank (601) classes on this level: no back items (no Clank fidgets)"),
        Some((packs, _)) => println!("gameplay: back packs (item, class) {:?} and Clank 601", packs.iter().map(|p| (p.0, p.1)).collect::<Vec<_>>()),
    }
    hero_level_setup(&mut game.hero, back_classes.as_ref(), level_index);

    // The moby loop's services and the load pass (counter 0) on the game's stream.
    let mut svc = Services::new();
    svc.level = level_index;
    if let Ok(sp) = rc_formats::gameplay::parse_splines(&lv.gameplay) { svc.set_splines(&sp); }
    // The volume sections (cuboids, spheres, cylinders, pills, paths, grind paths) for the trigger tests.
    match rc_formats::volumes::parse_volumes(&lv.gameplay) {
        Ok(v) => {
            // The grind paths are also the hero's rails and cables (hero::boots).
            game.grind_paths = std::sync::Arc::new(v.grind_paths.clone());
            svc.set_volumes(v)
        }
        Err(e) => eprintln!("gameplay: no trigger volumes ({e}): every volume test is false"),
    }
    match class_joint_lists(lv) {
        Ok(j) => svc.joint_lists = j,
        Err(e) => eprintln!("gameplay: no class joint lists ({e:#}): the Blarg flyers' exhaust sits at the flyer origin"),
    }
    let n_static = game.mobys.first_dynamic;
    svc.groups = statics.groups(&lv.gameplay);
    // The "use" system: talk tables, text, talk slots, save values (crate::interact_render; before the load pass).
    crate::interact_render::install(&mut svc, lv, &statics.instance_to_moby, state.as_ref().map(|s| &s.0));
    if let Some(gs) = &state {
        svc.counters.bolts = gs.0.global.bolts;
        // 0x15ee20 (the challenge-gated pads 1135 and gold-weapon offers read it) and 0x13e520.
        svc.counters.times_completed = gs.0.global.completes;
        svc.counters.gold_weapons = gs.0.global.gold_weapons.to_vec();
        if let Some(l) = gs.0.levels.get(level_index as usize) { svc.counters.level_bolts[level_index as usize % 20] = l.bolts; }
    }
    // Moby collision: the class blobs and the loader's grid registrations (MobyBuildMatrix per instance).
    match moby_collision_blobs(level_index) {
        Ok(b) => svc.set_moby_collision(b),
        Err(e) => eprintln!("gameplay: no moby collision ({e:#}): the collision queries see no mobys"),
    }
    svc.build_grid(&mut game.mobys);
    println!(
        "gameplay: moby collision: {} class blobs, {} grid entries",
        svc.coll_classes.len(), svc.grid.cells.iter().map(Vec::len).sum::<usize>()
    );
    // Class-27 emitters: owner k belongs to gameplay instance `o.instance`, i.e. to moby 0x1acc00[o.instance].
    let emitters: HashMap<MobyId, usize> = particles.as_ref().map(|p| {
        p.sys.owners.iter().enumerate().filter_map(|(k, o)| Some((statics.instance_to_moby.get(o.instance).copied().flatten()?, k))).collect()
    }).unwrap_or_default();
    // The level's mission state: 0x15fc88 / 0x14c050 from the save, the per-mission deaths 0x14ee90 cleared
    // (a fresh load; the ammo crates 501 read them in the load pass).
    let level_save = state.as_ref().and_then(|s| s.0.levels.get(level_index as usize)).map(|l| l.missions).unwrap_or_default();
    let missions = LevelMissions::fresh_load(level_index, level_save);
    let mut sched = Scheduler::new();
    let rng0 = game.rng;
    let n_load = {
        let hero = game.hero.clone();
        // view None: the view before the level's first render (all zero: FastBSphereCheck culls, type06).
        let mut ext = Externals { level: level_index, emitters: &emitters, view: None, water: water.as_deref_mut() };
        let mut w = World::new(&mut game.mobys, &hero, &mut game.rng, &classes, &mut svc, game.counter);
        w.camera = game.camera.out.pos;
        w.coll = Some(coll);
        w.particles = particles.as_deref_mut().map(|p| &mut p.sys);
        w.external = Some(&mut ext);
        w.missions = &missions;
        sched.load_pass(&mut w)
    };
    // LoadLevelCoreData: 0x15f5cc++ after the load pass.
    game.finish_load();
    if let Some(p) = particles.as_mut() { p.external = true; }

    // Ratchet as an extra instance of his class; the gameplay-instance entities are hidden.
    let slots = (class.joint_count as u32).max(ExtraMobys::max_skinned_joint(&m.classes[placed.class]) as u32 + 1).max(1);
    let mut extra = ExtraMobys::new(lv, vec![0; moby_render::EXTRA_RECORD_SIZE], crate::moby_anim::identity_palette(slots), &mut buffers);
    let hm = &game.mobys.mobys[hero_id];
    let t = Transform::from_matrix(moby_render::extra_model(rows3(&hm.rows), placed.scale, [hm.position[0], hm.position[1], hm.position[2]]));
    let entities = extra.spawn(&mut commands, lv, &m.classes[placed.class], 0, t, "Ratchet (play)", &mut meshes, &mut images, &mut materials);
    let mut hidden = 0;
    for (e, tag) in &gameplay_entities {
        if tag.0 as usize == hero_ii && !entities.contains(&e) {
            commands.entity(e).insert((SpawnHidden, Visibility::Hidden));
            hidden += 1;
        }
    }
    // Instances the loader did not create (spawn test): never drawn.
    let pending: std::collections::HashSet<usize> = statics.pending.iter().copied().collect();
    let mut unspawned = 0;
    for (e, tag) in &gameplay_entities {
        if pending.contains(&(tag.0 as usize)) {
            commands.entity(e).insert((SpawnHidden, Visibility::Hidden));
            unspawned += 1;
        }
    }
    if let Some(a) = anim.as_mut() {
        for &ii in &statics.pending { if let Some(k) = occl.anim_index(ii) { a.hidden[k] = true; a.pending[k] = None; } }
    }
    let hero_k = occl.anim_index(hero_ii);
    if let (Some(a), Some(k)) = (anim.as_mut(), hero_k) { a.pending[k] = None; }

    // The static mobys the table drives (a class with a Rust port), and the dynamic slots.
    let driven: Vec<(MobyId, usize, Option<usize>)> = (0..n_static)
        .filter(|&id| game.mobys.mobys[id].update_fn.and_then(scheduler::ported).is_some())
        .map(|id| { let ii = statics.moby_to_instance[id]; (id, ii, occl.anim_index(ii)) })
        .collect();
    let dynamic = DynMobys::new(lv, game.mobys.mobys.len() - n_static, &mut buffers);

    let inst = &m.instances[hero_ii];
    let h = &game.hero;
    let mut per_class: std::collections::BTreeMap<i16, usize> = std::collections::BTreeMap::new();
    for &(id, _, _) in &driven { *per_class.entry(game.mobys.mobys[id].o_class).or_default() += 1; }
    println!(
        "gameplay: Ratchet = instance {hero_ii} = moby {hero_id} (class slot {}), ground-snapped to {:.3?} yaw {:.4}, fidget timer {}; death z {}; \
         moby table {} static ({} instances not created by the spawn test, {unspawned} entities hidden) + {} dynamic slots (spawnable count; ship {:?}{}); \
         {hidden} gameplay entities hidden, drawn as an extra instance ({} entities, {slots} palette slots)",
        placed.class, h.position(), h.yaw().to_f32(), h.fidget_timer, lv.death_z, n_static, statics.pending.len(), game.mobys.mobys.len() - n_static, ship_id,
        if ship_id.is_some_and(|id| game.mobys.mobys[id].mode & 1 != 0) { ", hidden by the mission NPC" } else { "" }, entities.len()
    );
    println!(
        "gameplay: moby load pass: {n_load} mobys run, rng {:#010x} -> {:#010x}, tick counter now {}; scheduler-driven statics per class {per_class:?}; \
         {} groups; {} class-27 emitters, water 751 {}; dynamic slots drawn as extras ({} palette matrices each)",
        rng0.state, game.rng.state, game.counter, svc.groups.lists.iter().flatten().count(), emitters.len(),
        if water.as_ref().is_some_and(|w| w.external) { "on the game stream" } else { "not run" }, dynamic.pal_slots
    );
    commands.insert_resource(PlayView { view: game.camera.out, tan_half_fov: play_camera::GAME_TAN_HALF_FOV });
    let mut play = Play {
        game,
        ratchet,
        hero_id,
        hero_k,
        class: placed.class,
        scale: placed.scale,
        slots,
        light_word: inst.light_word(),
        ambient: inst.ambient_rgb(),
        spawn_moby,
        extra,
        entities,
        uploaded: None,
        sched,
        classes: std::sync::Arc::new(classes),
        svc,
        driven,
        moby_to_instance: statics.moby_to_instance.clone(),
        missions,
        lit: HashMap::new(),
        ship: ship_id.zip(ship_ii),
        emitters,
        level: level_index,
        dynamic,
        sounds: HashMap::new(),
        ratchet_hidden: false,
        debug_hits: std::env::var("RC_DEBUG_HIT").ok().map(|v| {
            v.split(',').filter_map(|h| {
                let (a, b) = h.trim().split_once('@')?;
                let id = if a.trim() == "hero" { hero_id } else { a.parse().ok()? };
                Some((id, b.parse().ok()?))
            }).collect()
        }).unwrap_or_default(),
        mirror_anim,
        item_data,
        back_classes,
        trace: std::env::var("RC_PLAY_TRACE").is_ok_and(|v| v.trim() == "1"),
        respawn: false,
        frozen_hint: false,
    };
    publish_anim(&mut play, anim.as_deref_mut());
    drive_statics(&mut play, lv, &mut occl, anim.as_deref_mut());
    commands.insert_resource(play);
}

/// The back items' classes: `(back item id, o_class, class)` per pack, and Clank.
type BackPacks = (Vec<(i32, i16, MobyAnimClass)>, MobyAnimClass);

/// The back items' anim classes on this level: the pack moby of each back item 2 / 3 / 4 (the item definitions'
/// class +0x10: Heli-Pack 607, Thruster-Pack 608, Hydro-Pack 609; those values without the definitions) that the
/// level has, and Clank (item 1's 601). None without the pack of item 2 or Clank.
fn back_classes(lv: &crate::level_load::LoadedLevel, items: Option<&ItemData>) -> Option<BackPacks> {
    let m = &lv.mobys;
    let anim = |o: i32| m.classes.iter().position(|c| c.o_class == o).map(|ci| m.anim[ci].clone());
    let class_of = |id: i32, fallback: i32| items.map(|d| d.def(id).o_class).filter(|&o| o > 0).unwrap_or(fallback);
    let packs: Vec<(i32, i16, MobyAnimClass)> =
        [(2, 607), (3, 608), (4, 609)].into_iter().filter_map(|(id, o)| { let o = class_of(id, o); Some((id, o as i16, anim(o)?)) }).collect();
    if !packs.iter().any(|p| p.0 == 2) { return None; }
    Some((packs, anim(class_of(1, 601))?))
}

/// What the hero code needs from the level after `HeroInit` (load and respawn): the back items' classes and
/// the level index 0x15ed84.
fn hero_level_setup(hero: &mut Hero, back: Option<&BackPacks>, level: u32) {
    if let Some((packs, clank)) = back { hero.set_back_packs(packs.clone(), clank.clone()); }
    hero.idle.level = level as i32;
}

/// The table → the renderer for the scheduler-driven static mobys (module doc): placement, light block (for
/// changed rows / ambient / light word), hide, anim state and snapshot.
fn drive_statics(p: &mut Play, lv: &crate::level_load::LoadedLevel, occl: &mut MobyOcclusion, mut anim: Option<&mut MobyAnim>) {
    let table = &p.game.mobys;
    let lighting = lv.mobys.lighting.as_ref();
    for &(id, ii, k) in &p.driven {
        let m = &table.mobys[id];
        // +0x23 alpha 0 (the Blarg flyers' altitude fade above z 175) draws nothing; the partial fade (0 < a <
        // 0x80) is not rendered.
        let hidden = m.state >= 0x80 || m.mode & 0x81 != 0 || m.alpha == 0;
        let rows = rows_bits(&m.rows);
        let key = (std::array::from_fn(|i| rows[i / 3][i % 3]), [m.ambient[0], m.ambient[1], m.ambient[2]], m.light);
        let lights = match lighting {
            Some(l) if !hidden && p.lit.get(&id) != Some(&key) => {
                p.lit.insert(id, key);
                Some(light::moby_lights(&rows, &l.bank, m.light, key.1, 0x80))
            }
            _ => None,
        };
        occl.drive(ii, pos3(m), rows3(&m.rows), m.scale, lights.as_ref(), hidden);
        occl.look(ii, m.alpha, m.mode);
        if let (Some(a), Some(k)) = (anim.as_deref_mut(), k) {
            a.drive(k, m.anim, p.svc.snapshots.get(id).and_then(|s| s.as_ref()));
        }
    }
}

/// The main loop's catch-up rule for this rendered frame: RCNT1 counts 0x2580 per 60 Hz field.
fn set_budget(det: Res<Deterministic>, time: Res<Time<Real>>, mut budget: ResMut<TickBudget>) {
    let rcnt1 = (time.delta_secs_f64() * 60.0 * FIELD_RCNT1 as f64) as u32;
    budget.0 = if det.0 { 1 } else { ticks_for_frame(rcnt1) };
}

/// Tab: fly ↔ game camera (the fly camera resumes from the current view). R: respawn.
fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut source: ResMut<CameraSource>,
    play: Option<ResMut<Play>>,
    mut cams: Query<(&Transform, &mut FlyCam)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        *source = match *source {
            CameraSource::Play => {
                for (t, mut c) in &mut cams {
                    let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
                    (c.yaw, c.pitch) = (yaw, pitch);
                }
                println!("camera: fly (Tab: back to the game camera; the pad is neutral meanwhile)");
                CameraSource::Fly
            }
            CameraSource::Fly => {
                println!("camera: game");
                CameraSource::Play
            }
        };
    }
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(mut p) = play { p.respawn = true; }
    }
}

/// Ratchet's animation into his `MobyAnim` instance (never advanced there: `skip_advance`).
fn publish_anim(p: &mut Play, anim: Option<&mut MobyAnim>) {
    let (Some(a), Some(k)) = (anim, p.hero_k) else { return };
    let mut s = p.ratchet.state;
    s.skip_advance = true;
    a.instances[k].state = s;
    a.snapshots[k] = p.ratchet.snapshot.clone();
}

/// Respawn (the death reload's hero side): the hero, pad and camera as at load, at the checkpoint record when one
/// was reached, else at the level's uid-0 moby; tick counter and RNG continue.
fn respawn(p: &mut Play, coll: &rc_formats::collision::Collision, class: &MobyAnimClass, death_z: f32, state: Option<&GameState>, session: Option<&mut SessionState>) {
    let mut table = std::mem::take(&mut p.game.mobys);
    table.mobys[p.hero_id] = p.spawn_moby.clone();
    let (counter, rng, options) = (p.game.counter, p.game.rng, p.game.options);
    // The hero init on the running stream (its fidget-timer draw included).
    let mut g = Game::with_rng(coll, table, p.hero_id, options, death_z, rng);
    g.counter = counter;
    g.item_data = p.item_data.clone();
    g.grind_paths = p.game.grind_paths.clone();
    hero_level_setup(&mut g.hero, p.back_classes.as_ref(), p.level);
    // Hero init 0x226b70: the hero block is cleared, HP = max HP.
    if let (Some(gs), Some(s)) = (state, session) {
        s.hero_init(gs.global.max_hp);
        g.hero.health = s.hp;
    }
    // The death reload's placement `0x29adc8`: with a checkpoint record (class 805 → `0x29ac10`) the hero block's
    // position and Euler angles are the record's (the rest of the hero init stays); the camera snaps behind him.
    if let Some(cp) = p.svc.save.checkpoint {
        use rc_game::hero::physics::{euler_rows, from_f32x3};
        g.hero.pos = from_f32x3(cp.pos);
        g.hero.rot = from_f32x3(cp.rot);
        g.hero.rows = euler_rows(g.hero.rot);
        let cam = rc_game::follow_camera::CamInput { hero: &g.hero, pad: &g.pad, coll, mobys: None, hero_moby: None };
        g.camera = rc_game::follow_camera::Camera::new(&cam, options.camera);
    }
    p.game = g;
    p.ratchet = RatchetAnim::new(class);
    p.ratchet.mirror = p.mirror_anim;
    p.frozen_hint = false;
}

/// The tick's HUD weapon slot, the ammo table and the sound layer (one system parameter).
type HudAudio<'w> = (Option<ResMut<'w, HeldWeapon>>, Option<Res<'w, AmmoTable>>, Option<ResMut<'w, crate::audio_out::AudioOut>>);

type MainCamera<'w, 's> = Query<'w, 's, &'static Transform, (With<Camera3d>, Without<crate::sky_render::SkyCamera>)>;

/// One 60 Hz gameplay tick (see the module docs). Skipped in the modes whose update does not run the game
/// tick (crate::menu_render: `Mode::advances_tick`, e.g. the mode-3 menus).
#[allow(clippy::too_many_arguments)]
fn tick(
    play: Option<ResMut<Play>>,
    level: Res<crate::Level>,
    pad: Res<PadFrame>,
    script: Res<PlayScript>,
    source: Res<CameraSource>,
    mut budget: ResMut<TickBudget>,
    mut anim: Option<ResMut<MobyAnim>>,
    mut attach: Option<ResMut<MobyAttach>>,
    mut particles: Option<ResMut<ParticleSim>>,
    cams: MainCamera,
    mut view: Option<ResMut<PlayView>>,
    mut state: Option<ResMut<Persistent>>,
    mut session: Option<ResMut<Session>>,
    mode: Option<Res<crate::menu_render::MenuMode>>,
    (mut occl, mut water): (Option<ResMut<MobyOcclusion>>, Option<ResMut<WaterState>>),
    (mut held, ammo, mut audio): HudAudio,
) {
    let Some(mut play) = play else { return };
    if mode.as_ref().is_some_and(|m| !m.state.mode.advances_tick()) { return; }
    if budget.0 == 0 { return; }
    budget.0 -= 1;
    let lv = &level.0;
    let Some(coll) = lv.collision.as_ref() else { return };
    let p = &mut *play;
    let class = &lv.mobys.anim[p.class];
    if std::mem::take(&mut p.respawn) {
        respawn(p, coll, class, lv.death_z, state.as_deref().map(|s| &s.0), session.as_deref_mut().map(|s| &mut s.0));
        println!("gameplay: respawned at the uid-0 moby (R)");
    }

    let input = match &script.0 {
        // Indexed by the main-loop frame (= the gameplay tick index, counter − 1 after the load's increment,
        // until a menu pauses the tick).
        Some(s) => s.at(mode.as_ref().map_or(p.game.counter.saturating_sub(1), |m| m.loop_frame)),
        None if *source == CameraSource::Play => pad.0,
        None => PadInput::neutral(),
    };
    // The view of the last rendered frame (0x16d140, `FastBSphereCheck`): the main camera as last drawn.
    let view_cull = cams.iter().next().map(crate::particle_render::bsphere_view);
    let level_index = p.level;
    let parts_cell = RefCell::new(particles.as_deref_mut());
    let svc_cell = RefCell::new(&mut p.svc);
    let (sched, classes_arc, emitters, missions) = (&mut p.sched, &p.classes, &p.emitters, &p.missions);
    let classes: &ClassTable = classes_arc;
    let debug_hits = &p.debug_hits;
    let water_cell = RefCell::new(water.as_deref_mut());
    // The sound layer (crate::audio_out): the moby loop's class sounds and the tick's sound step.
    let audio_cell = RefCell::new(audio.as_deref_mut());
    let hero_id = p.hero_id;
    let mut n_active = 0usize;
    let mut mobys = |table: &mut MobyTable, hero: &Hero, rng: &mut Rng, cam: &CameraView, coll: &Collision, counter: u64| {
        let mut parts = parts_cell.borrow_mut();
        let mut svc = svc_cell.borrow_mut();
        let mut water_ref = water_cell.borrow_mut();
        let mut audio_ref = audio_cell.borrow_mut();
        let mut sink = audio_ref.as_deref_mut().map(|a| ClassSoundSink { audio: a.system(), listener: class_sounds::listener_of(cam), hero: Some(hero_id) });
        let mut ext = Externals { level: level_index, emitters, view: view_cull.as_ref(), water: water_ref.as_deref_mut() };
        let mut w = World::new(table, hero, rng, classes, &mut svc, counter);
        w.sound = sink.as_mut().map(|s| s as &mut dyn rc_game::moby_update::services::SoundSink);
        w.camera = cam.pos;
        w.coll = Some(coll);
        w.particles = parts.as_deref_mut().map(|p| &mut p.sys);
        w.view = view_cull.as_ref();
        w.external = Some(&mut ext);
        w.missions = missions;
        // RC_DEBUG_HIT: a wrench-like hit (flags 0x10000, damage 1) delivered before the moby loop of tick index
        // N (counter N + 1: the load pass ran at 0); to Ratchet an enemy-contact-like hit (flags 1: the hero's
        // hit intake takes it; no attacker, so he is pushed straight back).
        for &(id, _) in debug_hits.iter().filter(|h| h.1 + 1 == counter) {
            let flags = if id == hero_id { 1 } else { 0x1_0000 };
            w.deliver_hit(id, &HitTemplate { flags, damage: rc_game::ps2v::Pf::ONE, ..Default::default() });
        }
        // PromptTick 0x278eb8 and this tick's pad for the "use" system (moby_update::interact), before the loop.
        w.svc.interact.begin_tick(hero.loop_in.pad.pressed, hero.state);
        n_active = sched.tick(&mut w);
    };
    let mut parts = |hero: &Hero, cam: &CameraView, rng: &mut Rng, counter: u64| {
        if let Some(sim) = parts_cell.borrow_mut().as_deref_mut() {
            sim.sys.counter = counter;
            // 0x167240 as the previous tick's camera update left it (the type-11 sparks read it), and the game's
            // one rand stream.
            sim.sys.camera = [cam.pos[0].0, cam.pos[1].0, cam.pos[2].0];
            sim.sys.cam_yaw = cam.yaw().to_f32();
            // The particles the hero update spawned (sparks, sand, …: rc_game::hero::fx), in its order.
            rc_game::hero::fx::create_particles(hero, &mut sim.sys);
            crate::particle_render::update_parts(sim, Some(rng));
        }
        // FUN_00220928, right after UpdateParts.
        svc_cell.borrow_mut().glints.update();
    };
    // The moby collision the hero and the camera query (and Ratchet's MobyBuildMatrix, the camera's crate hit).
    let mut world = HeroWorld { world: SharedServices { svc: &svc_cell, classes: classes_arc.clone() }, water: HeroWater(&water_cell) };
    let mut hooks = TickHooks { mobys: &mut mobys, particles: &mut parts, world: Some(&mut world) };
    // The hand-swap globals in from the saved game / session (the quick-select ring writes the request).
    p.game.item_globals = item_globals(state.as_deref().map(|s| &s.0), session.as_deref().map(|s| &s.0));
    // The item table 0x13d4c0 (the hero's owned mirror) and the back slot's globals: the saved back item 0x14166c
    // (equipped[3]), 0x15ed94, the request 0x141414 (the session's temp back item) and Clank hidden 0x141628.
    // The ammo table 0x13d428 and the items' "uses ammo" (records +8) and weapon fields (definitions) for the weapons.
    if let Some(a) = ammo.as_deref() {
        for (i, &(has, _)) in a.0.iter().enumerate().take(p.game.hero.weapons.uses_ammo.len()) { p.game.hero.weapons.uses_ammo[i] = has; }
        if p.game.hero.weapons.defs.is_empty() { p.game.hero.weapons.defs = a.1.clone(); }
    }
    if let Some(gs) = state.as_deref() {
        p.game.hero.weapons.ammo = gs.0.global.ammo;
        p.game.hero.owned.0 = gs.0.global.owned;
        svc_cell.borrow_mut().interact.sync_game(&gs.0);
        p.game.hero.back_slot.slot.saved = gs.0.global.equipped[3];
        p.game.hero.back_slot.thruster_last = gs.0.global.thruster_last;
    }
    if let Some(s) = session.as_deref() {
        p.game.hero.back_slot.slot.request = s.0.temp_back;
        p.game.hero.back_slot.clank_hidden = s.0.clank_hidden;
    }
    let mut hits = CellHits { svc: &svc_cell, classes, coll };
    // The sound step (after the camera, before the counter increment).
    let mut sound = |table: &MobyTable, hero: &Hero, cam: &CameraView, rng: &mut Rng, counter: u64| {
        let mut audio_ref = audio_cell.borrow_mut();
        let Some(out) = audio_ref.as_deref_mut() else { return };
        let (sys, buf) = out.parts();
        class_sounds::sound_step(sys, table, hero, cam, rng, counter, buf);
        out.push_frame();
    };
    let has_audio = audio_cell.borrow().is_some();
    // idle.counter: 0x15f5cc as the hero update reads it (the counter before this tick's increment).
    p.game.hero.idle.counter = p.game.counter as i32;
    // Ratchet's own sounds (his animation triggers, his voices) inside the hero update; the listener is the
    // camera the hero update sees (the previous tick's).
    let mut hero_sounds = HeroClassSounds {
        audio: || std::cell::RefMut::filter_map(audio_cell.borrow_mut(), |a| a.as_deref_mut().map(|o| o.system())).ok(),
        class,
        listener: class_sounds::listener_of(&p.game.camera.out),
        hero: hero_id,
        counter: p.game.counter,
    };
    let mut anim_ctl = p.ratchet.ctl(class);
    let sound = if has_audio { Some(&mut sound as &mut rc_game::tick::SoundHook) } else { None };
    let report = p.game.tick_with_hero_sounds(Some(&input.bytes()), coll, &mut anim_ctl, &mut hooks, &mut hits, sound, &mut hero_sounds);
    // … and back out.
    let g = p.game.item_globals;
    if let Some(gs) = state.as_mut() {
        let gl = &mut gs.0.global;
        if gl.equipped[0] != g.saved { gl.equipped[0] = g.saved; }
        if gl.last_hand_item != g.previous { gl.last_hand_item = g.previous; }
        if gl.wrench_held != g.wrench_flag { gl.wrench_held = g.wrench_flag; }
    }
    if let Some(s) = session.as_mut() {
        if s.0.temp_hand != g.request { s.0.temp_hand = g.request; }
        let b = p.game.hero.back_slot.slot.request;
        if s.0.temp_back != b { s.0.temp_back = b; }
    }
    if let Some(gs) = state.as_mut() {
        let (saved, last) = (p.game.hero.back_slot.slot.saved, p.game.hero.back_slot.thruster_last);
        let gl = &mut gs.0.global;
        if gl.equipped[3] != saved { gl.equipped[3] = saved; }
        if gl.thruster_last != last { gl.thruster_last = last; }
    }

    // The ammo the weapons used (0x249450: the ammo and the ammo-used stat 0x13dea0).
    if let Some(gs) = state.as_mut() {
        let w = &mut p.game.hero.weapons;
        if gs.0.global.ammo != w.ammo { gs.0.global.ammo = w.ammo; }
        if w.used.iter().any(|&u| u != 0) {
            for (t, u) in gs.0.global.ammo_used.iter_mut().zip(w.used.iter_mut()) { *t += std::mem::take(u); }
        }
    }
    // The melee entries' stats records (SetState 0x23cf98: 0x1416c0 = levels[8] 3007, misc 0/1, gadget 17).
    if p.game.hero.melee.entered != [0; 3] {
        if let (Some(gs), Some(s)) = (state.as_mut(), session.as_ref()) {
            rc_game::hero::melee::apply_melee_stats(&mut p.game.hero.melee, &mut gs.0, &s.0);
        }
    }
    // Moby sounds: queued and counted (no moby-sound entry in rc_game::audio yet).
    for ev in p.svc.sounds.drain(..) { *p.sounds.entry((ev.o_class, ev.index)).or_default() += 1; }
    // The talk system's saved-game writes (moby_update::interact::GameWrite).
    if let Some(gs) = state.as_mut() {
        for w in p.svc.interact.apply_writes(&mut gs.0) { println!("interact: tick {}: game write {w:?}", p.game.counter); }
    }
    // The bolt counter (0x15ed98, 0x13df38[level]) into the persistent state the HUD and menus read.
    if let Some(gs) = state.as_mut() {
        let c = &p.svc.counters;
        if gs.0.global.bolts != c.bolts { gs.0.global.bolts = c.bolts; }
        if let Some(l) = gs.0.levels.get_mut(level_index as usize) {
            let lb = c.level_bolts[level_index as usize % 20];
            if l.bolts != lb { l.bolts = lb; }
        }
    }
    if let Some(o) = occl.as_deref_mut() { drive_statics(p, lv, o, anim.as_deref_mut()); }

    if p.trace {
        let h = &p.game.hero;
        let s = &p.ratchet.state;
        println!(
            "tick {:5}: state {:#04x} pos {:.4?} yaw {:+.4} air {:3} | anim {}:{} -> {}:{} t {:.3} | cam {:.3?} | {:?} | rng {:#010x} \
             mobys {n_active} bolts {} free {} dyn {}/{} parts {} grass seq1 {} | hand {}:{} {:?} combo {} hit {} fr {:.2} tip {:.3?} | back {} | snd {} | hp {} inv {} | shake {:.4} {:.4}",
            p.game.counter - 2, h.state, h.position(), h.yaw().to_f32(), h.air_ticks, s.seq_a, s.frame_a, s.seq_b, s.frame_b, s.t,
            report.camera.pos_f32(), report.hero, p.game.rng.state, p.svc.counters.bolts, p.game.mobys.free_slots,
            p.dynamic.drawn, p.dynamic.live, particles.as_ref().map_or(0, |s| s.sys.pool.count),
            p.game.mobys.mobys.iter().filter(|m| matches!(m.o_class, 724 | 725) && m.anim.seq_b == 1).count(),
            h.items.slot.id, h.items.slot.state, h.items.slot.item.as_ref().map(|m| (m.anim.seq_b, m.anim.frame_b, m.position)),
            h.melee.combo, h.melee.hit, p.ratchet.frame.to_f32(), rc_game::hero::physics::to_f32x3(h.melee.tip),
            h.back.as_ref().map_or("-".to_string(), |b| format!("pack {}:{} clank {}:{}", b.pack.anim.seq_b, b.pack.anim.frame_b, b.clank.anim.seq_b, b.clank.anim.frame_b)),
            audio_cell.borrow().as_ref().map_or("-".to_string(), |a| { let st = a.stats(); format!("{} plays, class {}/{}", st.plays, st.class_slots, st.class_sounds) }),
            h.health, h.f510, p.game.camera.shake[0].offset.to_f32(), p.game.camera.shake[1].offset.to_f32()
        );
    }
    // The swim's ripple disturbances (`RippleDisturb` 0x2b82a8 on every patch); splashes, bubbles and swim sounds
    // are not drawn / voiced yet.
    for e in std::mem::take(&mut p.game.hero.swim.events) {
        if let rc_game::hero::swim::SwimEvent::Ripple { x, y, r, amp } = e {
            if let Some(sim) = water_cell.borrow_mut().as_deref_mut().and_then(|w| w.ripple.as_mut()) {
                let n = sim.patches.len();
                use rc_game::ps2v::Pf;
                sim.disturb(Pf::f(x), Pf::f(y), Pf::f(r), Pf::f(amp), 0..n, false);
            }
        }
    }
    // Hits taken and deaths (rc_game::hero::damage): the game state's counters (0x15eea8 / 0x13df88[level],
    // 0x15eeac / 0x13dfd8[level]) and the killer's mission deaths (0x14ee90, `LevelMissions::hero_death`).
    for e in std::mem::take(&mut p.game.hero.damage.events) {
        let gs = state.as_mut().map(|g| &mut g.0);
        match e {
            DamageEvent::Hit => {
                if let Some(gs) = gs {
                    gs.global.total_hits += 1;
                    if let Some(l) = gs.levels.get_mut(level_index as usize) { l.hits += 1; }
                }
            }
            DamageEvent::Died { killer_mission, killer_class } => {
                if let Some(gs) = gs {
                    gs.global.total_deaths += 1;
                    if let Some(l) = gs.levels.get_mut(level_index as usize) { l.deaths += 1; }
                }
                p.missions.hero_death(killer_mission);
                println!("gameplay: tick {}: Ratchet died in state {:#x} (killer class {killer_class:?}, mission {killer_mission:?})", p.game.counter, p.game.hero.state);
            }
        }
    }
    if let HeroTick::Unimplemented(s) = report.hero {
        if !std::mem::replace(&mut p.frozen_hint, true) { println!("gameplay: hero frozen in unported state {s:#x}; R respawns"); }
    }
    // The death flag 0x141401 (the death sequence 0x2319b0, or x/y outside 2..1022): the main loop runs no
    // catch-up tick and does the death reload at the end of the frame — here the hero init at the respawn point
    // (the checkpoint record, else the level's spawn).
    if p.game.hero.fell_out != 0 {
        let (at, st) = (p.game.hero.position(), p.game.hero.state);
        budget.0 = 0;
        respawn(p, coll, class, lv.death_z, state.as_deref().map(|s| &s.0), session.as_deref_mut().map(|s| &mut s.0));
        let from = if p.svc.save.checkpoint.is_some() { "the checkpoint" } else { "the uid-0 moby" };
        println!("gameplay: tick {}: death flag 0x141401 (state {st:#x} at {at:.2?}); respawned at {from}", p.game.counter);
    }

    publish_anim(p, anim.as_deref_mut());
    let hm = &p.game.mobys.mobys[p.hero_id];
    if let Some(a) = attach.as_mut() { a.set_host(rows_bits(&hm.rows), [hm.position[0], hm.position[1], hm.position[2]]); }
    if let Some(v) = view.as_mut() { v.view = p.game.camera.out; }
    if let Some(s) = session.as_mut() { s.0.hp = p.game.hero.health; }
    // The HUD's weapon slot (0x24f9c0): the held item 0x140408 when it has an ammo HUD and the hero is on foot.
    if let Some(hw) = held.as_mut() {
        let id = p.game.hero.items.slot.id;
        let w = ammo.as_ref().and_then(|a| a.0.get(id.max(0) as usize).copied()).filter(|&(has, _)| has && id > 0 && p.game.hero.mode == 0)
            .map(|(_, max)| (id as u16, state.as_ref().and_then(|s| s.0.global.ammo.get(id as usize).copied()).unwrap_or(0), max as i32));
        if hw.0 != w { hw.0 = w; }
    }
}

/// Ratchet's record (model matrix, light block for the current rows) and palette after a tick, then the
/// dynamic mobys ([`upload_dynamic`]).
#[allow(clippy::too_many_arguments)]
fn upload(
    play: Option<ResMut<Play>>,
    level: Res<crate::Level>,
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut transforms: Query<&mut Transform, Without<Camera3d>>,
    cams: MainCamera,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut point_lights: ResMut<moby_render::PointLightFrame>,
) {
    let Some(mut p) = play else { return };
    if p.uploaded == Some(p.game.counter) { return; }
    // The point-light bank after the tick (MobyProc merges it into the mobys' third light, moby_render).
    let lights: Vec<_> = p.svc.point_lights.active().copied().collect();
    if point_lights.0 != lights { point_lights.0 = lights.clone(); }
    p.uploaded = Some(p.game.counter);
    let lv = &level.0;
    let class = &lv.mobys.anim[p.class];
    let mut palette = crate::moby_anim::identity_palette(p.slots);
    let f = moby_anim::evaluate_with_snapshot(class, &p.ratchet.state, p.ratchet.snapshot.as_ref());
    for (k, b) in f.iter().take(p.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() { palette[k] = b; }
    let hm = &p.game.mobys.mobys[p.hero_id];
    let rows = rows_bits(&hm.rows);
    let lights = lv.mobys.lighting.as_ref().map(|l| light::moby_lights(&rows, &l.bank, p.light_word, p.ambient, 0x80));
    let model = moby_render::extra_model(rows3(&hm.rows), p.scale, [hm.position[0], hm.position[1], hm.position[2]]);
    let mut record = moby_render::extra_record(&model, lights.as_ref(), 0);
    moby_render::write_point_light(&mut record, moby_render::point_light_merge(&point_lights.0, sphere_centre(hm), &rows3(&hm.rows)));
    let t = Transform::from_matrix(model);
    for &e in &p.entities {
        if let Ok(mut tr) = transforms.get_mut(e) { *tr = t; }
    }
    // Hidden in first person (mode bit 1, `HeroSyncMoby` 0x229f20 → 0x2486c0).
    let hidden = hm.mode & rc_game::moby_runtime::mode::HIDDEN != 0;
    if hidden != p.ratchet_hidden {
        p.ratchet_hidden = hidden;
        let v = if hidden { Visibility::Hidden } else { Visibility::Inherited };
        for &e in &p.entities { commands.entity(e).insert(v); }
    }
    if let Some(mut buf) = buffers.get_mut(&p.extra.palette) { buf.data = Some(palette); }
    if let Some(mut buf) = buffers.get_mut(&p.extra.instances) { buf.data = Some(record); }
    let cam = cams.iter().next().map(|t| {
        let [fwd, left, up] = crate::game_camera::game_rows(t);
        (crate::game_camera::game_eye(t), crate::moby_lod::camera_rows(fwd, left, up))
    });
    upload_dynamic(&mut p, lv, cam, &mut commands, &mut buffers, &mut meshes, &mut images, &mut materials, &point_lights.0);
}

/// moby+0x00 (the sphere centre, integer units) in game units; the position when the moby has no sphere.
fn sphere_centre(m: &Moby) -> [f32; 3] {
    if m.bsphere[3] == 0.0 { return pos3(m); }
    [m.bsphere[0] / 1024.0, m.bsphere[1] / 1024.0, m.bsphere[2] / 1024.0]
}

/// The mobys in the dynamic slots (module doc): per slot, drawn when live (state < 0x80), not hidden
/// (mode & 0x81) and passing MobyProc's draw-distance / near / frustum culls (`moby_lod::moby_proc` on the
/// sequence sphere); its record (model, light block for its rows / light word / ambient) and palette are
/// rewritten, its (slot, class) entities spawned on first use and shown, the others hidden. The ship's slot
/// is drawn by the static path.
#[allow(clippy::too_many_arguments)]
fn upload_dynamic(
    p: &mut Play,
    lv: &crate::level_load::LoadedLevel,
    cam: Option<(Vec3, [Vec3; 3])>,
    commands: &mut Commands,
    buffers: &mut Assets<ShaderBuffer>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    point_lights: &[rc_game::point_lights::PointLight],
) {
    let m = &lv.mobys;
    let table = &p.game.mobys;
    let first = table.first_dynamic;
    let d = &mut p.dynamic;
    let ship = p.ship.map(|s| s.0);
    let (mut rec_changed, mut pal_changed) = (false, false);
    let (mut live, mut drawn) = (0, 0);
    let pal_bytes = d.pal_slots as usize * 64;
    for slot in 0..d.shown.len() {
        let id = first + slot;
        let Some(mo) = table.mobys.get(id) else { break };
        if Some(id) == ship { continue; }
        let ci = (mo.state < 0x80).then(|| d.class_ix.get(&mo.o_class).copied()).flatten();
        if ci.is_some() { live += 1; }
        // MobyProc: the culls, then the vertex alpha (fade × +0x23) and the blend (moby_render::MobyBlend).
        let pick = ci.and_then(|ci| {
            if mo.mode & 0x81 != 0 { return None; }
            let Some((eye, rows)) = cam else { return Some((ci, mo.alpha, false)) };
            let c = &m.classes[ci].class.header;
            let inp = crate::moby_lod::ProcInput {
                position: pos3(mo),
                rows: rows3(&mo.rows),
                scale: mo.scale,
                draw_distance: mo.draw_dist as i32,
                lod_trans: c.lod_trans,
                shine_distance: 0,
                alpha: mo.alpha,
            };
            let sphere = crate::moby_lod::world_sphere(&inp, crate::moby_lod::seq_sphere(&m.anim[ci], &mo.anim, c.bsphere));
            let v = crate::moby_lod::view_centre(sphere, eye, &rows);
            crate::moby_lod::moby_proc(v, sphere[3], &inp).ok().map(|p| (ci, p.alpha, p.fading))
        });
        let ci = pick.map(|p| p.0);
        d.visible[slot] = ci.is_some() as u8;
        d.shown[slot] = ci;
        let model = moby_render::extra_model(rows3(&mo.rows), mo.scale, pos3(mo));
        let look = pick.map(|(ci, alpha, fading)| (&m.classes[ci], moby_render::SlotLook { model, alpha, fading, mode: mo.mode }));
        d.extra.show_slot(commands, lv, slot as u32, look, meshes, images, materials, buffers);
        let Some(ci) = ci else { continue };
        drawn += 1;
        let rows = rows_bits(&mo.rows);
        let lights = m.lighting.as_ref().map(|l| light::moby_lights(&rows, &l.bank, mo.light, [mo.ambient[0], mo.ambient[1], mo.ambient[2]], 0x80));
        let mut rec = moby_render::extra_record(&model, lights.as_ref(), slot as u32 * d.pal_slots);
        moby_render::write_point_light(&mut rec, moby_render::point_light_merge(point_lights, sphere_centre(mo), &rows3(&mo.rows)));
        let at = slot * moby_render::EXTRA_RECORD_SIZE;
        if d.records[at..at + rec.len()] != rec[..] {
            d.records[at..at + rec.len()].copy_from_slice(&rec);
            rec_changed = true;
        }
        let snap = p.svc.snapshots.get(id).and_then(|s| s.as_ref());
        let f = moby_anim::evaluate_with_snapshot(&m.anim[ci], &mo.anim, snap);
        let bytes: Vec<u8> = f.iter().take(d.pal_slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).collect();
        let at = slot * pal_bytes;
        if d.palette[at..at + bytes.len()] != bytes[..] {
            d.palette[at..at + bytes.len()].copy_from_slice(&bytes);
            pal_changed = true;
        }
    }
    (d.live, d.drawn) = (live, drawn);
    if rec_changed {
        if let Some(mut buf) = buffers.get_mut(&d.extra.instances) { buf.data = Some(d.records.clone()); }
    }
    if pal_changed {
        if let Some(mut buf) = buffers.get_mut(&d.extra.palette) { buf.data = Some(d.palette.clone()); }
    }
}

/// +0x31 after drawing (MobyProc writes it every frame): 1 for a moby drawn this frame, 0 on every skip
/// (occlusion, the draw-distance / near / frustum culls, hidden by its update, spawn-hidden, no geometry).
/// The moby loop of the next tick reads it (`fun_0020d868`'s activity rule).
fn write_visible(play: Option<ResMut<Play>>, occl: Option<Res<MobyOcclusion>>, anim: Option<Res<MobyAnim>>) {
    let (Some(mut p), Some(occl)) = (play, occl) else { return };
    let p = &mut *p;
    let hidden = |ii: usize| -> bool { occl.anim_index(ii).zip(anim.as_ref()).is_some_and(|(k, a)| a.hidden[k]) };
    let drawn = |ii: usize| -> u8 { (occl.anim_index(ii).is_some() && occl.drawn(ii) && !hidden(ii)) as u8 };
    let t = &mut p.game.mobys;
    let first = t.first_dynamic;
    for (id, m) in t.mobys[..first].iter_mut().enumerate() { m.visible = p.moby_to_instance.get(id).map_or(0, |&ii| drawn(ii)); }
    // Ratchet is drawn by his extra instance.
    t.mobys[p.hero_id].visible = 1;
    for (slot, &v) in p.dynamic.visible.iter().enumerate() {
        if let Some(m) = t.mobys.get_mut(first + slot) { m.visible = v; }
    }
    if let Some((id, ii)) = p.ship { t.mobys[id].visible = drawn(ii); }
}
