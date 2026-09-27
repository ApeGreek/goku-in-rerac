# tools/repo-checks: layout guard tests

Dev only, never ships. No dependencies; it exists for its tests. Rules: `docs/plan/repo_reorg.md`.

* **Command:** `cargo test -p repo-checks` (also part of `cargo test --workspace`).
* **Inputs:** the repository tree, read as text. **Outputs:** none.
* **Checks:**
  1. No product crate (`crates/*/Cargo.toml`) depends on anything under `tools/` (every `path = "..."` is resolved,
     and no manifest names a tool package: `rc-trace`, `repo-checks`, `xtask`).
  2. No file under `crates/` mentions personal material or dev output: `.p2s`, `~/PS2`, `PS2/ratchet1`, `work/`,
     `/traces/`, `RC_PERSONAL`. The known exceptions are listed in `src/lib.rs` `KNOWN_OFFENDERS` (today the disc
     golden tests' fallback to the ISO in `~/PS2/ratchet1/`; a follow-up), and a stale entry fails the test.
  3. The top level holds only `crates tools docs extracted work target dist`, dotfiles, `Cargo.toml`, `Cargo.lock` and
     `README.md`.
