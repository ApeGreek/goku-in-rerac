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

Tests, `rc-trace` and `cargo dev` read `<repo>/extracted` (`RC_EXTRACTED` overrides it). Regenerate it from scratch:

```
rm -rf extracted && cargo run --release -p rc-extract -- extract --iso ~/PS2/ratchet1/"Ratchet & Clank (USA) (En,Fr,De,Es,It).iso" --out extracted
```

About 4 GiB in seconds. Nothing else in `extracted/` needs keeping: the retired C++ extractor's leftovers
(`.dec` copies, `*_dump.bin`, `core/` / `gameplay/` / `textures/` splits, previews, `overlay.elf`, `vu/`) are read by
nothing; the overlay ELFs and VU listings were copied to `work/ghidra-import/` and `work/vu/` on 2026-09-27.

**What may live in `extracted/`:** only what `randcrw-extract` writes: the Tier 0 archive (`toc.bin`, `boot/`,
`global/`, `levels/NN/*.bin` and their `bindata/ music/ speech/ scene/` folders, `extract-info.json`) and the engine
cache `cache/`. **What may not:** trace dumps, savestates, reports, captures, exports, Ghidra material, anything a dev
tool writes. Those go to `work/` (generated) or `~/PS2/ratchet1/` (personal).

## Players

Players never run these by hand: the launcher (docs/workflows/launcher.md) runs `extract` into its own data folder
and starts the game with `--data-dir`.
