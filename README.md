# randcrw

A faithful reimplementation of Ratchet & Clank (2002, PS2) in Rust on Bevy, run from the
owner's own disc, extracted once from their disc image. This is a personal project: it is not
redistributed, and nothing from the disc (assets, code, dumps, savestates) is stored in this
repository. Everything disc-derived lives in git-ignored directories (`extracted/`, `ghidra/`,
`decomp/export/`) or in the user's own data folder.

| Path | What it is |
|---|---|
| `crates/rc-formats` | Disc and level format readers plus bit-exact ports of load-time passes (lighting, animation), golden-tested against the C++ extractor |
| `crates/rc-engine` | Bevy app (the `randcrw` executable): level viewer replaying the game's per-frame render decisions (LOD, culling, fog, sky) |
| `crates/rc-game` | Game logic ports (collision queries so far) |
| `crates/rc-trace` | PCSX2 harness: compares our results with the game's EE RAM (savestate or PINE) |
| `src/core`, `tools/extract` | C++ format oracle and extractor (`rc_extract`), the reference the Rust ports are checked against |
| `decomp/` | Ghidra naming/export scripts and name tables (the exports themselves are ignored) |
| `docs/formats`, `docs/plan` | Format specs, reverse-engineering notes, roadmap and decisions |

## Toolchain

Rust comes from Homebrew's rustup; put it on `PATH` first:

```
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
cargo dev                        # run the engine (Bevy dynamically linked, fast relink)
cargo test -p rc-formats         # unit + golden tests (golden ones skip without extracted/)
cargo run --release -p rc-engine # release / profiling build
```

`cargo dev` and `cargo dev-build` are aliases in `.cargo/config.toml` (`-p rc-engine --features dev`).
The crate is `rc-engine`; its executable is `randcrw` (`target/debug/randcrw`, `target/release/randcrw`).

The C++ extractor (CMake + Ninja):

```
cmake -S . -B build -G Ninja && cmake --build build
./build/tests/rc_tests
./build/tools/extract/rc_extract info
./build/tools/extract/rc_extract unpack   # writes extracted/
```

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
3. the development default: `RC_EXTRACTED`, else `<repo>/extracted` (the C++ `rc_extract unpack`
   tree, same layout).

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

Tests and `rc-trace` read the development `extracted/` tree (`RC_EXTRACTED` overrides it;
`rc_formats::test_data::root()`), because they also use the C++-derived files it holds (`.dec`,
dumps, `core/` and `gameplay/` splits, `overlay.elf`); they skip when it is absent.

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

## PCSX2 harness

With a savestate taken on Novalis (see `docs/plan/trace_harness.md` for the setup):

```
PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo run -p rc-trace -- compare-tfrag-light --state latest --level 01
```

With PINE enabled in PCSX2, `--pine` replaces `--state latest` and reads the running game.

## Reference material (not vendored)

* `~/Globals/wrench`: Wrench (GPL-3), used as format documentation only; all reader code here
  is written from the specs in `docs/formats/`.
* `~/Globals/jak-project`: OpenGOAL (ISC), architectural reference.
* `~/Globals/ghidra-emotionengine-reloaded`: PS2 processor module for Ghidra.
* `~/Globals/ghidra-mcp`: GhidraMCP bridge to the Ghidra project.

See `docs/plan/roadmap.md` for status and milestones and `docs/plan/decisions.md` for the
decisions already taken.
