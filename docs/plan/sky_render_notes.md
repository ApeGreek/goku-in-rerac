# Sky renderer notes (from the decomp)

Boot = `SCUS_971.99`; level01 = `/levels/level01.elf` (full export). The shell draw code is engine code,
byte-identical in every overlay ("Lombyte exact-boot-match"); the per-frame *dispatch* (which matrix each
shell gets) is level code. Loader: `crates/rc-formats/src/sky.rs`; format: docs/formats/shrub_sky_rac1.md part 2.

## 1. Not a VU1 renderer
The sky has no VU1 microprogram. The EE transforms vertices with VU0 macro code and sends finished GIF
packets through VIF1 DIRECT (PATH2). Functions (boot / level01):

| Boot | level01 | Role |
|---|---|---|
| 0x2028e0 | – | loader: relocates offsets, sets header+4 (`clear_screen`) = 1, rewrites texture defs (TEX0 cache, offsets >> 4, log2 w/h) |
| 0x1e9ab8 `transition_draw_sky` / (frame render 0x1f39d0) | 0x252570 `DrawSky` (from `DrawWorld` 0x21a1b8, Lombyte `DrawDebugProfiler`) | `SetupSkyGifPaging`, per-shell loop, `DoSkyGifPaging`, then TEST_1 = 0x5360b, ZBUF_1 = ZBP, ZMSK 0 |
| 0x22ae70 `update_sky_effects` | 0x29ee10 (animated) / 0x29edb0 (static) — not NTSC/PAL (§6) | per-shell matrix + `SkyDrawShell` |
| 0x22b690 `sky_draw_shell` | 0x29f668 | `flags == 0` → textured, else gouraud |
| 0x22b6e8 / 0x22b928 | 0x29f6c0 / 0x29f900 | textured / gouraud shell: cluster cull, double-buffered SPR DMA of `data_size` bytes |
| 0x22c4c8 | 0x2a00b0 `SkyBsphereCheck` | per-cluster bounding-sphere cull |
| 0x22bf94 | (boot-match) | vertex transform + clip flags into scratchpad 0x70002000 |
| 0x22c208 / 0x22c0e0 | (boot-match) | textured / gouraud triangle-list GIF builder |
| 0x22bba0 `sky_sprite_proc` | – | star sprites (levels with `maximum_sprite_count > 0`: 0,2,5,6,7,13,15,17), particle VU1 program 0x101080 |

## 2. Draw order and frame clear — confidence high
* Frame render: `append_gif_transfer_packet` (full-screen clear, Z = 0) **only if** no sky, or header+4 != 0
  (or sky disabled). Then `ResetGsRegisters`, **sky**, tfrag, tie, shrub, moby, particles.
* Shells are drawn in index order 0..shell_count-1 (painter's order), clusters in order, faces in order.
* `clear_screen` disc value is dead: loader sets it to 1 (so frame 1 clears); level01's animated dispatch,
  the boot `update_sky_effects` and boot 0x22b288 set it to 0 every frame, so in steady state **the game
  does not clear**. Shell 0 of most levels is a gouraud dome that covers the screen and writes Z = 0 — it
  is the colour and depth clear.
* Per-level survey (levels 00–18, confidence high): the dispatch zeroes +4 every frame on every level except
  **05, 07, 10, 13, 14, 15**, which therefore clear every frame (05, 07, 10, 15 are the levels with no gouraud
  shell). All others clear only on the first frame after load.

## 3. GS state per shell — confidence high (static A+D blocks re-read from the ELF)
Each shell first sends a 7-qw A+D block (REF 0x13d0f0 gouraud / 0x13d160 textured; ZBUF entries patched
with the real ZBP by `set_pal_mode` 0x1f34e8):

| Reg | Gouraud (0x13d0f0) | Textured (0x13d160) |
|---|---|---|
| ZBUF_1 | ZMSK 0 (**writes Z**) | ZMSK 1 (no Z write) |
| TEST_1 | 0x30000: no alpha test, ZTST ALWAYS | 0x3180b: ATE, GEQUAL AREF 0x80, AFAIL FB_ONLY; ZTST ALWAYS |
| ALPHA_1 | 0x8000000044: (Cs−Cd)·As + Cd | 0x2000000044: same equation |
| PRIM | 0x4b: triangle, Gouraud, ABE, no TME, **FGE 0** | 0x5b: triangle, Gouraud, TME, ABE, **FGE 0** |
| TEX1 / CLAMP | bilinear mag/min, no mips; CLAMP both axes | same |

After a gouraud shell: TEST_1 = 0x3180b, ZBUF_1 ZMSK 1. After the sky: TEST_1 = 0x5360b, ZMSK 0.
So: **no depth test, no fog, standard alpha blending**. The textured alpha test is inert: failing pixels
(A < 0x80) still write colour (AFAIL FB_ONLY) and Z is masked anyway. Every vertex has GS Z = 0 (far plane for GEQUAL),
so later world geometry always passes the depth test over the sky.

## 4. Vertex stream — confidence high (golden-tested: `sky_gs_vertices`)
Per face: three vertices in stored index order, triangle list, no back-face culling. Textured: ST =
u16/4096 (Q = 1), RGBAQ = (0x80, 0x80, 0x80, low byte of vertex alpha), TEX0 (PSMT8, TCC 1, MODULATE,
CT32 CLUT CSM1) re-sent when the face texture changes. Gouraud: RGBAQ = the vertex's attribute word
(per-vertex RGBA; header colour unused). A face is dropped when all three vertices share an outside clip
flag (mask 0x2f); clusters are culled by bounding sphere — GPU clipping makes both unnecessary.

## 5. Matrix, projection, depth — confidence high
`0x22bf94`: `p = (x, y, z, 1)·SkyM·C`, SkyM = 4×4 at boot 0x1d96e0 (level01 0x1bdc60), C = boot 0x187040,
one of the view·projection variants built by `fun_001f2260` from the **camera rotation only** (see
game_camera_fog.md; camera translation is applied separately by each world renderer, not here). SkyM is
identity (translation row 0) in the normal paths ⇒ the sky is centred on the camera and never translates.
Then XY = (p.xy/p.w)·(0x18ce90·¼) + 0x18cea0 in 12.4; Z is not used (sent as 0). The raw s16 vertex
integers are used unscaled; only direction matters. FOV = the world camera's (same C). 

**0x187040, re-derived.** `fun_001f2260`: 0x186f40 = rotation-only view (columns −left, −up, forward from
the camera rows; translation row 0, w = 1). `UpdateViewContext` 0x1f2d98: 0x18ce00 = the world projection
0x18cdc0 with the w column replaced by 1/n (w = z/n instead of the fog slope); 0x18ce40 = 0x18ce00 with
row 0 / cf08 (1024), row 1 / cf0c (832), rows 2/3 z lane / Zs. Then 0x187040 rows 0, 1 = 0x18ce40 rows ×
cec0 (= cf08/cf00 = 4), rows 2, 3 copied, and finally 0x187040 = 0x186f40 · 0x187040 (`fun_001fa378`,
row-vector). So C = V_rot · P' with x' = x/(t·n), y' = y/(t_y·n), z' = z(f+n)/(n(f−n)) − 2f/(f−n),
w' = z/n. The transform scales vf26 = 0x18ce90 = (1024, 832, Zs, cf10) by ¼ on x/y only and adds
0x18cea0 = (2048, 2048, …): X = 2048 + 256·x/(t·z), Y = 2048 + 208·y/(t_y·z) — **exactly the world
projection's screen mapping**, same t = 0.63, t_y = t·0.775 (NTSC), n = 32, f = 745472, no extra scale.

**Z = 0 confirmed.** The transform does compute a Z lane (`vftoi4.xyz`), but both GIF builders drop it:
gouraud 0x22c0e0 and textured 0x22c208 store XYZ2 as `x & 0xffff | (y & 0xffff) << 16` in the low word and
zero in the high word (`sw zero,0x14(t8)` / `sw zero,0x24(t8)`). Textured RGBAQ = 0x808080 | alpha << 24
with Q = 1.0 (`lui 0x3f80`), so the texture mapping is affine (S/Q with Q constant).

## 6. Per-shell rotation (level code) — confidence high (levels 00–18 surveyed)
No rotation data in the file. level01 0x252570: `if (*(s32*)0x15ed84 == 1)` animated 0x29ee10 else static
0x29edb0 (all identity). The test is on the level index 0x15ed84, not on the PAL flag 0x15ed80 (written 0/1 by
`SetTimeBase` 0x276258 = boot `set_time_base` 0x214970), so the two loops are animated/static, not NTSC/PAL as
render_pipeline.md and the old Ghidra names `DrawSkyShells_PAL`/`_NTSC` have it. Animated: shells 0, 1 identity; shells 2, 3, 4 get SkyM = Euler(0, 0, θ) (rotation
about Z-up, `fun_001fa070`) with
* shell 2: θ = (gp-counter `uGpffff89cc` & 0x3ffff)·2π/262144 − π (one turn per 262144 ticks)
* shell 3: θ = (`DAT_0015f5cc` & 0x1ffff)·2π/131072 − π
* shell 4: θ = (`uGpffff89cc` & 0xffff)·2π/65536 − π
0x15ed84 is the current level index (help bitmask `1 << level`, per-level tables), so the animated path is
simply "level == 1". Both counter names are the same address 0x15f5cc (gp = 0x166c00): a 60 Hz game-logic
tick counter, +1 per logic tick in gameplay modes (frozen in pause/freeze modes; at 30 fps render the main
loop runs a catch-up tick), reset to 0 at level init plus one +1 at level start. θ rises 2π/P per tick.

Euler builder boot 0x1fa070: M = Rz·Ry·Rx in row-vector form (v' = v·M), translation 0; Rz rows (c, s, 0),
(−s, c, 0), (0, 0, 1) — counter-clockwise about world +Z (up); Ry rows (c, 0, −s), (0, 1, 0), (s, 0, c).

Per-level rotating shells (θ(P) = (c mod P)·2π/P − π, "& mask" = mod mask+1; unlisted shells identity;
shells ≥ the header count skipped), survey of levels 00–18:

| Level | Rotations | Stars (sky_sprite_proc) |
|---|---|---|
| 00 | s3 = θ(0x8000) | before s2: 244 twinkle + 12 moving |
| 01 Novalis | s2 = θ(0x40000), s3 = θ(0x20000), s4 = θ(0x10000) | – |
| 02 | – | 246 twinkle + 10 moving |
| 03, 04 | s1 = θ(0x10000), s2 = θ(0x20000) | – |
| 05 | s1 = θ(50000) | 120 + 8 |
| 06 | – | 234 twinkle, 6 fixed, 16 moving |
| 07 | s2 = θ(50000) | yes |
| 08 | s1 = θ(40000) | – |
| 09 | s3: Euler (0, 0.3, θ(25000)) → M = Rz(θ)·Ry(0.3) | – |
| 10 | s0 = θ over 2c, P 2^17; s1 = 5c, P 2^17; s2 = 10c, P 2^18; s3 = 10c, P 2^17 | – |
| 11, 12, 16 | s1 = θ(0x40000), s2 = θ(0x20000), s3 = θ(0x10000) | – |
| 13, 15 | – | yes |
| 14 | s1 = θ(50000), s2 = θ(100000) | – |
| 17 | – | 240 + 16 |
| 18 | s4 = θ(40000), s5 = θ(60000) | – |

Stars: the file's `sprite_count` is 0; stars are generated with `rand()` on the first frame, drawn with the
particle VU1 program between the listed shells, and ALPHA_1 = 0x8000000044 is written inside the star step.

Boot 0x22b288 /
level01 0x29f260 (called from 0x2a3b90, not the frame render) is a variant: shell i gets extra Euler
offsets and uniform scale 1, 1, 1.25, 1.5, 1.75, 2.0 plus translation from 0x160520.
The boot `update_sky_effects` (template for star levels): shells 0, 1 identity; star sprites
(0x100 records of 0x20 bytes at `sprites`, radius 50, twinkle colours, `sky_sprite_proc`); ALPHA_1 =
0x8000000044; shells 2, 3 identity.

## 7. Textures — confidence high
Paged into GS memory per frame (`DoSkyGifPaging`, list at boot 0x18d040); TEX0.TBP/CBP from 0x15ee74.
Decode = level-texture rules (CSM1 swizzle, alpha 0x80 → 1.0). Sampler: bilinear, clamp, no mips.
FX textures (first `fx_count`) are the star sprite textures.

## Open
Star sprite record layout and generation (levels with stars); the `sqrt(1 + t²)` identity behind the
cluster-cull constants (medium; irrelevant to the port, which lets the GPU clip).

## In the port (2026-09-26)
`crates/rc-engine/src/sky_render.rs` + `assets/shaders/sky.wgsl`; loaded by `level_load` (`LevelSky`).
* **Camera.** A second `Camera3d` (`SkyCamera`, order −1, render layer 1) with the world `GameProjection`
  and the main camera's transform copied each frame; the vertex shader transforms directions (w = 0) through
  the mesh rotation (SkyM) and `view_from_world`, so the view has no translation, then `clip_from_view`:
  identical to C = V_rot·P' above. clip.z = 0 → depth 0 = GS Z 0 (the far end; world geometry always passes).
* **Order.** One draw per run of consecutive faces with one texture in (shell, cluster, face) order (Novalis:
  5 draws, 1024 triangles). All draws are `AlphaMode::Blend` (Transparent3d); `depth_bias` = draw index ×
  10^6 overrides Bevy's back-to-front sort so the draws run in the game's order.
* **GS state.** `specialize`: depth compare Always for all; depth writes for gouraud only; no culling.
  Gouraud: RGB = vertex bytes / 255 (no TME: the GS writes Cf as is), As = vertex alpha. Textured:
  MODULATE with Cf = 0x80 (Cv = Ct), As = (At·a) >> 7, raw texel alpha restored from the decoder's scaling.
  Blend SrcAlpha / OneMinusSrcAlpha; no fog; RGBA and ST `@interpolate(linear)` (Q = 1 → affine). Sampler
  bilinear, clamp, no mips.
* **Clear.** The sky camera clears colour to the level background (`ClearColor`) and depth to 0 on the first
  frame; from frame 2 on it stops clearing colour on levels that zero +4 (all but 05/07/10/13/14/15), so the
  gouraud dome is the clear. It keeps clearing depth to 0, which equals the dome's Z = 0 writes wherever the
  dome covers (everywhere on Novalis). The main camera never clears: `ClearColorConfig::None` and
  `Camera3dDepthLoadOp::Load` (Bevy shares one depth texture per target + MSAA), so the world pass draws over
  the sky with the sky's depth, like the rest of the GS chain.
* **Rotation.** `level_rotations` (the table above) with c = 1 + ⌊60·t⌋ from app start (`RC_SKY_ROT=0`
  freezes at c = 1; any other number is a speed multiplier for checking). Novalis: shells 2, 3, 4 turn once per
  72.8, 36.4 and 18.2 minutes.
* **Novalis.** Shell 0 is a gouraud dome (670 vertices, all alpha 0x80), shells 1–4 textured (4 textures,
  512×64/128, texel alpha ≤ 0x80, vertex alpha 0x80), no stars, clears on the first frame only.
* **Known differences.** Blending happens in linear light (sRGB target) instead of on the display bytes, so
  fractional As (cloud edges) mixes slightly differently; As > 0x80 would clamp (none on Novalis); the GPU
  clips triangles at the camera plane where the GS would rasterise the EE's projected values unclipped (the
  EE only drops faces whose three vertices share an outside flag); per-cluster sphere culling skipped
  (no visible effect). Not ported: star sprites.

## Stars in the port (2026-09-26)

Code: `crates/rc-game/src/sky_stars.rs` (generation + per-frame update, pure, unit-tested),
`crates/rc-engine/src/sky_stars.rs` + `assets/shaders/sky_stars.wgsl` (`SkySpriteProc` and the draw). Read from the
disassembly of the level00 overlay's per-level dispatch copies (0x289330 L00, 0x2895a0 L02, 0x289bd8 L05, 0x289cb8 L06,
0x28a208 L07, 0x28a920 L13, 0x28aa98 L15, 0x28ac88 L17; generic generator/update 0x288ec0/0x289108, `SkySpriteProc`
0x28bd58). Each star level's own overlay copy (02 0x28a448, 05 0x2b3760, 06 0x297d20, 07 0x2b1d60, 13 0x2954d0,
15 0x279000, 17 0x283008; generators 05 0x2b32e8/0x2b3530, 07 0x2b18e0/0x2b1b28, 13 0x295040/0x295288, 15
0x278b68/0x278db0) matches instruction for instruction after normalising relocated addresses. Confidence high.

* **Dispatch.** Star step position (the stars never use SkyM, so they do not turn with a shell):
  00 before s2 (s0, s1, stars, s2, rotating s3, s4); 02 before s2; 05 before s1; 06 before s1; 07 before s1;
  **13 before s2** (s0, s1, stars, s2 — not after s0); 15 before s1; 17 before s1. Counts: 00 244 + 12; 05, 07, 13,
  15 120 + 8; 02 246 + 10; 06 234 + 6 fixed + 16; 17 240 + 16. The step runs while header +8 == 0 (first frame after
  load; the sky scratch is all zero on the disc, checked for 00/05/06), writes +8, then updates in the same frame.
* **Generation / update**: see the module doc of `rc_game::sky_stars` (rand order per record, formulas). Summary:
  twinklers sit at radius ≤ 50 on a dome z = 36·cos²b + 16 (00/02/05/07/13/15) or on the full radius-50 sphere
  (06/17); size (32..79)/256; base colour 0x50 grey tinted +g on blue or on red|green (red gets `0x50 | g`, an OR),
  alpha 0x30..0x4f; every frame RGB = base − 0x20 + 4·(5-bit fields of `rand() >> 16`) — per-frame noise, not a
  periodic twinkle. Moving stars: two u16 angle counters +1 per frame (one lap per 4096 frames, |cos| keeps them
  in the upper hemisphere), red 0xf0/0x20/0x20 with alpha 0x70 for 8 of every 64 frames else 0x20 (06/17: colour
  from overlay 0x160500, alpha 0x70/0x24). Level 06 fixed stars: 3 positions (overlay 0x1bdf30, xyz + size 0.35)
  × 2 sprites half a period apart, colour 0x1604f0 (0x70 0x60 0x38), alpha a 48-frame triangle 0 → 0x60 → 0,
  rotation re-randomised within ±8/256 turn of 0 or ½ turn each time the phase wraps.
* **Draw.** Per record in index order: skip if texture byte < 0; c = R·pos with the rotation-only view; frustum
  test `tan·c.z − (|c.xy| − size·sec) ≥ 0` (no near/far, no fade); half-diagonal **Q = size·832/c.z** pixels (the
  w lane of 0x16cc40 = (256, 208, 1024, 832)); the particle VU1 kind-0 sprite (1.0625 y factor, rotation +8 in
  radians via VU0 sin/cos); texture = sky texture byte 2 (header +0x10, the 32×32 textures 0..3), MODULATE,
  bilinear, clamp. **ALPHA_1 = 0x48 (additive) on every star** — the record's byte 3; the 0x8000000044 written after
  the step only restores the shell blend. TEST_1/ZBUF_1 are what the previous shell left (ZTST ALWAYS, ZMSK 1): no
  depth test or write; the GS Z the VU computes is unused.
* **Schedule and RNG.** The dispatch runs in the frame render after the frame's game tick. The port runs one star
  frame per 60 Hz `FixedUpdate` tick after the particle tick, on `ParticleSim::rng` (the shared stream). At 30 fps the
  game's catch-up tick would skip a star frame (not modelled). No `srand` in any star code (see particles.md §5).
* **Bevy.** `spawn_sky` reserves a draw-order slot (`SkyStarOrder`) at the dispatch's position; one mesh per star
  texture on the sky layer, `depth_bias` = slot × 10^6 (+ group), blend `One, OneMinusSrcAlpha` with alpha 0
  (additive, order-independent within the step), depth compare Always, no depth write, no fog.
  `RC_SKY_STARS=0` disables the step; `RC_STAR_STATS=1` prints the rng state and a hash of the records per second.
* **Checked on screen** (`RC_SCREENSHOT_FRAME` 120/180): level 00 `RC_CAM=154,184,30,154,214,80` — 52 sprites pass the
  cull, ~20 visible over the shell's own texture stars (the rest behind geometry), coloured points plus a red moving
  light; level 06 `RC_CAM=234,266,230,246,293,244` — the (12, 27, 8) fixed star is an orange four-point flare at
  frame 120 (one sprite at alpha 0x60) and an eight-point flare at 180 (both sprites at 0x30, different rotations);
  level 05 looking up — sprites under the rotating cloud shell s1; Novalis — no stars. Two runs of the same frame
  are pixel-identical. Not checked against PCSX2.
* **Differences.** Blending in linear light (sRGB target) instead of on display bytes; VU0 `vdiv` precision and the
  camera matrix come from Bevy f32; sec = √(1 + tan²) instead of the game's `atan`/`fast_cos` value.
