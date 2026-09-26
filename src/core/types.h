#pragma once
#include <cstdint>
#include <cstddef>

namespace rc {

// Fixed-width aliases matching the sizes the PS2 EE compiler used, so that
// ported structs keep their on-disk / in-memory layout byte for byte.
using u8 = uint8_t;
using u16 = uint16_t;
using u32 = uint32_t;
using u64 = uint64_t;
using s8 = int8_t;
using s16 = int16_t;
using s32 = int32_t;
using s64 = int64_t;
using f32 = float;

static_assert(sizeof(float) == 4);

} // namespace rc
