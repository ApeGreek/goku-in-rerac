# Hero feel pass (trace-driven)

We record Ratchet in the real game (PCSX2), replay the same pad input in the port, and diff the two tick by tick.
Ghidra (the logic) and PCSX2 (the behaviour) are the truth; an impression ("the original's Heli-Pack long jumps have
more weight") is only a lead. The first question is the **Heli-Pack long jump** (state 0xa) and chaining them:

- per jump: takeoff speed, apex height and tick, airtime, the horizontal speed curve, gravity per tick, and how
  fast Ratchet slows down on landing;
- landing to the next jump: the landing state and anim, momentum ×0.8, the timer 0x13f524, the crouch re-press;
- the camera following the jump.

Tools: `crates/rc-trace` (`record`, `replay-hero`, `hero-jumps`, `hero-snap`). Code: `hero_trace.rs` (format,
reading a sample from EE memory), `hero_record.rs` (PINE recorder), `hero_replay.rs` (port run), `hero_analysis.rs`
(jumps, diff, report).

## 1. One-time PCSX2 setup

PCSX2 → **Settings → Advanced → Enable PINE** (tick it), **PINE slot 28011** (the default). Takes effect at once
(otherwise restart PCSX2). Check it from a terminal in the repo:

```sh
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
cargo run -q -p rc-trace -- pine-info
```

It prints the PCSX2 version, `status running` and `SCUS-97199 | Ratchet & Clank`. (Without PINE it fails with
"No such file or directory" on the socket `$TMPDIR/pcsx2.sock`.)

## 2. Procedure (the user)

Recordings go to `~/PS2/ratchet1/traces/` (outside the repo, never committed). Stand somewhere open and flat
(Kerwan's big plaza, or Novalis' flat ground by the ship), with the Heli-Pack on Clank. Don't fast-forward
(no Tab / turbo): the game must run at normal speed.

1. **Savestate first**, so a take can be redone: press **F1** in PCSX2 (or pass `--savestate 5` to the recorder,
   which asks PCSX2 to save to slot 5 before it starts).
2. **Start the recorder:**
   ```sh
   export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
   cargo run -q --release -p rc-trace -- record --seconds 90
   ```
   It prints where it writes (`~/PS2/ratchet1/traces/hero_<UTC time>.tsv`) and a status line every second. Press
   **Enter** in the terminal to stop early. Click back into PCSX2 to play.
3. **Move script** (stick = left stick; crouch = R1; jump = ✕). Leave ~1 s standing still between the parts:
   1. Stand still 2 s (the replay starts from a standing tick).
   2. **One Heli long jump:** run forward 1 s, hold R1, press ✕ (keep the stick forward). Land, let go of
      everything, wait 2 s.
   3. **Five chained long jumps:** run forward, then R1 + ✕ five times in a row, re-crouching and jumping as
      soon as you can after each landing, stick forward all the time. Stop, wait 2 s.
   4. **Standing high jump:** standing still, hold R1, press ✕. Land, wait 2 s.
   5. **Glide:** jump (✕), and hold ✕ while falling (best off a ledge) until you land. Wait 2 s.
4. Stop (Enter, or let `--seconds` run out). The summary shows rows, **missed ticks** (want 0), the states seen
   and the jumps found.
5. Compare with the port:
   ```sh
   cargo run -q --release -p rc-trace -- replay-hero --trace ~/PS2/ratchet1/traces/hero_<time>.tsv
   ```
   It prints the first divergence, per-field max / mean error, the per-jump table (PCSX2 vs port) and the state
   sequences side by side, and writes `hero_<time>.port.tsv` (the port's trace, same format) and
   `hero_<time>.report.txt` (everything, plus each jump's per-tick height and horizontal-speed curves).
   `hero-jumps --trace FILE` prints the per-jump table of one trace alone.

Several short takes (one per move-script part, each started standing) diff more cleanly than one long take: the
first divergence of a long take shifts everything after it.

## 3. The recorder

`rc-trace record [--pine [slot]] [--out FILE] [--seconds N] [--savestate N] [--poll-us N] [--quiet]`

- **Read-only.** It never writes EE memory and never pauses the emulator. The only request besides reads is
  `--savestate N`, off by default. (There is no `--give-items`: the user owns the items.)
- **One batched PINE message per poll** (~120 `MsgRead64`): the tick counter, all the fields, the tick counter
  again. A poll whose two counter reads differ straddled a tick and is dropped ("torn"). A row is written the
  first time a new counter value is seen, so each tick is recorded once; a counter step > 1 is counted as missed
  ticks (the summary lists the gaps). Idle polls sleep `--poll-us` (250 µs).
- **Tick alignment:** a row with `tick = N` is the state after the gameplay tick that incremented 0x15f5cc to N,
  with the pad input that tick consumed. The PAD record is rebuilt by `UpdatePad` only at the start of the next
  main-loop iteration (after vsync), so a read right after the increment sees that tick's input.
- **Pad bytes.** The game reads libpad2 into a stack buffer, so the 18 bytes are rebuilt from the PAD record
  (0x13c940): the axis / pressure copy +0x140..+0x17c (stored before the input lock, mirror undone), held +0x1c0
  (buttons, before the lock) and raw +0x1b0 (d-pad; its pressures where a lock cleared it). The rebuilt bytes
  decode to exactly what the game decoded (unit-tested against the port's `PadState`). libpad2's newer DMA
  buffer (PAD +0x1c / +0x9c, frame counters +0x7c / +0xfc) is kept as `lib_pad` for diagnosis only.
- **Camera record** is in each level overlay's data: level01 0x167240, level03 0x166ec0 (found in a Kerwan
  savestate); other levels are located at start (and on a level change) by scanning 0x160000..0x170300 for a record
  whose position is within 60 units of Ratchet, Euler w = 0, unit rows at +0x210 / +0x220 / +0x230 and Euler yaw =
  atan2(forward). The address goes in `# meta camera=`.

### Fields (column: address)

| column | address | meaning |
|---|---|---|
| tick / mode / level / mirror | 0x15f5cc / 0x15f5c4 / 0x15ed84 / 0x15edb4 | tick counter, game mode, planet, mirrored controls |
| pad | PAD 0x13c940 +0x140/+0x1b0/+0x1c0 | the 18 libpad2 bytes (rebuilt, see above) |
| lib_pad | PAD +0x1c / +0x9c | newer libpad2 DMA buffer (diagnostic) |
| pad_held / pad_pressed / pad_held_u | 0x13cae0 / 0x13cae4 / 0x13cb00 | PAD masks |
| state / substate / group / prev_state | 0x1413d4 / 0x1413d8 / 0x1413dc / 0x1413e0 | state machine |
| st_timer | 0x13f4e8 | ticks in the state |
| pos_x/y/z, yaw | 0x13f3d0.., 0x13f3e8 | position (feet), yaw |
| vel_*, disp_*, mom_* | 0x13f430, 0x13f450, 0x13f4a0 | velocity, displacement this tick, carried momentum |
| eff_len_xy, fwd_speed, target_yaw, target_speed, speed | 0x13f4b4, 0x13f4b8, 0x13f4d0, 0x13f4e0, 0x13f4e4 | |
| ground_z, height, air_ticks | 0x13f628, 0x13f62c, 0x13f65e (s16) | ground, height above it, air ticks |
| t_508, t_510, t_lockout, t_524, t_542, t_70c | 0x13f508, 0x13f510, 0x13f514, 0x13f524, 0x13f542 (s16), 0x13f70c (s16) | timers: Thruster lockout, invulnerability, landing lockout, glide jump buffer, jump lockout, landing-run blend |
| j_* | 0x13f744..0x13f7f0 | jump block fields (takeoff pos 0x13f750, landed 0x13f768, takeoff tick 0x13f770, h 0x13f780, g 0x13f7f0, …) |
| jump_raw | 0x13f720..0x13f800 | the whole jump block (PCSX2 only) |
| anim_seq / anim_frame / anim_t | [0x1413d0]+0x53 / +0x51 / +0x54 | Ratchet's moby anim (key B) |
| anim_key, anim_speed | 0x13fdf8, 0x13fde0 | key time readout, playback speed |
| stick_x/y, stick_mag, health | 0x141070, 0x1415ec, 0x1415f8 | |
| owned | 0x13d4c0..+37 | item-owned table (2 Heli-Pack, 3 Thruster-Pack) |
| back_state, back_id | 0x1404f4, 0x1404f8 | back item slot 3 (state 2 = ready; id 2/3/4) |
| cam_x/y/z, cam_roll/pitch/yaw | camera +0x00.., +0x10 / +0x14 / +0x18 | the `Camera` record: position, Euler |
| cam_fx.., cam_lx.., cam_ux.. | camera +0x210 / +0x220 / +0x230 | its forward / left / up rows (the hero reads yaw and rows) |

### Format

Tab-separated text: `# rc-trace hero trace v1`, `# meta key=value` lines, one `# field name<TAB>address<TAB>meaning`
line per column, the header of column names, then one row per tick. Floats are shortest round-trip (`{:?}`), so
parsing gives the recorded f32 bit for bit; byte blocks are hex (`-` = none). Columns are matched by name
(unknown ones ignored, missing ones 0), so the format can grow.

## 4. The replay

`rc-trace replay-hero --trace FILE [--extracted DIR] [--level N] [--start TICK] [--ticks N] [--tol X]
[--camera-at-hero] [--out-port FILE] [--report FILE] [--states N]`

- **Harness:** the hero-only run of rc-game's pack tests on the recorded level (collision, Ratchet's class and
  sequence table, the back packs 607 / 608 / 609 and Clank 601, the follow camera), one `Game` ticked through
  `Game::tick` with the recorded pad bytes (the port's normal input path). **No mobys** (moving platforms, crates,
  enemies) and no hand items: record on open ground.
- **Start:** the first standing, grounded, mode-0 sample (or `--start TICK`): position, yaw, target yaw,
  velocity, momentum, health, the owned table, the back item; the pad primed with that sample's bytes; the tick
  counter = its tick; the camera set up behind Ratchet at the **recorded camera's yaw** (the stick is read
  relative to the camera), and the recorded camera record (position, Euler, rows) published for the first tick
  (`Camera::new` publishes only a position). The follow camera's springs start at rest (they are not recorded), so
  the camera columns may drift for the first second. `--camera-at-hero` uses the hero's yaw instead.
- Ticks the recording missed run with the next recorded pad (reported as "filled").
- **Diff:** paired by tick; floats beyond `--tol` (1e-3), integers exact, angles wrapped. Reports the first
  divergence (hero columns; the camera separately), per-column max / mean error, ticks over tolerance and the
  first one. Exit status 0 = no divergence, 2 = divergence.
- **Jumps:** an airborne run (air ticks > 0) is a jump when it lasts ≥ 4 ticks or reaches a jump / glide state
  (groups 4 / 5); its windup is the jump-state ticks before the takeoff. Per jump: entry / takeoff / landing
  ticks, windup, takeoff horizontal speed and vz, apex height and tick, airtime, horizontal distance, mean air
  speed, gravity rising / falling (mean second difference of z, u/s²), landing state / anim / speed (and at +5,
  +10 ticks), momentum and 0x13f524 / 0x13f514 at landing, landing → next jump entry / takeoff, and the camera's
  rise, lag behind the apex and mean distance. The k-th jump of each side is compared.

No rc-game seam was needed: everything the replay sets is a public `Hero` / `Game` field.

## 5. Results

No PCSX2 recording yet: PINE was disabled in the user's PCSX2 (`EnablePINE = false` in PCSX2.ini) when the tools
were built, so the live test is pending (§1). Checked offline:

- The addresses, read with `hero-snap` from the user's savestates: Novalis (slots 1 / 2: camera 0x167240) and
  Kerwan (slot 3: camera 0x166ec0, the only record the scan finds), Heli-Pack owned and on the back (0x1404f4 = 2,
  0x1404f8 = 2), Ratchet standing (state 0, air ticks 0), the pad rebuilt as neutral.
- The pipeline on level data: `tests/hero_replay_selfcheck.rs` (a pad-only recording on Novalis → replay → the
  port's trace through the file format → replayed again from its own first sample: no divergence at all, and the
  long jumps are segmented), and `replay-hero` on a Kerwan trace seeded from the slot-3 savestate.
- The port's Heli-Pack long jump on Kerwan's plaza (run, R1 + ✕), for the comparison to come: windup 6 ticks,
  takeoff 8.6 u/s horizontal and 5.49 u/s up, apex 1.415 at 29 ticks, airtime 60 ticks, gravity −11 u/s²
  constant, 8.7 units, lands in 3 (anim 5) at the takeoff speed and slows to ~6.3 u/s within 5 ticks; the
  follow camera does not rise at all during the jump.
