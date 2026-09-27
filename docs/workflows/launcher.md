# Workflow: the launcher (dev version)

The launcher is a separate repo: `~/Repos/randcrw-launcher` (Tauri; its README covers building and running it). The
contract between the two repos is `docs/plan/launcher_contract.md`; the mod system is `docs/plan/mods.md`.

## Run a dev build of the game from the launcher

| Task | Command / action |
|---|---|
| Make a version folder and zip of this repo's game | `tools/package/package.sh` (docs/workflows/release.md) |
| Install it in the launcher | Launcher → Settings → Version Management → Development → **Install from zip…** → `dist/randcrw-<version>-<os>-<arch>.zip`, then **Set active** |

Instead of the zip, **Add build folder…** with the unpacked `dist/randcrw-<version>-<os>-<arch>/` folder uses it in
place (re-run `tools/package/package.sh --no-build` after a `cargo build --release` to refresh it).

The launcher then extracts the game data with the version's own `randcrw-extract` into its data folder
(`<data root>/games/rac1/data/`) and starts `randcrw --data-dir <that folder>`. That folder is separate from the
repo's development `extracted/` (docs/workflows/game-data.md).
