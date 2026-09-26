#include "core/vif.h"
#include <cstring>

namespace rc {

std::vector<VifPacket> parse_vif(std::span<const u8> list) {
    std::vector<VifPacket> out;
    size_t pos = 0;
    while (pos + 4 <= list.size()) {
        u32 code;
        std::memcpy(&code, list.data() + pos, 4);
        VifPacket p;
        p.cmd = (code >> 24) & 0x7f;
        p.num = (code >> 16) & 0xff;
        p.imm = code & 0xffff;
        p.offset = pos;
        size_t payload = 0;
        if (p.is_unpack()) {
            payload = (size_t(p.count()) * p.element_size() + 3) & ~size_t(3);
        } else {
            switch (p.cmd) {
            case VIF_STMASK: payload = 4; break;
            case VIF_STROW: case VIF_STCOL: payload = 16; break;
            case VIF_MPG: payload = size_t(p.count()) * 8; break;
            case VIF_DIRECT: case VIF_DIRECTHL: payload = size_t(p.imm == 0 ? 65536 : p.imm) * 16; break;
            default: payload = 0; break;
            }
        }
        if (pos + 4 + payload > list.size()) throw FormatError("VIF packet runs past end of list");
        p.data = list.subspan(pos + 4, payload);
        out.push_back(p);
        pos += 4 + payload;
    }
    return out;
}

} // namespace rc
