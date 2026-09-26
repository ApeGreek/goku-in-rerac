// rc_vudis: disassembles a VU1 microprogram image (as produced by tools/vu/extract_vu.py).
#include <cstdio>
#include <fstream>
#include <vector>
#include <string>
#include "decompiler/VuDisasm/VuDisassembler.h"

int main(int argc, char** argv) {
    if (argc < 2) { std::fprintf(stderr, "usage: rc_vudis <program.bin> [vu0|vu1]\n"); return 2; }
    std::ifstream f(argv[1], std::ios::binary);
    std::vector<char> data((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
    bool vu0 = argc > 2 && std::string(argv[2]) == "vu0";
    decompiler::VuDisassembler dis(vu0 ? decompiler::VuDisassembler::VuKind::VU0 : decompiler::VuDisassembler::VuKind::VU1);
    auto prog = dis.disassemble(data.data(), int(data.size()), false);
    std::printf("%s", dis.to_string(prog).c_str());
    return 0;
}
