# VU1 microprograms in the boot ELF

The boot ELF `SCUS_971.99` carries the VU1 microcode as 43 `.DVP.overlay`
sections (SN Systems' DVP overlay convention), described by `.DVP.ovlytab`
(43 × 12-byte entries) and `.DVP.ovlystrtab`. The section payloads live in
`.vutext` (EE address 0x100080, 0x12290 bytes).

Section name pattern: `.DVP.overlay..<vu_offset>.<program_id>.<line>.<chunk>`.
`ovlytab` entry: `{ u32 name_offset_in_strtab; u32 ee_address; u32 vu1_address; }`.

Each program is split into 0x800-byte chunks, one section per chunk, uploaded
to consecutive VU1 micro-memory addresses. Grouping by `program_id` gives the
12 distinct microprograms. Chunk counts are a rough size measure (VU1 micro
memory is 16 KiB, so 8 chunks is essentially a full-size program).

| program_id | chunks | VU1 range | EE address of chunk 0 | Renderer (from the EE code, see docs/plan/render_pipeline.md) |
|---|---|---|---|---|
| 104691 | 2 | 0x0000–0x0A3F | 0x00100090 | **VU0** program loaded by `DrawMobysSetup` (moby-side VU0 helper) |
| 436083 | 1 | 0x0000–0x057F | 0x00100af0 | **VU0** program loaded at end of frame / transitions |
| 221571 | 1 | 0x0000–0x06C7 | 0x00101088 | VU1 **particles** (`PartProc`) |
| 56467 | 2 | 0x0000–0x0B5F | 0x00101768 | VU1 **shrub** (`ShrubProc`, first list) |
| 912339 | 3 | 0x0000–0x1267 | 0x001022e8 | VU1 **shrub** (`ShrubProc`, second list; likely "Shrub Near") |
| 55907 | 8 | 0x0000–0x3A5F | 0x00103578 | VU1 **tfrag** main (`TfragProc`) |
| 903379 | 4 | 0x0000–0x1DCF | 0x00107028 | VU1 **tfrag** second/fallback strip list |
| 13507 | 4 | 0x0000–0x1EC7 | 0x00108e28 | VU1 **tie** (`TieProc`) |
| 224979 | 7 | 0x0000–0x375F | 0x0010ad28 | VU1 **tie** (`TieProc`, second program) |
| 28259 | 2 | 0x0C80–0x0F6F, 0x0000–0x002F | 0x0010e4d0 | **VU0** patch loaded once in `InitOnce` |
| 57843 | 3 | 0x0000–0x125F | 0x0010e818 | VU1 **textured sprite / billboard** program (resident id 7) |
| 13859 | 6 | 0x0000–0x283F | 0x0010faa8 | VU1 **moby** renderer (resident id 6) |

Assignments recovered from the EE code on 2026-09-26: VU0 programs go through `VU0_loadMicroProgram` (VIF0 DMA), VU1 programs are spliced into the VIF1 chain as REF tags pointing at `chunk0_addr - 0x10`. The engine caches the resident VU1 program id in a global and re-adds the program only when it changes.

Level overlays may carry additional VU code in their `.data`; not yet checked.

## Disassembly

The listings the format docs cite by line number, `work/vu/<id>.txt` (`lower | upper` per line, labels `Ln:` at
branch targets; images in `work/vu/<id>.bin`), were written by the C++-era tools `tools/vu/extract_vu.py` and
`rc_vudis` (a vendored copy of OpenGOAL's VU disassembler). Both were removed with the C++ reference on 2026-09-27;
the listings were copied from the development `extracted/vu/` into `work/vu/` (git-ignored reference data, kept by
hand) in the repo reorg of 2026-09-27. All 12 programs disassembled completely. A VU disassembler is still needed as a
proper dev tool (no Rust one yet; a follow-up in docs/plan/repo_reorg.md) to regenerate them.

Upstream OpenGOAL's decode tables lack five instructions that R&C uses; the removed vendored copy had them added (a
new tool needs them too):

| instruction | table / encoding | used by |
|---|---|---|
| `ITOF4` | upper special, op11 `00100'1111'01` | 13859 (moby clipper: converts `ftoi4` screen XYZ back to float) |
| `SUBA` | upper special, op11 `01011'1111'00` | 13859 (per-plane clip-distance routines) |
| `MADDi` | upper op6 `100011` | 28259 (sine polynomial) |
| `MADDAi` | upper special, op11 `01000'1111'11` | 28259 (sine polynomial) |
| `FMOR` | lower op7 `0011011` | 28259 (reads MAC zero flags to skip zero Euler angles) |

28259's image spans 0x0000–0x0F6F with a zero-filled gap: instructions 6–399
(`lq. vf00, 0(vi00) | addx. vf00, vf00, vf00`) are padding, not code. The kind
argument (`vu0`/`vu1`) did not change the output.
