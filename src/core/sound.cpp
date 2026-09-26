#include "core/sound.h"

#include <algorithm>
#include <cstring>

namespace rc {

namespace {
constexpr s32 kK0[5] = {0, 60, 115, 98, 122};
constexpr s32 kK1[5] = {0, 0, -52, -55, -60};
}

SampleExtent sample_extent(Buffer samples, u32 offset) {
    if (offset % 16) throw FormatError("sample offset not frame aligned");
    SampleExtent e;
    e.offset = offset;
    for (u32 f = 0;; f++) {
        size_t at = size_t(offset) + size_t(f) * 16;
        if (at + 16 > samples.size()) throw FormatError("sample runs past the ADPCM chunk without an end flag");
        u8 flags = samples.read<u8>(at + 1);
        if (flags & 4) e.loop_start = s32(f);
        if (flags & 1) {
            e.frames = f + 1;
            e.looped = (flags & 2) != 0;
            size_t next = at + 16;
            e.next_flags = next + 16 <= samples.size() ? s32(samples.read<u8>(next + 1)) : -1;
            return e;
        }
    }
}

std::vector<s16> decode_adpcm(Buffer frames, bool rounded) {
    std::vector<s16> out;
    out.reserve(frames.size() / 16 * 28);
    s32 h1 = 0, h2 = 0;
    for (size_t at = 0; at + 16 <= frames.size(); at += 16) {
        const u8* f = frames.data() + at;
        int shift = f[0] & 15;
        int filter = std::min(f[0] >> 4, 4);
        for (int i = 0; i < 28; i++) {
            int nib = (f[2 + i / 2] >> ((i & 1) * 4)) & 15;
            s32 s = s32(s16(u16(nib << 12))) >> shift;
            if (rounded) {
                s += (kK0[filter] * h1 + kK1[filter] * h2 + 32) >> 6;
            } else {
                s += (kK0[filter] * h1) >> 6;
                s += (kK1[filter] * h2) >> 6;
            }
            s = std::clamp<s32>(s, -32768, 32767);
            h2 = h1;
            h1 = s;
            out.push_back(s16(s));
        }
    }
    return out;
}

SoundBank parse_sound_bank(Buffer bytes) {
    SoundBank b;
    b.file_type = bytes.read<u32>(0);
    if (b.file_type != 3 || bytes.read<u32>(4) != 2) throw FormatError("sound bank: expected type 3 with 2 chunks");
    for (int k = 0; k < 2; k++) {
        b.chunk_offset[k] = bytes.read<u32>(8 + 8 * k);
        b.chunk_size[k] = bytes.read<u32>(12 + 8 * k);
    }
    Buffer blk = bytes.sub(b.chunk_offset[0], b.chunk_size[0]);
    b.samples = bytes.copy(b.chunk_offset[1], b.chunk_size[1]);
    if (blk.read_string(0, 4) != "SBlk") throw FormatError("sound bank: chunk 0 is not SBlk");
    SfxHeader& h = b.header;
    h.version = blk.read<u32>(4);
    h.flags = blk.read<u32>(8);
    h.bank_id = blk.read<u32>(0xc);
    h.bank_num = blk.read<s8>(0x10);
    h.n_sounds = blk.read<s16>(0x16);
    h.n_grains = blk.read<s16>(0x18);
    h.n_vags = blk.read<s16>(0x1a);
    h.first_sound = blk.read<u32>(0x1c);
    h.first_grain = blk.read<u32>(0x20);
    h.vags_in_sr = blk.read<u32>(0x24);
    h.vag_data_size = blk.read<u32>(0x28);
    h.sram_alloc_size = blk.read<u32>(0x2c);
    h.next_block = blk.read<u32>(0x30);
    if (h.version != 1) throw FormatError("SBlk: only version 1 grains are supported");
    if (h.n_sounds < 0) throw FormatError("SBlk: negative sound count");
    std::vector<u32> offsets;
    for (s16 i = 0; i < h.n_sounds; i++) {
        size_t o = size_t(h.first_sound) + 12 * size_t(i);
        SfxSound s;
        s.vol = blk.read<s8>(o);
        s.vol_group = blk.read<s8>(o + 1);
        s.pan = blk.read<s16>(o + 2);
        s.n_grains = blk.read<u8>(o + 4);
        s.instance_limit = blk.read<s8>(o + 5);
        s.flags = blk.read<u16>(o + 6);
        s.first_grain = blk.read<u32>(o + 8);
        for (u32 k = 0; k < s.n_grains; k++) {
            size_t g = size_t(h.first_grain) + s.first_grain + 0x28 * size_t(k);
            SfxGrain gr;
            gr.type = blk.read<u32>(g);
            gr.delay = blk.read<s32>(g + 4);
            std::vector<u8> d = blk.copy(g + 8, 32);
            std::copy(d.begin(), d.end(), gr.data.begin());
            if (gr.type == 1 || gr.type == 9) {
                u32 off;
                std::memcpy(&off, gr.data.data() + 16, 4);
                offsets.push_back(off);
            }
            b.grains.push_back(gr);
        }
        b.sounds.push_back(s);
    }
    std::sort(offsets.begin(), offsets.end());
    offsets.erase(std::unique(offsets.begin(), offsets.end()), offsets.end());
    for (u32 off : offsets) b.vags.push_back(sample_extent(Buffer(b.samples), off));
    return b;
}

LevelSoundDefs parse_level_sound_defs(Buffer idx, const LevelCore& core, Buffer data) {
    LevelSoundDefs out;
    s32 base = core.header.sound_remap_offset;
    if (base <= 0) return out;
    s16 defs_off = idx.read<s16>(size_t(base)), defs_count = idx.read<s16>(size_t(base) + 2);
    s16 map_off = idx.read<s16>(size_t(base) + 4), map_count = idx.read<s16>(size_t(base) + 6);
    if (defs_count < 0 || map_count < 0 || defs_off < 0 || map_off < 0) throw FormatError("sound remap: negative field");
    for (s16 k = 0; k < map_count; k++) out.map.push_back(idx.read<u16>(size_t(base + map_off) + 4 * size_t(k)));
    out.level_defs = idx.read_multiple<SoundDef>(size_t(base + defs_off), size_t(defs_count), "level sound defs");
    for (SoundDef& d : out.level_defs) {
        s16 i = s16(d.index);
        // FUN_00258128: signed compare, 0xffff + warning when too large; a negative index (-1 on levels 0, 11, 14)
        // reads the u16 before the map, as the game does.
        d.index = i < map_count ? idx.read<u16>(size_t(s64(base) + map_off + 4 * s64(i))) : 0xffff;
    }
    for (size_t c = 0; c < core.moby_classes.size(); c++) {
        const ClassEntry& e = core.moby_classes[c];
        size_t at = size_t(base) + 8 + 4 * c;
        s16 off = idx.read<s16>(at), count = idx.read<s16>(at + 2);
        ClassSoundIds cs;
        cs.o_class = e.o_class;
        for (s16 j = 0; j < count; j++) {
            if (off < 0) throw FormatError("sound remap: negative class list offset");
            cs.bank_ids.push_back(idx.read<u16>(size_t(base + off) + 4 * size_t(j)));
        }
        if (e.offset_in_asset_wad > 0) {
            size_t blob = size_t(e.offset_in_asset_wad);
            u8 n = data.read<u8>(blob + 0xd);
            s32 ptr = data.read<s32>(blob + 0x28);
            cs.header_count = n;
            if (n > 0 && ptr > 0) cs.defs = data.read_multiple<SoundDef>(blob + size_t(ptr), n, "class sound defs");
            // FUN_00258128 copies header-count entries, walking past this class's list when the remap count is
            // smaller (level 10 class 1229: 12 defs, 6 ids).
            for (size_t j = 0; j < cs.defs.size(); j++) cs.defs[j].index = idx.read<u16>(size_t(base + off) + 4 * j);
        }
        out.classes.push_back(std::move(cs));
    }
    return out;
}

std::array<s32, 15> read_music_table(Buffer level_header) {
    std::array<s32, 15> t{};
    for (int k = 0; k < 15; k++) t[size_t(k)] = level_header.read<s32>(0x148 + 4 * size_t(k));
    return t;
}

} // namespace rc
