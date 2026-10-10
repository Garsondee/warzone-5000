# VALIDATION design note (settling round)

## 1. Dossier schema (RON, `content/dossier/<id>.ron`; extends `docs/validation/dossier-template.ron`)
```
Dossier( id, name, role: Calibration|HeldOut, era, game_id: Option<String>,   // game_id: the fictional vehicle this maps to (owner answer C-001)
  quantities: [ Quantity( id: "static.wheelbase_m", category: Static|Powertrain|Dynamics|Mobility|Turret,
                          unit: "m", param: Param(..), evidence: Primary|Secondary|Cross, notes: "" ) ] )
```
`evidence` is new: `Primary` = I read the manual or test report; `Secondary` = seen only in a search summary or a third-party page (UNVERIFIED); `Cross` = an open model (Chrono), plausibility only, never scored. `Param` is unchanged. Checks: `Param::check`, ids unique and `<category>.<name>_<unit-suffix>`, Spec/Measured carry a source, Estimate carries a band, `Secondary` source starts with `UNVERIFIED`, the same quantity never appears twice with different values.

## 2. Scenario catalogue (each a script: `ScriptedCommands` plus a world built from a terrain constructor)
standing start to 32 km/h and to 80% of top speed; top speed on level hard ground; braking from 32 km/h (full brake, ABS off, distance from the driven trace); gradient (bisection on slope angle until the vehicle cannot hold 5 s of climb); side slope (bisection on tilt until rollover or slide, 5 s hold); vertical step and trench (bisection on height or width, the vehicle at 5 km/h); fording (limit read from the `CapabilityTable`, a one-line check); pivot turn and minimum turn radius at walking pace and at 20 km/h; turret traverse 360 degrees and gun elevation limits (read from the rig); bump-strip ride (RMS vertical acceleration at 20 km/h).
**Measurement:** the harness reads `Frame` samples from the replay plus the force ledger; it never reaches inside a model. Events are timed to the tick (`standing_start_..._within_one_tick`).

## 3. Verdicts (ADR-0007 values, unchanged)
Classes: static 3%, power 5%, top speed 7%, acceleration/braking 15%, mobility bracketed (published <= sim <= 1.25 x published). Green = published inside the envelope *and* nominal within tolerance; amber = within 2x; red otherwise. A `Secondary` quantity can be at most **amber-capped "provisional green"**, shown hatched on the dashboard, until a primary source confirms it (PROVISIONAL(S9 card)).

## 4. Monte Carlo and envelope
Draw each `Estimate` (and each `Secondary`) input uniformly in `[lo, hi]` with a seeded `Pcg32` (seed = hash of vehicle id and scenario id, so results do not depend on run order), 200 samples for the dashboard, 1000 for milestones; report the 5th-95th percentile. Ordered iteration only. The envelope is only as honest as the bands: widening a band widens the envelope (`envelope_widens_when_a_band_widens`).

## 5. Dashboard
One HTML page, inline SVG, no URLs: (a) per-vehicle table "published | simulated | band | verdict", (b) traffic-light grid vehicles x quantities, (c) tornado chart per benchmark, (d) impact matrix, (e) provenance meter with four slices plus "secondary (unverified)", (f) calibration log.

## 6. Impact-matrix runner
For each lever: baseline, lever x0.9, lever x1.1; rerun the benchmark set; record signed relative change per benchmark. Compare signs with the table in `IMPACT-MATRIX.md`. **Negative controls:** a model that ignores one lever must be flagged dead; a benchmark nobody can move must be flagged orphan. Fuzz later (build step 6).

## 7. Risk spike result
See `spike-s9.md`: sources are only partly reachable; everything from the web is `Secondary`.

## 8. CCRs expected (text only; I do not edit `w5k_contract`)
- **CCR-V1, telemetry:** `VehicleModel` exposes `fn telemetry(&self) -> &[(&'static str, f64)]` (named SI scalars: engine rpm, gear, wheel slip, suspension travel) so the harness can report peak torque, shift points and ride travel without privileged access. Default impl returns an empty slice, so no model breaks.
- **CCR-V2, scenario terrains:** constructors in `w5k_sim` for `Ramp{slope_rad}`, `SideSlope{tilt_rad}`, `Step{height_m}`, `Trench{width_m}`, `WaterDepth{depth_m}` implementing `WorldQuery`.
- **CCR-V3 (small):** a `CapabilityTable` field for rated fording depth so the fording scenario can read it.
Until they land I build the scenarios on `FlatPlane` and `BumpStrip` and test the measurement code on the stand-in.

## 9. Build order I will follow
PR 2 dossier loader plus the M998 dossier and its three named tests; PR 3 scenario measurement on the stand-in; PR 4 verdicts with the negative control; then Monte Carlo, impact matrix, dashboard. Each under 400 lines of non-test code.
