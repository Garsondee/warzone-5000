# ADR-0007: How we validate against real vehicles

**Status:** Accepted (ARCH, 2026-10-08). The tolerances below are starting values; lane VALIDATION proposes changes by card.

## Context
The owner wants example vehicles built from parametric parts "designed to closely mimic real vehicles" and the physics checked against real values.
Public data is uneven: headline specifications (mass, dimensions, power, top speed, ground pressure, climb limits, range) are findable but
inconsistent between sources (an Abrams is quoted between 62 and 73 t); gear ratios, torque curves, spring rates, suspension travel and
centre-of-mass height are mostly unpublished.

## Decision
1. **Dossier per vehicle** (`content/dossier/<id>.ron`): every quantity is `{value, band, provenance, source}` with provenance `SPEC`, `MEASURED`, `ESTIMATE` (needs a band) or `TUNED`. Every figure is cited by the dossier lane; none from memory.
2. **Calibrate on civilians, hold out the tanks.** Calibration set: M998 HMMWV, M113A3, M4A3 Sherman. Held out: M1A1 Abrams, Leopard 2A5, T-72B, M35 6x6, Tiger II (a stress case). Only **global** constants are tuned, never per vehicle; each calibration is logged with the held-out score change; a held-out regression above 5% needs the owner.
3. **Pass rule:** the published value lies inside the simulation's *envelope* (a seeded Monte Carlo over the estimated inputs) and the nominal result is within tolerance: static quantities 3%, power 5%, top speed 7%, acceleration and braking 15%; mobility limits are *bracketed*: published <= simulated <= 1.25 x published.
4. **Targets by milestone:** M1: M998 within 15/7/15% on acceleration, top speed and braking, quarter-car frequency within 1% and damping ratio within 5%. M2: held-out median error at most 12% and maximum 25% over at least 20 quantities.
5. **Test pyramid:** unit and property tests; analytic benches (step response, energy drift, braking `v^2/2 mu g`, power balance, dyno curve, tyre slip curve, Bekker plate sinkage and Wong's drawbar-pull examples, pivot-turn torque near `mu W L / 4`, recoil momentum); subsystem checks; whole-vehicle checks against the dossier; scenarios and goldens; the Design Impact Matrix; fuzz.
6. **Dashboard:** `w5k validation dashboard` writes one self-contained HTML page (traffic lights per vehicle and quantity, sensitivity "tornado" charts, the matrix); it runs nightly in CI.
7. **Fun overrides** (deliberate departures from reality) exist only as logged, owner-signed deviations.

## Consequences
The claim we can honestly make is "inside the envelope of what is published". Where public data is thin we say so, widen the band, and let the envelope show it.
