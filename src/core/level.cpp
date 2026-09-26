#include "core/level.h"
#include <cstring>

namespace rc {

LevelDataHeader read_level_data_header(Buffer data) {
    LevelDataHeader h;
    if (data.size() < sizeof h) throw FormatError("level data WAD too small for header");
    std::memcpy(&h, data.data(), sizeof h);
    return h;
}

std::vector<OverlaySection> parse_ratchet_executable(Buffer b) {
    std::vector<OverlaySection> out;
    size_t pos = 0;
    u32 entry = 0;
    while (pos + 16 <= b.size()) {
        u32 dest = b.read<u32>(pos);
        u32 size = b.read<u32>(pos + 4);
        u32 type = b.read<u32>(pos + 8);
        u32 ep = b.read<u32>(pos + 12);
        if (out.empty()) entry = ep;
        else if (ep != entry) break; // the game's own termination rule
        if (size > b.size() - pos - 16) throw FormatError("overlay section runs past end of lump");
        OverlaySection s;
        s.dest_address = dest;
        s.section_type = type;
        s.entry_point = ep;
        s.data = b.copy(pos + 16, size);
        out.push_back(std::move(s));
        pos += 16 + size;
    }
    if (out.empty()) throw FormatError("overlay has no sections");
    return out;
}

namespace {

struct Donor { const char* name; u32 type; u32 flags; u32 align; };
// docs/formats/wad_layouts_rac1.md section 4.4
constexpr u32 SHF_WRITE = 1, SHF_ALLOC = 2, SHF_EXECINSTR = 4, SHF_MIPS_GPREL = 0x10000000;
const Donor RAC1_DONOR[7] = {
    {".lit", 1, SHF_WRITE | SHF_ALLOC | SHF_MIPS_GPREL, 64},
    {".bss", 8, SHF_WRITE | SHF_ALLOC | SHF_MIPS_GPREL, 64},
    {".data", 1, SHF_WRITE | SHF_ALLOC, 64},
    {"lvl.vtbl", 1, SHF_ALLOC, 1},
    {"lvl.camvtbl", 1, SHF_ALLOC, 1},
    {"lvl.sndvtbl", 1, SHF_ALLOC, 1},
    {".text", 1, SHF_ALLOC | SHF_EXECINSTR, 64},
};

template <typename T> void put(std::vector<u8>& v, size_t at, T value) {
    if (v.size() < at + sizeof(T)) v.resize(at + sizeof(T));
    std::memcpy(v.data() + at, &value, sizeof(T));
}
template <typename T> void push(std::vector<u8>& v, T value) { put(v, v.size(), value); }

} // namespace

std::vector<u8> write_overlay_elf(const std::vector<OverlaySection>& secs, u32 e_flags) {
    bool donor_ok = secs.size() == 7;
    for (size_t i = 0; donor_ok && i < 7; i++) donor_ok = secs[i].section_type == RAC1_DONOR[i].type;

    // Section name string table.
    std::vector<u8> shstr = {0};
    std::vector<u32> name_off;
    for (size_t i = 0; i < secs.size(); i++) {
        std::string n = donor_ok ? RAC1_DONOR[i].name : ".unknown_" + std::to_string(i);
        name_off.push_back(u32(shstr.size()));
        shstr.insert(shstr.end(), n.begin(), n.end());
        shstr.push_back(0);
    }
    u32 shstr_name = u32(shstr.size());
    const char* shs = ".shstrtab";
    shstr.insert(shstr.end(), shs, shs + std::strlen(shs) + 1);

    // Layout: ELF header, program headers (one PT_LOAD per PROGBITS section),
    // section payloads, string table, section headers.
    std::vector<size_t> loadable;
    for (size_t i = 0; i < secs.size(); i++) if (secs[i].section_type != 8 && !secs[i].data.empty()) loadable.push_back(i);
    const u32 ehsize = 52, phentsize = 32, shentsize = 40;
    u32 phoff = ehsize;
    u32 data_off = phoff + phentsize * u32(loadable.size());

    std::vector<u8> f;
    f.resize(data_off);
    std::vector<u32> sec_off(secs.size(), 0);
    for (size_t i = 0; i < secs.size(); i++) {
        while (f.size() % 16) f.push_back(0);
        sec_off[i] = u32(f.size());
        if (secs[i].section_type != 8) f.insert(f.end(), secs[i].data.begin(), secs[i].data.end());
    }
    while (f.size() % 4) f.push_back(0);
    u32 shstr_off = u32(f.size());
    f.insert(f.end(), shstr.begin(), shstr.end());
    while (f.size() % 4) f.push_back(0);
    u32 shoff = u32(f.size());
    u32 shnum = u32(secs.size()) + 2; // null + sections + shstrtab

    // ELF header
    const u8 ident[16] = {0x7f, 'E', 'L', 'F', 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    std::memcpy(f.data(), ident, 16);
    put<u16>(f, 16, 2);              // ET_EXEC
    put<u16>(f, 18, 8);              // EM_MIPS
    put<u32>(f, 20, 1);              // EV_CURRENT
    put<u32>(f, 24, secs[0].entry_point);
    put<u32>(f, 28, phoff);
    put<u32>(f, 32, shoff);
    put<u32>(f, 36, e_flags);
    put<u16>(f, 40, ehsize);
    put<u16>(f, 42, phentsize);
    put<u16>(f, 44, u16(loadable.size()));
    put<u16>(f, 46, shentsize);
    put<u16>(f, 48, u16(shnum));
    put<u16>(f, 50, u16(shnum - 1));

    // Program headers
    for (size_t k = 0; k < loadable.size(); k++) {
        size_t i = loadable[k];
        size_t at = phoff + k * phentsize;
        put<u32>(f, at + 0, 1);                       // PT_LOAD
        put<u32>(f, at + 4, sec_off[i]);
        put<u32>(f, at + 8, secs[i].dest_address);
        put<u32>(f, at + 12, secs[i].dest_address);
        put<u32>(f, at + 16, u32(secs[i].data.size()));
        put<u32>(f, at + 20, u32(secs[i].data.size()));
        put<u32>(f, at + 24, 7);                      // RWX
        put<u32>(f, at + 28, 16);
    }

    // Section headers: null, sections, shstrtab
    auto sh = [&](u32 name, u32 type, u32 flags, u32 addr, u32 off, u32 size, u32 align) {
        push<u32>(f, name); push<u32>(f, type); push<u32>(f, flags); push<u32>(f, addr);
        push<u32>(f, off); push<u32>(f, size); push<u32>(f, 0); push<u32>(f, 0);
        push<u32>(f, align); push<u32>(f, 0);
    };
    sh(0, 0, 0, 0, 0, 0, 0);
    for (size_t i = 0; i < secs.size(); i++) {
        u32 flags = donor_ok ? RAC1_DONOR[i].flags : (SHF_ALLOC | SHF_WRITE | SHF_EXECINSTR);
        u32 align = donor_ok ? RAC1_DONOR[i].align : 1;
        sh(name_off[i], secs[i].section_type, flags, secs[i].dest_address, sec_off[i], u32(secs[i].data.size()), align);
    }
    sh(shstr_name, 3, 0, 0, shstr_off, u32(shstr.size()), 1);
    return f;
}

} // namespace rc
