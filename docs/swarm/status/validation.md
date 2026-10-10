# Status: VALIDATION

**Last updated:** 2026-10-10 UTC | **Branch:** lane/validation/harness | **Contract pinned:** contract-v0.1 | **Phase:** building

## Done
- Spike S9 (`docs/lanes/validation/spike-s9.md`), design note with CCR text (`design-note.md`), theory note stub (`docs/theory/validation.md`).

## In progress
- PR #14 (settling) and #16 (dossier loader, M998 dossier) merged.
- Harness v0 PR: verdict classes, replay measurement, scoring, `w5k validation dashboard --out DIR` (self-contained HTML). Image: `docs/lanes/validation/media/dashboard-v0.png`.

## Blocked
- Nothing blocking. Primary sources are unreachable from lane sessions (proxy denies army.mil, DTIC, archive.org, Wikipedia, globalsecurity.org).

## Next
0. Harness v0 follow-ups: run on CHASSIS's Mule replay when it lands (`--replay FILE --top-speed-run` only for a real top-speed drive); scenario scripts (standing start, braking, gradient bisection).
1. (done in PR #16) dossier loader plus M998 dossier.
2. Scenario measurement on the stand-in model; verdicts with the double-power negative control.
3. M113A3 and M4A3 dossiers (about 14 and 12 quantities reachable: under the 25 target).

## Cards needed / PROVISIONAL decisions in force
- **S9-card (ARCH to number):** may the owner drop manuals in `content/dossier/sources/` or allow `*.army.mil`, `apps.dtic.mil`, `archive.org`? Default: carry on with UNVERIFIED secondary figures, never promoted to verified. Work tagged PROVISIONAL(S9-card).

## Evidence
- Reachability table in `spike-s9.md`. Tests: `dossier_ron_round_trips_and_every_quantity_passes_param_check`, `every_dossier_quantity_has_a_source_or_is_an_estimate_with_a_band`, `no_figure_is_duplicated_with_a_different_value`, plus the Secondary/UNVERIFIED rule and the M998 count. No image: nothing visible yet (dashboard is step 7).

## Owner instructions received
- 2026-10-10 (STATE): run without checking in; continue into build steps. Dossier-to-game-vehicle mapping lives in `content/dossier/` (C-001).
