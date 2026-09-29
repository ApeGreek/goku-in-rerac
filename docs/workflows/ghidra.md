# Workflow: the Ghidra project

Tools: `tools/ghidra/` (README there). Conventions and the history of the naming runs: `docs/plan/decomp_workflow.md`.
Run every command from the repo root with the Ghidra GUI open on the project and the GhidraMCP plugin serving
`127.0.0.1:8089`.

> The scripts moved from `decomp/scripts/` on 2026-09-27 and have **not been re-run since** (paths updated only).
> Start with a dry run where a script has one.

## What the project holds, and why it is disposable

* **Programs.** `/SCUS_971.99` is the boot ELF (the engine and every system shared by all levels). Each level has its
  own **overlay**: a block of level-specific code and data the game loads over a fixed address window when the level
  starts. Overlays overlap the boot ELF's game-code window and each other, so each one is its own program,
  `/levels/levelNN.elf` (00–18); boot addresses are not valid inside a level and vice versa.
* **The source of truth is not the project.** It is the ISO (through `extracted/`), the name tables in
  `tools/ghidra/names/` (`boot_functions.csv`, `overlay_names.csv`, `doc_names.csv`, `clusters.tsv`) and the scripts.
  Everything in the project can be rebuilt from those, so the project is disposable: a broken or lost project costs
  analysis time, not knowledge. Names we derive go into the CSVs first, then into Ghidra.
* **Where it is.** Today `~/ratchet1.gpr` + `~/ratchet1.rep`. Its target home is `~/PS2/ratchet1/ghidra/` (the user
  moves it; nothing in the repo depends on the path).
* **Re-import is only for a from-scratch setup.** The existing project already has every program analysed and named;
  do not re-import into it.

## Naming flow: CSV → apply → export

| Task | Command |
|---|---|
| Record a name | Add a row to `tools/ghidra/names/doc_names.csv` (columns and rules: `docs/plan/decomp_workflow.md`) |
| See what applying would do | `python3 tools/ghidra/scripts/apply_doc_names.py --dry-run` |
| Apply the names and re-export the touched functions | `python3 tools/ghidra/scripts/apply_doc_names.py --apply --export` |
| Re-export export entries whose name went stale | `python3 tools/ghidra/scripts/apply_doc_names.py --sync-export` |
| Export one program's decompiled C | `python3 tools/ghidra/scripts/export_decomp.py SCUS_971.99` |
| Export the levels (level01 in full, others only their own functions) | `python3 tools/ghidra/scripts/export_overlays.py` |

The export lands in `work/decomp/<program>/` (one `.c` per function plus `index.tsv`; git-ignored, it is the game's
code). Agents and people read decompiled code there.

## Masked overlay diff: does a function differ between levels?

Each level overlay compiles its own copy of the engine and hero code, linked at other addresses. `overlay-diff`
answers "is level N's copy the same code as level 01's, and if not, what differs" without Ghidra (it reads
`extracted/levels/NN/overlay.bin`; names come from `work/decomp/*/index.tsv` when present). Read disassembly only for
the cells it flags.

| Task | Command |
|---|---|
| The hero table (the functions `crates/rc-game/src/hero/**` cites, and their callees) | `cargo run -p rc-trace -- overlay-diff` |
| Any functions | `cargo run -p rc-trace -- overlay-diff --fn L01:23cf98 --fn L00:20cd08 --name mine` |
| Functions another module cites | `cargo run -p rc-trace -- overlay-diff --cite crates/rc-game/src/moby_update --callees 0 --name classes` |
| Longer hunks (default 24 lines a side) | add `--max-lines 400` |

Outputs in `work/overlay_diff/`: `NAME_matrix.txt` (one character per level), `NAME.tsv` (the counterpart address
and the counts per cell), `NAME_diffs.txt` (per differing cell only the differing instructions, disassembled, with
two aligned instructions of context; "level code neither level 01 nor level 00 has" first).

| Cell | Meaning |
|---|---|
| `=` | identical after masking the relocated fields (`jal` targets, `%hi`/`%lo`, `$gp`, stack offsets), and the same boot addresses, constants and callees (the TSV cell is `= <counterpart address>`: a level-address map for any list of `--fn`s) |
| `c` | the function's own code is identical; a callee differs (see the callee's row) |
| `S` | a switch whose shared cases are identical; only the set of cases differs |
| `k` | the same code reading other constants |
| `s` | differs from level 01, but all of it is level 00's (the superset build's) code |
| `U` | the level has code neither level 01 nor level 00 has: read these |
| `D` | differs, and there is no level-00 copy to check against |
| `?` | no counterpart found (the level lacks the function, or the matching failed; the method is in the TSV) |

Method and limits: `tools/trace/src/overlay_diff.rs` (module doc); results for the hero:
`docs/plan/level_generalisation.md` H2. It replaces re-running `overlay_diff.py` for identity questions (X1): the
masking is the port's own (`rc_formats::level_overlay::mask`, `%lo` included), so there is no second copy of it.
`overlay_diff.py` still builds `names/clusters.tsv` for the naming flow.

## Set up from scratch

Only for a new machine or a lost project. Needs `extracted/` (docs/workflows/game-data.md), the PS2 processor module
(`~/Globals/ghidra-emotionengine-reloaded`) and the GhidraMCP plugin (`~/Globals/ghidra-mcp`) installed in Ghidra.
One step per row, in order:

| Step | Command / action |
|---|---|
| 1. Create the project | Ghidra GUI → File → New Project → `ratchet1` in `~/PS2/ratchet1/ghidra/`; leave it open |
| 2. Import and analyse the boot ELF and the 19 levels | `python3 tools/ghidra/scripts/import_levels.py` |
| 3. Boot names (Lombyte) | `python3 tools/ghidra/scripts/apply_boot_names.py` |
| 4. Cluster the overlays by code hash | `python3 tools/ghidra/scripts/overlay_diff.py` |
| 5. Overlay names from the clusters | `python3 tools/ghidra/scripts/apply_overlay_names.py` |
| 6. Names from our docs | `python3 tools/ghidra/scripts/apply_doc_names.py --apply` |
| 7. Export the boot ELF | `python3 tools/ghidra/scripts/export_decomp.py SCUS_971.99` |
| 8. Export the levels | `python3 tools/ghidra/scripts/export_overlays.py` |

Step 2 reads the level ELFs from `work/ghidra-import/levelNN.elf`. **Follow-up:** they should come from
`randcrw-extract export --what code` (not built yet; docs/plan/repo_reorg.md). Until then the 19 files in
`work/ghidra-import/` are copies of the retired C++ extractor's `overlay.elf` files, made on 2026-09-27; keep them
(they are in the backup below).

## Backups

| Task | Command |
|---|---|
| Back up the project (GUI closed) | `tar -C ~ -czf ~/PS2/ratchet1/ghidra-backup-$(date +%Y%m%d).tgz ratchet1.gpr ratchet1.rep` |
| Back up the level ELFs the from-scratch import needs | `tar -C work -czf ~/PS2/ratchet1/ghidra-import-$(date +%Y%m%d).tgz ghidra-import` |

The name tables and scripts are in git, so they need no backup of their own. After the project moves to
`~/PS2/ratchet1/ghidra/`, back up that folder instead.

## Reference material (not vendored)

Checked out next to the repo on the author's machine, never copied into it:

* `~/Globals/wrench`: Wrench (GPL-3), used as format documentation only; all reader code here is written from the
  specs in `docs/formats/`.
* `~/Globals/jak-project`: OpenGOAL (ISC), architectural reference.
* `~/Globals/ghidra-emotionengine-reloaded`: PS2 processor module for Ghidra.
* `~/Globals/ghidra-mcp`: GhidraMCP bridge to the Ghidra project.
