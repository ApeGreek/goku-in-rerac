"""Decodes the animations of a BT3 character (file 0x598 + chara*10, extracted from YOUR own disc)."""
import struct, math

def bpe(src):
    size, pack = struct.unpack_from("<ii", src, 0)
    s = memoryview(src)[8:8 + pack]; i = 0; out = bytearray()
    while i < pack:
        left = list(range(256)); right = [0] * 256
        c = 0; count = s[i]; i += 1
        while True:
            if count > 127:
                c += count - 127; count = 0
            if c == 256: break
            for _ in range(count + 1):
                left[c] = s[i]; i += 1
                if c != left[c]:
                    right[c] = s[i]; i += 1
                c += 1
            if c == 256: break
            count = s[i]; i += 1
        n = (s[i] << 8) + s[i + 1]; i += 2
        stack = []
        for _ in range(n):
            stack.append(s[i]); i += 1
            while stack:
                c = stack.pop()
                if c == left[c]: out.append(c)
                else: stack.append(right[c]); stack.append(left[c])
    return bytes(out[:size])

def quat(lo, hi):
    p = lo | hi << 32
    largest = p >> 60
    v = [((p >> (i * 20)) & 0xFFFFF) / 1048575.0 - 0.5 for i in range(3)]
    v = [x * 1.41421356 for x in v]
    w = math.sqrt(max(0.0, 1 - sum(x * x for x in v)))
    v.insert(largest, w)
    return v  # x, y, z, w

def anims(pak):
    n = struct.unpack_from("<I", pak, 0)[0]
    for k in range(1, n):
        a, b = struct.unpack_from("<II", pak, k * 4)
        yield k - 1, (pak[a & ~3:b & ~3] if a != b and b > a else None)

def decode(raw):
    """-> (length in frames, {node: [(frame, (x,y,z) or None, quat)]})"""
    d = bpe(raw)
    if len(d) < 0x94:
        return 0, {}
    _, nev, length, _ = struct.unpack_from("<BBHH", d, 0)
    tracks = {}
    for node in range(71):
        off = struct.unpack_from("<H", d, 6 + node * 2)[0]
        if not off: continue
        t = off * 4
        flags, cnt = struct.unpack_from("<HH", d, t)
        keys = []
        if flags & 1:
            times = struct.unpack_from(f"<{cnt}H", d, t + 4 + cnt * 8)
            for j in range(cnt):
                lo, hi = struct.unpack_from("<II", d, t + 4 + j * 8)
                keys.append((times[j], None, quat(lo, hi)))
        else:
            for j in range(cnt):
                x, y, z, fr, lo, hi = struct.unpack_from("<3fIII", d, t + 4 + j * 0x18)
                keys.append((fr, (x, y, z), quat(lo, hi)))
        tracks[node] = keys
    return length, tracks
