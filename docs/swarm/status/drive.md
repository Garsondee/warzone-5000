# Status: DRIVE

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 | **Branch:** lane/drive/settling | **Contract pinned:** contract-v0.1 (f8f5e5d) | **Phase:** settling

## Done
- Spike S-D (`spikes/drive/spike_d.py`, `docs/lanes/drive/spike-d.md`): implicit stick/slip clutch stable 60 Hz to 1 kHz; explicit regularised sign never stable; converter fine with linearised pump torque; no DRIVE substepping needed.
- Design note `docs/lanes/drive/design-note.md` with the CCRs expected (all additive, none blocking).

## In progress
- Settling PR open for review. Nothing else until it is reviewed.

## Blocked
- Nothing.

## Next
- After review: engine tests and `w5k_drive` engine/coupling code (build order 1-2); lumped bench to test the lock with a speed-dependent load.

## Cards needed / PROVISIONAL decisions in force
- None. Fuel map shape (quadratic bowl around the BSFC best point) is an ESTIMATE, not a card.

## Evidence
- `python3 -I spikes/drive/spike_d.py` output in `docs/lanes/drive/spike-d-output.txt`. No image yet (no plotter); spike plots to follow with the first bench.

## Owner instructions received
- none

## Handoff note (fill in when you stop)
- Changed: ... | Unfinished: ... | Surprised me: ... | I would do next: ...
