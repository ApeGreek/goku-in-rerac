#pragma once
#include <array>
#include <string>
#include <vector>
#include "core/buffer.h"
#include "core/level.h"
#include "core/texture.h"

namespace rc {

// Per-level HUD graphics (hud_header + hud_banks), the fonts' glyph tables in the level overlay and the
// level message text. Spec: docs/plan/hud_text.md §1, §3.1, §5.
// Rust port: crates/rc-formats/src/{hud,font,strings}.rs (golden: `rc_extract hud` → hud_dump.bin).

// hud_header +0x00..+0xb4; every per-bank array is u32[8], the first 5 used.
struct HudHeader {
    u16 icon_count, frame_count;          // icon count includes the 0xffff terminator
    u32 icon_offset, frame_offset, palette_offset, texture_offset;
    u32 palette_cum[8], texture_cum[8];   // cumulative per-bank counts
    u32 bank_size[8];                     // decompressed bank sizes (0 = absent)
    u32 runtime_base[8], runtime_stash[8];
};
static_assert(sizeof(HudHeader) == 0xb4);

struct HudIcon { u16 id, frame_count, first_frame; u8 anim_mode, ticks_per_frame; };
struct HudFrameEntry { s16 palette, texture; };
struct HudPalette { u32 offset_flags; u16 cbp, pad; };       // offset | 0x80000000
struct HudTexture { u32 offset_flags; u16 tbp; u8 log2_w, log2_h; };
static_assert(sizeof(HudIcon) == 8 && sizeof(HudFrameEntry) == 4 && sizeof(HudPalette) == 8 && sizeof(HudTexture) == 8);

struct Hud {
    HudHeader header{};
    std::vector<HudIcon> icons;
    std::vector<HudFrameEntry> frames;
    std::vector<HudPalette> palettes;
    std::vector<HudTexture> textures;
    std::array<std::vector<u8>, 5> banks;   // decompressed
};

Hud parse_hud(Buffer header, const std::array<std::vector<u8>, 5>& banks);
// Frame i decoded like every level texture (CSM1 CLUT, alpha scaled 0x80 -> 0xff).
Image decode_hud_frame(const Hud& hud, size_t frame);

struct Glyph { u8 u, v; s8 y_off, advance; };
static_assert(sizeof(Glyph) == 4);
constexpr size_t GLYPH_COUNT = 232;

struct GlyphTables {
    std::array<u32, 3> address{};                             // regular (FX 1), small (FX 2), large (FX 3)
    std::array<std::array<Glyph, GLYPH_COUNT>, 3> table{};
};

// Finds the tables through the plain font wrappers (`jal GetEffectTex; li a0,n; lui t2,hi; 6 moves;
// jal FontPrint; addiu t2,t2,lo`) and reads them from the overlay sections.
GlyphTables find_glyph_tables(const std::vector<OverlaySection>& sections);

struct Message { s32 id = 0, help_audio = -1; std::string text; };
// Messages of language `lang` from the decompressed gameplay file (+0x10 + 4*lang -> block).
std::vector<Message> parse_messages(Buffer gameplay, u32 lang);

} // namespace rc
