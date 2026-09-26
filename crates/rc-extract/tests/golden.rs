//! Golden tests against the user's real disc image and the C++ oracle's `extracted/` tree. Ignored by default:
//!
//! ```sh
//! RC_ISO=~/PS2/ratchet1/...iso RC_EXTRACT_OUT=/some/scratch/dir \
//!   cargo test -p rc-extract --release --test golden -- --ignored --nocapture --test-threads 1
//! ```
//! `RC_EXTRACT_OUT` (default: the system temp dir) receives `full/` (~4.0 GiB) and `ntsc/` (~2.6 GiB); delete them
//! afterwards. `builtin_table_matches_the_disc` is also how the committed table is checked after regenerating it
//! with `randcrw-extract table --iso <image> --output crates/rc-extract/data/scus_971_99.tsv`.

use rc_extract::build_db::{self, BUILDS};
use rc_extract::extract::{self, Options};
use rc_extract::{verify, Event, INFO_FILE};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn iso() -> PathBuf { PathBuf::from(std::env::var_os("RC_ISO").expect("set RC_ISO to the NTSC-U disc image")) }
fn extracted() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted") }
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

/// The raw lumps `rc_extract unpack` writes (everything else in `extracted/` is derived: `.dec`, splits, dumps, previews).
fn cpp_raw_lumps(root: &Path) -> BTreeSet<String> {
    files(root).into_iter().filter(|p| {
        let parts: Vec<&str> = p.split('/').collect();
        match parts.as_slice() {
            ["toc.bin"] => true,
            ["boot", _] => true,
            ["global", ..] => p.ends_with(".bin") && !p.ends_with("_dump.bin"),
            ["levels", _, f] => f.ends_with(".bin") && !f.ends_with("_dump.bin"),
            ["levels", _, "music" | "speech" | "scene" | "bindata", f] => f.ends_with(".bin"),
            _ => false,
        }
    }).collect()
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    let (mut fa, mut fb) = (std::fs::File::open(a).unwrap(), std::fs::File::open(b).unwrap());
    if fa.metadata().unwrap().len() != fb.metadata().unwrap().len() { return false; }
    let (mut ba, mut bb) = (vec![0u8; 1 << 20], vec![0u8; 1 << 20]);
    loop {
        let n = fa.read(&mut ba).unwrap();
        if n == 0 { return true; }
        fb.read_exact(&mut bb[..n]).unwrap();
        if ba[..n] != bb[..n] { return false; }
    }
}

fn quiet(_: Event) {}

#[test]
#[ignore = "needs RC_ISO and ~7 GiB of scratch space"]
fn extract_matches_cpp_extracted_tree() {
    let (iso, cpp) = (iso(), extracted());
    let full = out_root().join("full");
    let _ = std::fs::remove_dir_all(&full);

    let t = Instant::now();
    let (id, s) = extract::extract(&iso, &full, BUILDS, &Options::default(), &mut quiet).unwrap();
    let t_extract = t.elapsed();
    assert!(id.supported);
    println!("extract: {} files, {} bytes ({:.1} MiB) in {:.2} s", s.files, s.bytes, rc_extract::mib(s.bytes), t_extract.as_secs_f64());

    let mut ours = files(&full);
    assert!(ours.remove(INFO_FILE));
    let theirs = cpp_raw_lumps(&cpp);
    let only_ours: Vec<_> = ours.difference(&theirs).collect();
    let only_theirs: Vec<_> = theirs.difference(&ours).collect();
    assert!(only_ours.is_empty() && only_theirs.is_empty(), "only ours: {only_ours:?}\nonly C++: {only_theirs:?}");
    let t = Instant::now();
    let differing: Vec<&String> = ours.iter().filter(|p| !same_bytes(&full.join(p), &cpp.join(p))).collect();
    assert!(differing.is_empty(), "byte differences: {differing:?}");
    println!("golden: {} files byte-identical to extracted/ (compare {:.2} s)", ours.len(), t.elapsed().as_secs_f64());

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
