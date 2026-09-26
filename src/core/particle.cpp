#include "core/particle.h"

namespace rc {

s32 PartDefs::start(size_t type) const {
    if (type >= offsets.size()) return -1;
    s32 off = offsets[type];
    s32 s = off == 0 ? 0 : off - header[2];
    return (s >= 0 && size_t(s) < blob.size()) ? s : -1;
}

PartDefs parse_part_defs(Buffer idx, s32 offset) {
    if (offset <= 0) throw FormatError("level has no part_defs");
    PartDefs d;
    std::vector<s32> h = idx.read_multiple<s32>(size_t(offset), 4, "part_defs header");
    for (int k = 0; k < 4; k++) d.header[k] = h[k];
    if (d.header[0] < 0 || d.header[0] > 0x100) throw FormatError("implausible part_defs count");
    if (d.header[2] < 0 || d.header[3] < 0) throw FormatError("negative part_defs blob range");
    d.offsets = idx.read_multiple<s32>(size_t(offset) + 0x10, size_t(d.header[0]), "part_defs offsets");
    d.blob = idx.read_multiple<u8>(size_t(offset) + size_t(d.header[2]), size_t(d.header[3]), "part_defs blob");
    return d;
}

static Image decode_bank(Buffer bank, s32 palette, s32 texture, s32 w, s32 h) {
    if (palette < 0 || texture < 0 || w <= 0 || h <= 0) throw FormatError("bank texture with negative offset or size");
    return decode_indexed8(bank.sub(size_t(texture), size_t(w) * size_t(h)).bytes(), u32(w), u32(h), bank.sub(size_t(palette), 1024).bytes());
}

ParticleTextures parse_particle_textures(Buffer idx, const LevelCoreHeader& h, Buffer part_bank, Buffer fx_bank) {
    ParticleTextures p;
    if (h.part_textures.count > 0 && h.part_textures.offset > 0)
        p.entries = idx.read_multiple<PartTextureEntry>(size_t(h.part_textures.offset), size_t(h.part_textures.count), "part_textures");
    if (h.fx_textures.count > 0 && h.fx_textures.offset > 0)
        p.fx_entries = idx.read_multiple<FxTextureEntry>(size_t(h.fx_textures.offset), size_t(h.fx_textures.count), "fx_textures");
    p.defs = parse_part_defs(idx, h.part_defs_offset);
    for (const auto& e : p.entries) p.textures.push_back(decode_bank(part_bank, e.palette, e.texture, e.side, e.side));
    for (const auto& e : p.fx_entries) {
        bool present = e.width > 0 && e.height > 0 && e.palette >= 0 && e.texture >= 0;
        if (present) p.fx_textures.push_back(decode_bank(fx_bank, e.palette, e.texture, e.width, e.height));
        else p.fx_textures.push_back(std::nullopt);
    }
    return p;
}

} // namespace rc
