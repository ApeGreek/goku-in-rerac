#pragma once
#include <memory>
#include <string>
#include <vector>
#include "core/buffer.h"

namespace rc {

// Minimal ISO 9660 reader sufficient for PS2 discs. PS2 games put a handful of
// named files in the filesystem (SYSTEM.CNF, the boot ELF, sometimes more) and
// address the rest of the disc by raw sector number, so the reader exposes both
// the directory tree and raw sector access.
constexpr u32 SECTOR_SIZE = 2048;

struct IsoEntry {
    std::string name;      // uppercase, without the ";1" version suffix
    std::string path;      // "/DIR/FILE.EXT"
    u32 lba = 0;           // first sector
    u32 size = 0;          // bytes
    bool is_directory = false;
};

class IsoImage {
public:
    explicit IsoImage(const std::string& path);
    ~IsoImage();

    u32 sector_count() const { return sector_count_; }
    // Reads count sectors starting at lba. Throws if out of range.
    std::vector<u8> read_sectors(u32 lba, u32 count) const;
    std::vector<u8> read_bytes(u64 byte_offset, u64 size) const;

    const std::vector<IsoEntry>& entries() const { return entries_; }
    const IsoEntry* find(const std::string& path) const; // case-insensitive, leading '/' optional
    std::vector<u8> read_file(const IsoEntry& e) const { return read_bytes(u64(e.lba) * SECTOR_SIZE, e.size); }

    std::string volume_id() const { return volume_id_; }
    // 2048 for a plain .iso, 2352 for a raw .bin dump (only the 2048 user bytes are returned).
    u32 raw_sector_size() const { return raw_sector_size_; }

private:
    void walk_directory(u32 lba, u32 size, const std::string& prefix, int depth);
    struct Impl;
    std::unique_ptr<Impl> impl_;
    std::vector<IsoEntry> entries_;
    std::string volume_id_;
    u32 sector_count_ = 0;
    u32 raw_sector_size_ = SECTOR_SIZE;
    u32 raw_user_offset_ = 0;
};

} // namespace rc
