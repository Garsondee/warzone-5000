# VALIDATION capability table for WORLD's mobility map
From: validation   To: world (and ai)   Needed by: `w5k world mobility` (PR 137)   Status: DELIVERED (a request for WORLD to confirm the shape)

**What it is:** per vehicle, what the proving ground *measured* by running the vehicle (not a spec): JSON `w5k.capability.v1`, one file per vehicle plus all of them in one array.
**Where:** `docs/lanes/validation/capability/<vehicle>.json` and `capability.json` (scout_4x4, mule_4x4, hauler_4x4). Regenerate with `w5k validation capability --out DIR [--vehicles a.ron,...]` (about 6 s per vehicle); the files in the repo are a snapshot at the integration head of the day.

## Shape (every field optional except `schema` and `vehicle`)
```json
{ "schema": "w5k.capability.v1", "vehicle": "mule_4x4", "contract_pin": "contract-v0.3",
  "surface": "dry asphalt (the proving ground's flat plane)",
  "max_grade_ratio":       { "value": 0.645, "unit": "ratio", "test": "gradeability", "light": "not scored", "note": "" },
  "max_side_slope_rad":    { "value": 0.723, "unit": "rad",   "test": "side_slope_rollover", "light": "green", "note": "slide" },
  "braking_distance_m":    { "value": 11.96, "unit": "m",     "test": "braking_50kmh", "light": "green", "note": "" },
  "braking_speed_m_s": 13.88,
  "time_0_48kmh_s":        { "value": 4.65,  "unit": "s",     "test": "accel_0_48kmh", "light": "not scored", "note": "" },
  "max_lateral_accel_m_s2":{ "value": 7.87,  "unit": "m/s^2", "test": "skidpad", "light": "not scored", "note": "grip plateau ..." },
  "max_step_m":            { "value": 0.344, "unit": "m",     "test": "step_climb", "light": "green", "note": "" },
  "missing": [] }
```
- SI units, key suffix = unit. `max_grade_ratio` is rise over run (0.6 = 60%). `braking_distance_m` is from `braking_speed_m_s`; scale quadratically for other speeds (`CapabilityTable::braking_distance_m` does).
- `light` is the proving-ground verdict where a scorer exists (side slope, braking, step climb): `green`, `amber`, `red`; `not scored` = measured but no oracle yet (gradeability, acceleration, skidpad). **A red value is still exported** (so you see what the sim does) but it is a model result that failed its physics check: treat it as untrusted. `note` says what limited it (side slope: `roll` or `slide`).
- A test that did not run or ended early leaves its field out and its id in `missing`. Never a guess.

## What it is not
Only hard dry ground is covered: **no per-surface values and no soft-ground (`soil_go`) or trench/fording data yet**. For the contract's `CapabilityTable`: `max_grade` and `top_speed_m_s` are per material there; this export has one value for the flat asphalt plane, so WORLD derives per-surface limits from the material friction ratio (its call) until the proving ground runs on other surfaces. Top speed, turn radius and the turret fields are absent.

## What I need from WORLD
Confirm the shape works for `w5k world mobility` (or say which key or unit to change). I will not edit the contract.
