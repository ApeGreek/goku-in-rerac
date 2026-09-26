//! The items the hero code keeps on Ratchet (docs/formats/moby_rac1.md §0.4 "In the port"): the wrench in
//! his hand, and on his back Clank (class 601) plus the pack moby of the back slot (the Heli-Pack 607, the
//! Thruster-Pack 608 or the Hydro-Pack 609: whichever `Hero::back` holds).
//!
//! Which items (level01.elf = the in-game engine; the port loads a level with the boot ELF's initial data
//! and an empty save):
//! * `FUN_0022f3c0` (0x22f3c0, called every frame from the hero update `FUN_00228870` → `FUN_00231268`)
//!   creates the hero's item mobys once. Hand: item `0x141424` (temporary item, 0) → else **8 (wrench)**
//!   while `0x15ed90` ("the wrench is the held item", 1 in the boot ELF's data, set again by `FUN_002307e0`
//!   whenever the wrench is selected) is non-zero → else the saved weapon `0x141660` → else 8. A wrench
//!   hand moby then gets `fun_00212f90(moby, 1, 0, 1)` (sequence 1, one-tick blend). Back: the pack moby
//!   (`0x1404d0`) of item `0x141430` → else the saved back item `0x14166c` → else **2** (3 when `0x15ed94`,
//!   0 in the boot ELF) = class **607**, and **Clank** (`0x1404d4`, `CreateMoby(def[1].o_class = 601)`)
//!   always; both hidden (mode |= 0x41) only while `0x141628` ≠ 0. Head (`0x141668`) and feet
//!   (`0x141664`) items are created only when non-zero: none.
//! * Every item moby copies Ratchet's moby+0x38..0x3f (light sets, cross-fade, ambient) at creation and
//!   every frame (`FUN_0022fec0`), so it is lit with Ratchet's light selection and ambient; its own
//!   rotation rows give the model-space light vectors.
//!
//! Per 60 Hz tick, in the hero update's order (`FUN_00228870`), after Ratchet's advance:
//! 1. The back items' animation: **with the game tick** it is the hero's (`rc_game::hero::idle`, `Hero::back`:
//!    created by `HeroItemsCreate`, advanced every tick, blended by the back table of `SetAnim` / the advance,
//!    `FUN_00242930`'s return to sequence 1, Clank's random fidgets `FUN_002473e0`); this module copies each
//!    back moby's state and pose snapshot and does not advance them. Without it (`RC_PLAY=0`, or before the
//!    first hero update) they are advanced here and loop their current sequence.
//! 2. `FUN_0022a940` → `fun_002646d0`: the pose `P` of Ratchet's joint lists {0, 1, 2, 3, 4, 5, 6, 29, 30}
//!    (`rc_formats::moby_anim::evaluate_chains`, `fun_00210850`/`fun_002109b8`), each turned into
//!    `W = [R | pos] · diag(scale/1024 on the translation) · P` (`attach_matrix`).
//! 3. `FUN_0022fec0`, each item: position = `W.r3` of its list (the wrench list 0 → joint 56, the back list
//!    5 → joint 5), `MobyAnimAdvance` of its own state, rotation rows = `W.r0..r2`, then the three columns
//!    normalised (`FUN_00271030`) — the item keeps its own class scale (moby+0x2c) and Ratchet's joint
//!    scale does not reach it. (Gloves 10/17/20/25, head items 5–7 and boots 28/29 instead copy Ratchet's
//!    finger / head / foot joints into a synthetic keyframe, `FUN_0022a9c8`; not needed here, not ported.)
//!
//! Draw: the item's palette is `MobyAnimEval` of its own state (after its advance), its record the usual
//! `MobyInst` (crate::moby_render "Extra" block), in a record / palette buffer of its own.
//!
//! **With the game tick** (crate::gameplay) the hand item is the game's (`rc_game::hero::items`: created, swapped,
//! placed and animated by the hero update, the wrench's combo sequences, the bomb glove posed from Ratchet's hand):
//! this module only draws it, one entity set per hand class (wrench 71, bomb glove 192), shown while the slot holds it.
//! Without it (`RC_PLAY=0`) the wrench is placed and advanced here as before.
//!
//! **The Swingshot** (item 12, class 0xd0 on list 1) is a hand item like the others; its hook (class 0xd1, a moby
//! of its own in the game, kept with the hand item by `rc_game::hero::swingshot`) is drawn at the hook's position
//! and rows while the Swingshot is in hand, and the rope between them by [`rope_draw`] (the game's `0x2dba30`).
//!
//! Not modelled: Clank's antenna glow moby
//! (class 1204 on Clank's joint list 6 with a pulsing +0x90 colour).
//!
//! `RC_ATTACH=0` disables the items. With `RC_ANIM=0` (Ratchet frozen in the bind pose) they are not
//! spawned either.

use crate::moby_anim::MobyAnim;
use crate::moby_render::{self, ExtraMobys, MobyMaterial, MobyOcclusion};
use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use bevy::render::storage::ShaderBuffer;
use rc_formats::gadget;
use rc_formats::moby::LevelMobyClass;
use rc_formats::moby_anim::{self, AnimState, MobyAnimClass, MobyFrame, Rows};
use rc_formats::moby_light::{self as light, V4};

/// Ratchet's joint lists `FUN_0022a940` evaluates every frame (the 9 words at 0x208c70); the attach
/// matrix of list `HERO_LISTS[i]` lands at 0x13fe10 + 0x40·i, which the items index by their definition's
/// attach word (`+0x04` of the 0x4c-byte item definitions at 0x179f48).
const HERO_LISTS: [usize; 9] = [0, 1, 2, 3, 4, 5, 6, 29, 30];

/// `RC_ATTACH` (default on).
pub fn enabled() -> bool { !std::env::var("RC_ATTACH").is_ok_and(|v| v.trim() == "0") }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Hand,
    Back,
    /// Placed by the game (the Swingshot's hook), not by a host joint.
    Hook,
}

/// What an item hangs from: the host's gameplay instance, the hero slot, the host joint list (index into
/// [`HERO_LISTS`] = the definition's attach word) and whether its matrix columns are normalised.
#[derive(Component, Clone, Copy, Debug)]
pub struct AttachedTo {
    pub host: usize,
    pub slot: Slot,
    pub joint_list: usize,
    pub normalise: bool,
}

struct Item {
    name: &'static str,
    /// The item moby's class (hand items are matched to the game's hand slot by it).
    o_class: i16,
    /// Shown (a hand item is shown only while the game holds it).
    visible: bool,
    shown: Option<bool>,
    attach: AttachedTo,
    anim: MobyAnimClass,
    state: AnimState,
    snapshot: Option<MobyFrame>,
    /// moby+0x2c = the class scale (`CreateMoby`).
    scale: f32,
    /// First palette slot and slot count.
    base: u32,
    slots: u32,
    /// moby+0xc0.. rows and moby+0x10 position (game units) after the last update.
    rows: [V4; 3],
    position: [f32; 3],
    entities: Vec<Entity>,
}

#[derive(Resource)]
pub struct MobyAttach {
    host_k: usize,
    host_class: usize,
    /// (index into HERO_LISTS, first byte list = root-to-joint chain) for the lists Ratchet's class has.
    chains: Vec<(usize, Vec<u8>)>,
    host_rows: [V4; 3],
    host_pos: [f32; 3],
    host_scale: f32,
    host_light: u32,
    host_ambient: [u8; 3],
    items: Vec<Item>,
    extra: ExtraMobys,
    palette_len: u32,
    ticks: u64,
    uploaded: Option<u64>,
}

impl MobyAttach {
    /// Ratchet's moby+0xc0.. rows and +0x10 position after the hero's write-back (crate::gameplay moves him;
    /// without it they stay the placed instance's). Read by the next `update`.
    pub fn set_host(&mut self, rows: [V4; 3], position: [f32; 3]) {
        self.host_rows = rows;
        self.host_pos = position;
    }
}

pub struct MobyAttachPlugin;

impl Plugin for MobyAttachPlugin {
    fn build(&self, app: &mut App) {
        // Setup in the first PreUpdate: after moby_render's Startup spawn (MobyOcclusion) and moby_spawn's
        // PostStartup pass (which tags gameplay entities by MeshTag and must not see these).
        app.add_systems(PreUpdate, setup)
            // After moby_anim's FixedUpdate tick (Ratchet's advance) within the same fixed step.
            .add_systems(FixedPostUpdate, update)
            .add_systems(PostUpdate, (upload, rope_draw));
    }
}

/// Ratchet's class blob (joint lists) and the gadget classes, re-read from the level's core data.
pub(crate) fn load_blobs() -> Result<(Vec<u8>, Vec<gadget::GadgetClass>)> {
    let (root, index) = (crate::level_load::extracted_root(), crate::level_load::level_index());
    let read = |name: &str| crate::disc_source::level_file(&root, index, name);
    let data = rc_formats::wad::decompress(&read("core_data.bin")?).context("decompressing core_data")?;
    let core = rc_formats::level::parse_level_core(&read("core_index.bin")?, data.len()).context("parsing core index")?;
    let blk = core.blocks.iter().find(|b| b.name == "moby_class/0000").ok_or_else(|| anyhow!("no moby_class/0000 block"))?;
    let ratchet = data.get(blk.offset..blk.offset + blk.size).ok_or_else(|| anyhow!("moby_class/0000 out of range"))?.to_vec();
    let gadgets = gadget::parse_gadget_classes(&core, &data).context("parsing gadget classes")?;
    Ok((ratchet, gadgets))
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut done: Local<bool>,
    mut commands: Commands,
    level: Res<crate::Level>,
    occl: Option<Res<MobyOcclusion>>,
    anim: Option<Res<MobyAnim>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<MobyMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if *done { return; }
    let (Some(occl), Some(anim)) = (occl, anim) else { return };
    *done = true;
    if !enabled() { println!("moby attach: RC_ATTACH=0: no items on Ratchet"); return; }
    if !anim.enabled { println!("moby attach: animation disabled (RC_ANIM=0 / RC_MOBY_CPU_LIGHT=1): no items on Ratchet"); return; }
    match build(&mut commands, &level.0, &occl, &anim, &mut meshes, &mut images, &mut materials, &mut buffers) {
        Ok(a) => {
            let names: Vec<String> = a.items.iter().map(|i| format!("{} (list {} -> joint {})", i.name, HERO_LISTS[i.attach.joint_list], a.chains.iter().find(|c| c.0 == i.attach.joint_list).and_then(|c| c.1.last()).copied().unwrap_or(0))).collect();
            println!("moby attach: on Ratchet (gameplay instance {}): {}", a.items[0].attach.host, names.join(", "));
            commands.insert_resource(a);
        }
        Err(e) => warn!("moby attach: no items on Ratchet: {e:#}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    commands: &mut Commands,
    level: &crate::level_load::LoadedLevel,
    occl: &MobyOcclusion,
    anim: &MobyAnim,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<MobyMaterial>,
    buffers: &mut Assets<ShaderBuffer>,
) -> Result<MobyAttach> {
    let m = &level.mobys;
    let host_ii = m
        .instances
        .iter()
        .zip(&m.placed)
        .position(|(i, p)| i.o_class == gadget::RATCHET_O_CLASS && p.is_some())
        .ok_or_else(|| anyhow!("no placed class-0 instance"))?;
    let host_k = occl.anim_index(host_ii).ok_or_else(|| anyhow!("Ratchet has no animation slot"))?;
    if anim.hidden.get(host_k).copied().unwrap_or(false) { return Err(anyhow!("Ratchet is hidden after the load pass")); }
    let placed = m.placed[host_ii].unwrap();
    let inst = &m.instances[host_ii];
    let (ratchet_blob, gadgets) = load_blobs()?;
    let rc = &m.classes[placed.class].class;
    let chains: Vec<(usize, Vec<u8>)> =
        HERO_LISTS.iter().enumerate().filter_map(|(i, &l)| gadget::joint_list(&ratchet_blob, &rc.header, l).ok().map(|(a, _)| (i, a))).filter(|c| !c.1.is_empty()).collect();

    // The classes: the wrench from the gadget table (sequences in its own blob), 607 / 601 from the level.
    let wrench = gadgets.iter().find(|g| g.moby.o_class == gadget::WRENCH_O_CLASS).ok_or_else(|| anyhow!("no wrench (class 71) in the gadget table"))?;
    let wrench_anim = MobyAnimClass::new(&wrench.moby.class, moby_anim::parse_sequences(&wrench.blob, &wrench.moby.class).context("wrench sequences")?);
    let level_class = |o: i32| -> Result<(LevelMobyClass, MobyAnimClass)> {
        let ci = m.classes.iter().position(|c| c.o_class == o).ok_or_else(|| anyhow!("class {o} not on this level"))?;
        Ok((m.classes[ci].clone(), m.anim[ci].clone()))
    };
    // The pack mobys of the back items (2 Heli-Pack 607 is required; 3 Thruster-Pack 608 and 4 Hydro-Pack 609 when
    // the level has them) and Clank.
    let (pack, pack_anim) = level_class(BACK_PACK_O_CLASS)?;
    let more_packs: Vec<(&'static str, LevelMobyClass, MobyAnimClass)> =
        [("Thruster-Pack", 608), ("Hydro-Pack", 609)].into_iter().filter_map(|(n, o)| level_class(o).ok().map(|(c, a)| (n, c, a))).collect();
    let (clank, clank_anim) = level_class(CLANK_O_CLASS)?;
    let host = |slot, list| AttachedTo { host: host_ii, slot, joint_list: list, normalise: true };
    let glove = gadgets.iter().find(|g| g.moby.o_class == BOMB_GLOVE_O_CLASS).ok_or_else(|| anyhow!("no bomb glove (class 192) in the gadget table"))?;
    let glove_anim = MobyAnimClass::new(&glove.moby.class, moby_anim::parse_sequences(&glove.blob, &glove.moby.class).context("bomb glove sequences")?);
    let mut specs: Vec<(&'static str, LevelMobyClass, MobyAnimClass, AttachedTo)> = vec![
        ("wrench", wrench.moby.clone(), wrench_anim, host(Slot::Hand, WRENCH_ATTACH)),
        ("bomb glove", glove.moby.clone(), glove_anim, AttachedTo { host: host_ii, slot: Slot::Hand, joint_list: GLOVE_ATTACH, normalise: false }),
        ("back pack", pack, pack_anim, host(Slot::Back, BACK_ATTACH)),
    ];
    for (n, c, ac) in more_packs { specs.push((n, c, ac, host(Slot::Back, BACK_ATTACH))); }
    specs.push(("Clank", clank, clank_anim, host(Slot::Back, BACK_ATTACH)));
    // The Swingshot (item 12: class 0xd0, attach word 1) and its hook (0xd1), when the level's gadget table has them.
    for (name, o, slot, list) in [("Swingshot", SWINGSHOT_O_CLASS, Slot::Hand, SWINGSHOT_ATTACH), ("Swingshot hook", SWINGSHOT_HOOK_O_CLASS, Slot::Hook, SWINGSHOT_ATTACH)] {
        // The gadget table, else the level's own classes (the hook is a `CreateMoby(0xd1)` of the level).
        if let Some(g) = gadgets.iter().find(|g| g.moby.o_class == o) {
            let Ok(seqs) = moby_anim::parse_sequences(&g.blob, &g.moby.class) else { continue };
            specs.push((name, g.moby.clone(), MobyAnimClass::new(&g.moby.class, seqs), host(slot, list)));
        } else if let Ok((c, ac)) = level_class(o) {
            specs.push((name, c, ac, host(slot, list)));
        }
    }
    for s in &specs {
        if !chains.iter().any(|c| c.0 == s.3.joint_list) { return Err(anyhow!("Ratchet's class has no joint list {}", HERO_LISTS[s.3.joint_list])); }
    }

    let mut items = Vec::new();
    let mut palette_len = 0u32;
    let mut geometry = Vec::new();
    for (name, class, ac, attach) in specs {
        let slots = (ac.joint_count as u32).max(ExtraMobys::max_skinned_joint(&class) as u32 + 1).max(1);
        // CreateMoby: init_moby_instance's state; the wrench then blends to sequence 1 (FUN_0022f3c0).
        let mut state = AnimState::spawn(&ac);
        let mut snapshot = None;
        if attach.slot == Slot::Hand { moby_anim::set_sequence(&mut state, &ac, 1, 0, 1, &mut snapshot); }
        // Without the game tick only the wrench, the Heli-Pack and Clank show (the old viewer behaviour).
        let visible = !matches!(name, "bomb glove" | "Thruster-Pack" | "Hydro-Pack" | "Swingshot" | "Swingshot hook");
        items.push(Item {
            name, o_class: class.o_class as i16, visible, shown: None, attach, anim: ac, state, snapshot, scale: class.class.header.scale,
            base: palette_len, slots, rows: [[0; 4]; 3], position: [0.0; 3], entities: Vec::new(),
        });
        palette_len += slots;
        geometry.push(class);
    }
    let records = vec![0u8; items.len() * moby_render::EXTRA_RECORD_SIZE];
    let extra = ExtraMobys::new(level, records, crate::moby_anim::identity_palette(palette_len), buffers);
    let mut a = MobyAttach {
        host_k, host_class: placed.class, chains,
        host_rows: crate::moby_light::instance_rows(inst), host_pos: inst.position, host_scale: placed.scale,
        host_light: inst.light_word(), host_ambient: inst.ambient_rgb(),
        items, extra, palette_len, ticks: 0, uploaded: None,
    };
    // Placement before the first tick (the game creates the items in the first hero update).
    let host = &anim.instances[host_k];
    place(&mut a, &level.mobys.anim[placed.class], &host.state, anim.snapshots[host_k].as_ref(), false, false);
    for (slot, (item, class)) in a.items.iter_mut().zip(&geometry).enumerate() {
        let t = Transform::from_matrix(model_of(item));
        item.entities = a.extra.spawn(commands, level, class, slot as u32, t, item.name, meshes, images, materials);
        for &e in &item.entities { commands.entity(e).insert(item.attach); }
    }
    Ok(a)
}

/// Item definition attach words (0x179f48 + 0x4c·item, +0x04): wrench (item 8) 0, pack / Clank (items
/// 1–4) 5. Classes: item 2 = 607 (+0x08), Clank = item 1's 601.
const WRENCH_ATTACH: usize = 0;
/// Item 10 (bomb glove, class 192): attach word 6 (list 6), not normalised.
const GLOVE_ATTACH: usize = 6;
const BOMB_GLOVE_O_CLASS: i32 = 192;
const WRENCH_O_CLASS_I16: i16 = gadget::WRENCH_O_CLASS as i16;
const BACK_ATTACH: usize = 5;
/// Item 12 (the Swingshot, class 0xd0): attach word 1; its hook moby class 0xd1 (`0x2dcbf0`).
const SWINGSHOT_ATTACH: usize = 1;
const SWINGSHOT_O_CLASS: i32 = rc_game::hero::swingshot::SWINGSHOT_CLASS as i32;
const SWINGSHOT_HOOK_O_CLASS: i32 = rc_game::hero::swingshot::HOOK_CLASS as i32;
const BACK_PACK_O_CLASS: i32 = 607;
const CLANK_O_CLASS: i32 = 601;

fn rows_f32(rows: &[V4; 3]) -> [[f32; 3]; 3] { rows.map(|r| [0, 1, 2].map(|k| f32::from_bits(r[k]))) }

fn model_of(item: &Item) -> Mat4 { moby_render::extra_model(rows_f32(&item.rows), item.scale, item.position) }

/// Steps 2–3 of the tick (module docs): attach matrices from Ratchet's current state, then every item's
/// position, (optionally) advance, rows, column normalisation.
fn place(a: &mut MobyAttach, host_class: &MobyAnimClass, host: &AnimState, snap: Option<&MobyFrame>, advance_back: bool, advance_hand: bool) {
    let chains: Vec<&[u8]> = a.chains.iter().map(|c| c.1.as_slice()).collect();
    let ps = moby_anim::evaluate_chains(host_class, host, snap, &chains);
    let ws: Vec<(usize, Rows)> = a.chains.iter().zip(&ps).map(|(c, p)| (c.0, moby_anim::attach_matrix(p, &a.host_rows, a.host_pos, a.host_scale))).collect();
    for item in &mut a.items {
        if item.attach.slot == Slot::Hook { continue; }
        let Some(&(_, w)) = ws.iter().find(|(l, _)| *l == item.attach.joint_list) else { continue };
        item.position = [w[3][0], w[3][1], w[3][2]];
        let adv = if item.attach.slot == Slot::Back { advance_back } else { advance_hand && item.o_class == WRENCH_O_CLASS_I16 };
        if adv { moby_anim::advance(&mut item.state, &item.anim); }
        let mut rows: [V4; 3] = [0, 1, 2].map(|i| w[i].map(f32::to_bits));
        if item.attach.normalise { moby_anim::normalise_columns(&mut rows); }
        item.rows = rows;
    }
}

/// One 60 Hz tick, after Ratchet's `MobyAnimAdvance` (crate::moby_anim tick in FixedUpdate). With the game
/// tick running, the hand item is the game's (`rc_game::hero::items`: which item, its animation, rows and
/// position as `HeroItemsAttach` placed them) and the back items' animation is the hero's (`Hero::back`: pack
/// and Clank, with Clank's fidgets); the back items are placed here.
fn update(attach: Option<ResMut<MobyAttach>>, anim: Option<Res<MobyAnim>>, level: Res<crate::Level>, play: Option<Res<crate::gameplay::Play>>) {
    let (Some(mut a), Some(anim)) = (attach, anim) else { return };
    let hand = play.as_ref().map(|p| p.game.hero.items.slot.item.clone());
    // The Swingshot's hook (a moby of its own in the game: advanced every tick, placed by the item's update).
    let hook = play.as_ref().and_then(|p| p.game.hero.swing.item.hook.filter(|_| p.game.hero.swing.item.alive));
    // The back mobys' animation state and pose snapshot as the hero update left them (item slot 3: pack
    // 0x1404d0, Clank 0x1404d4). The pack shown is the one of the slot's item (`Back::pack_o_class`: the Heli-,
    // Thruster- or Hydro-Pack), none while the slot is empty between a put-away and the next creation; Clank
    // hidden (0x141628) hides both (mode |= 0x41).
    let back = play.as_ref().and_then(|p| p.game.hero.back.as_ref().map(|b| (b, p.game.hero.back_slot.clank_hidden != 0)));
    if let Some((b, hidden)) = back {
        for item in a.items.iter_mut().filter(|i| i.attach.slot == Slot::Back) {
            let m = if item.o_class == CLANK_O_CLASS as i16 {
                Some(&b.clank)
            } else if item.o_class == b.pack_o_class && b.state != 0 {
                Some(&b.pack)
            } else {
                None
            };
            item.visible = m.is_some() && !hidden;
            if let Some(m) = m {
                item.state = m.anim;
                item.snapshot = m.snapshot.clone();
            }
        }
    }
    let (k, class) = (a.host_k, &level.0.mobys.anim[a.host_class]);
    place(&mut a, class, &anim.instances[k].state, anim.snapshots[k].as_ref(), back.is_none(), hand.is_none());
    if let Some(h) = hand {
        for item in a.items.iter_mut().filter(|i| i.attach.slot == Slot::Hand) {
            match h.as_ref().filter(|m| m.o_class == item.o_class) {
                Some(m) => {
                    item.visible = true;
                    item.state = m.anim;
                    item.snapshot = m.snapshot.clone();
                    item.rows = m.rows;
                    item.position = m.position;
                }
                None => item.visible = false,
            }
        }
        for item in a.items.iter_mut().filter(|i| i.attach.slot == Slot::Hook) {
            item.visible = hook.is_some();
            if let Some(k) = hook {
                moby_anim::advance(&mut item.state, &item.anim);
                item.rows = k.rows.map(|r| r.map(f32::to_bits));
                item.position = k.pos;
            }
        }
    }
    a.ticks += 1;
}

/// Palettes and records of the items after a tick, and the entities' transforms (blended-pass sorting).
fn upload(
    attach: Option<ResMut<MobyAttach>>,
    level: Res<crate::Level>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut transforms: Query<&mut Transform>,
    mut commands: Commands,
) {
    let Some(mut a) = attach else { return };
    if a.uploaded == Some(a.ticks) { return; }
    a.uploaded = Some(a.ticks);
    for item in &mut a.items {
        if item.shown != Some(item.visible) {
            item.shown = Some(item.visible);
            let v = if item.visible { Visibility::Inherited } else { Visibility::Hidden };
            for &e in &item.entities { commands.entity(e).insert(v); }
        }
    }
    let lighting = level.0.mobys.lighting.as_ref();
    let mut palette = crate::moby_anim::identity_palette(a.palette_len);
    let mut records = Vec::with_capacity(a.items.len() * moby_render::EXTRA_RECORD_SIZE);
    for item in &a.items {
        let f = moby_anim::evaluate_with_snapshot(&item.anim, &item.state, item.snapshot.as_ref());
        let at = item.base as usize * 64;
        for (k, b) in f.iter().take(item.slots as usize).flat_map(|r| r.iter().flatten().flat_map(|v| v.to_le_bytes())).enumerate() { palette[at + k] = b; }
        let lights: Option<light::MobyLights> = lighting.map(|l| light::moby_lights(&item.rows, &l.bank, a.host_light, a.host_ambient, 0x80));
        let model = model_of(item);
        records.extend_from_slice(&moby_render::extra_record(&model, lights.as_ref(), item.base));
        let t = Transform::from_matrix(model);
        for &e in &item.entities {
            if let Ok(mut tr) = transforms.get_mut(e) { *tr = t; }
        }
    }
    if let Some(mut buf) = buffers.get_mut(&a.extra.palette) { buf.data = Some(palette); }
    if let Some(mut buf) = buffers.get_mut(&a.extra.instances) { buf.data = Some(records); }
}

// ---------------------------------------------------------------------------------------------------------------
// The Swingshot's rope (`0x2dba30`, registered for the frame by the hand item's update through `0x21afe0`).

/// The rope's entity and mesh (created on the first frame the level has the effect texture).
#[derive(Default)]
struct RopeGfx {
    entity: Option<Entity>,
    mesh: Option<Handle<Mesh>>,
    shown: bool,
}

/// `0x2dba30`'s strip in game coordinates: quads `(v0, e0, v1, e1)` with the UVs (0,0), (1,0), (0,1), (1,1). From
/// the hook toward the hand's joint 0 in steps of at most 0.2; the first edge is the hook ± 0.05 in z, each next
/// edge the step's point ± 0.05 across the view (`cross(point − camera, step)`), both displaced along
/// `cross(dir, dir − 0.1·z)` by `sin(π·s/L) · sin(π·s/wavelength) · cos(phase) · amplitude`.
pub fn rope_quads(r: &rc_game::hero::swingshot::Rope, camera: [f32; 3]) -> Vec<[[f32; 3]; 4]> {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let add = |a: [f32; 3], b: [f32; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let len = |a: [f32; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let set_len = |a: [f32; 3], l: f32| { let n = len(a); if n == 0.0 { [0.0; 3] } else { a.map(|x| x * l / n) } };
    let hook = r.to;
    let dir = sub(r.from, hook);
    let total = len(dir);
    let mut out = Vec::new();
    if total.is_nan() || total <= 0.0 { return out; }
    let bend = cross(dir, [dir[0], dir[1], dir[2] - 0.1]);
    let (mut v0, mut v1) = ([hook[0], hook[1], hook[2] - 0.05], [hook[0], hook[1], hook[2] + 0.05]);
    let wave = r.phase.cos();
    let (mut s, mut cursor) = (0.0f32, hook);
    let pi = std::f32::consts::PI;
    while s < total {
        let step = (total - s).min(0.2);
        s += step;
        let env = (s * pi / total).sin();
        let w = if r.wavelength != 0.0 { (s * pi / r.wavelength).sin() } else { 0.0 };
        let off = w * env * wave;
        cursor = add(cursor, set_len(dir, step));
        let side = set_len(cross(sub(cursor, camera), dir), 0.05);
        let disp = set_len(bend, off * r.amplitude);
        let e0 = add(add(cursor, side), disp);
        let e1 = add(sub(cursor, side), disp);
        out.push([v0, e0, v1, e1]);
        (v0, v1) = (e0, e1);
    }
    out
}

fn rope_mesh(quads: &[[[f32; 3]; 4]]) -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    let mut pos = Vec::with_capacity(quads.len() * 4);
    let mut uv = Vec::with_capacity(quads.len() * 4);
    let mut idx = Vec::with_capacity(quads.len() * 6);
    for (k, q) in quads.iter().enumerate() {
        let b = (4 * k) as u32;
        for (v, t) in q.iter().zip([[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]) {
            pos.push(crate::tfrag_render::game_to_bevy(*v).to_array());
            uv.push(t);
        }
        idx.extend_from_slice(&[b, b + 1, b + 2, b + 1, b + 3, b + 2]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_indices(Indices::U32(idx))
}

/// Draws the rope of this tick (none: hidden). Effect texture 0xf (`GetEffectTex(0xf)`, bilinear, repeat), vertex
/// colour 0x80808080 (the texture as it is), GS alpha 0x80 = 1.0 (the texture's alpha doubled), alpha blended.
fn rope_draw(
    mut st: Local<RopeGfx>,
    play: Option<Res<crate::gameplay::Play>>,
    level: Res<crate::Level>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let Some(play) = play else { return };
    let item = &play.game.hero.swing.item;
    let rope = item.rope.filter(|_| item.alive);
    if st.entity.is_none() {
        if rope.is_none() { return; }
        let Some(t) = level.0.particles.textures.as_ref().and_then(|t| t.fx_textures.get(0xf)).and_then(|t| t.as_ref()) else { return };
        use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let mut img = Image::new_uninit(Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 }, TextureDimension::D2, TextureFormat::Rgba8UnormSrgb, bevy::asset::RenderAssetUsages::RENDER_WORLD);
        img.data = Some(t.rgba.chunks(4).flat_map(|c| [c[0], c[1], c[2], (c[3] as u16 * 2).min(255) as u8]).collect());
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
        let image = images.add(img);
        let mat = mats.add(StandardMaterial { base_color_texture: Some(image), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, fog_enabled: false, ..default() });
        let mesh = meshes.add(rope_mesh(&[]));
        let e = commands
            .spawn((Name::new("Swingshot rope"), Mesh3d(mesh.clone()), MeshMaterial3d(mat), Transform::IDENTITY, Visibility::Hidden, bevy::camera::visibility::NoFrustumCulling))
            .id();
        (st.entity, st.mesh) = (Some(e), Some(mesh));
    }
    let (Some(e), Some(h)) = (st.entity, st.mesh.clone()) else { return };
    match rope {
        Some(r) => {
            let cam = play.game.camera.out.pos_f32();
            if let Some(mut m) = meshes.get_mut(&h) { *m = rope_mesh(&rope_quads(&r, cam)); }
            if !st.shown { commands.entity(e).insert(Visibility::Inherited); }
            st.shown = true;
        }
        None => {
            if st.shown { commands.entity(e).insert(Visibility::Hidden); }
            st.shown = false;
        }
    }
}
