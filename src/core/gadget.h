#pragma once
#include <vector>
#include "core/level_core.h"
#include "core/moby.h"

namespace rc {

// RAC1 gadget classes: Ratchet's hand-held weapons/gadgets (the wrench, o_class 71, among them).
// Spec: docs/formats/moby_rac1.md section 0.4 "Gadget classes".
//
// The core index lists them in a separate table (header.gadget_count / gadget_offset, 0x10-byte
// GadgetEntry records). Each entry points at one WAD stream inside the decompressed core data
// (offset_in_asset_wad .. + compressed_size); the stream decompresses to an ordinary moby class
// blob. The class keeps a normal moby class table entry with offset 0, whose textures[16] map the
// class's GS texture slots into the moby texture table, as for any other moby class.
// In game (level01.elf FUN_00258128, the core loader) the table is copied to the gadget arrays
// (o_class, compressed pointer, size, texture slots) and a gadget is decompressed on demand into a
// 0x18000-byte buffer (select_world_object_resource_tables, L01 0x259788 / boot 0x204a40).
struct GadgetClass {
    GadgetEntry entry;
    ClassEntry class_entry{};        // the moby class table entry with the same o_class
    size_t decompressed_size = 0;
    MobyClass mc;
};

constexpr size_t GADGET_BUFFER_SIZE = 0x18000;   // per game buffer (two, alternating)

// Parses every gadget class of a level in gadget-table order. core_data is the whole
// decompressed core data. Throws FormatError on a missing moby table entry or a bad stream.
std::vector<GadgetClass> parse_gadget_classes(const LevelCore& core, Buffer core_data);

} // namespace rc
