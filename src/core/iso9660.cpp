#include "core/iso9660.h"
#include <algorithm>
#include <cctype>
#include <fstream>

namespace rc {

struct IsoImage::Impl {
    mutable std::ifstream file;
    u64 file_size = 0;
};

static std::string upper(std::string s) {
    for (auto& c : s) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return s;
}

IsoImage::IsoImage(const std::string& path) : impl_(std::make_unique<Impl>()) {
    impl_->file.open(path, std::ios::binary);
    if (!impl_->file) throw std::runtime_error("cannot open ISO: " + path);
    impl_->file.seekg(0, std::ios::end);
    impl_->file_size = static_cast<u64>(impl_->file.tellg());

    // Detect raw 2352-byte sectors (CD "bin" dumps) by the 12-byte sync pattern.
    static const u8 sync[12] = {0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00};
    u8 head[16] = {};
    impl_->file.seekg(0);
    impl_->file.read(reinterpret_cast<char*>(head), sizeof head);
    if (std::equal(std::begin(sync), std::end(sync), head)) {
        raw_sector_size_ = 2352;
        raw_user_offset_ = (head[15] == 2) ? 24 : 16; // mode 2 form 1 vs mode 1
    }
    sector_count_ = static_cast<u32>(impl_->file_size / raw_sector_size_);

    // Primary Volume Descriptor lives at sector 16.
    std::vector<u8> pvd = read_sectors(16, 1);
    Buffer b(pvd);
    if (b.read<u8>(0) != 1 || b.read_string(1, 5) != "CD001") {
        throw FormatError("no ISO 9660 primary volume descriptor at sector 16");
    }
    volume_id_ = b.read_string(40, 32);
    while (!volume_id_.empty() && volume_id_.back() == ' ') volume_id_.pop_back();
    if (b.read<u16>(128) != SECTOR_SIZE) throw FormatError("unexpected logical block size");

    // Root directory record is embedded at offset 156 of the PVD.
    u32 root_lba = b.read<u32>(156 + 2);
    u32 root_size = b.read<u32>(156 + 10);
    walk_directory(root_lba, root_size, "", 0);
}

IsoImage::~IsoImage() = default;

std::vector<u8> IsoImage::read_sectors(u32 lba, u32 count) const {
    if (u64(lba) + count > sector_count_) {
        throw FormatError("sector range " + std::to_string(lba) + "+" + std::to_string(count) +
                          " beyond end of image (" + std::to_string(sector_count_) + " sectors)");
    }
    std::vector<u8> out(size_t(count) * SECTOR_SIZE);
    if (raw_sector_size_ == SECTOR_SIZE) {
        impl_->file.seekg(u64(lba) * SECTOR_SIZE);
        impl_->file.read(reinterpret_cast<char*>(out.data()), out.size());
    } else {
        for (u32 i = 0; i < count; i++) {
            impl_->file.seekg(u64(lba + i) * raw_sector_size_ + raw_user_offset_);
            impl_->file.read(reinterpret_cast<char*>(out.data() + size_t(i) * SECTOR_SIZE), SECTOR_SIZE);
        }
    }
    if (!impl_->file) throw std::runtime_error("short read from ISO");
    return out;
}

std::vector<u8> IsoImage::read_bytes(u64 byte_offset, u64 size) const {
    u32 first = static_cast<u32>(byte_offset / SECTOR_SIZE);
    u32 last = static_cast<u32>((byte_offset + size + SECTOR_SIZE - 1) / SECTOR_SIZE);
    std::vector<u8> sectors = read_sectors(first, last - first);
    size_t skip = static_cast<size_t>(byte_offset % SECTOR_SIZE);
    return std::vector<u8>(sectors.begin() + skip, sectors.begin() + skip + size);
}

void IsoImage::walk_directory(u32 lba, u32 size, const std::string& prefix, int depth) {
    if (depth > 8) throw FormatError("directory nesting too deep");
    u32 sectors = (size + SECTOR_SIZE - 1) / SECTOR_SIZE;
    std::vector<u8> dir = read_sectors(lba, sectors);
    Buffer b(dir);
    size_t pos = 0;
    while (pos < size) {
        u8 len = b.read<u8>(pos);
        if (len == 0) {
            // Records never straddle sectors; a zero length means skip to the next sector.
            pos = (pos / SECTOR_SIZE + 1) * SECTOR_SIZE;
            continue;
        }
        u32 e_lba = b.read<u32>(pos + 2);
        u32 e_size = b.read<u32>(pos + 10);
        u8 flags = b.read<u8>(pos + 25);
        u8 name_len = b.read<u8>(pos + 32);
        std::string name = b.read_string(pos + 33, name_len);
        pos += len;
        if (name == "\0" || name == "\1" || name.empty() || name[0] == 0 || name[0] == 1) continue; // "." and ".."
        if (auto semi = name.find(';'); semi != std::string::npos) name.erase(semi);
        IsoEntry e;
        e.name = upper(name);
        e.path = prefix + "/" + e.name;
        e.lba = e_lba;
        e.size = e_size;
        e.is_directory = (flags & 2) != 0;
        entries_.push_back(e);
        if (e.is_directory) walk_directory(e_lba, e_size, e.path, depth + 1);
    }
}

const IsoEntry* IsoImage::find(const std::string& path) const {
    std::string want = upper(path);
    if (want.empty() || want[0] != '/') want = "/" + want;
    for (const auto& e : entries_) if (e.path == want) return &e;
    return nullptr;
}

} // namespace rc
