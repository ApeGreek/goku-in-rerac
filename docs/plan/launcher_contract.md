# Launcher ↔ game contract v0

Status: **in force** (2026-09-26). The launcher (`randcrw-launcher`, a separate Tauri repo) and this game repo both
implement exactly the text in "Contract (verbatim)". The section after it, "Clarifications", records how the game
side implements the points the contract leaves open. Nothing in it changes a field, a command or a code; where it adds
something, the addition is optional for the launcher to use.

Background and the decisions behind it: `docs/plan/launcher_extractor.md`, `docs/plan/decisions.md`.

## Contract (verbatim)

## Launcher ↔ game contract v0 (the orchestrator's spec; the launcher and the game repo both implement exactly this)
- A **version** (one downloadable game build, official or a mod) is a folder `versions/<source>/<version>/` containing `randcrw-manifest.json`:
  `{"schema":1,"name":"randcrw","version":"0.1.0","game":"rac1","runtime":"<relative path to game binary>","extractor":"<relative path to extractor binary>","supported_discs":["SCUS_971.99"],"data_format":1}`
  The launcher's Development source points at a local folder with this manifest (e.g. a dev build folder).
- **Extractor CLI** (`randcrw-extract`):
  - `randcrw-extract identify --iso <path> --json` → identifies the disc only.
  - `randcrw-extract extract --iso <path> --out <game_data_dir> [--ntsc-only] --json` → full extraction.
  - `randcrw-extract verify --out <game_data_dir> --json` → re-hashes against the built-in size/SHA-1 table.
  - With `--json`, stdout is JSON lines, one object per line:
    - `{"type":"progress","stage":"identify|copy|verify","done":<bytes>,"total":<bytes>,"file":"<path>"}`
    - `{"type":"info","message":"..."}`
    - `{"type":"disc","serial":"SCUS_971.99","region":"NTSC-U","version":"1.00","supported":true}`
    - `{"type":"error","code":<n>,"message":"..."}`
    - `{"type":"done","elapsed_ms":<n>}`
  - The process exit code equals the error code.
  - Codes:
    - 0 ok
    - 10 cannot open/read file
    - 11 not an ISO9660 image
    - 20 not Ratchet & Clank (unknown disc)
    - 21 Ratchet & Clank but an unsupported build/region (only SCUS_971.99 v1.00 for now)
    - 30 write failure
    - 31 not enough disk space
    - 40 verification failed
    - 99 internal error
  - On success the extractor writes `<game_data_dir>/extract-info.json`: `{"disc":"SCUS_971.99","data_format":1,"extractor_version":"...","ntsc_only":false,"files":<n>,"bytes":<n>}`.
- **Runtime:**
  - `<runtime> --version-json` prints `{"name":"randcrw","version":"...","game":"rac1","data_format":1}` and exits.
  - The launcher starts the game with `<runtime> --data-dir <game_data_dir>`; the env var `RC_DATA_DIR` is equivalent.
- **Folders:** per-OS app data root named `randcrw`, e.g. macOS `~/Library/Application Support/randcrw/`. Under it:
  - `versions/`
  - `games/rac1/data/` (the extracted game data)
  - `logs/`
  - `settings/`

  The user can move the data root. Never use the name "randcre" anywhere user-facing.

## Clarifications (game side, `crates/rc-extract`)

### Output stream
1. **One terminal line.** Every run ends with exactly one `done` line (exit code 0) or exactly one `error` line (exit
   code = its `code`). Nothing follows it. The exit code is authoritative if the stream is cut short (process killed).
2. **Extra fields are additive.** Objects may carry fields beyond the contract; the launcher must ignore unknown
   fields. Today only the `disc` line has extras:
   `{"type":"disc","serial":"SCUS_971.99","region":"NTSC-U","version":"1.00","supported":true,"game":"rac1","title":"Ratchet & Clank","elf_sha1":"72fd…"}`.
   `game` is `rac1`, `rac2`, `rac3`, `racdl` or `unknown`.
3. **When the `disc` line appears.** Whenever the boot ELF named by `SYSTEM.CNF` was read and hashed: on success (0),
   for code 21, and for code 20 when the disc is a PS2 disc of another game (`"game":"unknown"`,
   `"supported":false`). It comes after the `identify` progress lines and before any `error` line. There is no `disc`
   line for codes 10 and 11, or for code 20 without a readable boot ELF.
4. **Progress.** `done`/`total` are bytes; `total` is fixed for a stage. `file` is the archive-relative path (forward
   slashes) of the file most recently started; for `identify` it is the boot ELF's disc path (`SCUS_971.99`). Lines are
   rate-limited to about 10 per second, and each stage always ends with a line where `done == total`.
5. **Stages per command.**
   - `identify`: `identify` (the boot ELF is hashed).
   - `extract`: `identify`, then `copy`. The SHA-1 of every file is computed while it is copied and checked against the
     built-in table in the same pass, so `extract` has no separate `verify` stage. A mismatch fails with code 40.
   - `verify`: `verify` (every file is re-read from disk and hashed).
6. **Info lines** are human-readable and not meant to be parsed, with one exception kept stable for bug reports: for an
   unknown R&C build (code 21) one info line starts with `new build DB row: ` followed by a ready-to-paste Rust row for
   `crates/rc-extract/src/build_db.rs`.

### Codes in practice
| Code | Raised when |
|---|---|
| 10 | The image cannot be opened or read (missing, permission, I/O error), **or it is truncated**: it has fewer sectors than its ISO 9660 primary volume descriptor declares, or a lump runs past its end. |
| 11 | No ISO 9660 primary volume descriptor at sector 16 (random files, CHD, CSO, too small), **or a raw 2352-byte-sector image** (`.bin`); only 2048-byte `.iso` images are accepted for now. |
| 20 | An ISO 9660 image that is not a Ratchet & Clank disc (no `SYSTEM.CNF`, or a boot ELF of another game). |
| 21 | A Ratchet & Clank disc that is not SCUS_971.99 v1.00: other RC1 regions and demos (SCES_509.16, SCPS_150.37, …), an RC1 serial whose boot-ELF SHA-1 is unknown, and the sequels (R&C 2, 3, Deadlocked; `game` says which). |
| 30 | Creating the output folder or writing, renaming or deleting a file under `--out` failed. |
| 31 | The free space at `--out` is below the bytes to be written plus 64 MiB (checked before anything is written), or a write failed with "disk full". |
| 40 | `extract`: a copied file's size or SHA-1 differs from the table (the disc image is damaged). `verify`: a file is missing, has the wrong size or SHA-1, or `extract-info.json` is missing or unreadable. Info lines name up to 50 bad files. |
| 99 | A bug, or invalid command-line arguments (the message starts with `usage:`). |

### Extraction behaviour
7. **Cancelling = killing the process.** The extractor is crash-safe instead of catching signals:
   - `extract-info.json` is deleted when `extract` starts and written last (temp file + rename), so **a data folder
     without `extract-info.json` is incomplete** and must not be launched.
   - Each file is written as `<name>.partial` and renamed only after all its bytes are written and its SHA-1 matched.
     A killed run leaves at most a few `.partial` files and never a complete-looking wrong file.
   - Re-running `extract` into the same folder removes stale `.partial` files and rewrites everything. The launcher may
     also simply delete the folder.
8. **`--ntsc-only`** skips the PAL copies, which the NTSC-U game never reads: every `levels/NN/gameplay_pal.bin`,
   every `levels/NN/scene/KK_pal.bin`, `global/credits_images_pal/*` and the PAL FMVs `global/mpegs/` 021–039,
   052–062, 075–079, 084–087. `extract-info.json` records `"ntsc_only":true` and `verify` then skips those files.
   Files already present from an earlier full extraction are left alone.
9. **`extract-info.json`** has exactly the contract's keys. `files`/`bytes` count the archive files written by this run
   (not `extract-info.json` itself). `extractor_version` is the `randcrw-extract` crate version.
10. **Paths.** `--out` is created if missing. `--iso` may be omitted when the environment variable `RC_ISO` is set
    (developer convenience; the launcher always passes `--iso`). `--flag=value` is accepted as well as `--flag value`.
11. **Verify** reads `extract-info.json` to learn the disc and `ntsc_only`, then checks exactly the table's files.
    Extra files in the folder (logs, caches, mods) are ignored.
12. **Extra commands and flags** (not used by the launcher): `table` (developer tool that regenerates the committed
    size/SHA-1 table from a disc), `--threads <n>` (copy/hash workers, default 4), `--help`, `--version`.

### Runtime and folders
13. The extractor and the runtime never compute the per-OS data root; they only receive paths (`--out`,
    `--data-dir`/`RC_DATA_DIR`). The launcher owns the root and the `versions/`, `logs/`, `settings/` folders.
14. **Runtime exit codes.** When the runtime cannot start, it prints one `error: ...` line on stderr and exits before
    any window opens (`crates/rc-engine/src/disc_source.rs`). The launcher should show that line with the message:

    | Exit | Raised when | Launcher |
    |---|---|---|
    | 0 | Normal quit (also after `--version-json`) | |
    | 2 | `--data-dir` given without a value | "randcrw couldn't start." (a launcher/game mismatch) |
    | 3 | The data folder is missing, is not a data folder (no `toc.bin`), or the extraction is incomplete (no `extract-info.json`) | "The game data is missing or incomplete." + Re-extract |
    | 4 | `extract-info.json` has another `data_format`, or cannot be read | "The game data doesn't match this randcrw version." + Re-extract |

    Any other non-zero exit (a panic is 101; a signal has no code) is a crash; point at the log. Unknown arguments are
    ignored with a warning, so a newer launcher's extra flags do not stop an older runtime. `--data-dir` wins over
    `RC_DATA_DIR`, and `--data-dir=<dir>` is accepted.
15. **Packaged version folder** (`tools/package/package.sh`, README "Packaging"): `randcrw` (`randcrw.exe` on
    Windows), `randcrw-extract`, `assets/shaders/*.wgsl`, `randcrw-manifest.json`, `README.txt`. The manifest names
    the binaries by these relative paths. The runtime finds `assets/` next to its real (symlink-resolved) executable,
    so the folder can be copied or moved as a whole, but `randcrw` must not be copied out of it alone.
16. `<game_data_dir>` (`games/rac1/data/`) holds the Tier 0 archive directly, in the same layout as the development
    `extracted/` tree: `boot/`, `toc.bin`, `global/`, `levels/NN/`, plus `extract-info.json`. See
    `docs/plan/launcher_extractor.md` §4.1.
