#include "core/wad.h"
#include <cstring>

namespace rc {

bool is_wad(std::span<const u8> b) {
    return b.size() >= WAD_HEADER_SIZE && b[0] == 'W' && b[1] == 'A' && b[2] == 'D';
}

u32 wad_compressed_size(std::span<const u8> b) {
    if (!is_wad(b)) throw FormatError("not a WAD stream");
    u32 v;
    std::memcpy(&v, b.data() + 3, 4); // unaligned little-endian
    return v;
}

namespace {

struct Reader {
    const u8* begin; // start of packet stream, alignment origin
    const u8* end;
    const u8* ptr;
    u8 next() {
        if (ptr >= end) throw FormatError("WAD: read past end of stream");
        return *ptr++;
    }
};

void copy_literals(Reader& in, std::vector<u8>& out, size_t n) {
    if (size_t(in.end - in.ptr) < n) throw FormatError("WAD: literal runs past end of stream");
    out.insert(out.end(), in.ptr, in.ptr + n);
    in.ptr += n;
}

void copy_match(std::vector<u8>& out, size_t displacement, size_t len) {
    if (displacement == 0 || displacement > out.size()) throw FormatError("WAD: match source before start of output");
    size_t src = out.size() - displacement;
    out.reserve(out.size() + len);
    // Byte-by-byte forward copy: overlapping matches are run-length expansion.
    for (size_t i = 0; i < len; i++) out.push_back(out[src + i]);
}

} // namespace

std::vector<u8> wad_decompress(std::span<const u8> bytes) {
    u32 total = wad_compressed_size(bytes);
    if (total < WAD_HEADER_SIZE || total > bytes.size()) throw FormatError("WAD: compressed size exceeds buffer");

    Reader in{bytes.data() + WAD_HEADER_SIZE, bytes.data() + total, bytes.data() + WAD_HEADER_SIZE};
    std::vector<u8> out;

    while (in.ptr < in.end) {
        u8 flag = in.next();

        if (flag < 0x10) {
            // Literal packet. No trailing little literal.
            size_t n = flag == 0 ? size_t(in.next()) + 18 : size_t(flag) + 3;
            copy_literals(in, out, n);
            if (in.ptr < in.end && *in.ptr < 0x10) throw FormatError("WAD: adjacent literal packets");
            continue;
        }

        size_t match_len;
        size_t displacement;
        u8 little; // byte whose low two bits give the trailing literal length

        if (flag < 0x20) {
            // Far match, or a control packet when the displacement is zero.
            match_len = flag & 7;
            if (match_len == 0) match_len = size_t(in.next()) + 7;
            u8 b0 = in.next();
            u8 b1 = in.next();
            u32 a = (flag >> 3) & 1;
            u32 far = u32(b1) * 0x40 + (b0 >> 2);
            if (a == 0 && far == 0) {
                if (match_len != 1) {
                    // Pad packet: skip to the next 0x1000 boundary of the packet stream and stop the packet.
                    size_t pos = size_t(in.ptr - in.begin);
                    size_t aligned = (pos + 0xFFF) & ~size_t(0xFFF);
                    in.ptr = in.begin + aligned; // may land at or past end, which ends the loop
                    continue;
                }
                // Dummy packet: no copy, only the little literal.
                copy_literals(in, out, b0 & 3);
                continue;
            }
            // Bit 3 of the flag extends the 14-bit far displacement to 15 bits.
            // Verified against retail data: decompressed core_data sizes match the
            // sizes recorded in each level's core index (see tools/extract/verify.py).
            displacement = 0x4000 * (a + 1) + far;
            match_len += 2;
            little = b0;
        } else if (flag < 0x40) {
            match_len = flag & 0x1f;
            if (match_len == 0) match_len = size_t(in.next()) + 0x1f;
            match_len += 2;
            u8 b1 = in.next();
            u8 b2 = in.next();
            displacement = size_t(b2) * 0x40 + (b1 >> 2) + 1;
            little = b1;
        } else {
            u8 b1 = in.next();
            match_len = size_t(flag >> 5) + 1;
            displacement = size_t(b1) * 8 + ((flag >> 2) & 7) + 1;
            little = flag;
        }

        copy_match(out, displacement, match_len);
        copy_literals(in, out, little & 3);
    }
    return out;
}

} // namespace rc
