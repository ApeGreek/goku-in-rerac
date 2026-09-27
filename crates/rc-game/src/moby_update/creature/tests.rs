//! Unit tests of the shared creature layer and the critter on hand-built floors.

use super::*;
use crate::hero::physics::v4;
use crate::hero::testkit;
use crate::hero::Hero;
use crate::moby_runtime::{ClassInfo, Moby, MobyTable};
use crate::moby_update::scheduler::{self, Scheduler};
use crate::moby_update::services::{HitRecord, Services};
use crate::moby_update::ClassTable;
use crate::ps2v::Pf;
use crate::rng::{Rng, LEVEL_SEED};
use rc_formats::collision::Collision;

fn seeded() -> Rng { let mut r = Rng::new(); r.srand(LEVEL_SEED); r }

fn draws(from: &Rng, to: &Rng) -> usize {
    let mut r = *from;
    for n in 0..100_000 {
        if r.state == to.state { return n; }
        r.rand();
    }
    panic!("state not reached");
}

#[test]
fn rotations_wrap_like_the_game() {
    assert!((add_rot(3.0, 0.5) - (3.5 - 2.0 * PI)).abs() < 1e-6);
    assert!((sub_rot(-3.0, 0.5) - (-3.5 + 2.0 * PI)).abs() < 1e-6);
    assert!((diff_rots(3.0, -3.0) - (2.0 * PI - 6.0)).abs() < 1e-6);
}

#[test]
fn turn_toward_stops_on_the_target() {
    // Braking in time: at most the last step's residual past the heading (the game's reversal branch adds the braked
    // velocity without the snap), then it settles.
    let (mut a, mut v) = (0.0f32, 0.0f32);
    for _ in 0..600 {
        turn::turn_toward(2.0, DT2 * 12.566, DT2 * 12.566, DT * 25.13, &mut a, &mut v);
        assert!(a <= 2.0 + 0.005, "overshoot {a}");
    }
    assert!((a - 2.0).abs() < 1e-3, "settled at {a}");
    let mut x = 0.0;
    assert_eq!(turn::approach(1.0, 0.3, &mut x), 0.7);
}

#[test]
fn rate_slot_takes_the_last_free_slot() {
    let mut s = [0i32; 3];
    assert_eq!(fx::rate_slot(&mut s, 10, 60), 2);
    assert_eq!(fx::rate_slot(&mut s, 11, 60), 1);
    assert_eq!(fx::rate_slot(&mut s, 12, 60), 0);
    assert_eq!(fx::rate_slot(&mut s, 13, 60), -1);
    assert_eq!(fx::rate_slot(&mut s, 71, 60), 2);
}

// -------------------------------------------------------------------------------------------------
// A small world: Ratchet (index 0) and one creature (index 1) on a floor.

struct Sim {
    table: MobyTable,
    classes: ClassTable,
    hero: Hero,
    rng: Rng,
    svc: Services,
    sched: Scheduler,
    mesh: Collision,
    counter: u64,
}

/// Eight sequences of 20 keys, key k at time k (one key per tick), for the creature classes.
fn anim_class() -> rc_formats::moby_anim::MobyAnimClass {
    use rc_formats::moby_anim::{MobyAnimClass, MobyFrame, MobyFrameHeader, MobySequence, MobySequenceHeader, IDENTITY};
    let key = |k: i16| MobyFrame {
        header: MobyFrameHeader { rate: 1.0, time: k * 16, qwc: 1, quat_bytes: 8, scale_count: 0, trans_offset: 8, trans_count: 0 },
        quats: vec![[0, 0, 0, 0x7fff]],
        scales: vec![],
        trans: vec![],
        payload: vec![0, 0, 0, 0, 0, 0, 0xff, 0x7f, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    let seq = || Some(MobySequence { header: MobySequenceHeader { frame_count: 20, loop_sound: 0xff, ..Default::default() }, frames: (0..20).map(key).collect(), triggers: vec![] });
    MobyAnimClass { joint_count: 1, skeleton: vec![IDENTITY], rest: vec![[0.0; 3]], parent_word: vec![0], sequences: (0..8).map(|_| seq()).collect() }
}

fn classes() -> ClassTable {
    let mut t = ClassTable::default();
    for (slot, oc) in [(1u8, 577i16), (2, 0x23c), (3, 1747), (4, 1748), (5, 1749), (6, 0x70), (7, 13), (8, 0x27f)] {
        let info = ClassInfo { slot, update_fn: scheduler::port_update_fn(oc), scale: 1.0, has_collision: false, ..Default::default() };
        let anim = matches!(oc, 577 | 0x23c).then(anim_class);
        t.classes.insert(oc, (info, anim));
    }
    t
}

/// A critter with Novalis's pvars (instance 579: range 5, speed 4, ±30°, hoverer).
fn critter_pvars() -> Vec<u8> {
    let mut p = vec![0u8; 0x270];
    let set = |p: &mut Vec<u8>, o: usize, b: &[u8]| p[o..o + b.len()].copy_from_slice(b);
    set(&mut p, 0, &0x20u32.to_le_bytes());
    set(&mut p, 0xc, &0x110u32.to_le_bytes());
    set(&mut p, 0x10, &0x120u32.to_le_bytes());
    set(&mut p, 0x14, &0x60u32.to_le_bytes());
    set(&mut p, 0x18, &0x180u32.to_le_bytes());
    set(&mut p, 0x110, &[0, 0, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 4, 0, 15, 0]);
    for (o, x) in [(0x130, 0.008f32), (0x138, 0.2), (0x13c, 0.125), (0x148, 0.5), (0x150, 0.4), (0x168, 0.2), (0x16c, 0.0333), (0x1f8, 30.0), (0x1fc, 5.0), (0x200, 20.0), (0x204, 4.0)] {
        set(&mut p, o, &x.to_le_bytes());
    }
    set(&mut p, 0x140, &0x200i32.to_le_bytes());
    set(&mut p, 0x216, &8i16.to_le_bytes());
    set(&mut p, 0x21c, &(-1i32).to_le_bytes());
    p
}

impl Sim {
    fn new(creature: i16, at: [f32; 3], pvars: Vec<u8>, hero_at: [f32; 3]) -> Sim {
        let mut h = Moby::init_instance(0, 0, Some(&ClassInfo { scale: 1.0, ..Default::default() }));
        h.mode |= crate::moby_runtime::mode::NO_UPDATE;
        h.position = [hero_at[0], hero_at[1], hero_at[2], 0.0];
        let classes = classes();
        let info = classes.classes[&creature].0;
        let mut m = Moby::init_instance(1, creature, Some(&info));
        m.position = [at[0], at[1], at[2], 0.0];
        m.update_dist = 0xff;
        m.mode |= PVAR_HEADER;
        m.pvars = pvars;
        m.rows = rc_formats::moby_anim::IDENTITY;
        let table = MobyTable::new(vec![h, m], 16);
        let mut hero = Hero::new();
        hero.pos = v4(hero_at[0], hero_at[1], hero_at[2]);
        hero.body_point = v4(hero_at[0], hero_at[1], hero_at[2] + 0.7);
        Sim { table, classes, hero, rng: seeded(), svc: Services::new(), sched: Scheduler::new(), mesh: testkit::floor(10.0, 20, 40, 20, 40), counter: 0 }
    }
    fn load(&mut self) {
        let mut w = World::new(&mut self.table, &self.hero, &mut self.rng, &self.classes, &mut self.svc, 0);
        w.coll = Some(&self.mesh);
        self.sched.load_pass(&mut w);
        self.counter = 1;
    }
    fn tick(&mut self) {
        self.table.free_slot_pass(self.counter);
        let mut w = World::new(&mut self.table, &self.hero, &mut self.rng, &self.classes, &mut self.svc, self.counter);
        w.coll = Some(&self.mesh);
        w.camera = self.hero.pos;
        self.sched.tick(&mut w);
        self.counter += 1;
    }
    fn critter(&self) -> &Moby { &self.table.mobys[1] }
}

#[test]
fn critter_hovers_with_three_draws_per_pick() {
    // Ratchet far away: the hoverer lifts 6 at init (1 draw: the mirror), then picks a hover spot every 120 ticks.
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [150.0, 150.0, 10.0]);
    let r0 = s.rng;
    s.load();
    assert_eq!(draws(&r0, &s.rng), 1);
    assert_eq!((s.critter().state, s.critter().position[2]), (0xe, 16.0));
    let r1 = s.rng;
    for _ in 0..600 { s.tick(); }
    let n = draws(&r1, &s.rng);
    assert!(n.is_multiple_of(3) && (12..=18).contains(&n), "hover draws {n}");
    let p = s.critter().position;
    let d = ((p[0] - 120.0).powi(2) + (p[1] - 120.0).powi(2)).sqrt();
    assert!(d <= 3.01 && (15.0..=17.0).contains(&p[2]), "hover spot {p:?}");
}

#[test]
fn critter_lands_walks_and_bites_ratchet() {
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [126.0, 120.0, 10.0]);
    s.load();
    let mut seen = std::collections::BTreeSet::new();
    let mut bitten = None;
    for t in 0..900 {
        s.tick();
        seen.insert(s.critter().state);
        let slot = s.table.mobys[0].hit_slot;
        if slot != 0xff && bitten.is_none() {
            let r: HitRecord = s.svc.hits.records[slot as usize];
            assert_eq!((r.target, r.attacker, r.flags, r.damage), (0, Some(1), 1, Pf::ONE));
            bitten = Some(t);
        }
        s.table.mobys[0].hit_slot = 0xff;
    }
    eprintln!("critter states {seen:?}, bitten at {bitten:?}, now at {:?}", s.critter().position);
    assert!(seen.contains(&0x11) && seen.contains(&3) && seen.contains(&4), "states {seen:?}");
    assert!(bitten.is_some(), "never bit Ratchet");
}

#[test]
fn a_hit_kills_the_critter_with_pieces_explosion_and_bolts() {
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [150.0, 150.0, 10.0]);
    s.table.mobys[1].spawn_flag = 0xff; // a spawn-flagged instance drops bolts (SetDeathBits)
    s.table.mobys[1].b4 = 3;
    s.load();
    for _ in 0..5 { s.tick(); }
    // The wrench's record: exact push, type 0 / subtype 1, class 0x47, damage 1, no attacker.
    let dir = [Pf::f(1.0), Pf::ZERO, Pf::ONE, Pf::f(crate::hero::damage::EXACT_PUSH_W)];
    let t = crate::moby_update::services::HitTemplate { dir, attacker: None, flags: 0x1_0000, b18: 0, b19: 1, h1a: 0x47, damage: Pf::ONE, w20: 1 };
    crate::moby_update::services::deliver_hit_in(&mut s.table, &mut s.svc.hits, 1, &t);
    s.tick();
    assert_eq!(s.critter().state, 99, "death flight");
    assert_eq!(s.critter().mode & crate::moby_runtime::mode::TARGETABLE, 0);
    let mut dyn_classes = std::collections::BTreeMap::new();
    for _ in 0..240 {
        s.tick();
        for m in &s.table.mobys[s.table.first_dynamic..] {
            if m.state < 0xfd { *dyn_classes.entry(m.o_class).or_insert(0) += 1; }
        }
    }
    eprintln!("dynamic mobys seen (class → ticks alive): {dyn_classes:?}; unported {:?}", s.svc.fx.unported);
    assert!(s.critter().is_deleted(), "critter not deleted");
    for c in [1747, 1748, 1749] { assert!(dyn_classes.contains_key(&c), "piece {c}"); }
    assert!(dyn_classes.keys().any(|c| (13..=16).contains(c)), "no bolts");
    assert!(dyn_classes.contains_key(&0x27f), "no explosion light");
}

#[test]
fn resolver_cools_down_repeated_hits_of_one_kind() {
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [150.0, 150.0, 10.0]);
    let rec = |dmg: f32| HitRecord { dir: [Pf::f(1.0), Pf::ZERO, Pf::ONE, Pf::f(crate::hero::damage::EXACT_PUSH_W)], b28: 0, b29: 1, h2a: 0x47, damage: Pf::f(dmg), flags: 0x10000, target: 1, ..Default::default() };
    let mut w = World::new(&mut s.table, &s.hero, &mut s.rng, &s.classes, &mut s.svc, 1);
    let a = damage::resolve(&mut w, 1, Some(rec(1.0)), 0x20, 0, 4);
    assert_eq!((a.kind, a.damage, a.out5), (Some(1), 1.0, 2));
    assert_eq!(pi16(&w, 1, 0x3c), 15, "cooldown ticks(15)");
    // The wrench push is redirected away from Ratchet (at (150, 150)): unit xy length.
    let d = a.hit.unwrap().dir.map(|x| f32::from_bits(x.0));
    assert!((len2(d) - 1.0).abs() < 1e-5 && d[0] < 0.0 && d[1] < 0.0, "push {d:?}");
    let b = damage::resolve(&mut w, 1, Some(rec(1.0)), 0x20, 0, 4);
    assert_eq!((b.damage, b.out5, b.reaction), (0.0, 1, 0xb), "second hit inside the cooldown");
    for _ in 0..14 { damage::resolve(&mut w, 1, None, 0x20, 0, 4); }
    let c = damage::resolve(&mut w, 1, Some(rec(1.0)), 0x20, 0, 4);
    assert_eq!(c.damage, 1.0, "after the cooldown");
}

#[test]
fn flash_ramps_to_red_and_back() {
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [150.0, 150.0, 10.0]);
    let mut w = World::new(&mut s.table, &s.hero, &mut s.rng, &s.classes, &mut s.svc, 1);
    flash::start(&mut w, 1, 0x110);
    let mut reds = Vec::new();
    for _ in 0..22 {
        flash::update(&mut w, 1, 0x110);
        reds.push(w.m(1).ambient[0]);
    }
    assert_eq!(reds[3], 0x80, "full red after the 4-tick fade-in: {reds:?}");
    assert_eq!(*reds.last().unwrap(), 0x40, "back to the ambient: {reds:?}");
    assert!(reds.windows(2).skip(4).all(|p| p[1] <= p[0]), "fade-out monotonic: {reds:?}");
}

#[test]
fn region_walls_and_waypoints() {
    // A 10×10 arena (walls: every edge, w ≠ 0) and a graph of two points seeing each other.
    let mut s = Sim::new(577, [120.0, 120.0, 10.0], critter_pvars(), [150.0, 150.0, 10.0]);
    let sq: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 1.0], [10.0, 0.0, 0.0, 1.0], [10.0, 10.0, 0.0, 1.0], [0.0, 10.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]];
    let wall: Vec<[f32; 4]> = vec![[5.0, 2.0, 0.0, 1.0], [5.0, 8.0, 0.0, 1.0]];
    let graph: Vec<[f32; 4]> = vec![[5.0, 9.0, 0.0, f32::from_bits(2)], [2.0, 9.0, 0.0, f32::from_bits(1)]];
    s.svc.set_splines(&[sq, wall, graph]);
    let w = World::new(&mut s.table, &s.hero, &mut s.rng, &s.classes, &mut s.svc, 1);
    assert!(region::point_in_polygon(&w, 0, [5.0, 5.0, 0.0, 0.0]));
    assert!(!region::point_in_polygon(&w, 0, [15.0, 5.0, 0.0, 0.0]));
    let (a, b) = ([2.0, 5.0, 0.0, 0.0], [8.0, 5.0, 0.0, 0.0]);
    assert!(region::crosses(&w, 1, a, b));
    let (hit, p) = region::clamp(&w, 1, a, b);
    assert!(hit && (p[0] - 5.0).abs() < 1e-5, "{p:?}");
    // Blocked straight: the way round the wall goes through the graph point both ends see.
    let via = region::line_of_sight(&w, 0.0, &[1], 2, a, b).expect("a way round");
    assert_eq!([via[0], via[1]], [5.0, 9.0]);
    assert_eq!(region::line_of_sight(&w, 0.0, &[1], 2, a, [3.0, 5.0, 0.0, 0.0]), Some([3.0, 5.0, 0.0, 0.0]));
}

// -------------------------------------------------------------------------------------------------
// The enemy additions: SpringTurn, the projectile helpers.

#[test]
fn spring_turn_is_spring_turn2_on_any_angle() {
    // SpringTurn2 is SpringTurn on the moby's yaw: same velocity, same result, never past the target.
    let mut sim = Sim::new(577, [30.0, 30.0, 10.0], critter_pvars(), [25.0, 30.0, 10.0]);
    let (mut v1, mut v2, mut a) = (0.0f32, 0.0f32, 0.4f32);
    let mut w = World::new(&mut sim.table, &sim.hero, &mut sim.rng, &sim.classes, &mut sim.svc, 0);
    set_yaw(&mut w, 1, 0.4);
    for _ in 0..200 {
        a = turn::spring_turn(a, -2.9, DT2 * 2.0 * PI, DT2 * PI, DT * 2.0 * PI, &mut v1);
        turn::spring_turn2(&mut w, 1, -2.9, DT2 * 2.0 * PI, DT2 * PI, DT * 2.0 * PI, &mut v2);
        assert_eq!(a.to_bits(), yaw(&w, 1).to_bits());
        assert_eq!(v1.to_bits(), v2.to_bits());
    }
    assert!(diff_rots(a, -2.9) < 1e-3, "settled at {a}");
    // The wrap: from 3.0 to −3.0 it turns forwards through π.
    let mut v = 0.0;
    let b = turn::spring_turn(3.0, -3.0, 0.02, 0.3, 0.1, &mut v);
    assert!(v > 0.0 && (b > 3.0 || b < -3.0), "{b} {v}");
}

#[test]
fn projectile_parts_draw_like_the_spawners() {
    let mut sim = Sim::new(577, [30.0, 30.0, 10.0], critter_pvars(), [25.0, 30.0, 10.0]);
    let mut w = World::new(&mut sim.table, &sim.hero, &mut sim.rng, &sim.classes, &mut sim.svc, 0);
    // Types 22 / 26: one raw rand() with a record (no particle system: as with a free record).
    let before = *w.rng;
    projectile::part22(&mut w, &crate::particles::type22::Spawn { size: 1.0, pos: [5.0; 4], vel: [0.0; 4], c1: 0, c2: 0, life: 10 });
    assert_eq!(draws(&before, w.rng), 1);
    let before = *w.rng;
    projectile::part26(&mut w, 1.0, 0, 0x8080_8080, 10);
    assert_eq!(draws(&before, w.rng), 1);
    // Type 16: nothing for life 0; the throttle draws only over the frame loads 0.9 / 0.95 / 1.0.
    let t16 = |life| crate::particles::type16::Spawn { size: 1.0, pos: [5.0; 4], vel: [0.0; 4], c1: 0, c2: 0, life, kind: 0 };
    let before = *w.rng;
    projectile::part16(&mut w, &t16(0));
    assert_eq!(draws(&before, w.rng), 0);
    let before = *w.rng;
    projectile::part16(&mut w, &t16(60));
    assert_eq!(draws(&before, w.rng), 1);
    w.svc.frame_load[0] = Pf::f(0.96);
    let before = *w.rng;
    projectile::part16(&mut w, &t16(60));
    let n = draws(&before, w.rng);
    assert!((1..=3).contains(&n), "{n}");
    assert_eq!(w.svc.fx.part_spawns.get(&22), Some(&1));
    assert!(projectile::in_world([2.0, 1021.0, 500.0, 0.0]) && !projectile::in_world([1.9, 5.0, 5.0, 0.0]));
}

#[test]
fn projectile_sweep_hits_the_floor_and_misses_the_air() {
    let mut sim = Sim::new(577, [30.0, 30.0, 10.0], critter_pvars(), [25.0, 30.0, 10.0]);
    let mut w = World::new(&mut sim.table, &sim.hero, &mut sim.rng, &sim.classes, &mut sim.svc, 0);
    w.coll = Some(&sim.mesh);
    let t = projectile::template(&w, 1, [0.4, 0.0, 1.0, crate::hero::damage::EXACT_PUSH_W], 0x1_0001, (1, 1), 1.0, 1);
    assert_eq!((t.flags, t.b18, t.b19, t.h1a, t.attacker), (0x1_0001, 1, 1, 577, Some(1)));
    let hit = projectile::sweep(&mut w, [120.0, 120.0, 11.0, 0.0], [120.0, 120.0, 9.0, 0.0], 0, 0.5, Some(1), &t).expect("the floor at z 10");
    assert!((hit[2] - 10.0).abs() < 1e-3, "{hit:?}");
    assert!(projectile::sweep(&mut w, [120.0, 120.0, 13.0, 0.0], [120.4, 120.0, 12.9, 0.0], 0, 0.5, Some(1), &t).is_none());
}
