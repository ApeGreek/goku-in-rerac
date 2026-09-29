# Ratchet & Clank: ReWrite (randcrw)

## Please read first

> [!IMPORTANT]
> This repository is for developing randcrw. It is not how you play it. Players will install and run the game
> through the randcrw launcher, a separate project. Neither the game nor the launcher has been released yet, so there
> are no downloads (coming later).

> [!WARNING]
> **You need your own, legally obtained PlayStation 2 disc of Ratchet & Clank (2002).** This project contains no game
> assets and no copy of the game's code. You extract the game data yourself, from an image of your own disc. The
> committed verification tables hold only sizes, counts and hashes.
>
> Supported right now: **NTSC-U, SCUS-97199, version 1.00, only.** The extractor identifies the disc by its serial and
> the SHA-1 of its boot executable and refuses anything else, including:
> - PAL (SCES-50916) and NTSC-J (SCPS-15037) discs, and the demo discs;
> - a Greatest Hits disc whose boot executable differs from v1.00 (Greatest Hits discs share the serial SCUS-97199);
> - the later releases: the PS3 HD collection, the PS4 and PS Now versions, and the 2016 game;
> - the sequels (they are recognised by name and refused).
>
> randcrw is an unofficial fan project. It is not affiliated with or endorsed by Sony Interactive Entertainment or
> Insomniac Games. Ratchet & Clank and all related names are trademarks of their respective owners.

## Table of contents

- [Please read first](#please-read-first)
- [Project description](#project-description)
  - [Strategy](#strategy)
  - [Objectives](#objectives)
  - [Platforms](#platforms)
- [Current status](#current-status)
- [Methodology](#methodology)
- [Setting up a development environment](#setting-up-a-development-environment)
  - [Requirements](#requirements)
  - [Building and running the game](#building-and-running-the-game)
  - [Testing](#testing)
  - [Development switches](#development-switches)
  - [Other dev chores](#other-dev-chores)
- [Technical project overview](#technical-project-overview)
- [License](#license)

## Project description

randcrw is a native rewrite of Ratchet & Clank (2002, PlayStation 2) in Rust, on the Bevy engine. The game's systems
are rebuilt from a decompilation of the original code and run as native code. It is not an emulator, not a static
recompiler, and it does not model the PS2 hardware.

### Strategy

1. Decompile the game in Ghidra: the boot executable and each level's code overlay, with the names we derive kept in
   committed tables.
2. Port the game's shared systems natively, matching the original's behaviour and values. Every port cites the
   addresses of the functions it reproduces.
3. Write our own extractor, `randcrw-extract`, that turns an image of your disc into a data folder the engine reads
   directly. The engine never reads the disc image itself.
4. Build a launcher (Tauri, separate repository) that installs game versions, runs the extractor once and starts the
   game. Mod support is planned on top of it.

### Objectives

- **Native and fast.** No emulation layer: custom Bevy render pipelines with WGSL shaders derived from the game's VU1
  programs, and game logic in plain Rust.
- **Faithful.** The port must behave like the original: what the player sees, hears and feels. Where the PS2
  hardware causes a visible effect, the port reproduces the result natively. Bit-exact comparison is a diagnostic
  tool, not the goal (the native-first policy in [`docs/plan/decisions.md`](docs/plan/decisions.md)).
- **Shared the way the original shares it.** A system the game uses on many levels is ported once and reused wherever
  the game uses it, not copied per level.
- **Mods, later.** Through the launcher; the design is in [`docs/plan/mods.md`](docs/plan/mods.md).

### Platforms

- **macOS on Apple Silicon:** the development platform and the only one tested.
- **Windows and Linux:** planned, untested. The code has their settings paths and the packaging script has branches
  for them, but neither has been built or run there.

## Current status

Early, in active development, and not playable from start to finish.

- **All 19 levels load and render:** terrain, ties and shrubs with the game's own LOD, culling and lighting; the sky;
  animated objects ("mobys"); fog, occlusion, particles and shadows.
- **Novalis is the most complete level.** On the other levels a growing share of the objects run their ported update
  code. The latest census (2026-09-29) counts about 2,955 created object instances, in 375 port units, that still
  run no update ([`docs/plan/class_census.md`](docs/plan/class_census.md)).
- **In the port:** Ratchet's movement and hero states, the wrench, the weapons and gadgets; the sea and water; the
  pause menu, the HUD and the in-game map; the Gadgetron vendor; in-engine cutscenes and the FMV movies (our own
  MPEG-2 decoder); sound effects and music.
- **Missing:** planet travel and the ship; the main menu and saving / loading; the Clank, Giant Clank and Hologuise
  sections; the hoverboard and races; ship combat; checking jump and landing timing against the original.

Every known gap is listed by missing system in [`docs/plan/gaps.md`](docs/plan/gaps.md), and
[`docs/plan/priority_order.md`](docs/plan/priority_order.md) orders them.

## Methodology

- **Decompilation.** The boot executable and the 19 level overlays are separate programs in one Ghidra project, with a
  PS2 Emotion Engine processor module. Names live in `tools/ghidra/names/` and are applied by script; the first boot
  function names came from Lombyte's splat configuration. The decompiled C is exported to `work/decomp/`, which is
  git-ignored and never committed. See [`docs/workflows/ghidra.md`](docs/workflows/ghidra.md) and
  [`docs/plan/decomp_workflow.md`](docs/plan/decomp_workflow.md).
- **Per-level code identity.** Each level overlay carries its own compiled copy of the engine and hero code, linked at
  other addresses. A masked comparison (relocated jump targets, address halves and `$gp` offsets hidden) decides
  whether a level's copy is the same code as the port's reference. A port then runs on every level whose code
  matches, and the levels that differ are listed. `cargo run -p rc-trace -- overlay-diff` answers this for any
  function. See [`docs/plan/level_generalisation.md`](docs/plan/level_generalisation.md).
- **PCSX2 traces.** `rc-trace` reads the game's memory from PCSX2 savestates, or live over PINE, and compares it with
  the port. Findings reach the tests only as small, numbers-only fixtures. See
  [`docs/workflows/pcsx2.md`](docs/workflows/pcsx2.md) and [`docs/plan/trace_harness.md`](docs/plan/trace_harness.md).
- **Coverage tables.** A port's plan doc lists every call and branch of the functions it ported, each marked as
  ported (with the file and function), a gap ID, or not applicable with a reason.
- **Tests without personal files.** The loaders are golden-tested on all 19 levels against committed hash tables.
  Tests read only the extracted game data and committed fixtures, never the disc image, savestates or recordings; a
  guard test enforces this. See [`docs/workflows/testing.md`](docs/workflows/testing.md).
- **References.** Wrench (GPL-3) is used as format documentation only; every reader here is written from the specs in
  [`docs/formats/`](docs/formats/). OpenGOAL is the architectural model: decompiled game code on a rewritten runtime.

## Setting up a development environment

### Requirements

- **Rust**, stable, through rustup ([rustup.rs](https://rustup.rs)). On macOS, Homebrew's rustup also works; it is
  keg-only, so put it on `PATH` first:

  ```sh
  brew install rustup
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
  rustup default stable
  ```

- **An image of your disc** (`.iso`), and about 4.4 GiB of free space for the extracted data.
- **Optional:** [cargo-nextest](https://nexte.st) (the test commands use it when installed) and cargo-sweep (keeps
  `target/` small):

  ```sh
  cargo install cargo-nextest --locked
  cargo install cargo-sweep --locked
  ```

- **Optional, for research:** Ghidra with the Emotion Engine processor module and the GhidraMCP plugin, plus
  Python 3 for the scripts ([`docs/workflows/ghidra.md`](docs/workflows/ghidra.md)); PCSX2 with PINE enabled
  ([`docs/workflows/pcsx2.md`](docs/workflows/pcsx2.md)).
- **Optional, for the launcher:** Node and npm; see [`docs/workflows/launcher.md`](docs/workflows/launcher.md).

Any editor with rust-analyzer works; nothing in the repository depends on one.

### Building and running the game

Run everything from the repository root.

1. **Check your disc image.** This prints the serial, region and version, and whether it is supported:

   ```sh
   cargo run --release -p rc-extract -- identify --iso "<path to your disc image>.iso"
   ```

2. **Extract the game data** into `extracted/`, the development data folder (git-ignored). Set `RC_ISO` once, then:

   ```sh
   export RC_ISO="<path to your disc image>.iso"
   cargo xtask regen-data
   ```

   This builds `randcrw-extract`, extracts into a staging folder, verifies every file's size and SHA-1, and only then
   swaps the result in. It takes about 8 seconds on the development machine. `--iso <image>` works instead of
   `RC_ISO`. To extract into another folder with the extractor directly:

   ```sh
   cargo run --release -p rc-extract -- extract --iso "<path to your disc image>.iso" --out <data folder>
   ```

   `--ntsc-only` skips the PAL copies the disc also carries (about 1.4 GiB the game never reads). The other
   subcommands (`verify`, `prepare`, `export`) are in [`docs/workflows/game-data.md`](docs/workflows/game-data.md).

3. **Build and run:**

   ```sh
   cargo dev
   ```

   `cargo dev` is an alias for `cargo run -p rc-engine --features dev`, which links Bevy dynamically so a rebuild
   relinks in seconds. The first build compiles Bevy and takes a few minutes. It starts on Novalis;
   `RC_LEVEL=<n> cargo dev` picks another level. For another data folder: `cargo dev -- --data-dir <data folder>`.

   A release or profiling build: `cargo run --release -p rc-engine`. The crate is `rc-engine`; its executable is
   `randcrw` (`target/debug/randcrw`, `target/release/randcrw`).

### Testing

Tests run only through `cargo xtask`, never through `cargo test` or `cargo nextest` by hand. The xtask commands pass
the flags that reuse the `cargo dev` Bevy build; tests that need game data skip when `extracted/` is absent.

```sh
cargo xtask test-quick                # unit tests of every crate (about 5 s)
cargo xtask test-job hero world       # a job's tier: unit tests plus those areas' integration tests
cargo xtask test-full                 # the full suite
cargo check-all                       # type check everything, tests included
cargo clippy-all                      # lints
```

The areas, the tiers, how to run one test, and the hero digest are in
[`docs/workflows/testing.md`](docs/workflows/testing.md) and [`tools/xtask/README.md`](tools/xtask/README.md).

### Development switches

The engine reads `RC_*` environment variables for debugging. The ones used most:

| Variable | Effect |
|---|---|
| `RC_LEVEL` | Level index to load (default 1, Novalis) |
| `RC_GIVE_ITEMS` | `id,id,…`: own those items from the start, and equip the last one per slot |
| `RC_GIVE_BOLTS` | `n`: start with n bolts |
| `RC_HERO_AT` | `x,y,z[,yaw]`: place Ratchet there at the level load |
| `RC_CAM` | Starting camera `ex,ey,ez,tx,ty,tz` (eye and target) |
| `RC_SCREENSHOT` | Save a screenshot to this path, then exit |
| `RC_DUMP_FRAMES` | `start..end`: save those frames as PNGs (frame-exact), then exit |
| `RC_AUDIO` | `0`: no audio |

The full table, the command-line options and the settings file are in
[`docs/workflows/dev-switches.md`](docs/workflows/dev-switches.md).

### Other dev chores

```sh
cargo xtask help                      # list the dev chores
cargo xtask sweep                     # keep target/ under 30 GB (needs cargo-sweep; --dry-run, --limit 20G)
cargo xtask package                   # release build of randcrw + randcrw-extract, packaged for the launcher
```

Packaging: [`docs/workflows/release.md`](docs/workflows/release.md). Running a local build from the launcher:
[`docs/workflows/launcher.md`](docs/workflows/launcher.md).

## Technical project overview

```
crates/          the product; it ships, and never depends on tools/
  rc-formats       readers for the disc and level formats, golden-tested against committed hashes
  rc-data          the engine cache: decompressed game data on disk, served once per process
  rc-game          the game logic ports (plain Rust, no Bevy)
  rc-engine        the Bevy app; executable `randcrw`
  rc-extract       the extractor; executable `randcrw-extract` (identify, extract, verify, prepare, export)
  rc-video         our own MPEG-2 decoder and PSS movie player core
tools/           dev only, never ships; one README each
  trace            `rc-trace`: the PCSX2 harness and the overlay diff
  ghidra           Ghidra import, naming and export scripts, and the name tables
  package          release packaging
  repo-checks      guard tests for the repository layout and rules
  xtask            `cargo xtask <command>`: one short command per dev chore
docs/
  formats/         format specs
  plan/            investigations, decisions, roadmap, gaps
  workflows/       one page per dev workflow
extracted/       (git-ignored) game data only, as randcrw-extract writes it
work/            (git-ignored) generated dev output: decompiler export, traces, captures, exports
dist/, target/   (git-ignored) release packages, build output
```

Data has four homes: source in the repository; game data, rebuilt from your disc, in `extracted/`; generated dev
output, rebuilt by a tool, in `work/`; and personal material (the disc image, the Ghidra project, savestates,
recordings) in a folder of your own outside the repository. Only source and game data may feed tests. Details:
[`docs/plan/repo_reorg.md`](docs/plan/repo_reorg.md).

Where to read next:

- [`docs/plan/roadmap.md`](docs/plan/roadmap.md): milestones and what each system does in the port.
- [`docs/plan/decisions.md`](docs/plan/decisions.md): the decisions taken, and why.
- [`docs/plan/gaps.md`](docs/plan/gaps.md) and [`docs/plan/priority_order.md`](docs/plan/priority_order.md): what is
  missing, and in what order.
- [`docs/plan/launcher_contract.md`](docs/plan/launcher_contract.md): the interface between the launcher, the
  extractor and the game.
- [`docs/plan/launcher_extractor.md`](docs/plan/launcher_extractor.md): the data folder layout and the extractor's
  design.
- [`docs/workflows/`](docs/workflows/): game data, testing, Ghidra, PCSX2, release, launcher, dev switches.

## License

All rights reserved for now. The workspace's `Cargo.toml` declares `license = "UNLICENSED"`: no licence is granted.
