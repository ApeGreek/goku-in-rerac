#!/usr/bin/env python3
"""Bulk-exports decompiled C for every function of a program open in Ghidra to
decomp/export/<program>/<address>_<name>.c plus an index.tsv, so the code can be
grepped and read without round-tripping through the GUI. Re-run after renames.
"""
import os, sys, re
sys.path.insert(0, os.path.dirname(__file__))
from ghidra_http import get, post

program = sys.argv[1] if len(sys.argv) > 1 else "SCUS_971.99"
out = os.path.join(os.path.dirname(__file__), "../export", re.sub(r"[^A-Za-z0-9_.-]", "_", program))
os.makedirs(out, exist_ok=True)
funcs = get("list_functions", program=program, limit=20000)["functions"]
index = []
ok = fail = 0
for f in funcs:
    addr, name = f["address"].lower(), f["name"]
    res = get("decompile_function", address="0x" + addr, program=program, timeout=120)
    text = res.get("decompiled") or res.get("code") or res.get("result") if isinstance(res, dict) else res
    if not isinstance(text, str) or not text.strip():
        text = str(res)
        fail += 1
    else:
        ok += 1
    fname = f"{addr}_{re.sub(r'[^A-Za-z0-9_]', '_', name)}.c"
    with open(os.path.join(out, fname), "w") as fh:
        fh.write(f"// {program} {addr} {name}\n{text}\n")
    index.append(f"{addr}\t{name}\t{fname}")
    if (ok + fail) % 200 == 0:
        print(f"...{ok + fail}/{len(funcs)}")
with open(os.path.join(out, "index.tsv"), "w") as fh:
    fh.write("\n".join(index) + "\n")
print(f"{program}: exported {ok} functions, {fail} without decompiler output, to {out}")
