# PSS movies (`mpegs[88]`, `global/mpegs/NNN.bin`)

The game's FMVs. Sony's "PSS" is an MPEG-2 program stream with the audio as SPU ADPCM in private stream 1. The
engine plays them natively from these files (decision U10, `docs/plan/decisions.md`); playback in the game and in the
port: `docs/plan/cutscenes.md` §5. Readers: `rc_formats::pss` (demuxer, audio), `rc_video` (MPEG-2 video, colour).
Confidence: **[H]** checked on all 84 files, **[M]** read in the code only, **[L]** inferred.

## 1. Files [H]

`mpegs[88]` in the TOC (0x17f8, `{lsn, bytes}`); the extractor archives each non-empty entry raw as
`global/mpegs/NNN.bin` (84 files, 2.7 GiB; 002, 021, 074, 079 are empty entries). Which caller plays which index:
`docs/plan/cutscenes_transitions.md` §5 (in-level n = `2 + n` NTSC / `21 + n` PAL; transitions 40–50 / 52–62; extras
70–74 / 75–79; title attract 80–83 / 84–87; 0, 1, 64–69 unknown).

## 2. Program stream [H]

| unit | bytes | notes |
|---|---|---|
| pack header `00 00 01 BA` | 14 + stuffing (byte 13 & 7) | MPEG-2 form (`01` marker bits); the files are packed in 2 KiB sectors |
| system header `00 00 01 BB` | 6 + length | first pack only |
| video PES `00 00 01 E0` | 6 + length | MPEG-2 PES header (`10` marker, flags, header length; PTS / PTS + DTS on picture starts); payload = the video elementary stream |
| audio PES `00 00 01 BD` (private stream 1) | 6 + length | PES header (PTS), then the 4-byte sub-stream header **`FF A1 00 cc`**, cc = audio channel, then audio bytes |
| padding `00 00 01 BE` | 6 + length | |
| end code `00 00 01 B9` | 4 | then zero padding to the sector end (< 2 KiB) |

**Audio channels = languages.** `StartPssMovie(lsn, size, 0x15ed88)` passes the game language to
`video_dec_set_stream(…, 3 = sceMpegStrADPCM, language, pcmCallback)` (`initAll` 0x31a668); `pcmCallback` 0x31b5d0
skips the 4-byte sub-stream header. The files carry channel 0 only (0, 1, 70–87) or channels 0, 2, 3, 4, 5 (English,
French, German, Spanish, Italian; 3–69). The port plays the language's channel, or channel 0 when it is missing [L:
the game would then get no audio and, since `readMpeg` waits for audio before starting the display, never show the
picture; English is the default].

## 3. Audio: `SShd` + `SSbd` + interleaved SPU ADPCM [H]

Each channel's bytes (concatenated in file order) start with a 0x28-byte header, little-endian:

| offset | field | values on the disc |
|---|---|---|
| 0x00 | `SShd` | |
| 0x04 | header size | 0x18 |
| 0x08 | type | 0x10 = SPU ADPCM |
| 0x0c | sample rate | 48000; 44100 in `mpegs[0]`, `[1]`, `[45]`, `[57]` |
| 0x10 | channels | 2 |
| 0x14 | interleave (bytes per channel block) | 0x20 (two 16-byte ADPCM frames) |
| 0x18, 0x1c | loop start / end | −1, −1 |
| 0x20 | `SSbd` | |
| 0x24 | body size | = the bytes that follow, exactly, in every file |

Body: blocks of `interleave` bytes, left then right (`sendADPCM` 0x31ae68 gathers each channel by skipping
`interleave·(channels − 1)` bytes). Each channel is one continuous SPU ADPCM stream (the frame format of
`docs/plan/audio.md` §2.3; flags 0 in the stream, the EE patches loop flags into its SPU ring). The port decodes each
channel from zero history with `rc_formats::vag::decode_frame` (rounded form), 28 samples per frame, and resamples
44.1 kHz to the 48 kHz output linearly. Audio durations equal the video durations within 0.3 s.

## 4. Video: MPEG-2 [H]

Header survey of all 84 files (`rc-video/tests/movies.rs` `every_movie_decodes`):

| field | values |
|---|---|
| size | 512 × 416 in every file (the NTSC GS draw buffer; the PAL copies too) |
| profile / level | 0x48 = Main Profile @ Main Level |
| chroma | 4:2:0 |
| frame rate | code 5 = 30 fps (NTSC in-level, transitions, attract), 3 = 25 fps (PAL copies), 4 = 29.97 fps (extras 70–73) |
| aspect code | 1 (square) or 2 (4:3) — the display ignores it (the picture fills the frame buffer) |
| quantiser matrices | default intra + loaded non-intra, or loaded intra + default non-intra (one sequence header); no quant-matrix extension |
| pictures | frame pictures only (`picture_structure` 3); I, P, B (per GOP typically one I, 4–5 P, 10–11 B) |
| `intra_dc_precision` | 8 bits (70–72, 75–77), 9 bits (`mpegs[1]` and the PAL copies), 10 bits (the rest) |
| `q_scale_type`, `intra_vlc_format` | both values used |
| `concealment_motion_vectors`, `repeat_first_field` | 0 |
| progressive | 82 files: `progressive_sequence` = 1, `frame_pred_frame_dct` = 1 (frame DCT, frame prediction, zig-zag scan) |
| interlaced | `mpegs[73]`, `[78]` (the 437 s extras): `progressive_sequence` = 0, `frame_pred_frame_dct` = 0 (per-macroblock frame / field prediction and frame / field DCT), **alternate scan**, `top_field_first` 0; no dual-prime |
| extensions | sequence extension; sequence display extension (ignored) in 64–66, 73, 78, 80–83 |

84 files, **98,629 pictures, 3,587 s**; every file decodes to its end without an error; no B picture lacks a
reference (every stream starts with a closed GOP).

**Decoder** (`rc_video::mpeg2`): exactly the tools above — ISO/IEC 13818-2 Annex B VLC tables as direct lookup
tables, intra DC prediction, both dequantisations with saturation and mismatch control, frame and field motion
compensation in frame pictures (field vectors predicted from `PMV >> 1`, stored doubled), half-sample
interpolation, rounded bidirectional averaging, skipped macroblocks (P: zero vector; B: frame prediction from the
vector predictors, also after a field-predicted macroblock), display reordering. The IDCT is separable `f32` with the
exact basis, IEEE 1180 compliant (peak error 1, per-pixel MSE ≤ 2e-4, overall mean error ≤ 8e-6). Anything else
(field pictures, dual prime, 4:2:2, concealment vectors) is an error, not a guess.

**Colour.** The IPU's CSC turns limited-range BT.601 YCbCr into full-range RGB, each chroma sample covering 2 × 2
pixels. The port does the same with the standard BT.601 matrix (16.16 fixed point, rounded, clamped) and
nearest chroma [M: the IPU's own fixed-point constants are not reproduced].

## 5. Checks

* **Correctness oracle** (dev only: the Homebrew ffmpeg run from the agent's scratchpad, never in product code or tests): frames dumped raw
  (`dump_yuv`) vs `ffmpeg -fps_mode passthrough -f rawvideo -pix_fmt yuv420p`. Same picture count and order
  everywhere. With ffmpeg's float IDCT (`-idct faani`): `mpegs[5]` first 200, `[73]` (interlaced, alternate scan,
  field MC) / `[22]` (PAL, 9-bit DC, mixed q_scale / VLC tables) / `[70]` (8-bit DC, 29.97) first 600, `[1]` all 363:
  max |Δ| = 1, overall PSNR 97.6–102.5 dB, worst frame ≥ 88.4 dB (the float IDCTs differ only on rounding ties).
  With ffmpeg's default integer IDCT: `[5]` all 1446 frames 65.4 dB overall, worst 62.9 dB, max |Δ| 5 (drift of
  1-LSB IDCT differences inside a GOP).
* Unit tests: synthetic program streams (packs, system header, split packets, two audio channels, padding, end code,
  errors), audio de-interleaving against `vag::decode`, VLC tables (prefix-free, code-space sums, 111 run / level
  pairs in both DCT tables), IEEE 1180, a synthetic I / P / B stream with hand-computed samples, BT.601 values.
* Real files: `rc-formats` `pss::tests::real_movies_demux` (0, 5, 40; `RC_PSS_ALL=1` all), `rc-video`
  `holofilm_start`; `every_movie_decodes` (ignored, 2 min): decode speed 774 pictures/s single-threaded in a dev build
  (26× real time).
