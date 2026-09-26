#!/usr/bin/env python3
import functools; print = functools.partial(print, flush=True)  # unbuffered progress when run in the background
"""Imports every level overlay ELF into the open Ghidra project, analyzes it,
applies Lombyte's boot-match names, and saves. Idempotent: skips levels whose
program already exists in the project.

Programs are named levelNN.elf under /levels. Overlay addresses overlap the
boot ELF by design, so each overlay is its own program.
"""
import json, os, sys, time
sys.path.insert(0, os.path.dirname(__file__))
from ghidra_http import get, post

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
NAMES = os.path.expanduser("~/Globals/Lombyte/config/overlays/us/names")
levels = [int(a) for a in sys.argv[1:]] or list(range(19))

def existing_programs():
    files = get("list_project_files", folder="/levels")
    return {f["name"] for f in files.get("files", [])} if isinstance(files, dict) else set()

def apply_names(program, level):
    path = os.path.join(NAMES, f"level-{level:02d}.json")
    if not os.path.exists(path):
        return 0
    funcs = {f["address"].lower(): f["name"] for f in get("list_functions", program=program, limit=10000)["functions"]}
    applied = 0
    for row in json.load(open(path))["functions"]:
        ev = row.get("name_evidence") or {}
        name = ev.get("name")
        addr = f"{row['addr']:08x}"
        if not name or funcs.get(addr, "").startswith("FUN_") is False:
            continue
        res = post("rename_function", old_name="0x" + addr, new_name=name, program=program, strict_mode="off")
        if isinstance(res, dict) and (res.get("success") is True or res.get("status") == "success"):
            post("set_comment", address="0x" + addr, comment=f"Lombyte {ev.get('method')}: boot 0x{ev.get('boot_addr', 0):08x}", type="plate", program=program)
            applied += 1
    return applied

have = existing_programs()
for lv in levels:
    name = f"level{lv:02d}.elf"
    if name in have:
        print(f"{name}: already in project, skipping import")
    else:
        t = time.time()
        res = post("import_file", file_path=os.path.join(ROOT, "ghidra/import", name), project_folder="/levels", auto_analyze=True)  # no language: forces the ELF loader; the EE plugin picks r5900 from e_flags
        if not (isinstance(res, dict) and res.get("success")):
            print(f"{name}: import failed: {str(res)[:300]}")
            continue
        while True:
            st = get("analysis_status", program=name)
            if isinstance(st, dict) and not st.get("analyzing", st.get("is_analyzing", False)):
                break
            time.sleep(5)
        print(f"{name}: imported and analyzed in {time.time() - t:.0f}s")
    n = apply_names(name, lv)
    post("save_program", program=name)
    post("close_program", name=name, save=True)
    print(f"{name}: applied {n} names, saved and closed")
print("done")
