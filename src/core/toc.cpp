#include "core/toc.h"
#include "core/wad.h"
#include <cstdio>
#include <cstring>

namespace rc {

const std::vector<GlobalField>& rac1_global_fields() {
    using K = EntryKind;
    static const std::vector<GlobalField> fields = {
        {"debug_font", 0x0008, 1, K::SectorRange},
        {"save_game", 0x0010, 1, K::SectorRange},
        {"ratchet_seqs", 0x0018, 28, K::SectorRange},
        {"hud_seqs", 0x00f8, 20, K::SectorRange},
        {"vendor", 0x0198, 1, K::SectorRange},
        {"vendor_audio", 0x01a0, 37, K::SectorRange},
        {"help_controls", 0x02c8, 12, K::SectorRange},
        {"help_moves", 0x0328, 15, K::SectorRange},
        {"help_weapons", 0x03a0, 15, K::SectorRange},
        {"help_gadgets", 0x0418, 14, K::SectorRange},
        {"help_ss", 0x0488, 7, K::SectorRange},
        {"options_ss", 0x04c0, 7, K::SectorRange},
        {"frontbin", 0x04f8, 1, K::SectorRange},
        {"mission_ss", 0x0500, 81, K::SectorRange},
        {"planets", 0x0788, 19, K::SectorRange},
        {"unknown_0820", 0x0820, 38, K::SectorRange},
        {"goodies_images", 0x0950, 10, K::SectorRange},
        {"character_sketches", 0x09a0, 19, K::SectorRange},
        {"character_renders", 0x0a38, 19, K::SectorRange},
        {"skill_images", 0x0ad0, 31, K::SectorRange},
        {"epilogue_english", 0x0bc8, 12, K::SectorRange},
        {"epilogue_french", 0x0c28, 12, K::SectorRange},
        {"epilogue_italian", 0x0c88, 12, K::SectorRange},
        {"epilogue_german", 0x0ce8, 12, K::SectorRange},
        {"epilogue_spanish", 0x0d48, 12, K::SectorRange},
        {"sketchbook", 0x0da8, 30, K::SectorRange},
        {"commercials", 0x0e98, 4, K::SectorRange},
        {"item_images", 0x0eb8, 9, K::SectorRange},
        {"qwark_boss_audio", 0x0f00, 240, K::Sector32},
        {"irx", 0x12c0, 1, K::SectorRange},
        {"spaceships", 0x12c8, 4, K::SectorRange},
        {"unknown_12e8", 0x12e8, 20, K::SectorRange},
        {"space_plates", 0x1388, 6, K::SectorRange},
        {"transition", 0x13b8, 1, K::SectorRange},
        {"space_audio", 0x13c0, 36, K::SectorRange},
        {"sound_bank", 0x14e0, 1, K::SectorRange},
        {"unknown_14e8", 0x14e8, 1, K::SectorRange},
        {"music", 0x14f0, 1, K::SectorRange},
        {"hud_header", 0x14f8, 1, K::SectorRange},
        {"hud_banks", 0x1500, 5, K::SectorRange},
        {"all_text", 0x1528, 1, K::SectorRange},
        {"unknown_1530", 0x1530, 28, K::SectorRange},
        {"post_credits_helpdesk_girl_seq", 0x1610, 1, K::SectorRange},
        {"post_credits_audio", 0x1618, 18, K::SectorRange},
        {"credits_images_ntsc", 0x16a8, 20, K::SectorRange},
        {"credits_images_pal", 0x1748, 20, K::SectorRange},
        {"unknown_17e8", 0x17e8, 2, K::SectorRange},
        {"mpegs", 0x17f8, 88, K::SectorByteRange},
        {"help_audio", 0x1ab8, 900, K::Sector32},
        {"levels", 0x28c8, 19, K::SectorRange},
    };
    return fields;
}

TableOfContents read_rac1_toc(const IsoImage& iso) {
    TableOfContents toc;
    std::vector<u8> head = iso.read_sectors(RAC1_TOC_SECTOR, 1);
    Buffer hb(head);
    s32 version = hb.read<s32>(0);
    s32 header_size = hb.read<s32>(4);
    if (version != 1) throw FormatError("TOC: version " + std::to_string(version) + " != 1; not a RAC1 disc?");
    if (header_size <= 8 || header_size > 0x200000) throw FormatError("TOC: implausible header size");
    if (header_size != s32(RAC1_TOC_SIZE)) {
        std::fprintf(stderr, "warning: TOC header size 0x%x differs from retail 0x%x\n", header_size, RAC1_TOC_SIZE);
    }
    toc.raw = iso.read_bytes(u64(RAC1_TOC_SECTOR) * SECTOR_SIZE, u64(header_size));
    Buffer b(toc.raw);

    // Level table: each SectorRange offset points at an amalgamated level header
    // whose second word is its own size. Validate that signature.
    const GlobalField* lv = nullptr;
    for (const auto& f : rac1_global_fields()) if (std::strcmp(f.name, "levels") == 0) lv = &f;
    for (u32 i = 0; i < lv->count; i++) {
        s32 sector = b.read<s32>(lv->offset + i * 8);
        if (sector == 0) continue;
        if (u64(sector) + 5 > iso.sector_count()) continue;
        std::vector<u8> hdr = iso.read_bytes(u64(sector) * SECTOR_SIZE, RAC1_LEVEL_HEADER_SIZE);
        Buffer h(hdr);
        if (h.read<s32>(4) != s32(RAC1_LEVEL_HEADER_SIZE)) {
            std::fprintf(stderr, "warning: level table entry %u at sector %d lacks the 0x2434 signature\n", i, sector);
            continue;
        }
        Level level;
        level.table_index = i;
        level.header_sector = u32(sector);
        std::memcpy(&level.header, hdr.data(), sizeof(LevelHeader));
        toc.levels.push_back(level);
    }
    return toc;
}

static u32 be32(const u8* p) { return u32(p[0]) << 24 | u32(p[1]) << 16 | u32(p[2]) << 8 | u32(p[3]); }

Lump probe_lump(const IsoImage& iso, const std::string& name, u32 sector) {
    Lump l;
    l.name = name;
    l.sector = sector;
    std::vector<u8> head = iso.read_sectors(sector, 1);
    if (head.size() >= 0x30 && std::memcmp(head.data(), "VAGp", 4) == 0) {
        l.bytes = 0x30 + u64(be32(head.data() + 0x0c));
        l.size_known = true;
    } else if (is_wad(head)) {
        l.bytes = wad_compressed_size(head);
        l.size_known = true;
    } else {
        l.bytes = SECTOR_SIZE;
        l.size_known = false;
    }
    return l;
}

static void push_range(std::vector<Lump>& out, const std::string& name, s32 sector, u64 bytes, bool known) {
    if (sector == 0 && bytes == 0) return;
    Lump l;
    l.name = name;
    l.sector = u32(sector);
    l.bytes = bytes;
    l.size_known = known;
    out.push_back(l);
}

static std::string idx(const std::string& base, u32 i, u32 count) {
    if (count == 1) return base;
    char buf[16];
    std::snprintf(buf, sizeof buf, "/%03u", i);
    return base + buf;
}

std::vector<Lump> global_lumps(const IsoImage& iso, const TableOfContents& toc) {
    std::vector<Lump> out;
    Buffer b(toc.raw);
    for (const auto& f : rac1_global_fields()) {
        if (std::strcmp(f.name, "levels") == 0) continue;
        for (u32 i = 0; i < f.count; i++) {
            std::string name = idx(f.name, i, f.count);
            switch (f.kind) {
            case EntryKind::SectorRange: {
                s32 off = b.read<s32>(f.offset + i * 8), size = b.read<s32>(f.offset + i * 8 + 4);
                push_range(out, name, off, u64(size) * SECTOR_SIZE, false);
                break;
            }
            case EntryKind::SectorByteRange: {
                s32 off = b.read<s32>(f.offset + i * 8), size = b.read<s32>(f.offset + i * 8 + 4);
                push_range(out, name, off, u64(size), true);
                break;
            }
            case EntryKind::Sector32: {
                s32 off = b.read<s32>(f.offset + i * 4);
                if (off != 0) out.push_back(probe_lump(iso, name, u32(off)));
                break;
            }
            }
        }
    }
    return out;
}

std::vector<Lump> level_lumps(const IsoImage& iso, const Level& level) {
    std::vector<Lump> out;
    const LevelHeader& h = level.header;
    char base[32];
    std::snprintf(base, sizeof base, "level%02d", h.id);
    std::string p = base;
    push_range(out, p + "/data", h.data.offset, u64(h.data.size) * SECTOR_SIZE, false);
    push_range(out, p + "/gameplay_ntsc", h.gameplay_ntsc.offset, u64(h.gameplay_ntsc.size) * SECTOR_SIZE, false);
    push_range(out, p + "/gameplay_pal", h.gameplay_pal.offset, u64(h.gameplay_pal.size) * SECTOR_SIZE, false);
    push_range(out, p + "/occlusion", h.occlusion.offset, u64(h.occlusion.size) * SECTOR_SIZE, false);
    for (u32 i = 0; i < 36; i++) push_range(out, idx(p + "/bindata", i, 36), h.bindata[i].offset, u64(h.bindata[i].size), true);
    for (u32 i = 0; i < 15; i++) if (h.music[i]) out.push_back(probe_lump(iso, idx(p + "/music", i, 15), u32(h.music[i])));
    // Scenes: speech/KK_<lang> per language (probed VAG size), then scene/KK_ntsc and scene/KK_pal, each
    // the region's whole contiguous sector run (every chunk WAD plus the sentinel sector).
    for (u32 k = 0; k < RAC1_SCENE_RECORDS; k++) {
        const SceneRecord& r = h.scenes[k];
        char nb[32];
        for (u32 l = 0; l < 6; l++) {
            if (!r.speech[l]) continue;
            std::snprintf(nb, sizeof nb, "/speech/%02u_%s", k, RAC1_SCENE_LANGUAGES[l]);
            out.push_back(probe_lump(iso, p + nb, u32(r.speech[l])));
        }
        const s32* regions[2] = {r.ntsc, r.pal};
        for (u32 g = 0; g < 2; g++) {
            u32 n = scene_region_entries(regions[g]);
            if (n == 0) continue;
            std::snprintf(nb, sizeof nb, "/scene/%02u_%s", k, g ? "pal" : "ntsc");
            u32 first = u32(regions[g][0]), last = u32(regions[g][n - 1]);
            push_range(out, p + nb, s32(first), u64(last - first + 1) * SECTOR_SIZE, true);
        }
    }
    return out;
}

const char* const RAC1_SCENE_LANGUAGES[6] = {"en", "l1", "fr", "de", "es", "it"};

u32 scene_region_entries(const s32* sectors) {
    u32 n = 0;
    while (n < RAC1_SCENE_CHUNK_SLOTS && sectors[n] != 0) n++;
    return n;
}

} // namespace rc
