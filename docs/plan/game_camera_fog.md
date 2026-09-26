# Game camera, projection and fog (tfrag path)

Boot ELF `SCUS_971.99` addresses (Lombyte names in parentheses where they exist). Implemented in
`crates/rc-engine/src/game_camera.rs` and `crates/rc-engine/assets/shaders/tfrag.wgsl`. A unit test
(`game_camera::tests`) checks that the Bevy clip matrix, the VU pipeline below and `fun_001f2070` agree.

## 1. Functions

| Address | Name | Role |
|---|---|---|
| 0x1f2c60 | `InitViewContext__Fv` | near/far/FOV defaults, screen half sizes, default fog |
| 0x1f2d98 | `UpdateViewContext__Fv` | builds projection `0x18cdc0` (+ variants `0x18ce00`, `0x18ce40`), fog terms, all VU constant blocks (tfrag `0x1de750`), calls `SetTfragDists` |
| 0x1f2588 | `UpdateFog__Fi` | copies level fog (0x15f484..) or alternate fog (0x1610c4.., when `0x1872d4 != 0`) into view context, calls `UpdateViewContext` |
| 0x1e9b10 | `fun_001e9b10` (level init) | reads gameplay-file level settings: bg colour, fog colour, fog near/far distance, near/far intensity → 0x15f484..0x15f494; calls `UpdateFog` |
| 0x1ee4b0 | `fun_001ee4b0` | fog zones: lerps fog from env-transition records at 0x19ae10 (game units ×1024, density d → F = 255 − 255·d) |
| 0x1f2260 | `fun_001f2260` (camera matrices) | view matrix `0x186f40` from camera rotation, `0x186f80` = view·proj, guard-band / clip variants `0x186fc0`, `0x187000`, `0x187040`; called each frame by `DrawDebugProfiler` (frame render, 0x1f39d0) |
| 0x2333a8 | `draw_tfrag` (`DrawTfrag`) | M = Translate(−Camera.pos·1024) · `0x186f80`, uploaded to VU qw 5 and 0x14d |
| 0x233fb0 | `tfrag_proc` (`TfragProc`) | puts the 17-qw block at 0x1de740 in the chain (DMA CNT + `UNPACK V4_32 num=15 → 0x290`, then VIF `MSCAL 0; BASE 0; OFFSET 0x148`) |
| 0x1f2070 | `fun_001f2070` | EE world→screen for one point; same maths (`Q = cf10 / w`, `(x·Q + 2048)·16`) |
| 0x1f3868 | `reset_gs_registers` | FOGCOL (reg 0x3d) = `cf30 | cf34<<8 | cf38<<16` |
| 0x1f33b8 | `fun_001f33b8` | render-to-texture view (far 524288, fog 0..524288 / 255..0); restored by `InitViewContext` |
| 0x1fa378 | `fun_001fa378` | `sceVu0MulMatrix(d, a, b)`: d = b then a (row-vector `b·a`) |

Confidence: high (straight from the decompiler output; constants re-read from instruction bytes).

## 2. Camera

`Camera` at **0x187080** (Lombyte `Camera`): +0x00 position (game units), +0x10 Euler rotation
(x, y, z radians), +0x210 (0x187290) 3×3 rotation rows. `fun_001f2260` uses the rows at 0x187290 when
`0x18c32c == 0` (normal), else builds `RotZ(rz)·RotY(ry)·RotX(rx)` (sce rot matrices, X applied first).
The usual camera update (`fun_001eccd8`) sets row0 = look direction, row1 = −cross(row0, `0x13f5e0`)
(a runtime vector, presumably world up), row2 = cross(row1, row0). **Rows are (forward, left, up)** in the
right-handed Z-up world: the Euler branch at zero angles gives rows = identity (look +X, left +Y, up +Z),
and only that reading gives a non-mirrored image with the view matrix below. (The VU `opmula/opmsub`
operand order in the decompiler output is ambiguous, so the sign of the cross products is inferred from
this, confidence medium-high.)

View matrix `0x186f40` (VU row-vector convention, row k multiplies coordinate k):
camera `x = −left·p`, `y = −up·p`, `z = forward·p`, i.e. x right, **y down** (GS screen Y), z forward.
No scale: camera space is in integer units (game units × 1024), because `DrawTfrag` translates the
integer tfrag positions by `−Camera.pos × 1024`. Confidence: high.

## 3. Projection `0x18cdc0` (rows multiply camera x, y, z, 1)

```
n = 0x18cda0 = 32.0            f = 0x18cda4 = 745472.0      (integer units; 1/32 and 728 game units)
tx = 0x18cdb0 = 0.63 (0x3f2147ae)   ty = tx · 0.775 (NTSC, 0x3f466666; PAL 0.756) = 0.48825
W/2 = 0x18cf00 = 256, H/2 = 0x18cf04 = 208 (from SetupFS_AA_buffer(0x200, 0x1a0, ..) → 512×416)
Zs = 0xcafffbe0 = −8388080.0
row0 = (W/2 / (tx·n), 0, 0, 0)
row1 = (0, H/2 / (ty·n), 0, 0)
row2 = (0, 0, (f+n)/(n(f−n))·Zs, cf10 / n)        ← w column is the FOG slope, not 1
row3 = (0, 0, −2nf/(n(f−n))·Zs, 0)
cf10 = 0x160b30 = (If − In)·n / (Df − Dn)         (see §5)
```
`tx` is the tangent of the horizontal half-FOV: hFOV = 2·atan(0.63) = 64.42°, vFOV = 2·atan(0.48825) =
52.06°. Aspect in pixels 512:416; pixel aspect is non-square (406.35 vs 426.01 px per unit tangent),
the 0.775 factor being the TV correction. Camera code can change `0x18cdb0` (cutscene cameras via
`+0x1c` of their record, `fun_0022eaa8` clamps to a per-mode minimum from 0x1d9b48, 0.63 default).
Confidence: high for the numbers; that gameplay on Novalis keeps 0.63 is inferred (default, not traced).

## 4. VU constants (tfrag block 0x1de750 → VU qw 656..670), per lane

| qw | x | y | z | w |
|---|---|---|---|---|
| 656 | cf10 (Q numerator) | If (fog lower clamp) | In (fog upper clamp) | 3072.0 (ADC add, static) |
| 657 | 0.5 | 1.0 | 2048.0 (UV bias) | 0 (static) |
| 658 | GIF tag: PRE=1, PRIM=0x7c (tri-strip, IIP, TME, **FGE**, ABE), NREG=3, REGS=0x412 (ST, RGBAQ, XYZF2) | | | |
| 659 | same, PRIM=0x7d (fan, fallback clipper) | | | |
| 660 | A+D tag, NLOOP 5 | | | |
| 661 | 2048.0 | 2048.0 | 8388112.0 (0x4afffc20) | cf14 (fog offset) |
| 662 | 1/1024 | 1/832 | 1/Zs | 1/cf10 |
| 663 | 1024 | 832 | Zs | cf10 |
| 664 | 1, 1, 1, 1 (colour multiplier, static) | | | |
| 665 | 4, 4, 1, 1 (`cf08/cf00`, `cf0c/cf04`) | | | |
| 666..669 | LOD morph slopes/intercepts from `SetTfragDists` (0x233068), in fog-w units (`cf20`) | | | |
| 670 | GIF tag with EOP | | | |

Static ELF values at 0x1de750.. are placeholders; the lanes named after view-context globals are rewritten
by `UpdateViewContext`. Confidence: high.

Per vertex (vu1_tfrag_analysis.md §2) with M = T·view·proj, camera depth z (integer units):
`p.w = z·cf10/n`, so `Q = cf10/p.w = n/z` and
```
X = 2048 + 256·x/(0.63·z)          (pixels, 12.4 after ftoi4; XYOFFSET 2048−256 → window 0..512)
Y = 2048 + 208·y/(0.48825·z)       (y down)
Z = 8388112 + Zs·((f+n)/(f−n) − 2fn/((f−n)·z))   → 16776192 at z=n, 32 at z=f; ZBUF PSMZ24, ZTST GEQUAL
F = trunc(clamp(p.w + cf14, If, In))
```
Guard band (qw662): `|x_pix − 2048| ≤ 1024`, `|y_pix − 2048| ≤ 832`, and n ≤ z ≤ f; triangles failing it
are dropped via ADC (L127) or clipped by the fallback program. At z → ∞ Z goes to −696, so geometry just
beyond f would wrap; distance culling (`0x160ec0` = 512000 = 500 game units) keeps it away.

Bevy mapping (`GameProjection`): `clip = (x_v/tx, y_v/ty, (a·d + b)/2^24, d)`, d = −z_view (game
units), with `a = n·A + 8388112`, `b = n·C/1024` computed in f64 (A, C = row2.z, row3.z). Depth = GS Z/2^24,
reverse-Z ordering like the GS. The camera viewport is letterboxed to 512:416; the 1024×832 window is
exactly 2× the draw buffer. Confidence: high (test agrees to <1e-3 px, <4 Z units).

## 5. Fog

Level settings = gameplay file section pointer 0 (wad_layouts_rac1.md §3.3); `fun_001e9b10` reads
`+0x0c/10/14` fog colour (low byte of each s32), `+0x18` Dn, `+0x1c` Df, `+0x20` In, `+0x24` If
(distances in integer units along the view axis, intensities are the GS F value). `UpdateViewContext`:
```
cf10 = (If − In)·n/(Df − Dn)         slope = cf10/n = (If − In)/(Df − Dn)   (0x18cdec)
cf14 = (In·Df − If·Dn)/(Df − Dn)     ⇒ F(z) = In + (If − In)·(z − Dn)/(Df − Dn), clamped to [If, In]
cf20 = (If − In)/((Df − Dn)/1024)    (per game unit; used by SetTfragDists and mobies)
```
F is linear in camera **depth** (not radial distance). GS (FGE=1): `C = FOGCOL + (C − FOGCOL)·F/255`
on RGB after texture modulate, alpha untouched; F interpolated linearly in screen space. FOGCOL =
level fog colour (`reset_gs_registers`). Note the projection itself divides by the fog slope, so a
level with If == In would break the game's Q (none of the 19 levels does).

**Novalis (level 01):** bg (100, 255, 255), FOGCOL (105, 127, 180), Dn 0, Df 245760 (240 game
units), In 255, If 102 → qw656 = (−0.0199219, 102, 255, 3072), qw661.w = 255. Every level's values
are in `extracted/levels/NN/gameplay/level_settings.bin`, loaded by `level_load.rs` from
`gameplay_ntsc.bin`. Confidence: high for formula/values; the blend rounding is medium: the GS manual
formula is `(F·C + (255 − F)·FOGCOL) >> 8`, PCSX2's HW renderer uses `trunc(mix(FOGCOL, C, F/255))`;
the port uses the unquantised `mix(.., F/255)`.

### 5.1 Per-frame fog in the port (2026-09-26)

The fog is no longer fixed at load. `UpdateFog` runs at the end of every frame render and copies the level fog
globals (level01 0x15f444..0x15f454, boot 0x15f484..), or the alternate set while underwater (level01
0x161204..0x161214, boot 0x1610c4..), into the view context; `UpdateViewContext` then recomputes cf10/cf14/cf20
and `SetTfragDists`. The globals themselves change inside fog zones (`fun_001ee4b0`, world_animation.md §5).
Port: `game_camera::GameFog` (resource: view-context `LevelFog`, the `TfragFog` uniform built from it, and
the particle far) is rewritten each frame by `fog_state.rs` and pushed into every material's fog uniform
(binding 2 of tfrag/tie/moby/shrub/billboard), `TieLodState::fog` (per-instance F), the tfrag LOD constants
(`TfragLodUniform::set_fog`, qw666..669 and the w slope) and `particle_render` (500 / 64-unit far). Uniforms
are only written where they differ. The projection is unchanged: the port's clip matrix does not carry the
fog slope in row2.w (§3), so no projection update is needed when the fog changes. `RC_FOG=0` still disables
the blend, `RC_FOG_ZONES=0` keeps the level settings.

## 6. Not implemented / open

- Done: fog zones (`fun_001ee4b0`) and the underwater alternate fog, §5.1.
- LOD morph (qw666..669, in fog-w units) and the ADC guard-band drop / fallback clipper; the GPU clips
  per pixel instead, so edge triangles near the screen border / near plane can differ.
- Exact Z precision: Bevy stores f32 depth, the GS truncates to 24-bit; same ordering, ties may differ.
- XYOFFSET field offsets for interlace/AA (0x13cf60/70) and the AA blit are not modelled.
- Done: vertex colour and F are `@interpolate(linear)` (GS Gouraud is screen-space linear; ST stays
  perspective-correct). PRIM ABE=1 with ALPHA_1 = 0x8000000044 / TEST_1 = 0x5360b is modelled per batch
  (`TfragMaterial::alpha_mode`): opaque where As = 0x80 everywhere, SrcAlpha blend otherwise (Novalis:
  only texture 35). Open: Z write of blended pixels with As >= 0x60 (AFAIL = RGB_ONLY), GS chain draw
  order, and blending in display space instead of linear light.
- Done: background = GS clear colour. Level settings +0x00/04/08 (s32 r, g, b) → `set_background_color`
  (0x1fb280) → RGBAQ of the clear packet at 0x152040 (TEST_1 = ZTST ALWAYS, 16 untextured sprites over
  512×416, Z = 0), emitted first in every frame by `append_gif_transfer_packet` (0x1fb368) unless the sky
  is drawn and its header `clear_screen` (+0x04, zeroed by `update_sky_effects` each frame) is 0: with a
  sky the game does not clear. The port has no sky, so it clears (`main.rs`, `game_camera::level_background`).
