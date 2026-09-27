# Workflow: release packaging

Tool: `tools/package/package.sh` (README there). Launcher contract: `docs/plan/launcher_contract.md`.

| Task | Command |
|---|---|
| Build and package a release | `tools/package/package.sh` |
| Package the binaries already in `target/release` | `tools/package/package.sh --no-build` |

Output: `dist/randcrw-<version>-<os>-<arch>/` and its `.zip` (git-ignored). The version comes from
`crates/rc-engine/Cargo.toml`. The build is a plain `--release` build (never `--features dev`).

## The allow-list

The script packages exactly these and fails if anything else ends up in the folder:

* `randcrw` (crate `rc-engine`) and `randcrw-extract` (crate `rc-extract`),
* `assets/shaders/*.wgsl`,
* `randcrw-manifest.json` (the launcher manifest),
* `README.txt`.

So nothing from `tools/`, `extracted/`, `work/` or `~/PS2/ratchet1/` can ship, and no disc data can be packaged. The
script also runs the packaged `randcrw --version-json` to check it.

macOS is tested; the Linux and Windows (Git Bash) branches are written but untested. No signing yet.
