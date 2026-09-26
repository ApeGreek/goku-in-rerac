//! Ground-truth harness: read the PS2's EE memory (PCSX2 savestates or the PINE socket) and
//! compare it with what our reimplementation computes. Doc: docs/plan/trace_harness.md.

pub mod ee;
pub mod novalis_spawn;
pub mod pine;
pub mod port_sim;
pub mod tfrag_light_cmp;
pub mod tie_shrub_cmp;
pub mod zip;

use std::path::PathBuf;

/// Default `extracted/` directory (the workspace's, git-ignored); override with `--extracted` or `RC_EXTRACTED`.
pub fn default_extracted() -> PathBuf {
    let p = std::env::var_os("RC_EXTRACTED").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    p.canonicalize().unwrap_or(p)
}
