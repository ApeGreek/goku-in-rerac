# tools/xtask: one short command per dev chore

Dev only, never ships. Package `xtask` (workspace member at `tools/xtask`), run through the Cargo alias in
`.cargo/config.toml` (`xtask = "run -q -p xtask --"`), so every command is `cargo xtask <command>`. Std only, no
dependencies: it shells out to `cargo` and to `tools/package/package.sh`, so building it compiles only this crate.

| Command | What it does |
|---|---|
| `cargo xtask help` | Lists the commands (also `cargo xtask` alone). |
| `cargo xtask regen-data` | Rebuilds the dev game data folder `extracted/` from your disc image (below). |
| `cargo xtask package [--no-build]` | Runs `tools/package/package.sh` with the same arguments (`docs/workflows/release.md`). |

## `regen-data`

```
cargo xtask regen-data [--iso <image>] [--data-dir <dir>] [--force] [--check]
```

* **Inputs:** the disc image (`--iso`, else `RC_ISO`); the Tier 0 table `crates/rc-extract/data/*.tsv`.
* **Output:** the data folder: `--data-dir`, else `RC_EXTRACTED`, else `<repo>/extracted`.

What it does, in order:

1. **The disc.** Without `--iso` or `RC_ISO`, it prints how to set `RC_ISO` once (fish: `set -Ux RC_ISO "…"`; zsh /
   bash: an `export` line in `~/.zshrc` / `~/.bashrc`) and exits with code 2.
2. **What the folder holds.** Every file is sorted into:
   * **extractor output:** the Tier 0 table's files, `extract-info.json`, `cache/`, `exports/`;
   * **known C++ extractor leftovers,** listed with counts and sizes, then deleted with the old folder: `*.partial`,
     `.dec` copies, `*_dump.bin`, `levels/NN/{core,gameplay,textures}/` splits, `levels/NN/overlay.{elf,txt}`, `vu/`,
     `levels/NN/*.{png,obj}` previews, `.DS_Store` (the overlay ELFs and VU listings were copied to `work/` on
     2026-09-27);
   * **unknown:** anything else. The first 20 are listed and the run stops (exit 1) unless `--force`, which deletes
     them too.
   `--check` stops here and changes nothing (no disc needed). It never replaces `/`, your home folder, the repository
   or a folder holding `.git` / `Cargo.toml`; inside the repo only `extracted/` or a folder under `work/`.
3. **Extract into staging.** It builds `randcrw-extract` with the normal dev profile (`cargo build -p rc-extract`,
   never `--release`) and runs `extract` (which ends with `prepare`, the engine cache) into a staging folder on the
   same filesystem: `work/data-staging` for `<repo>/extracted`, else `.<name>.xtask-staging` next to the folder.
4. **Verify** the staging folder (`randcrw-extract verify`: every Tier 0 file's size and SHA-1).
5. **Swap.** The old folder is renamed aside (`work/data-old`, or `.<name>.xtask-old`), the new one renamed in, the
   old one deleted. Any failure before the swap deletes the staging folder and leaves the old folder untouched; a run
   killed mid-swap is repaired by the next run (the aside folder is restored or deleted).
6. **Summary:** files, bytes, engine cache lumps and size, and the time of each step.

Timing on the author's Mac (2026-09-27, ISO in the page cache): extract + prepare 4.3 s, verify 3 s, a no-op build
0.1 s, swap 0.2 s; about 8 s in all, plus a few seconds when `randcrw-extract` needs rebuilding. The dev profile builds the
workspace crates at `opt-level = 1`, which is enough: the extractor is I/O-bound, as fast as the release build.
It needs free space for the new data (about 4.4 GiB) while the old folder still exists.

Exit codes: 0 done (or `--check`), 1 refused or failed (the old folder unchanged), 2 bad arguments or no disc image.

**Tests:** `cargo test -p xtask` (argument parsing, the leftover classifier against the committed Tier 0 table, the
safety guard, staging paths). End to end it was run against scratch folders only, with `--data-dir`.
