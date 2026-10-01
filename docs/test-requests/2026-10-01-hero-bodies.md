---
status: open
job: hero-bodies
date: 2026-10-01
commit: 58c8b71
areas: [hero, classes, ui]
---

# The other hero bodies: the body switch, Clank (body 1), Giant Clank (body 2) and the classes that hand them over

## 1. Summary

- **One body system** (`rc-game/src/hero/bodies.rs`, G-HERO-005): the body word 0x1413f4 (`Hero::mode`: 0 Ratchet,
  1 Clank, 2 Giant Clank, 3 Hologuise) picks the moby the hero code drives (`Hero::hero_moby`), the states it runs, the
  capsule, the wall / ledge probe heights, the bolt pickup radii, the HUD and the classes' branches. Ported once as the
  superset of every level's copy: `SwitchCharacter` (L01 `0x231348` family + L00 `0x210a08` family with Giant Clank's
  energy 200), leaving the body (L01 `0x231450` / L00 `0x210b30` and every level copy), the per-body idle `0x227638`
  (what `SetState(0)` becomes in a body), and the body's hero update `HeroUpdateAlt` (L00 `0x2062b0`).
- **Clank** (`hero/bodies/clank.rs`): states 0x43..0x52 and 0x7d (entries, physics, transitions), his own health
  0x1415fc, the lean `0x215b68`, the antenna glow moby 0x4b4, the rotor moby 0x47a, the command flash.
- **Giant Clank** (`hero/bodies/giant.rs`): states 0x5a..0x62, his energy 0x140980 and the hit intake's body-2 branch
  (`0x231580`), the walk / fall / punch smash spheres, the landing shockwave (class 0x593), the pilot Ratchet in the
  cockpit (`0x2061f0`).
- **Classes that hand the hero over** (all placed instances): Orxon's Clank section 22 (L10, 1), Blarg's Clank
  station 1061 (L06, 1), Giant Clank's pads 1451 (L15, 1) / 1899 (L18, 1), the shockwave 0x593 (L15 / L18, made by
  the hero code).
- **Plug-and-play**: a class hands the hero a body with `hero::bodies::queue_switch(w, mode, state, moby)` and gives
  it back with `queue_leave(w)` (both go through `HeroCall`, applied by the tick); `HeroCall::BodyIdle` is the per-body
  idle; a class branching on the body reads `World::hero.mode` (Clank's kick: `hero.state == 0x51`). The engine binds
  the hero animation to the body moby (`AnimCtl::bind_body`, `HeroAnimCtl`), draws the body from the table and loads the
  body classes' joint lists (`BodyJoints`). Camera class 22 (giant-Clank focus, another lane) can read
  `Hero::mode == 2` and `Hero::hero_moby`.

## 2. Where it lives

- `crates/rc-game/src/hero/bodies.rs`: `switch_character`, `leave_body`, `body_idle`, `queue_switch`, `queue_leave`,
  `restore_from_checkpoint`, `apply_cmds` (`BodyCmd`), `body_update`, `springs`, `glow`, `after_update`; module doc =
  the coverage table of the switch / leave / idle / HeroUpdateAlt / HUD / checkpoint rows.
- `crates/rc-game/src/hero/bodies/clank.rs`: `entry`, `jump_entry`, `kick_entry`, `physics`, `climb_horizontal`,
  `transitions` (`tr_*`), `lean`, `after_update`; coverage table in its module doc (L00 SetState `0x2223f8`, physics
  `0x217970`, transitions `0x229b70`).
- `crates/rc-game/src/hero/bodies/giant.rs`: `entry`, `jump_entry`, `physics`, `punch`, `transitions`, `beam_end`,
  `after_update`, `Spawn`; coverage table in its module doc.
- Hero hooks: `hero.rs` (body update branch in `hero_update_with_sounds`), `hero/anim.rs` (`HeroAnimCtl`, `BodyAnim`,
  `AnimCtl::bind_body` / `bound_body` / `body_snapshot` / `ratchet_generic`), `hero/ground.rs` (SetState 0 in a body),
  `hero/common.rs` (`idle_seq` 0 in a body), `hero/physics.rs` (`size_capsule`, `wall_check`, `post_move` per body),
  `hero/ledge.rs` (`LedgeDims`, Clank's dims, probe B's mode-1 rule), `hero/jump.rs` (0x4c's horizontal, Clank's
  lean), `hero/damage.rs` (body-2 hit intake), `hero/fx.rs` (`JointData::bodies`, `voice_queue`), `hero/idle.rs`
  (`clank_glow_word`, `Idle::detach_all`), `hero/worn.rs` (`Worn::airless`), `hero/registry.rs` (routing).
- Units: `moby_update/classes/units/clank_section.rs` (L10 0x298b68), `blarg_clank_lift.rs` (L06 0x2fc640),
  `giant_pad.rs` (L15 0x2ed068 / L18 0x2fb868), `giant_shockwave.rs` (L15 0x29e9f8 spawn / 0x29ead0 update).

## 3. Behaviours to verify

### B1. The switch into a body (`switch_character`, 0x231348 / 0x210a08)
- **Claim:** in order: a raised weapon put away; 0x1413f6 = 1; the hero moby = Ratchet and his after-images end;
  Ratchet's collision off (+0x98 = −1); Ratchet and his items hidden; the hero's looping sounds stopped; every joint
  record detached (angles / targets 0, scale 1); `Hero::mode` = mode and the hero moby = the body moby; the capsule
  resized and snapped to its targets; body +0x34 |= 6; playback speed / rate 1; the anim loop cleared;
  `Bodies::state_param` = state; mode 1: Clank's health swapped in (`saved_health` = health, health = `clank_health`);
  mode 2: energy = 200; then `SetState(state, 1)`.
- **Setup:** any level, Ratchet on foot with a weapon raised and an after-image trail if possible; a Clank (0x57) or
  Giant Clank (0x1a3) moby in the table.
- **Trigger:** `queue_switch(w, 1, 0x43, clank)` / `(2, 0x5a, giant)` from a class update (or the units below).
- **Expect:** after the tick `hero.mode` = 1 / 2, `hero.hero_moby(ratchet)` = the body, Ratchet's moby hidden and
  without collision, `hero.state` = 0x43 / 0x5a, the capsule = Clank 0.45 / 0.3 / 0.6 or Giant Clank 4.45 / 3.75 /
  5.25 (bottom / radius / top), Clank: health = his own (4 the first time), Giant Clank: `bodies.energy` = 200.
- **Edge cases:** switching while a weapon is up; switching twice in a row; the hero animation is bound to the body
  (sequences are the body class's, not Ratchet's).
- **Suggested method:** unit test on `switch_character` + `apply_cmds` with a synthetic table; level-harness test on
  L10.

### B2. Leaving the body (`leave_body`, 0x231450 / 0x210b30)
- **Claim:** mode 1: Clank's health saved back, Ratchet's restored, the rotor and the glow mobys deleted; mode 2 with
  an energy HUD handle: released (−1); every joint record detached; mode = 0, body +0x34 &= ~6, body moby cleared;
  loops stopped; hero moby = Ratchet placed at the hero's position, his collision back (+0x98 = 0), rate 1, loop
  cleared; unless state 100 in game mode 2 / 6: `SetState(0, 1)`.
- **Setup / Trigger:** as Clank or Giant Clank, `queue_leave(w)`.
- **Expect:** Ratchet shown at the body's last position, state 0, health = his own; 0x4b4 / 0x47a mobys gone.
- **Edge cases:** leaving in state 100 with game mode 2 / 6 (no SetState); leaving body 2 twice (handle already −1).
- **Suggested method:** unit test.

### B3. The per-body idle (`body_idle`, 0x227638) and SetState 0 in a body
- **Claim:** `SetState(0)` in a body runs group 0, 0x1415d4 = 0, the look clear, then body 1 → 0x43, 2 → 0x5a,
  3 → 0x53 (body 0 is `SetState(0, 1)`); `idle_seq` is 0 in a body.
- **Suggested method:** unit test on `Hero::set_state(c, 0, …)` with mode 1 / 2.

### B4. Clank's ground states (0x43 idle, 0x44 / 0x50 walk / run, 0x47 death, 0x52 pit)
- **Claim:** 0x43: group 0, anim 0 over 9; pit (0x14063a with 0x13f65c = 0) → 0x52; wrap: 50 % fidget 1 (from 0) or
  back to 0; L1 / L2 → 1; air → 0x45; ✕ (8) → 0x49; □ (9) → 0x51; stick > 0.2 → 0x44; the command flash (> 42 of 57
  left) → anim 0x12. 0x44 / 0x50: speed = |eff.xy|, anim 2 over 8; turn constants (0.035 / 0.008 × (stick + 0.35),
  0.15, 550° / 570°·(stick + 0.35)/s), speed step 7.5·(1 − 0.3·residual)·dt² / 8.5·dt²; walk ↔ run (seqs 2 / 3, frame
  carried, below 1.5·dt / above 1.8·dt), playback |eff|·90 / ·20 (0x50 ·196); air ≥ 5 ticks above 0.8 or a slope →
  0x45; stick < 0.17 → 0x43. 0x47: group 0x14, health 0, anim 5 over 12, wrap → death fade. 0x52: group 2, 5.5·dt down
  the slope, air accel 25·dt², gravity 18·dt²; after 300 → death fade; off the pit → 0x45 / 0x43.
- **Setup:** RC_LEVEL=10 as Clank (B13).
- **Expect:** state sequence and speeds as above; the walk / run sequences of class 0x57.
- **Suggested method:** unit tests on `clank::physics` / `transitions` with synthetic input; QA in game for feel.

### B5. Clank's air states (0x45 fall, 0x46 hurt, 0x49 jump, 0x4f glide)
- **Claim:** 0x45: group 2, vel = eff, anim 4 over 8; stick × 3·dt, air control unless 0x13f514, gravity 12·dt²,
  ≥ −10·dt; L06 below z 115 → death fade; landing clamps vz (−9·dt) → 0x47 (no health) / 0x44 / 0x43; ledge → 0x4a;
  ✕ (5) above 0.6 → 0x4f. 0x46: group 7, motion ×0.5, vel clamped to 5·dt, `HeroTakeDamage(1)`, flash 45,
  invulnerable 50, anim 6 on curve −3; |vel.xy| ×0.92 (×0.99 in the air), ground −0.004 / air eff −24·dt²
  (0xbc23d700 damping); after 32 in the air above 1 → 0x45; after 35 → 0x43 / 0x47. 0x49: h 1.3..1.35 (ramp 14,
  takeoff 5), frames 18 / 27 / 27, bottom 0.6, gravity 25·dt², air speed 2·dt, windup brake 50·dt², anim 7 over 5
  from frame 5; apex playback 0.5, descent playback (60-tick ETA, 0.2..2.7), landing → 0x43 after 12, → 0x45 above 2
  after 40, □ near the ground → 0x51, ledge → 0x4a, ✕ (5) above 0.55 → 0x4f. 0x4f: group 5, |eff| capped 4.5·dt, anim
  0xe over 12; stick × 2·dt, turn (0.04, 0.2, 500°/s), speed step 15 / 7·dt², vz = −1.44·dt; after 30 without ✕
  held → 0x45; ledge → 0x4a; landed → 0x44 / 0x43 (0x13f524 for an early landing). Rotor sequence 1 (blend 10) in
  0x4f, else 0 (blend 17).
- **Edge cases:** the jump lockout 0x13f542 undoes the jump; the run branch's 0.82 compare never fires.
- **Suggested method:** unit tests on the state functions; QA in game (glide feel, rotor).

### B6. Clank's ledges (0x4a grab, 0x4b hang, 0x4c climb, 0x4d / 0x4e shimmy)
- **Claim:** probe B / C with Clank's dims (`clank::LEDGE`: top 0.725, low 0.7, step 0.25, min above 0.7, wall from
  −0.4, hang −0.71, out 0.32); probe B is "on" in mode 1 for group 4 descending or vz < 2.5·dt, group 2, states
  0x4d..0x4f. 0x4a: group 3, 0x13f83c = 12·dt², anim 8 over 7 + 2 from frame 18; wrap → 0x4b. 0x4a / 0x4b physics:
  face the wall (0.02, 0.2, 270°/s), pull ≤ 1.7·dt, z at 3·dt. 0x4b: probe C lost → 0x45 (lockout 10); back + ✕ /
  R1 / R2 → 0x45 (10); ✕ → 0x4c; stick aside + probe C there → 0x4d / 0x4e. 0x4c: h 1.55..1.59, ramp 1, takeoff 13,
  frames 10 / 18 / 18, bottom 0.85, gravity 28·dt²; forward 1.25·dt over key 5..15, 0 by 18. 0x4d / 0x4e: vel 0,
  playback 1.3; probes here and 0.3 aside, both within 30°: turn to the mean, move to the midpoint at table
  0x1c3cf0[key]·0.5 (`SHIMMY_SPEED`); released / turned / end + wrap → 0x4b; back + ✕ → 0x45 (lockout 40); ✕ → 0x4c.
- **Edge cases:** Ratchet's ledge dims unchanged (`RATCHET_LEDGE`) — regression risk in `ledge.rs`.
- **Suggested method:** unit tests on `ledge::dims` and the probes with synthetic collision; QA on Orxon ledges.

### B7. Clank's kick (0x51)
- **Claim:** group-6 prologue; kick rows 7 / 8 (`KICK_ROWS`), the second chains from the first within its window; aim
  assist (5, 45°, 45°); anim 0xf / 0x10 over 5 from frame 1; speed 3.5·dt before key 5.5; in the hit window a sphere
  0.25 at the mean of joint lists 2 / 3 (0 / 1 for the second kick), damage 1, push 1, flags 0x10000; playback → 1
  (0.2 a tick); speed step 37 / 28·dt²; transitions: pit → 0x52, past the idle frame → 0x43, □ in the window → 0x51,
  ✕ after the jump frame → 0x49.
- **Expect:** creatures in reach take 1 damage (`BodyHit::Sphere` delivered by `after_update` through
  `creature::attack::sphere_hit`); units reading `hero.state == 0x51` see it (Orxon brawlers 1202).
- **Suggested method:** level-harness test on L10 with a brawler near Clank.

### B8. Clank's burn (0x7d) and surface reaction
- **Claim:** body 1 grounded on surface 1 → 0x7d: group 0x14, 0x13f51c = 10000, burn floor = ground z, health 0,
  Ratchet's moby at the hero, `PlayClassSound(9, 0, Ratchet)`, anim 0x11 over 8; vel 0, vz −0.35·dt; 0.8 under the
  floor or after 150 → death fade.
- **Suggested method:** unit test.

### B9. Clank's HeroUpdateAlt part (glow, rotor, command flash, lean)
- **Claim:** record 25's scale = `scale18`; glow pulse `0x2278c0` on the body; the antenna glow 0x4b4 created (scale
  class×1.7, glow 0x801432d7) at joint list 7 (0.0257 down its rows), scale approaching 1.7 (×1.4 while flashing);
  glow phase 170°/s (700°/s while flashing) with colours by the command; the rotor 0x47a created and placed at joint
  list 5 with rows = joint × Rz(−yaw), light = the body's; a command → class sound cmd + 16, flash 57. The lean
  (records 24..26) runs in 0x44, 0x4f, groups 2 / 4.
- **Suggested method:** level-harness test (moby counts / positions of 0x4b4 / 0x47a after the switch); visual QA.

### B10. Giant Clank's movement (0x5a idle, 0x5b walk, 0x5c fall, 0x62 death)
- **Claim:** 0x5a: anim 0 over 15; edge brake (3.7, 0), speed → 0 at 12.6·dt²; wrap after 50 ticks on 0: 10 % fidget
  1 / 40 % fidget 2; L1 / L2 → 1; ✕ → 0x5e; □ (15) → 0x60; △ with no lockout → 0x61; ○ → 0x5f; air above 2 → 0x5c;
  stick > 0.22 → 0x5b. 0x5b: anim 3 over 8 from 18; three spheres r 2.7, damage 40, push 1, flags 0x30000 at 1.8
  ahead / 1.4 up at −45°, 0°, +45°; footstep (key 30 or 0 passed): camera shake 0.1 for 20 ticks; turn (0.021 / 0.3 /
  130°/s × (stick + 0.35)); speed step 7.5·(1 − 0.3·r) / 8.5·dt²; records 27..29 leaning; playback |eff|·8 (≥ 0.75);
  air 5 ticks above 1.5 → 0x5c; stopped 20 → 0x5a. 0x5c: anim 5 over 8 above 2.5 else 0 over 11; sphere 3.5 at the
  feet −1 (damage 40 in physics; 1 in the descending transition); landing → 0x62 (no energy) / 0x5b / 0x5a. 0x62:
  group 0x14, anim 7 over 12, wrap → death fade.
- **Expect:** crates / creatures under or ahead of him smashed; camera shakes on footsteps.
- **Suggested method:** level-harness test on L15 (crates near the pad); unit tests on the state functions.

### B11. Giant Clank's jump (0x5e) and the shockwave (class 0x593)
- **Claim:** h 4.2..4.3 (ramp 15) after 19, frames 17 / 22 / 40, bottom 0.6, gravity 30·dt² (50 descending), turn
  250°/s, air speed 4·dt, anim factor 2, anim 4 over 8; descending: sphere (3.5, 40); landing: camera shake 0.15 for
  30; first landed tick: `giant_shockwave::spawn(2, 15·dt, 40, (x, y, ground + 0.2), ticks(25))`; → 0x5a after 12.
  The shockwave: ambient (0x80, 0x80, 0x70), scale 2·2·class scale, each tick f = scale/cs·0.5 + growth, scale =
  2f·cs; life < ticks(40): alpha = life·0x7f / ticks(40); a sphere hit (radius f, damage 40, push 1, attacker the
  hero moby, flags 0x30000); deleted when the life runs out.
- **Edge cases:** ticks(25) < ticks(40), so the alpha fades from the start; it never hits Giant Clank himself (the
  attacker is the hero moby).
- **Suggested method:** unit test on `giant_shockwave::update` (scale series, alpha, life); level-harness on L15.

### B12. Giant Clank's attacks (0x5f missiles, 0x60 punches, 0x61 beam) and hurt (0x5d)
- **Claim:** 0x5f: anim 8 over 8; turn (0.01, 0.2, 70°/s); key 4 → a `Spawn::Missile` from joint lists 3 / 4
  (dropped: G-HERO-036); ○ held keeps firing; released + wrap → 0x5a. 0x60: the punch row (next of three when chained
  within 10 ticks), anim 9 + row over 7 (13 chained) from frame 1; turn (0.02, 0.2, 110°/s); fists `BodyHit::Punch`
  r 2, damage 4, flags 0xb0000 with an exact push, plus the 2.2 sweep (40); lunge rows 0 / 1: 21·dt over key 7..15
  (step 180 / 140), row 2: 30·dt over 11..20 (150 / 120); □ (17) after frame 21 (25.5 on seq 0xb) and 20 ticks →
  0x60; past 28.5 or wrap → 0x5a. 0x61: anim 0xc over 8; up to key 22 the beam point (3 ahead, 5 up), playback 0.5;
  22..55 playback 1; wrap → lockout `beam_lock` = ticks(301), → 0x5a (the lockout counts down in `post_move`).
  0x5d: group 7, motion ×0.5, vel ≤ 7·dt, invulnerable 77, anim 6 on curve −3; |vel.xy| ×0.92 / ×0.99; after 32 in
  the air above 1.5 → 0x5c; after 40 / wrap → 0x5a / 0x62.
- **Suggested method:** unit tests on `giant::physics` / `transitions` / `punch`.

### B13. Giant Clank's hit intake (body 2, 0x231580)
- **Claim:** only hits with flag bit 2 are taken; energy −= (int)damage, ≥ 0; flag 4 or energy < 1 → 0x5d, the beam
  ended (`beam_end`), on the ground the knockback (7·dt, 3.5·dt). Ratchet's health untouched.
- **Suggested method:** unit test on `damage::hit_intake` with mode 2.

### B14. Body-dependent sizes and points
- **Claim:** wall check lines Clank 0.35 up (×0.3), 0.27 long; Giant Clank 1.5, 5.0; body point 0x13f420 0.4 up
  (Clank) / 4.0 (Giant Clank); bolt radii 2.125 / 1.25 (Clank), 15 / 3 (Giant Clank); Ratchet's unchanged.
- **Suggested method:** unit tests on `wall_check`, `post_move`, `World::bolt_radii`.

### B15. HUD and menus in a body
- **Claim:** body 1: the health orbs show Clank's max / health (0x1415fc); body 2: no health drawn; the quick-select
  ring does not open in body 1 (△ is Clank's command menu, not ported) — it opens on foot or in body 3.
- **Suggested method:** unit test on `hud::draw_health` with `Inputs::body`; QA in game.

### B16. Orxon's Clank section (class 22, L10, 1 instance at (226, 194, 49))
- **Claim:** every tick: update distance 0xff; the two class-805 checkpoints toggled by the mission (+8 bit 8);
  first visit: death bits set; no O2 Mask: Clank (pvar +0x00, the 0x57 at (220.7, 196.5, 49)) gets collision, the hero
  teleported to cuboid +0x08 (mission not done and +0x14 ≠ −1) else +0x14, `SwitchCharacter(1, 0x43, clank)`,
  music (2, 4). With the mask: as Clank → leave, teleport to cuboid +0x04, music (0, 5), Clank hidden / frozen / no
  collision; then airless = 1 and the controller deletes itself. Already visited without the mask: hint record 32
  bumped and prompt (1, 0x53f1) unless game mode ≠ 0 or shown ≥ ticks(300) times; in Clank it returns.
- **Edge cases:** both mask states; second visit; pvar −1 guards.
- **Suggested method:** level-harness test on L10 (with / without item 6).

### B17. Blarg's Clank station (class 1061, L06, 1 instance at (218.31, 158.49, 130.41))
- **Claim:** see the coverage table in `blarg_clank_lift.rs`: states 0..8, the pad cuboid prompts (9, 0x1781 / 0x177e
  / 0x1789 / 0x1785), △ → jump lockout 60, timer 120, `FadeToBlack(16)`, then leave (as Clank) / switch to Clank
  (no mask) / Ratchet through the airlock (mask); the three field mobys told 0x2f53e8 / 0x2f54a0 / 0x2f5360; the ride
  spring (1·dt², 1·dt², 2·dt) between floor and floor + height; the airless flag = +0x94; the glow tween.
- **Suggested method:** level-harness test on L06; QA in game.

### B18. Giant Clank's pads (1451 L15 at (130.51, 222.42, 25.43); 1899 L18 at (604.09, 727.49, 96.02))
- **Claim:** state 0: Giant Clank (pvar +0x60) hidden, not drawn, no collision; L18 with the mission done → 3, else 1;
  1 → 2; in 2, Ratchet on the pad, grounded, game mode ≠ 2, state ≠ 0x1d: prompt (4, 0x3aa4) on foot / (4, 0x3aa5) as
  Giant Clank off L18; △ with the prompt owner 4: as Giant Clank on L15 → his moby hidden, leave, his anim cut, scene 9,
  music (0, 5); on foot → Giant Clank shown with collision, `SwitchCharacter(2, 0x5a, giant)`, L15 scene 8 + entry
  pose saved + music (2, 4); L18 scene 6 + pose saved, → 3. Otherwise +0x68 = −1.
- **Suggested method:** level-harness test on L15 (switch in and out).

### B19. Checkpoint and respawn in a body
- **Claim:** a checkpoint records `(hero.mode, bodies.state_param)`; respawn switches back into the body with the first
  moby of class 0x57 (body 1) / 0x1a3 (body 2) (`restore_from_checkpoint`), the body anim binding reset first.
- **Suggested method:** level-harness test on L10: switch, hit a checkpoint, die, check mode and state after respawn.

### B20. Airless flag (0x14161b)
- **Claim:** `HeroFields::airless` written by classes 22 / 1061 lands in `Worn::airless`; with it set and the O2 Mask
  owned the head item goes on (`worn.rs`).
- **Suggested method:** unit test on the worn rule.

## 4. Shared code touched (regression risk)

- `rc-game/src/tick.rs`: the hero moby is now `hero.hero_moby(ratchet)` for the hit message, carriers, env, update,
  hit slot, anim copy and matrix build; the pending checkpoint restore runs before the hero calls; `apply_cmds` after
  them; `bodies::after_update` after the hero update; the items update runs only on foot. On foot all of these resolve
  to Ratchet: no change intended — every existing hero / weapon / platform test should be unchanged.
- `rc-game/src/hero/anim.rs`: `RatchetAnimCtl` refactored onto `RatchetAnimRef`; new `HeroAnimCtl` wraps it. Risk:
  Ratchet's anim evaluation and the trace compares (`rc-trace`, `tools/trace/src/port_sim.rs` passes
  `body_classes: None`).
- `rc-game/src/hero/ledge.rs` (probes parametrized by `LedgeDims`; Ratchet's values in `RATCHET_LEDGE`),
  `physics.rs` (`size_capsule` / `wall_check` / `post_move` body branches), `ground.rs` (SetState 0), `common.rs`
  (`idle_seq`), `jump.rs`, `damage.rs` (hit intake mask), `fx.rs` (`voice_queue` split out of `flush`), `idle.rs`
  (`clank_glow_word` extracted from the attachment blink), `worn.rs`, `registry.rs` (routing + its tests: the
  implemented set now includes 0x43..0x52, 0x5a..0x62, 0x7d; the stubs test allows `Module::Bodies`).
- `rc-game/src/moby_update/services.rs`: `HeroCall::{SwitchCharacter, LeaveBody, BodyIdle}`, `HeroFields::{airless,
  save_entry_pose}`, `SaveBits::checkpoint_body`, `Services::level_words`, `bolt_radii` per body.
- `rc-game/src/moby_update/classes/checkpoint.rs` (records the body), `units/mod.rs` (4 new units).
- `rc-game/src/audio/class_sounds.rs` (`HeroClassSounds::body_classes`: anim sounds by the moby's class).
- `rc-game/src/hud.rs` (`Inputs::body` / `clank_max`), rc-engine `hud_render.rs`, `menu_render.rs` (ring gate reads
  `hero.mode`), `shadow_render.rs` (shadow on the hero moby), `moby_attach.rs` (attachments hidden in a body),
  `gameplay.rs` (body joint lists, body anim binding, respawn restore, body classes driven).

## 5. Not ported (don't test as working)

- The Hologuise body 3 (states 0x53..0x59, its tint / 18-tick timer / squash table, the disguise leave in SetState's
  prologue): G-WPN-006.
- The item slots' pass at the switch `0x231088`: G-HERO-035.
- Giant Clank's missile class 0x100 and beam class 0x5f3 (their spawns are dropped): G-HERO-036.
- Clank's command menu (`0x238b18` / `0x238b80` / `0x238f88`) and the gadgetbots 857 that obey it: G-UI-018,
  G-CLS-001.
- Giant Clank's energy bar HUD (`0x24f9c0` / draw `0x24f248`): G-UI-011.
- The hero draw callback `0x229440` points in bodies 1 / 2 (joint lists 7 / 6 → 0x1410d0): G-REN-005.
- The cheat 0x15edb3 scales (glow 3.1, record 28 ×1.4): G-SAV-006.
- L10 class 22's gp−0x6878 hint reset: G-UI-017.
- The leave classes 1446 (L15 `0x2ec760`) / 1422 (L18 boss), the section starters on levels 0, 4, 7, 9, 13, 17 and the
  remaining body-branch units: G-HERO-005 / G-CLS-001. Camera class 22: G-HERO-027 (camera lane).

## 6. In-game QA spots (for the user)

- **Orxon as Clank:** `RC_LEVEL=10` (no O2 Mask): the level should hand you Clank right away (music change). Walk,
  run, jump (✕), glide (✕ held in the air: the rotor spins), kick (□, twice for the combo), hang and shimmy on ledges,
  climb up (✕). Listen for Clank's footsteps / kick sounds; the antenna glow pulses.
- **Orxon with the mask:** `RC_LEVEL=10 RC_GIVE_ITEMS=6`: Ratchet stays himself, the O2 Mask goes on.
- **Blarg station:** `RC_LEVEL=6 RC_HERO_AT=218.3,154.5,131.5,1.57`: walk onto the lift pad, △ at the prompt: fade,
  then Clank; the pad rides the shaft; △ again as Clank gives Ratchet back.
- **Quartu Giant Clank:** `RC_LEVEL=15 RC_HERO_AT=130.5,219.5,27,1.57`: step onto the pad, △: Giant Clank (scene 8,
  music). Walk (ground shakes, crates smashed), punch (□ ×3), jump (✕: the landing shockwave ring grows and fades),
  ○ / △ do their animations (no missiles / beam yet). Step back onto the pad and △ to climb out.
- **Veldin 2 Giant Clank:** `RC_LEVEL=18 RC_HERO_AT=604.1,724.5,97.5,1.57` (pad 1899).

## 7. Results

(The test expert fills this in.)
