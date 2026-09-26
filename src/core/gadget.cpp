#include "core/gadget.h"
#include "core/wad.h"
#include <string>

namespace rc {

std::vector<GadgetClass> parse_gadget_classes(const LevelCore& core, Buffer core_data) {
    std::vector<GadgetClass> out;
    for (const GadgetEntry& g : core.gadgets) {
        const std::string id = "gadget " + std::to_string(g.class_number);
        if (g.offset_in_asset_wad <= 0 || g.compressed_size < s32(WAD_HEADER_SIZE)) throw FormatError(id + ": bad table entry");
        Buffer stream = core_data.sub(size_t(g.offset_in_asset_wad), size_t(g.compressed_size));
        if (!is_wad(stream.bytes()) || wad_compressed_size(stream.bytes()) != u32(g.compressed_size))
            throw FormatError(id + ": not a WAD stream of the table's size");
        GadgetClass gc;
        gc.entry = g;
        bool found = false;
        for (const ClassEntry& e : core.moby_classes) if (e.o_class == g.class_number) { gc.class_entry = e; found = true; break; }
        if (!found) throw FormatError(id + ": no moby class table entry");
        std::vector<u8> blob = wad_decompress(stream.bytes());
        gc.decompressed_size = blob.size();
        try {
            gc.mc = parse_moby_class(Buffer(blob));
        } catch (const std::exception& e) {
            throw FormatError(id + ": " + e.what());
        }
        out.push_back(std::move(gc));
    }
    return out;
}

} // namespace rc
