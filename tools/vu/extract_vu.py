#!/usr/bin/env python3
"""Extracts the VU microprograms embedded in the boot ELF as .DVP.overlay
sections into extracted/vu/<program_id>.bin, assembling each program's chunks at
their VU1 (or VU0) micro-memory addresses. See docs/formats/vu_microprograms.md."""
import os, re, struct, sys, collections

elf_path = sys.argv[1] if len(sys.argv) > 1 else "extracted/boot/SCUS_971.99"
out_dir = sys.argv[2] if len(sys.argv) > 2 else "extracted/vu"
d = open(elf_path, "rb").read()
shoff, = struct.unpack_from("<I", d, 0x20)
shentsize, shnum, shstrndx = struct.unpack_from("<HHH", d, 0x2e)
shstr_off = struct.unpack_from("<I", d, shoff + shstrndx * shentsize + 0x10)[0]
# The .DVP.overlay sections are zero-filled in the file: the code bytes live in
# .vutext at the EE addresses recorded in .DVP.ovlytab {name_off, ee_addr, vu_addr}.
sections = {}
for i in range(shnum):
    name_off, typ, flags, addr, off, size = struct.unpack_from("<IIIIII", d, shoff + i * shentsize)
    name = d[shstr_off + name_off:d.index(b"\0", shstr_off + name_off)].decode()
    sections[name] = (typ, addr, off, size)
_, _, ovlytab_off, ovlytab_size = sections[".DVP.ovlytab"]
_, _, ovlystr_off, _ = sections[".DVP.ovlystrtab"]
_, vutext_addr, vutext_off, vutext_size = sections[".vutext"]
programs = collections.defaultdict(list)
for i in range(ovlytab_size // 12):
    name_off, ee_addr, vu_addr = struct.unpack_from("<III", d, ovlytab_off + i * 12)
    name = d[ovlystr_off + name_off:d.index(b"\0", ovlystr_off + name_off)].decode()
    m = re.match(r"\.DVP\.overlay\.\.0x([0-9a-f]+)\.(\d+)\.(\d+)\.(\d+)$", name)
    assert m and int(m.group(1), 16) == vu_addr, name
    prog, line, chunk = int(m.group(2)), int(m.group(3)), int(m.group(4))
    size = sections[name][3]
    file_off = ee_addr - vutext_addr + vutext_off
    assert vutext_off <= file_off and file_off + size <= vutext_off + vutext_size, name
    programs[prog].append((chunk, vu_addr, d[file_off:file_off + size], line))
os.makedirs(out_dir, exist_ok=True)
for prog, chunks in sorted(programs.items()):
    chunks.sort()
    end = max(vu + len(data) for _, vu, data, _ in chunks)
    img = bytearray(end)
    for _, vu, data, _ in chunks:
        img[vu:vu + len(data)] = data
    open(os.path.join(out_dir, f"{prog}.bin"), "wb").write(img)
    print(f"program {prog}: {len(chunks)} chunks, {end:#06x} bytes, first source line {chunks[0][3]}")
