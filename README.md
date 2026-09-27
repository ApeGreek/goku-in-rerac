# randcrw

A faithful reimplementation of Ratchet & Clank (2002, PS2) in Rust on Bevy, run from the
owner's own disc, extracted once from their disc image. This is a personal project: it is not
redistributed, and nothing from the disc (assets, code, dumps, savestates) is stored in this
repository; the committed verification tables hold only sizes, counts and hashes.

## Repository map

```
crates/                 PRODUCT (ships; never depends on tools/)
  rc-formats              disc and level format readers, bit-exact load-time passes, golden-tested
                          against committed snapshot hashes (data/loader_snapshots.tsv)
  rc-data                 Tier 1 engine cache
  rc-game                 game logic ports (pure Rust, no Bevy)
  rc-engine               Bevy app, executable `randcrw`
  rc-extract              `randcrw-extract`: extract / verify / prepare / export
tools/                  DEV ONLY (never ships; one README each)
  ghidra/scripts, names   Ghidra import / naming / export scripts and the name tables
  trace                   PCSX2 harness (package `rc-trace`): the port vs the game's EE RAM
  package                 release packaging (package.sh)
  repo-checks             guard tests for this layout
docs/                   formats/ (format specs), plan/ (investigations, roadmap, decisions),
                        workflows/ (one page per dev workflow: ghidra, pcsx2, game-data, release, launcher)
extracted/   (ignored)  game data only, as randcrw-extract writes it
work/        (ignored)  generated dev output: decomp/, trace/, ghidra-import/, vu/, exports/, captures/
dist/, target/ (ignored) release packages, build output
~/PS2/ratchet1/         outside the repo: the ISO, savestates/, traces/ (recordings), the Ghidra project's
                        future home ghidra/ (today ~/ratchet1.gpr + ~/ratchet1.rep)
```

Four kinds of data, four homes:

| Kind | Home | In git |
|---|---|---|
| Source: code, docs, Ghidra scripts, name lists, committed expected values (hash tables, distilled fixtures) | the repo | yes |
| Game data, rebuildable from the ISO | `extracted/` (dev; players: the launcher's data folder) | no |
| Generated dev output, rebuildable by a tool | `work/` | no |
| Personal material: the ISO, the Ghidra project, savestates, recordings | `~/PS2/ratchet1/` | no |

Only source and game data may feed tests; personal material never does (PCSX2 findings reach tests as distilled,
numbers-only fixtures). `tools/repo-checks` guards the product side. Layout and rules: `docs/plan/repo_reorg.md`.

## Toolchain

Rust comes from Homebrew's rustup; put it on `PATH` first:

```
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
cargo dev                        # run the engine (Bevy dynamically linked, fast relink)
cargo test --workspace           # all tests (the ones that need game data skip without extracted/)
cargo run --release -p rc-engine # release / profiling build
```

`cargo dev` and `cargo dev-build` are aliases in `.cargo/config.toml` (`-p rc-engine --features dev`).
The crate is `rc-engine`; its executable is `randcrw` (`target/debug/randcrw`, `target/release/randcrw`).

The loaders' golden tests (`crates/rc-formats/tests/golden.rs`) compare what the Rust loaders produce for all 19
levels with `crates/rc-formats/data/loader_snapshots.tsv` (per test, level and section: item count, byte count,
SHA-1). The table was generated while the output was byte-identical to the C++ reference extractor, retired on
2026-09-27 (`docs/plan/decisions.md`). After an intended loader change, `RC_SNAPSHOT_WRITE=1 cargo test -p
rc-formats --test golden` rewrites the rows of the tests that ran.

## Game data

The engine never reads the disc image. The user extracts their own disc once with
`randcrw-extract` (`crates/rc-extract`; the launcher runs it), and the game then reads only that
data folder (layout: `docs/plan/launcher_extractor.md` §4.1; contract:
`docs/plan/launcher_contract.md`):

```
cargo run --release -p rc-extract -- extract --iso "<your disc>.iso" --out <data folder>
cargo dev -- --data-dir <data folder>     # or RC_DATA_DIR=<data folder> cargo dev
cargo dev -- --version-json               # {"name":"randcrw","version":"0.1.0","game":"rac1","data_format":1}
```

The data folder is chosen in this order:

1. `--data-dir <folder>` (also `--data-dir=<folder>`), how the launcher starts the game;
2. `RC_DATA_DIR`;
3. the development default: `RC_EXTRACTED`, else `<repo>/extracted` (a `randcrw-extract`
   data folder, same layout).

The folder must exist and hold `toc.bin`, and for 1 and 2 also the extractor's
`extract-info.json` with a matching `data_format` (the development tree may lack it: one
warning). Otherwise the engine prints one `error:` line saying what to do (re-extract via the
launcher) and exits before opening a window: code 3 (folder missing, not a data folder, or
extraction incomplete), code 4 (data format mismatch or unreadable `extract-info.json`), code 2
(`--data-dir` without a value). `--version-json` prints the contract line and exits without
touching any data.

**Engine cache (Tier 1).** `extract` ends by building `<data folder>/cache/v1/` (stage `prepare`,
about 0.2 s, 440 MiB): every level's core data, gameplay file and HUD banks, WAD-decompressed once,
plus `stamp.toml`. `randcrw-extract prepare --out <data folder>` rebuilds or refreshes it from the
archive alone (no disc). The engine reads lumps through `rc-data` (once per process each); when the
cache is missing, stale (stamp: cache version, converter versions, `extract-info.json` hash) or a
lump is corrupt (XXH64 trailer), it rebuilds what it needs on the fly with one log line per lump,
and falls back to in-memory decompression when the folder cannot be written. The development
`extracted/` tree therefore gets `extracted/cache/v1/` on the first `cargo dev`. The folder can be
deleted at any time. `RC_CACHE=0` skips it; `RC_PERF_LOG=1` prints where each lump came from.

**Exports (Tier 2, optional).** `randcrw-extract export --out <data folder> [--to <dir>] [--what
textures,audio,models,levels,collision,text|all] [--level NN]` writes usable formats from the archive (no disc):
indexed PNG textures with JSON sidecars (original CLUT and GS format), WAV audio with loop points, glTF 2.0
levels, collision and skinned, animated moby models, and JSON tables and text. Default folder `<data
folder>/exports/`; the full disc is about 60,000 files and 3.5 GiB in about 10 s. The game never reads them; the
launcher's "Export assets…" runs the same command. Layout and what is lossy: `docs/plan/launcher_extractor.md`
§5.5; how mods will use them: `docs/plan/mods.md` §2.1a.

Tests and `rc-trace` read the development `extracted/` tree (`RC_EXTRACTED` overrides it;
`rc_formats::test_data::root()`); they read only Tier 0 files, so any `randcrw-extract` data folder works, and they
skip when it is absent. Decompressed lumps, core blocks (`moby_class/NNNN`, `ratchet_seq/NNN`, …) and gameplay
sections (`level_settings`, …) come from the Rust loaders (`rc_formats::test_data`). An `extracted/` tree written by
the retired C++ extractor still works; its extra files (`.dec`, `*_dump.bin`, `core/` and `gameplay/` splits,
`overlay.elf`, PNG/OBJ previews, `vu/`) are no longer read; `docs/workflows/game-data.md` regenerates a clean tree.

Port settings (MSAA, the "Port Options" page) live in `~/Library/Application Support/randcrw/settings.toml`
(macOS), `$XDG_CONFIG_HOME/randcrw/` (Linux) or `%APPDATA%\randcrw\` (Windows); an older
`randcre/settings.toml` there is copied over once on the first start. `RC_SETTINGS_FILE` overrides.

## Packaging

```
tools/package/package.sh              # release build of randcrw + randcrw-extract, then package
tools/package/package.sh --no-build   # package the binaries already in target/release
```

It writes `dist/randcrw-<version>-<os>-<arch>/` and a `.zip` of it (`dist/` is git-ignored). The folder is a
launcher version (`docs/plan/launcher_contract.md`): `randcrw`, `randcrw-extract`, `assets/shaders/*.wgsl`,
`randcrw-manifest.json` (version from `crates/rc-engine/Cargo.toml`) and `README.txt`. The script fails if anything
else ends up in the folder, so no disc data can be packaged, and it checks the packaged `randcrw --version-json`.
The build is a plain `--release` build without `--features dev`. macOS is tested; the Linux and Windows (Git Bash)
branches are written but untested. No signing yet: a downloaded zip is quarantined by macOS Gatekeeper.

The runtime loads its shaders from `assets/` next to its (symlink-resolved) executable, or from
`../Resources/assets` in a macOS `.app`; when neither exists (`cargo dev`, `target/*/randcrw`) it uses the repo's
`crates/rc-engine/assets` (`main.rs` `asset_dir`).

## Engine environment switches

Boolean switches are on with `1` (or off with `0` where the default is on).

| Variable | Effect |
|---|---|
| `RC_DATA_DIR` | Game data folder when `--data-dir` is not given (see "Game data") |
| `RC_EXTRACTED` | Development data tree (default `<repo>/extracted`); the engine's fallback when neither `--data-dir` nor `RC_DATA_DIR` is set, and the tests' root |
| `RC_CACHE` | `0`: do not use or write the engine cache `<data>/cache/v1` (decompress in memory, once per process) |
| `RC_PERF_LOG` | `1`: print one line per game-data lump request (engine cache, memory or decompressed; MiB, ms) |
| `RC_LEVEL` | Level index to load (default 1, Novalis) |
| `RC_CAM` | Starting camera `ex,ey,ez,tx,ty,tz` (eye and target, game units) |
| `RC_SCREENSHOT` | Save a screenshot to this path, then exit |
| `RC_SCREENSHOT_DELAY` | Seconds before the screenshot (default 3) |
| `RC_NOVSYNC` | `1`: present without vsync, so `fps:` measures headroom |
| `RC_FOG` | `0`: disable fog |
| `RC_NO_LIGHT` | `1`: skip the load-time lighting passes (tfrag, tie, shrub, moby); stored colours used |
| `RC_LOD` | `0`: force tfrag LOD 0 |
| `RC_LOD_TINT` | `1`: tint tfrag LOD 1 red, LOD 2 blue, clipping path green |
| `RC_TIE_LOD` | `0`: force tie LOD 0 with morph k = 0 (culling unchanged) |
| `RC_TIE_LOD_TINT` | `1`: tint tie LOD 1 red, LOD 2 blue |
| `RC_NO_TIES` | `1`: do not draw ties |
| `RC_NO_SHRUBS` | `1`: do not draw shrubs |
| `RC_SKY_ROT` | Sky rotation speed in ticks per 60 Hz tick (`0` freezes; default 1) |
| `RC_ANIM` | `0`: disable moby animation (bind pose) |
| `RC_MOBY_CPU_LIGHT` | `1`: use the bit-exact CPU moby lighting (bind pose) instead of the GPU path |
| `RC_MOBY_LIGHT_CHECK` | `1`: compare GPU vs bit-exact CPU moby lighting at load and report |
| `RC_OCCL` | `0`: occlusion off; `1`: freeze the mask built from the starting camera |
| `RC_OCCL_STATS` | `1`: print occlusion cell and cull counts, at most once a second |
| `RC_GIVE_HYDROPACK` | `1`: own the Hydro-Pack (item 4) from the start (debug; the swim code reads it) |
| `RC_GIVE_ITEMS` | `id,id,…` (decimal or `0x` hex): own those items from the start (debug); the last back item among them (2 Heli-Pack, 3 Thruster-Pack, 4 Hydro-Pack) is the saved back item Clank wears, and the last hand item (e.g. 12, the Swingshot) is requested into the hand |
| `RC_HERO_AT` | `x,y,z[,yaw]`: place Ratchet there at the level load, before the hero init's ground snap (debug; e.g. on a grind rail with `RC_GIVE_ITEMS=29`) |

## Dev workflows

One page each in `docs/workflows/`: `ghidra.md` (the Ghidra project: set-up, naming, decompiler export),
`pcsx2.md` (savestates, PINE, recording and comparing with `rc-trace`), `game-data.md` (extract / verify / prepare /
export, regenerating the dev `extracted/`), `release.md` (packaging) and `launcher.md` (the launcher's dev setup).
For example, with a savestate taken on Novalis:

```
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo run -p rc-trace -- compare-tfrag-light --state latest --level 01
```

## Reference material (not vendored)

* `~/Globals/wrench`: Wrench (GPL-3), used as format documentation only; all reader code here
  is written from the specs in `docs/formats/`.
* `~/Globals/jak-project`: OpenGOAL (ISC), architectural reference.
* `~/Globals/ghidra-emotionengine-reloaded`: PS2 processor module for Ghidra.
* `~/Globals/ghidra-mcp`: GhidraMCP bridge to the Ghidra project.

See `docs/plan/roadmap.md` for status and milestones and `docs/plan/decisions.md` for the
decisions already taken.
