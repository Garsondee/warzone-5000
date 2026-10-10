# Status: WORLD

**Last updated:** 2026-10-10 UTC | **Branch:** lane/world/settling | **Contract pinned:** contract-v0.1 (commit f8f5e5d) | **Phase:** settling

## Done
- Spike S-W (`docs/lanes/world/spike-w.md`, code `crates/w5k_world/src/spike_w.rs`): height 24 ns, sample 61 ns, raycast 12 us, generator hash is a constant, 9 tests green.
- Design note `docs/lanes/world/design-note.md`; CCR text `docs/swarm/requests/world-ccr-materials-props.md`.

## In progress
- Opening the settling PR into `integration`.

## Blocked
- Nothing.

## Next
1. Branch `lane/world/build`: real heightfield module + the data-driven bump strip (CHASSIS needs it first).
2. `MaterialTable` from RON with cited soil numbers (Wong tables; anything unciteable tagged UNVALIDATED).
3. Generator (hills with grade clamp, roads, mud, trees, barricades, village), then `w5k world export|preview`.

## Cards needed / PROVISIONAL decisions in force
- None. C-004 default (parametric world) applies.

## Evidence
- Tests: see spike table. Image: `docs/lanes/world/media/spike-w-map.png`.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md); continue into build steps without waiting for review.
