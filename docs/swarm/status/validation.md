# Status: VALIDATION

**Last updated:** 2026-10-10 UTC | **Branch:** lane/validation/impact | **Contract pinned:** contract-v0.3 | **Phase:** building

## Done
- Spike S9 (`docs/lanes/validation/spike-s9.md`), design note with CCR text (`design-note.md`), theory note stub (`docs/theory/validation.md`).

## In progress
- **Reply to ARCH (lane sessions cannot send messages): the proving-ground spec PR is #57.** Runner target: result JSON `w5k.proving.result.v1` (spec section 2); the runner must echo in `inputs` the numbers the sim actually used (mass_kg, mu, track_m, cg_height_m, wheel_radius_m, power_w ...); required keys per test are in the spec and `proving::TESTS`.
- Impact Matrix runner `w5k validation impact` (slice 2 stage A): 11 levers via FORGE's lever API, 6 benchmarks, three vehicles, 41 of 57 signs right (72%; 79% at a 1% no-change threshold); findings in `docs/lanes/validation/impact-v0.md`. PR open.
- Terrain scorer (`w5k_validate::terrain`, `w5k validation terrain --stats FILE`): reads WORLD's stats JSON. Finding: every road and ground roughness spectrum falls as about n^-4 where ISO 8608 assumes n^-2, so all are red (WORLD to check the generator and the fit window). WORLD's Wong ranges are marked Unverified, so friction lights are provisional. PR open.
- Proving-ground spec merged (#57); scorers (#81) merged. Scorers for (b) braking, (e) side slope, (g) step climb in `w5k_validate::oracle` (PR open); next (a), (c), (d), quarter-car for (f), terrain scorer, dashboard grid. Lanes may now message ARCH (RULES 5b): only for blockers.
- (earlier) PROVING-GROUND spec (owner direction via ARCH): `docs/validation/proving-ground.md` plus the result schema `w5k_validate::proving`; next: scorers for (b), (e), (g), then (a), (c), (d), quarter-car oracle for (f), terrain scorer, dashboard grid.
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
- **Oracle tolerance class** (ARCH to number): exact analytic oracles (braking `v^2/2mu g`, rigid rollover `atan(t/2h)`) need a class: default green 10%, amber 20%. Work tagged PROVISIONAL(oracle-class card).
- Published figures unreachable from lane sessions (M998 stopping distance, 0-32 km/h, ISO 8608 class limits, Wong friction table): marked SOURCE NEEDED, lights stay 'not measured'. Same ask as the S9 card.
- **S9-card (ARCH to number):** may the owner drop manuals in `content/dossier/sources/` or allow `*.army.mil`, `apps.dtic.mil`, `archive.org`? Default: carry on with UNVERIFIED secondary figures, never promoted to verified. Work tagged PROVISIONAL(S9-card).

## Evidence
- Reachability table in `spike-s9.md`. Tests: `dossier_ron_round_trips_and_every_quantity_passes_param_check`, `every_dossier_quantity_has_a_source_or_is_an_estimate_with_a_band`, `no_figure_is_duplicated_with_a_different_value`, plus the Secondary/UNVERIFIED rule and the M998 count. No image: nothing visible yet (dashboard is step 7).

## Owner instructions received
- 2026-10-10 (STATE): run without checking in; continue into build steps. Dossier-to-game-vehicle mapping lives in `content/dossier/` (C-001).
- Note: the closure-based lever test `every_lever_changes_the_vehicle_def_and_keeps_its_params_valid` was removed with my own RON-edit levers; FORGE's lever API carries that check.
