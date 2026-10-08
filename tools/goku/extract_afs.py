"""Extracts files from the AFS archives inside the BT3 ISO (standard library only, no 7z).
usage: python3 extract_afs.py <bt3.iso> <out_dir> <id> [id ...]   (global game ids, e.g. 0x590)"""
import struct, sys, pathlib
S = 2048
def rd(f, lba, n): f.seek(lba * S); return f.read(n)
def find(f, lba, size, name):
    d = rd(f, lba, size); i = 0
    while i < len(d):
        L = d[i]
        if L == 0: i = (i // S + 1) * S; continue
        nl = d[i + 32]; nm = d[i + 33:i + 33 + nl].decode().split(";")[0]
        if nm.upper() == name: return struct.unpack_from("<I", d, i + 2)[0], struct.unpack_from("<I", d, i + 10)[0]
        i += L
    raise SystemExit(f"{name} not found")
iso, out = sys.argv[1], pathlib.Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True)
with open(iso, "rb") as f:
    pvd = rd(f, 16, S); root = pvd[156:190]
    rl, rs = struct.unpack_from("<I", root, 2)[0], struct.unpack_from("<I", root, 10)[0]
    dl, ds = find(f, rl, rs, "DATA")
    for a in sys.argv[3:]:
        gid = int(a, 0)
        arc, idx = ("PZS3US0.AFS", gid) if gid <= 0 else ("PZS3US1.AFS", gid - 1) if gid < 0xD48 else ("PZS3US2.AFS", gid - 0xD48)
        al, _ = find(f, dl, ds, arc)
        f.seek(al * S); magic, cnt = struct.unpack("<4sI", f.read(8))
        f.seek(al * S + 8 + idx * 8); off, size = struct.unpack("<II", f.read(8))
        f.seek(al * S + off); (out / f"{gid:#06x}.bin").write_bytes(f.read(size))
        print(f"{gid:#06x} {arc}[{idx}] {size} bytes")
