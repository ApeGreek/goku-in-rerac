//! `cargo xtask <command>`: one short command per dev chore (alias in `.cargo/config.toml`). Dev only, never ships;
//! std only. Doc: `tools/xtask/README.md`.

mod classify;
mod regen;
mod test;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const HELP: &str = "\
cargo xtask <command> [options]    dev chores, one command each (tools/xtask/README.md)

commands:
  regen-data [--iso <image>] [--data-dir <dir>] [--force] [--check]
      Rebuild the dev game data folder (extracted/, or RC_EXTRACTED) from your disc image with the dev build of
      randcrw-extract: extract into a staging folder, verify it, then swap it in and delete the old folder.
      The retired C++ extractor's leftovers are listed and removed; any unknown file stops it unless --force.
        --iso <image>     the disc image (default: RC_ISO)
        --data-dir <dir>  the folder to rebuild (default: RC_EXTRACTED, else <repo>/extracted)
        --force           also delete unknown files
        --check           only list what the folder holds; change nothing (no disc needed)
  package [--no-build]
      Release packaging: runs tools/package/package.sh (dist/randcrw-<version>-<os>-<arch>/ and its .zip).

test tiers (docs/workflows/testing.md §2; RC_AUDIO=0; cargo-nextest when installed, else `cargo test-all`):
  test-quick [crate]
      Unit tests only: every crate's lib and bin tests (`cargo test-all --lib --bins`, about 5 s). With nextest a
      crate name runs just that crate's (without nextest every crate runs: a single one would rebuild).
  test-job <area>...
      The per-job tier: the unit tests plus the integration binaries of each area. Areas (aliases):
        hero, weapons, classes (creatures, mobys), world (levels, collision, water), ui (menus, hud, map, save,
        vendor), audio, formats, data, extract, video (movies), engine (render, input: unit tests only), trace,
        repo (guards, layout), shared (for shared code: the all-levels smoke, and the NO_IDLE hero digest
        compared with work/test-results/hero_digest_no_idle.txt).
  test-full
      The full suite (on merge to main, or when the coordinator asks), then the NO_IDLE hero digest compared
      with the baseline; a mismatch fails. It never rewrites the baseline.
  digest-baseline
      The only command that writes the NO_IDLE hero digest baseline, and prints what changed: for a human who
      has decided a digest change is intended, or before a job's first edit when there is none.
    Options of the test commands:
      --cargo-test        use `cargo test-all` even when nextest is installed
      -- <args>           passed on: to nextest (e.g. --no-capture), or to the test binaries under cargo test
  help
      This list.";

/// The repository root (this crate lives at `tools/xtask`).
pub fn repo_root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    p.canonicalize().unwrap_or(p)
}

/// The `cargo` that ran us (`CARGO` is set by `cargo run`), else `cargo` from `PATH`.
pub fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (cmd, rest) = match argv.split_first() {
        Some((c, r)) => (c.to_string_lossy().into_owned(), r),
        None => ("help".to_string(), &argv[..]),
    };
    match cmd.as_str() {
        "regen-data" => regen::run(rest),
        "package" => package(rest),
        "test-quick" => test::quick(rest),
        "test-job" => test::job(rest),
        "test-full" => test::full(rest),
        "digest-baseline" => test::baseline(rest),
        "help" | "--help" | "-h" => { println!("{HELP}"); ExitCode::SUCCESS }
        other => {
            eprintln!("error: unknown command {other:?}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

/// `cargo xtask package [args]`: `tools/package/package.sh [args]`, from the repo root.
fn package(args: &[OsString]) -> ExitCode {
    let root = repo_root();
    let status = Command::new("bash").arg(root.join("tools/package/package.sh")).args(args).current_dir(&root).status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(1)),
        Err(e) => { eprintln!("error: cannot run bash tools/package/package.sh: {e}"); ExitCode::FAILURE }
    }
}
