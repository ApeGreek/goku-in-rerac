#pragma once
#include <array>
#include <optional>
#include <vector>
#include "core/buffer.h"
#include "core/level_core.h"

namespace rc {

// 989snd sound bank (sound_bank lump), SPU ADPCM decoding, the game's SoundDef records with the core-index
// remap, and the level header music table. Spec: docs/plan/audio.md section 2 and 4.
// Rust port: crates/rc-formats/src/sound_bank.rs and vag.rs (golden test sound_banks_match_cpp_for_every_level).

struct SfxHeader {
    u32 version = 0, flags = 0, bank_id = 0;
    s8 bank_num = 0;
    s16 n_sounds = 0, n_grains = 0, n_vags = 0;
    u32 first_sound = 0, first_grain = 0;
    u32 vags_in_sr = 0, vag_data_size = 0, sram_alloc_size = 0, next_block = 0;
};

struct SfxSound {
    s8 vol = 0, vol_group = 0;
    s16 pan = 0;
    u8 n_grains = 0;
    s8 instance_limit = 0;
    u16 flags = 0;
    u32 first_grain = 0;       // byte offset relative to SfxHeader::first_grain
};

struct SfxGrain {                // version-1 grain, 0x28 bytes
    u32 type = 0;
    s32 delay = 0;             // 240 Hz ticks
    std::array<u8, 32> data{};
};

// One sample of the ADPCM chunk as a voice plays it.
struct SampleExtent {
    u32 offset = 0;            // byte offset into the sample chunk
    u32 frames = 0;            // up to and including the first frame with the end flag
    s32 loop_start = -1;       // frame index of the last loop-start flag before the end, or -1
    bool looped = false;       // end frame has the repeat flag
    s32 next_flags = -1;       // flags of the frame after the end frame, -1 past the data
};

struct SoundBank {
    u32 file_type = 0;
    u32 chunk_offset[2] = {}, chunk_size[2] = {};
    SfxHeader header;
    std::vector<SfxSound> sounds;
    std::vector<SfxGrain> grains;        // all grains in sound order
    std::vector<SampleExtent> vags;      // distinct tone sample offsets, ascending
    std::vector<u8> samples;             // chunk 1
};

SoundBank parse_sound_bank(Buffer bytes);
SampleExtent sample_extent(Buffer samples, u32 offset);

// SPU ADPCM, 16-byte frames of 28 samples. rounded = PCSX2 form (K0*h1 + K1*h2 + 32) >> 6; otherwise
// OpenGOAL's separate truncating shifts.
std::vector<s16> decode_adpcm(Buffer frames, bool rounded = true);

struct SoundDef {                  // 0x20 bytes
    f32 near_dist, far_dist;
    s32 vol_far, vol_near, pb_lo, pb_hi;
    u8 looped, flags;
    u16 index;                     // after the remap: bank sound id (0xffff = none)
    u32 bank_handle;
};
static_assert(sizeof(SoundDef) == 0x20);

struct ClassSoundIds {
    s32 o_class = 0;
    std::vector<u16> bank_ids;           // from the remap list
    std::optional<u8> header_count;      // class header +0x0d, when the class has a blob
    std::vector<SoundDef> defs;          // class blob defs (+0x28) with index = remapped id
};

struct LevelSoundDefs {
    std::vector<SoundDef> level_defs;    // remapped
    std::vector<u16> map;
    std::vector<ClassSoundIds> classes;  // core-index class order
};

// core_index: raw lump; core_data: decompressed.
LevelSoundDefs parse_level_sound_defs(Buffer core_index, const LevelCore& core, Buffer core_data);

// level_header.bin: music[15] at 0x148.
std::array<s32, 15> read_music_table(Buffer level_header);

} // namespace rc
