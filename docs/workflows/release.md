# Workflow: release packaging

Tool: `tools/package/package.sh` (README there), run through `cargo xtask package` (`tools/xtask`). Launcher
contract: `docs/plan/launcher_contract.md`.

| Task | Command |
|---|---|
| Build and package a release | `cargo xtask package` |
| Package the binaries already in `target/release` | `cargo xtask package --no-build` |

`cargo xtask package [args]` runs `tools/package/package.sh [args]` from the repo root; calling the script directly
does the same.

Output: `dist/rerac-<version>-<os>-<arch>/` and its `.zip` (git-ignored). The version is the one workspace version,
`version` under `[workspace.package]` in `Cargo.toml` (every crate has `version.workspace = true`). The build is a
plain `--release` build (never `--features dev`).

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

## Cutting a release

1. Set the version in `Cargo.toml` (`[workspace.package]`, `version = "X.Y.Z"`) and add a `## vX.Y.Z[-pre]` section
   to `CHANGELOG.md`. It is player-facing: plain and short, no build internals. Commit on `main`.
2. Tag that commit with an annotated tag and push the tag:

   ```sh
   git tag -a vX.Y.Z[-pre] -m "ReRAC vX.Y.Z[-pre]"
   git push origin main vX.Y.Z[-pre]
   ```

   The tag's `X.Y.Z` must equal the workspace version (the release workflow checks); the `-pre` part (`-alpha`,
   `-beta`, `-rc…`) exists only in the tag.

## GitHub Actions (`.github/workflows/`)

Both run on `macos-14` (Apple Silicon, the tested platform), install stable Rust with rustup and cache with
`Swatinem/rust-cache`.

* **`ci.yml`** (push to `main`, pull requests): `cargo check-all`, `cargo clippy-all`, then
  `cargo xtask test-full --no-game-data`. The runner has no disc and no `extracted/`, and never gets one. With
  `--no-game-data` the data root is an empty folder: the tests that need game data take their early return, and xtask
  lists each of them as SKIPPED (in the log, as a warning annotation and in the job summary), so none of them passes
  silently. The hero digest needs game data and does not run. The same command works locally (`tools/xtask/README.md`).
* **`release.yml`** (push of a `v*` tag; or by hand with `workflow_dispatch`, input `tag`: an existing tag to release
  again, or empty to only build the package as a workflow artifact): `cargo xtask package`, then a check that the tag
  matches the workspace version, then a GitHub Release for the tag, with `rerac-<version>-macos-arm64.zip` (the
  launcher picks this asset) and `rerac-manifest.json`. It is marked a prerelease when the tag contains `-alpha`,
  `-beta` or `-rc`. The notes are the tag's `CHANGELOG.md` section, else the tag's annotation. Permissions:
  `contents: write`.

Secrets:

* `GITHUB_TOKEN` (automatic) creates the release.
* `SITE_DISPATCH_TOKEN` (optional repository secret): a token that may create `repository_dispatch` events on
  `re-rac/re-rac.github.io` (fine-grained: that repository, Contents read and write). When it exists, the release
  workflow sends the site a `release-published` event (payload: `tag`, `repository`) so the site can rebuild; without
  it the step is skipped.
