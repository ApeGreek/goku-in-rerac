---
status: closed
job: batch-5 full-suite results
date: 2026-10-01
commit: 58c8b71
areas: [classes, hero, weapons, world]
---

# Batch 5: full-suite failures at the commit (for information, not blocking)

## 1. Summary

`cargo xtask test-full` on the batch-5 tree, run once before the commit:
1451 tests run, 1444 passed (20 slow), **7 failed**, 41 skipped (227.9 s under nextest). `cargo check-all` and
`cargo clippy-all` were clean. The batch was built without tests; the failures below are listed so the test expert
can decide, per test, whether the old expectation or the new code is wrong. The "most likely request" column comes
from each request's section 4 (Shared code touched); it is a lead, not a verdict.

| # | Test | Most likely request |
|---|------|---------------------|
| F1 | `rc-game follow_camera::level::tests::unported_class_is_wanted` | `2026-10-01-camera-classes.md` (expected) |
| F2 | `rc-game::world sea_levels::sea_inventory_all_levels` | `2026-10-01-water-liquids.md` (expected) |
| F3 | `rc-game::world water_levels::novalis_751_port_is_the_ripple_module` | `2026-10-01-water-liquids.md` |
| F4 | `rc-game::classes creatures_w3::boarders_wait_hidden_and_jump_aboard` | `2026-10-01-pokitaru-boats-creatures.md` |
| F5 | `rc-game::classes creatures_w3::boarding_limits_and_the_boats_end` | `2026-10-01-pokitaru-boats-creatures.md` |
| F6 | `rc-game::classes creatures_w3::a_knocked_biter_in_the_sea_bubbles_and_is_lost` | `2026-10-01-pokitaru-boats-creatures.md` |
| F7 | `rc-game::weapons hero_reactive_novalis::novalis_suck_cannon_pulls_and_fires_a_critter` | `2026-10-01-hero-bodies.md` / `2026-10-01-hero-consumers.md` |

## 2. Failures

### F1. `follow_camera::level::tests::unported_class_is_wanted` (rc-game unit test)
```
panicked at crates/rc-game/src/follow_camera/level.rs:1168:9:
assertion `left == right` failed
  left: None
 right: Some(3)
```
Camera-classes: the test uses class 3 as its "unported" class; the rail camera class 3 now runs, so the assertion
for class 3 changes. Expected; the test needs another unported class (or a new expectation).

### F2. `sea_levels::sea_inventory_all_levels` (rc-game::world)
```
panicked at crates/rc-game/tests/world/sea_levels.rs:38:9:
assertion `left == right` failed: level 01: sea ports
  left: [11]
 right: []
```
Water-liquids: `water/sea.rs` `PORTS` grew to 12 (ports 8–11, the liquid meshes); the test's EXPECTED inventory
does not list them yet (level 01 now reports port 11). Expected; the inventory needs the new ports for every level.

### F3. `water_levels::novalis_751_port_is_the_ripple_module` (rc-game::world)
```
panicked at crates/rc-game/tests/world/water_levels.rs:309:5:
assertion `left == right` failed: the stream
  left: 3647861496
 right: 3608014390
```
Water-liquids: 751's tick is now split into its game-order phases and the drip creates a 787 moby
(`water/managers.rs::update_751`), while the direct path (`RippleSim::tick_with`, no moby system) does not. The
port's RNG stream now differs from the direct sim's. The request says Novalis rand ledgers may change at drip ticks
(intended); the test expert decides whether the two paths should still match or the test compares the wrong thing.

### F4. `creatures_w3::boarders_wait_hidden_and_jump_aboard` (rc-game::classes)
```
panicked at crates/rc-game/tests/classes/creatures_w3.rs:347:5:
assertion `left == right` failed: shown
  left: (0, 1)
 right: (1, 0)
```
### F5. `creatures_w3::boarding_limits_and_the_boats_end` (rc-game::classes)
```
panicked at crates/rc-game/tests/classes/creatures_w3.rs:372:5:
assertion `left == right` failed: without an explosion
  left: 8
 right: 0
```
F4 and F5, Pokitaru boats: the boats 1075 on level 11 now update. The request (section 4) says the hidden boarders
appear when a boat's state passes 1 and are deleted at 5 (`skip_to_end`), and that tests that assumed the boats
static (state 0 forever) may now see them sailing. Both tests drive the boarders by hand on that assumption.

### F6. `creatures_w3::a_knocked_biter_in_the_sea_bubbles_and_is_lost` (rc-game::classes)
```
panicked at crates/rc-game/tests/classes/creatures_w3.rs:452:5:
assertion `left == right` failed
  left: 11
 right: 3
```
Level 11: the test counts new type-34 particles over one tick (expects the knocked biter's 3 bubbles, sees 11). Most
likely the boats (pokitaru-boats-creatures: they now sail and emit their own effects in the same tick, and path
13 / 28 moves with a boat); second candidate water-liquids (the underwater flag on level 11's moby water changed in
`fog_zones.rs`). The test should count only the biter's particles, or the extra 8 need explaining.

### F7. `hero_reactive_novalis::novalis_suck_cannon_pulls_and_fires_a_critter` (rc-game::weapons)
```
panicked at crates/rc-game/tests/weapons/hero_reactive_novalis.rs:477:5:
assertion `left == right` failed: the slot emptied
  left: (1, 0, 1)
 right: (0, 0, 0)
```
The critter is swallowed (all earlier asserts pass) and the fire happens, but on the fire tick the cannon's slot
still reads held. Most likely hero-bodies (`tick.rs`: the hero moby via `hero_moby`, `apply_cmds` moved after the
hero calls, the items update only on foot: a one-tick shift in when the slot empties would show exactly here) or
hero-consumers (`run_calls_with`, `items::slot_pass`). Also touched: hud-slots (the Suck Cannon count reads the
cannon's state, should be read-only). Not an intended change in any request: treat as a likely regression.

## 3. NO_IDLE hero digest

`test-full` compared `work/test-results/digest_job.txt` with the baseline `hero_digest_no_idle.txt` (not
rewritten; digest-baseline was not run): **DIFFERS**. The state / timer / pos columns are identical on all 2800
lines; only the hash column differs, from the first line on (`lake 0 state 0x0 timer 1`: `701721dcaa1a1412`, baseline
`6456e282d33b61e4`). Hash-only differences are expected from this batch: the camera moby's slot (camera-classes:
"the camera moby's slot", camera moby 1007 created after the camera update), the new camera target modes, and the
foot motes on the shared RNG. The test expert confirms the cause before the baseline is rewritten.

## 7. Results (the test expert fills this in)
