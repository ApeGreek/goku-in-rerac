# What we take from OpenGOAL (jak-project)

Checkout: `~/Globals/jak-project`. Read `docs/project-overview.md` and
`docs/progress-notes/` there for detail. This file records the patterns we
adopt and the ones we deliberately do differently.

> Written for the C++ plan that was superseded by Rust on Bevy the same day (decisions.md, 2026-09-26). The patterns
> still apply; the C++ destinations named below (`src/...`, `third_party/`, `vendor.yaml`) were never built or were
> removed when the C++ reference extractor was retired (2026-09-27).

## Adopt

| Pattern | Where in jak-project | Our equivalent |
|---|---|---|
| Extractor identifies the build by boot-ELF filename (serial) + XXH64 hash, looked up in a small database with expected file counts and flags; prints a ready-to-paste DB row for unknown builds | `decompiler/extractor/extractor_util.cpp` (`findElfFile`, `validate`) | `tools/extract` `info` stage: same serial+hash table in `src/core/build_db.*` |
| User data lives in an ignored `iso_data/` folder; human-readable dumps in `decompiler_out/`, runtime-packed assets in `out/` | top-level `.gitignore` | `extracted/` (raw lumps, human-inspectable), `assets/` (runtime-packed), both ignored |
| Game code keeps building real PS2 DMA chains; the renderer walks the chain and dispatches per bucket to a renderer class | `game/graphics/opengl_renderer/OpenGLRenderer.cpp`, `common/dma/dma.h` | Same. Game code produces the same VIF/GIF packets the PS2 would; our renderer consumes them. This is what keeps rendering identical rather than "similar". |
| One renderer class per VU1 microprogram; shader re-derives the VU1 math; PS2 constant blocks kept as structs with `static_assert(sizeof)` | `background/TFragment.h` (`TFragData`), `shaders/*.vert` | `src/engine/render/<program>.{h,cpp}` + shader per R&C VU1 program (moby, tie, tfrag, shrub, sky, hud, particles) |
| When a VU program is too irregular for a shader, transliterate it instruction by instruction against a small software-VU helper, with the original asm in comments | `Shadow_PS2.cpp`, `ocean/OceanMid_PS2.cpp`, `game/common/vu.h` | `src/engine/vu/` software-VU helpers, used only where needed |
| Leftover EE asm that cannot be decompiled yet is mechanically translated to C++ over an `ExecutionContext` with `u128 gprs[32]` | `game/mips2c/` | `src/game/mips2c/` as an escape hatch, to be removed function by function |
| GS ALPHA register tuple pattern-matched onto host blend functions; unsupported combinations assert loudly instead of approximating silently | `DirectRenderer.cpp` `update_gl_blend` | Same policy: assert on unknown GS state |
| Faithful reimplementation of the IOP driver (file streaming, VAG streams, RPC) as C++ with differences commented | `game/overlord/` | `src/engine/iop/` for R&C's IOP modules |
| Per-file divergence log | `docs/progress-notes/jak1/code_status.md` | `docs/plan/divergences.md` |
| `vendor.yaml` pinning every vendored third-party lib with tag, sha and licence | `vendor.yaml` | same file at repo root |
| `u128` union, `Vf` 16-byte aligned vector, `sse2neon` shim so SSE intrinsics build on Apple Silicon | `common/common_types.h`, `game/common/vu.h`, `common/util/simd_util.h` | `src/core/u128.h`, `src/core/simd.h` |

## Do differently

* **Float exactness.** OpenGOAL uses host IEEE floats everywhere and only
  reproduces VU min/max bit tricks. Our goal is identical behaviour, so the
  PCSX2 trace harness decides per subsystem whether EE-exact float helpers are
  needed. Decision recorded in `docs/plan/decisions.md` when made.
* **Frame rate.** OpenGOAL exposes `target_fps`. We simulate at the disc's
  native rate only. Rendering may run faster; simulation does not.
* **Graphics API.** OpenGOAL is OpenGL-only. We target Metal on macOS via SDL3.
* **No GOAL layer.** The game is C++; ported code is compiled directly into the
  runtime, no interpreter, no dynamic linker, no listener. Level overlays become
  ordinary compiled code selected by level id.

## Libraries we will likely take (all permissively licensed)

SDL3, fmt, nlohmann/json, CLI11, xxhash, stb_image, imgui (debug UI),
googletest. Vendored under `third_party/` with a `vendor.yaml`.
