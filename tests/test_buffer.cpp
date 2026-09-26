#include "test.h"
#include "core/buffer.h"

TEST(buffer_reads_little_endian) {
    std::vector<rc::u8> bytes = {0x78, 0x56, 0x34, 0x12, 'h', 'i', 0, 0};
    rc::Buffer b(bytes);
    CHECK(b.read<rc::u32>(0) == 0x12345678);
    CHECK(b.read<rc::u16>(0) == 0x5678);
    CHECK(b.read_string(4, 4) == "hi");
}

TEST(buffer_bounds_checked) {
    std::vector<rc::u8> bytes(4);
    rc::Buffer b(bytes);
    CHECK_THROWS(b.read<rc::u32>(1));
    CHECK_THROWS(b.sub(3, 2));
    CHECK(b.sub(2, 2).size() == 2);
}
