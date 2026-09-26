#pragma once
#include <string>
#include <vector>
#include "core/iso9660.h"

namespace rc {

// RAC1 table of contents. Lives at a fixed sector; every pointer in it is an
// absolute sector number. Spec: docs/formats/disc_layout.md section 2.
constexpr u32 RAC1_TOC_SECTOR = 1500;
constexpr u32 RAC1_TOC_SIZE = 0x2960;
constexpr u32 RAC1_LEVEL_HEADER_SIZE = 0x2434;

struct SectorRange { s32 offset = 0; s32 size = 0; };      // both in sectors
struct SectorByteRange { s32 offset = 0; s32 size = 0; };  // offset in sectors, size in bytes

enum class EntryKind { SectorRange, SectorByteRange, Sector32 };

// One field of the global header: `count` entries of `kind` starting at `offset`.
struct GlobalField {
    const char* name;
    u32 offset;
    u32 count;
    EntryKind kind;
};
const std::vector<GlobalField>& rac1_global_fields();

// A single resolved lump on disc.
struct Lump {
    std::string name;   // e.g. "hud_seqs/03", "level05/data"
    u32 sector = 0;
    u64 bytes = 0;      // exact bytes when known, else sectors*0x800
    bool size_known = false;
};

// One in-engine scene record (0x250 bytes; docs/plan/cutscenes_transitions.md section 1): speech VAG
// sector per language (0 En, 1 unused, 2 Fr, 3 De, 4 Es, 5 It), then the NTSC and PAL chunk WAD sectors.
// A chunk's size is the sector difference to the next entry; the last entry is a 1-sector sentinel.
// (Wrench reads this block as 30 x 0x128 {sounds[6], wads[68]}.)
constexpr u32 RAC1_SCENE_RECORDS = 15;
constexpr u32 RAC1_SCENE_CHUNK_SLOTS = 71;
struct SceneRecord {
    s32 speech[6];
    s32 ntsc[RAC1_SCENE_CHUNK_SLOTS];
    s32 pal[RAC1_SCENE_CHUNK_SLOTS];
};
static_assert(sizeof(SceneRecord) == 0x250);
// File-name language codes of SceneRecord::speech (extracted `speech/KK_<lang>.bin`).
extern const char* const RAC1_SCENE_LANGUAGES[6];
// Number of leading non-zero entries of a region's chunk list.
u32 scene_region_entries(const s32* sectors);

struct LevelHeader {
    s32 id;
    s32 header_size;
    SectorRange data;
    SectorRange gameplay_ntsc;
    SectorRange gameplay_pal;
    SectorRange occlusion;
    SectorByteRange bindata[36];
    s32 music[15];
    SceneRecord scenes[RAC1_SCENE_RECORDS];
};
static_assert(sizeof(LevelHeader) == RAC1_LEVEL_HEADER_SIZE);

struct Level {
    u32 table_index = 0;
    u32 header_sector = 0;
    LevelHeader header{};
};

struct TableOfContents {
    std::vector<u8> raw;          // the 0x2960 bytes
    std::vector<Level> levels;    // discovered in table order
};

TableOfContents read_rac1_toc(const IsoImage& iso);

// Resolves a bare sector pointer's size by probing for VAG or WAD headers.
Lump probe_lump(const IsoImage& iso, const std::string& name, u32 sector);

// Enumerates every global lump (non-level) in the TOC.
std::vector<Lump> global_lumps(const IsoImage& iso, const TableOfContents& toc);
// Enumerates every lump belonging to a level.
std::vector<Lump> level_lumps(const IsoImage& iso, const Level& level);

} // namespace rc
