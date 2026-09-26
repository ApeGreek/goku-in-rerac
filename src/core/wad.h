#pragma once
#include <span>
#include <vector>
#include "core/buffer.h"

namespace rc {

// Insomniac's "WAD" LZ77 stream: 16-byte header ("WAD" + s32 compressed size
// + 9 free bytes) followed by a byte-oriented packet stream. Full format
// description in docs/formats/disc_layout.md section 3.
constexpr size_t WAD_HEADER_SIZE = 0x10;

bool is_wad(std::span<const u8> bytes);
// Total stream size in bytes including the header, read from the header.
u32 wad_compressed_size(std::span<const u8> bytes);
std::vector<u8> wad_decompress(std::span<const u8> bytes);

} // namespace rc
