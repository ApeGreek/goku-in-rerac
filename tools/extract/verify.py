#!/usr/bin/env python3
"""Cross-checks an rc_extract output directory against independent ground truth.

1. Each level's decompressed core_data size must equal the assets_decompressed_size
   field at offset 0x8c of that level's core_index, and the compressed size must equal
   the field at 0x88.
2. Each level overlay's entry point, .text address and size must match Lombyte's
   independently recovered overlay index (config/overlays/us/index.json), if present.
"""
import json, os, struct, sys

root = sys.argv[1] if len(sys.argv) > 1 else "extracted"
lombyte = os.path.expanduser("~/Globals/Lombyte/config/overlays/us/index.json")
lidx = {e["index"]: e for e in json.load(open(lombyte))["levels"]} if os.path.exists(lombyte) else {}

bad = 0
for lv in sorted(os.listdir(os.path.join(root, "levels"))):
    d = os.path.join(root, "levels", lv)
    ci = open(os.path.join(d, "core_index.bin"), "rb").read()
    comp_size, dec_size = struct.unpack_from("<ii", ci, 0x88)
    raw = os.path.getsize(os.path.join(d, "core_data.bin"))
    wad_size = struct.unpack_from("<i", open(os.path.join(d, "core_data.bin"), "rb").read(7), 3)[0]
    dec_path = os.path.join(d, "core_data.dec")
    dec = os.path.getsize(dec_path) if os.path.exists(dec_path) else -1
    ok = (dec == dec_size) and (wad_size == comp_size)
    line = f"level {lv}: core_data compressed {wad_size} (index says {comp_size}), decompressed {dec} (index says {dec_size})"
    ov = {}
    for l in open(os.path.join(d, "overlay.txt")):
        parts = dict(kv.split("=") for kv in l.split()[2:])
        ov[int(l.split()[1])] = parts
    entry = int(ov[0]["entry"], 16)
    text_addr = int(ov[6]["dest"], 16); text_size = int(ov[6]["size"], 16)
    if int(lv) in lidx:
        e = lidx[int(lv)]
        ov_ok = e["entry"] == entry and e["text"]["addr"] == text_addr and e["text"]["size"] == text_size
        line += f"; overlay entry/text {'match' if ov_ok else 'MISMATCH'} Lombyte"
        ok = ok and ov_ok
    # 3. core blocks must tile the decompressed core data without overlap
    import re
    blocks = []
    for l in open(os.path.join(d, "core", "index.txt")):
        m = re.match(r"(\S+)\s+offset=0x([0-9a-f]+) size=0x([0-9a-f]+)", l)
        if m: blocks.append((int(m.group(2), 16), int(m.group(3), 16)))
    blocks.sort()
    overlaps = sum(1 for a, b in zip(blocks, blocks[1:]) if a[0] + a[1] > b[0])
    covered = sum(b[1] for b in blocks)
    tiled = overlaps == 0 and covered == dec_size
    line += f"; core blocks {len(blocks)} {'tile exactly' if tiled else f'MISMATCH (overlaps {overlaps}, covered {covered})'}"
    ok = ok and tiled
    print(("ok   " if ok else "FAIL ") + line)
    bad += not ok
print("all levels verified" if not bad else f"{bad} levels failed")
sys.exit(1 if bad else 0)
