# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/settling | **Contract pinned:** f8f5e5d (0.1.1) | **Phase:** settling round, awaiting review

## Done
- Spike S1 (`spikes/chassis/s1`, `docs/lanes/chassis/spike-s1.md`): stable above ~50 Hz for the 12 Hz hop mode; frequency error under 0.1%, damping under 1%, drift under 0.04%/min; rule `ceil(20 f_max/60)` confirmed; relaxation-length friction holds a 10% grade, Coulomb sign(v) creeps.
- Design note `docs/lanes/chassis/design-note.md` with the glue API and three CCRs (text only).

## In progress
- Nothing; stopped for review as the brief says.

## Blocked
- Nothing.

## Next
- After review: build step 1 (`quarter_car` bench and its four tests), then suspension.

## Cards needed / PROVISIONAL decisions in force
- None.

## Evidence
- `python3 -I spikes/chassis/s1/s1.py` output in `spikes/chassis/s1/output.txt`. No Rust changed.

## Owner instructions received
- none

## Handoff note (fill in when you stop)
- (fill in at M1)
