//! Repository-layout guards (dev only, never ships; tests in `tests/guards.rs`, doc `tools/repo-checks/README.md`
//! and `docs/plan/repo_reorg.md`). Plain text reads, no dependencies.
//!
//! 1. No product crate (`crates/*/Cargo.toml`) depends on anything under `tools/`.
//! 2. No file under `crates/` names personal or dev-output paths ([`PERSONAL_PATTERNS`]), apart from the
//!    listed known lines ([`KNOWN_OFFENDERS`], each a documented follow-up).
//! 3. The top level holds only the agreed folders ([`TOP_LEVEL_DIRS`]), dotfiles and the Cargo / README files.

use std::path::{Component, Path, PathBuf};

/// The repository root (this crate lives at `tools/repo-checks`).
pub fn repo_root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }

/// Allowed top-level folders.
pub const TOP_LEVEL_DIRS: &[&str] = &["crates", "tools", "docs", "extracted", "work", "target", "dist"];
/// Allowed top-level files (besides dotfiles).
pub const TOP_LEVEL_FILES: &[&str] = &["Cargo.toml", "Cargo.lock", "README.md"];

/// Text the product must not contain: savestates, the personal folder, dev output, recordings, the personal env var.
pub const PERSONAL_PATTERNS: &[&str] = &[".p2s", "~/PS2", "PS2/ratchet1", "work/", "/traces/", "RC_PERSONAL"];

/// Known product lines that match [`PERSONAL_PATTERNS`] and cannot change without a product change:
/// (file under the repo, text on the line). Each one is a follow-up in docs/plan/repo_reorg.md; remove the
/// entry when it is fixed (the guard fails on a stale entry).
pub const KNOWN_OFFENDERS: &[(&str, &str)] = &[
    // The disc golden tests fall back to the ISO in the personal folder when RC_ISO is unset.
    ("crates/rc-formats/tests/golden.rs", "Needs the user's disc image (`RC_ISO`, else `~/PS2/ratchet1/"),
    ("crates/rc-formats/tests/golden.rs", ".join(\"PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso\")"),
    ("crates/rc-game/tests/game_state_novalis.rs", ".join(\"PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso\")"),
    ("crates/rc-extract/tests/golden.rs", "RC_ISO=~/PS2/ratchet1/...iso"),
];

/// Lexical normalisation (`a/b/../c` → `a/c`), relative to the repo root.
pub fn normalise(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => { out.pop(); }
            Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// Every `path = "..."` value in a manifest's text (dependency tables, `[lib]`, `[[bin]]`).
pub fn manifest_paths(toml: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in toml.lines() {
        let line = line.split('#').next().unwrap_or("");
        let mut rest = line;
        while let Some(i) = rest.find("path") {
            let before_ok = i == 0 || !rest.as_bytes()[i - 1].is_ascii_alphanumeric() && rest.as_bytes()[i - 1] != b'_' && rest.as_bytes()[i - 1] != b'-';
            let after = rest[i + 4..].trim_start();
            if before_ok && after.starts_with('=') {
                let v = after[1..].trim_start();
                if let Some(v) = v.strip_prefix('"') {
                    if let Some(end) = v.find('"') { out.push(v[..end].to_string()); }
                }
            }
            rest = &rest[i + 4..];
        }
    }
    out
}

/// Paths in the manifest of the crate at `crate_dir` (repo-relative) that resolve under `tools/`.
pub fn tool_dependencies(crate_dir: &Path, toml: &str) -> Vec<String> {
    manifest_paths(toml).into_iter().filter(|p| normalise(&crate_dir.join(p)).starts_with("tools")).collect()
}

/// The patterns of [`PERSONAL_PATTERNS`] on one line. `work/` and `/traces/` count only as a path segment
/// (not inside `framework/`).
pub fn personal_hits(line: &str) -> Vec<&'static str> {
    PERSONAL_PATTERNS.iter().copied().filter(|pat| {
        line.match_indices(pat).any(|(i, _)| {
            if *pat == "work/" { i == 0 || !(line.as_bytes()[i - 1].is_ascii_alphanumeric() || line.as_bytes()[i - 1] == b'_' || line.as_bytes()[i - 1] == b'-') } else { true }
        })
    }).collect()
}

/// Every file under `dir`, recursively (skips `target/`).
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { if e.file_name() != "target" { stack.push(p); } } else { out.push(p); }
        }
    }
    out.sort();
    out
}
