//! A RAC1 disc image: ISO 9660 filesystem (boot files) + the TOC at sector 1500 + per-level lumps.
//! Spec: docs/formats/disc_layout.md sections 1-2, docs/formats/wad_layouts_rac1.md 2.3.
//!
//! `Disc::level` produces exactly the raw lumps `rc_extract unpack` writes under
//! `extracted/levels/NN/` (same lump selection, same byte sizes; `tools/extract/main.cpp`
//! `unpack_level`), so the engine can load straight from the user's disc image and the extracted
//! tree stays a byte-for-byte oracle (`tests/golden.rs`, `disc_matches_extracted_for_every_level`).

use crate::iso9660::{bad, IsoImage, Result, SECTOR_SIZE};
use crate::level::{parse_level_data_header, ByteRange};
use crate::toc::{self, LevelHeader, SectorRange, StreamLump, LEVEL_HEADER_SIZE, TOC_SECTOR};
use crate::buf::Buf;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

const SS: u64 = SECTOR_SIZE as u64;

#[derive(Clone, Debug)]
pub struct TocLevel {
    /// Slot in the 19-entry level table at TOC+0x28c8.
    pub table_index: usize,
    /// Absolute sector of the 0x2434-byte amalgamated level header.
    pub header_sector: u32,
    pub header: LevelHeader,
}

/// The TOC blob (`header_size` bytes at sector 1500, 0x2960 on retail) and the levels it points at.
#[derive(Clone, Debug)]
pub struct Toc {
    pub raw: Vec<u8>,
    pub levels: Vec<TocLevel>,
}

/// Every raw lump of one level's "level" group, byte-identical to the extractor's files.
/// `None` = the extractor writes no file (empty range). Member <-> file under `levels/NN/`:
/// see `LevelFiles::files`.
#[derive(Clone, Debug)]
pub struct LevelFiles {
    pub id: u32,
    pub header: LevelHeader,
    /// `level_header.bin`: the 0x2434 bytes at the header sector.
    pub level_header: Vec<u8>,
    /// Lumps of the uncompressed data container (`header.data`), sliced by its 0x58-byte ByteRange table.
    pub overlay: Option<Vec<u8>>,
    pub sound_bank: Option<Vec<u8>>,
    pub core_index: Option<Vec<u8>>,
    pub gs_ram: Option<Vec<u8>>,
    pub hud_header: Option<Vec<u8>>,
    pub hud_banks: [Option<Vec<u8>>; 5],
    /// Still WAD-compressed, exactly as on disc.
    pub core_data: Option<Vec<u8>>,
    /// Whole sector ranges (WAD-compressed streams padded to the sector size).
    pub gameplay_ntsc: Option<Vec<u8>>,
    pub gameplay_pal: Option<Vec<u8>>,
    pub occlusion: Option<Vec<u8>>,
}

impl LevelFiles {
    /// `(file name under extracted/levels/NN/, bytes)` for every present member.
    pub fn files(&self) -> Vec<(String, &[u8])> {
        let mut named: Vec<(String, &Option<Vec<u8>>)> = vec![
            ("overlay.bin".into(), &self.overlay),
            ("sound_bank.bin".into(), &self.sound_bank),
            ("core_index.bin".into(), &self.core_index),
            ("gs_ram.bin".into(), &self.gs_ram),
            ("hud_header.bin".into(), &self.hud_header),
        ];
        named.extend(self.hud_banks.iter().enumerate().map(|(i, h)| (format!("hud_bank_{i}.bin"), h)));
        named.extend([
            ("core_data.bin".into(), &self.core_data),
            ("gameplay_ntsc.bin".into(), &self.gameplay_ntsc),
            ("gameplay_pal.bin".into(), &self.gameplay_pal),
            ("occlusion.bin".into(), &self.occlusion),
        ]);
        let mut v = vec![("level_header.bin".to_string(), self.level_header.as_slice())];
        v.extend(named.into_iter().filter_map(|(n, m)| m.as_deref().map(|b| (n, b))));
        v
    }

    /// The member the extractor writes as `levels/NN/<name>`.
    pub fn file(&self, name: &str) -> Option<&[u8]> { self.files().into_iter().find(|(n, _)| n == name).map(|(_, b)| b) }

    pub fn total_bytes(&self) -> usize { self.files().iter().map(|(_, b)| b.len()).sum() }
}

pub struct Disc<R = File> {
    iso: IsoImage<R>,
    toc: Toc,
}

impl Disc<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> { Self::new(IsoImage::open(path)?) }
}

impl<R: Read + Seek> Disc<R> {
    /// Reads the TOC and every level header it points at (as `read_rac1_toc` in src/core/toc.cpp).
    pub fn new(iso: IsoImage<R>) -> Result<Self> {
        let head = iso.read_sectors(TOC_SECTOR as u32, 1)?;
        let hb = Buf(&head);
        if hb.i32(0)? != 1 { return bad(format!("TOC: version {} != 1; not a RAC1 disc?", hb.i32(0)?)); }
        let header_size = hb.i32(4)?;
        if header_size <= 8 || header_size > 0x200000 { return bad("TOC: implausible header size"); }
        let raw = iso.read_bytes(TOC_SECTOR * SS, header_size as u64)?;
        let mut levels = Vec::new();
        for (table_index, sector) in toc::level_header_sectors(&raw)?.into_iter().enumerate() {
            let Some(sector) = sector else { continue };
            if sector as u64 + 5 > iso.sector_count() as u64 { continue; }
            let hdr = iso.read_bytes(sector as u64 * SS, LEVEL_HEADER_SIZE as u64)?;
            // Entries without the 0x2434 signature are skipped, as the C++ reader does.
            let Ok(header) = toc::parse_level_header(&hdr) else { continue };
            levels.push(TocLevel { table_index, header_sector: sector, header });
        }
        Ok(Disc { iso, toc: Toc { raw, levels } })
    }

    pub fn iso(&self) -> &IsoImage<R> { &self.iso }
    pub fn toc(&self) -> &Toc { &self.toc }
    /// Level ids (the header's `id`, = the `NN` of `extracted/levels/NN`) in table order.
    pub fn level_ids(&self) -> Vec<u32> { self.toc.levels.iter().map(|l| l.header.id as u32).collect() }

    /// The boot ELF's filesystem path, from `SYSTEM.CNF`'s `BOOT2 = cdrom0:\NAME;1` (spec 1.4).
    pub fn boot_elf_path(&self) -> Result<String> {
        let Some(cnf) = self.iso.find("/SYSTEM.CNF") else { return bad("SYSTEM.CNF missing") };
        let cnf = self.iso.read_file(cnf)?;
        let text = String::from_utf8_lossy(&cnf);
        const KEY: &str = "BOOT2 = cdrom0:\\";
        let Some(at) = text.find(KEY) else { return bad("SYSTEM.CNF has no BOOT2 line") };
        let rest = &text[at + KEY.len()..];
        let name = &rest[..rest.find([';', '\r', '\n']).unwrap_or(rest.len())];
        Ok(format!("/{}", name.replace('\\', "/")))
    }

    /// The unwrapped boot ELF (`extracted/boot/SCUS_971.99` on NTSC-U). Falls back to the first
    /// root file starting with `\x7fELF` if `SYSTEM.CNF` does not name an existing file (spec 2.6 B).
    pub fn boot_elf(&self) -> Result<Vec<u8>> {
        if let Some(e) = self.boot_elf_path().ok().and_then(|p| self.iso.find(&p)) { return self.iso.read_file(e); }
        for e in self.iso.entries().iter().filter(|e| !e.is_directory && e.size > 4 && e.path.matches('/').count() == 1) {
            if self.iso.read_bytes(e.lba as u64 * SS, 4)? == b"\x7fELF" { return self.iso.read_file(e); }
        }
        bad("no boot ELF in the filesystem")
    }

    fn toc_level(&self, id: u32) -> Result<&TocLevel> {
        match self.toc.levels.iter().find(|l| l.header.id as u32 == id) {
            Some(l) => Ok(l),
            None => bad(format!("no level {id} in the TOC")),
        }
    }

    fn sector_range(&self, r: SectorRange) -> Result<Option<Vec<u8>>> {
        if r.offset == 0 && r.size == 0 { return Ok(None); }
        if r.offset < 0 || r.size < 0 { return bad(format!("negative sector range {r:?}")); }
        Ok(Some(self.iso.read_bytes(r.offset as u64 * SS, r.size as u64 * SS)?))
    }

    /// The level-group lumps of level `id`, exactly as `rc_extract unpack` writes them.
    pub fn level(&self, id: u32) -> Result<LevelFiles> {
        let lv = self.toc_level(id)?;
        let h = lv.header;
        let level_header = self.iso.read_bytes(lv.header_sector as u64 * SS, LEVEL_HEADER_SIZE as u64)?;

        // The data container: uncompressed, byte-offset lumps. Read in one go, as the extractor does.
        if h.data.offset <= 0 || h.data.size <= 0 { return bad(format!("level {id}: empty data range")); }
        let data = self.iso.read_bytes(h.data.offset as u64 * SS, h.data.size as u64 * SS)?;
        let dh = parse_level_data_header(&data)?;
        let lump = |r: ByteRange, what: &'static str| -> Result<Option<Vec<u8>>> {
            if !r.present() { return Ok(None); }
            Ok(Some(Buf(&data).sub(r.offset as usize, r.size as usize, what)?.0.to_vec()))
        };
        let mut hud_banks: [Option<Vec<u8>>; 5] = Default::default();
        for (dst, r) in hud_banks.iter_mut().zip(dh.hud_banks) { *dst = lump(r, "hud bank")?; }
        Ok(LevelFiles {
            id,
            header: h,
            level_header,
            overlay: lump(dh.overlay, "overlay")?,
            sound_bank: lump(dh.sound_bank, "sound bank")?,
            core_index: lump(dh.core_index, "core index")?,
            gs_ram: lump(dh.gs_ram, "gs ram")?,
            hud_header: lump(dh.hud_header, "hud header")?,
            hud_banks,
            core_data: lump(dh.core_data, "core data")?,
            gameplay_ntsc: self.sector_range(h.gameplay_ntsc)?,
            gameplay_pal: self.sector_range(h.gameplay_pal)?,
            occlusion: self.sector_range(h.occlusion)?,
        })
    }

    /// The global `save_game` lump (TOC +0x10, whole sectors = `extracted/global/save_game.bin`): the
    /// memory-card icon files and the blank save template (`rc_formats::save_game::SaveGameLump`).
    pub fn save_game_lump(&self) -> Result<Vec<u8>> {
        let r = toc::global_sector_range(&self.toc.raw, toc::SAVE_GAME_FIELD)?;
        match self.sector_range(r)? { Some(b) => Ok(b), None => bad("TOC: empty save_game range") }
    }

    /// The audio and scene lumps of level `id` (VAG / WAD sizes probed from their headers).
    pub fn level_stream_lumps(&self, id: u32) -> Result<Vec<StreamLump>> {
        let h = self.toc_level(id)?.header;
        toc::level_stream_lumps(&h, |sector| Ok(toc::probe_lump_size(&self.iso.read_sectors(sector, 1)?).0))
    }

    /// The bytes of one lump from `level_stream_lumps` (`extracted/levels/NN/<name>.bin`).
    pub fn read_lump(&self, l: &StreamLump) -> Result<Vec<u8>> { self.iso.read_bytes(l.sector as u64 * SS, l.bytes) }

    /// Scene `k`'s region file of level `id` (`levels/NN/scene/KK_ntsc.bin` / `_pal.bin`: every chunk
    /// WAD plus the sentinel sector; `crate::scene::Scene::load`). None when the region is empty.
    pub fn scene_region(&self, id: u32, k: usize, pal: bool) -> Result<Option<Vec<u8>>> {
        let h = self.toc_level(id)?.header;
        let Some(rec) = h.scenes.get(k) else { return bad(format!("no scene record {k}")) };
        match rec.region_sectors(pal) {
            Some((first, n)) => Ok(Some(self.iso.read_bytes(first as u64 * SS, n as u64 * SS)?)),
            None => Ok(None),
        }
    }

    /// Scene `k`'s speech VAG of level `id` in language `lang` (0x15ed88 index; `levels/NN/speech/KK_<lang>.bin`).
    pub fn scene_speech(&self, id: u32, k: usize, lang: usize) -> Result<Option<Vec<u8>>> {
        let h = self.toc_level(id)?.header;
        let Some(&s) = h.scenes.get(k).and_then(|r| r.speech.get(lang)) else { return bad(format!("no scene record {k} / language {lang}")) };
        if s <= 0 { return Ok(None); }
        let bytes = toc::probe_lump_size(&self.iso.read_sectors(s as u32, 1)?).0;
        Ok(Some(self.iso.read_bytes(s as u64 * SS, bytes)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iso9660::tests::mini_iso;
    use std::io::Cursor;

    /// Adds a TOC at sector 1500 with one level (id 3) to the synthetic image.
    fn disc_image() -> (Vec<u8>, Vec<u8>) {
        const HDR: usize = 1510;   // level header sector (5 sectors), data follows
        let (mut img, elf) = mini_iso(1600);
        let put = |img: &mut Vec<u8>, at: usize, v: i32| img[at..at + 4].copy_from_slice(&v.to_le_bytes());
        let toc = TOC_SECTOR as usize * SECTOR_SIZE;
        put(&mut img, toc, 1);
        put(&mut img, toc + 4, toc::TOC_SIZE as i32);
        put(&mut img, toc + toc::LEVEL_TABLE_OFFSET + 8, HDR as i32);   // table slot 1
        put(&mut img, toc + toc::LEVEL_TABLE_OFFSET + 12, 1);
        let h = HDR * SECTOR_SIZE;
        put(&mut img, h, 3);
        put(&mut img, h + 4, LEVEL_HEADER_SIZE as i32);
        put(&mut img, h + 8, HDR as i32 + 5);   // data: 2 sectors
        put(&mut img, h + 12, 2);
        put(&mut img, h + 16, HDR as i32 + 7);  // gameplay_ntsc: 1 sector
        put(&mut img, h + 20, 1);
        put(&mut img, h + 0x148, HDR as i32 + 8);   // music[0]: a VAG of 0x30 + 0x20 bytes
        let d = (HDR + 5) * SECTOR_SIZE;
        // ByteRange table: overlay, sound_bank, core_index, gs_ram, hud_header, hud_banks[5], core_data.
        let ranges: [(i32, i32); 11] = [(0x80, 0x10), (-1, 0), (0x100, 0x44), (0x200, 0x300), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (-1, 0), (0x600, 0x123)];
        for (i, (o, s)) in ranges.iter().enumerate() { put(&mut img, d + i * 8, *o); put(&mut img, d + i * 8 + 4, *s); }
        for i in 0x80..0x1000 { img[d + i] = (i * 13) as u8; }
        let v = (HDR + 8) * SECTOR_SIZE;
        img[v..v + 4].copy_from_slice(b"VAGp");
        img[v + 0x0c..v + 0x10].copy_from_slice(&0x20u32.to_be_bytes());
        (img, elf)
    }

    #[test]
    fn reads_toc_boot_elf_and_level_lumps_from_a_synthetic_disc() {
        let (img, elf) = disc_image();
        let disc = Disc::new(IsoImage::new(Cursor::new(img.clone())).unwrap()).unwrap();
        assert_eq!(disc.toc().raw.len(), toc::TOC_SIZE);
        assert_eq!(disc.level_ids(), [3]);
        assert_eq!(disc.toc().levels[0].table_index, 1);
        assert_eq!(disc.boot_elf_path().unwrap(), "/SCUS_971.99");
        assert_eq!(disc.boot_elf().unwrap(), elf);

        let l = disc.level(3).unwrap();
        let d = 1515 * SECTOR_SIZE;
        assert_eq!(l.overlay.as_deref(), Some(&img[d + 0x80..d + 0x90]));
        assert_eq!(l.core_index.as_deref(), Some(&img[d + 0x100..d + 0x144]));
        assert_eq!(l.gs_ram.as_deref(), Some(&img[d + 0x200..d + 0x500]));
        assert_eq!(l.core_data.as_deref(), Some(&img[d + 0x600..d + 0x723]));
        assert!(l.sound_bank.is_none() && l.hud_header.is_none() && l.hud_banks.iter().all(Option::is_none));
        assert_eq!(l.gameplay_ntsc.as_deref(), Some(&img[1517 * SECTOR_SIZE..1518 * SECTOR_SIZE]));
        assert!(l.gameplay_pal.is_none() && l.occlusion.is_none());
        let names: Vec<String> = l.files().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["level_header.bin", "overlay.bin", "core_index.bin", "gs_ram.bin", "core_data.bin", "gameplay_ntsc.bin"]);
        assert_eq!(l.file("gs_ram.bin").unwrap().len(), 0x300);

        let streams = disc.level_stream_lumps(3).unwrap();
        assert_eq!(streams, [StreamLump { name: "music/000".into(), sector: 1518, bytes: 0x50 }]);
        assert_eq!(disc.read_lump(&streams[0]).unwrap(), &img[1518 * SECTOR_SIZE..1518 * SECTOR_SIZE + 0x50]);
        assert!(disc.level(4).is_err());
    }
}
