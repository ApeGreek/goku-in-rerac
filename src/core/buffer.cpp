#include "core/buffer.h"
#include <fstream>
#include <filesystem>

namespace rc {

size_t Buffer::check(size_t offset, size_t size, const char* what) const {
    if (offset > data_.size() || size > data_.size() - offset) {
        throw FormatError(std::string("read out of bounds for ") + what + " at 0x" +
                          [](size_t v) { char b[32]; snprintf(b, sizeof b, "%zx", v); return std::string(b); }(offset));
    }
    return offset;
}

Buffer Buffer::sub(size_t offset, size_t size) const {
    check(offset, size, "subrange");
    return Buffer(data_.subspan(offset, size));
}

std::string Buffer::read_string(size_t offset, size_t max_len) const {
    check(offset, max_len, "string");
    const char* p = reinterpret_cast<const char*>(data_.data() + offset);
    size_t n = 0;
    while (n < max_len && p[n] != '\0') n++;
    return std::string(p, n);
}

std::vector<u8> Buffer::copy(size_t offset, size_t size) const {
    check(offset, size, "copy");
    return std::vector<u8>(data_.begin() + offset, data_.begin() + offset + size);
}

std::vector<u8> read_file(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) throw std::runtime_error("cannot open " + path);
    f.seekg(0, std::ios::end);
    std::vector<u8> out(static_cast<size_t>(f.tellg()));
    f.seekg(0);
    f.read(reinterpret_cast<char*>(out.data()), out.size());
    return out;
}

void write_file(const std::string& path, std::span<const u8> bytes) {
    std::filesystem::create_directories(std::filesystem::path(path).parent_path());
    std::ofstream f(path, std::ios::binary);
    if (!f) throw std::runtime_error("cannot write " + path);
    f.write(reinterpret_cast<const char*>(bytes.data()), bytes.size());
}

} // namespace rc
