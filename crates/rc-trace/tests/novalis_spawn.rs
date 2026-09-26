//! Regression on the first real savestate (Novalis spawn, 2026-09-26; docs/plan/trace_results_novalis.md).
//! Needs `extracted/traces/novalis_spawn_ee.bin` (`rc-trace dump-ee --state latest --out ...`, disc-derived,
//! git-ignored) and `extracted/`; skipped when either is absent.

use rc_trace::ee::EeSource;
use rc_trace::novalis_spawn as ns;

#[test]
fn novalis_spawn_state_checks() {
    let root = rc_trace::default_extracted();
    let dump = root.join("traces/novalis_spawn_ee.bin");
    if !dump.exists() || !root.join("levels/01/core_data.dec").exists() { eprintln!("skipped: no savestate dump"); return; }
    let img = EeSource::Raw(dump).load().unwrap();
    let mut out = ns::Out::default();
    // Tie / shrub palettes: shrubs all equal, ties all but one entry (a ±1 on two channels).
    ns::check_tie_shrub(&img, &root, &mut out, 0).unwrap();
    let t = |out: &ns::Out, name: &str| out.tallies.iter().find(|t| t.0 == name).map(|t| (t.1, t.2)).unwrap();
    assert_eq!(t(&out, "g. shrub entries"), (28992, 28992));
    assert!(t(&out, "g. tie entries").0 >= 96511);
    // Game state: every chunk equal except flags 15/16 (scene 5) and the entry-record times.
    let gs = ns::port_game_state(&root).unwrap();
    ns::check_game_state(&img, &gs, &mut out).unwrap();
    let (ok, total) = t(&out, "a. game state (chunks, excl. play-dependent)");
    assert!(total - ok <= 4, "{ok}/{total}");
    // The port run on the savestate's timeline (trace_results_novalis.md "Fixes applied"): the spawn test and
    // the index remap, the load pass pinned on the stream, the tick counter, 751's underwater colours.
    let tl = ns::Timeline::read(&img).unwrap();
    let lv = rc_trace::port_sim::LevelData::load(&root, 1).unwrap();
    let opt = rc_trace::port_sim::SimOptions { cutscene_ticks: tl.ticks.saturating_sub(tl.mode_frames.max(0) as u64), ..Default::default() };
    let sim = ns::run_port_hold(&lv, &tl, 2, &opt).unwrap();
    ns::check_rng(&img, &sim, &tl, &mut out).unwrap();
    ns::check_fog(&img, &lv, &sim, &mut out).unwrap();
    let csv = std::env::temp_dir().join("rc_trace_novalis_spawn_mobys_test.csv");
    ns::check_mobys(&img, &sim, &mut out, &csv).unwrap();
    for name in ["e. tick counter", "f. moby slots paired by index", "f. rejected instances absent from RAM", "e. load pass: bolt spin bits",
        "e. load pass: crate 500 turns", "d. underwater look", "d. fog globals"] {
        let (ok, total) = t(&out, name);
        assert!(total > 0 && ok == total, "{name}: {ok}/{total}");
    }
    assert_eq!(t(&out, "f. moby slots paired by index"), (929, 929));
    assert_eq!(t(&out, "f. rejected instances absent from RAM"), (54, 54));
}
