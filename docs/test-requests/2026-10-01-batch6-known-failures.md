---
status: open
job: batch-6 full-suite results
date: 2026-10-01
commit: 7bf9dc6
areas: [classes, ui, weapons, world]
---

# Batch 6: full-suite failures at the commit (for information, not blocking)

## 1. Summary

`cargo xtask test-full` on the batch-6 tree, run once before the commit: 1451 tests run, 1427 passed (17 slow),
**24 failed**, 41 skipped (251.6 s under nextest). `cargo check-all` and `cargo clippy-all` were clean. The batch was
built without tests and added none; the failures are listed so the test expert can decide, per test, whether the old
expectation or the new code is wrong. The "most likely request" column comes from each request's section 4 (Shared
code touched); it is a lead, not a verdict.

Seven of the 24 are the batch-5 failures, still failing (section 3). Seventeen are new.

**Boot order (high risk, planet-travel).** The travel lane moved every per-level renderer's start onto the new
`LevelStartup` / `LevelPostStartup` schedules. **No frame-exact screenshot, boot or trace test failed — but the suite
has none that boots the engine.** The rc-engine tests that ran are its 66 unit tests (all passed); rc-trace's 31 tests
passed. None of them covers the boot order, so this suite does not check the boot-order change: the frame-exact
captures in planet-travel section 4 still need to be re-run by hand.

| # | Test | Most likely request |
|---|------|---------------------|
| N1 | `rc-game moby_update::classes::breakables::tests::every_port_address_names_one_port` | `2026-10-01-minigames.md` |
| N2 | `rc-game::classes cheap_classes_a::units_resolve_on_their_levels` | `2026-10-01-minigames.md` |
| N3 | `rc-game::classes cheap_classes_b::units_resolve_on_their_levels` | `2026-10-01-minigames.md` |
| N4 | `rc-game::classes cheap_classes_c::units_resolve_on_their_levels` | `2026-10-01-minigames.md` |
| N5 | `rc-game::classes cheap_classes_f::air_traffic_wreck_blast_bolts_pieces_sound_and_hide` | `2026-10-01-story.md` (expected) |
| N6 | `rc-game::classes orxon_gemlik_enemies::weapons_hurt_and_kill_a_scout` | `2026-10-01-story.md` (expected) |
| N7 | `rc-game::classes enemy_units::a_flyer_knocked_into_a_pit_dies` | `2026-10-01-media-cheats.md` |
| N8 | `rc-game::classes enemy_units::buzz_bombs_chase_ratchet_and_blow_up` | `2026-10-01-media-cheats.md` |
| N9 | `rc-game::classes enemy_units::flyers_land_charge_and_bite_ratchet` | `2026-10-01-media-cheats.md` |
| N10 | `rc-game::classes enemy_units::the_suck_cannon_and_the_taunter_act_on_a_flyer` | `2026-10-01-media-cheats.md` |
| N11 | `rc-game::classes enemy_units::the_taunter_alerts_a_buzz_bomb` | `2026-10-01-media-cheats.md` |
| N12 | `rc-game::classes enemy_units::weapons_knock_back_and_kill_a_flyer` | `2026-10-01-media-cheats.md` |
| N13 | `rc-game::ui interaction_vendor::novalis_vendor_prompt_open_buy_close` | `2026-10-01-space-visuals.md` / `2026-10-01-menus-flow.md` |
| N14 | `rc-game::ui pause_pages_novalis::goodies_skill_points_and_movies` | `2026-10-01-media-cheats.md` (likely expected) |
| N15 | `rc-game::world cutscene_novalis::bridge_cutaway_after_the_mission` | `2026-10-01-story.md` / `2026-10-01-media-cheats.md` |
| N16 | `rc-game::world level_ports::novalis_ports_are_the_class_number_registry` | `2026-10-01-planet-travel.md` (expected) |
| N17 | `rc-game::world level_ports::every_level_runs_the_ports_its_class_table_names` | `2026-10-01-planet-travel.md` (expected) |
| F1 | `rc-game follow_camera::level::tests::unported_class_is_wanted` | batch 5: `2026-10-01-camera-classes.md` |
| F2 | `rc-game::world sea_levels::sea_inventory_all_levels` | batch 5: `2026-10-01-water-liquids.md` |
| F3 | `rc-game::world water_levels::novalis_751_port_is_the_ripple_module` | batch 5: `2026-10-01-water-liquids.md` |
| F4 | `rc-game::classes creatures_w3::boarders_wait_hidden_and_jump_aboard` | batch 5: `2026-10-01-pokitaru-boats-creatures.md` |
| F5 | `rc-game::classes creatures_w3::boarding_limits_and_the_boats_end` | batch 5: `2026-10-01-pokitaru-boats-creatures.md` |
| F6 | `rc-game::classes creatures_w3::a_knocked_biter_in_the_sea_bubbles_and_is_lost` | batch 5: `2026-10-01-pokitaru-boats-creatures.md` |
| F7 | `rc-game::weapons hero_reactive_novalis::novalis_suck_cannon_pulls_and_fires_a_critter` | batch 5: `2026-10-01-hero-bodies.md` / `2026-10-01-hero-consumers.md` |

## 2. New failures

### N1–N4. One update address, two units (rc-game unit test and three `units_resolve_on_their_levels`)
```
panicked at crates/rc-game/src/moby_update/classes/breakables.rs:382:13:
assertion `left == right` failed: Unit(233) 0x2ece00
  left: Some(Unit(95))
 right: Some(Unit(233))
---
panicked at crates/rc-game/tests/classes/cheap_classes_{a,b,c}.rs:{61,69,63}:57:
Unit(233) and Unit(95) share 0x2ece00
```
Minigames: `hoven_drone.rs` registers the level-12 drone shot's update at `0x2ece00`, and the Oltanis pop-up turret's
shot 681 (U440, level 14, `units/mod.rs`) already uses `0x2ece00`. Overlay addresses repeat across levels, but these
four tests require one unit per address, so they all fail on this one collision. Either the registry or the tests
need to key on (level, address).

### N5. `cheap_classes_f::air_traffic_wreck_blast_bolts_pieces_sound_and_hide` (rc-game::classes)
```
panicked at crates/rc-game/tests/classes/cheap_classes_f.rs:238:9:
assertion `left == right` failed: class 795: only 795's skill point is not ported
  left: 0
 right: 1
```
### N6. `orxon_gemlik_enemies::weapons_hurt_and_kill_a_scout` (rc-game::classes)
```
panicked at crates/rc-game/tests/classes/orxon_gemlik_enemies.rs:278:5:
assertion `left == right` failed
  left: None
 right: Some(1)
```
N5 and N6 are expected. Story section 4 says the skill points are now ported, so these `unported` counters are 0 /
None now. The expectations need updating.

### N7–N12. Novalis flyers and buzz bombs (rc-game::classes `enemy_units`)
```
enemy_units.rs:776:5  assertion failed: lv.table.mobys[22].state >= 0x80              (a_flyer_knocked_into_a_pit_dies)
enemy_units.rs:817:5  #26 states [2, 3, 4, 253]                                       (buzz_bombs_chase_ratchet_and_blow_up)
enemy_units.rs:694:5  states [[15], [15], [15], [15], [15], [17]]; no LAND → CHARGE    (flyers_land_charge_and_bite_ratchet)
enemy_units.rs:762:5  assertion failed: back                                          (the_suck_cannon_and_the_taunter_act_on_a_flyer)
enemy_units.rs:875:5  left: 12.0  right: 24.0                                         (the_taunter_alerts_a_buzz_bomb)
enemy_units.rs:720:5  left: 15  right: 6                                              (weapons_knock_back_and_kill_a_flyer)
```
All six tests drive the flying biter 63 and the buzz bomb 52, the two classes in this file that media-cheats changed:
each update now calls `manip::big_head` (flying_biter at +0x230, buzz_bomb at +0x280). The request says there is "no
change with the cheats off". But `head_node` with the cheat off still calls `detach` whenever `attached(owner, ofs)`
reads true. If these classes keep their own state in that pvar range, an off-cheat call would change them. Other
tests in the same binary pass, so this looks like a regression rather than an intended change.

### N13. `interaction_vendor::novalis_vendor_prompt_open_buy_close` (rc-game::ui)
```
talk slots 11; hand-offs [(100, OpenVendor { vendor: Some(3) })]; purchases [(215, (16, false, 2500, 1))]; ...
panicked at crates/rc-game/tests/ui/interaction_vendor.rs:325:5:
assertion `left == right` failed: 40 frames of substate 2 after the frame that entered it
  left: 420
 right: 288
```
The vendor's exit runs 172 frames longer than before: substate 2 lasts 173 frames where the test expects 40. The power-off and fold asserts before it pass. Leads:
space-visuals (`interact_render` vendor: the demo request and `VendorExit(1)`, `travel_render`'s vendor demo) and
menus-flow (vendor leftovers: `interact_render` exit stops speech, `vendor_render`). Not an intended change in either
request.

### N14. `pause_pages_novalis::goodies_skill_points_and_movies` (rc-game::ui)
```
panicked at crates/rc-game/tests/ui/pause_pages_novalis.rs:412:5:
assertion `left == right` failed
  left: (1797280, [Denied])
 right: (1797280, [])
```
Pressing ✕ on the locked Sketchbook entry now plays the Denied sound, where the test expects silence. Media-cheats: `enter` calls
`goodies_unlocks` (the Goodies items 5..8 change from −1 to 2 / 3 / 10; more Goodies pages loaded). Probably intended:
the test's "action −1, no sound" assumption is out of date. Confirm against the game.

### N15. `cutscene_novalis::bridge_cutaway_after_the_mission` (rc-game::world)
```
requests: [(0, StartScene { scene: 5, arrival: true }), (0, MissionDone { mission: 3 }), (6, MissionDone { mission: 4 }),
           (545, StartScene { scene: 3, arrival: false })]
panicked at crates/rc-game/tests/world/cutscene_novalis.rs:259:5:
[StartScene { scene: 5, arrival: true }, MissionDone { mission: 3 }, MissionDone { mission: 4 }, StartScene { scene: 3, arrival: false }]
```
Scene 3 starts, but `StartMovie { movie: 3 }` and scene 4 never follow, and the missions done are now 3 and 4 at ticks 0 / 6
(not 0). Leads: story (`cinematic::start_scene` now sets game mode 2, so the mission NPC's update in the same tick
reads mode 2, and the Novalis story driver marks missions done at load) and media-cheats (`play_movie_b` /
`enter_slideshow` set `game_mode`, `EngineRequest::PauseSounds` replaced). Needs a look: the cutaway chain stops
after scene 3.

### N16–N17. Level 01 class 531 is now the ship (rc-game::world `level_ports`)
```
level_ports.rs:38:9   level 01 class 531 (update 0x2ece90)  left: Some(Unit(16))  right: Some(Ship)
level_ports.rs:55:39  level 01 class 531                    left: Ship            right: Unit(16)
```
Expected. Planet-travel says `ClassUpdate::Ship` now runs the ship's update in the moby loop, so the class-number registry
maps 531 to `Ship`, while the code address `0x2ece90` still resolves to `Unit(16)`. The tests need the ship as an
exception, or the Ship port needs registering by address.

## 3. Batch-5 failures still failing

All seven failures from `2026-10-01-known-failures.md` still fail, with the same assertion values. Details are in that file:
- F1 `follow_camera::level::tests::unported_class_is_wanted`: left `None`, right `Some(3)` (level.rs:1168).
- F2 `sea_levels::sea_inventory_all_levels`: level 01 sea ports left `[11]`, right `[]` (sea_levels.rs:38).
- F3 `water_levels::novalis_751_port_is_the_ripple_module`: the stream left `3647861496`, right `3608014390`.
- F4 `creatures_w3::boarders_wait_hidden_and_jump_aboard`: shown left `(0, 1)`, right `(1, 0)`.
- F5 `creatures_w3::boarding_limits_and_the_boats_end`: without an explosion left `8`, right `0`.
- F6 `creatures_w3::a_knocked_biter_in_the_sea_bubbles_and_is_lost`: left `11`, right `3`.
- F7 `hero_reactive_novalis::novalis_suck_cannon_pulls_and_fires_a_critter`: the slot emptied left `(1, 0, 1)`,
  right `(0, 0, 0)`.

## 4. NO_IDLE hero digest

`test-full` compared `work/test-results/digest_job.txt` with the baseline `hero_digest_no_idle.txt`. The baseline was
not rewritten: digest-baseline was not run. Result: **DIFFERS**, the same as at batch 5. The state / timer / pos columns match on all 2800
lines. Only the hash column differs, from the first line on: `lake 0 state 0x0 timer 1 pos [162.53032, 136.39348,
60.0]`, `701721dcaa1a1412` vs baseline `6456e282d33b61e4`. The first-line hash is the same as in the batch-5 run, so
batch 6 does not appear to have changed it. The batch-5 causes still apply: the camera moby's slot, the camera target
modes and the foot motes. The fields gadgets added to `post_move` (f52a / f52c count-down, disguise timer) did not change
the printed columns.

## 7. Results (the test expert fills this in)
