#include "core/hud.h"

namespace rc {

namespace {

// First bank b (0..4) with i < cum[b].
int owning_bank(const u32 (&cum)[8], size_t i) {
    for (int b = 0; b < 5; b++) if (i < cum[b]) return b;
    return -1;
}

} // namespace

Hud parse_hud(Buffer header, const std::array<std::vector<u8>, 5>& banks) {
    Hud h;
    h.header = header.read<HudHeader>(0, "hud header");
    const HudHeader& H = h.header;
    if (H.icon_count == 0 || H.icon_count > 0x1000 || H.frame_count > 0x4000 || H.palette_cum[4] > 0x1000 || H.texture_cum[4] > 0x1000)
        throw FormatError("implausible hud header counts");
    h.icons = header.read_multiple<HudIcon>(H.icon_offset, H.icon_count, "hud icons");
    if (h.icons.back().id != 0xffff) throw FormatError("hud icon table has no terminator");
    h.frames = header.read_multiple<HudFrameEntry>(H.frame_offset, H.frame_count, "hud frames");
    h.palettes = header.read_multiple<HudPalette>(H.palette_offset, H.palette_cum[4], "hud palettes");
    h.textures = header.read_multiple<HudTexture>(H.texture_offset, H.texture_cum[4], "hud textures");
    for (size_t b = 0; b < 5; b++) {
        if (banks[b].size() < H.bank_size[b]) throw FormatError("hud bank shorter than its header size");
        h.banks[b] = banks[b];
    }
    return h;
}

Image decode_hud_frame(const Hud& hud, size_t frame) {
    if (frame >= hud.frames.size()) throw FormatError("hud frame out of range");
    const HudFrameEntry& f = hud.frames[frame];
    if (f.palette < 0 || f.texture < 0 || size_t(f.palette) >= hud.palettes.size() || size_t(f.texture) >= hud.textures.size())
        throw FormatError("hud frame entry out of range");
    const HudPalette& p = hud.palettes[size_t(f.palette)];
    const HudTexture& t = hud.textures[size_t(f.texture)];
    int pb = owning_bank(hud.header.palette_cum, size_t(f.palette));
    int tb = owning_bank(hud.header.texture_cum, size_t(f.texture));
    if (pb < 0 || tb < 0) throw FormatError("hud entry owned by no bank");
    u32 w = 1u << (t.log2_w & 31), h = 1u << (t.log2_h & 31);
    Buffer px = Buffer(hud.banks[size_t(tb)]).sub(t.offset_flags & 0x7fffffff, size_t(w) * h);
    Buffer clut = Buffer(hud.banks[size_t(pb)]).sub(p.offset_flags & 0x7fffffff, 1024);
    return decode_indexed8(px.bytes(), w, h, clut.bytes());
}

GlyphTables find_glyph_tables(const std::vector<OverlaySection>& sections) {
    GlyphTables out;
    bool have[3] = {false, false, false};
    for (const auto& s : sections) {
        if (s.section_type != 1) continue;
        size_t n = s.data.size() / 4;
        auto w = [&](size_t k) { u32 x; std::memcpy(&x, s.data.data() + 4 * k, 4); return x; };
        for (size_t i = 0; i + 10 < n; i++) {
            if ((w(i) >> 26) != 3 || (w(i + 9) >> 26) != 3) continue;           // jal GetEffectTex ... jal FontPrint
            u32 li = w(i + 1), lui = w(i + 2), lo = w(i + 10);
            if ((li & 0xffff0000u) != 0x24040000u || (lui & 0xffff0000u) != 0x3c0a0000u || (lo & 0xffff0000u) != 0x254a0000u) continue;
            u32 fx = li & 0xffff;
            if (fx < 1 || fx > 3) continue;
            u32 addr = ((lui & 0xffff) << 16) + u32(s32(s16(lo & 0xffff)));
            if (have[fx - 1] && out.address[fx - 1] != addr) throw FormatError("font wrappers disagree on a glyph table");
            out.address[fx - 1] = addr;
            have[fx - 1] = true;
        }
    }
    for (size_t f = 0; f < 3; f++) {
        if (!have[f]) throw FormatError("font wrapper not found");
        bool read = false;
        for (const auto& s : sections) {
            if (s.section_type == 8 || out.address[f] < s.dest_address) continue;
            size_t off = out.address[f] - s.dest_address;
            if (off + GLYPH_COUNT * 4 > s.data.size()) continue;
            std::memcpy(out.table[f].data(), s.data.data() + off, GLYPH_COUNT * 4);
            read = true;
            break;
        }
        if (!read) throw FormatError("glyph table outside the overlay");
    }
    return out;
}

std::vector<Message> parse_messages(Buffer gameplay, u32 lang) {
    u32 off = gameplay.read<u32>(0x10 + 4 * size_t(lang), "text block pointer");
    Buffer blk = gameplay.sub(off);
    u32 count = blk.read<u32>(0), size = blk.read<u32>(4);
    blk = blk.sub(0, size < 8 ? 8 : size);
    std::vector<Message> out;
    for (u32 k = 0; k < count; k++) {
        Message m;
        s32 text = blk.read<s32>(8 + 16 * size_t(k));
        m.id = blk.read<s32>(8 + 16 * size_t(k) + 4);
        m.help_audio = blk.read<s32>(8 + 16 * size_t(k) + 8);
        if (text < 0) throw FormatError("negative text offset");
        size_t p = size_t(text);
        while (blk.read<u8>(p) != 0) m.text.push_back(char(blk.read<u8>(p++)));
        out.push_back(std::move(m));
    }
    return out;
}

} // namespace rc
