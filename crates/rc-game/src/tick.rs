//! One gameplay tick in the game's order (level01 `FUN_002aba68`, called from the main loop after
//! `UpdatePad`): pad → mobys → hero `0x228870` → particles → camera `0x20eca8` → tick counter `0x15f5cc`++.
//! Spec: `docs/plan/player_controller.md` §0.
//!
//! The moby updates and the particles are callbacks (their ports live elsewhere); the free-slot pass
//! 0x263300 runs before them as in the game. Point lights and the help system are not modelled.
//!
//! **One `rand` stream** ([`Game::rng`], `srand(1234)` at level init): the moby hook gets it (the scheduler's
//! classes and the external updates draw from it in run order), then the hero, then the particle hook
//! (`UpdateParts`); the camera draws nothing; then the sound step; the render phase (sky stars) draws after the
//! counter increment.
//!
//! **Moby collision** ([`MobySystem`], the moby system's state outside the table: grid, class blobs, pose cache,
//! hit log): the hero's queries read the table as the moby loop left it ([`Env::mobys`], Ratchet ignored); the
//! write-back's `MobyBuildMatrix(Ratchet)` (HeroSyncMoby 0x229f20) rebuilds his bounding sphere and grid cells;
//! the camera's queries read the table after that ([`CamInput::mobys`]) and a crate blocking its line gets a hit.
//!
//! **Catch-up rule** (main loop `entry` 0x259c40): after a rendered frame, if RCNT1 counted more than one
//! 60 Hz field (0x2580), the game runs one extra `UpdatePad` + gameplay tick — at most one. The caller
//! decides how many ticks to run per rendered frame; [`ticks_for_frame`] is the game's rule.
//!
//! **Sound** ([`Game::tick_with_sound`]): the sound step runs after the camera and before the counter
//! increment on the same stream (the occlusion origin's 3 draws every tick, the pitch bends of new plays);
//! the moby loop's class sounds reach the audio layer through its `SoundSink` during the moby hook.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::assign_op_pattern)] // FPU compare semantics and op order are spelled out on purpose.

use crate::follow_camera::{CamInput, Camera, CameraOptions, CameraView};
use crate::hero::items::{items_update, HitSink, ItemData, ItemEnv, ItemGlobals, NoHits};
use crate::hero::{hero_update_with_sounds, AnimCtl, Env, Hero, HeroSounds, HeroTick, NoHeroSounds};
use crate::collision_query::OwnedScene;
use crate::moby_runtime::{MobyId, MobyTable};
use crate::moby_update::services::{HitRecord, HitTemplate};
use crate::pad::PadState;
use crate::ps2v::Pf;
use crate::rng::Rng;
use rc_formats::collision::Collision;

/// RCNT1 counts in one NTSC field (the main loop's catch-up threshold).
pub const FIELD_RCNT1: u32 = 0x2580;

/// The main loop's rule: 2 ticks when the frame took longer than one field (RCNT1 > 0x2580), else 1.
pub fn ticks_for_frame(rcnt1_elapsed: u32) -> u32 { if rcnt1_elapsed > FIELD_RCNT1 { 2 } else { 1 } }

/// Game-wide options the tick reads.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameOptions {
    /// 0x15edb4: mirrored controls.
    pub mirror: bool,
    pub camera: CameraOptions,
}

/// The gameplay state one tick advances.
pub struct Game {
    pub pad: PadState,
    pub mobys: MobyTable,
    /// Ratchet's moby in [`Game::mobys`] (`+0xa6 == 0`).
    pub hero_moby: MobyId,
    pub hero: Hero,
    pub camera: Camera,
    pub rng: Rng,
    /// `0x15f5cc`: gameplay tick counter.
    pub counter: u64,
    pub options: GameOptions,
    /// `0x15f638`: the level's death height (gameplay header +0x28).
    pub death_z: Pf,
    /// The item definitions and classes of the hand items (None: no hand item is created, as before the
    /// melee port).
    pub item_data: Option<ItemData>,
    /// The hand-swap globals outside the hero block (the engine syncs them with the saved game / session).
    pub item_globals: ItemGlobals,
    /// The tick in its mode-2 form (the in-engine scene frame, `CutsceneModeUpdate` 0x2aca80): the moby loop, the
    /// hero, the particles, the sound step and the counter run, but never `CameraUpdate` 0x20eca8 (and its crate hit),
    /// whose springs resume where they were on the first mode-0 tick, nor the free-slot pass `MobyFreeSlotBookkeeping`
    /// 0x263300 (only `InLevelFrameUpdate` runs it).
    pub camera_paused: bool,
    /// The level's grind paths (gameplay section 0x74) the hero rides (`hero::boots`; empty: no rails).
    pub grind_paths: std::sync::Arc<Vec<rc_formats::volumes::GrindPath>>,
}

/// The moby hook: `(table, hero, rng, camera, collision, counter)`.
pub type MobyHook<'a> = dyn FnMut(&mut MobyTable, &Hero, &mut Rng, &CameraView, &Collision, u64) + 'a;

/// The particle hook: `(hero, camera, rng, counter)`.
pub type PartsHook<'a> = dyn FnMut(&Hero, &CameraView, &mut Rng, u64) + 'a;

/// The sound step (`sound_update` 0x2a0638 and the class sounds of the tick, e.g.
/// `audio::class_sounds::sound_step`): `(table, hero, camera, rng, counter)` with this tick's camera, after
/// the camera update and before the counter increment.
pub type SoundHook<'a> = dyn FnMut(&MobyTable, &Hero, &CameraView, &mut Rng, u64) + 'a;

/// The moby system's state the hero and the camera reach outside the table (`moby_update::Services`: the grid
/// 0x19bc60, the class collision blobs, the pose cache and snapshots, the hit log), e.g.
/// `moby_update::services::SharedServices`.
pub trait MobySystem {
    /// What the collision kernels read of `table` now (a snapshot; None: no moby collision, world only).
    fn scene(&mut self, table: &MobyTable) -> Option<OwnedScene>;
    /// `MobyBuildMatrix` 0x265bd8 on moby `id`: matrix, bounding sphere, grid re-registration.
    fn build_matrix(&mut self, table: &mut MobyTable, id: MobyId);
    /// `FUN_0026e968(target, tmpl)`: deliver a hit.
    fn deliver_hit(&mut self, table: &mut MobyTable, target: MobyId, tmpl: &HitTemplate);
    /// The level's water-height tables for the hero's ground probe (`0x26ed38`: the class-751 ripple patches;
    /// None: the water faces' own heights).
    fn water(&self) -> Option<&dyn crate::hero::swim::WaterQuery> { None }
    /// The hit message of moby `target` (`MobyGetHitMessage`'s lookup: the record of slot `+0xa4` when it is
    /// for `target`; None: no record). The hero's hit intake reads Ratchet's (docs/plan/hero_states.md P2).
    fn hit_message(&self, _table: &MobyTable, _target: MobyId) -> Option<HitRecord> { None }
    /// The hero-block fields the moby loop's class updates wrote this tick (`moby_update::services::HeroFields`;
    /// None: none), applied to the hero right after the moby loop.
    fn take_hero_writes(&mut self) -> Option<crate::moby_update::services::HeroFields> { None }
    /// The camera shake requests the moby loop's class updates made this tick (`World::shake_camera`), in order;
    /// the tick stores them into the camera's shake records right after the moby loop.
    fn take_camera_shakes(&mut self) -> Vec<crate::follow_camera::ShakeRequest> { Vec::new() }
    /// The cinematic camera calls the moby loop made this tick (`CameraScript` / `CameraScript2` …,
    /// [`crate::cinematic`]), in order; the tick applies them to the camera right after the moby loop.
    fn take_cinematic(&mut self) -> Vec<crate::cinematic::CinematicCall> { Vec::new() }
    /// The moby loop's run list for the camera at `camera` (the `0x15ffe4` chain the hero's Swingshot target
    /// searches walk: `moby_update::scheduler::build_active_list`); None: every live moby in table order.
    fn run_list(&self, _table: &MobyTable, _camera: crate::hero::physics::V4) -> Option<Vec<MobyId>> { None }
    /// The level's volume sections (the cuboids the Swingshot targets' records name); None: none.
    fn volumes(&self) -> Option<std::sync::Arc<rc_formats::volumes::Volumes>> { None }
}

/// Callbacks for the subsystems ported elsewhere.
pub struct TickHooks<'a> {
    /// The moby update loop (0x279470 …, `moby_update::Scheduler::tick`), after the free-slot pass: the table,
    /// the hero block (last tick's), the game's one `rand` stream, the camera as the previous tick's camera
    /// update left it (0x167240 …), the level collision and the tick counter.
    pub mobys: &'a mut MobyHook<'a>,
    /// `UpdateParts` (particles), between the hero and the camera: the hero, the camera as the previous tick's
    /// camera update left it (0x167240, read by the type-11 sparks), the game's `rand` stream, the tick counter.
    pub particles: &'a mut PartsHook<'a>,
    /// The moby collision for the hero and the camera (None: their queries are world only, Ratchet's grid
    /// registration stays as loaded, the camera hits nothing).
    pub world: Option<&'a mut dyn MobySystem>,
}

/// What one tick did.
#[derive(Clone, Copy, Debug)]
pub struct TickReport {
    pub hero: HeroTick,
    pub camera: CameraView,
    /// The 30-tick blocked reset of the camera fired this tick.
    pub camera_reset: bool,
}

impl Game {
    /// A game on `coll` in the loader's order (`InitLevelRenderGlobals` 0x255958): `srand(1234)` (its line
    /// 79, before the instance loop), then the hero init `HeroInit` 0x226b70 on its moby (index `hero_moby` in
    /// `mobys`, ground-snapped by [`Hero::init_from_moby`], which takes the stream's first draw for the fidget
    /// timer), and the camera snapped behind it. The tick counter is 0 until [`Game::finish_load`].
    pub fn new(coll: &Collision, mobys: MobyTable, hero_moby: MobyId, options: GameOptions, death_z: f32) -> Game {
        let mut rng = Rng::new();
        rng.srand(crate::rng::LEVEL_SEED);
        Game::with_rng(coll, mobys, hero_moby, options, death_z, rng)
    }

    /// [`Game::new`] continuing the stream `rng` (a respawn's hero init draws from the running stream).
    pub fn with_rng(coll: &Collision, mut mobys: MobyTable, hero_moby: MobyId, options: GameOptions, death_z: f32, mut rng: Rng) -> Game {
        let hero = Hero::init_from_moby(&mut mobys.mobys[hero_moby], coll, &mut rng);
        let pad = PadState::default();
        let camera = Camera::new(&CamInput { hero: &hero, pad: &pad, coll, mobys: None, hero_moby: None }, options.camera);
        Game { pad, mobys, hero_moby, hero, camera, rng, counter: 0, options, death_z: Pf::f(death_z), item_data: None, item_globals: ItemGlobals::default(), camera_paused: false, grind_paths: Default::default() }
    }

    /// The end of the level load (`LoadLevelCoreData` 0x258128): `0x15f5cc++` right after
    /// `MobyLoadTimeUpdatePass`, so the load pass runs at counter 0 and the first gameplay tick sees 1. Call it
    /// once, after the caller's `Scheduler::load_pass`.
    pub fn finish_load(&mut self) { self.counter += 1; }

    /// One gameplay tick: `UpdatePad` with `pad_data` (None = disconnected), then the tick body. The hero's
    /// attacks hit nothing ([`Game::tick_with_hits`] takes a sink).
    pub fn tick(&mut self, pad_data: Option<&[u8]>, coll: &Collision, anim: &mut dyn AnimCtl, hooks: &mut TickHooks) -> TickReport {
        self.tick_with_hits(pad_data, coll, anim, hooks, &mut NoHits)
    }

    /// [`Game::tick`] with the hit sink the hero's attacks (the wrench) deliver to.
    pub fn tick_with_hits(&mut self, pad_data: Option<&[u8]>, coll: &Collision, anim: &mut dyn AnimCtl, hooks: &mut TickHooks, hits: &mut dyn HitSink) -> TickReport {
        self.tick_with_sound(pad_data, coll, anim, hooks, hits, None)
    }

    /// [`Game::tick_with_hits`] with the sound step `sound` (None: no sound layer) after the camera and before
    /// the counter increment, in game modes 0 and 2 alike (`sound_update` runs in both). The hero's own sounds
    /// are not played ([`Game::tick_with_hero_sounds`]).
    pub fn tick_with_sound(
        &mut self,
        pad_data: Option<&[u8]>,
        coll: &Collision,
        anim: &mut dyn AnimCtl,
        hooks: &mut TickHooks,
        hits: &mut dyn HitSink,
        sound: Option<&mut SoundHook>,
    ) -> TickReport {
        self.tick_with_hero_sounds(pad_data, coll, anim, hooks, hits, sound, &mut NoHeroSounds)
    }

    /// [`Game::tick_with_sound`] with the hero's sounds (`hero_update_with_sounds`: the class sound of Ratchet's
    /// animation trigger right after his advance, the hurt / death voices after the transitions) played at
    /// their points inside the hero update through `hero_sounds`.
    #[allow(clippy::too_many_arguments)]
    pub fn tick_with_hero_sounds(
        &mut self,
        pad_data: Option<&[u8]>,
        coll: &Collision,
        anim: &mut dyn AnimCtl,
        hooks: &mut TickHooks,
        hits: &mut dyn HitSink,
        sound: Option<&mut SoundHook>,
        hero_sounds: &mut dyn HeroSounds,
    ) -> TickReport {
        self.pad.update(pad_data, self.options.mirror);
        if !self.camera_paused { self.mobys.free_slot_pass(self.counter); }
        // The globals the moby loop reads outside the moby system: this tick's pad, the last camera update's Euler,
        // Ratchet's anim fields after his last update (moby_update::services::LoopGlobals).
        let e = self.camera.out.euler;
        self.hero.loop_in = crate::moby_update::services::LoopGlobals { pad: self.pad.clone(), cam_euler: [e[0].to_f32(), e[1].to_f32(), e[2].to_f32()], anim: anim.view() };
        (hooks.mobys)(&mut self.mobys, &self.hero, &mut self.rng, &self.camera.out, coll, self.counter);
        // The classes' stores into the hero block (the flow 679's push, the lift's lockouts …) land before the hero
        // update, as in the game (moby_update::services::HeroFields).
        let hero_writes = hooks.world.as_deref_mut().and_then(|w| w.take_hero_writes());
        if let Some(f) = &hero_writes { f.apply(&mut self.hero); }
        // Their camera shakes (stores into 0x167260 / 0x167270; the camera update at the end of the tick applies them).
        for r in hooks.world.as_deref_mut().map(|w| w.take_camera_shakes()).unwrap_or_default() { self.camera.request_shake(r); }
        let hero_moby = Some(self.hero_moby);
        // Their cinematic camera calls (CameraScript, CameraScript2, HeroTeleport's camera reset: crate::cinematic).
        let cine = hooks.world.as_deref_mut().map(|w| w.take_cinematic()).unwrap_or_default();
        crate::cinematic::apply_camera_calls(&mut self.camera, &cine, &CamInput { hero: &self.hero, pad: &self.pad, coll, mobys: None, hero_moby });
        // Ratchet's hit message as the moby loop left it (the hit intake 0x231580 and the hurt entries read it).
        self.hero.damage.hit = hooks.world.as_deref().and_then(|w| w.hit_message(&self.mobys, self.hero_moby)).map(|r| hero_hit(&self.mobys, &r));
        // The hero's queries see the table as the moby loop left it (a snapshot: the hero holds its own moby).
        let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
        // The carriers' platform blocks as the moby loop left them (`HeroPlatformUpdate`, hero::platform).
        let mut carriers = crate::hero::platform::Carriers::collect(&self.mobys, self.hero_moby);
        carriers.grind = self.grind_paths.clone();
        // The Swingshot targets of the moby loop's run list (hero::swingshot, the weapon check's searches).
        carriers.targets = self.swing_targets(hooks.world.as_deref());
        // The weapon's target (0x13fda0) where the moby loop left it (SetState 0x23 aims at it).
        crate::hero::weapons::refresh_aim(&mut self.hero, &self.mobys);
        let hero_tick = {
            let mobys = scene.as_ref().map(OwnedScene::scene);
            let view = self.camera.out;
            let env = Env {
                coll,
                pad: &self.pad,
                cam_yaw: view.yaw(),
                cam_rows: view.rows,
                mirror: self.options.mirror,
                death_z: self.death_z,
                mobys: mobys.as_ref(),
                hero_moby,
                water: hooks.world.as_deref().and_then(|w| w.water()),
                world: Some(&carriers),
            };
            // The classes' calls into the hero code (SetState / SetAnim: the bolt crank), with the hero's context.
            if let Some(f) = &hero_writes { f.run_calls(&mut self.hero, &mut crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None }); }
            crate::cinematic::run_hero_calls(&mut self.hero, &cine, &mut crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None });
            let moby = &mut self.mobys.mobys[self.hero_moby];
            hero_update_with_sounds(&mut self.hero, moby, &env, anim, &mut self.rng, hero_sounds)
        };
        drop(scene);
        // The mobys the hero update created (CreateMoby inside it: the water splash 775), before anything else can take a
        // slot, with their MobyBuildMatrix (hero::fx::create_mobys).
        if !self.hero.fx.mobys.is_empty() {
            for id in crate::hero::fx::create_mobys(&mut self.hero, &mut self.mobys, self.hero_moby, hits, self.counter) {
                if let Some(w) = hooks.world.as_deref_mut() { w.build_matrix(&mut self.mobys, id); }
            }
        }
        // The hero's camera shakes (the stomp's landing, …: its stores into 0x167260 / 0x167270 during the update).
        for r in std::mem::take(&mut self.hero.fx.shakes) { self.camera.request_shake(r); }
        // HeroSyncMoby 0x229f20: Ratchet's hit slot +0xa4 = 0xff (the message is consumed by this update).
        if hero_tick != HeroTick::OutOfBounds { self.mobys.mobys[self.hero_moby].hit_slot = 0xff; }
        self.hero.damage.hit = None;
        if hero_tick == HeroTick::Ran {
            // The write-back's MobyBuildMatrix(Ratchet) (HeroSyncMoby 0x229f20): bounding sphere from his
            // animation fields (+0x50..0x54, the moby's own in the game), matrix, grid re-registration.
            let v = anim.view();
            let a = &mut self.mobys.mobys[self.hero_moby].anim;
            (a.seq_a, a.seq_b, a.frame_a, a.frame_b, a.t) = (v.seq_a, v.seq_b, v.frame_a, v.frame_b, v.t);
            if let Some(w) = hooks.world.as_deref_mut() { w.build_matrix(&mut self.mobys, self.hero_moby); }
        }
        // The hits the pack states queued in their physics (the stomp's descent, the Thruster long jump's crates).
        if !self.hero.packs.hits.is_empty() { crate::hero::packs::deliver_hits(&mut self.hero, &mut self.mobys, self.hero_moby, hits); }
        // HeroItemsUpdate 0x231268 (hand slot): create, attach, the swap, the item's update (the wrench's hit).
        if hero_tick == HeroTick::Ran {
            if let Some(data) = self.item_data.as_ref() {
                // 0x1abe80, the targetable mobys of the moby loop's run list (crate::targeting), for the items' aim searches.
                let targets = if self.hero.items.slot.item.is_some() { self.target_list(hooks.world.as_deref()) } else { Vec::new() };
                let ienv = ItemEnv { data, pad: &self.pad, frame: self.counter as i32, hero_moby: self.hero_moby, coll: Some(coll), camera: Some((self.camera.out.pos_f32(), self.camera.out.rows_f32()[0])), camera_up: Some(self.camera.out.rows_f32()[2]), targets: &targets };
                items_update(&mut self.hero, &mut self.item_globals, &mut self.mobys, &*anim, &mut self.rng, &ienv, hits);
                // The slot loop's item update that needs the hero's context (the Swingshot's hook: SetState, the
                // collision lines), at the same point of the frame (hero::gadgets).
                if self.hero.gadgets.pending.is_some() || self.hero.swing.item.alive || self.hero.weapons.deferred.is_some() || self.hero.weapons.pending_draw || self.hero.weapons.pending_idle {
                    let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
                    let mobys = scene.as_ref().map(OwnedScene::scene);
                    let view = self.camera.out;
                    let env = Env {
                        coll,
                        pad: &self.pad,
                        cam_yaw: view.yaw(),
                        cam_rows: view.rows,
                        mirror: self.options.mirror,
                        death_z: self.death_z,
                        mobys: mobys.as_ref(),
                        hero_moby,
                        water: hooks.world.as_deref().and_then(|w| w.water()),
                        world: Some(&carriers),
                    };
                    let mut c = crate::hero::states::Ctx { env: &env, anim: &mut *anim, rng: &mut self.rng, voice: None };
                    crate::hero::gadgets::after_items(&mut self.hero, &mut c, data);
                }
                // The hand item's class sounds (the wrench's hit, the Swingshot's fire / hit / pull), right after its
                // update.
                crate::hero::gadgets::flush_item_sounds(&mut self.hero, &self.mobys.mobys[self.hero_moby], hero_sounds, &mut self.rng);
            }
            // FUN_00227e90: the walk / run footsteps (after HeroItemsUpdate in 0x228870).
            crate::hero::fx::walk_footsteps(&mut self.hero, &self.mobys.mobys[self.hero_moby], &anim.view(), hero_sounds, &mut self.rng);
        }
        (hooks.particles)(&self.hero, &self.camera.out, &mut self.rng, self.counter);
        let resets = self.camera.resets;
        if self.camera_paused {
            if let Some(f) = sound { f(&self.mobys, &self.hero, &self.camera.out, &mut self.rng, self.counter); }
            self.counter += 1;
            return TickReport { hero: hero_tick, camera: self.camera.out, camera_reset: false };
        }
        // The camera's queries see the table after the hero's write-back (Ratchet re-registered).
        let scene = hooks.world.as_deref_mut().and_then(|w| w.scene(&self.mobys));
        let camera = {
            let mobys = scene.as_ref().map(OwnedScene::scene);
            self.camera.update(&CamInput { hero: &self.hero, pad: &self.pad, coll, mobys: mobys.as_ref(), hero_moby })
        };
        drop(scene);
        // The first-person camera's store of 0x1413f5 (`0x316330`; every SetState clears it).
        if self.camera.first_person_flag() { self.hero.f13f5 = 1; }
        // A crate blocking the camera line (0x312ef8): FUN_0026e808(20.0, tmpl, Ratchet, 0x800000, dir), +0x18 /
        // +0x19 = 3, +0x1a = Ratchet's class, then FUN_0026e968.
        if let (Some((id, dir)), Some(w)) = (self.camera.hit, hooks.world.as_deref_mut()) {
            let h1a = self.mobys.mobys[self.hero_moby].o_class as u16;
            let tmpl = HitTemplate { dir, attacker: hero_moby, flags: 0x80_0000, b18: 3, b19: 3, h1a, damage: Pf::b(0x41a0_0000), w20: 1 };
            w.deliver_hit(&mut self.mobys, id, &tmpl);
        }
        // The sound step with this tick's camera (the listener 0x167240).
        if let Some(f) = sound { f(&self.mobys, &self.hero, &self.camera.out, &mut self.rng, self.counter); }
        self.counter += 1;
        TickReport { hero: hero_tick, camera, camera_reset: self.camera.resets != resets }
    }
}

impl Game {
    /// `0x1abe80` as the moby loop built it: the targetable mobys of its run list for the camera it ran with (the
    /// previous tick's), in order (`crate::targeting::target_list`); without a moby system, the table's order.
    fn target_list(&self, world: Option<&dyn MobySystem>) -> Vec<crate::moby_runtime::MobyId> {
        let order = world.and_then(|w| w.run_list(&self.mobys, self.camera.out.pos)).unwrap_or_else(|| {
            self.mobys.mobys.iter().enumerate().take_while(|(_, m)| m.state != crate::moby_runtime::state::END).map(|(i, _)| i).collect()
        });
        crate::targeting::target_list(&self.mobys, &order)
    }

    /// The Swingshot targets the hero sees this tick: the target mobys of the moby loop's run list (in its order)
    /// with their records and cuboids, and the camera as the previous tick's update left it.
    fn swing_targets(&self, world: Option<&dyn MobySystem>) -> crate::hero::swingshot::Targets {
        use crate::hero::swingshot::{Targets, PULL_CLASS, SWING_CLASS};
        let view = self.camera.out;
        let (cam, yaw, pitch) = (view.pos_f32(), view.yaw().to_f32(), view.euler[1].to_f32());
        if !self.mobys.mobys.iter().any(|m| m.o_class == PULL_CLASS || m.o_class == SWING_CLASS) { return Targets { camera: cam, cam_yaw: yaw, cam_pitch: pitch, ..Targets::default() }; }
        let order = world.and_then(|w| w.run_list(&self.mobys, view.pos)).unwrap_or_else(|| {
            self.mobys.mobys.iter().enumerate().take_while(|(_, m)| m.state != crate::moby_runtime::state::END).map(|(i, _)| i).collect()
        });
        let volumes = world.and_then(|w| w.volumes());
        Targets::collect(&self.mobys, &order, volumes.as_deref(), cam, yaw, pitch)
    }
}

/// A hit record for the hero, with what the hero code reads of the attacker (class +0xa6, position +0x10,
/// mission +0xb0) out of the table.
fn hero_hit(table: &MobyTable, r: &HitRecord) -> crate::hero::damage::HeroHit {
    let attacker = r.attacker.and_then(|id| table.mobys.get(id).map(|m| crate::hero::damage::Attacker {
        id,
        o_class: m.o_class,
        pos: [m.position[0], m.position[1], m.position[2]],
        mission: m.mission,
    }));
    crate::hero::damage::HeroHit {
        attacker,
        flags: r.flags,
        b28: r.b28,
        damage: r.damage.to_f32(),
        w30: r.w30,
        dir: [r.dir[0].to_f32(), r.dir[1].to_f32(), r.dir[2].to_f32(), r.dir[3].to_f32()],
    }
}
