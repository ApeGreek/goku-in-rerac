#include "core/texture.h"
#include <zlib.h>
#include <cstring>

namespace rc {

Image decode_indexed8(std::span<const u8> idx, u32 w, u32 h, std::span<const u8> clut) {
    if (idx.size() < size_t(w) * h) throw FormatError("indexed texture data too small");
    if (clut.size() < 1024) throw FormatError("palette too small");
    u8 pal[256][4];
    for (u32 i = 0; i < 256; i++) {
        const u8* e = clut.data() + clut_index(i) * 4;
        pal[i][0] = e[0]; pal[i][1] = e[1]; pal[i][2] = e[2]; pal[i][3] = scale_alpha(e[3]);
    }
    Image img;
    img.width = w; img.height = h;
    img.rgba.resize(size_t(w) * h * 4);
    for (size_t p = 0; p < size_t(w) * h; p++) std::memcpy(&img.rgba[p * 4], pal[idx[p]], 4);
    return img;
}

namespace {
void put_be32(std::vector<u8>& v, u32 x) { v.push_back(x >> 24); v.push_back(x >> 16); v.push_back(x >> 8); v.push_back(x); }
void chunk(std::vector<u8>& out, const char* type, const std::vector<u8>& data) {
    put_be32(out, u32(data.size()));
    size_t start = out.size();
    out.insert(out.end(), type, type + 4);
    out.insert(out.end(), data.begin(), data.end());
    u32 crc = u32(crc32(0, out.data() + start, uInt(out.size() - start)));
    put_be32(out, crc);
}
} // namespace

std::vector<u8> encode_png(const Image& img) {
    std::vector<u8> raw;
    raw.reserve((size_t(img.width) * 4 + 1) * img.height);
    for (u32 y = 0; y < img.height; y++) {
        raw.push_back(0); // filter: none
        raw.insert(raw.end(), img.rgba.begin() + size_t(y) * img.width * 4, img.rgba.begin() + size_t(y + 1) * img.width * 4);
    }
    uLongf clen = compressBound(uLong(raw.size()));
    std::vector<u8> comp(clen);
    if (compress2(comp.data(), &clen, raw.data(), uLong(raw.size()), 6) != Z_OK) throw std::runtime_error("zlib compress failed");
    comp.resize(clen);

    std::vector<u8> out = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
    std::vector<u8> ihdr;
    put_be32(ihdr, img.width); put_be32(ihdr, img.height);
    ihdr.push_back(8); ihdr.push_back(6); ihdr.push_back(0); ihdr.push_back(0); ihdr.push_back(0);
    chunk(out, "IHDR", ihdr);
    chunk(out, "IDAT", comp);
    chunk(out, "IEND", {});
    return out;
}

void write_png(const std::string& path, const Image& img) { write_file(path, encode_png(img)); }

} // namespace rc
