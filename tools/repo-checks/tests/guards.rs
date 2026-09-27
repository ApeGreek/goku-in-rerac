//! Repository-layout guards (docs/plan/repo_reorg.md "Rules"). Read-only; no data needed.

use repo_checks::*;
use std::path::Path;

/// Rule: `crates/` ships and never depends on anything under `tools/`.
#[test]
fn product_crates_do_not_depend_on_tools() {
    let root = repo_root();
    let mut checked = 0;
    let mut bad = Vec::new();
    for e in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
        let manifest = e.path().join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
        checked += 1;
        let dir = Path::new("crates").join(e.file_name());
        for p in tool_dependencies(&dir, &text) { bad.push(format!("{}: path = {p:?}", manifest.display())); }
        // A workspace-inherited or git dependency naming a tool package would also be a dependency on tools/.
        for tool in ["rc-trace", "repo-checks", "xtask"] {
            if text.lines().any(|l| l.trim_start().starts_with(&format!("{tool} ")) || l.trim_start().starts_with(&format!("{tool}="))) {
                bad.push(format!("{}: depends on the tool package {tool}", manifest.display()));
            }
        }
    }
    assert!(checked >= 5, "found only {checked} product manifests under crates/");
    assert!(bad.is_empty(), "product crates depend on tools/:\n{}", bad.join("\n"));
}

/// The manifest scan itself: a path under tools/ is caught, product paths are not.
#[test]
fn manifest_scan_catches_a_tool_path() {
    let toml = "[dependencies]\nrc-formats = { path = \"../rc-formats\" }\nrc-trace = { path = \"../../tools/trace\" } # dev\n[lib]\npath = \"src/lib.rs\"\n";
    assert_eq!(manifest_paths(toml), ["../rc-formats", "../../tools/trace", "src/lib.rs"]);
    assert_eq!(tool_dependencies(Path::new("crates/rc-game"), toml), ["../../tools/trace"]);
}

/// Rule: no product source or test names personal material or dev output (the savestates, `~/PS2/ratchet1`,
/// `work/`, recordings, `RC_PERSONAL`), except the known lines listed in `KNOWN_OFFENDERS` (follow-ups).
#[test]
fn product_does_not_reference_personal_or_dev_paths() {
    let root = repo_root();
    let mut hits = Vec::new();
    let mut used = vec![false; KNOWN_OFFENDERS.len()];
    for f in files_under(&root.join("crates")) {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let rel = f.strip_prefix(&root).unwrap_or(&f).to_string_lossy().replace('\\', "/");
        let rel = normalise(Path::new(&rel)).to_string_lossy().into_owned();
        for (n, line) in text.lines().enumerate() {
            let pats = personal_hits(line);
            if pats.is_empty() { continue; }
            match KNOWN_OFFENDERS.iter().position(|(file, needle)| *file == rel && line.contains(needle)) {
                Some(k) => used[k] = true,
                None => hits.push(format!("{rel}:{}: {pats:?}: {}", n + 1, line.trim())),
            }
        }
    }
    assert!(hits.is_empty(), "product files reference personal / dev paths:\n{}", hits.join("\n"));
    let stale: Vec<_> = KNOWN_OFFENDERS.iter().zip(&used).filter(|(_, u)| !**u).map(|(o, _)| o).collect();
    assert!(stale.is_empty(), "KNOWN_OFFENDERS entries no longer found (remove them): {stale:?}");
}

#[test]
fn personal_patterns_match_segments_only() {
    assert_eq!(personal_hits("see work/decomp/level01.elf"), ["work/"]);
    assert!(personal_hits("the framework/ folder and network/io").is_empty());
    assert_eq!(personal_hits("~/PS2/ratchet1/savestates/x.p2s"), [".p2s", "~/PS2", "PS2/ratchet1"]);
}

/// Rule: the top level holds only crates tools docs extracted work target dist, dotfiles, and the Cargo /
/// README files.
#[test]
fn top_level_holds_only_the_agreed_folders() {
    let root = repo_root();
    let mut bad = Vec::new();
    for e in std::fs::read_dir(&root).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') { continue; }
        let ok = if e.path().is_dir() { TOP_LEVEL_DIRS.contains(&name.as_str()) } else { TOP_LEVEL_FILES.contains(&name.as_str()) };
        if !ok { bad.push(name); }
    }
    bad.sort();
    assert!(bad.is_empty(), "unexpected top-level entries (product goes in crates/, dev tools in tools/, output in work/): {bad:?}");
}
