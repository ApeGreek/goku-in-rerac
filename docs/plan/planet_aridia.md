# Aridia (level 02): the batch C checklist

Batch C's fourth planet, started 2026-10-02. Most of Aridia already ran ported code (the sandsharks, the surfer and his
agent, the turntables, the vendor, the path ships 1212). The census of 2026-10-02 (`cargo run -p rc-trace --
class-census`) left twelve placed classes without a port; this page tracks them. Porting aid: `rc-trace peek` (the
overlay disassembled, gp words, instances, cuboids, splines; `cargo run -p rc-trace -- peek --help`).

## Classes placed without a port (census 2026-10-02)

| class | placed | what | status |
|---|---|---|---|
| 651 | 3 | the anti-grav lifts and their glow column (draw `0x2dc2c8`) | **ported** (`units::aridia_lift`) |
| 656, 675, 732 | 2, 1, 3 | animated props (clips and random rests; 732 hums) | **ported** (`units::aridia_idlers`) |
| 733 / 1208 | 1 / 0 | the background cannon and its smoking shell; its boom travels to the camera | **ported** (`units::aridia_cannon`) |
| 735 / 736 | 1 / 1 | the gate halves (a quaternion swing once triggered) | **ported** (`units::aridia_gates`) |
| 762 | 1 | the boulder only a blast breaks (global flag 0x13d39d) | **ported** (`units::aridia_boulder`, with Novalis's burst helpers) |
| 792 | 1 | the platform an animated arm carries | **ported** (`units::aridia_arm_platform`) |
| 1213 | 6 | the big path ships (also 1973 on 09 / 10) | **ported** (`units::path_ship`, the [`BIG`] kind) |
| 1479 | 5 | the fire vents (type-2 flame streams and spark bursts) | **ported** (`units::aridia_fire`) |
| 713 | 1 | the launch tube: Ratchet walks in, is held (state 0x72) and carried to the other end while the camera flies the path; its rings 1057 / 1058 and four doors | **ported** (`units::aridia_tube`) |

## Shared-system changes in this pass

- `SoundSink::set_volume` / `World::set_volume`: `SoundSetVolume` 0x2a1968 (the slot's +0x10 volume; the cannon's boom).
- `DrawCallbacks::tick`: the tick counter the frame's draw reads (the lift's spinning glow column).
- `engine_trail::Blob::size_div` (the big path ships on level 10 divide the blob sizes by 1.5).
- `breakables::novalis::puffs` takes the puffs' colours; `puffs` / `bits` shared with 762.
- `cinematic::script_mode` (level02 `0x2f8a18`): the script camera's mode and timer mid-script.
- 788 (the Hoverboard agent) now asks the loader for its joint lists (its head-turn manipulators warned without them).

## Open

- The user's QA: docs/test-requests/2026-10-02-aridia.md.
