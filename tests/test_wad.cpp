#include "test.h"
#include "core/wad.h"

static std::vector<rc::u8> wad(std::initializer_list<int> packets) {
    std::vector<rc::u8> v = {'W', 'A', 'D', 0, 0, 0, 0, 'T', 'E', 'S', 'T', 0, 0, 0, 0, 0};
    for (int p : packets) v.push_back(rc::u8(p));
    rc::u32 size = rc::u32(v.size());
    std::memcpy(v.data() + 3, &size, 4);
    return v;
}

TEST(wad_literal_then_little_match) {
    // 0x01 => 4 literal bytes "abcd"; then little match flag 0xC0: len=(6+1)=7, disp=b1*8+0+1 with b1=0 => 1,
    // little literal = flag&3 = 0. Copies 'd' seven times.
    auto v = wad({0x01, 'a', 'b', 'c', 'd', 0xC0, 0x00});
    auto out = rc::wad_decompress(v);
    CHECK(std::string(out.begin(), out.end()) == "abcdddddddd");
}

TEST(wad_medium_match_with_little_literal) {
    // literal "xy", then medium match flag 0x21: len=1+2=3, b1=0x02 -> (b1>>2)=0 and little=2, b2=0 -> disp=1.
    // Copies 'y' 3 times, then 2 literal bytes "!?".
    auto v = wad({0x01, 'x', 'y', 'z', 'w', 0x21, 0x02, 0x00, '!', '?'});
    auto out = rc::wad_decompress(v);
    CHECK(std::string(out.begin(), out.end()) == "xyzwwww!?");
}

TEST(wad_dummy_packet_carries_little_literal) {
    // dummy: 0x11, b0 = 0x02 (2 literal bytes), b1 = 0
    auto v = wad({0x01, 'a', 'b', 'c', 'd', 0x11, 0x02, 0x00, 'e', 'f'});
    auto out = rc::wad_decompress(v);
    CHECK(std::string(out.begin(), out.end()) == "abcdef");
}

TEST(wad_rejects_adjacent_literals) {
    auto v = wad({0x01, 'a', 'b', 'c', 'd', 0x01, 'e', 'f', 'g', 'h'});
    CHECK_THROWS(rc::wad_decompress(v));
}

TEST(wad_pad_packet_ends_stream) {
    auto v = wad({0x01, 'a', 'b', 'c', 'd', 0x12, 0x00, 0x00, 0xEE, 0xEE});
    auto out = rc::wad_decompress(v);
    CHECK(std::string(out.begin(), out.end()) == "abcd");
}
