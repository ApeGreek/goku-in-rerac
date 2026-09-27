# Cutscenes: the kinds, how they are driven, and what the port plays (RAC1 NTSC)

Addresses are **level01.elf** unless marked. Confidence: **[H]** read in the code and checked on disc data or in a
headless / engine run, **[M]** read in the code only, **[L]** inference. The detailed format and timing of the
mode-2 scenes and of the level transitions stay in `cutscenes_transitions.md`; this page answers "what kinds of
cutscene are there, and are they one system" and documents the in-level cinematic port (2026-09-27).

## 1. Answer in brief

The game has **three distinct kinds** of cutscene, and **no script system**. Two of them share nothing but the
call that starts them.

| kind | examples | started by | data | game mode | camera | player | skip |
|---|---|---|---|---|---|---|---|
| **A. Scene** (in-engine, pre-authored) | Novalis arrival (scene 5), the Plumber (scenes 0 / 1), the Novalis mission's scenes 3 / 4 | `DialogStreamStart(k)` 0x2ac330 from a class (mission NPC 730/790, talking NPC 774, infobot 750, ship, menu replay) | level header scene record k: 30 Hz animation chunks with a per-tick **camera track**, **actor animation + position tracks**, subtitles, and a speech VAG per language | **2** (the gameplay tick stops; `CutsceneModeUpdate` 0x2aca80 runs instead) | the chunk's camera record, 60 Hz | state 100, hidden; `SetState(0,1)` at the end | Start + L1 L2 R1 R2 after 18 ticks (Start alone once the game is beaten / from the replay menu / level 0) |
| **B. Cinematic cutaway** (live gameplay, scripted by one class) | door / bridge cutaways, the gunship strafe over the bridge, the troop drop-in, the gold bolt, the bolt crank's door view, the vendor | a moby class's own update, calling the camera and hero functions directly | **none of its own**: the class's pvars, cuboids (camera placement = a cuboid's centre + Euler angles), timers | **0** (the world keeps running: enemies, shells, doors move) | the **script camera** (camera type 5): `CameraScript` / `CameraScript2` | hero state **0x72** (held, no control), letterbox bars, HUD hidden (global `0x15f404`) | **none** |
| **C. Movie** (FMV, PSS) | new-game opening, Veldin → Novalis flagship scenes, in-level holofilms (the Novalis mission's movie 3, infobot movies), extras, title attract | in level: `DialogStreamUpdate(n)` 0x2acf50 → `StartPssMovie` 0x2ad0c0; between levels: `fun_00231608` from `DoSpaceTransition` 0x2a68f8; title: boot 0x1e9488 | `mpegs[88]` in the TOC: MPEG-2 program stream, 512×416 30 fps video + SShd SPU-ADPCM 48 kHz stereo | **1** (`MovieModeUpdate` 0x2ad498 blocks the game loop) | – | – | the same Start / Start+shoulders rule (inside `readMpeg` 0x31a308); any button in replay mode 2 |

**Your examples:**
* *Ships swoop in and shoot holes in the bridge* → kind **B**: the gunship 688 instance 695's own scripted state
  (`GunshipUpdate` 0x2f7728), with the script camera on cuboid 23 (§3.3). Not a scene file.
* *Reaching the Plumber* → kind **A**: scenes 0 (2318 ticks) and 1 of Novalis, started by the talking NPC 774's dialogue
  (`FUN_0027b028` / `0x27b550`, node scene id); the same player as the arrival scene.
* *The random door-opening cutscenes* → kind **B**: camera trigger 737 commanded by another class (on Novalis the
  mission NPC 790 lowers the hinged bridge 746 and shows it through trigger 863 for 3 s, §4).
* *Connected to the Novalis intro?* Only kind A is: the arrival is scene 5, played by exactly the same code as the
  Plumber's. Kinds B and C do not use the scene files at all.
* *Movies like the first Drek holofilm* → kind **C**, special in every way: a separate game mode, a separate file
  format (PSS on the disc root table, not in the level), no in-engine rendering. In-level holofilms are played
  between two scenes by the class that owns the story beat: the infobot 750 (`InfobotUpdate` 0x2fbf80 states 8–10:
  scene pvar+0xc → movie pvar+0xd → scene pvar+0xe) and the Novalis mission NPC (scene 3 → movie 3 → scene 4).

**Shared code (clusters.tsv):** the scene player (`DialogStreamStart` / `CutsceneModeUpdate` / end `0x2ac608`)
and the movie player are engine code in all 19 overlays; the script camera (`CameraScript` cbb1cc1f, update
4b806e22, `CameraScript2` d0b32970) is in 19 / 19 / 16 overlays; the camera trigger 737 (5c26590d) is on levels 1, 2,
3, 7, 10, 13; the gunship (7751f2bf) and the mission NPC (2c0aee80) exist only on Novalis.

## 2. Kind A: scenes (mode 2) — the arrival scene's system

Unchanged from `cutscenes_transitions.md` §1–§4 (format, player, audio, skip) and already ported
(`rc_formats::scene`, `rc_game::scene_player`, `rc-engine/src/scene_render.rs`, `rc_game::audio::scene`). What
changed now is **who starts them**: the engine no longer hard-codes the Novalis trigger; it plays the scenes the
classes ask for (`rc_game::cinematic::EngineRequest::StartScene`, and the talkers' `interact::Handoff::Scene`) on
the frame after the tick that asked, which for the arrival is the same frame as before (app frame 3; scene tick 1 on
frame 14; control back on frame 1532) [H: engine log]. Scenes asked for while one runs are queued.

End of a scene (`0x2ac608`), now applied by the engine: Ratchet `SetState(0, 1)` (queued on the hero-block channel for
the next tick), and for a talker's scene the teleport in front of it (`Interact::scene_end_place`, 0x16cd26) and its
dialogue refresh (`Interact::scene_ended`, `FUN_0027b550(npc, rec, 1)`).

Novalis scenes: 0 the Plumber / Water Pump Worker 774 (auto node, 2318 ticks), then after buying the Infobot scene 1
→ movie 2 (`mpegs[4]`) → scene 2 (his talk tree, `interaction.md`); 3 the mission's hand-over (811 / 750 / 731, 1078), 4
the Qwark-ad reaction with ship 531 (1536), 5 the arrival (1508), 6 = scene 0 without audio.

**`RC_SCENE`** only concerns the arrival: `RC_SCENE=0` drops the arrival request, `RC_SCENE=<k>` plays scene k after
gameplay tick 1 instead; every other hand-off (talkers, the mission's scenes 3 / 4) always plays, and a scene that
cannot be loaded is reported as ended (skipped) to its talker. Checked: `RC_SCENE=0` in front of the Water Pump
Worker plays scene 0 from frame 3 (2342 frames), puts Ratchet in front of him and his dialogue advances to
"△ Buy Infobot for 500 bolts" (frame 2450).

## 3. Kind B: in-level cinematics — the script camera and its callers

### 3.1 The engine functions every caller uses [H]
| function | effect |
|---|---|
| `CameraScript(pos, euler, mode, ticks, collide)` 0x316ef8 | the type-5 camera switched in with `FUN_0020d110` (a cut) and its init 0x3171b8: position spring k 0.3 / damping 0.3 / max 1 per axis, Euler spring k 0.05 / 0.3 / 1; camera and targets = (pos, euler); mode +0x88; timer D+0xe0/e4 (modes 2, 3). Mode 3 instead keeps the current view and records its (yaw, elevation, distance) about Ratchet |
| update 0x317670 → 0x317278 | mode 0 spring toward the targets; 1 snap; 2 cosine-eased move applied to the current pose each tick; 3 timed swing about Ratchet (anchor eased to him, yaw eased — the long way round when the short way and the target view disagree by > 20° —, elevation eased, distance on the Hermite curve `0x26cc00(D+0x124, 0, 1, D+0x128, t)`, Euler eased), then hold the targets. Rows = `EulerToMatrix(euler)`. `collide` → sphere push `0x20f2a8` r 0.5 (no Novalis caller) |
| `0x316dd0` / `0x316e28` / `0x316e88(a, b)` | target position / target Euler / mode 3's curve slopes |
| `CameraScript2(kind)` 0x317070 | release: 0 = the script camera jumps 1 behind / 1.2 above Ratchet, then the follow camera is switched in **cut** (`+0x7e = 1` → `+0x8e = 1`) and snaps behind him; 2 = **blend** (0x167370, rates 0.018); 4 = blend from the copied pose (0.018, 0.01 on level 1); 1 = pose copy, no blend [1 / 4: M] |
| `0x15f404` | 1 = letterbox + HUD off. `DrawScreenFade` 0x21b7d8 (in the HUD layer, after `HudDraw` and `DrawHud2D_B`): bar height 0x15f408 +1 px per frame to 24 while set, −1 per frame after; two opaque black bars (PRIM 0x104 strip, RGBA 0x80000000) at the top and bottom of the frame. `HudDraw` 0x24fb50 skips every slot and the banners while set. Also read by the gunship shells (`0x2f6a30`: fewer particles, shells hit mobys) |
| `SetState(0x72, …)` / `HeroTeleport(pos, euler, 0x72, 1)` 0x2368e0 | the hold: group 9, 0x1413fc = 1 (no control), idle anim over 10 ticks, idle physics, no transitions. HeroTeleport also clears the motion block and resets the follow camera behind him |

The camera types (table 0x20c480 {type, activation, init, update, release}): 0 follow, 3 (0x315358), 4 first person,
**5 script**, 6, 7 (region cameras), 0x11. Type 5's activation 0x317668 returns 0: only `CameraScript` brings it up.

### 3.2 Camera trigger 737 (`CameraTriggerUpdate` 0x2fb5b0) [H] — the generic cutaway
Pvars: P[0..7] target pos / Euler, P[8] trigger cuboid, P[9] camera cuboid, P[10] Ratchet's cuboid (−1: stays), P[11]
hold ticks, P[12] 0 = cut to P[9] / else move there over P[13] ticks (mode 2). Fires on Ratchet's feet in P[8] **or
moby+0xbc = 1** (another class's command, usually with P[11] set); holds while the timer runs and +0xbc = 1
(re-targeting the camera from P[0..7] each tick, so a commander can aim it); releases with `CameraScript2(0 / 2)`,
`SetState(0, 1)` (`FUN_002405a0`; 0x53 in hero mode 3), `0x15f404 = 0`; then only a new command re-arms it. Spent
when its spawn id is collected or its death bit set. Novalis: 860 (cuboid 42 → camera 43, Ratchet to 42), 863
(camera 51), 859 / 861 / 862 (no cuboid, no commander on the disc: never fire).

### 3.3 Gunship 688 fly-by (`GunshipUpdate` 0x2f7728) [H] — the bridge strafe
Instance 695 (P+0x170 = cuboid 22 at the bridge's west end, P+0x174 = camera cuboid 23). On Ratchet's feet in cuboid
22 (and not seen before: spawn-id / death bit): shown, `SetState(0x72, 0)`, `CameraScript(cuboid 23, 0, 0, 0)`,
`0x15f404 = 1`, timer ticks(510); the update body re-runs within the same tick until the timer is at 390 (the ship
jumps 120 ticks ahead on its path), then flies and fires at the target cuboids 19 / 20 (the bridge); with 60 ticks left
it wakes the troopers; at 0: `CameraScript2(0)`, `SetState(0, 1)`, flag 0, killed + death bits, deleted. **The strafe
is the gunship's own scripted state**: the enemies agent owns the class (`classes/gunship.rs`); it queues
`creature::ScriptRequest::{Start, End}` and sets `creature::Globals::cutscene` (= 0x15f404). The cinematic layer turns
the requests into exactly the calls above (`cinematic::from_creature`) and keeps them in `Cinematic::creature_log`
(the last 64, for reports and tests: `creatures_enemies_novalis::novalis_gunship_bridge_fly_by` checks one Start
{0x72, cuboid 23} then one End by the gunship). What the camera side expects of the gunship:
Start once on the trigger tick with the cuboid 23 centre / Euler and hero state 0x72; End once when the timer runs
out; the flag set / cleared with them. Instance 694 (P+0x170 = −1) is an ambient gunship with no cutscene.

### 3.4 Other in-level cinematics (not on the Novalis critical path)
* **Gold bolt 1134** `GoldBoltUpdate` 0x307ca0: pickup cutaway, `CameraScript(orbit point, 1, …)`, the targets
  re-set each tick, `HeroTeleport` + `CameraScript2(0)` at the end, flag 0x15f404 [M]. Not ported (no owner yet).
* **Bolt crank 280** 0x2e0c68: `CameraScript(…, 3, ticks(180))` + `0x316e88`: the swing to the door / rotator it
  drives, with `0x316e88(pvar+0x28, +0x2c)` (`cinematic::camera_curve`); `CameraScript2(4)` on leaving. The class is
  the crank agent's; it calls `rc_game::cinematic`. Engine check (the crank agent's `engine_script_stop180.txt`,
  frames 45 / 80 / 140 / 215): the camera swings from behind Ratchet round to the door over the 180 ticks, then
  blends back.
* **Path enemy 459** P+0x1c4 camera cuboid (no Novalis trooper has one), **vendor 11** (`OpenVendorMenu`), 
  **collapsing platform 701** state 4 (`CameraScript2(0)` + `SetState(0,1)` when P+0x8c ≠ −1: a leftover with no
  starting call; instance 711 has P+0x8c = 24, so the game resets Ratchet to idle once its debris timer runs out
  after the fall [M]; the class belongs to the world agent).

## 4. The Novalis mission (classes 730 / 790, `MissionNpcUpdate` 0x2fad68) [H]
Class 790 (instance 905 at (189.2, 203.8, 40.6), mission 0) is the **Blarg troop dropship** of the first Novalis
mission (the catalogue's "mission NPC" name is Lombyte's); 730 (instance 858, pvar +0x04 = −1) runs the same code
without a drop-in camera. Sequence (§ table in `mission_npc.rs`):
1. Load pass: hidden 30 units up. **Tick 1**: mission open, mode 0, flag 0x13d397 clear → `DialogStreamStart(5)`
   (the arrival scene), flag set. Every tick until triggered: the ship on the pad hidden (`FUN_002a2450`).
2. Ratchet in cuboid 42 → trigger 860 fires (Ratchet to cuboid 42's centre, 0x72, camera at cuboid 43) and the ship
   commands it for ticks(420), aiming cuboid 43's Euler at itself; it drops at 5 u/s (≈ 6 s) while 860 re-aims the
   camera every tick (spring mode 0: the camera turns to follow it), lands (anim 1 → 2), and the troopers 325–327
   attack. After 420 ticks: control back.
3. All three troopers dead → 45 ticks → scene 3 → movie 3 (`mpegs[5]`) → scene 4 → `UnlockPlanet(3)` (Kerwan) +
   banner, the bridge trigger 863 commanded for ticks(180) and the hinged bridge halves 869 / 870 (class 746,
   `HingedBridgeUpdate` 0x2fb8a8) commanded (+0xbc = 2): they swing from 36° to 0° in 2 s about a pivot 11 below their
   origin (into the arch) while the camera shows them; `SetMissionDone(0)`, checkpoint at cuboid 55 (the landing
   pad), save, the ship shown.

## 5. Kind C: movies (FMV) — findings and options (decision U10 stays open)

**Files [H].** `mpegs[88]` (TOC 0x17f8, `{lsn, bytes}`), archived raw in Tier 0 as `global/mpegs/NNN.bin` (84 files,
2.8 GB; NTSC and PAL copies). Each is an MPEG-2 program stream (pack header `00 00 01 BA`, MPEG-2 marker), video
MPEG-2 MP@ML 512×416 at 29.97 fps, audio in private stream 1 (sub-id 0xFF) with an `SShd` header: SPU ADPCM, 48 kHz,
stereo, interleave 0x20 (`mpegs[0]` 44.1 kHz). Decoded on the PS2 by the IPU through `sceMpeg` (`readMpeg` 0x31a308)
and fed to the IOP through 989snd's movie ADPCM calls.

**When [H].** In level: `DialogStreamUpdate(n)` → `mpegs[2 + n]` (PAL 21 + n), from the NPC talk nodes (scene id
bit 0x4000), infobots (pvar byte +0xd), the Novalis mission (n = 3 → `mpegs[5]`, 37.6 MB) and the ship; transitions:
`mpegs[40..50]` (new game 40–42, Veldin → Novalis 43–45, …); extras 70–74; title attract 80–83. `StartPssMovie`
stops sounds and music, `FadeToBlack(4)`, mode 1; `MovieExitToGameplay` 0x2ad2b8: `FadeToBlack(4)`, the level music
restarts (`music_start_track(0x151708, 1, 0x400)`), mode 0, the talker's dialogue continues.

**Skip [H].** Replay mode 2: any pressed button; else Start alone when the game is beaten / completed / replaying /
on level 0; else Start + L1 L2 R1 R2 held (the scene rule).

**Options.**
| option | how | licence | build time / size | fidelity |
|---|---|---|---|---|
| 1. Transcode at extraction | `randcrw-extract` calls a user-installed `ffmpeg` once: PSS → AV1/VP9 + Opus in WebM (or MPEG-2 kept + PCM), decoded at runtime by a small player | ffmpeg stays an external tool (LGPL/GPL not linked); the runtime decoder must be one we can link | no engine build impact if the runtime decoder is light; extraction needs ffmpeg | lossy re-encode (good at high bitrate); SShd ADPCM must be decoded ourselves or by ffmpeg (it knows PSS audio) |
| 2. External ffmpeg at runtime | spawn `ffmpeg` on the PSS, read raw frames / PCM over a pipe | not linked | none | exact decode; depends on the user's ffmpeg at play time |
| 3. **Native pure-Rust decoder** | demux PSS (trivial, ours), decode MPEG-2 video (I/P/B, 4:2:0, no interlace tools needed at 512×416 progressive — to be confirmed), decode SShd SPU ADPCM with the **existing** `rc_formats::vag` ADPCM code | ours | one crate of our own, no dependency; a few thousand lines; fast incremental builds | bit-exact to the disc stream |

**Recommendation: option 3**, staged: the PSS demuxer and the SShd audio (reusing the VAG decoder) first — they are
small and make the movies audible with a still frame — then the MPEG-2 video decoder as a self-contained module in
`rc-formats` (no Bevy, testable headless, frames uploaded as a texture by the engine). It fits the project's rules
(no new dependency, native, owned file formats, launcher extracts once) and avoids a runtime or extraction dependency
on ffmpeg. Option 1 is the fallback if the decoder effort is not wanted: it keeps the engine free of MPEG code at the
cost of an ffmpeg step at extraction. Needs the user's decision (U10).

**The hook.** `rc-engine/src/scene_render.rs::play_movie(n)`: called for every in-level movie request; it logs the
file and returns at once, which is exactly what a skipped movie does in the game (mode 0 again, the talker refreshed,
the class moves on). A decoder plugs in there (blocking playback full-screen, the skip rule above, then return).

## 6. In the port (2026-09-27)

**rc-game**
* `follow_camera/script.rs`: the script camera (type 5) — `Camera::camera_script`, `camera_script_targets`,
  `camera_script_curve`, `camera_script2`, `script_active`; modes 0–3; release kinds 0 / 2 / 4 / 1. Hooked into
  `Camera::update` (the script camera is the active camera while up; no activation check runs). Native `f32`.
* `cinematic.rs`: the call layer for classes (`camera_script`, `camera_targets`, `camera_curve`, `camera_script2`,
  `letterbox`, `hero_state`, `hero_teleport`, `start_scene`, `start_movie`) and `EngineRequest`; the tick applies the
  camera calls right after the moby loop (`MobySystem::take_cinematic`), the hero calls go through
  `HeroFields::call` / `pose` / `clear_motion` (HeroTeleport keeps only the yaw of the Euler; every Novalis caller
  passes 0 pitch / roll).
* `hero/scripted.rs`: state 0x72 (entry, idle physics, no transitions).
* Classes: `camera_trigger.rs` (737), `mission_npc.rs` (730 / 790), `hinged_bridge.rs` (746).
**rc-engine** (`scene_render.rs`): scene / movie requests from the classes and the talkers, the scene end's
`SetState(0, 1)` / teleport / dialogue refresh, `SetMissionDone`, the ship hide / show, the movie stub, the letterbox
bars and the HUD off while `0x15f404` (`creature::Globals::cutscene`). `RC_DEBUG_KILL=t` (dev check): the mission's
three troopers deleted before tick t. With `RC_SCENE=0 RC_HERO_AT=159.5,208.44,40 RC_DEBUG_KILL=500` the whole mission
runs in the engine: drop-in cutaway from the first ticks for 420 ticks, scene 3 from frame 548, the movie stub, scene 4 from 1652, the bridge
cutaway ≈ 3215–3395 (frame 3290: the halves mid-swing under the bars).

**Verified (headless, `crates/rc-game/tests/cutscene_novalis.rs`, Novalis from `extracted/`):**
* the arrival scene is requested once, in tick counter 1, with the ship hidden;
* drop-in: trigger on the tick Ratchet is in cuboid 42; Ratchet at the cuboid centre, state 0x72, **420 ticks** held
  with the stick pushed (he does not move), letterbox flag, camera exactly at cuboid 43 and turning down after the
  dropship (forward z 0.61 → −0.07); released: state 0 (then walking), flag off, trigger state 3;
* after the kills: scene 3 → movie 3 → scene 4 → mission done; bridge cutaway 863 for **180 ticks**, camera at cuboid
  51, the bridge flat after 120 ticks, Ratchet held throughout, then released;
* gunship: cuboid 22 → script camera at cuboid 23 for **390 ticks**, 0x72 and letterbox throughout, then released;
* two runs identical tick for tick; the `novalis_hero_digest` guard byte-identical.
**Engine** (frame-exact, captures in the agent scratchpad `cutscenes/`): arrival run unchanged (frames 3 / 14 / 194 /
1532), music held through the scene and resumed at 1564, speech present (WAV correlation ≥ 0.99 per second with the
previous scene capture); drop-in / bridge / gunship frames show the bars, no HUD, the camera placements; two runs give
identical PNGs and WAVs.

**Not yet:** kind C decoding (U10); gold bolt 1134's pickup cutaway; mode-2 scenes of other talkers (the interaction
agent's `Handoff::Scene` is consumed here, untested in the engine for want of a talker scene on the path);
`UnlockPlanet` / banner / memcard save (logged); the collapsing platform's stray release; the script camera's collision
push; `CameraScript2` kinds 1 / 4 pose copy details; the FX driver 1546 of scene 5.
