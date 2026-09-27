//! `cargo xtask <command>`: one short command per dev chore (alias in `.cargo/config.toml`). Dev only, never ships;
//! std only. Doc: `tools/xtask/README.md`.

mod classify;
mod regen;

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
