# Workflow: game data (extract, verify, prepare, export)

Tool: `randcrw-extract` (product crate `crates/rc-extract`; the launcher runs the same binary). Layout of a data
folder: `docs/plan/launcher_extractor.md` §4.1. All commands run from the repo root with
`export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`; `RC_ISO` can stand in for `--iso`.

## The commands

| Task | Command |
|---|---|
| Identify a disc image | `cargo run --release -p rc-extract -- identify --iso "<disc>.iso"` |
| Extract (Tier 0 archive, then the engine cache) | `cargo run --release -p rc-extract -- extract --iso "<disc>.iso" --out <data folder>` |
| Verify a data folder against the built-in size/SHA-1 table | `cargo run --release -p rc-extract -- verify --out <data folder>` |
| Rebuild the engine cache (`<data folder>/cache/v1`) | `cargo run --release -p rc-extract -- prepare --out <data folder>` |
| Export usable formats (PNG, WAV, glTF, JSON) | `cargo run --release -p rc-extract -- export --out <data folder> --to work/exports` |

`export` defaults to `<data folder>/exports/`; for dev work send it to `work/exports/` (git-ignored) so `extracted/`
stays game data only.

## The development data folder: `extracted/`

Tests, `rc-trace` and `cargo dev` read `<repo>/extracted` (`RC_EXTRACTED` overrides it). **The one command to
(re)build it:**

```
cargo xtask regen-data
```

It needs the disc image: set `RC_ISO` once (fish: `set -Ux RC_ISO "<disc>.iso"`; zsh / bash: an
`export RC_ISO="<disc>.iso"` line in `~/.zshrc` / `~/.bashrc`; without it the command prints these lines with your
disc's path), or pass `--iso <disc>.iso`. It lists what `extracted/` holds, builds the dev `randcrw-extract`, extracts
into `work/data-staging`, verifies it, and only then swaps it in and deletes the old folder, so a failure never
leaves you without data. About 8 s; the new data is about 4.4 GiB (3,072 files: 2,937 Tier 0 files,
`extract-info.json`, a 438 MiB engine cache). `cargo xtask regen-data --check` only lists what the folder holds.
Details and exit codes: `tools/xtask/README.md`.

The retired C++ extractor's leftovers (`.dec` copies, `*_dump.bin`, `core/` / `gameplay/` / `textures/` splits,
PNG / OBJ previews, `overlay.elf` / `overlay.txt`, `vu/`) are read by nothing; the overlay ELFs and VU listings were
copied to `work/ghidra-import/` and `work/vu/` on 2026-09-27. `regen-data` recognises them, lists their counts and
deletes them with the old folder. Any other file stops it (move it to `work/` or `~/PS2/ratchet1/`, or pass
`--force` to delete it).

**What may live in `extracted/`:** only what `randcrw-extract` writes: the Tier 0 archive (`toc.bin`, `boot/`,
`global/`, `levels/NN/*.bin` and their `bindata/ music/ speech/ scene/` folders, `extract-info.json`) and the engine
cache `cache/`. **What may not:** trace dumps, savestates, reports, captures, exports, Ghidra material, anything a dev
tool writes. Those go to `work/` (generated) or `~/PS2/ratchet1/` (personal).

## Players

Players never run these by hand: the launcher (docs/workflows/launcher.md) runs `extract` into its own data folder
and starts the game with `--data-dir`.
