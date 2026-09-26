//! Swim unit tests on hand-built pools: floor faces (surface 1) under water faces (surface 0).

use super::*;
use crate::hero::testkit::*;
use crate::hero::HeroTick;
use crate::pad::{button, PadInput};
use rc_formats::collision::Collision;
use std::collections::BTreeMap;

const WATER: f32 = 14.0;

/// A pool over cells `x0..x1 × y0..y1` (cell units of 4): the floor height `floor(x)` at each cell corner (a
/// ramp when it varies) and a water face at `water` where `water` is above the floor (None: dry).
/// Vertices and quads per cell key.
type CellFaces = BTreeMap<(i16, i16, i16), (Vec<[f32; 3]>, Vec<([u8; 4], u8)>)>;

fn pool(floor: impl Fn(f32) -> f32, water: Option<f32>, x0: i16, x1: i16, y0: i16, y1: i16) -> Collision {
    let mut cells = CellFaces::new();
    let mut quad = |cx: i16, cy: i16, z: [f32; 2], ty: u8| {
        let (x, y) = (cx as f32 * 4.0, cy as f32 * 4.0);
        let cz = (z[0].min(z[1]) / 4.0).floor() as i16;
        let e = cells.entry((cx, cy, cz)).or_default();
        let b = e.0.len() as u8;
        e.0.extend_from_slice(&[[x, y, z[0]], [x, y + 4.0, z[0]], [x + 4.0, y + 4.0, z[1]], [x + 4.0, y, z[1]]]);
        e.1.push(([b, b + 1, b + 2, b + 3], ty));
    };
    for cx in x0..x1 {
        for cy in y0..y1 {
            let (xa, xb) = (cx as f32 * 4.0, cx as f32 * 4.0 + 4.0);
            let z = [floor(xa), floor(xb)];
            quad(cx, cy, z, 0x21);
            if let Some(w) = water {
                if z[0] < w || z[1] < w { quad(cx, cy, [w, w], 0x00); }
            }
        }
    }
    mesh(cells.into_iter().map(|((cx, cy, cz), (v, q))| cell([cx, cy, cz], &v, &q)).collect())
}

/// A 4-deep pool (floor 10, water 14) over x, y in 0..48.
fn deep() -> Collision { pool(|_| 10.0, Some(WATER), 0, 12, 0, 12) }

fn runner(pos: [f32; 3]) -> Runner {
    let mut r = Runner::new(pos, 0.0);
    // The recording anim advances the readout by the playback speed; the stroke curve reads how far it moved.
    r.anim.v.frame_step = 0.6;
    r
}

fn run(r: &mut Runner, coll: &Collision, input: PadInput, n: usize) {
    for _ in 0..n {
        assert_eq!(r.tick(coll, input), HeroTick::Ran, "state {:#x}", r.hero.state);
    }
}

fn states(r: &Runner) -> Vec<i32> {
    let mut v: Vec<i32> = Vec::new();
    for l in &r.log { if v.last() != Some(&l.0) { v.push(l.0); } }
    v
}

/// Drop into the deep pool and settle on the surface.
fn floating() -> (Collision, Runner) {
    let coll = deep();
    let mut r = runner([24.0, 24.0, 18.0]);
    run(&mut r, &coll, PadInput::neutral(), 150);
    (coll, r)
}

#[test]
fn fall_into_deep_water_floats_on_the_surface() {
    let (_, r) = floating();
    let s = states(&r);
    eprintln!("states {s:x?}, z {}, bob {} level {}", r.hero.position()[2], r.hero.swim.bob.to_f32(), r.hero.swim.level.to_f32());
    assert_eq!(s, vec![0, 6, id::SURFACE_IDLE]);
    let entry = r.log.iter().find(|l| l.0 == id::SURFACE_IDLE).unwrap();
    // The fall reaches the level: entered within the band |W − (z + 0.45)| < max(0.2, |vz| + 0.07).
    assert!(entry.2[2] < WATER && entry.2[2] > WATER - 1.5, "entered at z {}", entry.2[2]);
    assert_eq!(r.hero.group, 0x12);
    assert_eq!(r.hero.water_level.to_f32(), WATER);
    assert!((r.hero.position()[2] - (WATER - 0.12)).abs() < 0.03, "floats at {}", r.hero.position()[2]);
    assert_eq!(r.hero.f0634, 1);
    assert_eq!(r.hero.swim.oxygen, OXYGEN_MAX);
    assert!(r.hero.swim.events.contains(&SwimEvent::Sound(3)));
    assert!(r.hero.swim.events.iter().any(|e| matches!(e, SwimEvent::Splash { big: true, .. })));
}

#[test]
fn surface_swim_speed_and_stop() {
    let (coll, mut r) = floating();
    let x0 = r.hero.position()[0];
    run(&mut r, &coll, PadInput::neutral().stick(0.0, -1.0), 120);
    assert_eq!(r.hero.state, id::SURFACE_SWIM);
    // Target 3 u/s × full stick, reached through SpeedStep(4·dt²).
    assert!((r.hero.speed.to_f32() - 3.0 / 60.0).abs() < 1e-6, "speed {}", r.hero.speed.to_f32());
    let x1 = r.hero.position()[0];
    // 0.6 of the readout per tick on the curve's tail (0.4): 3·dt × 0.24 × 5.5 per tick.
    let per_tick = 3.0 / 60.0 * 0.24 * 5.5;
    let moved = x1 - x0;
    eprintln!("moved {moved} in 120 ticks (steady {per_tick}/tick), z {}", r.hero.position()[2]);
    assert!(moved > 40.0 * per_tick && moved < 120.0 * per_tick, "moved {moved}");
    assert!((r.hero.position()[1] - 24.0).abs() < 1e-3);
    assert!((r.hero.position()[2] - (WATER - 0.12)).abs() < 0.03);
    // Release: the speed runs out, then treading water again.
    run(&mut r, &coll, PadInput::neutral(), 60);
    assert_eq!(r.hero.state, id::SURFACE_IDLE);
    assert_eq!(r.hero.speed, Pf::ZERO);
}

#[test]
fn no_pack_dives_slowly_and_r1_does_nothing() {
    let (coll, mut r) = floating();
    assert!(!r.hero.swim.hydro_pack);
    // R1 alone on the surface: no dive without the pack.
    run(&mut r, &coll, PadInput::neutral().press(button::R1), 10);
    assert_eq!(r.hero.state, id::SURFACE_IDLE);
    // □: the plain dive 0x33 (dive-in substate, pitched down), R1 held changes nothing.
    run(&mut r, &coll, PadInput::neutral(), 2);
    run(&mut r, &coll, PadInput::neutral().press(button::SQUARE), 1);
    assert_eq!(r.hero.state, id::UNDERWATER);
    assert_eq!(r.hero.group, 0x11);
    assert_eq!(r.hero.substate, 1);
    let z0 = r.hero.position()[2];
    run(&mut r, &coll, PadInput::neutral().press(button::R1 | button::SQUARE), 40);
    assert_eq!(r.hero.state, id::UNDERWATER);
    let z1 = r.hero.position()[2];
    eprintln!("dive: z {z0} → {z1}, pitch {}, speed {}", r.hero.rot[1].to_f32(), r.hero.speed.to_f32());
    assert!(z1 < z0 - 0.3, "dove {z0} → {z1}");
    assert!(r.hero.rot[1].to_f32() > 0.5, "pitched down");
    assert!(r.hero.speed.to_f32() <= 6.0 / 60.0 + 1e-6);
    // Air drains 11 per tick under water.
    assert!(r.hero.swim.oxygen < OXYGEN_MAX && r.hero.swim.oxygen > OXYGEN_MAX - 11 * 45);
}

#[test]
fn hydro_pack_dive_thrust_and_surface() {
    let (coll, mut r) = floating();
    r.hero.swim.hydro_pack = true;
    run(&mut r, &coll, PadInput::neutral(), 2);
    // R1 pressed on the surface dives with the pack, straight into 0x35 while held.
    run(&mut r, &coll, PadInput::neutral().press(button::R1), 1);
    assert_eq!(r.hero.state, id::HYDRO);
    let dive = PadInput::neutral().press(button::R1 | button::SQUARE).stick(0.0, -1.0);
    run(&mut r, &coll, dive, 50);
    assert_eq!(r.hero.state, id::HYDRO);
    let deep_z = r.hero.position()[2];
    eprintln!("thrust dive: z {deep_z}, speed {}, pitch {}", r.hero.speed.to_f32(), r.hero.rot[1].to_f32());
    assert!(deep_z < WATER - 1.5, "z {deep_z}");
    assert!((r.hero.speed.to_f32() - 7.0 / 60.0).abs() < 1e-6, "full thrust {}", r.hero.speed.to_f32());
    // Climb: R1 + ✕ pitch up; back on the surface when rising within 0.4 of the level.
    let climb = PadInput::neutral().press(button::R1 | button::CROSS).stick(0.0, -1.0);
    let mut surfaced = None;
    for t in 0..240 {
        r.tick(&coll, climb);
        if r.hero.state == id::SURFACE_IDLE { surfaced = Some(t); break; }
    }
    eprintln!("surfaced after {surfaced:?} ticks at z {}, pitch {}", r.hero.position()[2], r.hero.rot[1].to_f32());
    assert!(surfaced.is_some(), "never surfaced; states {:x?}", states(&r));
    // Released pack: the pitch straightens (0x236520) on the surface.
    run(&mut r, &coll, PadInput::neutral(), 120);
    assert!(r.hero.rot[1].to_f32().abs() < 0.01 && r.hero.rot[0].to_f32().abs() < 0.01, "pitch {} roll {}", r.hero.rot[1].to_f32(), r.hero.rot[0].to_f32());
    assert!((r.hero.position()[2] - (WATER - 0.12)).abs() < 0.05);
}

#[test]
fn out_of_air_drowns_unless_masked() {
    let (coll, mut r) = floating();
    r.hero.swim.o2_mask = true;
    run(&mut r, &coll, PadInput::neutral().press(button::SQUARE), 1);
    run(&mut r, &coll, PadInput::neutral(), 1000);
    assert_eq!(r.hero.group, 0x11, "state {:#x}", r.hero.state);
    assert_eq!(r.hero.swim.oxygen, OXYGEN_MAX);
    r.hero.swim.o2_mask = false;
    let mut t = 0;
    while r.hero.state != id::DROWN && t < 1000 { r.tick(&coll, PadInput::neutral()); t += 1; }
    eprintln!("drowned after {t} ticks");
    assert_eq!(r.hero.state, id::DROWN);
    assert!((908..=910).contains(&t), "10000 / 11 per tick");
    assert_eq!(r.hero.health, 0);
    let z = r.hero.position()[2];
    run(&mut r, &coll, PadInput::neutral(), 60);
    assert!(r.hero.position()[2] < z, "sinks");
    assert_eq!(r.hero.state, id::DROWN, "locked in the death group");
}

#[test]
fn jump_out_of_deep_water() {
    let (coll, mut r) = floating();
    run(&mut r, &coll, PadInput::neutral().press(button::CROSS), 1);
    assert_eq!(r.hero.state, id::WATER_JUMP);
    assert_eq!(r.hero.jump.curve_on, 1);
    let n0 = r.log.len();
    run(&mut r, &coll, PadInput::neutral(), 90);
    let top = r.log[n0..].iter().map(|l| l.2[2]).fold(f32::MIN, f32::max);
    eprintln!("water jump apex {} (level {WATER}), states {:x?}", top, states(&r));
    assert!(top > WATER + 0.5, "apex {top}");
    // Falls back in and floats again.
    assert_eq!(r.hero.state, id::SURFACE_IDLE);
}

#[test]
fn swim_up_a_ramp_wades_out() {
    // Deep for x < 16, a ramp from 10 up to 15 over x 16..32, dry ground at 15 beyond.
    let floor = |x: f32| if x <= 16.0 { 10.0 } else if x >= 32.0 { 15.0 } else { 10.0 + (x - 16.0) * 0.3125 };
    let coll = pool(floor, Some(WATER), 0, 12, 0, 8);
    let mut r = runner([8.0, 16.0, 18.0]);
    run(&mut r, &coll, PadInput::neutral(), 150);
    assert_eq!(r.hero.state, id::SURFACE_IDLE);
    let mut seen = Vec::new();
    for _ in 0..600 {
        r.tick(&coll, PadInput::neutral().stick(0.0, -1.0));
        if seen.last() != Some(&r.hero.state) { seen.push(r.hero.state); }
        if r.hero.position()[0] > 36.0 { break; }
    }
    eprintln!("ramp: states {seen:x?}, end {:?}, depth {}", r.hero.position(), r.hero.f15f4.to_f32());
    assert!(seen.contains(&id::SURFACE_SWIM));
    assert!(seen.contains(&id::WADE), "never waded: {seen:x?}");
    assert!(r.hero.position()[0] > 32.0, "stuck at {:?}", r.hero.position());
    assert!(!matches!(r.hero.group, 0x11 | 0x12));
}

/// The ground probe takes the water level from the level's tables (a ripple patch) when they cover the hit.
#[test]
fn water_level_comes_from_the_water_tables() {
    struct Wave;
    impl WaterQuery for Wave {
        fn water_height(&self, p: [f32; 3]) -> Option<f32> { (p[0] < 30.0).then_some(p[2] + 0.1) }
    }
    let coll = deep();
    let mut r = runner([24.0, 24.0, 18.0]);
    r.water = Some(Box::new(Wave));
    run(&mut r, &coll, PadInput::neutral(), 150);
    assert_eq!(r.hero.state, id::SURFACE_IDLE);
    eprintln!("tables: level {} z {} bob {} smoothed {}", r.hero.water_level.to_f32(), r.hero.position()[2], r.hero.swim.bob.to_f32(), r.hero.swim.level.to_f32());
    assert_eq!(r.hero.water_level.to_f32(), WATER + 0.1);
    // The hero rides the table's level (the bob still settling from the fall).
    assert_eq!(r.hero.swim.level.to_f32(), WATER + 0.1);
    assert!((r.hero.position()[2] - (WATER + 0.1 - 0.12 + r.hero.swim.bob.to_f32())).abs() < 1e-5);
}
