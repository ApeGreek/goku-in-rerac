# Goku in ReRAC

A personal, offline mod of [ReRAC](https://github.com/re-rac/rerac) (the native PC port of Ratchet & Clank, 2002):
Goku from *Dragon Ball Z: Budokai Tenkaichi 3* (BT3, PS2) is drawn in place of Ratchet and plays BT3's original
fighter animations, driven by Ratchet's game logic.

**No game data is included in this repository.** You need your own discs of both games; everything game-derived
(`extracted/`, the Goku model `goku.glb`) is generated locally from them and must not be shared.

## What it does

- Goku's model follows Ratchet's position and facing and plays the BT3 motion matching Ratchet's state: idle, run,
  jump, fall, the three wrench combo swings as the BT3 rush (0x37–0x39), the jump attack as a flying kick (0x6D).
- Bomb Glove: a charged ki blast (0x7B wind-up, 0x7D release), the bomb drawn in Goku's hand.
- L1 (look stance) + □: the Kamehameha. Holding L1 charges it (0x105 start, 0x106 charge loop); □ fires it (0x107,
  then 0x108 while the beam is out). The camera stays behind Goku's shoulder instead of going first person, aiming
  along the game's own camera. The wrench is still thrown (same flight, damage, bolt pickup) but is invisible; a
  blue ki beam is drawn from Goku's hand to it.
- The wrench is never drawn; Clank, the packs and hand weapons hang from Goku's bones.

Animation ids come from the comments in [Tenkaichi3Decomp](https://github.com/z3xox/Tenkaichi3Decomp)
(`src/battle/btl_act_*.c`, e.g. `BtlAct_SuperBeamHandler` for the beam).

## Files

| Path | What |
|---|---|
| `crates/rc-engine/src/hero_glb.rs` | the mod: model, animation per hero state, root-motion pinning, bone anchors, Kamehameha beam, shoulder camera |
| `crates/rc-engine/src/moby_attach.rs` | wrench hidden, items attached to Goku's bones |
| `crates/rc-engine/src/play_camera.rs` | the shoulder camera hook |
| `crates/rc-engine/src/afterimage_render.rs` | no Ratchet / wrench after-images with the mod on |
| `crates/rc-engine/src/gameplay.rs` | Ratchet's own meshes hidden; the bomb drawn in Goku's hand |
| `crates/rc-engine/src/export_obj.rs` | dev tool: `RC_EXPORT_CLASS=<class>` exports a moby class to OBJ for Blender |
| `tools/goku/` | BT3 extraction and conversion scripts (Python standard library only) |
| `play-goku.cmd` | launcher (Windows) |

## Setup

1. Extract Ratchet & Clank's data as described in [README.md](README.md):
   `cargo xtask regen-data --iso "<your Ratchet & Clank disc>.iso"`
2. Make Goku's model from your BT3 disc (Goku's files are global ids 0x590–0x599: 0x590 is the model, 0x598 the
   animations):
   ```
   python tools/goku/extract_afs.py "<your BT3 disc>.iso" bt3 0x590 0x598
   python tools/goku/bt3_to_gltf.py bt3/0x0590.bin crates/rc-engine/assets/goku.glb --anims bt3/0x0598.bin
   ```
   The model's bones are named `nodo_<n>` and its clips `anim_<id>`; the code relies on those names.
3. Run `play-goku.cmd` (starts directly in Veldin), or set the variables yourself:
   `RC_HERO_GLB=goku.glb cargo dev -j 4`. Use `-j 4`: more parallel jobs can run out of memory on 16 GB.

## Tuning

`RC_HERO_SCALE` (default 0.09), `RC_HERO_YAW` (degrees, default 90), `RC_CLANK_POS` (`x,y,z` in the model's bind
space), `RC_CLANK_JOINT` (default `nodo_16`), `RC_HERO_FORCE=<clip index>` (loop one clip to look at it).

## Testing

Scripted input and frame dumps, e.g. the Kamehameha:

```
RC_HERO_GLB=goku.glb RC_LEVEL=1 RC_FRONTEND=0 RC_SCENE=0 \
RC_PLAY_SCRIPT='40-260:press L1,75-77:press SQUARE,160-162:press SQUARE' \
RC_DUMP_FRAMES=40..240 RC_DUMP_DIR=frames cargo dev -j 4
```
