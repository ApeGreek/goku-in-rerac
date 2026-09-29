# Repo layout: product vs dev tools (as built)

Status: **done** 2026-09-27 (commit "Reorganise repo: product crates vs dev tools"). This is the as-built spec; the
deferred product changes are under **Follow-ups**. Decision row: `docs/plan/decisions.md` (2026-09-27).

## 1. Why

Dev helpers had landed wherever the task that needed them ran: Ghidra scripts and name tables in a root `decomp/`
beside the product, PCSX2 dumps and the user's savestates inside the game-data folder (`extracted/traces/`), a
committed test that read a dump of the user's personal savestate, a Ghidra import script pointing at a removed
folder, and the PCSX2 harness among the shipped crates.

## 2. Four kinds of data, four homes

| Kind | Home | Git |
|---|---|---|
| Source: code, docs, Ghidra scripts, name lists, committed expected values (hash tables, distilled fixtures) | repo | yes |
| Game data, rebuildable from the ISO | `extracted/` (dev) | no |
| Generated dev output, rebuildable by a tool | `work/` | no |
| The user's personal material: ISO, Ghidra project, savestates, recordings | `~/PS2/ratchet1/` | no |

Only source and game data may feed tests. Personal material never does.

## 3. Layout

```
crates/                 PRODUCT: rc-formats rc-data rc-game rc-engine (randcrw) rc-extract (randcrw-extract)
tools/                  DEV ONLY, each subfolder has a README (purpose, commands, inputs, outputs)
  ghidra/scripts/         Ghidra scripts (from decomp/scripts/; import_overlays.py renamed import_levels.py)
  ghidra/names/           name tables (from decomp/names/)
  trace/                  PCSX2 harness, package rc-trace (from crates/rc-trace)
  package/                release packaging
  repo-checks/            guard tests for this layout (no dependencies)
  xtask/                  `cargo xtask <command>` dev chores: regen-data, package (std only; added 2026-09-27)
docs/formats/ docs/plan/ docs/workflows/
extracted/   (ignored)  game data only (what randcrw-extract writes)
work/        (ignored)  generated dev output:
  decomp/                 decompiler export (from decomp/export/, moved by hand, not regenerated)
  ghidra-import/          level ELFs for a from-scratch Ghidra import (levelNN.elf, 19 files)
  trace/                  EE / scratchpad dumps, reports, CSVs, replay output of rc-trace
  vu/                     VU microprogram listings the format docs cite by line number
  exports/ captures/      conventions: randcrw-extract exports for dev work; screenshots and debug captures
  data-staging/ data-old/ transient: `cargo xtask regen-data` extracts into the first and swaps the old extracted/
                          through the second (both gone after a successful run)
dist/ target/ (ignored)
~/PS2/ratchet1/         the ISO; savestates/; traces/ (recordings); ghidra/ (the project's future home)
```

No other top-level folders (`tools/repo-checks` checks it). The Ghidra project stays at `~/ratchet1.gpr` +
`~/ratchet1.rep` until the user moves it to `~/PS2/ratchet1/ghidra/`.

## 4. Rules

1. **Product vs tooling.** `crates/` ships and never depends on anything under `tools/` (`tools/repo-checks` parses
   every `crates/*/Cargo.toml`). `tools/` never ships (`tools/package/package.sh`'s allow-list).
2. **Data goes to its one home.** `extracted/` holds only what `randcrw-extract` writes. Tools write generated output
   only to `work/` (`rc-trace` refuses paths inside `extracted/`); the one deliberate exception is a committed
   fixture (`rc-trace distill-spawn`). Personal material stays in `~/PS2/ratchet1/`.
3. **No test depends on personal files.** PCSX2 findings reach tests only as distilled, numbers-only fixtures
   (`tools/trace/tests/fixtures/novalis_spawn.tsv`). `tools/repo-checks` rejects product files that mention `.p2s`,
   `~/PS2`, `PS2/ratchet1`, `work/`, `/traces/` or `RC_PERSONAL`, apart from the listed known lines (Follow-up 6).
4. **One documented command per task** in `docs/workflows/`; no instruction to recover anything from git history.
5. **Every agent brief states product or tooling, and which folder** (`docs/plan/orchestration.md` §3.2, §4.3).
6. **No speculative code.** Only what a current workflow uses.

## 5. What was done

**Moves.** `git mv`: `decomp/scripts` → `tools/ghidra/scripts`, `decomp/names` → `tools/ghidra/names`,
`crates/rc-trace` → `tools/trace` (package name `rc-trace` kept, so `cargo run -p rc-trace` works). Plain `mv`
(untracked): `decomp/export/*` → `work/decomp/`; `extracted/traces/` → the two savestates `novalis_spawn.p2s`,
`novalis_idle.p2s` and their PNGs to `~/PS2/ratchet1/savestates/`, everything else (dumps, reports, CSVs, stdout
captures) to `work/trace/`. Copied, not moved (the user's `extracted/` stays until regenerated):
`extracted/levels/NN/overlay.elf` → `work/ghidra-import/levelNN.elf` (00–18) and `extracted/vu/` → `work/vu/`.
`decomp/` and `extracted/traces/` are gone. `.gitignore`: `/work/` replaces `/decomp/export/`.

**tools/trace.** `work_dir()` (`RC_WORK`, else `<repo>/work`), `trace_out_dir()` (`work/trace`), `personal_dir()`
(`RC_PERSONAL`, else `~/PS2/ratchet1`), `savestates_dir()`, `recordings_dir()`; every write goes through
`write_output`, which refuses `extracted/`. Default outputs moved to `work/trace/` (tfrag CSV, spawn report and moby
CSV, `dump-ee` default `work/trace/<name>_ee.bin`, `replay-hero` output). `--state NAME` resolves
`~/PS2/ratchet1/savestates/NAME.p2s`. New commands: `save-state NAME` (keep the newest PCSX2 state) and
`distill-spawn` (below).

**The spawn test.** `tests/trace/novalis_spawn.rs` used to read `extracted/traces/novalis_spawn_ee.bin`, a dump of the user's
savestate. `distill-spawn` now reads such a dump once and writes `tests/fixtures/novalis_spawn.tsv` (176 KB of
numbers: counters, fog and underwater look, RAM light bank, per-chunk FNV-1a hashes, the 1022 moby slots' slot /
class / spawn id / state / mode / group / distances / load-pass draws, per-palette hashes of 1508 ties and 1208
shrubs). The test compares the port (only `extracted/` needed) with it using the same thresholds. `distill-spawn`
runs the fixture checks and the savestate checks side by side and refuses to write when a tally differs; on
2026-09-27 all 11 shared tallies agreed (tie instances 1507/1508, shrubs 1208/1208, game state 250/254, moby slots
929/929, rejected 54/54, spin bits 281/281, crate turns 199/199, fog, underwater, tick counter equal; rand state 0/1
in both). The ties are compared per instance by hash (the savestate check also counted entries: 96511/96512).
The other trace tests (`hero_replay_selfcheck`, `synthetic`) never read personal files.

**Ghidra scripts.** Paths repointed (names `tools/ghidra/names/`, export `work/decomp/`, import ELFs
`work/ghidra-import/`, `ROOT` one level deeper); `import_levels.py` also imports `extracted/boot/SCUS_971.99` when
the project lacks it. Every script says in its header that it has not been re-run since the reorg. Nothing was run
against the project.

**tools/repo-checks.** Three guards (§4 rules 1–3) plus self-tests of the scanners.

**Docs.** `docs/workflows/{ghidra,pcsx2,game-data,release,launcher}.md`, READMEs for every tool, the README's repo map,
the orchestration handbook's rules, and every reference to the old paths.

## 6. Follow-ups

Deferred because they need product changes (hard limit of the reorg) or a Ghidra run:

1. **`randcrw-extract export --what code [--level NN]`** (product, `crates/rc-extract`): each level's overlay as an
   ELF with the original load addresses, for `work/ghidra-import/`. Check: byte-identical to the 19 saved
   `work/ghidra-import/levelNN.elf`. Then `docs/workflows/ghidra.md` step 2 uses it.
2. **`randcrw-extract verify --strict`** (product): also report files in a data folder the archive does not have
   (e.g. C++-era leftovers in a dev `extracted/`).
3. **`RC_REQUIRE_DATA=1`** (product, `rc_formats::test_data` and the test helpers): tests fail instead of skipping
   when `extracted/` is missing, for CI-style runs.
4. **Loud "SKIPPED" messages** in `rc_formats::test_data` (product): one clear line per test that skips for want of
   data.
5. **Engine captures default to `work/captures/`** (product, `rc-engine` `RC_SCREENSHOT`): a relative or bare name
   lands there instead of the working directory.
6. **The disc golden tests' ISO fallback** (product tests): `crates/rc-formats/tests/formats/golden.rs`
   (`disc_matches_extracted_for_every_level`, doc line and fallback line), `crates/rc-game/tests/ui/game_state_novalis.rs`
   (`disc_save_game_lump_matches_extracted`) and the example command in `crates/rc-extract/tests/extract/golden.rs` name
   `~/PS2/ratchet1/<disc>.iso`. Make them `RC_ISO`-only, then drop the four `KNOWN_OFFENDERS` entries in
   `tools/repo-checks/src/lib.rs`.
7. **Merge the three apply scripts** (`apply_boot_names.py`, `apply_overlay_names.py`, `apply_doc_names.py`) into
   one; needs a Ghidra run to verify, so not done blind.
8. **VU microprogram disassembler** as a dev tool (`tools/vu/`, OpenGOAL's ISC disassembler may be mirrored with
   attribution) to regenerate `work/vu/`; until then `work/vu/` is hand-kept reference data.
9. **Re-run the Ghidra scripts once** (dry runs first) to confirm the path updates, then drop the "not re-run" headers.
10. **User steps:** move the Ghidra project to `~/PS2/ratchet1/ghidra/`; regenerate the dev `extracted/`
    (`cargo xtask regen-data`, `docs/workflows/game-data.md`) to drop the ~5 GiB of C++-era leftovers; decide whether the unpacked disc folder
    `~/PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It)/` next to the ISO is still needed.
