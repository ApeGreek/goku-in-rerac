# Workflow: release packaging

Tool: `tools/package/package.sh` (README there), run through `cargo xtask package` (`tools/xtask`). Launcher
contract: `docs/plan/launcher_contract.md`.

| Task | Command |
|---|---|
| Build and package a release | `cargo xtask package` |
| Package the binaries already in `target/release` | `cargo xtask package --no-build` |

`cargo xtask package [args]` runs `tools/package/package.sh [args]` from the repo root; calling the script directly
does the same.

Output: `dist/rerac-<version>-<os>-<arch>/` and its `.zip` (git-ignored). The version comes from
`crates/rc-engine/Cargo.toml`. The build is a plain `--release` build (never `--features dev`).

## The allow-list

The script packages exactly these and fails if anything else ends up in the folder:

* `rerac` (crate `rc-engine`) and `rerac-extract` (crate `rc-extract`),
* `assets/shaders/*.wgsl`,
* `rerac-manifest.json` (the launcher manifest),
* `README.txt`.

So nothing from `tools/`, `extracted/`, `work/` or `~/PS2/ratchet1/` can ship, and no disc data can be packaged. The
script also runs the packaged `rerac --version-json` to check it.

macOS is tested; the Linux and Windows (Git Bash) branches are written but untested. No signing yet: a downloaded zip is
quarantined by macOS Gatekeeper.

## Where the runtime finds its shaders

The runtime loads its shaders from `assets/` next to its (symlink-resolved) executable, or from `../Resources/assets`
in a macOS `.app`; when neither exists (`cargo dev`, `target/*/rerac`) it uses the repo's `crates/rc-engine/assets`
(`main.rs` `asset_dir`).
