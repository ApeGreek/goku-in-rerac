# Decisions

| Date | Decision | Why |
|---|---|---|
| 2026-09-26 | Write our own extractor from day one; no Wrench runtime dependency | The engine has to read the original formats at runtime anyway. Wrench (GPL-3) is a reference for format knowledge, not a code source. |
| 2026-09-26 | OpenGOAL and Wrench are guiding references only | OpenGOAL's decompiled-game-code + rewritten-runtime split is the architecture; its GOAL-specific tooling does not apply since R&C is C++. |
| 2026-09-26 | ~~C++20, CMake, Ninja~~ **Superseded the same day**: the runtime is a faithful reimplementation in **Rust on Bevy** (wgpu custom render pipelines, WGSL shaders derived from the VU1 programs). Game logic is re-authored in Rust with the decompiled C++ as the behaviour reference, not ported line by line. The C++ extractor (`tools/extract`, `src/core`) stays as a verified oracle for tests while the Rust loaders are written, then retires. | User chose the reimplementation route after weighing it against a bit-exact port (see the SK8 and iw4L precedents). Consequence: identical behaviour becomes a verification target reached through PCSX2 trace diffing rather than a property inherited from ported code. |
| 2026-09-26 | Disc image lives outside the repo (`~/PS2/ratchet1/`), path via `RC_ISO` | Nothing copyrighted can be committed by accident. All derived data goes to git-ignored `extracted/`. |
| 2026-09-26 | Fixed-width types `u8..u64, s8..s64, f32` in `src/core/types.h` | Ported structs must match EE sizes and alignment byte for byte so on-disc data loads directly. |
| 2026-09-26 | Disc is NTSC-U `SCUS_971.99`, SYSTEM.CNF VER 1.00, VMODE NTSC (60 Hz). Boot ELF sha1 `72fd1de379dafd8f243dceeb3be50980f3fd3cd5`, 1,383,028 bytes, plain unpacked ELF. Filesystem holds only SYSTEM.CNF, the ELF and IOPRP243.IMG; all game data is sector-addressed through the TOC in the ELF. | Lombyte targets this exact executable; rac1-decomp targets PAL v2.00 and its addresses will need translating. |
| 2026-09-26 | Full ISO sha256 `ab849fe7cc9cc81c487d61b0d3ea15b5849943481b6a6ebf4d9aa9cf7bc40d9d` (4,214,095,872 bytes), identical to the dump Lombyte recorded. Lombyte's `config/overlays/us/level-table.json` (19 levels, absolute byte offsets for level/audio/scene WADs) and its per-level overlay maps are therefore valid cross-checks for our extractor. | Independent ground truth for the TOC parser and overlay extraction. |
| 2026-09-27 | ~~EE float exactness policy~~ **Resolved** by "Native-first fidelity policy (2026-09-27)" below: no new PS2 float modelling; the existing model (`tfrag_light::ps2`, `rc_game::ps2v::Pf`) stays for PCSX2 diagnosis and is replaced last in Phase 2. | Was pending until the PCSX2 harness showed where host IEEE floats diverge; the user decided the question on behaviour grounds instead. |
| 2026-09-26 | GPU backend: wgpu via Bevy (Metal on macOS, Vulkan/DX12 elsewhere) | Follows from the Rust/Bevy decision. |
| 2026-09-27 | **Native-first fidelity policy.** The port must *behave* like the original (what the player sees, hears and feels) using our own native systems. Bit-exactness is a diagnostic tool, not the goal. Phase 1 (in force now): no new hardware modelling; reproduce noticeable hardware effects by their result, natively; existing layers stay for diagnosis; PCSX2 mismatches are triaged. Phase 2 (later): strict native pass over `hardware_fidelity_layers.md`. Details in the section below. | Hardware modelling costs code, tests and time and ties the port to PS2 arithmetic the player cannot perceive. What matters is behaviour; PCSX2 bit comparisons remain useful to find misunderstandings of the game. |

## Native-first fidelity policy (2026-09-27)

Decided by the user on 2026-09-27. Resolves the former pending "EE float exactness policy" row.

**Goal.** The port must behave like the original: what the player sees, hears and feels. It gets there with our own
native systems. Bit-exactness is a diagnostic tool, not the goal.

**Phase 1: now (in force immediately).**

- No new hardware modelling. New code uses standard IEEE floats and native Bevy mechanisms by default.
- When a hardware difference would be noticeable in play, reproduce the *result* natively, not the mechanism.
  Example: on the PS2 the water ripple steps every 9 ticks because of float truncation. The native fix is "step every
  9 ticks", not a model of the PS2 float unit. Each such reproduction is recorded in `docs/plan/hardware_fidelity_layers.md`.
- The existing hardware-fidelity layers (catalogued in `docs/plan/hardware_fidelity_layers.md`) stay for now. They serve
  diagnosis (bit-exact PCSX2 comparison).
- PCSX2 comparison mismatches are triaged into two kinds (procedure in `docs/plan/trace_harness.md`, "Triage"):
  1. **Misunderstandings of the game**: wrong formula, constant, field, operation order or timing. Fix natively.
  2. **Pure hardware-arithmetic differences**: last-bit float rounding, GS blend byte differences, sound-chip
     interpolation. Accept with a documented tolerance (`hardware_fidelity_layers.md`, "Tolerances"), unless the difference
     is noticeable in play; then reproduce the result natively.

**Phase 2: later, strict native pass.**

- Replace each layer in `hardware_fidelity_layers.md` in its "Suggested order" section, keeping only behaviour-level fixes
  where a difference is noticeable in play.
- The PS2 float model (A1, with B5) goes last, after the PCSX2 checks have served their purpose.
