//! The test tiers of docs/workflows/testing.md §2, one command each:
//!
//! - `test-quick [crate]`: every crate's unit tests (`cargo test-all --lib --bins`, about 5 s).
//! - `test-job <area…>`: the per-job tier (§2.1): the unit tests, the integration binaries of each named area ([`AREAS`],
//!   the area map of §3), and for the area `shared` the shared-code guard set (the NO_IDLE hero digest compared with
//!   the baseline, and the all-levels smoke).
//! - `test-full`: the full suite (§2.2), then the digest compared with the baseline (never rewritten), then
//!   `cargo xtask sweep` (a failure there only warns).
//! - `digest-baseline`: the only command that writes the baseline, for a human who has decided a digest change is
//!   intended (or a job with none, before its first edit); it prints what changed.
//!
//! All of them set `RC_AUDIO=0`, run from the repo root and use cargo-nextest when it is installed (`cargo nextest`
//! answers), else the `cargo test-all` alias. The nextest runs pass the same flags as the alias
//! (`--workspace --features rc-engine/dev`), so both share the dev (dynamic) Bevy build.

use crate::{cargo, repo_root};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

/// Part of an area: a whole integration binary, or the tests of one whose names start with one of the prefixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// (package, integration binary).
    Bin(&'static str, &'static str),
    /// (package, integration binary, test-name prefixes: a module `name::`, or one test's full path).
    Tests(&'static str, &'static str, &'static [&'static str]),
}

/// An area of testing.md §3: its name, its aliases, and its integration tests. The unit tests of every area run in
/// every `test-job` (they are not split by area). `shared` is the shared-code guard set (§2.1 step 3); its digest
/// comparison is added by [`job`].
pub struct Area {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub parts: &'static [Part],
}

use Part::{Bin, Tests};

pub const AREAS: &[Area] = &[
    Area { name: "hero", aliases: &[], parts: &[Bin("rc-game", "hero")] },
    Area { name: "weapons", aliases: &[], parts: &[Bin("rc-game", "weapons")] },
    Area { name: "classes", aliases: &["creatures", "mobys"], parts: &[Bin("rc-game", "classes")] },
    Area { name: "world", aliases: &["levels", "collision", "water"], parts: &[Bin("rc-game", "world")] },
    Area { name: "ui", aliases: &["menus", "hud", "map", "save", "vendor"], parts: &[Bin("rc-game", "ui")] },
    Area {
        name: "audio",
        aliases: &[],
        parts: &[
            Tests("rc-game", "ui", &["reverb_conformance::", "sound_conformance::"]),
            Tests("rc-game", "weapons", &["hero_guns_novalis::novalis_ryno_salvo_sounds_mix_without_overflow"]),
        ],
    },
    Area { name: "formats", aliases: &[], parts: &[Bin("rc-formats", "formats")] },
    Area { name: "data", aliases: &[], parts: &[Bin("rc-data", "data")] },
    Area { name: "extract", aliases: &[], parts: &[Bin("rc-extract", "extract")] },
    Area { name: "video", aliases: &["movies"], parts: &[Bin("rc-video", "movies")] },
    Area { name: "engine", aliases: &["render", "input"], parts: &[] },
    Area { name: "trace", aliases: &[], parts: &[Bin("rc-trace", "trace")] },
    Area { name: "repo", aliases: &["guards", "layout"], parts: &[Bin("repo-checks", "guards")] },
    Area { name: "shared", aliases: &[], parts: &[Tests("rc-game", "world", &["all_levels_smoke::"])] },
];

/// The area named `name` (or one of its aliases).
pub fn area(name: &str) -> Option<&'static Area> { AREAS.iter().find(|a| a.name == name || a.aliases.contains(&name)) }

/// The runner: nextest when installed, else `cargo test`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runner { Nextest, CargoTest }

#[derive(Debug, Default, PartialEq, Eq)]
struct Opts {
    /// `--cargo-test`: use `cargo test-all` even when nextest is installed.
    cargo_test: bool,
    /// The positional arguments (the crate of `test-quick`, the areas of `test-job`).
    names: Vec<String>,
    /// After `--`: passed to the runner as is.
    extra: Vec<OsString>,
}

fn parse(argv: &[OsString]) -> Result<Opts, String> {
    let mut o = Opts::default();
    let mut it = argv.iter();
    while let Some(a) = it.next() {
        match a.to_str() {
            Some("--") => { o.extra = it.cloned().collect(); break; }
            Some("--cargo-test") => o.cargo_test = true,
            Some(s) if s.starts_with('-') => return Err(format!("unknown option {s}")),
            Some(s) => o.names.push(s.to_string()),
            None => return Err(format!("unexpected argument {a:?}")),
        }
    }
    Ok(o)
}

fn runner(o: &Opts) -> Runner {
    if o.cargo_test { return Runner::CargoTest; }
    let ok = cargo().args(["nextest", "--version"]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success());
    if ok { Runner::Nextest } else { Runner::CargoTest }
}

/// One command to run: its arguments to `cargo` and extra environment.
#[derive(Debug, PartialEq, Eq)]
pub struct Step {
    pub args: Vec<String>,
    pub env: Vec<(&'static str, String)>,
}

fn step(args: &[&str]) -> Step { Step { args: args.iter().map(|s| s.to_string()).collect(), env: Vec::new() } }

/// `cargo nextest run` with the flags of the `test-all` alias.
const NEXTEST: &[&str] = &["nextest", "run", "--workspace", "--features", "rc-engine/dev"];

/// A nextest filterset matching the parts.
fn filterset(parts: &[Part]) -> String {
    let id = |p: &str, b: &str| format!("binary_id({p}::{b})");
    parts
        .iter()
        .map(|p| match p {
            Bin(pkg, bin) => id(pkg, bin),
            Tests(pkg, bin, pre) => {
                let t: Vec<String> = pre.iter().map(|s| format!("test(/^{}/)", s.replace('.', "\\."))).collect();
                format!("({} & ({}))", id(pkg, bin), t.join(" | "))
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// The unit tests of every crate, or of one (`krate`: a package name, `-` or `_` alike).
fn quick_steps(r: Runner, krate: Option<&str>) -> Vec<Step> {
    match r {
        Runner::Nextest => {
            let mut s = step(NEXTEST);
            s.args.extend(["--lib", "--bins"].map(String::from));
            if let Some(k) = krate { s.args.extend(["-E".to_string(), format!("package({})", k.replace('_', "-"))]); }
            vec![s]
        }
        // `cargo test -p <crate>` would resolve other features and rebuild (testing.md §1); the unit tests of every
        // crate take about 5 s, so without nextest they all run.
        Runner::CargoTest => vec![step(&["test-all", "--lib", "--bins"])],
    }
}

/// The unit tests and the integration parts of the areas (duplicates dropped), without the digest.
fn job_steps(r: Runner, areas: &[&Area]) -> Vec<Step> {
    let mut parts: Vec<Part> = Vec::new();
    for a in areas { for p in a.parts { if !parts.contains(p) { parts.push(*p); } } }
    // A whole binary covers any filtered part of it.
    let whole: Vec<&str> = parts.iter().filter_map(|p| match p { Bin(_, b) => Some(*b), _ => None }).collect();
    parts.retain(|p| !matches!(p, Tests(_, b, _) if whole.contains(b)));
    let mut bins: Vec<&str> = Vec::new();
    for p in &parts { let b = match p { Bin(_, b) | Tests(_, b, _) => *b }; if !bins.contains(&b) { bins.push(b); } }
    match r {
        Runner::Nextest => {
            let mut s = step(NEXTEST);
            s.args.extend(["--lib", "--bins"].map(String::from));
            for b in &bins { s.args.extend(["--test".to_string(), b.to_string()]); }
            let mut expr = "kind(lib) | kind(bin)".to_string();
            if !parts.is_empty() { expr += &format!(" | {}", filterset(&parts)); }
            s.args.extend(["-E".to_string(), expr]);
            vec![s]
        }
        Runner::CargoTest => {
            let mut out = quick_steps(r, None);
            let mut s = step(&["test-all"]);
            for p in &parts { if let Bin(_, b) = p { s.args.extend(["--test".to_string(), b.to_string()]); } }
            if s.args.len() > 1 { out.push(s); }
            for p in &parts {
                if let Tests(_, b, pre) = p {
                    let mut s = step(&["test-all", "--test", b, "--"]);
                    s.args.extend(pre.iter().map(|x| x.to_string()));
                    out.push(s);
                }
            }
            out
        }
    }
}

/// The full suite. There are no doctests (checked 2026-09-29: every lib's `Doc-tests` runs 0), and nextest does not
/// run doctests; add `cargo test-all --doc` here when the first one appears.
fn full_steps(r: Runner) -> Vec<Step> {
    match r {
        Runner::Nextest => vec![step(&[NEXTEST, &["--no-fail-fast"]].concat())],
        Runner::CargoTest => vec![step(&["test-all", "--no-fail-fast"])],
    }
}

fn results_dir() -> PathBuf { repo_root().join("work/test-results") }
/// The digest baseline (git-ignored; written only by `digest-baseline`).
pub fn baseline_path() -> PathBuf { results_dir().join("hero_digest_no_idle.txt") }
fn job_digest_path() -> PathBuf { results_dir().join("digest_job.txt") }

/// Ratchet's NO_IDLE digest into `out` (the test runs in the package folder, so the path is absolute).
fn digest_step(out: &std::path::Path) -> Step {
    let mut s = step(&["test-all", "-q", "--test", "hero", "--", "--exact", "hero_novalis::novalis_hero_digest"]);
    s.env = vec![("RC_HERO_DIGEST", out.display().to_string()), ("RC_HERO_DIGEST_NO_IDLE", "1".to_string())];
    s
}

fn run_step(s: &Step, extra: &[OsString]) -> bool {
    let mut c = cargo();
    c.args(&s.args).current_dir(repo_root()).env("RC_AUDIO", "0");
    // Extra arguments go to the test binaries: after the `--` a step already has, else after a new one.
    if !extra.is_empty() {
        if !s.args.iter().any(|a| a == "--") && s.args.first().is_some_and(|a| a != "nextest") { c.arg("--"); }
        c.args(extra);
    }
    for (k, v) in &s.env { c.env(k, v); }
    let shown: Vec<String> = s.env.iter().map(|(k, v)| format!("{k}={v}")).chain(["RC_AUDIO=0".to_string(), "cargo".to_string()]).chain(s.args.iter().cloned()).collect();
    eprintln!("xtask: {}", shown.join(" "));
    match c.status() {
        Ok(st) => st.success(),
        Err(e) => { eprintln!("error: cannot run cargo: {e}"); false }
    }
}

fn run_all(steps: &[Step], extra: &[OsString]) -> bool {
    // Every step runs (a failing unit test does not hide an area's failures); the result is their conjunction.
    let mut ok = true;
    for s in steps { ok &= run_step(s, extra); }
    ok
}

fn done(ok: bool) -> ExitCode { if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE } }

fn opts(argv: &[OsString]) -> Result<Opts, ExitCode> {
    parse(argv).map_err(|e| { eprintln!("error: {e}\n\n{}", crate::HELP); ExitCode::from(2) })
}

/// `cargo xtask test-quick [crate] [--cargo-test] [-- <runner args>]`.
pub fn quick(argv: &[OsString]) -> ExitCode {
    let o = match opts(argv) { Ok(o) => o, Err(c) => return c };
    if o.names.len() > 1 { eprintln!("error: test-quick takes at most one crate"); return ExitCode::from(2); }
    let r = runner(&o);
    if r == Runner::CargoTest && !o.names.is_empty() { eprintln!("xtask: no nextest: running every crate's unit tests (a single crate would rebuild)"); }
    done(run_all(&quick_steps(r, o.names.first().map(String::as_str)), &o.extra))
}

/// `cargo xtask test-job <area…> [--cargo-test] [-- <runner args>]`.
pub fn job(argv: &[OsString]) -> ExitCode {
    let o = match opts(argv) { Ok(o) => o, Err(c) => return c };
    if o.names.is_empty() { eprintln!("error: test-job needs at least one area: {}", area_names()); return ExitCode::from(2); }
    let mut areas = Vec::new();
    for n in &o.names {
        match area(n) {
            Some(a) => areas.push(a),
            None => { eprintln!("error: unknown area {n:?}; areas: {}", area_names()); return ExitCode::from(2); }
        }
    }
    let r = runner(&o);
    let mut ok = run_all(&job_steps(r, &areas), &o.extra);
    if areas.iter().any(|a| a.name == "shared") { ok = digest_check() && ok; }
    done(ok)
}

/// Runs the digest into `work/test-results/digest_job.txt` and compares it with the baseline.
fn digest_check() -> bool {
    let (base, now) = (baseline_path(), job_digest_path());
    if !base.exists() {
        eprintln!("xtask: no digest baseline at {}: take one before the first edit with `cargo xtask digest-baseline`", base.display());
        return false;
    }
    if std::fs::create_dir_all(results_dir()).is_err() || !run_step(&digest_step(&now), &[]) { return false; }
    let same = matches!((std::fs::read(&base), std::fs::read(&now)), (Ok(a), Ok(b)) if a == b);
    if same { eprintln!("xtask: hero digest identical to the baseline"); } else { eprintln!("xtask: HERO DIGEST DIFFERS: {} vs {}", now.display(), base.display()); }
    same
}

/// `cargo xtask digest-baseline`: writes the digest baseline, and prints what changed against the old one.
pub fn baseline(argv: &[OsString]) -> ExitCode {
    if !argv.is_empty() { eprintln!("error: digest-baseline takes no arguments"); return ExitCode::from(2); }
    let old = std::fs::read_to_string(baseline_path()).ok();
    let ok = std::fs::create_dir_all(results_dir()).is_ok() && run_step(&digest_step(&baseline_path()), &[]);
    if !ok { return done(false); }
    let new = std::fs::read_to_string(baseline_path()).unwrap_or_default();
    match old {
        None => eprintln!("xtask: wrote the first baseline {}", baseline_path().display()),
        Some(o) if o == new => eprintln!("xtask: baseline unchanged ({})", baseline_path().display()),
        Some(o) => {
            let d = line_diff(&o, &new);
            eprintln!("xtask: BASELINE CHANGED ({}): {} line(s) differ; the first ones:", baseline_path().display(), d.len());
            for l in d.iter().take(20) { eprintln!("{l}"); }
        }
    }
    done(true)
}

/// The lines that differ between two texts, position by position (`-` old, `+` new, with the line number).
fn line_diff(old: &str, new: &str) -> Vec<String> {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let mut out = Vec::new();
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i), b.get(i));
        if x == y { continue; }
        if let Some(x) = x { out.push(format!("  {:>6} - {x}", i + 1)); }
        if let Some(y) = y { out.push(format!("  {:>6} + {y}", i + 1)); }
    }
    out
}

/// `cargo xtask test-full [--cargo-test] [-- <runner args>]`: the full suite, then the digest compared with the baseline
/// (a mismatch fails; the baseline is never rewritten here), then the target/ sweep (a failure only warns).
pub fn full(argv: &[OsString]) -> ExitCode {
    let o = match opts(argv) { Ok(o) => o, Err(c) => return c };
    if !o.names.is_empty() { eprintln!("error: test-full takes no areas"); return ExitCode::from(2); }
    let ok = run_all(&full_steps(runner(&o)), &o.extra);
    let digest = digest_check();
    // Keep target/ under the limit; a refused or failed sweep never fails the test run.
    if !crate::sweep::run(&crate::sweep::Opts::default()) { eprintln!("xtask: warning: the target/ sweep did not run (above); the test result stands"); }
    done(ok && digest)
}

fn area_names() -> String { AREAS.iter().map(|a| a.name).collect::<Vec<_>>().join(", ") }

#[cfg(test)]
mod tests {
    use super::*;

    fn os(v: &[&str]) -> Vec<OsString> { v.iter().map(OsString::from).collect() }

    #[test]
    fn parses_names_flags_and_extra() {
        let o = parse(&os(&["hero", "--cargo-test", "ui", "--", "--nocapture", "x"])).unwrap();
        assert_eq!(o, Opts { cargo_test: true, names: vec!["hero".into(), "ui".into()], extra: os(&["--nocapture", "x"]) });
        assert!(parse(&os(&["--bogus"])).is_err());
    }

    #[test]
    fn areas_resolve_by_name_and_alias() {
        assert_eq!(area("mobys").unwrap().name, "classes");
        assert_eq!(area("water").unwrap().name, "world");
        assert_eq!(area("vendor").unwrap().name, "ui");
        assert!(area("nope").is_none());
        // Names and aliases are unique.
        let mut all: Vec<&str> = AREAS.iter().flat_map(|a| std::iter::once(a.name).chain(a.aliases.iter().copied())).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }

    /// The help lists every area and alias.
    #[test]
    fn help_lists_every_area() {
        for a in AREAS { for n in std::iter::once(a.name).chain(a.aliases.iter().copied()) { assert!(crate::HELP.contains(n), "{n}"); } }
    }

    /// Every binary the table names is an integration binary on disk (`tests/<bin>/main.rs` or `tests/<bin>.rs`).
    #[test]
    fn every_area_binary_exists() {
        let dir = |pkg: &str| match pkg { "rc-trace" => "tools/trace".to_string(), "repo-checks" => "tools/repo-checks".to_string(), p => format!("crates/{p}") };
        for a in AREAS {
            for p in a.parts {
                let (Bin(pkg, bin) | Tests(pkg, bin, _)) = *p;
                let t = repo_root().join(dir(pkg)).join("tests");
                assert!(t.join(bin).join("main.rs").exists() || t.join(format!("{bin}.rs")).exists(), "{}: {pkg} {bin}", a.name);
            }
        }
    }

    #[test]
    fn job_steps_nextest_one_run_with_a_filterset() {
        let s = job_steps(Runner::Nextest, &[area("hero").unwrap(), area("audio").unwrap()]);
        assert_eq!(s.len(), 1);
        let a = s[0].args.join(" ");
        assert!(a.starts_with("nextest run --workspace --features rc-engine/dev --lib --bins --test hero --test ui --test weapons -E "), "{a}");
        assert!(a.ends_with("kind(lib) | kind(bin) | binary_id(rc-game::hero) | (binary_id(rc-game::ui) & (test(/^reverb_conformance::/) | test(/^sound_conformance::/))) | (binary_id(rc-game::weapons) & (test(/^hero_guns_novalis::novalis_ryno_salvo_sounds_mix_without_overflow/)))"), "{a}");
    }

    #[test]
    fn job_steps_cargo_test_whole_binaries_then_filtered_runs() {
        let s = job_steps(Runner::CargoTest, &[area("ui").unwrap(), area("audio").unwrap(), area("shared").unwrap()]);
        let got: Vec<String> = s.iter().map(|s| s.args.join(" ")).collect();
        // `ui` covers audio's ui part; audio's weapons test and the smoke stay filtered runs.
        assert_eq!(got, [
            "test-all --lib --bins",
            "test-all --test ui",
            "test-all --test weapons -- hero_guns_novalis::novalis_ryno_salvo_sounds_mix_without_overflow",
            "test-all --test world -- all_levels_smoke::",
        ]);
    }

    #[test]
    fn line_diff_marks_changed_added_and_removed_lines() {
        assert!(line_diff("a\nb\n", "a\nb\n").is_empty());
        assert_eq!(line_diff("a\nb\nc", "a\nB"), ["       2 - b", "       2 + B", "       3 - c"]);
    }

    #[test]
    fn quick_and_full_steps() {
        assert_eq!(quick_steps(Runner::Nextest, Some("rc_game"))[0].args.join(" "), "nextest run --workspace --features rc-engine/dev --lib --bins -E package(rc-game)");
        assert_eq!(quick_steps(Runner::CargoTest, Some("rc-game"))[0].args.join(" "), "test-all --lib --bins");
        assert_eq!(full_steps(Runner::Nextest)[0].args.join(" "), "nextest run --workspace --features rc-engine/dev --no-fail-fast");
        assert_eq!(full_steps(Runner::CargoTest)[0].args.join(" "), "test-all --no-fail-fast");
    }
}
