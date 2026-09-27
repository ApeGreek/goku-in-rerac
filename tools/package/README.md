# tools/package: release packaging

Dev only, never ships itself. Workflow: `docs/workflows/release.md`.

* **Purpose:** builds a launcher version folder of the game (launcher contract: `docs/plan/launcher_contract.md`) and
  zips it.
* **Command:** `tools/package/package.sh` (release build, then package); `--no-build` packages the binaries already in
  `target/release`.
* **Inputs:** `target/release/randcrw` and `randcrw-extract`, `crates/rc-engine/assets/shaders/*.wgsl`, the version in
  `crates/rc-engine/Cargo.toml`.
* **Outputs:** `dist/randcrw-<version>-<os>-<arch>/` and its `.zip` (git-ignored). The folder is checked against an
  exact allow-list (the two binaries, `assets/shaders/*.wgsl`, `randcrw-manifest.json`, `README.txt`), so nothing
  from `tools/`, `extracted/` or `work/` can be packaged.
