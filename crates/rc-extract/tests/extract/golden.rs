//! Golden tests against the user's real disc image and the committed size/SHA-1 table
//! (`crates/rc-extract/data/scus_971_99.tsv`, which the Rust extractor's output matched file for file against the
//! retired C++ reference extractor when it was written). Ignored by default:
//!
//! ```sh
//! RC_ISO=~/PS2/ratchet1/...iso RC_EXTRACT_OUT=/some/scratch/dir \
//!   cargo test -p rc-extract --release --test extract -- golden:: --ignored --nocapture --test-threads 1
//! ```
//! `RC_EXTRACT_OUT` (default: the system temp dir) receives `full/` (~4.0 GiB) and `ntsc/` (~2.6 GiB); delete them
//! afterwards. `builtin_table_matches_the_disc` is also how the committed table is checked after regenerating it
//! with `randcrw-extract table --iso <image> --output crates/rc-extract/data/scus_971_99.tsv`.

use rc_extract::build_db::{self, BUILDS};
use rc_extract::extract::{self, Options};
use rc_extract::{verify, Event, INFO_FILE};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn iso() -> PathBuf { PathBuf::from(std::env::var_os("RC_ISO").expect("set RC_ISO to the NTSC-U disc image")) }
fn out_root() -> PathBuf { std::env::var_os("RC_EXTRACT_OUT").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("randcrw-extract-golden") }

fn files(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            if e.file_type().unwrap().is_dir() { stack.push(e.path()); continue; }
            out.insert(e.path().strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
    out
}

fn quiet(_: Event) {}

#[test]
#[ignore = "needs RC_ISO and ~7 GiB of scratch space"]
fn extract_matches_the_committed_table() {
    let iso = iso();
    let full = out_root().join("full");
    let _ = std::fs::remove_dir_all(&full);

    let t = Instant::now();
    let (id, s) = extract::extract(&iso, &full, BUILDS, &Options::default(), &mut quiet).unwrap();
    let t_extract = t.elapsed();
    assert!(id.supported);
    println!("extract: {} files, {} bytes ({:.1} MiB) in {:.2} s", s.files, s.bytes, rc_extract::mib(s.bytes), t_extract.as_secs_f64());

    let mut ours = files(&full);
    assert!(ours.remove(INFO_FILE));
    // The Tier 1 engine cache `extract` ends with (rc-data's own tests check its contents).
    assert!(ours.iter().any(|p| p == "cache/v1/stamp.toml"));
    ours.retain(|p| !p.starts_with("cache/"));
    // Exactly the table's files; `verify` below re-hashes every one against its size and SHA-1.
    let table: BTreeSet<String> = build_db::parse_table(BUILDS[0].table.unwrap()).unwrap().into_iter().map(|e| e.path).collect();
    let only_ours: Vec<_> = ours.difference(&table).collect();
    let only_table: Vec<_> = table.difference(&ours).collect();
    assert!(only_ours.is_empty() && only_table.is_empty(), "only extracted: {only_ours:?}\nonly in the table: {only_table:?}");
    assert_eq!(s.files as usize, table.len());

    let t = Instant::now();
    let (_, v) = verify::verify(&full, BUILDS, rc_extract::DEFAULT_THREADS, None, &mut quiet).unwrap();
    println!("verify: {} files in {:.2} s", v.files, t.elapsed().as_secs_f64());

    let ntsc = out_root().join("ntsc");
    let _ = std::fs::remove_dir_all(&ntsc);
    let t = Instant::now();
    let (_, n) = extract::extract(&iso, &ntsc, BUILDS, &Options { ntsc_only: true, ..Options::default() }, &mut quiet).unwrap();
    println!("extract --ntsc-only: {} files, {} bytes ({:.1} MiB) in {:.2} s", n.files, n.bytes, rc_extract::mib(n.bytes), t.elapsed().as_secs_f64());
    let skipped: Vec<_> = ours.difference(&files(&ntsc)).cloned().collect();
    assert!(skipped.iter().all(|p| rc_extract::archive::is_pal_copy(p)));
    assert_eq!(skipped.len() as u64, s.files - n.files);
    verify::verify(&ntsc, BUILDS, rc_extract::DEFAULT_THREADS, None, &mut quiet).unwrap();
}

#[test]
#[ignore = "needs RC_ISO"]
fn builtin_table_matches_the_disc() {
    let t = Instant::now();
    let (id, rows) = extract::table(&iso(), BUILDS, rc_extract::DEFAULT_THREADS, &mut quiet).unwrap();
    println!("table: {} rows hashed from the image in {:.2} s", rows.len(), t.elapsed().as_secs_f64());
    let committed = build_db::parse_table(BUILDS[0].table.unwrap()).unwrap();
    assert_eq!(id.serial, BUILDS[0].serial);
    assert_eq!(rows, committed, "regenerate crates/rc-extract/data/scus_971_99.tsv");
}
