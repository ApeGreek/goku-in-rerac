#pragma once
#include <span>
#include <vector>
#include "core/buffer.h"

namespace rc {

// One VIF1 code with its inline payload. Spec: docs/formats/tfrag_rac1.md 2.1.
struct VifPacket {
    u8 cmd = 0;        // bits 30..24 of the code word
    u8 num = 0;        // bits 23..16 (0 means 256 for unpacks)
    u16 imm = 0;       // bits 15..0
    size_t offset = 0; // byte offset of the code word in the list
    std::span<const u8> data; // inline payload (unpack data, STROW rows, ...)

    bool is_unpack() const { return (cmd & 0x60) == 0x60; }
    u8 vn() const { return (cmd >> 2) & 3; }   // 0..3 = 1..4 components
    u8 vl() const { return cmd & 3; }          // 0=32-bit 1=16-bit 2=8-bit 3=5-bit
    bool usn() const { return (imm >> 14) & 1; }
    u16 addr() const { return imm & 0x3ff; }
    u32 count() const { return num == 0 ? 256 : num; }
    u32 element_size() const { return ((32 >> vl()) * (vn() + 1)) / 8; }
};

enum VifCmd : u8 { VIF_NOP = 0, VIF_STCYCL = 1, VIF_OFFSET = 2, VIF_BASE = 3, VIF_ITOP = 4, VIF_STMOD = 5, VIF_MSKPATH3 = 6, VIF_MARK = 7,
    VIF_FLUSHE = 0x10, VIF_FLUSH = 0x11, VIF_FLUSHA = 0x13, VIF_MSCAL = 0x14, VIF_MSCALF = 0x15, VIF_MSCNT = 0x17,
    VIF_STMASK = 0x20, VIF_STROW = 0x30, VIF_STCOL = 0x31, VIF_MPG = 0x4A, VIF_DIRECT = 0x50, VIF_DIRECTHL = 0x51 };

// Parses a VIF command list. Stops at the end of the buffer.
std::vector<VifPacket> parse_vif(std::span<const u8> list);

} // namespace rc
