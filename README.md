# randcre

A faithful reimplementation of Ratchet & Clank (2002, PS2) in Rust on Bevy, run from the
owner's own disc image. This is a personal project: it is not redistributed, and nothing from
the disc (assets, code, dumps, savestates) is stored in this repository. Everything
disc-derived lives in git-ignored directories (`extracted/`, `ghidra/`, `decomp/export/`).

| Path | What it is |
|---|---|
| `crates/rc-formats` | Disc and level format readers plus bit-exact ports of load-time passes (lighting, animation), golden-tested against the C++ extractor |
| `crates/rc-engine` | Bevy app: level viewer replaying the game's per-frame render decisions (LOD, culling, fog, sky) |
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

The C++ extractor (CMake + Ninja):

```
cmake -S . -B build -G Ninja && cmake --build build
./build/tests/rc_tests
./build/tools/extract/rc_extract info
./build/tools/extract/rc_extract unpack   # writes extracted/
```

## Disc image

The engine reads the level straight from the disc image: `RC_ISO` if set, else the default
`~/PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso` (NTSC-U, SCUS-97199). Without an
image it falls back to the files `rc_extract unpack` wrote to `extracted/`. The Rust disc reader
is golden-tested byte for byte against `extracted/` for all 19 levels, so both sources give the
same bytes.

## Engine environment switches

Boolean switches are on with `1` (or off with `0` where the default is on).

| Variable | Effect |
|---|---|
| `RC_ISO` | Path of the disc image (default above) |
| `RC_SOURCE` | `iso` or `extracted`: force the data source (default: image if found, else `extracted/`) |
| `RC_EXTRACTED` | Root of the extracted tree (default `<repo>/extracted`) |
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
