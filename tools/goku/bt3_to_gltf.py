"""Converts a BT3 character (model file extracted from YOUR own disc) to glTF (.glb) with textures.
usage: python3 bt3_to_gltf.py <model_file.bin> <out.glb> [--anims anim_file.bin] [--skip-nodes 12,13] [--conjugate]
Standard library only. The output contains game data: keep it to yourself, do not share it."""
import struct, sys, json, zlib

# ---- GS memory (addressing from the Tenkaichi3Decomp port, port/src/gs/gs_core.c, MIT) ----
B32 = [[0,1,4,5,16,17,20,21],[2,3,6,7,18,19,22,23],[8,9,12,13,24,25,28,29],[10,11,14,15,26,27,30,31]]
B16 = [[0,2,8,10],[1,3,9,11],[4,6,12,14],[5,7,13,15],[16,18,24,26],[17,19,25,27],[20,22,28,30],[21,23,29,31]]
C32 = [[0,1,4,5,8,9,12,13],[2,3,6,7,10,11,14,15],[16,17,20,21,24,25,28,29],[18,19,22,23,26,27,30,31],
       [32,33,36,37,40,41,44,45],[34,35,38,39,42,43,46,47],[48,49,52,53,56,57,60,61],[50,51,54,55,58,59,62,63]]
C8F = [[0,4,16,20,32,36,48,52,2,6,18,22,34,38,50,54],[8,12,24,28,40,44,56,60,10,14,26,30,42,46,58,62],
       [33,37,49,53,1,5,17,21,35,39,51,55,3,7,19,23],[41,45,57,61,9,13,25,29,43,47,59,63,11,15,27,31]]
C4F = [[0,8,32,40,64,72,96,104,2,10,34,42,66,74,98,106,4,12,36,44,68,76,100,108,6,14,38,46,70,78,102,110],
       [16,24,48,56,80,88,112,120,18,26,50,58,82,90,114,122,20,28,52,60,84,92,116,124,22,30,54,62,86,94,118,126],
       [65,73,97,105,1,9,33,41,67,75,99,107,3,11,35,43,69,77,101,109,5,13,37,45,71,79,103,111,7,15,39,47],
       [81,89,113,121,17,25,49,57,83,91,115,123,19,27,51,59,85,93,117,125,21,29,53,61,87,95,119,127,23,31,55,63]]
C8 = [[C8F[y & 3][(x ^ 4) if (y >> 2) & 1 else x] + (y >> 2) * 64 for x in range(16)] for y in range(16)]
C4 = [[C4F[y & 3][(x ^ 4) if (y >> 2) & 1 else x] + (y >> 2) * 128 for x in range(32)] for y in range(16)]

def addr(bp, bw, psm, x, y):
    bw = bw or 1
    if psm == 0x14:  # 4 bit: address in nibbles
        return ((bp + ((y >> 2) & ~0x1F) * (bw >> 1) + ((x >> 2) & ~0x1F) + B16[(y >> 4) & 7][(x >> 5) & 3]) << 9) + C4[y & 15][x & 31]
    if psm == 0x13:  # 8 bit: in bytes
        return ((bp + ((y >> 1) & ~0x1F) * (bw >> 1) + ((x >> 2) & ~0x1F) + B32[(y >> 4) & 3][(x >> 4) & 7]) << 8) + C8[y & 15][x & 15]
    return ((bp + (y & ~0x1F) * bw + ((x >> 1) & ~0x1F) + B32[(y >> 3) & 3][(x >> 3) & 7]) << 6) + C32[y & 7][x & 7]  # 32 bit: in words

def decode_texture(tf, i):
    po, co, ps, cs, _, _, pblt, cblt, _, _ = struct.unpack_from("<10I", tf, 8 + 0 + struct.unpack_from("<I", tf, 4)[0] * 4 + i * 0x40 - 8)
    t0 = struct.unpack_from("<Q", tf, struct.unpack_from("<I", tf, 4)[0] * 4 + i * 0x40 + 0x30)[0]
    psm, tbw, tw, th = (t0 >> 20) & 0x3F, (t0 >> 14) & 0x3F, 1 << ((t0 >> 26) & 15), 1 << ((t0 >> 30) & 15)
    if psm not in (0x13, 0x14) or not co:
        return None
    vram = {}
    def upload(ofs, blt):  # packet: DIRECT + GIF A+D (BITBLTBUF, TRXPOS, TRXREG, TRXDIR) + IMAGE
        p = ofs * 4
        rw, rh = struct.unpack_from("<II", tf, p + 0x30)
        dbw, dpsm = blt & 0x3F, (blt >> 8) & 0x3F
        assert dpsm == 0, "upload is not 32-bit"
        data = struct.unpack_from(f"<{rw * rh}I", tf, p + 0x60)
        return dbw, rw, rh, data
    dbw, rw, rh, data = upload(po, pblt)
    for y in range(rh):
        for x in range(rw):
            vram[addr(0, dbw, 0, x, y)] = data[y * rw + x]
    _, cw, ch, cdata = upload(co, cblt)
    clut = list(cdata)
    if psm == 0x13 and len(clut) == 256:  # CSM1: swap of the 8-colour blocks
        clut = [clut[(c & ~0x18) | ((c & 8) << 1) | ((c & 16) >> 1)] for c in range(256)]
    px = bytearray()
    for y in range(th):
        px.append(0)
        for x in range(tw):
            a = addr(0, tbw, psm, x, y)
            if psm == 0x14:
                c = (vram.get(a >> 3, 0) >> ((a & 7) * 4)) & 15
            else:
                c = (vram.get(a >> 2, 0) >> ((a & 3) * 8)) & 255
            v = clut[c] if c < len(clut) else 0
            px += bytes((v & 255, (v >> 8) & 255, (v >> 16) & 255, min(255, ((v >> 24) & 255) * 2)))
    return tw, th, png(tw, th, bytes(px))

def png(w, h, raw):
    c = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    return b"\x89PNG\r\n\x1a\n" + c(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + c(b"IDAT", zlib.compress(raw)) + c(b"IEND", b"")

def entry(pak, n):
    a, b = struct.unpack_from("<II", pak, n * 4)
    return pak[a & ~3:b & ~3] if a != b else None

def parse_model(m):
    """Returns (bones, meshes). Bones: (id, parent, origin). Meshes: (node, tbp, [vertices, 12 floats])."""
    bones, meshes, stack = [], [], [-1]
    o = struct.unpack_from("<I", m, 0x6C)[0]
    while True:
        nxt, pop, last, _, nid = struct.unpack_from("<iHHHH", m, o)
        origin = struct.unpack_from("<3f", m, o + 0x10)
        bones.append((nid, stack[-1], origin))
        stack.append(nid)
        if pop:
            del stack[-pop:]
        if nxt > 0x60:
            p, end, tbp = o + 0x68, o + nxt, 0  # +0x60: DMA tag
            while p < end:
                w = struct.unpack_from("<I", m, p)[0]
                cmd, num = (w >> 24) & 0x7F, ((w >> 16) & 0xFF) or 256
                if cmd >= 0x60:
                    vn, vl = (cmd >> 2) & 3, cmd & 3
                    size = ((32 >> vl) * (vn + 1) * num + 31) // 32 * 4
                    if cmd == 0x6C and w & 0x3FF == 0 and num == 5:  # texture reference: TEX0 in words 5/6
                        tbp = struct.unpack_from("<I", m, p + 4 + 16)[0] & 0x3FFF
                    elif cmd == 0x6C and num % 3 == 0:
                        v = struct.unpack_from(f"<{num * 4}f", m, p + 4)
                        meshes.append((nid, tbp, [v[i * 12:i * 12 + 12] for i in range(num // 3)]))
                    p += 4 + size
                elif w == 0x70000000:
                    break
                else:
                    p += 4
        if last or nxt <= 0:
            break
        o += nxt
    return bones, meshes

def main():
    src, out = sys.argv[1], sys.argv[2]
    skip = set(int(x) for x in sys.argv[sys.argv.index("--skip-nodes") + 1].split(",")) if "--skip-nodes" in sys.argv else set()
    pak = open(src, "rb").read()
    bones, meshes = parse_model(entry(pak, 3))
    tf = entry(pak, 0xC)
    cnt, eo = struct.unpack_from("<II", tf, 0)
    tex_by_tbp = {}
    for i in range(cnt):
        tbp = struct.unpack_from("<I", tf, eo * 4 + i * 0x40 + 0x20)[0]
        if tbp not in tex_by_tbp:
            tex_by_tbp[tbp] = i
    # glTF
    bin_, views, accs = bytearray(), [], []
    def add(data, target=None):
        while len(bin_) % 4: bin_.append(0)
        views.append({"buffer": 0, "byteOffset": len(bin_), "byteLength": len(data), **({"target": target} if target else {})})
        bin_.extend(data)
        return len(views) - 1
    def acc(fmt, rows, typ, comp=5126, target=34962, mm=False):
        data = b"".join(struct.pack(fmt, *r) for r in rows)
        a = {"bufferView": add(data, target), "componentType": comp, "count": len(rows), "type": typ}
        if mm:
            a["min"] = [min(r[i] for r in rows) for i in range(3)]; a["max"] = [max(r[i] for r in rows) for i in range(3)]
        accs.append(a); return len(accs) - 1
    ids = [b[0] for b in bones]
    joint_of = {nid: j for j, nid in enumerate(ids)}
    origin = {b[0]: b[2] for b in bones}
    nodes = []
    for nid, parent, org in bones:
        po = origin.get(parent, (0, 0, 0)) if parent >= 0 else (0, 0, 0)
        nodes.append({"name": f"nodo_{nid}", "translation": [org[k] - po[k] for k in range(3)]})
    for nid, parent, _ in bones:
        if parent >= 0:
            nodes[joint_of[parent]].setdefault("children", []).append(joint_of[nid])
    images, textures, materials, mat_of = [], [], [], {}
    def material(tbp):
        if tbp in mat_of: return mat_of[tbp]
        i = tex_by_tbp.get(tbp); dec = decode_texture(tf, i) if i is not None else None
        mat = {"name": f"mat_{tbp}", "doubleSided": True, "pbrMetallicRoughness": {"metallicFactor": 0, "roughnessFactor": 1}}
        if dec:
            images.append({"bufferView": add(dec[2]), "mimeType": "image/png"})
            textures.append({"source": len(images) - 1, "sampler": 0})
            mat["pbrMetallicRoughness"]["baseColorTexture"] = {"index": len(textures) - 1}
            mat["alphaMode"] = "MASK"
        materials.append(mat); mat_of[tbp] = len(materials) - 1; return mat_of[tbp]
    parent_of = {b[0]: b[1] for b in bones if b[1] >= 0}
    groups = {}
    for nid, tbp, verts in meshes:
        if nid in skip: continue
        g = groups.setdefault(tbp, ([], []))
        base = len(g[0])
        g[0].extend((v, joint_of.get(nid, 0), joint_of.get(parent_of.get(nid, nid), 0)) for v in verts)
        for i in range(len(verts) - 2):  # triangle strip
            a, b, c = base + i, base + i + 1, base + i + 2
            if len({verts[i][:3], verts[i + 1][:3], verts[i + 2][:3]}) == 3:
                g[1].append((a, b, c) if i % 2 == 0 else (b, a, c))
    prims = []
    for tbp, (vs, tris) in groups.items():
        prims.append({"attributes": {
            "POSITION": acc("<3f", [v[:3] for v, _, _ in vs], "VEC3", mm=True),
            "NORMAL": acc("<3f", [v[4:7] for v, _, _ in vs], "VEC3"),
            "TEXCOORD_0": acc("<2f", [(v[8], -v[9]) for v, _, _ in vs], "VEC2"),
            "JOINTS_0": acc("<4H", [(j, k, 0, 0) for _, j, k in vs], "VEC4", comp=5123),
            "WEIGHTS_0": acc("<4f", [(min(1, max(0, v[3])), 1 - min(1, max(0, v[3])), 0, 0) for v, _, _ in vs], "VEC4")},
            "indices": acc("<3I", tris, "SCALAR", comp=5125, target=34963) if False else None,
            "material": material(tbp)})
        idx = b"".join(struct.pack("<3I", *t) for t in tris)
        accs.append({"bufferView": add(idx, 34963), "componentType": 5125, "count": len(tris) * 3, "type": "SCALAR"})
        prims[-1]["indices"] = len(accs) - 1
    ibm = acc("<16f", [(1,0,0,0, 0,1,0,0, 0,0,1,0, -origin[n][0],-origin[n][1],-origin[n][2],1) for n in ids], "MAT4", target=None)
    animations = []
    if "--anims" in sys.argv:
        import os
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from bt3_anim import anims, decode
        conj = "--conjugate" in sys.argv
        for k, raw in anims(open(sys.argv[sys.argv.index("--anims") + 1], "rb").read()):
            if raw is None or len(raw) < 16: continue
            length, tracks = decode(raw)
            samplers, channels = [], []
            for nid, keys in tracks.items():
                if nid not in joint_of: continue
                keys = sorted({int(f): (t, q) for f, t, q in keys}.items())
                times = acc("<f", [(f / 60.0,) for f, _ in keys], "SCALAR", target=None)
                g_t = accs[times]; g_t["min"] = [keys[0][0] / 60.0]; g_t["max"] = [keys[-1][0] / 60.0]
                qs = [((-q[0], -q[1], -q[2], q[3]) if conj else tuple(q)) for _, (t, q) in keys]
                samplers.append({"input": times, "output": acc("<4f", qs, "VEC4", target=None), "interpolation": "LINEAR"})
                channels.append({"sampler": len(samplers) - 1, "target": {"node": joint_of[nid], "path": "rotation"}})
                if keys[0][1][0] is not None:
                    par = parent_of.get(nid)
                    po = origin.get(par, (0, 0, 0)) if par is not None else (0, 0, 0)
                    samplers.append({"input": times, "output": acc("<3f", [tuple(t[i] - po[i] for i in range(3)) for _, (t, q) in keys], "VEC3", target=None), "interpolation": "LINEAR"})
                    channels.append({"sampler": len(samplers) - 1, "target": {"node": joint_of[nid], "path": "translation"}})
            if channels:
                animations.append({"name": f"anim_{k:03d}", "samplers": samplers, "channels": channels})
    nodes.append({"name": "goku_mesh", "mesh": 0, "skin": 0})
    root = {"name": "goku", "children": [joint_of[b[0]] for b in bones if b[1] < 0] + [len(nodes) - 1], "scale": [1, -1, -1]}
    nodes.append(root)
    while len(bin_) % 4: bin_.append(0)
    g = {"asset": {"version": "2.0", "generator": "bt3_to_gltf (dati dal disco dell'utente)"}, "scene": 0,
         "scenes": [{"nodes": [len(nodes) - 1]}], "nodes": nodes, "meshes": [{"primitives": prims}],
         "skins": [{"joints": list(range(len(bones))), "inverseBindMatrices": ibm}],
         "materials": materials, **({"animations": animations} if animations else {}), "textures": textures, "images": images,
         "samplers": [{"magFilter": 9729, "minFilter": 9729}],
         "accessors": accs, "bufferViews": views, "buffers": [{"byteLength": len(bin_)}]}
    js = json.dumps(g).encode(); js += b" " * (-len(js) % 4)
    open(out, "wb").write(struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(bin_)) +
                          struct.pack("<II", len(js), 0x4E4F534A) + js + struct.pack("<II", len(bin_), 0x004E4942) + bin_)
    print(f"{out}: {len(bones)} bones, {sum(len(v) for v, _ in groups.values())} vertices, {len(materials)} materials, {len(images)} textures, {len(animations)} animations")

if __name__ == "__main__":
    main()
