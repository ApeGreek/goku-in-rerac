//! Ground-truth harness: read the PS2's EE memory (PCSX2 savestates or the PINE socket) and
//! compare it with what our reimplementation computes. Doc: docs/plan/trace_harness.md.

pub mod ee;
pub mod hero_analysis;
pub mod hero_record;
pub mod hero_replay;
pub mod hero_trace;
pub mod novalis_spawn;
pub mod pine;
pub mod port_sim;
pub mod tfrag_light_cmp;
pub mod tie_shrub_cmp;
pub mod zip;

use std::path::PathBuf;

/// Default `extracted/` directory (the workspace's, git-ignored); override with `--extracted` or `RC_EXTRACTED`.
pub fn default_extracted() -> PathBuf {
    let p = rc_formats::test_data::root();
    p.canonicalize().unwrap_or(p)
}
