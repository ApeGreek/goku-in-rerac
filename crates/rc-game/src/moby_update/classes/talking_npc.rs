//! Talking NPCs, class 774: `TalkingNpcUpdate` level01 0x2ff118 (Novalis' "Water Pump Worker", Batalia's "Big
//! Turret Guy"). The dialogue is the shared talk system ([`interact::talk_update`], docs/plan/interaction.md §3);
//! this class only decides when to run it and what the end of a scene means. Native `f32`.
//!
//! The pvar block starts with the talk block (`crate::moby_update::interact::talk`); +0x20 the debug name, +0x44
//! cleared at init, +0x4c a checkpoint cuboid (−1 none).
//!
//! States (+0x20):
//! * **0**: +0x30 = 0xff (update distance), +0x44 = 0. Level 1: planet 2 (Aridia) already unlocked
//!   (`0x13dd42`) → state 3 (the worker is gone); else state 1. Level 8: state 4; others 7. Then
//!   `NpcTalkRegister`.
//! * **1**: `NpcTalkUpdate`; when it starts a scene: `FUN_002783a8(2.2, npc)` (after the scene Ratchet stands 2.2
//!   in front of the NPC, facing it: [`Interact::scene_end_place`]) and state 2.
//! * **2**: waits while game mode 2 (a scene) runs; then, the NPC's mission (+0xb0) not done yet: `SetMissionDone`
//!   ([`crate::cinematic::set_mission_done`]) and, with a checkpoint cuboid +0x4c, the checkpoint record
//!   `FUN_0029ac10(centre, Euler)` of that cuboid ([`super::checkpoint::record`]); state 1; when the node that
//!   played last (talk +0x04) is 4 (Novalis:
//!   the Infobot sold): `UnlockPlanet(2)` and `ShowPlanetBanner(2)` (`crate::cinematic`: the saved game's planet
//!   bits and the HUD banner "Infobot for Planet Aridia acquired"), state 3, a save (logged).
//! * **3**: `DeleteMoby`.
//! * **4 / 5** (Batalia's turret guy; state 5 moves Ratchet to a fixed point): not ported (counted).
//!
//! The head look-at (manipulators `FUN_002777d8` toward the hero within 8 units, random glances every 180..300
//! ticks) is not ported (counted).

use crate::moby_runtime::MobyId;
use crate::moby_update::interact::{self, talk, GameWrite};
use crate::moby_update::services::{pvar as p, World};

pub const UPDATE_FN: u32 = 0x2ff118;
pub const CLASSES: [i16; 1] = [774];
/// `FUN_002783a8(2.2, npc)`.
const PLACE_DISTANCE: f32 = 2.2;

/// `TalkingNpcUpdate` (0x2ff118).
pub fn update(w: &mut World, id: MobyId) {
    if w.m(id).pvars.len() < 0x50 { return; }
    interact::poll_scene_end(w, id);
    // Drawn last frame and within 32 units of the camera: the shadow probe, shadow range 0x1a.
    if w.m(id).visible != 0 {
        let (p, c) = (w.m(id).position, w.camera.map(|x| f32::from_bits(x.0)));
        if ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt() < 32.0 {
            crate::shadows::probe_down(w, id);
            w.mm(id).b7f = 0x1a;
        }
    }
    match w.m(id).state {
        0 => {
            w.mm(id).update_dist = 0xff;
            p::set_i32(&mut w.mm(id).pvars, 0x44, 0);
            if w.svc.level == 1 {
                if w.svc.interact.game.planet_unlocked.get(2).copied().unwrap_or(0) != 0 {
                    w.delete_moby(id);
                    return;
                }
                w.mm(id).state = 1;
            } else {
                w.mm(id).state = if w.svc.level == 8 { 4 } else { 7 };
            }
            interact::talk_register(w, id);
        }
        1 => {
            if interact::talk_update(w, id) {
                let m = w.m(id);
                let yaw = m.rotation[2];
                let pos = [m.position[0] + yaw.cos() * PLACE_DISTANCE, m.position[1] + yaw.sin() * PLACE_DISTANCE, m.position[2]];
                w.svc.interact.scene_end_place = Some((pos, interact::add_rot(yaw, std::f32::consts::PI)));
                w.mm(id).state = 2;
            }
        }
        2 => {
            if w.svc.interact.talker == Some(id) { return; }
            let mission = w.m(id).mission;
            if mission != 0xff && w.missions.mission_done(w.svc.level, mission) != 0xff {
                crate::cinematic::set_mission_done(w, mission);
                let c = p::i32(&w.m(id).pvars, 0x4c);
                if let Some(s) = w.svc.volumes.shape(rc_formats::volumes::ShapeKind::Cuboid, c).copied() {
                    super::checkpoint::record(w, super::checkpoint::Record { pos: s.centre(), rot: s.euler });
                }
            }
            w.mm(id).state = 1;
            if p::i16(&w.m(id).pvars, talk::LAST) == 4 {
                // `UnlockPlanet(2)` (its own banner unless on level 2), `ShowPlanetBanner(2)`, the save.
                crate::cinematic::unlock_planet(w, 2);
                crate::cinematic::show_planet_banner(w, 2);
                w.mm(id).state = 3;
                w.svc.interact.writes.push(GameWrite::Save);
            }
        }
        3 => w.delete_moby(id),
        _ => w.svc.unported("talking npc: Batalia turret guy states 4/5"),
    }
    w.svc.unported("talking npc: head look-at");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cinematic::EngineRequest;
    use crate::moby_runtime::{Moby, MobyTable};
    use crate::moby_update::services::LevelMissions;

    /// State 2 after the talker's scene: its open mission is set done through the one mission writer and its
    /// checkpoint cuboid (+0x4c) becomes the checkpoint record (`FUN_0029ac10(centre, Euler)`); once done, nothing.
    #[test]
    fn scene_end_sets_mission_and_checkpoint() {
        let mut m = Moby { o_class: 774, mission: 1, state: 2, pvars: vec![0; 0x50], ..Moby::default() };
        p::set_i32(&mut m.pvars, 0x4c, 0);
        let mut t = MobyTable::new(vec![m], 4);
        let hero = crate::hero::Hero::new();
        let mut rng = crate::rng::Rng::new();
        let classes = crate::moby_update::ClassTable::default();
        let mut svc = crate::moby_update::Services::new();
        svc.level = 1;
        let (c, e) = ([1.0, 2.0, 3.0], [0.0, 0.0, 1.5]);
        svc.volumes = std::sync::Arc::new(rc_formats::volumes::Volumes { cuboids: vec![super::super::checkpoint::tests::cube(c, e)], ..Default::default() });
        let open = LevelMissions::fresh_load(1, [0; 16]);
        {
            let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 0);
            w.missions = &open;
            update(&mut w, 0);
            assert_eq!(w.m(0).state, 1);
            assert_eq!(w.svc.cinematic.requests, [EngineRequest::MissionDone { mission: 1 }]);
            assert_eq!(w.svc.save.checkpoint, Some(super::super::checkpoint::Record { pos: c, rot: e }));
        }
        let mut done = open.clone();
        done.done[1] = 0xff;
        svc.cinematic.requests.clear();
        svc.save.checkpoint = None;
        t.mobys[0].state = 2;
        let mut w = World::new(&mut t, &hero, &mut rng, &classes, &mut svc, 1);
        w.missions = &done;
        update(&mut w, 0);
        assert!(w.svc.cinematic.requests.is_empty() && w.svc.save.checkpoint.is_none());
    }
}
