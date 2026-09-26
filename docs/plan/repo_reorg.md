# Repo reorg: product vs dev tools (deferred plan)

Status: **planned, not started** (written 2026-09-27). Carry it out at a natural pause; about 20–30 minutes
of agent time. Until then the current layout stays as it is.

## 1. Problem

Dev helpers have landed wherever the task that needed them happened to run, so the repo mixes the product
with the tooling that helps build it. There are no decided folders and no documented workflows. Examples as of
2026-09-27:

- **Ghidra level import.** A former C++ function wrote level overlays into a root `ghidra/import/` folder for
  Ghidra. It left with the C++ retirement (afe8174), but `decomp/scripts/import_overlays.py` is still here,
  untouched, and still reads `ghidra/import/`.
- **`decomp/` at the root** next to product code. It holds Ghidra scripts (`decomp/scripts/`), name tables
  (`decomp/names/`) and a 65 MB git-ignored decompiler export (`decomp/export/`).
- **PCSX2 trace data in the game-data folder.** `extracted/traces/` (101 MB) mixes the user's own recordings
  (`novalis_spawn.p2s`, `novalis_idle.p2s` savestates and their screenshots) with regenerable output: EE and
  scratchpad dumps (`*_ee.bin`, `*_spr.bin`, 32 MB each), reports, stdout captures and CSVs. `rc-trace` writes
  there by default (`crates/rc-trace/src/main.rs`: `compare-tfrag-light` CSV, `compare-novalis-spawn` report
  and moby CSV), and `crates/rc-trace/tests/novalis_spawn.rs` reads `traces/novalis_spawn_ee.bin` from it.
  Only `record-hero` already writes outside the repo (`~/PS2/ratchet1/traces/`, `hero_record.rs`).
- **C++-era leftovers in `extracted/`.** Files a `randcrw-extract` data folder does not have are still in the
  dev tree: `levels/NN/*.dec`, `*_dump.bin`, `overlay.elf`, `overlay.txt`, `*.obj`, `*_topdown.png`,
  `moby_0000_front.png`, the `core/`, `gameplay/` and `textures/` splits, and `extracted/vu/` (VU disassembly
  listings that the format docs cite by line number). `launcher_extractor.md` §4.1 already lists them as derived
  dev data, about 5 GiB, that nothing has read since 2026-09-27.
- **`rc-trace` sits in `crates/`** beside the shipped crates, although it is a PCSX2 dev tool.

## 2. Target layout

```
randcre/
├── crates/                 PRODUCT: ships; never depends on tools/
│   ├── rc-formats          disc + level format readers, load-time passes
│   ├── rc-data             Tier 1 engine cache
│   ├── rc-game             game logic ports
│   ├── rc-engine           Bevy app, binary `randcrw`
│   └── rc-extract          binary `randcrw-extract` (extract / verify / prepare / export)
├── tools/                  DEV TOOLING: never ships (package allow-list); one README per tool
│   ├── ghidra/
│   │   ├── README.md
│   │   ├── scripts/        ghidra_http, import_levels, apply_*_names, export_decomp, overlay_diff
│   │   └── names/          today's decomp/names (boot_functions.csv, overlay_names.csv, doc_names.csv, clusters.tsv)
│   ├── trace/              today's crates/rc-trace (PCSX2 harness; workspace member at the new path)
│   ├── package/            release packaging (package.sh)
│   └── vu/                 (future, only if renderer or shadow work needs it) VU microprogram disassembler
├── docs/
│   ├── formats/  plan/
│   └── workflows/          ghidra.md, pcsx2.md, game-data.md, release.md, launcher.md
├── extracted/   (ignored)  ONLY game data written by randcrw-extract: Tier 0, cache/, optionally exports/
├── work/        (ignored)  regenerable dev output
│   ├── decomp/             decompiler export (today decomp/export/)
│   ├── ghidra-import/      level ELFs from `randcrw-extract export --what code`
│   └── trace/              EE/scratchpad dumps, reports, CSVs, stdout captures
├── dist/        (ignored)  release packages
└── target/      (ignored)

~/PS2/ratchet1/             the user's persistent personal files, outside the repo
├── <disc>.iso
├── ghidra/                 Ghidra project (the user moves today's ~/ratchet1.gpr + ~/ratchet1.rep here)
└── traces/                 recordings (hero_*.tsv), PCSX2 savestates (*.p2s) and their screenshots
```

No other top-level folders: no `ghidra/`, no `decomp/`.

## 3. Rules

1. **Product vs tooling.** `crates/` ships and never depends on anything under `tools/`. A workspace test
   enforces this with `cargo metadata` (no product package has a path dependency under `tools/`). `tools/`
   never ships; `tools/package/package.sh`'s allow-list already enforces that.
2. **Data goes to its one home.** `extracted/` holds only what `randcrw-extract` writes. Regenerable dev output
   goes to `work/`. The user's irreplaceable files go to `~/PS2/ratchet1/`. No tool writes into `extracted/`.
3. **One documented command per task** in `docs/workflows/`. Never write a doc instruction telling anyone to
   recover something from git history for a simple task; if a workflow needs it, it lives in the tree.
4. **Every agent brief states product or tooling, and which folder it touches.** Added to the orchestration
   handbook (`docs/plan/orchestration.md` §3.2 brief template and §4.3 standing rules).
5. **No speculative code.** Only port or keep what a current, concrete workflow uses. Ghidra and PCSX2 are real
   workflows, so their tools are legitimate; `tools/vu/` comes back only when a task needs it.

## 4. Migration checklist

Do the moves with `git mv` so history follows. One agent, tooling work; touches `tools/`, `docs/`, `crates/`
(workspace list and the dependency test only), `.gitignore`, `README.md`.

### 4.1 Save first

- [ ] Before any cleanup of `extracted/`, copy the 19 `extracted/levels/NN/overlay.elf` files (still present on
      2026-09-27) to `work/overlay_elf_reference/`. They are the byte-identical reference for §4.4.

### 4.2 Moves

- [ ] `decomp/scripts/*` → `tools/ghidra/scripts/`; `decomp/names/*` → `tools/ghidra/names/`.
- [ ] `decomp/export/` → `work/decomp/` (plain move, ignored data).
- [ ] Delete `decomp/scripts/import_overlays.py` once `import_levels.py` (§4.4) works. Check whether
      `export_overlays.py` is still used; drop it if no workflow uses it (rule 5).
- [ ] `crates/rc-trace` → `tools/trace` (package name stays `rc-trace`).
- [ ] `extracted/traces/`: `*.p2s` and their `*.png` screenshots → `~/PS2/ratchet1/traces/`; everything else
      (`*_ee.bin`, `*_spr.bin`, `*_report*.txt`, `*_stdout*.txt`, `*.csv`, `tfrag_cmp.txt`) → `work/trace/`.
      Then remove `extracted/traces/`.
- [ ] Audit `extracted/`: run `randcrw-extract extract` into a scratch folder and list every path in
      `extracted/` it does not produce (expected: the C++-era files of §1 and `traces/`, about 5 GiB). Delete them
      after §4.1, except
      `extracted/vu/` (open question 2).

### 4.3 Path references to update

- [ ] **Workspace members** (`Cargo.toml`): `crates/rc-trace` → `tools/trace`; fix its `path = "../rc-formats"`
      style dependencies to `../../crates/…`. `Cargo.lock` needs no manual edit.
- [ ] **rc-trace paths**: default outputs in `main.rs` (`traces/level{NN}_tfrag_light_mismatches.csv`,
      `traces/novalis_spawn_report.txt`, `traces/novalis_spawn_mobys.csv`) → `<repo>/work/trace/`; the usage
      text line that names `extracted/traces/`; `tests/novalis_spawn.rs` reads `work/trace/novalis_spawn_ee.bin`
      (still skips when missing). `record-hero` already uses `~/PS2/ratchet1/traces/`.
- [ ] **`.gitignore`**: replace `/decomp/export/` with `/work/`.
- [ ] **Scripts**: `ROOT`-relative paths in the Ghidra scripts (`decomp/names`, `decomp/export`) → the new
      folders; `overlay_diff.py` keeps reading `extracted/boot` and `extracted/levels/NN/overlay.bin` (Tier 0).
- [ ] **Code comments** that cite `decomp/`: `rc-game/src/moby_update/{creature,triggers}.rs`,
      `rc-trace/src/tfrag_light_cmp.rs`.
- [ ] **Docs** that cite `decomp/` or `extracted/traces/`: `decomp_workflow.md` (becomes a pointer to
      `docs/workflows/ghidra.md`), `trace_harness.md`, `trace_results_novalis.md`, `particles.md`,
      `hardware_fidelity_layers.md`, `creatures.md`, `game_state.md`, `hero_states.md`, `level_generalisation.md`,
      `moby_update_catalogue.md`, `player_controller.md`, `roadmap.md`, `tfrag_lighting.md`, `triggers.md`, and
      the leftovers row of `launcher_extractor.md` §4.1. (`hero_feel_pass.md` already uses `~/PS2/ratchet1/traces/`.) Grep `decomp/`, `extracted/traces`, `crates/rc-trace`,
      `~/ratchet1` afterwards; zero hits outside dated history lines.
- [ ] **Handbook** (`docs/plan/orchestration.md`): §2 repository map, the brief template line (rule 4), §4.3.
- [ ] **README**: open with the repo map (the §2 tree, short form); the path table loses `decomp/` and
      moves `rc-trace` under tools.
- [ ] **decisions.md**: turn the pending row into a dated decision row.

### 4.4 New pieces

- [ ] `randcrw-extract export --what code [--level NN]` (product, `crates/rc-extract`): writes each level's
      overlay as an ELF with the original load addresses, in Rust, to `--to` (the workflow uses
      `work/ghidra-import/`). Check: byte-identical to the saved `overlay.elf` files from §4.1 for all 19 levels.
      Without them, the check is the Ghidra import itself.
- [ ] `tools/ghidra/scripts/import_levels.py`: imports those ELFs into the project's `/levels` folder via
      GhidraMCP (no language argument: that forces the raw loader; see memory gotchas).
- [ ] Workspace test for rule 1 (`cargo metadata`, in `crates/rc-extract/tests/` or a small workspace test).
- [ ] READMEs: `tools/ghidra/README.md`, `tools/trace/README.md`, `tools/package/README.md` (purpose + commands).
- [ ] `docs/workflows/`:
  - `ghidra.md`: set up the project from scratch (`~/PS2/ratchet1/ghidra/`, GhidraMCP), import boot + levels,
    apply names, export decomp to `work/decomp/`.
  - `pcsx2.md`: PINE (Tools → Show Advanced Settings), savestates to `~/PS2/ratchet1/traces/`, record/replay,
    regenerate EE dumps into `work/trace/`.
  - `game-data.md`: extract / verify / prepare / export; regenerating dev data (`work/`).
  - `release.md`: `tools/package/package.sh`.
  - `launcher.md`: building the game for the `randcrw-launcher` Development source.

### 4.5 Verification

- [ ] `cargo test --workspace` (including the new dependency test and rc-trace's tests at the new path).
- [ ] `cargo dev-build`.
- [ ] One Ghidra import test: export `--what code --level 01`, run `import_levels.py` into a **throwaway Ghidra
      folder** (not `/levels`), confirm it analyses as r5900 at the original addresses, then delete it. One level
      only: Ghidra analysis is slow.
- [ ] `git status` shows nothing new under `extracted/`; `ls` at the root shows no `decomp/` or `ghidra/`.

## 5. Open questions

1. **Ghidra project move.** The project is `~/ratchet1.gpr` + `~/ratchet1.rep` today (the workflow doc says
   `~/ratchet1`). The user moves it to `~/PS2/ratchet1/ghidra/`; confirm the new name before docs cite it.
2. **`extracted/vu/`** (784 KB of C++-era VU disassembly the format docs cite by line number). Options: keep it
   in `work/vu/` as reference data with a note, or regenerate it once `tools/vu/` exists. Deleting it breaks
   those citations.
3. **`~/PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It)/`** holds an unpacked disc filesystem next to the
   ISO. Is it still needed?
4. **`work/` vs a per-user location** for large regenerable data (trace dumps are 32 MB each). `work/` is the
   default; revisit only if disk use becomes a problem.
5. **Package name.** Keep `rc-trace` for the moved crate, or rename to match `tools/trace`?
