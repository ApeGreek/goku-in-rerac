# Test requests

Implementation agents don't test their own work. At the end of every job, the agent writes **one file here**
describing, in detail, what must be tested. A dedicated **test expert agent** (brief:
`docs/workflows/test-expert.md`) then picks a request, decides how to test each claim, writes and runs the tests, and
records the results in the same file.

- One file per job: `YYYY-MM-DD-<short-job-name>.md`. Never edit another job's file, except the test expert filling in
  the Results section and the status.
- Status (front matter): `open` → `in-test` → `passed` / `failed` (bugs listed) → `closed` (bugs fixed and retested).
- Personal files (the ISO, ~/PS2, savestates, recordings) are never test inputs. Tests use the extracted level data
  the existing harnesses use, or synthetic data.

## Format

```markdown
---
status: open
job: <short job name>
date: YYYY-MM-DD
commit: <filled in by the coordinator at commit time>
areas: [classes, hero, world, ...]   # cargo xtask test-job areas the tests belong to
---

# <What was implemented, one line>

## 1. Summary
What was built, which systems / classes / levels, instance counts. What is now plug-and-play and how a new consumer
plugs in.

## 2. Where it lives
Files and functions (path::fn), the decomp addresses they port, the module docs that hold the coverage tables.

## 3. Behaviours to verify   (the core: one entry per testable claim)
For EACH ported coverage-table row (or tight group of rows):
### B<n>. <short name>
- **Claim:** exactly what the game does (with values: timers, distances, speeds, counts, damage, flags, sound ids,
  particle types and counts, drop counts), and the decomp address.
- **Setup:** level, moby (class, spawn index or position), Ratchet's position / state / items, preconditions
  (e.g. "Ratchet in water", "within 32 of the camera", "after the first hit").
- **Trigger:** what to do (inputs, a hit with which weapon, time passing).
- **Expect:** every observable: state sequence, positions, side effects on other mobys, sounds, particles, lights,
  HUD / save / stats writes, rand draws if they matter.
- **Edge cases:** branches that are easy to miss (the second hit, the timeout path, the out-of-view path, level-
  specific branches, both sides of every condition).
- **Suggested method:** unit test on the function, level-harness test, trace compare against a PCSX2 recording,
  visual check, or "QA in game" (say which, and why).

## 4. Shared code touched (regression risk)
Every shared function or seam changed, what used to call it, and which existing tests / behaviours could change
and why (and whether that change is intended and faithful).

## 5. Not ported (don't test as working)
Every coverage row marked NOT ported, with its gap id.

## 6. In-game QA spots (for the user)
`RC_LEVEL=… RC_HERO_AT=x,y,z,yaw RC_GIVE_ITEMS=…` and what to look and listen for.

## 7. Results (the test expert fills this in)
Per behaviour: test name / method, pass / fail, notes. Bugs found: what, where, the evidence.
```
