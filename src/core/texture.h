#pragma once
#include <string>
#include <vector>
#include "core/buffer.h"

namespace rc {

// RGBA8 image, row-major, top-down, alpha already scaled to 0..255.
struct Image {
    u32 width = 0, height = 0;
    std::vector<u8> rgba;
};

// PS2 CSM1 CLUT order -> linear: within each 32-entry group swap the two
// middle 8-entry blocks. Involution. Spec: docs/formats/textures_rac1.md 4.3.
inline u32 clut_index(u32 i) { return (((i >> 3) & 1) != ((i >> 4) & 1)) ? (i ^ 0x18) : i; }

// PS2 alpha 0..0x80 -> 0..255 (0x80 = opaque). Spec 4.4.
inline u8 scale_alpha(u8 a) { return a < 0x80 ? u8(a * 2) : 0xFF; }

// Decodes an 8-bit indexed image with a 256-entry RGBA32 CLUT in CSM1 order.
Image decode_indexed8(std::span<const u8> indices, u32 width, u32 height, std::span<const u8> clut_1024);

// Minimal PNG writer (zlib-compressed RGBA8).
std::vector<u8> encode_png(const Image& img);
void write_png(const std::string& path, const Image& img);

} // namespace rc
