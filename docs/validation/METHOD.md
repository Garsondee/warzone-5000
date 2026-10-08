# Validation method

How we check the simulated vehicles against real ones. Decision record: ADR-0007. This page is VALIDATION's from the moment that lane starts; ARCH wrote the first draft.

## The idea in one paragraph
For each reference vehicle we collect real figures with their sources and uncertainty (the **dossier**), run the simulated vehicle through the same tests the real one was subjected to (the **scenarios**), and compare with a verdict per quantity.
Where the real data is thin the simulation's own inputs are uncertain too, so we compute an **envelope** (the spread of simulated results when the estimated inputs vary within their bands) and ask whether the published value lies inside it. We tune only global constants, on a calibration set of vehicles, and judge on a held-out set.

## Dossier schema (RON, `content/dossier/<id>.ron`; the template is `dossier-template.ron`)
A dossier is a list of quantities: `id` (stable, e.g. `static.mass_kg`), `category` (static, powertrain, dynamics, mobility, turret), `unit`, `value`, optional `lo`/`hi` band, `provenance` (`Spec`, `Measured`, `Estimate`, `Tuned`), `source` (document title and page, or URL and access date), `notes`.
Rules: every `Spec`/`Measured` has a source; every `Estimate` has a band; disagreeing sources are both recorded and the band covers them; nothing is from memory; a source you could not open is marked `UNVERIFIED`.

## Quantities to collect (at least 25 per vehicle)
- **Static:** mass (combat or curb, say which), length, width, height, wheelbase and track (wheeled) or track length and width (tracked), ground pressure, ground clearance, centre-of-mass height (usually an estimate).
- **Powertrain:** rated power and rpm, peak torque and rpm, gear ratios (usually unpublished: estimate), speed per gear, governed top speed, 0-32 km/h time, fuel capacity, range.
- **Dynamics:** braking distance from 32 km/h, turning radius or pivot capability, ride frequency (estimate), turret 360-degree traverse time, gun depression and elevation, rate of elevation.
- **Mobility:** maximum gradient, side slope, trench width, vertical step, fording depth.

## Scenarios (the same tests, run on the model)
Standing start to speed; top speed on level hard ground; braking from 32 km/h (and repeated stops for fade); gradient climb and hold (bisection for the limit); side slope (tilt table); vertical step and trench; fording depth (a limit in the capability table); pivot turn and minimum turn radius at walking pace and at 20 km/h; turret traverse and gun elevation; ride over the bump strip (RMS vertical acceleration). Each scenario is a script (`ScriptedCommands` plus a `WorldQuery` construction) so it is reproducible and can be a regression test.

## Verdicts (tolerances are ADR-0007's starting values)
A quantity is **green** when the published value lies inside the simulation's envelope **and** the nominal result is within tolerance: static 3%, power 5%, top speed 7%, acceleration and braking 15%. Mobility limits are *bracketed*: published <= simulated <= 1.25 x published (the published figure is a rating the vehicle is guaranteed to meet, so the model should meet it without wildly exceeding it). **Amber** is within 2x the tolerance. **Red** is anything else, and red stays red on the dashboard until the model is fixed or a card changes the tolerance.

## Envelopes
Sample the `ESTIMATE` inputs uniformly within their bands (and `SPEC`/`MEASURED` within published scatter), seeded by `Pcg32`; run the scenario for each sample; report the 5th to 95th percentile as the envelope. 200 samples for a dashboard run, 1000 for a milestone report.

## Calibration discipline
- Calibration set: M998 HMMWV, M113A3, M4A3 Sherman. Held out: M1A1, Leopard 2A5, T-72B, M35 6x6, Tiger II.
- Only **global** constants are tuned (e.g. a tyre-model shape constant, a soil-shear modulus), never per vehicle. Each tuning is logged in `docs/validation/calibration-log.md` with the held-out score before and after. A held-out regression above 5% needs the owner.

## Negative controls
Every check ships with a deliberately wrong model that must fail it (double the power, ignore a lever, break a sign). A test that cannot fail proves nothing.

## The Design Impact Matrix (`IMPACT-MATRIX.md`)
For each design lever, perturb it by +-10% on a baseline vehicle, rerun the benchmarks, and compare the sign and rough size of each change with the expected table. Report **dead levers** (change nothing anywhere) and **orphan effects** (a benchmark no lever can move) as failures: both mean the model is wrong or the design space is hollow.

## The dashboard (`w5k validation dashboard --out DIR`)
One self-contained HTML page: traffic lights per vehicle and quantity, the tornado chart per benchmark (which levers move it, and how much), the impact matrix, the provenance meter (counts of SPEC, MEASURED, ESTIMATE, TUNED), the calibration log. Nightly in CI.

## Reports to the owner
At every milestone: the dashboard, one vehicle's published-versus-simulated table, the biggest reds with their explanations (from the force ledger), and the list of figures that are only estimates.
