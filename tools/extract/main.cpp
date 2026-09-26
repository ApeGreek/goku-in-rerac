// rc_extract: reads the user's own Ratchet & Clank disc image and unpacks it.
//
// Stage plan (each stage is a subcommand so they can be tested independently):
//   info      identify the disc (serial, region, volume id, sector count)
//   ls        list ISO 9660 entries
//   toc       parse the table of contents (levels + global wads)
//   unpack    write every wad / lump to the output directory
#include <cstdio>
#include <cstring>
#include <string>
#include "core/iso9660.h"
#include "core/toc.h"
#include "core/level.h"
#include "core/level_core.h"
#include "core/texture.h"
#include "core/tfrag.h"
#include "core/tie.h"
#include "core/shrub.h"
#include "core/sky.h"
#include "core/moby.h"
#include "core/gadget.h"
#include "core/collision.h"
#include "core/occlusion.h"
#include "core/particle.h"
#include "core/hud.h"
#include "core/sound.h"
#include <cmath>
#include "core/wad.h"
#include <filesystem>
#include <map>
#include <algorithm>

namespace {

int usage() {
    std::fprintf(stderr,
        "usage: rc_extract <command> --iso <path> [--out <dir>]\n"
        "commands: info, ls, toc, unpack, textures, tfrag, tie, shrub, sky, moby, gadget, collision, occlusion, particles, sound, hud, scene\n");
    return 2;
}

struct Args {
    std::string command, iso, out = "extracted";
    int level = 1;
};

bool parse_args(int argc, char** argv, Args& a) {
    if (argc < 2) return false;
    a.command = argv[1];
    for (int i = 2; i < argc; i++) {
        std::string s = argv[i];
        auto next = [&](std::string& dst) { if (i + 1 >= argc) return false; dst = argv[++i]; return true; };
        if (s == "--iso") { if (!next(a.iso)) return false; }
        else if (s == "--out") { if (!next(a.out)) return false; }
        else if (s == "--level") { std::string v; if (!next(v)) return false; a.level = std::atoi(v.c_str()); }
        else return false;
    }
    if (a.iso.empty()) {
        if (const char* env = std::getenv("RC_ISO")) a.iso = env;
    }
    return !a.iso.empty() || a.command == "textures" || a.command == "tfrag" || a.command == "tie" || a.command == "shrub" || a.command == "sky" || a.command == "moby" || a.command == "gadget" || a.command == "collision" || a.command == "occlusion" || a.command == "particles" || a.command == "sound" || a.command == "hud" || a.command == "scene";
}

int cmd_info(const rc::IsoImage& iso) {
    std::printf("volume id      : %s\n", iso.volume_id().c_str());
    std::printf("raw sector size: %u\n", iso.raw_sector_size());
    std::printf("sectors        : %u (%.1f MiB)\n", iso.sector_count(),
                double(iso.sector_count()) * rc::SECTOR_SIZE / (1024.0 * 1024.0));
    if (const rc::IsoEntry* cnf = iso.find("/SYSTEM.CNF")) {
        std::vector<rc::u8> bytes = iso.read_file(*cnf);
        std::printf("SYSTEM.CNF     :\n%.*s\n", int(bytes.size()), reinterpret_cast<const char*>(bytes.data()));
    } else {
        std::printf("SYSTEM.CNF     : missing\n");
    }
    return 0;
}

int cmd_toc(const rc::IsoImage& iso) {
    rc::TableOfContents toc = rc::read_rac1_toc(iso);
    std::printf("TOC at sector %u, %zu bytes, %zu levels\n", rc::RAC1_TOC_SECTOR, toc.raw.size(), toc.levels.size());
    std::vector<rc::Lump> lumps = rc::global_lumps(iso, toc);
    rc::u64 total = 0;
    for (const auto& l : lumps) { std::printf("  %-40s sector %8u  %10llu bytes%s\n", l.name.c_str(), l.sector, (unsigned long long)l.bytes, l.size_known ? "" : " (sectors)"); total += l.bytes; }
    std::printf("global lumps: %zu, %.1f MiB\n", lumps.size(), double(total) / (1024.0 * 1024.0));
    for (const auto& lv : toc.levels) {
        const auto& h = lv.header;
        std::vector<rc::Lump> ll = rc::level_lumps(iso, lv);
        rc::u64 lt = 0; for (const auto& l : ll) lt += l.bytes;
        std::printf("level table[%2u] id=%2d header@%8u data@%8u+%6d gameplay@%8u+%5d occl@%8u+%4d  lumps=%3zu (%.1f MiB)\n",
            lv.table_index, h.id, lv.header_sector, h.data.offset, h.data.size, h.gameplay_ntsc.offset, h.gameplay_ntsc.size,
            h.occlusion.offset, h.occlusion.size, ll.size(), double(lt) / (1024.0 * 1024.0));
    }
    return 0;
}

struct UnpackStats { size_t lumps = 0, wads = 0, failed = 0; };

// Writes a lump as raw bytes and, if it is a WAD stream, a decompressed copy next to it.
void write_lump(const std::string& path, const std::vector<rc::u8>& bytes, UnpackStats& st, bool force_decompress = false) {
    rc::write_file(path + ".bin", bytes);
    st.lumps++;
    if (rc::is_wad(bytes) || force_decompress) {
        try {
            rc::write_file(path + ".dec", rc::wad_decompress(bytes));
            st.wads++;
        } catch (const std::exception& e) {
            st.failed++;
            std::fprintf(stderr, "WAD decode failed for %s: %s\n", path.c_str(), e.what());
        }
    }
}

void unpack_level(const rc::IsoImage& iso, const rc::Level& lv, const std::string& out, rc::u32 e_flags, UnpackStats& st) {
    char nn[16];
    std::snprintf(nn, sizeof nn, "/levels/%02d", lv.header.id);
    const rc::LevelHeader& h = lv.header;
    std::string d = out + nn;  // (a fixed char[64] here silently truncated long --out paths)
    rc::write_file(d + "/level_header.bin", iso.read_bytes(rc::u64(lv.header_sector) * rc::SECTOR_SIZE, rc::RAC1_LEVEL_HEADER_SIZE));

    // The data WAD: an uncompressed container with byte-offset lumps.
    std::vector<rc::u8> data = iso.read_bytes(rc::u64(h.data.offset) * rc::SECTOR_SIZE, rc::u64(h.data.size) * rc::SECTOR_SIZE);
    rc::Buffer db(data);
    rc::LevelDataHeader dh = rc::read_level_data_header(db);
    auto lump = [&](const rc::ByteRange& r) { return r.present() ? db.copy(size_t(r.offset), size_t(r.size)) : std::vector<rc::u8>{}; };

    if (dh.overlay.present()) {
        std::vector<rc::u8> ov = lump(dh.overlay);
        write_lump(d + "/overlay", ov, st);
        std::vector<rc::OverlaySection> secs = rc::parse_ratchet_executable(rc::Buffer(ov));
        rc::write_file(d + "/overlay.elf", rc::write_overlay_elf(secs, e_flags));
        std::string txt;
        for (size_t i = 0; i < secs.size(); i++) {
            char line[128];
            std::snprintf(line, sizeof line, "section %zu type=%u dest=0x%08x size=0x%x entry=0x%08x\n", i, secs[i].section_type, secs[i].dest_address, unsigned(secs[i].data.size()), secs[i].entry_point);
            txt += line;
        }
        rc::write_file(d + "/overlay.txt", std::vector<rc::u8>(txt.begin(), txt.end()));
    }
    if (dh.sound_bank.present()) write_lump(d + "/sound_bank", lump(dh.sound_bank), st);
    if (dh.core_index.present()) write_lump(d + "/core_index", lump(dh.core_index), st);
    if (dh.gs_ram.present()) write_lump(d + "/gs_ram", lump(dh.gs_ram), st);
    if (dh.hud_header.present()) write_lump(d + "/hud_header", lump(dh.hud_header), st);
    for (int i = 0; i < 5; i++) if (dh.hud_banks[i].present()) write_lump(d + "/hud_bank_" + std::to_string(i), lump(dh.hud_banks[i]), st);
    if (dh.core_data.present()) {
        std::vector<rc::u8> cd = lump(dh.core_data);
        write_lump(d + "/core_data", cd, st);
        if (dh.core_index.present() && rc::is_wad(cd)) {
            std::vector<rc::u8> dec = rc::wad_decompress(cd);
            std::vector<rc::u8> ci = lump(dh.core_index);
            rc::LevelCore core = rc::parse_level_core(rc::Buffer(ci), dec.size());
            std::string txt;
            char line[256];
            for (const auto& b : core.blocks) {
                if (b.size < 0 || rc::u64(b.offset) + rc::u64(b.size) > dec.size()) {
                    std::snprintf(line, sizeof line, "BAD %-24s offset=0x%08x size=%d\n", b.name.c_str(), b.offset, b.size);
                    txt += line;
                    st.failed++;
                    continue;
                }
                rc::write_file(d + "/core/" + b.name + ".bin", rc::Buffer(dec).copy(size_t(b.offset), size_t(b.size)));
                std::snprintf(line, sizeof line, "%-24s offset=0x%08x size=0x%x\n", b.name.c_str(), b.offset, b.size);
                txt += line;
            }
            std::snprintf(line, sizeof line, "\ngs_ram=%d moby_classes=%zu tie_classes=%zu shrub_classes=%zu textures tfrag=%zu moby=%zu tie=%zu shrub=%zu gadgets=%zu ratchet_seqs=%zu\n",
                core.header.gs_ram.count, core.moby_classes.size(), core.tie_classes.size(), core.shrub_classes.size(),
                core.tfrag_textures.size(), core.moby_textures.size(), core.tie_textures.size(), core.shrub_textures.size(),
                core.gadgets.size(), size_t(std::count_if(core.ratchet_seqs.begin(), core.ratchet_seqs.end(), [](rc::s32 v) { return v > 0; })));
            txt += line;
            rc::write_file(d + "/core/index.txt", std::vector<rc::u8>(txt.begin(), txt.end()));
        }
    }

    auto sector_lump = [&](const char* name, const rc::SectorRange& r) {
        if (r.offset == 0 && r.size == 0) return;
        write_lump(d + "/" + name, iso.read_bytes(rc::u64(r.offset) * rc::SECTOR_SIZE, rc::u64(r.size) * rc::SECTOR_SIZE), st);
    };
    sector_lump("gameplay_ntsc", h.gameplay_ntsc);
    // Split the decompressed gameplay file into its 37 pointer-table sections (spec 3.2).
    if (h.gameplay_ntsc.offset) {
        static const char* kSections[37] = {"level_settings", "directional_lights", "cameras", "sound_instances",
            "help_us_english", "help_uk_english", "help_french", "help_german", "help_spanish", "help_italian", "help_japanese", "help_korean",
            "tie_classes", "tie_instances", "shrub_classes", "shrub_instances", "moby_classes", "moby_instances", "moby_groups",
            "shared_data", "pvar_moby_links", "pvar_table", "pvar_data", "pvar_pointer_fixups",
            "cuboids", "spheres", "cylinders", "pills", "paths", "grind_paths", "point_light_grid", "point_lights",
            "env_transitions", "camera_collision_grid", "env_sample_points", "occlusion_mappings", "unused_90"};
        std::vector<rc::u8> gp = iso.read_bytes(rc::u64(h.gameplay_ntsc.offset) * rc::SECTOR_SIZE, rc::u64(h.gameplay_ntsc.size) * rc::SECTOR_SIZE);
        if (rc::is_wad(gp)) {
            std::vector<rc::u8> dec = rc::wad_decompress(gp);
            rc::Buffer b(dec);
            std::vector<std::pair<rc::s32, int>> starts;
            for (int i = 0; i < 37; i++) { rc::s32 o = b.read<rc::s32>(size_t(i) * 4); if (o > 0) starts.push_back({o, i}); }
            std::sort(starts.begin(), starts.end());
            std::string txt;
            for (size_t k = 0; k < starts.size(); k++) {
                rc::s32 start = starts[k].first;
                rc::s32 end = k + 1 < starts.size() ? starts[k + 1].first : rc::s32(dec.size());
                if (start < 0 || end < start || rc::u64(end) > dec.size()) { st.failed++; continue; }
                rc::write_file(d + "/gameplay/" + kSections[starts[k].second] + ".bin", b.copy(size_t(start), size_t(end - start)));
                char line[128];
                std::snprintf(line, sizeof line, "%-24s offset=0x%06x size=0x%x\n", kSections[starts[k].second], start, end - start);
                txt += line;
            }
            rc::write_file(d + "/gameplay/index.txt", std::vector<rc::u8>(txt.begin(), txt.end()));
        }
    }
    sector_lump("gameplay_pal", h.gameplay_pal);
    sector_lump("occlusion", h.occlusion);

    // Audio and scene groups: raw sector lumps, sizes probed from VAG/WAD headers.
    // Scene regions (scene/KK_ntsc, scene/KK_pal) hold several chunk WADs each: raw only, no .dec.
    for (const auto& l : rc::level_lumps(iso, lv)) {
        std::string n = l.name.substr(l.name.find('/') + 1);
        if (n == "data" || n == "gameplay_ntsc" || n == "gameplay_pal" || n == "occlusion") continue;
        std::vector<rc::u8> bytes = iso.read_bytes(rc::u64(l.sector) * rc::SECTOR_SIZE, l.bytes);
        if (n.rfind("scene/", 0) == 0) { rc::write_file(d + "/" + n + ".bin", bytes); st.lumps++; continue; }
        write_lump(d + "/" + n, bytes, st);
    }
}

// Decodes every level texture (tfrag/moby/tie/shrub tables, shrub billboards)
// to PNG under <out>/levels/NN/textures/. Reads from the unpacked lumps.
// Also writes textures/rgba.bin, the same images uncompressed, as the golden
// reference for the Rust port (crates/rc-formats/tests/golden.rs):
//   "RCTX" u32 count, then per texture in PNG-writing order:
//   u32 name_len, name (PNG path under textures/ without ".png"), u32 width, u32 height, width*height*4 RGBA bytes.
int cmd_textures(const std::string& out) {
    namespace fs = std::filesystem;
    size_t total = 0;
    for (const auto& lvdir : fs::directory_iterator(out + "/levels")) {
        std::string d = lvdir.path().string();
        if (!fs::exists(d + "/core_index.bin")) continue;
        std::vector<rc::u8> ci = rc::read_file(d + "/core_index.bin");
        std::vector<rc::u8> gs = rc::read_file(d + "/gs_ram.bin");
        std::vector<rc::u8> tex = rc::read_file(d + "/core/textures.bin");
        rc::LevelCore core = rc::parse_level_core(rc::Buffer(ci), 0);
        rc::Buffer gsb(gs), texb(tex);
        std::vector<rc::u8> raw = {'R', 'C', 'T', 'X', 0, 0, 0, 0};
        rc::u32 raw_count = 0;
        auto put32 = [&](rc::u32 v) { for (int k = 0; k < 4; k++) raw.push_back(rc::u8(v >> (8 * k))); };
        auto emit = [&](const char* name, const rc::Image& img) {   // name: "/textures/<kind>/<stem>.png"
            std::string stem = std::string(name + std::strlen("/textures/"));
            stem.resize(stem.size() - 4);
            put32(rc::u32(stem.size()));
            raw.insert(raw.end(), stem.begin(), stem.end());
            put32(img.width); put32(img.height);
            raw.insert(raw.end(), img.rgba.begin(), img.rgba.end());
            raw_count++;
            rc::write_png(d + name, img);
            total++;
        };
        auto dump = [&](const char* kind, const std::vector<rc::TextureEntry>& table) {
            for (size_t i = 0; i < table.size(); i++) {
                const rc::TextureEntry& e = table[i];
                if (e.width <= 0 || e.height <= 0 || e.data_offset < 0 || e.palette < 0) continue;
                try {
                    rc::Image img = rc::decode_indexed8(texb.sub(size_t(e.data_offset), size_t(e.width) * e.height).bytes(),
                                                        rc::u32(e.width), rc::u32(e.height), gsb.sub(size_t(e.palette) * 0x100, 1024).bytes());
                    char name[64];
                    std::snprintf(name, sizeof name, "/textures/%s/%03zu_%dx%d_t%d.png", kind, i, e.width, e.height, e.type);
                    emit(name, img);
                } catch (const std::exception& ex) {
                    std::fprintf(stderr, "%s %s[%zu]: %s\n", d.c_str(), kind, i, ex.what());
                }
            }
        };
        dump("tfrag", core.tfrag_textures);
        dump("moby", core.moby_textures);
        dump("tie", core.tie_textures);
        dump("shrub", core.shrub_textures);
        for (size_t i = 0; i < core.shrub_classes.size(); i++) {
            const rc::ShrubBillboardInfo& b = core.shrub_classes[i].billboard;
            if (b.width <= 0 || b.height <= 0) continue;
            try {
                rc::Image img = rc::decode_indexed8(gsb.sub(size_t(b.texture_offset) * 0x100, size_t(b.width) * b.height).bytes(),
                                                    rc::u32(b.width), rc::u32(b.height), gsb.sub(size_t(b.palette_offset) * 0x100, 1024).bytes());
                char name[64];
                std::snprintf(name, sizeof name, "/textures/billboard/%04d_%dx%d.png", core.shrub_classes[i].base.o_class, b.width, b.height);
                emit(name, img);
            } catch (const std::exception& ex) {
                std::fprintf(stderr, "%s billboard[%zu]: %s\n", d.c_str(), i, ex.what());
            }
        }
        for (int k = 0; k < 4; k++) raw[4 + k] = rc::u8(raw_count >> (8 * k));
        rc::write_file(d + "/textures/rgba.bin", raw);
    }
    std::printf("wrote %zu textures\n", total);
    return 0;
}

// Top-down painter's raster used to eyeball geometry. X right, Y up, Z sorted.
struct TopDown {
    struct Tri { float x[3], y[3], z[3]; rc::u8 rgb[3]; };
    std::vector<Tri> tris;
    void add(const float* x, const float* y, const float* z, rc::u8 r, rc::u8 g, rc::u8 b) {
        Tri t; for (int k = 0; k < 3; k++) { t.x[k] = x[k]; t.y[k] = y[k]; t.z[k] = z[k]; } t.rgb[0] = r; t.rgb[1] = g; t.rgb[2] = b; tris.push_back(t);
    }
    void write(const std::string& path, rc::u32 W = 1024) {
        float minx = 1e30f, miny = 1e30f, maxx = -1e30f, maxy = -1e30f;
        for (const Tri& t : tris) for (int k = 0; k < 3; k++) { minx = std::min(minx, t.x[k]); maxx = std::max(maxx, t.x[k]); miny = std::min(miny, t.y[k]); maxy = std::max(maxy, t.y[k]); }
        float span = std::max(maxx - minx, maxy - miny) + 1e-3f;
        rc::u32 H = rc::u32(std::ceil((maxy - miny) / span * W)) + 1;
        rc::Image img; img.width = W; img.height = H; img.rgba.assign(size_t(W) * H * 4, 0);
        for (size_t i = 0; i < img.rgba.size(); i += 4) img.rgba[i + 3] = 255;
        std::sort(tris.begin(), tris.end(), [](const Tri& a, const Tri& b) { return a.z[0] + a.z[1] + a.z[2] < b.z[0] + b.z[1] + b.z[2]; });
        for (const Tri& r : tris) {
            float px[3], py[3];
            for (int k = 0; k < 3; k++) { px[k] = (r.x[k] - minx) / span * (W - 1); py[k] = (H - 1) - (r.y[k] - miny) / span * (W - 1); }
            int x0 = std::max(0, int(std::floor(std::min({px[0], px[1], px[2]})))), x1 = std::min(int(W) - 1, int(std::ceil(std::max({px[0], px[1], px[2]}))));
            int y0 = std::max(0, int(std::floor(std::min({py[0], py[1], py[2]})))), y1 = std::min(int(H) - 1, int(std::ceil(std::max({py[0], py[1], py[2]}))));
            float area = (px[1] - px[0]) * (py[2] - py[0]) - (px[2] - px[0]) * (py[1] - py[0]);
            if (std::fabs(area) < 1e-6f) continue;
            for (int y = y0; y <= y1; y++) for (int x = x0; x <= x1; x++) {
                float cx = x + 0.5f, cy = y + 0.5f;
                float w0 = ((px[1] - cx) * (py[2] - cy) - (px[2] - cx) * (py[1] - cy)) / area;
                float w1 = ((px[2] - cx) * (py[0] - cy) - (px[0] - cx) * (py[2] - cy)) / area;
                float w2 = 1 - w0 - w1;
                if (w0 < 0 || w1 < 0 || w2 < 0) continue;
                rc::u8* p4 = &img.rgba[(size_t(y) * W + x) * 4];
                p4[0] = r.rgb[0]; p4[1] = r.rgb[1]; p4[2] = r.rgb[2];
            }
        }
        rc::write_png(path, img);
    }
};

// Parses one level's tfrag block, validates it against the header counts, writes
// an OBJ of LOD 0 and a top-down raster PNG for eyeballing.
int cmd_tfrag(const std::string& out, int level) {
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::vector<rc::u8> blk = rc::read_file(std::string(d) + "/core/tfrags.bin");
    std::vector<rc::Tfrag> tfrags = rc::parse_tfrags(rc::Buffer(blk));
    size_t bad_vert = 0, bad_tri = 0, tri_total = 0, vinfo_total = 0;
    std::string obj = "# randcre tfrag LOD0 export\n";
    size_t vbase = 1;
    float minx = 1e30f, miny = 1e30f, maxx = -1e30f, maxy = -1e30f, minz = 1e30f, maxz = -1e30f;
    TopDown raster;
    for (size_t ti = 0; ti < tfrags.size(); ti++) {
        const rc::Tfrag& t = tfrags[ti];
        std::vector<rc::TfragTriangle> tris = rc::tfrag_triangles(t, 0);
        if (t.positions.size() != t.header.vert_count) bad_vert++;
        if (tris.size() != t.header.tri_count) bad_tri++;
        tri_total += tris.size();
        vinfo_total += t.vertex_info.size();
        obj += "o tfrag_" + std::to_string(ti) + "\n";
        std::vector<float> wx(t.vertex_info.size()), wy(t.vertex_info.size()), wz(t.vertex_info.size());
        for (size_t i = 0; i < t.vertex_info.size(); i++) {
            const rc::TfragVertexInfo& vi = t.vertex_info[i];
            size_t pi = size_t(vi.vertex) / 2;
            if (pi >= t.positions.size()) { pi = 0; }
            const rc::TfragPosition& p = t.positions[pi];
            wx[i] = float(t.origin[0] + p.x) / 1024.0f; wy[i] = float(t.origin[1] + p.y) / 1024.0f; wz[i] = float(t.origin[2] + p.z) / 1024.0f;
            minx = std::min(minx, wx[i]); maxx = std::max(maxx, wx[i]); miny = std::min(miny, wy[i]); maxy = std::max(maxy, wy[i]); minz = std::min(minz, wz[i]); maxz = std::max(maxz, wz[i]);
            char line[160];
            std::snprintf(line, sizeof line, "v %.4f %.4f %.4f\nvt %.5f %.5f\n", wx[i], wy[i], wz[i], vi.s / 4096.0f, vi.t / 4096.0f);
            obj += line;
        }
        int last_mat = -1;
        for (const rc::TfragTriangle& tr : tris) {
            int mat = tr.ad_gif < t.ad_gifs.size() ? t.ad_gifs[tr.ad_gif].tex0.data_lo : -1;
            if (mat != last_mat) { obj += "usemtl tex" + std::to_string(mat) + "\n"; last_mat = mat; }
            char line[96];
            std::snprintf(line, sizeof line, "f %zu/%zu %zu/%zu %zu/%zu\n", vbase + tr.a, vbase + tr.a, vbase + tr.b, vbase + tr.b, vbase + tr.c, vbase + tr.c);
            obj += line;
            float rx[3], ry[3], rz[3];
            rc::u16 idx[3] = {tr.a, tr.b, tr.c};
            for (int k = 0; k < 3; k++) { if (idx[k] >= t.vertex_info.size()) idx[k] = 0; rx[k] = wx[idx[k]]; ry[k] = wy[idx[k]]; rz[k] = wz[idx[k]]; }
            size_t pi = size_t(t.vertex_info[idx[0]].vertex) / 2;
            const rc::TfragRgba c = pi < t.rgba.size() ? t.rgba[pi] : rc::TfragRgba{128, 128, 128, 128};
            raster.add(rx, ry, rz, rc::u8(std::min(255, c.r * 2)), rc::u8(std::min(255, c.g * 2)), rc::u8(std::min(255, c.b * 2)));
        }
        vbase += t.vertex_info.size();
    }
    rc::write_file(std::string(d) + "/tfrag_lod0.obj", std::vector<rc::u8>(obj.begin(), obj.end()));

    // tfrag_dump.bin: every parsed field, the golden reference for the Rust port
    // (crates/rc-formats/tests/golden.rs). "RCTF" u32 tfrag_count, 16-byte block header,
    // then per tfrag 25 sections, each u32 byte_length + raw little-endian bytes:
    //   header(0x40) vu(0x28) origin(4 s32) tier_counts(6 u32: pos common/lod01/lod0, vinfo common/lod01/lod0)
    //   positions vertex_info parent_lod01 unk2_lod01 parent_lod0 unk2_lod0
    //   strips[lod0..2] indices[lod0..2] ad_gifs rgba lights mspheres(raw) cube(raw 0x40)
    //   triangles[lod0..2] (a,b,c,ad_gif u16) world_positions (3 f32 per vertex-info entry, as in the OBJ)
    {
        std::vector<rc::u8> dump = {'R', 'C', 'T', 'F', 0, 0, 0, 0};
        rc::u32 n_tf = rc::u32(tfrags.size());
        std::memcpy(dump.data() + 4, &n_tf, 4);
        rc::Buffer bb(blk);
        std::span<const rc::u8> bhdr = bb.sub(0, 16).bytes();
        dump.insert(dump.end(), bhdr.begin(), bhdr.end());
        rc::s32 table_offset = bb.read<rc::s32>(0);
        auto sec = [&](const void* p, size_t n) {
            rc::u32 len = rc::u32(n);
            const rc::u8* lp = reinterpret_cast<const rc::u8*>(&len);
            dump.insert(dump.end(), lp, lp + 4);
            if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n);
        };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        for (const rc::Tfrag& t : tfrags) {
            const rc::TfragHeader& h = t.header;
            size_t base = size_t(table_offset) + size_t(h.data);
            sec(&h, sizeof h);
            sec(&t.vu, sizeof t.vu);
            sec(t.origin, sizeof t.origin);
            rc::u32 tiers[6] = {t.positions_common, t.positions_lod01, t.positions_lod0, t.vinfo_common, t.vinfo_lod01, t.vinfo_lod0};
            sec(tiers, sizeof tiers);
            vsec(t.positions);
            vsec(t.vertex_info);
            vsec(t.parent_indices_lod01); vsec(t.unk_indices_2_lod01); vsec(t.parent_indices_lod0); vsec(t.unk_indices_2_lod0);
            for (int l = 0; l < 3; l++) vsec(t.lod[l].strips);
            for (int l = 0; l < 3; l++) vsec(t.lod[l].indices);
            vsec(t.ad_gifs); vsec(t.rgba); vsec(t.lights);
            std::span<const rc::u8> ms = bb.sub(base + h.msphere_ofs, size_t(h.msphere_count) * 16).bytes();
            sec(ms.data(), ms.size());
            std::span<const rc::u8> cube = bb.sub(base + h.cube_ofs, 0x40).bytes();
            sec(cube.data(), cube.size());
            for (int l = 0; l < 3; l++) vsec(rc::tfrag_triangles(t, l));
            std::vector<float> world;
            for (size_t i = 0; i < t.vertex_info.size(); i++) {
                size_t pi = size_t(t.vertex_info[i].vertex) / 2;
                if (pi >= t.positions.size()) pi = 0;
                const rc::TfragPosition& p = t.positions[pi];
                world.push_back(float(t.origin[0] + p.x) / 1024.0f);
                world.push_back(float(t.origin[1] + p.y) / 1024.0f);
                world.push_back(float(t.origin[2] + p.z) / 1024.0f);
            }
            vsec(world);
        }
        rc::write_file(std::string(d) + "/tfrag_dump.bin", dump);
    }

    raster.write(std::string(d) + "/tfrag_topdown.png");
    std::printf("level %02d: %zu tfrags, %zu LOD0 triangles, %zu vertex-info entries; vert_count mismatches %zu, tri_count mismatches %zu\n",
                level, tfrags.size(), tri_total, vinfo_total, bad_vert, bad_tri);
    std::printf("world bounds x[%.1f,%.1f] y[%.1f,%.1f] z[%.1f,%.1f]\n", minx, maxx, miny, maxy, minz, maxz);
    return (bad_vert || bad_tri) ? 1 : 0;
}

// Parses every tie class of a level and its instances; writes tie_dump.bin (the golden
// reference for crates/rc-formats/src/tie.rs), ties_lod0.obj and ties_topdown.png.
int cmd_tie(const std::string& out, int level) {
    namespace fs = std::filesystem;
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::map<int, rc::TieClass> classes;
    size_t failed = 0, tris = 0, packets = 0, verts = 0, all_tris = 0, bad_lod_info = 0;
    for (const auto& f : fs::directory_iterator(std::string(d) + "/core/tie_class")) {
        int o_class = std::atoi(f.path().stem().string().c_str());
        try {
            std::vector<rc::u8> blob = rc::read_file(f.path().string());
            rc::TieClass tc = rc::parse_tie_class(rc::Buffer(blob));
            if (tc.header.o_class != o_class) throw rc::FormatError("header o_class mismatch");
            for (int lod = 0; lod < 3; lod++) {
                rc::u32 sv = 0, st = 0, ns = 0;
                for (const auto& pk : tc.lods[lod]) {
                    packets++;
                    verts += pk.vertices.size();
                    size_t n = rc::tie_triangles(pk).size();
                    all_tris += n;
                    if (lod == 0) tris += n;
                    for (const auto& s : pk.strips) { sv += s.vertex_count; st += s.vertex_count - 2u; ns++; }
                }
                const rc::TieLodInfo& li = tc.header.lod_info[lod];
                if (li.strip_vertex_count != sv || li.triangle_count != st || li.strip_count != ns) bad_lod_info++;
            }
            classes[o_class] = std::move(tc);
        } catch (const std::exception& e) {
            failed++;
            std::fprintf(stderr, "tie class %d: %s\n", o_class, e.what());
        }
    }
    std::vector<rc::u8> inst_bytes = rc::read_file(std::string(d) + "/gameplay/tie_instances.bin");
    std::vector<rc::TieInstance> inst = rc::parse_tie_instances(rc::Buffer(inst_bytes));
    size_t unresolved = 0;
    for (const auto& i : inst) if (!classes.count(i.o_class)) unresolved++;

    // tie_dump.bin: "RCTI" u32 class_count, then per class in o_class order: s32 o_class and
    // sections (u32 byte_length + raw little-endian bytes):
    //   header(0x80) header_ext normals(64 x s16[4]) ad_gifs(n x 0x50)
    // then for LOD 0, 1, 2 and each packet (counts from the header):
    //   packet_header(0x10) ad_gif_dest_src(8 s32) unpack(12) strips dinky fat colors colors_b
    //   slot_table vertices(26 bytes each, rc::TieVertex) draws triangles(a,b,c,ad_gif u16)
    // A draw is u8 ad_gif, u8 winding, u16 count, u16 vertex indices[count].
    // Finally u32 instance_count and one section with the raw 0xe0-byte instance records.
    {
        std::vector<rc::u8> dump = {'R', 'C', 'T', 'I', 0, 0, 0, 0};
        rc::u32 n_cls = rc::u32(classes.size());
        std::memcpy(dump.data() + 4, &n_cls, 4);
        auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
        auto sec = [&](const void* p, size_t n) { rc::u32 len = rc::u32(n); raw(&len, 4); raw(p, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        for (const auto& [o_class, tc] : classes) {
            raw(&o_class, 4);
            sec(&tc.header, sizeof tc.header);
            vsec(tc.header_ext);
            vsec(tc.normals);
            vsec(tc.ad_gifs);
            for (int lod = 0; lod < 3; lod++) for (const auto& pk : tc.lods[lod]) {
                sec(&pk.header, sizeof pk.header);
                rc::s32 ad[8];
                std::memcpy(ad, pk.ad_gif_dest, 16);
                std::memcpy(ad + 4, pk.ad_gif_src, 16);
                sec(ad, sizeof ad);
                sec(&pk.unpack, sizeof pk.unpack);
                vsec(pk.strips); vsec(pk.dinky); vsec(pk.fat);
                vsec(pk.colors); vsec(pk.colors_b); vsec(pk.slot_table);
                vsec(pk.vertices);
                std::vector<rc::u8> draws;
                for (const auto& dr : pk.draws) {
                    rc::u16 n = rc::u16(dr.vertices.size());
                    draws.push_back(dr.ad_gif); draws.push_back(dr.winding);
                    draws.push_back(rc::u8(n)); draws.push_back(rc::u8(n >> 8));
                    for (rc::u16 x : dr.vertices) { draws.push_back(rc::u8(x)); draws.push_back(rc::u8(x >> 8)); }
                }
                vsec(draws);
                vsec(rc::tie_triangles(pk));
            }
        }
        rc::u32 n_inst = rc::u32(inst.size());
        raw(&n_inst, 4);
        vsec(inst);
        rc::write_file(std::string(d) + "/tie_dump.bin", dump);
    }

    TopDown raster;
    {   // terrain underlay in grey so tie placement can be judged
        std::vector<rc::u8> blk = rc::read_file(std::string(d) + "/core/tfrags.bin");
        for (const rc::Tfrag& t : rc::parse_tfrags(rc::Buffer(blk))) {
            for (const rc::TfragTriangle& tr : rc::tfrag_triangles(t, 0)) {
                float rx[3], ry[3], rz[3];
                rc::u16 idx[3] = {tr.a, tr.b, tr.c};
                for (int k = 0; k < 3; k++) {
                    size_t pi = size_t(t.vertex_info[idx[k]].vertex) / 2;
                    const rc::TfragPosition& p = t.positions[pi];
                    rx[k] = float(t.origin[0] + p.x) / 1024.0f; ry[k] = float(t.origin[1] + p.y) / 1024.0f; rz[k] = float(t.origin[2] + p.z) / 1024.0f - 1000.0f;
                }
                raster.add(rx, ry, rz, 70, 70, 70);
            }
        }
    }
    // OBJ of all instances, LOD 0 (fat vertices at their base position, i.e. morph factor 0).
    std::string obj;
    size_t vbase = 1;
    for (size_t n = 0; n < inst.size(); n++) {
        const rc::TieInstance& in = inst[n];
        auto it = classes.find(in.o_class);
        if (it == classes.end()) continue;
        const rc::TieClass& tc = it->second;
        float k_scale = tc.header.scale / 1024.0f;
        auto world = [&](const rc::TieVertex& v, float w[3]) {
            float lx = v.x * k_scale, ly = v.y * k_scale, lz = v.z * k_scale;
            for (int r = 0; r < 3; r++) w[r] = in.matrix[0][r] * lx + in.matrix[1][r] * ly + in.matrix[2][r] * lz + in.matrix[3][r];
        };
        obj += "o tie_" + std::to_string(n) + "_c" + std::to_string(in.o_class) + "\n";
        for (const auto& pk : tc.lods[0]) {
            for (const auto& v : pk.vertices) {
                float w[3];
                world(v, w);
                char line[160];
                std::snprintf(line, sizeof line, "v %.4f %.4f %.4f\nvt %.5f %.5f\n", w[0], w[1], w[2], v.s / 4096.0f, v.t / 4096.0f);
                obj += line;
            }
            int cur = -1;
            for (const rc::TieTriangle& t : rc::tie_triangles(pk)) {
                if (t.ad_gif != cur) { cur = t.ad_gif; obj += "usemtl tex" + std::to_string(cur) + "\n"; }
                float rx[3], ry[3], rz[3];
                rc::u16 idx[3] = {t.a, t.b, t.c};
                for (int k = 0; k < 3; k++) { float w[3]; world(pk.vertices[idx[k]], w); rx[k] = w[0]; ry[k] = w[1]; rz[k] = w[2]; }
                raster.add(rx, ry, rz, 230, 140, 40);
                char line[96];
                size_t a = vbase + t.a, b = vbase + t.b, c = vbase + t.c;
                std::snprintf(line, sizeof line, "f %zu/%zu %zu/%zu %zu/%zu\n", a, a, b, b, c, c);
                obj += line;
            }
            vbase += pk.vertices.size();
        }
    }
    rc::write_file(std::string(d) + "/ties_lod0.obj", std::vector<rc::u8>(obj.begin(), obj.end()));
    raster.write(std::string(d) + "/ties_topdown.png");
    std::printf("level %02d: %zu tie classes parsed (%zu failed), %zu packets, %zu vertices, %zu triangles (%zu LOD0), %zu instances, %zu unresolved classes, %zu LOD-info mismatches\n",
                level, classes.size(), failed, packets, verts, all_tris, tris, inst.size(), unresolved, bad_lod_info);
    return (failed || unresolved || bad_lod_info) ? 1 : 0;
}

// Parses every shrub class of a level and its instances; writes shrub_dump.bin (the golden
// reference for crates/rc-formats/src/shrub.rs) and shrubs_topdown.png.
int cmd_shrub(const std::string& out, int level) {
    namespace fs = std::filesystem;
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::map<int, rc::ShrubClass> classes;
    size_t failed = 0, tris = 0, packets = 0, verts = 0, draws = 0, billboards = 0;
    for (const auto& f : fs::directory_iterator(std::string(d) + "/core/shrub_class")) {
        int o_class = std::atoi(f.path().stem().string().c_str());
        try {
            std::vector<rc::u8> blob = rc::read_file(f.path().string());
            rc::ShrubClass sc = rc::parse_shrub_class(rc::Buffer(blob));
            if (sc.header.o_class != o_class) throw rc::FormatError("header o_class mismatch");
            for (const auto& pk : sc.packets) { packets++; verts += pk.vertices.size(); draws += pk.draws.size(); tris += rc::shrub_triangles(pk).size(); }
            billboards += sc.billboard.size();
            classes[o_class] = std::move(sc);
        } catch (const std::exception& e) {
            failed++;
            std::fprintf(stderr, "shrub class %d: %s\n", o_class, e.what());
        }
    }
    std::vector<rc::u8> inst_bytes = rc::read_file(std::string(d) + "/gameplay/shrub_instances.bin");
    std::vector<rc::ShrubInstance> inst = rc::parse_shrub_instances(rc::Buffer(inst_bytes));
    size_t unresolved = 0;
    for (const auto& i : inst) if (!classes.count(i.o_class)) unresolved++;

    // shrub_dump.bin: "RCSH" u32 class_count, then per class in o_class order: s32 o_class and
    // sections (u32 byte_length + raw little-endian bytes):
    //   header(0x40) normals(24 x s16[4]) billboard(0 or 0x40)
    // then for each packet (header.packet_count):
    //   entry(8) packet_header(0x10) gif_tags(n x 0x10) ad_gifs(n x 0x40) part1(n x 8) part2(n x 8)
    //   vertices(16 bytes each, rc::ShrubVertex) draws triangles(a,b,c,texture u16)
    // A draw is u8 texture, u8 prim, u16 count, u16 vertex indices[count].
    // Finally u32 instance_count and one section with the raw 0x70-byte instance records.
    {
        std::vector<rc::u8> dump = {'R', 'C', 'S', 'H', 0, 0, 0, 0};
        rc::u32 n_cls = rc::u32(classes.size());
        std::memcpy(dump.data() + 4, &n_cls, 4);
        auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
        auto sec = [&](const void* p, size_t n) { rc::u32 len = rc::u32(n); raw(&len, 4); raw(p, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        for (const auto& [o_class, sc] : classes) {
            raw(&o_class, 4);
            sec(&sc.header, sizeof sc.header);
            vsec(sc.normals);
            vsec(sc.billboard);
            for (const auto& pk : sc.packets) {
                sec(&pk.entry, sizeof pk.entry);
                sec(&pk.header, sizeof pk.header);
                vsec(pk.gif_tags); vsec(pk.ad_gifs); vsec(pk.part1); vsec(pk.part2); vsec(pk.vertices);
                std::vector<rc::u8> dr;
                for (const auto& x : pk.draws) {
                    rc::u16 n = rc::u16(x.vertices.size());
                    dr.push_back(x.texture); dr.push_back(x.prim);
                    dr.push_back(rc::u8(n)); dr.push_back(rc::u8(n >> 8));
                    for (rc::u16 v : x.vertices) { dr.push_back(rc::u8(v)); dr.push_back(rc::u8(v >> 8)); }
                }
                vsec(dr);
                vsec(rc::shrub_triangles(pk));
            }
        }
        rc::u32 n_inst = rc::u32(inst.size());
        raw(&n_inst, 4);
        vsec(inst);
        rc::write_file(std::string(d) + "/shrub_dump.bin", dump);
    }

    TopDown raster;
    {
        std::vector<rc::u8> blk = rc::read_file(std::string(d) + "/core/tfrags.bin");
        for (const rc::Tfrag& t : rc::parse_tfrags(rc::Buffer(blk))) for (const rc::TfragTriangle& tr : rc::tfrag_triangles(t, 0)) {
            float rx[3], ry[3], rz[3]; rc::u16 idx[3] = {tr.a, tr.b, tr.c};
            for (int k = 0; k < 3; k++) { const rc::TfragPosition& p = t.positions[size_t(t.vertex_info[idx[k]].vertex) / 2]; rx[k] = float(t.origin[0] + p.x) / 1024.0f; ry[k] = float(t.origin[1] + p.y) / 1024.0f; rz[k] = float(t.origin[2] + p.z) / 1024.0f - 1000.0f; }
            raster.add(rx, ry, rz, 70, 70, 70);
        }
    }
    for (const rc::ShrubInstance& in : inst) {
        auto it = classes.find(in.o_class);
        if (it == classes.end()) continue;
        float k = it->second.header.scale / 1024.0f;
        auto xf = [&](const rc::ShrubVertex& v, float& wx, float& wy, float& wz) {
            float lx = v.x * k, ly = v.y * k, lz = v.z * k;
            wx = in.matrix[0][0] * lx + in.matrix[1][0] * ly + in.matrix[2][0] * lz + in.matrix[3][0];
            wy = in.matrix[0][1] * lx + in.matrix[1][1] * ly + in.matrix[2][1] * lz + in.matrix[3][1];
            wz = in.matrix[0][2] * lx + in.matrix[1][2] * ly + in.matrix[2][2] * lz + in.matrix[3][2];
        };
        for (const auto& pk : it->second.packets) for (const rc::ShrubTriangle& t : rc::shrub_triangles(pk)) {
            float rx[3], ry[3], rz[3];
            rc::u16 idx[3] = {t.a, t.b, t.c};
            for (int c = 0; c < 3; c++) xf(pk.vertices[idx[c]], rx[c], ry[c], rz[c]);
            raster.add(rx, ry, rz, rc::u8(std::min(255, in.g + 80)), rc::u8(std::min(255, in.g + 160)), 60);
        }
    }
    raster.write(std::string(d) + "/shrubs_topdown.png");
    std::printf("level %02d: %zu shrub classes parsed (%zu failed), %zu packets, %zu vertices, %zu draws, %zu triangles, %zu billboards, %zu instances, %zu unresolved\n",
                level, classes.size(), failed, packets, verts, draws, tris, billboards, inst.size(), unresolved);
    return (failed || unresolved) ? 1 : 0;
}

int cmd_sky(const std::string& out, int level) {
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::string path = std::string(d) + "/core/sky.bin";
    if (!std::filesystem::exists(path)) { std::printf("level %02d: no sky block\n", level); return 0; }
    std::vector<rc::u8> blk = rc::read_file(path);
    rc::Sky sky = rc::parse_sky(rc::Buffer(blk));
    size_t clusters = 0, tris = 0, verts = 0;
    for (const auto& sh : sky.shells) { clusters += sh.clusters.size(); for (const auto& c : sh.clusters) { tris += c.faces.size(); verts += c.vertices.size(); } }
    for (size_t i = 0; i < sky.texture_defs.size(); i++) {
        char name[64];
        std::snprintf(name, sizeof name, "/textures/sky/%02zu_%dx%d.png", i, sky.texture_defs[i].width, sky.texture_defs[i].height);
        rc::write_png(std::string(d) + name, rc::decode_sky_texture(rc::Buffer(blk), sky, i));
    }
    // sky_dump.bin (golden reference for crates/rc-formats/src/sky.rs): "RCSK", then sections
    // (u32 byte_length + raw little-endian bytes): header(0x40) fx_list texture_defs; u32 shell_count;
    // per shell: shell(cluster_count, flags: 8 bytes) cluster_headers(n x 0x20), then per cluster:
    // vertices st faces gs_vertices(rc::SkyGsVertex, 20 bytes each); u32 texture_count; per texture:
    // u32 width, u32 height, one section of RGBA8 pixels (decode_sky_texture).
    {
        std::vector<rc::u8> dump = {'R', 'C', 'S', 'K'};
        auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
        auto u32w = [&](size_t v) { rc::u32 x = rc::u32(v); raw(&x, 4); };
        auto sec = [&](const void* p, size_t n) { u32w(n); raw(p, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        sec(&sky.header, sizeof sky.header);
        vsec(sky.fx_list);
        vsec(sky.texture_defs);
        u32w(sky.shells.size());
        for (const auto& sh : sky.shells) {
            rc::s32 shdr[2] = {sh.cluster_count, sh.flags};
            sec(shdr, sizeof shdr);
            std::vector<rc::SkyClusterHeader> chs;
            for (const auto& c : sh.clusters) chs.push_back(c.header);
            vsec(chs);
            for (const auto& c : sh.clusters) { vsec(c.vertices); vsec(c.st); vsec(c.faces); vsec(rc::sky_gs_vertices(sh, c)); }
        }
        u32w(sky.texture_defs.size());
        for (size_t i = 0; i < sky.texture_defs.size(); i++) {
            rc::Image img = rc::decode_sky_texture(rc::Buffer(blk), sky, i);
            u32w(img.width); u32w(img.height); vsec(img.rgba);
        }
        rc::write_file(std::string(d) + "/sky_dump.bin", dump);
    }
    std::printf("level %02d: sky colour %d,%d,%d,%d clear=%d shells=%d textures=%d fx=%d clusters=%zu vertices=%zu triangles=%zu\n",
                level, sky.header.r, sky.header.g, sky.header.b, sky.header.a, sky.header.clear_screen, sky.header.shell_count,
                sky.header.texture_count, sky.header.fx_count, clusters, verts, tris);
    for (size_t i = 0; i < sky.shells.size(); i++) std::printf("  shell %zu: %d clusters, flags 0x%x\n", i, sky.shells[i].cluster_count, sky.shells[i].flags);
    return 0;
}

int cmd_moby(const std::string& out, int level) {
    namespace fs = std::filesystem;
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    size_t ok = 0, failed = 0, packets = 0, tris = 0, animated = 0;
    std::map<int, rc::MobyClass> classes;
    for (const auto& f : fs::directory_iterator(std::string(d) + "/core/moby_class")) {
        int o_class = std::atoi(f.path().stem().string().c_str());
        try {
            std::vector<rc::u8> blob = rc::read_file(f.path().string());
            rc::MobyClass mc = rc::parse_moby_class(rc::Buffer(blob));
            packets += mc.high_lod.size() + mc.low_lod.size() + mc.metal.size();
            for (const auto& pk : mc.high_lod) tris += pk.triangles.size();
            if (mc.header.joint_count > 1) animated++;
            classes[o_class] = std::move(mc);
            ok++;
        } catch (const std::exception& e) {
            failed++;
            std::fprintf(stderr, "moby class %d: %s\n", o_class, e.what());
        }
    }
    // moby_dump.bin: every parsed field, the golden reference for the Rust port
    // (crates/rc-formats/tests/golden.rs). "RCMB" u32 class_count, then per class in o_class order:
    // s32 o_class, then sections (u32 byte_length + raw little-endian bytes):
    //   header(0x48) sequence_pointers skeleton(f32 x16 per joint) common_trans(0x10 per joint)
    //   packet_counts(3 u32: high, low, metal)
    // and per packet (high LOD, low LOD, metal): entry(0x10) table_header(0x20 regular / 0x10 metal)
    //   transfers duplicates raw_vertices rgba_multipliers st index_bytes secret_indices texture_indices
    //   state(s32 initial_texture, u32 unresolved_duplicates) vertices triangles(a,b,c u32, material s32)
    // A vertex record is 38 bytes: s16 x,y,z; u8 az,el; u16 id; u8 type, duplicate, skin count;
    //   u8 joints[3]; u16 weights[3]; s16 s,t (0 for metal); f32 normal[3] (moby_normal).
    {
        std::vector<rc::u8> dump = {'R', 'C', 'M', 'B', 0, 0, 0, 0};
        rc::u32 n_cls = rc::u32(classes.size());
        std::memcpy(dump.data() + 4, &n_cls, 4);
        auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
        auto sec = [&](const void* p, size_t n) { rc::u32 len = rc::u32(n); raw(&len, 4); raw(p, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        for (const auto& [o_class, mc] : classes) {
            raw(&o_class, 4);
            sec(&mc.header, sizeof mc.header);
            vsec(mc.sequence_pointers);
            vsec(mc.skeleton);
            vsec(mc.common_trans);
            rc::u32 counts[3] = {rc::u32(mc.high_lod.size()), rc::u32(mc.low_lod.size()), rc::u32(mc.metal.size())};
            sec(counts, sizeof counts);
            for (const auto* lst : {&mc.high_lod, &mc.low_lod, &mc.metal}) for (const rc::MobyPacket& pk : *lst) {
                sec(&pk.entry, sizeof pk.entry);
                if (pk.metal) sec(&pk.metal_header, sizeof pk.metal_header); else sec(&pk.vth, sizeof pk.vth);
                vsec(pk.transfers); vsec(pk.duplicates); vsec(pk.raw_vertices); vsec(pk.rgba_multipliers);
                vsec(pk.st); vsec(pk.index_bytes); vsec(pk.secret_indices); vsec(pk.texture_indices);
                rc::s32 state[2] = {pk.initial_texture, rc::s32(pk.unresolved_duplicates)};
                sec(state, sizeof state);
                std::vector<rc::u8> recs;
                for (size_t vi = 0; vi < pk.vertices.size(); vi++) {
                    const rc::MobyVertex& v = pk.vertices[vi];
                    rc::s16 st[2] = {0, 0};
                    if (!pk.metal) { st[0] = pk.st[vi * 2]; st[1] = pk.st[vi * 2 + 1]; }
                    float n[3]; rc::moby_normal(v.normal_azimuth, v.normal_elevation, n);
                    rc::u8 flags[3] = {v.type, rc::u8(v.duplicate), v.skin.count};
                    auto put = [&](const void* p, size_t k) { recs.insert(recs.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + k); };
                    put(&v.x, 2); put(&v.y, 2); put(&v.z, 2); put(&v.normal_azimuth, 1); put(&v.normal_elevation, 1); put(&v.id, 2);
                    put(flags, 3); put(v.skin.joints, 3); put(v.skin.weights, 6); put(st, 4); put(n, 12);
                }
                vsec(recs);
                vsec(pk.triangles);
            }
        }
        rc::write_file(std::string(d) + "/moby_dump.bin", dump);
    }
    std::vector<rc::MobyInstance> inst = rc::parse_moby_instances(rc::Buffer(rc::read_file(std::string(d) + "/gameplay/moby_instances.bin")));
    size_t unresolved = 0;
    for (const auto& m : inst) if (!classes.count(m.o_class)) unresolved++;
    // Front-view raster of the class with the most high-LOD triangles (bind pose, X right, Z up).
    int best = -1; size_t best_tris = 0;
    for (const auto& [c, mc] : classes) { size_t t = 0; for (const auto& pk : mc.high_lod) t += pk.triangles.size(); if (t > best_tris) { best_tris = t; best = c; } }
    if (best >= 0) {
        TopDown raster;
        const rc::MobyClass& mc = classes[best];
        float k = mc.header.scale / 1024.0f;
        for (const auto& pk : mc.high_lod) for (const auto& t : pk.triangles) {
            float rx[3], ry[3], rz[3];
            rc::u32 ix[3] = {t.a, t.b, t.c};
            for (int c = 0; c < 3; c++) { const rc::MobyVertex& v = pk.vertices[ix[c]]; rx[c] = v.x * k; ry[c] = v.z * k; rz[c] = -v.y * k; }
            rc::u8 shade = rc::u8(96 + (pk.vertices[ix[0]].normal_elevation % 64) * 2);
            raster.add(rx, ry, rz, shade, shade, rc::u8(std::min(255, shade + 40)));
        }
        char name[64]; std::snprintf(name, sizeof name, "/moby_%04d_front.png", best);
        raster.write(std::string(d) + name, 512);
    }
    // Geometry sanity: every high-LOD vertex should lie within ~1.25x the class bounding sphere.
    size_t outside = 0, checked = 0, unresolved_dupes = 0, outside_nondupe = 0;
    for (const auto& [c, mc] : classes) {
        float k = mc.header.scale / 1024.0f;
        const float* bs = mc.header.bsphere;
        for (const auto& pk : mc.high_lod) {
            unresolved_dupes += pk.unresolved_duplicates;
            rc::u32 in_file = pk.vth.two_way_blend_vertex_count + pk.vth.three_way_blend_vertex_count + pk.vth.main_vertex_count;
            for (size_t vi = 0; vi < pk.vertices.size(); vi++) {
                const rc::MobyVertex& v = pk.vertices[vi];
                float dx = v.x * k - bs[0], dy = v.y * k - bs[1], dz = v.z * k - bs[2];
                checked++;
                if (std::sqrt(dx * dx + dy * dy + dz * dz) > bs[3] * 1.25f + 0.05f) { outside++; if (vi < in_file) outside_nondupe++; }
            }
        }
    }
    std::printf("  unresolved cross-packet duplicates %zu; outside-sphere vertices that are not duplicates %zu\n", unresolved_dupes, outside_nondupe);
    {   // worst distance/radius ratio per class, top 5
        std::vector<std::pair<float, int>> worst;
        for (const auto& [c, mc] : classes) {
            float k = mc.header.scale / 1024.0f; const float* bs = mc.header.bsphere; float w = 0;
            for (const auto& pk : mc.high_lod) for (const auto& v : pk.vertices) { float dx = v.x * k - bs[0], dy = v.y * k - bs[1], dz = v.z * k - bs[2]; w = std::max(w, std::sqrt(dx * dx + dy * dy + dz * dz) / std::max(bs[3], 1e-3f)); }
            worst.push_back({w, c});
        }
        std::sort(worst.rbegin(), worst.rend());
        std::printf("  worst dist/radius ratios:");
        for (size_t i = 0; i < std::min<size_t>(5, worst.size()); i++) std::printf(" class %d = %.2f", worst[i].second, worst[i].first);
        std::printf("\n");
    }
    std::printf("level %02d: %zu moby classes parsed (%zu failed), %zu packets, %zu high-LOD triangles, %zu animated classes, %zu instances, %zu without geometry; largest class %d (%zu tris); vertices outside bsphere %zu of %zu\n",
                level, ok, failed, packets, tris, animated, inst.size(), unresolved, best, best_tris, outside, checked);
    return failed ? 1 : 0;
}

// gadget_dump.bin: the RAC1 gadget classes (docs/formats/moby_rac1.md 0.4), golden reference for
// crates/rc-formats/src/gadget.rs. Read from core_index.bin, core_data.dec and gs_ram.bin.
// "RCGD" u32 gadget_count, then per gadget-table entry in table order: s32 o_class, then sections
// (u32 byte_length + raw little-endian bytes):
//   entry(GadgetEntry 0x10) class_entry(moby ClassEntry 0x20) decompressed_size(u32)
//   then exactly the per-class sections of moby_dump.bin (header .. per-packet triangles, see cmd_moby)
//   then textures: per class_entry.textures[k] != 0xff: u32 k, u32 moby texture index, u32 w, u32 h, w*h*4 RGBA
//   (w = h = 0 when the moby texture entry has no pixels).
int cmd_gadget(const std::string& out, int level) {
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::vector<rc::u8> ci = rc::read_file(std::string(d) + "/core_index.bin");
    std::vector<rc::u8> data = rc::read_file(std::string(d) + "/core_data.dec");
    std::vector<rc::u8> gs = rc::read_file(std::string(d) + "/gs_ram.bin");
    rc::LevelCore core = rc::parse_level_core(rc::Buffer(ci), data.size());
    std::vector<rc::GadgetClass> gadgets = rc::parse_gadget_classes(core, rc::Buffer(data));
    rc::Buffer db(data), gsb(gs);

    std::vector<rc::u8> dump = {'R', 'C', 'G', 'D', 0, 0, 0, 0};
    rc::u32 n = rc::u32(gadgets.size());
    std::memcpy(dump.data() + 4, &n, 4);
    auto raw = [&](const void* p, size_t k) { if (k) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + k); };
    auto sec = [&](const void* p, size_t k) { rc::u32 len = rc::u32(k); raw(&len, 4); raw(p, k); };
    auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
    size_t verts = 0, tris = 0, packets = 0, textures = 0, max_size = 0;
    for (const rc::GadgetClass& g : gadgets) {
        const rc::MobyClass& mc = g.mc;
        raw(&g.entry.class_number, 4);
        sec(&g.entry, sizeof g.entry);
        sec(&g.class_entry, sizeof g.class_entry);
        rc::u32 dsize = rc::u32(g.decompressed_size);
        sec(&dsize, 4);
        max_size = std::max(max_size, g.decompressed_size);
        sec(&mc.header, sizeof mc.header);
        vsec(mc.sequence_pointers);
        vsec(mc.skeleton);
        vsec(mc.common_trans);
        rc::u32 counts[3] = {rc::u32(mc.high_lod.size()), rc::u32(mc.low_lod.size()), rc::u32(mc.metal.size())};
        sec(counts, sizeof counts);
        for (const auto* lst : {&mc.high_lod, &mc.low_lod, &mc.metal}) for (const rc::MobyPacket& pk : *lst) {
            packets++;
            if (lst == &mc.high_lod) tris += pk.triangles.size();
            verts += pk.vertices.size();
            sec(&pk.entry, sizeof pk.entry);
            if (pk.metal) sec(&pk.metal_header, sizeof pk.metal_header); else sec(&pk.vth, sizeof pk.vth);
            vsec(pk.transfers); vsec(pk.duplicates); vsec(pk.raw_vertices); vsec(pk.rgba_multipliers);
            vsec(pk.st); vsec(pk.index_bytes); vsec(pk.secret_indices); vsec(pk.texture_indices);
            rc::s32 state[2] = {pk.initial_texture, rc::s32(pk.unresolved_duplicates)};
            sec(state, sizeof state);
            std::vector<rc::u8> recs;
            for (size_t vi = 0; vi < pk.vertices.size(); vi++) {
                const rc::MobyVertex& v = pk.vertices[vi];
                rc::s16 st[2] = {0, 0};
                if (!pk.metal) { st[0] = pk.st[vi * 2]; st[1] = pk.st[vi * 2 + 1]; }
                float nrm[3]; rc::moby_normal(v.normal_azimuth, v.normal_elevation, nrm);
                rc::u8 flags[3] = {v.type, rc::u8(v.duplicate), v.skin.count};
                auto put = [&](const void* p, size_t k) { recs.insert(recs.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + k); };
                put(&v.x, 2); put(&v.y, 2); put(&v.z, 2); put(&v.normal_azimuth, 1); put(&v.normal_elevation, 1); put(&v.id, 2);
                put(flags, 3); put(v.skin.joints, 3); put(v.skin.weights, 6); put(st, 4); put(nrm, 12);
            }
            vsec(recs);
            vsec(pk.triangles);
        }
        std::vector<rc::u8> tex;
        auto put32 = [&](rc::u32 v) { for (int k = 0; k < 4; k++) tex.push_back(rc::u8(v >> (8 * k))); };
        for (rc::u32 k = 0; k < 16; k++) {
            rc::u32 slot = g.class_entry.textures[k];
            if (slot == 0xff) continue;
            if (slot >= core.moby_textures.size()) throw rc::FormatError("gadget " + std::to_string(g.entry.class_number) + ": texture slot past the moby table");
            const rc::TextureEntry& e = core.moby_textures[slot];
            put32(k); put32(slot);
            if (e.width <= 0 || e.height <= 0 || e.data_offset < 0 || e.palette < 0) { put32(0); put32(0); continue; }
            rc::Image img = rc::decode_indexed8(db.sub(size_t(core.header.textures_base_offset) + size_t(e.data_offset), size_t(e.width) * e.height).bytes(),
                                                rc::u32(e.width), rc::u32(e.height), gsb.sub(size_t(e.palette) * 0x100, 1024).bytes());
            put32(img.width); put32(img.height);
            tex.insert(tex.end(), img.rgba.begin(), img.rgba.end());
            textures++;
        }
        vsec(tex);
    }
    rc::write_file(std::string(d) + "/gadget_dump.bin", dump);
    std::printf("level %02d: %zu gadget classes, %zu packets, %zu vertices, %zu high-LOD triangles, %zu textures, largest decompressed class 0x%zx (buffer 0x%zx):",
                level, gadgets.size(), packets, verts, tris, textures, max_size, rc::GADGET_BUFFER_SIZE);
    for (const rc::GadgetClass& g : gadgets) std::printf(" %d", g.entry.class_number);
    std::printf("\n");
    return 0;
}

int cmd_collision(const std::string& out, int level) {
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::vector<rc::u8> blk = rc::read_file(std::string(d) + "/core/collision.bin");
    rc::Collision col = rc::parse_collision(rc::Buffer(blk));
    std::vector<rc::CollisionTriangle> tris = rc::collision_triangles(col);
    size_t faces = 0, quads = 0, verts = 0;
    std::map<int, size_t> types;
    TopDown raster;
    for (const auto& cell : col.cells) {
        verts += cell.vertices.size() / 3;
        for (const auto& f : cell.faces) { faces++; quads += f.quad; types[f.type]++; }
    }
    for (const auto& t : tris) {
        float rx[3] = {t.a[0], t.b[0], t.c[0]}, ry[3] = {t.a[1], t.b[1], t.c[1]}, rz[3] = {t.a[2], t.b[2], t.c[2]};
        rc::u8 r = rc::u8(((t.surface & 3) << 6) | 0x30), g = rc::u8(((t.surface & 0xc) << 4) | 0x30), b = rc::u8((t.surface & 0xf0) | 0x30);
        raster.add(rx, ry, rz, r, g, b);
    }
    raster.write(std::string(d) + "/collision_topdown.png");
    // collision_dump.bin (golden reference for crates/rc-formats/src/collision.rs): "RCCL", then
    // sections (u32 byte_length + raw little-endian bytes) and u32 counts:
    //   header(8); root node bytes (4 + 2*z_count);
    //   u32 slab_count, per slab: u32 offset-from-mesh, node bytes (4 + 4*y_count);
    //   u32 row_count, per row: u32 offset-from-mesh, node bytes (4 + 4*x_count);
    //   u32 cell_count, per cell: record(s16 cx, cy, cz, u16 0, u32 leaf_word, u16 face_count, u8 vertex_count,
    //     u8 quad_count = 16 bytes), packed(u32 each), vertices(f32 xyz), faces(u8 v0 v1 v2 type each),
    //     quad_v3(u8 each);
    //   s32 hero_group_count, per group: record(16 raw bytes), raw vertices(8 bytes each),
    //     raw triangles(4 bytes each), sphere(4 f32), vertices(f32 xyz);
    //   triangles(rc::CollisionTriangle, 44 bytes each).
    {
        std::vector<rc::u8> dump = {'R', 'C', 'C', 'L'};
        auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
        auto u32w = [&](size_t v) { rc::u32 x = rc::u32(v); raw(&x, 4); };
        auto sec = [&](const void* p, size_t n) { u32w(n); raw(p, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        rc::Buffer m = rc::Buffer(blk).sub(size_t(col.header.mesh));
        auto node = [&](size_t at, size_t entry) { std::vector<rc::u8> n = m.copy(at, 4 + entry * m.read<rc::u16>(at + 2)); vsec(n); };
        sec(&col.header, sizeof col.header);
        node(0, 2);
        u32w(col.slab_offsets.size());
        for (rc::u32 o : col.slab_offsets) { u32w(o); node(o, 4); }
        u32w(col.row_offsets.size());
        for (rc::u32 o : col.row_offsets) { u32w(o); node(o, 4); }
        u32w(col.cells.size());
        for (const auto& c : col.cells) {
            rc::u8 rec[16] = {};
            rc::s16 xyz[4] = {c.cx, c.cy, c.cz, 0};
            std::memcpy(rec, xyz, 8); std::memcpy(rec + 8, &c.leaf_word, 4); std::memcpy(rec + 12, &c.face_count, 2);
            rec[14] = c.vertex_count; rec[15] = c.quad_count;
            sec(rec, sizeof rec);
            vsec(c.packed);
            vsec(c.vertices);
            std::vector<rc::u8> fr;
            for (const auto& f : c.faces) { fr.push_back(f.v[0]); fr.push_back(f.v[1]); fr.push_back(f.v[2]); fr.push_back(f.type); }
            vsec(fr);
            vsec(c.quad_v3);
        }
        u32w(rc::u32(col.hero_group_count));
        for (const auto& g : col.hero_groups) {
            rc::u8 rec[16];
            std::memcpy(rec, g.raw, 8); std::memcpy(rec + 8, &g.triangle_count, 2); std::memcpy(rec + 10, &g.vertex_count, 2); std::memcpy(rec + 12, &g.data, 4);
            sec(rec, sizeof rec);
            vsec(g.raw_vertices);
            std::vector<rc::u8> tr;
            for (size_t i = 0; i < g.triangles.size(); i += 3) { tr.insert(tr.end(), g.triangles.begin() + long(i), g.triangles.begin() + long(i) + 3); tr.push_back(0); }
            vsec(tr);
            sec(g.sphere, sizeof g.sphere);
            vsec(g.vertices);
        }
        vsec(tris);
        rc::write_file(std::string(d) + "/collision_dump.bin", dump);
    }
    std::printf("level %02d: %zu cells, %zu vertices, %zu faces (%zu quads), %zu triangles, %zu hero groups; %zu distinct surface types:",
                level, col.cells.size(), verts, faces, quads, tris.size(), col.hero_groups.size(), types.size());
    for (const auto& [t, n] : types) std::printf(" %d:%zu", t, n);
    std::printf("\n");
    return 0;
}

// Occlusion grid + gameplay mappings resolved like the level loader (FUN_00255958). Writes
// occlusion_dump.bin (golden reference for crates/rc-formats/src/occlusion.rs): "RCOC", then
// sections (u32 byte_length + raw bytes) and u32 values:
//   raw block (core/occlusion.bin); u32 masks_offset, u32 z_base | z_count << 16;
//   cells (u16 x, y, z, mask each, tree order); u32 mask_count;
//   octant override (0x410 raw bytes at core header 0xa8, or empty);
//   mapping records (s32 bit_index, s32 occlusion_id; tfrag ‖ tie ‖ moby) after u32 counts nt, ni, nm;
//   resolved u16 bits per tfrag, per tie instance, per static moby instance;
//   u32 tfrag_out_of_date, u32 ties_not_found, u32 mobys_not_found;
//   per mask: for tfrag, tie, moby: u32 visible_count, u32 FNV-1a over the u32 LE indices, where the
//     frame mask is the stored mask with bit 1023 forced on (BuildOcclVisibility's last store).
int cmd_occlusion(const std::string& out, int level) {
    char d[256];
    std::snprintf(d, sizeof d, "%s/levels/%02d", out.c_str(), level);
    std::string dir(d);
    std::vector<rc::u8> blk = rc::read_file(dir + "/core/occlusion.bin");
    rc::OcclusionGrid g = rc::parse_occlusion_grid(rc::Buffer(blk));
    for (const auto& c : g.cells)
        if (rc::occlusion_lookup(rc::Buffer(blk), c.x, c.y, c.z) != c.mask) throw rc::FormatError("occlusion: tree walk disagrees with enumeration");
    std::vector<rc::u8> ci = rc::read_file(dir + "/core_index.bin");
    std::vector<rc::u8> cd = rc::read_file(dir + "/core_data.dec");
    rc::LevelCore core = rc::parse_level_core(rc::Buffer(ci), cd.size());
    std::vector<rc::u8> oct;
    if (core.header.occlusion_oct_offset > 0) oct = rc::Buffer(cd).copy(size_t(core.header.occlusion_oct_offset), 0x10 + 8 * rc::OCCL_MASK_BYTES);
    rc::OcclusionMappings maps = rc::parse_occlusion_mappings(rc::Buffer(rc::read_file(dir + "/gameplay/occlusion_mappings.bin")));
    // tfrag headers: byte 0x3d (the loader's consistency key) and the on-disc u16 at 0x3a.
    std::vector<rc::u8> tb = rc::read_file(dir + "/core/tfrags.bin");
    rc::Buffer tbuf(tb);
    rc::s32 table = tbuf.read<rc::s32>(0), tcount = tbuf.read<rc::s32>(4);
    std::vector<rc::u8> h3d;
    size_t disc_3a_nonzero = 0;
    for (rc::s32 i = 0; i < tcount; i++) {
        h3d.push_back(tbuf.read<rc::u8>(size_t(table) + size_t(i) * 0x40 + 0x3d));
        disc_3a_nonzero += tbuf.read<rc::u16>(size_t(table) + size_t(i) * 0x40 + 0x3a) != 0;
    }
    std::vector<rc::TieInstance> ties = rc::parse_tie_instances(rc::Buffer(rc::read_file(dir + "/gameplay/tie_instances.bin")));
    std::vector<rc::s32> tie_occl;
    for (const auto& t : ties) tie_occl.push_back(t.occlusion_index);
    std::vector<rc::MobyInstance> mobys = rc::parse_moby_instances(rc::Buffer(rc::read_file(dir + "/gameplay/moby_instances.bin")));
    std::vector<rc::MobyOcclusionKey> moby_keys;
    for (const auto& m : mobys) moby_keys.push_back({m.occlusion, m.unknown_c});
    bool tfrag_stale = false;
    size_t ties_missing = 0, mobys_missing = 0;
    std::vector<rc::u16> tf_bits = rc::resolve_tfrag_occlusion(maps, h3d, &tfrag_stale);
    std::vector<rc::u16> tie_bits = rc::resolve_tie_occlusion(maps, tie_occl, &ties_missing);
    std::vector<rc::u16> moby_bits = rc::resolve_moby_occlusion(maps, moby_keys, &mobys_missing);

    std::vector<rc::u8> dump = {'R', 'C', 'O', 'C'};
    auto raw = [&](const void* p, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(p), reinterpret_cast<const rc::u8*>(p) + n); };
    auto u32w = [&](size_t v) { rc::u32 x = rc::u32(v); raw(&x, 4); };
    auto sec = [&](const void* p, size_t n) { u32w(n); raw(p, n); };
    auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
    vsec(blk);
    u32w(rc::u32(g.masks_offset));
    u32w(rc::u32(g.z_base) | rc::u32(g.z_count) << 16);
    vsec(g.cells);
    u32w(g.mask_count);
    vsec(oct);
    u32w(maps.tfrag.size()); u32w(maps.tie.size()); u32w(maps.moby.size());
    std::vector<rc::OcclusionMapping> all = maps.tfrag;
    all.insert(all.end(), maps.tie.begin(), maps.tie.end());
    all.insert(all.end(), maps.moby.begin(), maps.moby.end());
    vsec(all);
    vsec(tf_bits); vsec(tie_bits); vsec(moby_bits);
    u32w(tfrag_stale); u32w(ties_missing); u32w(mobys_missing);

    // Per-mask visible lists (count + hash), and which objects any mask ever shows.
    const std::vector<rc::u16>* kinds[3] = {&tf_bits, &tie_bits, &moby_bits};
    std::vector<std::vector<rc::u32>> mask_counts(3, std::vector<rc::u32>(g.mask_count));
    std::vector<std::vector<bool>> ever(3);
    for (int k = 0; k < 3; k++) ever[k].assign(kinds[k]->size(), false);
    for (rc::u32 mi = 0; mi < g.mask_count; mi++) {
        rc::u8 frame[rc::OCCL_MASK_BYTES];
        std::memcpy(frame, &g.masks[size_t(mi) * rc::OCCL_MASK_BYTES], sizeof frame);
        frame[0x7f] |= 0x80;
        for (int k = 0; k < 3; k++) {
            rc::u32 n = 0, h = 2166136261u;
            const auto& bits = *kinds[k];
            for (size_t i = 0; i < bits.size(); i++) {
                if (!rc::occl_visible(bits[i], frame)) continue;
                n++;
                ever[k][i] = true;
                for (int s = 0; s < 32; s += 8) { h ^= (rc::u32(i) >> s) & 0xff; h *= 16777619u; }
            }
            mask_counts[k][mi] = n;
            u32w(n); u32w(h);
        }
    }
    rc::write_file(dir + "/occlusion_dump.bin", dump);

    static const char* names[3] = {"tfrag", "tie", "moby"};
    std::printf("level %02d: %zu cells, %u masks (tail %zu B), grid x %d..%d y %d..%d z %d..%d, octant override %s; "
                "mappings %zu/%zu/%zu for %d tfrags, %zu ties, %zu mobys; tfrag out of date %d, ties not found %zu, mobys not found %zu; disc tfrag 0x3a nonzero %zu\n",
                level, g.cells.size(), g.mask_count, blk.size() - size_t(g.masks_offset) - size_t(g.mask_count) * rc::OCCL_MASK_BYTES,
                (int)std::min_element(g.cells.begin(), g.cells.end(), [](auto& a, auto& b) { return a.x < b.x; })->x,
                (int)std::max_element(g.cells.begin(), g.cells.end(), [](auto& a, auto& b) { return a.x < b.x; })->x,
                (int)std::min_element(g.cells.begin(), g.cells.end(), [](auto& a, auto& b) { return a.y < b.y; })->y,
                (int)std::max_element(g.cells.begin(), g.cells.end(), [](auto& a, auto& b) { return a.y < b.y; })->y,
                (int)g.z_base, (int)g.z_base + g.z_count - 1, oct.empty() ? "no" : "yes",
                maps.tfrag.size(), maps.tie.size(), maps.moby.size(), tcount, ties.size(), mobys.size(),
                (int)tfrag_stale, ties_missing, mobys_missing, disc_3a_nonzero);
    for (int k = 0; k < 3; k++) {
        size_t total = 0, lo = SIZE_MAX, hi = 0, always = 0, never = 0;
        for (const auto& c : g.cells) { size_t n = mask_counts[k][c.mask]; total += n; lo = std::min(lo, n); hi = std::max(hi, n); }
        std::string never_list;
        for (size_t i = 0; i < kinds[k]->size(); i++) {
            if ((*kinds[k])[i] == rc::OCCL_ALWAYS_VISIBLE) { always++; continue; }
            if (!ever[k][i]) { if (never < 8) never_list += " " + std::to_string(i); never++; }
        }
        std::printf("  %-5s: %zu objects, %zu always visible, %zu never visible in any cell%s%s; visible per cell min %zu max %zu, total over cells %zu\n",
                    names[k], kinds[k]->size(), always, never, never ? " (e.g." : "", never ? (never_list + ")").c_str() : "", lo, hi, total);
    }
    return 0;
}

// Particle and FX textures + part_defs of every level (docs/plan/particles.md 6, textures_rac1.md 8).
// Reads core_index.bin, core/part_bank.bin and core/fx_bank.bin; writes PNGs under textures/particle/ and
// textures/fx/, and particles_dump.bin, the golden reference for crates/rc-formats/src/particle_tex.rs:
//   "RCPT", then sections (u32 byte_length + raw little-endian bytes): part_textures entries (n x 0x10),
//   part_defs header (0x10), part_defs offsets (count x s32), part_defs blob, resolved starts (count x s32,
//   PartDefs::start); u32 texture count, per texture u32 width, u32 height, one section of RGBA8;
//   fx_textures entries (n x 0x10); u32 fx count, per fx u32 present, u32 width, u32 height, one RGBA8 section.
int cmd_particles(const std::string& out) {
    namespace fs = std::filesystem;
    size_t levels = 0, parts = 0, fxs = 0;
    std::vector<std::string> dirs;
    for (const auto& lvdir : fs::directory_iterator(out + "/levels")) dirs.push_back(lvdir.path().string());
    std::sort(dirs.begin(), dirs.end());
    for (const std::string& d : dirs) {
        if (!fs::exists(d + "/core_index.bin")) continue;
        std::vector<rc::u8> ci = rc::read_file(d + "/core_index.bin");
        std::vector<rc::u8> pb = fs::exists(d + "/core/part_bank.bin") ? rc::read_file(d + "/core/part_bank.bin") : std::vector<rc::u8>{};
        std::vector<rc::u8> fb = fs::exists(d + "/core/fx_bank.bin") ? rc::read_file(d + "/core/fx_bank.bin") : std::vector<rc::u8>{};
        rc::LevelCore core = rc::parse_level_core(rc::Buffer(ci), 0);
        rc::ParticleTextures p = rc::parse_particle_textures(rc::Buffer(ci), core.header, rc::Buffer(pb), rc::Buffer(fb));
        std::vector<rc::u8> dump = {'R', 'C', 'P', 'T'};
        auto raw = [&](const void* q, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(q), reinterpret_cast<const rc::u8*>(q) + n); };
        auto u32w = [&](size_t v) { rc::u32 x = rc::u32(v); raw(&x, 4); };
        auto sec = [&](const void* q, size_t n) { u32w(n); raw(q, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        vsec(p.entries);
        sec(p.defs.header, sizeof p.defs.header);
        vsec(p.defs.offsets);
        vsec(p.defs.blob);
        std::vector<rc::s32> starts;
        for (size_t t = 0; t < p.defs.offsets.size(); t++) starts.push_back(p.defs.start(t));
        vsec(starts);
        fs::create_directories(d + "/textures/particle");
        fs::create_directories(d + "/textures/fx");
        u32w(p.textures.size());
        for (size_t i = 0; i < p.textures.size(); i++) {
            const rc::Image& img = p.textures[i];
            u32w(img.width); u32w(img.height); vsec(img.rgba);
            char name[64];
            std::snprintf(name, sizeof name, "/textures/particle/%03zu.png", i);
            rc::write_png(d + name, img);
        }
        vsec(p.fx_entries);
        u32w(p.fx_textures.size());
        size_t fx_here = 0;
        for (size_t i = 0; i < p.fx_textures.size(); i++) {
            const auto& img = p.fx_textures[i];
            u32w(img ? 1 : 0);
            u32w(img ? img->width : 0); u32w(img ? img->height : 0);
            if (img) {
                vsec(img->rgba);
                char name[64];
                std::snprintf(name, sizeof name, "/textures/fx/%03zu_%ux%u.png", i, img->width, img->height);
                rc::write_png(d + name, *img);
                fx_here++;
            } else {
                u32w(0);
            }
        }
        rc::write_file(d + "/particles_dump.bin", dump);
        std::printf("%s: %zu particle textures, %zu/%zu fx textures, part_defs {%d, %d, %d, %d}, type 6 first frame %d\n",
                    d.c_str(), p.textures.size(), fx_here, p.fx_textures.size(), p.defs.header[0], p.defs.header[1],
                    p.defs.header[2], p.defs.header[3], p.defs.start(6) >= 0 ? p.defs.blob[size_t(p.defs.start(6))] : -1);
        levels++; parts += p.textures.size(); fxs += fx_here;
    }
    std::printf("particles: %zu levels, %zu particle textures, %zu fx textures\n", levels, parts, fxs);
    return 0;
}

// Sound banks, decoded samples, sound defs + remap and the music table of every level (docs/plan/audio.md 2, 4).
// Reads levels/NN/{sound_bank.bin, core_index.bin, core_data.dec, level_header.bin} (and global/sound_bank.bin);
// writes levels/NN/sound_dump.bin (and global/sound_dump.bin), the golden reference for
// crates/rc-formats/src/sound_bank.rs and vag.rs:
//   "RCSD", then sections (u32 byte_length + little-endian bytes):
//   1 bank header: u32 x 19 = file type, chunk count (2), chunk0 off/size, chunk1 off/size, version, flags, bank id,
//     bank num (s32), sounds, grains, vags (s32 each), first sound, first grain, vags in SR, vag data size,
//     SRAM alloc size, next block
//   2 sounds: s32 x 7 each = vol, vol group, pan, grain count, instance limit, flags, first grain
//   3 grains: u32 type, s32 delay, 32 data bytes each (sound order)
//   4 vags: u32 offset, u32 frames, s32 loop start frame, u32 looped, s32 next frame flags each
//   5 PCM: every vag's frames decoded (rounded form), s16, concatenated in vag order
//   6 PCM, OpenGOAL variant, same layout
//   7 level defs after the remap (0x20 each); 8 map (u16 each)
//   9 classes: per class s32 o_class, u32 n, n x u16 bank ids, s32 header count (-1 none), u32 m, m x 0x20 defs
//   10 music table: s32 x 15
// The global dump has sections 7-10 empty.
int cmd_sound(const std::string& out) {
    namespace fs = std::filesystem;
    auto dump_bank = [&](const std::string& dir, const std::vector<rc::u8>* ci, const std::vector<rc::u8>* cd, const std::vector<rc::u8>* lh) {
        std::vector<rc::u8> bytes = rc::read_file(dir + "/sound_bank.bin");
        rc::SoundBank b = rc::parse_sound_bank(rc::Buffer(bytes));
        std::vector<rc::u8> dump = {'R', 'C', 'S', 'D'};
        auto raw = [&](const void* q, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(q), reinterpret_cast<const rc::u8*>(q) + n); };
        auto u32w = [&](rc::u32 v) { raw(&v, 4); };
        auto section = [&](const std::vector<rc::u8>& v) { u32w(rc::u32(v.size())); raw(v.data(), v.size()); };
        std::vector<rc::u8> sec;
        auto put32 = [&](rc::u32 v) { sec.insert(sec.end(), reinterpret_cast<rc::u8*>(&v), reinterpret_cast<rc::u8*>(&v) + 4); };
        const rc::SfxHeader& h = b.header;
        for (rc::u32 v : {b.file_type, 2u, b.chunk_offset[0], b.chunk_size[0], b.chunk_offset[1], b.chunk_size[1], h.version, h.flags, h.bank_id,
                          rc::u32(rc::s32(h.bank_num)), rc::u32(rc::s32(h.n_sounds)), rc::u32(rc::s32(h.n_grains)), rc::u32(rc::s32(h.n_vags)),
                          h.first_sound, h.first_grain, h.vags_in_sr, h.vag_data_size, h.sram_alloc_size, h.next_block}) put32(v);
        section(sec); sec.clear();
        for (const auto& s : b.sounds)
            for (rc::s32 v : {rc::s32(s.vol), rc::s32(s.vol_group), rc::s32(s.pan), rc::s32(s.n_grains), rc::s32(s.instance_limit), rc::s32(s.flags), rc::s32(s.first_grain)}) put32(rc::u32(v));
        section(sec); sec.clear();
        for (const auto& g : b.grains) { put32(g.type); put32(rc::u32(g.delay)); sec.insert(sec.end(), g.data.begin(), g.data.end()); }
        section(sec); sec.clear();
        for (const auto& v : b.vags) { put32(v.offset); put32(v.frames); put32(rc::u32(v.loop_start)); put32(v.looped ? 1 : 0); put32(rc::u32(v.next_flags)); }
        section(sec); sec.clear();
        size_t samples = 0;
        for (bool rounded : {true, false}) {
            for (const auto& v : b.vags) {
                std::vector<rc::s16> pcm = rc::decode_adpcm(rc::Buffer(b.samples).sub(v.offset, size_t(v.frames) * 16), rounded);
                if (rounded) samples += pcm.size();
                sec.insert(sec.end(), reinterpret_cast<rc::u8*>(pcm.data()), reinterpret_cast<rc::u8*>(pcm.data()) + pcm.size() * 2);
            }
            section(sec); sec.clear();
        }
        size_t n_defs = 0, n_classes = 0;
        if (ci && cd && lh) {
            rc::LevelCore core = rc::parse_level_core(rc::Buffer(*ci), cd->size());
            rc::LevelSoundDefs d = rc::parse_level_sound_defs(rc::Buffer(*ci), core, rc::Buffer(*cd));
            u32w(rc::u32(d.level_defs.size() * sizeof(rc::SoundDef))); raw(d.level_defs.data(), d.level_defs.size() * sizeof(rc::SoundDef));
            u32w(rc::u32(d.map.size() * 2)); raw(d.map.data(), d.map.size() * 2);
            for (const auto& c : d.classes) {
                put32(rc::u32(c.o_class)); put32(rc::u32(c.bank_ids.size()));
                sec.insert(sec.end(), reinterpret_cast<const rc::u8*>(c.bank_ids.data()), reinterpret_cast<const rc::u8*>(c.bank_ids.data()) + c.bank_ids.size() * 2);
                put32(c.header_count ? rc::u32(*c.header_count) : 0xffffffffu); put32(rc::u32(c.defs.size()));
                sec.insert(sec.end(), reinterpret_cast<const rc::u8*>(c.defs.data()), reinterpret_cast<const rc::u8*>(c.defs.data()) + c.defs.size() * sizeof(rc::SoundDef));
                if (!c.bank_ids.empty()) n_classes++;
            }
            section(sec); sec.clear();
            std::array<rc::s32, 15> music = rc::read_music_table(rc::Buffer(*lh));
            for (rc::s32 m : music) put32(rc::u32(m));
            section(sec); sec.clear();
            n_defs = d.level_defs.size();
        } else {
            for (int k = 0; k < 4; k++) u32w(0);
        }
        rc::write_file(dir + "/sound_dump.bin", dump);
        std::printf("%s: %d sounds, %zu grains, %zu vags (%zu samples), %zu level defs, %zu classes with sounds\n",
                    dir.c_str(), b.header.n_sounds, b.grains.size(), b.vags.size(), samples, n_defs, n_classes);
        return samples;
    };
    std::vector<std::string> dirs;
    for (const auto& lvdir : fs::directory_iterator(out + "/levels")) dirs.push_back(lvdir.path().string());
    std::sort(dirs.begin(), dirs.end());
    size_t levels = 0, total = 0;
    for (const std::string& d : dirs) {
        if (!fs::exists(d + "/sound_bank.bin")) continue;
        std::vector<rc::u8> ci = rc::read_file(d + "/core_index.bin"), cd = rc::read_file(d + "/core_data.dec"), lh = rc::read_file(d + "/level_header.bin");
        total += dump_bank(d, &ci, &cd, &lh);
        levels++;
    }
    if (fs::exists(out + "/global/sound_bank.bin")) total += dump_bank(out + "/global", nullptr, nullptr, nullptr);
    std::printf("sound: %zu levels + global, %zu decoded samples\n", levels, total);
    return 0;
}

// hud: per level, the HUD tables and every decoded frame (docs/plan/hud_text.md §1), the three glyph tables
// found through the font wrappers (§3.1) and the English messages of gameplay_ntsc (§5), as
// levels/NN/hud_dump.bin: "RCHD", sec(header 0xb4), sec(icons), sec(frames), sec(palettes), sec(textures),
// u32 frame count, per frame {u32 w, u32 h, sec(rgba)}, u32 table address[3], sec(3 x 232 glyphs),
// u32 message count, per message {s32 id, s32 help_audio, sec(text)}. sec = u32 byte length + bytes.
int cmd_hud(const std::string& out) {
    namespace fs = std::filesystem;
    std::vector<std::string> dirs;
    for (const auto& lvdir : fs::directory_iterator(out + "/levels")) dirs.push_back(lvdir.path().string());
    std::sort(dirs.begin(), dirs.end());
    std::vector<rc::u8> global_header;
    if (fs::exists(out + "/global/hud_header.bin")) global_header = rc::read_file(out + "/global/hud_header.bin");
    size_t levels = 0, frames = 0, texels = 0, messages = 0, same_as_global = 0;
    for (const std::string& d : dirs) {
        if (!fs::exists(d + "/hud_header.bin") || !fs::exists(d + "/overlay.bin")) continue;
        std::vector<rc::u8> header = rc::read_file(d + "/hud_header.bin");
        std::array<std::vector<rc::u8>, 5> banks;
        rc::HudHeader hh = rc::Buffer(header).read<rc::HudHeader>(0, "hud header");
        for (size_t b = 0; b < 5; b++) {
            if (hh.bank_size[b] == 0) continue;
            std::string dec = d + "/hud_bank_" + std::to_string(b) + ".dec";
            banks[b] = fs::exists(dec) ? rc::read_file(dec) : rc::wad_decompress(rc::read_file(d + "/hud_bank_" + std::to_string(b) + ".bin"));
        }
        rc::Hud hud = rc::parse_hud(rc::Buffer(header), banks);
        rc::GlyphTables glyphs = rc::find_glyph_tables(rc::parse_ratchet_executable(rc::Buffer(rc::read_file(d + "/overlay.bin"))));
        std::vector<rc::u8> gp = rc::read_file(d + "/gameplay_ntsc.dec");
        std::vector<rc::Message> msgs = rc::parse_messages(rc::Buffer(gp), 0);

        std::vector<rc::u8> dump = {'R', 'C', 'H', 'D'};
        auto raw = [&](const void* q, size_t n) { if (n) dump.insert(dump.end(), reinterpret_cast<const rc::u8*>(q), reinterpret_cast<const rc::u8*>(q) + n); };
        auto u32w = [&](size_t v) { rc::u32 x = rc::u32(v); raw(&x, 4); };
        auto sec = [&](const void* q, size_t n) { u32w(n); raw(q, n); };
        auto vsec = [&](const auto& v) { sec(v.data(), v.size() * sizeof(v[0])); };
        sec(&hud.header, sizeof hud.header);
        vsec(hud.icons);
        vsec(hud.frames);
        vsec(hud.palettes);
        vsec(hud.textures);
        u32w(hud.frames.size());
        for (size_t i = 0; i < hud.frames.size(); i++) {
            rc::Image img = rc::decode_hud_frame(hud, i);
            u32w(img.width); u32w(img.height); vsec(img.rgba);
            texels += img.rgba.size() / 4;
        }
        for (rc::u32 a : glyphs.address) u32w(a);
        sec(glyphs.table.data(), sizeof glyphs.table);
        u32w(msgs.size());
        for (const auto& m : msgs) { u32w(rc::u32(m.id)); u32w(rc::u32(m.help_audio)); sec(m.text.data(), m.text.size()); }
        rc::write_file(d + "/hud_dump.bin", dump);

        bool global = !global_header.empty() && global_header.size() >= header.size() &&
                      std::equal(header.begin(), header.end(), global_header.begin());
        for (size_t b = 0; global && b < 5; b++) {
            std::string g = out + "/global/hud_banks/00" + std::to_string(b) + ".dec";
            global = fs::exists(g) ? rc::read_file(g) == banks[b] : banks[b].empty();
        }
        same_as_global += global;
        std::printf("%s: %zu icons, %zu frames, %zu palettes, %zu textures, glyph tables 0x%x/0x%x/0x%x, %zu messages%s\n",
                    d.c_str(), hud.icons.size() - 1, hud.frames.size(), hud.palettes.size(), hud.textures.size(),
                    glyphs.address[0], glyphs.address[1], glyphs.address[2], msgs.size(), global ? " (= global HUD)" : "");
        levels++; frames += hud.frames.size(); messages += msgs.size();
    }
    std::printf("hud: %zu levels (%zu with the global HUD), %zu frames (%zu texels), %zu English messages\n",
                levels, same_as_global, frames, texels, messages);
    return 0;
}

// Scene tables and chunks (docs/plan/cutscenes_transitions.md sections 1-2) of every level, from the
// unpacked `level_header.bin`, `scene/KK_{ntsc,pal}.bin` and `speech/KK_<lang>.bin`, dumped to
// <out>/levels/NN/scene_dump.bin as the golden reference for the Rust port
// (crates/rc-formats/src/scene.rs, tests/golden.rs `scenes_match_cpp_for_every_level`). Layout, all LE:
//   "SCN1", u32 records (15); per record: 6 x {u32 speech sector, u32 speech file bytes};
//   then NTSC, PAL: u32 entries, u32 sectors[entries], u32 chunks; per chunk: u32 decompressed size,
//   header[0x14], u32 camera records, records[0x20 each]; per actor: u32 offset, s32 class, s32 +4,
//   s32 +8, s32 track offset, sequence header[0x1c], per frame {u32 offset (rel. +0x10), frame header[0x10],
//   payload[qwc*16]}, u32 trigger count, u32 triggers[], positions[frames*16];
//   u32 subtitle count, per subtitle: entry[16], 5 x {u32 length, Latin-1 bytes}.
int cmd_scene(const std::string& out) {
    namespace fs = std::filesystem;
    size_t levels = 0, scenes = 0, chunks = 0, actors = 0, subtitles = 0, cuts = 0, regions = 0;
    long long ticks = 0;
    for (int lv = 0; lv < 19; lv++) {
        char nn[16];
        std::snprintf(nn, sizeof nn, "/levels/%02d", lv);
        std::string d = out + nn;
        if (!fs::exists(d + "/level_header.bin")) continue;
        std::vector<rc::u8> hb = rc::read_file(d + "/level_header.bin");
        rc::LevelHeader h;
        if (hb.size() < sizeof h) throw rc::FormatError("level header too short: " + d);
        std::memcpy(&h, hb.data(), sizeof h);
        std::vector<rc::u8> dump;
        auto u32w = [&](rc::u32 v) { for (int i = 0; i < 4; i++) dump.push_back(rc::u8(v >> (8 * i))); };
        auto raw = [&](const rc::u8* p, size_t n) { dump.insert(dump.end(), p, p + n); };
        raw(reinterpret_cast<const rc::u8*>("SCN1"), 4);
        u32w(rc::RAC1_SCENE_RECORDS);
        size_t lv_scenes = 0, lv_chunks = 0, lv_actors = 0;
        long long lv_ticks = 0;
        for (rc::u32 k = 0; k < rc::RAC1_SCENE_RECORDS; k++) {
            const rc::SceneRecord& r = h.scenes[k];
            for (rc::u32 l = 0; l < 6; l++) {
                char nb[64];
                std::snprintf(nb, sizeof nb, "/speech/%02u_%s.bin", k, rc::RAC1_SCENE_LANGUAGES[l]);
                u32w(rc::u32(r.speech[l]));
                u32w(r.speech[l] && fs::exists(d + nb) ? rc::u32(fs::file_size(d + nb)) : 0);
            }
            bool any = false;
            const rc::s32* lists[2] = {r.ntsc, r.pal};
            for (int g = 0; g < 2; g++) {
                rc::u32 n = rc::scene_region_entries(lists[g]);
                u32w(n);
                for (rc::u32 i = 0; i < n; i++) u32w(rc::u32(lists[g][i]));
                rc::u32 nchunks = n ? n - 1 : 0;
                u32w(nchunks);
                if (!nchunks) continue;
                any = true;
                regions++;
                char nb[64];
                std::snprintf(nb, sizeof nb, "/scene/%02u_%s.bin", k, g ? "pal" : "ntsc");
                std::vector<rc::u8> file = rc::read_file(d + nb);
                const rc::s32 tpc = g ? 80 : 96;
                for (rc::u32 c = 0; c < nchunks; c++) {
                    size_t at = size_t(lists[g][c] - lists[g][0]) * rc::SECTOR_SIZE;
                    size_t len = size_t(lists[g][c + 1] - lists[g][c]) * rc::SECTOR_SIZE;
                    if (at + len > file.size()) throw rc::FormatError(d + nb + ": chunk beyond the region file");
                    std::vector<rc::u8> dec = rc::wad_decompress(std::span<const rc::u8>(file.data() + at, len));
                    rc::Buffer b(dec);
                    try {
                    u32w(rc::u32(dec.size()));
                    raw(dec.data(), 0x14);
                    rc::s32 end = b.read<rc::s16>(0);
                    rc::s32 sub = b.read<rc::s32>(4);
                    rc::u32 count = b.read<rc::u16>(0xc);
                    rc::s32 cam = b.read<rc::s32>(0x10);
                    rc::s32 left = end - rc::s32(c) * tpc;
                    if (left <= 0) throw rc::FormatError(d + nb + ": chunk after the end tick");
                    rc::u32 ncam = rc::u32(std::min(left, tpc) + 1);
                    u32w(ncam);
                    raw(b.sub(size_t(cam), size_t(ncam) * 0x20).data(), size_t(ncam) * 0x20);
                    for (rc::u32 i = c == 0; i + 1 < ncam; i++) cuts += b.read<rc::u8>(size_t(cam) + i * 0x20 + 0xc) != 0;  // shown ticks 1..end-1
                    if (c == 0) { lv_ticks += end; lv_actors += count; }
                    for (rc::u32 a = 0; a < count; a++) {
                        rc::u32 ao = b.read<rc::u32>(0x14 + 4 * a);
                        u32w(ao);
                        for (int w = 0; w < 4; w++) u32w(b.read<rc::u32>(ao + 4 * w));
                        size_t seq = ao + 0x10;
                        raw(b.sub(seq, 0x1c).data(), 0x1c);
                        rc::u32 frames = b.read<rc::u8>(seq + 0x10), trig = b.read<rc::u8>(seq + 0x12);
                        if (trig == 0xff) trig = 0;  // scene sequences store 0xff: no trigger words follow
                        for (rc::u32 f = 0; f < frames; f++) {
                            rc::u32 fo = b.read<rc::u32>(seq + 0x1c + 4 * f);
                            u32w(fo);
                            rc::u32 qwc = b.read<rc::u16>(seq + fo + 6);
                            raw(b.sub(seq + fo, 0x10 + size_t(qwc) * 16).data(), 0x10 + size_t(qwc) * 16);
                        }
                        u32w(trig);
                        for (rc::u32 t = 0; t < trig; t++) u32w(b.read<rc::u32>(seq + 0x1c + 4 * (frames + t)));
                        rc::u32 track = b.read<rc::u32>(ao + 0xc);
                        raw(b.sub(track, size_t(frames) * 16).data(), size_t(frames) * 16);
                    }
                    std::vector<size_t> entries;
                    if (sub >= 0x400) {
                        for (size_t e = size_t(sub); b.read<rc::s16>(e) != -1; e += 16) entries.push_back(e);
                    }
                    u32w(rc::u32(entries.size()));
                    subtitles += entries.size();
                    for (size_t e : entries) {
                        raw(b.sub(e, 16).data(), 16);
                        for (int t = 0; t < 5; t++) {
                            rc::s16 o = b.read<rc::s16>(e + 4 + 2 * t);
                            std::vector<rc::u8> text;
                            if (o >= 0) for (size_t p = size_t(sub) + size_t(o); b.read<rc::u8>(p) != 0; p++) text.push_back(b.read<rc::u8>(p));
                            u32w(rc::u32(text.size()));
                            raw(text.data(), text.size());
                        }
                    }
                    } catch (const std::exception& e) {
                        throw rc::FormatError(d + nb + " chunk " + std::to_string(c) + ": " + e.what());
                    }
                    lv_chunks++;
                }
            }
            lv_scenes += any;
        }
        rc::write_file(d + "/scene_dump.bin", dump);
        std::printf("%s: %zu scenes, %zu chunks, %lld ticks (NTSC + PAL), %zu actors\n", d.c_str(), lv_scenes, lv_chunks, lv_ticks, lv_actors);
        levels++; scenes += lv_scenes; chunks += lv_chunks; ticks += lv_ticks; actors += lv_actors;
    }
    std::printf("scene: %zu levels, %zu scenes (%zu regions), %zu chunks, %lld ticks, %zu actors, %zu subtitle entries, %zu cut records\n",
                levels, scenes, regions, chunks, ticks, actors, subtitles, cuts);
    return 0;
}

int cmd_unpack(const rc::IsoImage& iso, const std::string& out) {
    rc::TableOfContents toc = rc::read_rac1_toc(iso);
    UnpackStats st;
    rc::u32 e_flags = 0x20924001;
    if (const rc::IsoEntry* elf = iso.find("/SCUS_971.99")) {
        std::vector<rc::u8> bytes = iso.read_file(*elf);
        e_flags = rc::Buffer(bytes).read<rc::u32>(36);
        rc::write_file(out + "/boot/" + elf->name, bytes);
    }
    if (const rc::IsoEntry* cnf = iso.find("/SYSTEM.CNF")) rc::write_file(out + "/boot/SYSTEM.CNF", iso.read_file(*cnf));
    if (const rc::IsoEntry* img = iso.find("/IOPRP243.IMG")) rc::write_file(out + "/boot/IOPRP243.IMG", iso.read_file(*img));
    rc::write_file(out + "/toc.bin", toc.raw);

    std::map<std::string, int> seen;
    for (const auto& l : rc::global_lumps(iso, toc)) {
        std::string name = l.name;
        if (seen[name]++) name += "." + std::to_string(seen[name]);
        write_lump(out + "/global/" + name, iso.read_bytes(rc::u64(l.sector) * rc::SECTOR_SIZE, l.bytes), st);
    }
    for (const auto& lv : toc.levels) unpack_level(iso, lv, out, e_flags, st);
    std::printf("wrote %zu lumps, %zu WAD streams decompressed, %zu failed\n", st.lumps, st.wads, st.failed);
    return st.failed ? 1 : 0;
}

int cmd_ls(const rc::IsoImage& iso) {
    for (const auto& e : iso.entries()) {
        std::printf("%c %10u bytes  lba %8u  %s\n", e.is_directory ? 'd' : 'f', e.size, e.lba, e.path.c_str());
    }
    return 0;
}

} // namespace

int main(int argc, char** argv) {
    Args args;
    if (!parse_args(argc, argv, args)) return usage();
    try {
        if (args.command == "textures") return cmd_textures(args.out);
        if (args.command == "tfrag") return cmd_tfrag(args.out, args.level);
        if (args.command == "tie") return cmd_tie(args.out, args.level);
        if (args.command == "shrub") return cmd_shrub(args.out, args.level);
        if (args.command == "sky") return cmd_sky(args.out, args.level);
        if (args.command == "moby") return cmd_moby(args.out, args.level);
        if (args.command == "gadget") return cmd_gadget(args.out, args.level);
        if (args.command == "collision") return cmd_collision(args.out, args.level);
        if (args.command == "occlusion") return cmd_occlusion(args.out, args.level);
        if (args.command == "particles") return cmd_particles(args.out);
        if (args.command == "hud") return cmd_hud(args.out);
        if (args.command == "sound") return cmd_sound(args.out);
        if (args.command == "scene") return cmd_scene(args.out);
        rc::IsoImage iso(args.iso);
        if (args.command == "info") return cmd_info(iso);
        if (args.command == "ls") return cmd_ls(iso);
        if (args.command == "toc") return cmd_toc(iso);
        if (args.command == "unpack") return cmd_unpack(iso, args.out);
        return usage();
    } catch (const std::exception& e) {
        std::fprintf(stderr, "error: %s\n", e.what());
        return 1;
    }
}
