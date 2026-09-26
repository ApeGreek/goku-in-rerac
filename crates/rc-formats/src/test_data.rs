//! Where tests and dev tools find the development data tree: `RC_EXTRACTED`, else `<workspace>/extracted`
//! (the C++ `rc_extract unpack` output; git-ignored, never shipped).
//!
//! Tests read this tree, not a launcher data folder, because they also use the C++-derived files it holds and the
//! Tier 0 archive does not (`.dec` copies, `*_dump.bin` goldens, the `core/` and `gameplay/` splits,
//! `overlay.elf`). Tests skip when a file they need is absent. The game runtime resolves its own data folder
//! (`rc-engine` `disc_source`: `--data-dir`, `RC_DATA_DIR`, then this tree).

use std::path::PathBuf;

/// The development data root: `RC_EXTRACTED` (when set and non-empty), else `<workspace>/extracted`.
pub fn root() -> PathBuf {
    std::env::var_os("RC_EXTRACTED")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"))
}
