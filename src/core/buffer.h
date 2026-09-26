#pragma once
#include <span>
#include <string>
#include <string_view>
#include <vector>
#include <stdexcept>
#include <cstring>
#include "core/types.h"

namespace rc {

struct FormatError : std::runtime_error {
    using std::runtime_error::runtime_error;
};

// Non-owning little-endian view over bytes with bounds-checked reads.
// The PS2 is little-endian, so every on-disc integer is read as-is.
class Buffer {
public:
    Buffer() = default;
    Buffer(std::span<const u8> bytes) : data_(bytes) {}
    Buffer(const std::vector<u8>& v) : data_(v) {}

    size_t size() const { return data_.size(); }
    const u8* data() const { return data_.data(); }
    std::span<const u8> bytes() const { return data_; }

    Buffer sub(size_t offset, size_t size) const;
    Buffer sub(size_t offset) const { return sub(offset, size() - check(offset, 0)); }

    template <typename T>
    T read(size_t offset, const char* what = "value") const {
        static_assert(std::is_trivially_copyable_v<T>);
        check(offset, sizeof(T), what);
        T out;
        std::memcpy(&out, data_.data() + offset, sizeof(T));
        return out;
    }
    template <typename T>
    std::vector<T> read_multiple(size_t offset, size_t count, const char* what = "array") const {
        static_assert(std::is_trivially_copyable_v<T>);
        check(offset, sizeof(T) * count, what);
        std::vector<T> out(count);
        if (count) std::memcpy(out.data(), data_.data() + offset, sizeof(T) * count);
        return out;
    }
    std::string read_string(size_t offset, size_t max_len) const;
    std::vector<u8> copy(size_t offset, size_t size) const;

private:
    size_t check(size_t offset, size_t size, const char* what = "range") const;
    std::span<const u8> data_;
};

std::vector<u8> read_file(const std::string& path);
void write_file(const std::string& path, std::span<const u8> bytes);

} // namespace rc
