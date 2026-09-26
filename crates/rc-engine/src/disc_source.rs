//! Where the level's disc bytes come from: the user's own disc image (preferred) or the
//! `extracted/` tree written by `rc_extract unpack` (fallback).
//!
//! Every request uses the extractor's relative file name (`levels/NN/core_data.bin`,
//! `boot/SCUS_971.99`, `toc.bin`); `rc_formats::disc` produces those exact bytes from the image
//! (verified for all 19 levels by `tests/golden.rs::disc_matches_extracted_for_every_level`), so the
//! two sources are interchangeable.
//!
//! Selection, once per process:
//! - `RC_SOURCE=extracted`: always `extracted/`.
//! - `RC_SOURCE=iso`: the image; failing to open it is an error.
//! - otherwise the image at `RC_ISO` (if set and non-empty), else
//!   `~/PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso` if it exists, else `extracted/`.

use anyhow::{bail, Context, Result};
use rc_formats::disc::{Disc, LevelFiles};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const DEFAULT_ISO: &str = "PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso";

struct IsoSource {
    path: PathBuf,
    disc: Disc,
    /// Parsed once per level: the loader asks for several members, some more than once.
    levels: Mutex<HashMap<u32, Arc<LevelFiles>>>,
}

enum Source { Iso(Box<IsoSource>), Extracted }

fn iso_path() -> Option<PathBuf> {
    match std::env::var_os("RC_ISO").filter(|v| !v.is_empty()) {
        Some(p) => Some(PathBuf::from(p)),
        None => Some(PathBuf::from(std::env::var_os("HOME")?).join(DEFAULT_ISO)).filter(|p| p.exists()),
    }
}

fn source() -> Result<&'static Source> {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    if let Some(s) = SOURCE.get() { return Ok(s); }
    let mode = std::env::var("RC_SOURCE").unwrap_or_default();
    let s = match (mode.as_str(), iso_path()) {
        ("extracted", _) | ("", None) => Source::Extracted,
        ("iso", None) => bail!("RC_SOURCE=iso but no disc image (set RC_ISO or place it at ~/{DEFAULT_ISO})"),
        ("iso" | "", Some(path)) => {
            let t0 = Instant::now();
            match Disc::open(&path) {
                Ok(disc) => {
                    println!("source: iso {} ({} sectors of {} bytes; open + TOC {:.1} ms)",
                        path.display(), disc.iso().sector_count(), disc.iso().raw_sector_size(), t0.elapsed().as_secs_f64() * 1e3);
                    Source::Iso(Box::new(IsoSource { path, disc, levels: Mutex::new(HashMap::new()) }))
                }
                Err(e) if mode.is_empty() => {
                    eprintln!("warning: cannot read disc image {}: {e}; falling back to extracted/", path.display());
                    Source::Extracted
                }
                Err(e) => return Err(e).with_context(|| format!("opening disc image {}", path.display())),
            }
        }
        (other, _) => bail!("RC_SOURCE must be \"iso\" or \"extracted\", got {other:?}"),
    };
    if matches!(s, Source::Extracted) { println!("source: extracted"); }
    Ok(SOURCE.get_or_init(|| s))
}

impl IsoSource {
    fn level(&self, id: u32) -> Result<Arc<LevelFiles>> {
        let mut levels = self.levels.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(l) = levels.get(&id) { return Ok(l.clone()); }
        let t0 = Instant::now();
        let files = Arc::new(self.disc.level(id).with_context(|| format!("reading level {id} from {}", self.path.display()))?);
        println!("iso: level {id:02} lumps read in {:.1} ms ({:.1} MiB)", t0.elapsed().as_secs_f64() * 1e3, files.total_bytes() as f64 / (1 << 20) as f64);
        levels.insert(id, files.clone());
        Ok(files)
    }

    /// `Some(bytes)` for the extractor files the disc reader reproduces, `None` for anything else.
    fn read(&self, rel: &str) -> Result<Option<Vec<u8>>> {
        if rel == "toc.bin" { return Ok(Some(self.disc.toc().raw.clone())); }
        if let Some(name) = rel.strip_prefix("boot/") {
            if self.disc.boot_elf_path().is_ok_and(|p| p.trim_start_matches('/').eq_ignore_ascii_case(name)) {
                return Ok(Some(self.disc.boot_elf()?));
            }
            return Ok(match self.disc.iso().find(name) { Some(e) => Some(self.disc.iso().read_file(e)?), None => None });
        }
        if let Some((nn, name)) = rel.strip_prefix("levels/").and_then(|r| r.split_once('/')) {
            let Ok(id) = nn.parse::<u32>() else { return Ok(None) };
            let level = self.level(id)?;
            if let Some(b) = level.file(name) { return Ok(Some(b.to_vec())); }
        }
        Ok(None)
    }
}

/// The bytes of `rel` (a path relative to the extraction root, as `rc_extract` names it).
pub fn read(root: &Path, rel: &str) -> Result<Vec<u8>> {
    if let Source::Iso(iso) = source()? {
        if let Some(bytes) = iso.read(rel)? { return Ok(bytes); }
    }
    let p = root.join(rel);
    std::fs::read(&p).with_context(|| format!("reading {}", p.display()))
}

/// `levels/NN/<name>`: `core_index.bin`, `core_data.bin` (WAD-compressed), `gs_ram.bin`, `gameplay_ntsc.bin`, ...
pub fn level_file(root: &Path, index: u32, name: &str) -> Result<Vec<u8>> { read(root, &format!("levels/{index:02}/{name}")) }

/// `read` for a full path under `root` (e.g. `root.join("boot/SCUS_971.99")`).
pub fn read_path(root: &Path, p: &Path) -> Result<Vec<u8>> {
    match p.strip_prefix(root) {
        Ok(rel) => read(root, &rel.to_string_lossy().replace('\\', "/")),
        Err(_) => std::fs::read(p).with_context(|| format!("reading {}", p.display())),
    }
}
