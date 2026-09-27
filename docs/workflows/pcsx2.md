# Workflow: PCSX2 (savestates, PINE, record, replay, compare)

Tool: `tools/trace` (package `rc-trace`; README there). Design, addresses and triage: `docs/plan/trace_harness.md`.
All commands run from the repo root with `export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.

## Where things go

| What | Where |
|---|---|
| Kept savestates (`.p2s`) and their screenshots | `~/PS2/ratchet1/savestates/` (personal; `RC_PERSONAL` overrides `~/PS2/ratchet1`) |
| Hero recordings (`hero_<UTC>.tsv`) | `~/PS2/ratchet1/traces/` (personal) |
| EE dumps, reports, CSVs, replay output | `work/trace/` (generated; `RC_WORK` overrides `work/`) |
| Test inputs | only distilled, numbers-only fixtures committed under `tools/trace/tests/fixtures/` |

`rc-trace` never writes into `extracted/`.

## One-time setup

| Task | Command / action |
|---|---|
| Enable PINE (live reads, recording) | PCSX2 → Tools → Show Advanced Settings; then Settings → Advanced → PINE: tick **Enable**, slot **28011** (the default) |
| Check PINE sees the game | `cargo run -q -p rc-trace -- pine-info` |

## Savestates

| Task | Command / action |
|---|---|
| Take a savestate | In PCSX2 press **F1** (Fn+F1) or System → Save State (lands in PCSX2's `sstates` folder) |
| Keep the newest one as `NAME` | `cargo run -p rc-trace -- save-state NAME` (copies it to `~/PS2/ratchet1/savestates/NAME.p2s`) |
| Dump a kept state's EE RAM | `cargo run -p rc-trace -- dump-ee --state NAME` (writes `work/trace/NAME_ee.bin`) |

Anywhere a command takes `--state`, `NAME` means the kept `~/PS2/ratchet1/savestates/NAME.p2s`, `latest` the newest
state in PCSX2's folder; `--ee FILE` reads a dump and `--pine` the running game.

## Compare

| Task | Command |
|---|---|
| tfrag lighting vs the game | `cargo run -p rc-trace -- compare-tfrag-light --state NAME --level 01` |
| Novalis spawn state (a–g in `trace_results_novalis.md`) | `cargo run -p rc-trace -- compare-novalis-spawn --state novalis_spawn` |
| Tie / shrub lighting only | `cargo run -p rc-trace -- compare-tie-shrub-light --state NAME` |

Reports and CSVs go to `work/trace/`. Exit status 0 = all equal, 2 = mismatches (triage them per
`trace_harness.md` §8), 1 = error.

## Record and replay (hero feel pass, `docs/plan/hero_feel_pass.md`)

| Task | Command |
|---|---|
| Record the hero live over PINE | `cargo run --release -p rc-trace -- record --seconds 90` (to `~/PS2/ratchet1/traces/hero_<UTC>.tsv`) |
| Replay a recording through the port and diff | `cargo run --release -p rc-trace -- replay-hero --trace ~/PS2/ratchet1/traces/hero_<UTC>.tsv` (to `work/trace/`) |
| Per-jump table of a recording | `cargo run -p rc-trace -- hero-jumps --trace FILE` |

## Findings reach tests only as distilled fixtures

A test never reads a savestate, an EE dump or a recording: those are personal or generated, and absent on any other
machine. When a comparison becomes a regression, distil the facts it needs into a small, human-readable,
numbers-only fixture (no raw memory), commit it, and test the port against it with only `extracted/`.

| Task | Command |
|---|---|
| Rewrite the Novalis spawn fixture | `cargo run -p rc-trace -- distill-spawn --state novalis_spawn` |
| Run the regression | `cargo test -p rc-trace --test novalis_spawn` |

`distill-spawn` writes `tools/trace/tests/fixtures/novalis_spawn.tsv` only after running the fixture checks and the
savestate checks side by side and finding the same tallies. A new scene gets its own `distill-*` command in the same
shape.
