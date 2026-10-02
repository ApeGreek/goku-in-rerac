# Test request: deaths, the fall camera and the stick feel (2026-10-02)

`cargo dev` with a gamepad.

### D1. No death loop on Veldin
- **Claim:** letting a horny toad on Veldin bite Ratchet to death brings him back once at the start with full health,
  and he stays alive.

### D2. A death resets the level
- **Claim:** after any death the sounds stop, the level's music starts again from the beginning, and the enemies and
  crates are back where they started.

### D3. The fall camera
- **Claim:** walking off a ledge into a bottomless drop, the camera stays where it is and tilts down to follow Ratchet
  as he falls away.

### D4. Running diagonally
- **Claim:** holding the left stick fully in any direction, including diagonals, Ratchet keeps running; he only walks
  with the stick partly tilted.

### D5. Camera and stick response
- **Claim:** the right stick turns the camera at full speed before the stick reaches its edge, and small tilts
  register sooner than before, close to the original on PCSX2.

### D6. The death fade and pause
- **Claim:** after any death the screen fades to black in about a quarter of a second, stays black for about
  three quarters of a second, then Ratchet is simply back at the checkpoint, with no fade-in. The same happens
  falling out of the level.

### D7. What Ratchet keeps after a death
- **Claim:** after a death Ratchet holds the weapon he last had out, keeps his boots, his pack and Clank, but any
  headgear (the O2 Mask, the Pilot's Helmet, the Sonic Summoner) is off until you put it on again.

### D8. The checkpoint's music after a death
- **Claim:** dying after a checkpoint plays the music that was playing when the checkpoint was reached, from the
  start.
