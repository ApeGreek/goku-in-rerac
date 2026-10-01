---
status: open
job: hoverboard
date: 2026-10-01
commit: f8c48c9
areas: [hero, classes, menus]
---

# The Hoverboard: hero states 0x6b..0x6f, the board 439 and its race

## 1. Summary

Rilgar's (and Kalebo III's) hoverboard had no port: the board class 439 did nothing and the hero froze in 0x6b. The
hero's board states, the race the board keeps (laps, places, wrong way, the finish) and the board class are ported
from level05's source (`hero/hoverboard.rs`, `moby_update/classes/units/hoverboard.rs`). The bot racers 717 race too (their own commit). Not yet: the boost pickups 133, the race HUD, the hoverboard camera (class 8: the follow camera
is used), level 16's state 0x3e and its board weapon.

## 2. Where it lives

Coverage tables in both module docs. Docs: gaps.md G-HERO-008, G-LVL-007; hero_states.md §1.3.

## 3. Behaviours to verify

Level 5. The board waits at (275.5, 334.4, 73.6); `RC_HERO_AT=272,333,74` lands next to it.

### B1. Mount
- **Claim:** walking within 5.5 of the board puts Ratchet on it at the race start (247, 287, 76), riding; the board
  sits under his feet and its thrusters puff.

### B2. Riding
- **Claim:** the board accelerates to 15 u/s, steers with the stick (turning against the camera), leans into turns;
  on slopes the body tilts to the ground; hitting a wall face on throws him off (0x6f) and he reappears on the board
  further back along the course; a hard capsule hit or a drop below the course throws him (0x6d) the same way.
- **Claim:** ✕ jumps (✕ held lifts higher); on a ramp surface the jump is the big ramp jump; L1 / R1 / L2 / R2 in the
  air play the four tricks and the stick spins him; landing tilted or mid-trick throws him off.

### B3. Pickups
- **Claim:** passing through a hoop (1139) or over a glowing pad (1140, its glow pulses) boosts for the pickup's time
  (the speed rises to 26 u/s, after-images behind Ratchet, the boost sound on the board).

### B4. Water and wrong way
- **Claim:** falling below 61.4 splashes into the water (0x6e); after the water timer he is back on the board.
- **Claim:** riding against the course for 3 s fades the screen to black and back with Ratchet placed on the course.

### B5. The race's end
- **Claim:** after three laps: first place in the first race tells the race girl (she returns to the finish);
  otherwise (or once the race is won) the "Quit Race?" dialog shows the place, the time and the score; "No"
  restarts the race at the start.

### B6. The racers (717)
- **Claim:** as soon as Ratchet is on his board the four racers set off from their lanes on their own boards with a
  start boost, join the racing line, take its branches and ramp jumps (a trick in a long jump), boost through hoops and
  pads, and keep near Ratchet: slower while ahead of him, faster while behind. Odd-numbered racers lean mirrored.

### B7. The roaming boost pickups (133)
- **Claim:** small pickups wander the course in five clusters (each toward a point of its path, a new one every ten
  seconds or when reached), dripping glowing goo; riding into one on the board bursts it into a goo spray with its
  sound and boosts Ratchet for a second; it reappears once he is 48 away.

### B8. The race HUD (slots 5 / 7)
- **Claim:** while on the board, a bar across the bottom shows "Lap: n/3" on the left, "Time: m:ss:hh" in the
  middle and "Place: 1st..5th" on the right (shadowed large text); off the board it is gone. After the race has been
  won once, a second bar above shows "Score: n".
