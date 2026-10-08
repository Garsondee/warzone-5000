# Decision queue

Anything that is the owner's to decide and that no document already settles becomes a **card** here. Nobody blocks on a card: work
continues on the **default** and is tagged `PROVISIONAL(card-id)` in the code, the design note or the PR until the owner answers.
ARCH keeps this file current. Lanes do not edit it: they write the card in their status file under "Cards needed" and ARCH lifts it here and numbers it.

## Card format
```
### C-NNN  <short question>
Asked by: <lane> | Date: <date> | Status: OPEN / ANSWERED (<date>) / SUPERSEDED
Options: (a) ... (b) ... (c) ...
Recommendation: <which and why, in two lines>
Cost of being wrong: <what has to be redone, roughly how much>
Default (what happens if nobody answers): <option>
Answer: <owner's decision, when given>
```

## Open cards

### C-001  What era is the game?
Asked by: ARCH | Date: 2026-10-08 | Status: OPEN
Options: (a) modern-ish, c. 1950s to today, mixed-era reference set; (b) WW2 only; (c) both, as separate content packs.
Recommendation: (a). The physics must generalise (leaf springs and manual boxes are a subset of the machinery torsion bars and automatics need), so the reference set mixes eras; the game's era can be narrowed later.
Cost of being wrong: low until M2 (the content catalogue), moderate after.
Default: (a).

### C-002  Which real vehicles are in the reference garage?
Asked by: ARCH | Status: OPEN
Options: (a) calibration M998 HMMWV, M113A3, M4A3 Sherman; held out M1A1 Abrams, Leopard 2A5, T-72B, M35 6x6, Tiger II; (b) the owner's own list.
Recommendation: (a): well-documented civilian-adjacent vehicles first, tanks held out.
Cost of being wrong: dossier research is redone for swapped vehicles (days).
Default: (a).

### C-003  How are courses authored?
Asked by: ARCH | Status: OPEN
Options: (a) data files made by a procedural generator first, an in-Godot editor later; (b) a Godot editor plugin from the start; (c) hand-built in Blender.
Recommendation: (a): testable and reproducible; the editor follows once the format settles.
Cost of being wrong: the editor is built later than wanted.
Default: (a).

### C-004  Art pipeline
Asked by: ARCH | Status: OPEN
Options: (a) procedural PBR camo and weathering plus a lofted hull-geometry kit; hand-modelled hero assets optional later; (b) hand-modelled assets from the start.
Recommendation: (a); the owner may supply hero assets at any time.
Cost of being wrong: shader and kit work partly redone.
Default: (a).

### C-005  Can the owner drive any vehicle?
Asked by: ARCH | Status: OPEN
Options: (a) yes, keyboard or gamepad in the Godot front end, through the same `Command` as the AI; (b) watch only.
Recommendation: (a): an animator judges suspension by feel, and it is nearly free given `Command`.
Cost of being wrong: low.
Default: (a).

### C-006  What happens to the parked sci-fi families and the draft / command-point / shop loop?
Asked by: ARCH | Status: OPEN
Options: (a) parked in the archive, revisited at M5; (b) deleted.
Recommendation: (a).
Cost of being wrong: low.
Default: (a).

### C-007  Which model do the lanes run on?
Asked by: ARCH | Status: OPEN
Options: (a) the same as the coordinator for every lane; (b) a stronger model for CHASSIS, TRACKS and ARCH, the default for the rest; (c) the default everywhere, upgraded when a lane stalls twice.
Recommendation: (c): measure first; the burn is reported at every check-in.
Cost of being wrong: money, or time lost to a lane that stalls.
Default: (c).

### C-008  Hold the fan-out until the weekly usage window resets (about 10 Oct 02:00 UTC)?
Asked by: ARCH | Status: OPEN
Options: (a) hold rank 1 until after the reset if the Launch Kit finishes first; (b) launch immediately.
Recommendation: (a): the session metadata shows a seven-day usage warning; a parallel fan-out burns the allowance quickly.
Cost of being wrong: (a) loses hours; (b) risks hitting the limit mid-wave.
Default: (a).

### C-009  The old prototype code in the working tree
Asked by: ARCH | Status: DONE as (a) on 2026-10-08 (moved with `git mv`, one directory at a time; deleting stays the owner's word)
Options: (a) moved to `reference/prototype-v0/` (read-only, excluded from the workspace and CI); (b) removed from the tree entirely (history keeps it at `2854baf`).
Recommendation: (a) until the port ledger is done, then (b). Moving or deleting the code needs the owner's explicit word in this environment.
Cost of being wrong: none; both are one command.
Default: (a).

### C-010  When do we build the Windows package?
Asked by: ARCH | Status: OPEN
Options: (a) at M2, with Godot drive mode; (b) at M1 with the replay player.
Recommendation: (a), with the S8 spike (Godot in cloud sessions, gdext on 4.6) reporting earlier.
Cost of being wrong: the owner sees a playable build later than wanted.
Default: (a).

### C-011  Pace of the fan-out
Asked by: ARCH | Status: OPEN
Options: (a) three ranks over about a day, governed by the burn and CI health; (b) everything at once; (c) rank 1 only until M1.
Recommendation: (a).
Cost of being wrong: (b) burns the allowance and floods review; (c) leaves parallel-safe work idle.
Default: (a).

### C-012  CI minutes policy
Asked by: ARCH | Status: OPEN
Options: (a) PR runs on Linux only (affected crates), Windows and goldens on `integration` and nightly; (b) Windows on every PR.
Recommendation: (a): Windows minutes cost double on private repositories.
Cost of being wrong: a Windows-only break is found at merge, not at PR time.
Default: (a).

## Answered cards
*(none yet; the owner's four scoping answers are recorded as ADR-0001 to ADR-0004.)*
