//! The per-level class registry (`moby_update::classes::LevelPorts`) on the 19 level overlays: the level's class
//! table decides which classes run a port (docs/plan/level_generalisation.md C1). Skipped without `extracted/`.

use rc_formats::level_overlay::LevelOverlay;
use rc_game::moby_update::classes::{for_class, ClassUpdate, LevelPorts};
use std::sync::Arc;

fn overlay(level: u32) -> Option<Arc<LevelOverlay>> {
    let b = std::fs::read(rc_formats::test_data::level_dir(level).join("overlay.bin")).ok()?;
    Some(Arc::new(LevelOverlay::parse(&b).unwrap()))
}

pub fn ports(level: u32) -> Option<LevelPorts> {
    let ov = overlay(level)?;
    Some(LevelPorts::from_overlays(&ov, &overlay, &[0x2bd100, 0x2fd0e8]))
}

#[test]
fn novalis_ports_are_the_class_number_registry() {
    let (Some(ov), Some(p), Some(gp)) = (overlay(1), ports(1), rc_formats::test_data::gameplay(1)) else { eprintln!("skipped: no extracted/"); return };
    let placed: std::collections::HashSet<i16> = rc_formats::gameplay::parse_moby_instances(&gp).unwrap().iter().map(|m| m.o_class as i16).collect();
    for e in ov.vtbl() {
        let oc = e.o_class as i16;
        if oc == 731 {
            // The third mission NPC class: the table runs MissionNpcUpdate for it too (not placed on Novalis).
            assert!(!placed.contains(&oc));
            assert_eq!(p.get(oc), Some(ClassUpdate::MissionNpc));
            continue;
        }
        assert_eq!(p.get(oc), for_class(oc), "level 01 class {oc} (update {:#x})", e.update);
    }
}

#[test]
fn every_level_runs_the_ports_its_class_table_names() {
    let Some(_) = overlay(1) else { eprintln!("skipped: no extracted/"); return };
    for level in 0..19 {
        let ov = overlay(level).unwrap();
        let p = ports(level).unwrap();
        let mut lost = Vec::new();
        let mut gained = Vec::new();
        for e in ov.vtbl() {
            let oc = e.o_class as i16;
            match (for_class(oc), p.get(oc)) {
                (Some(a), None) => lost.push((oc, a, format!("{:#x}", e.update))),
                (None, Some(b)) => gained.push((oc, b)),
                (Some(a), Some(b)) => assert_eq!(a, b, "level {level:02} class {oc}"),
                (None, None) => {}
            }
        }
        eprintln!("level {level:02}: {} classes, {} unported; by number but other code here {lost:?}; same code, other class number {gained:?}", ov.vtbl().len(), p.unported().len());
    }
    // Gemlik: its body pieces run FxGroupUpdate, the vendor, bombs and teleporters their ports.
    let p = ports(13).unwrap();
    for oc in [1733, 1734, 1735, 1801, 1802, 1803, 1804] { assert_eq!(p.get(oc), Some(ClassUpdate::FxPiece), "class {oc}"); }
    // The class-27 particle emitters (the engine's external update) are Novalis's own; the ripple manager 751 is a
    // water port (`rc_game::water::managers`), also only on Novalis.
    for level in 0..19 {
        let p = ports(level).unwrap();
        assert_eq!(p.external(27), (level == 1).then_some(0x2bd100), "level {level:02}");
        assert_eq!(p.in_table(751) && p.get(751) == Some(ClassUpdate::Water(0)), level == 1, "level {level:02}");
    }
    for (oc, u) in [(11, ClassUpdate::Vendor), (121, ClassUpdate::Bomb), (1135, ClassUpdate::TeleporterPad), (500, ClassUpdate::Crate), (758, ClassUpdate::SwingTarget), (803, ClassUpdate::SwingTarget)] {
        assert_eq!(p.get(oc), Some(u), "class {oc}");
    }
}
