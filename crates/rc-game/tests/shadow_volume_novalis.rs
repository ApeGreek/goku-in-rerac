//! Ratchet's posed shadow list (`rc_game::shadows::volume::pose_prims`, *BuildShadowList* 0x29b948) against the
//! game's, distilled from PCSX2 RAM at the Novalis idle savestate (`fixtures/shadow_ratchet_novalis_idle.tsv`:
//! his animation keys, rows, position, scale and size word as inputs; the 21 posed records as the expected
//! output). The pose comes from the port's joint evaluator (`moby_anim::evaluate_chains`, the joints' frames in
//! model space) on his class and the level's `ratchet_seq` table. Settles docs/plan/shadows.md §7 items 2 and 3:
//! the radius travels in w and is scaled by `scale·size/(1024·4096)`, and the shadow uses the posed joint matrices
//! (rotation included), not the skinning palette. Skipped without `extracted/`.

use rc_formats::moby_anim::{self, AnimState, MobyAnimClass};
use rc_formats::moby_shadow::ShadowBlock;
use rc_game::shadows::volume::{self, CasterPose};

struct Fixture {
    anim: (u8, u8, u8, u8, f32),
    rows: [[f32; 3]; 3],
    position: [f32; 3],
    scale: f32,
    size: u16,
    slab: [f32; 2],
    dir: [f32; 3],
    dir0: [f32; 3],
    pitch: f32,
    prims: Vec<(u8, [i32; 2], [f32; 4], [f32; 4])>,
}

fn fixture() -> Fixture {
    let text = include_str!("fixtures/shadow_ratchet_novalis_idle.tsv");
    let mut fx = Fixture { anim: (0, 0, 0, 0, 0.0), rows: [[0.0; 3]; 3], position: [0.0; 3], scale: 0.0, size: 0, slab: [0.0; 2], dir: [0.0; 3], dir0: [0.0; 3], pitch: 0.0, prims: Vec::new() };
    let mut row = 0;
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let v: Vec<&str> = line.split('\t').collect();
        let f = |k: usize| v[k].parse::<f32>().unwrap();
        let f3 = |k: usize| [f(k), f(k + 1), f(k + 2)];
        let f4 = |k: usize| [f(k), f(k + 1), f(k + 2), f(k + 3)];
        match v[0] {
            "anim" => fx.anim = (v[1].parse().unwrap(), v[2].parse().unwrap(), v[3].parse().unwrap(), v[4].parse().unwrap(), f(5)),
            "row" => {
                fx.rows[row] = f3(1);
                row += 1;
            }
            "position" => fx.position = f3(1),
            "scale" => fx.scale = f(1),
            "size" => fx.size = v[1].parse().unwrap(),
            "slab" => fx.slab = [f(1), f(2)],
            "dir" => fx.dir = f3(1),
            "dir0" => fx.dir0 = f3(1),
            "pitch" => fx.pitch = f(1),
            "prim" => fx.prims.push((v[1].parse().unwrap(), [v[2].parse().unwrap(), v[3].parse().unwrap()], f4(4), f4(8))),
            k => panic!("fixture key {k}"),
        }
    }
    fx
}

/// Ratchet's class 0 and his animation class with the level's `ratchet_seq` sequences (as the engine builds it).
fn ratchet() -> Option<(ShadowBlock, MobyAnimClass)> {
    let c = rc_formats::test_data::core(1)?;
    let blob = c.block("moby_class/0000")?;
    let class = rc_formats::moby::parse_moby_class(blob).unwrap();
    let block = ShadowBlock::of_class(blob).unwrap().unwrap();
    let seqs = (0..c.core.ratchet_seqs.len())
        .map(|i| c.block(&format!("ratchet_seq/{i:03}")).and_then(|b| moby_anim::parse_sequence(b, 0).ok()))
        .collect();
    Some((block, MobyAnimClass::new(&class, seqs)))
}

#[test]
fn ratchet_posed_list_matches_the_game() {
    let Some((block, anim)) = ratchet() else {
        eprintln!("skipped: no extracted/");
        return;
    };
    let fx = fixture();
    assert_eq!(block.prims.len(), fx.prims.len());
    let (seq_a, frame_a, seq_b, frame_b, t) = fx.anim;
    let state = AnimState { seq_a, frame_a, seq_b, frame_b, t, speed: 1.0, rate: 1.0, flags: 0, trigger_count: 0, skip_advance: false };
    let chains: Vec<Vec<u8>> = (0..anim.joint_count).map(|j| (0..=j as u8).collect()).collect();
    let refs: Vec<&[u8]> = chains.iter().map(Vec::as_slice).collect();
    let joints = moby_anim::evaluate_chains(&anim, &state, None, &refs);
    let pose = CasterPose { joints: &joints, rows: fx.rows, position: fx.position, scale: fx.scale, size: fx.size };
    let got = volume::pose_prims(&block, &pose);
    // Joints under the game's runtime joint-modifier list (moby +0x64 at the savestate: the idle joint records 18, 21..24,
    // 49, 50 and the head / torso manipulators 2, 7, 8), which neither the port's evaluator nor its drawn pose
    // applies (docs/plan/moby_animation.md §6.4, hero/idle.rs "not ported"): every joint below them is excluded
    // from the exact comparison and only reported.
    let modified = [2usize, 7, 8, 18, 21, 22, 23, 24, 49, 50];
    let under = |mut j: usize| loop {
        if modified.contains(&j) { break true; }
        match anim.parent(j) { Some(p) if p != j => j = p, _ => break false }
    };
    let (mut worst, mut worst_r, mut worst_layer, mut exact) = (0.0f32, 0.0f32, 0.0f32, 0);
    for (k, (g, (ty, seg, a, b))) in got.iter().zip(&fx.prims).enumerate() {
        assert_eq!((g.capsule as u8, g.segments), (*ty, *seg), "record {k}: type / segments (negative counts become 0)");
        let layered = block.prims[k].joints().iter().any(|&j| under(j as usize));
        for (p, q) in [(g.a, a), (g.b, b)] {
            let d = (0..3).map(|c| (p[c] - q[c]).abs()).fold(0.0, f32::max);
            if layered { worst_layer = worst_layer.max(d) } else { worst = worst.max(d) }
            worst_r = worst_r.max((p[3] - q[3]).abs());
        }
        if !layered { exact += 1; }
    }
    eprintln!("{exact} records off the modifier list: largest point difference {worst:.2e} units; {} below it: {worst_layer:.3} units; radius {worst_r:.2e}", got.len() - exact);
    // Radii are exact up to float rounding: w only goes through ×k.
    assert!(worst_r < 1e-6, "radius {worst_r}");
    assert!(exact >= 11, "the legs and hips: {exact}");
    // Points: the port's evaluator and the game's 0x267fc0 agree (hardware_fidelity_layers.md "Tolerances").
    assert!(worst < TOLERANCE, "largest point difference {worst}");
    // The layered joints move by the head look / idle records only (a few centimetres to 0.18 units at the ears' tips).
    assert!(worst_layer < 0.25, "{worst_layer}");
    // The slab and direction 1 the fixture carries are the ones the unit tests of rc_game::shadows reproduce.
    assert!((fx.slab[0] - 59.88).abs() < 1e-5 && (fx.slab[1] - 60.24).abs() < 1e-5);
    assert!((fx.dir.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-5);
}

/// Accepted point difference (world units): hardware_fidelity_layers.md "Tolerances".
const TOLERANCE: f32 = 1e-4;

/// Directions 0 and 1 from Ratchet's light (light word 0: set 0's light A, the level's gameplay light bank) and the
/// settled pitch 0.97, against RAM 0x1af000 / 0x1af010; his slab on the flat spawn ground (59.88 / 60.24).
#[test]
fn directions_and_slab_match_the_game() {
    use rc_game::moby_runtime::Moby;
    use rc_game::shadows::{self, HeroShadowIn, ShadowDirs};
    let Some(g) = rc_formats::test_data::gameplay(1) else {
        eprintln!("skipped: no extracted/");
        return;
    };
    let fx = fixture();
    let bank = rc_formats::tfrag_light::parse_light_bank(&g).unwrap();
    let dir_a: Vec<[f32; 4]> = bank.sets.iter().map(|s| s.dir_a).collect();
    let light = shadows::light_dir(&dir_a, 0);
    let mut dirs = ShadowDirs::default();
    shadows::update_shadow_dir(&mut dirs, light);
    let mut m = Moby::zeroed();
    m.position = [fx.position[0], fx.position[1], fx.position[2], 0.0];
    m.bsphere = [fx.position[0] * 1024.0, fx.position[1] * 1024.0, (fx.position[2] + 0.6) * 1024.0, 0.85 * 1024.0];
    let mut pitch = fx.pitch;
    let h = HeroShadowIn { ground_z: fx.position[2], ..Default::default() };
    shadows::update_hero_shadow(&mut m, &mut pitch, &mut dirs, &h, light, |_, _| Some(60.0));
    let close = |a: [f32; 4], b: [f32; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 2e-6);
    assert!(close(dirs.0[0], fx.dir0), "direction 0 {:?} vs RAM {:?}", dirs.0[0], fx.dir0);
    assert!(close(dirs.0[1], fx.dir), "direction 1 {:?} vs RAM {:?}", dirs.0[1], fx.dir);
    assert!((m.shadow_lo - fx.slab[0]).abs() < 1e-5 && (m.shadow_hi - fx.slab[1]).abs() < 1e-5);
}
