# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/build | **Contract pinned:** f8f5e5d (0.1.1) | **Phase:** settling round, awaiting review

## Done
- Spike S1 (`spikes/chassis/s1`, `docs/lanes/chassis/spike-s1.md`): stable above ~50 Hz for the 12 Hz hop mode; frequency error under 0.1%, damping under 1%, drift under 0.04%/min; rule `ceil(20 f_max/60)` confirmed; relaxation-length friction holds a 10% grade, Coulomb sign(v) creeps.
- Design note `docs/lanes/chassis/design-note.md` with the glue API and three CCRs (text only).

- Settling PR #7 merged. ARCH go + owner instruction (STATE.md, 2026-10-10 02:30) verified in repo; building without waiting for review.
- Build step 1: `quarter_car` bench with its four oracle tests (this PR).

## In progress
- Next PR: `suspension` (all SpringKinds, damper, bump stop).

## Blocked
- Nothing.

## Next
- suspension, tyre (rolling-resistance speed scale as a Param, not a const-ok literal), hull, wheeled assembly on the bump strip, ledger, glue API.

## Cards needed / PROVISIONAL decisions in force
- None.

## Evidence
- `python3 -I spikes/chassis/s1/s1.py` output in `spikes/chassis/s1/output.txt`. No Rust changed.

## Owner instructions received
- ARCH (2026-10-10): CCR-1..3 accepted in principle; keep tyre extras local, loaded from RON, tagged PROVISIONAL(CCR-chassis-1..3); rolling-resistance fade speed must be a `Param`.

## Handoff note (fill in when you stop)
- (fill in at M1)
