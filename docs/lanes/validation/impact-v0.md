# Design Impact Matrix v0: first run (scout, mule, hauler)

`w5k validation impact --out DIR` perturbs ten levers by +10% (RON edits of the `VehicleDef`, one proving-ground run per variant, about 30 s in total), compares the sign of each change with `docs/validation/IMPACT-MATRIX.md` and writes `impact.json` and `impact.md`. Nothing is tuned: this is a measurement.

**Coverage today:** 6 of the 16 benchmarks (B1 timed 0-48 km/h, not 0-32; B4, B6, B7, B11, B12) and 10 levers. Ride (B9, B10), fuel (B3), soft ground (B8), pivot (B13) and the turret benchmarks have no runner yet, and the size check (within a factor of 2) is not implemented: only two table rows carry a number.
**Threshold:** a change under 0.5% counts as "no change" (`~0`).

## Result: 37 of 54 scored signs agree with the table (69%), below the 80% target

Cells are the change in the benchmark for +10% of the lever; `ok` agrees with the table, `WRONG` does not (the table's sign follows), `unlisted` moved but the table has no row for it (reported, not scored).

### hauler_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -4.3% ok | +6.2% unlisted | +0.0%  | +0.0%  | +0.0%  |
| com_height | +1.6% unlisted | +2.1% ok | +0.0%  | -7.5% ok | -7.7% unlisted | -7.8% WRONG (table: ~0) |
| engine_power | -6.8% ok | +0.0%  | +0.0% WRONG (table: +) | +0.0%  | +0.0%  | +0.8% unlisted |
| final_drive | +1.0% WRONG (table: -) | -0.3%  | -4.6% unlisted | +0.0%  | +0.0%  | +0.4%  |
| first_gear | -0.5% unlisted | -0.0%  | +0.0% WRONG (table: +) | +0.0%  | +0.0%  | +0.0%  |
| ground_clearance | -0.3%  | -0.0%  | +0.0%  | +0.0% WRONG (table: -) | +0.0% WRONG (table: +) | +0.1%  |
| mass | +6.2% ok | -0.1% WRONG (table: +) | -1.5% ok | -0.6% unlisted | +0.0%  | -0.9% unlisted |
| ride_frequency | +1.2% unlisted | -0.1%  | +0.0%  | +0.7% unlisted | +0.0%  | +0.8% WRONG (table: ~0) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | +8.1% ok | +0.0%  | +9.4% WRONG (table: ~0) |
| tyre_mu | +0.0%  | -2.7% ok | +0.0%  | +0.0%  | +0.0%  | +0.7% WRONG (table: ~0) |

### mule_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.8% ok | +1.9% unlisted | +0.0%  | +0.0%  | +0.0%  |
| com_height | -0.0%  | +1.4% ok | -1.9% unlisted | +0.2% ok | +4.7% unlisted | -3.5% ok |
| engine_power | -9.3% ok | -0.1%  | +0.0% WRONG (table: +) | +0.0%  | -51.2% unlisted | -0.0%  |
| final_drive | -2.2% ok | +0.8% unlisted | +0.9% unlisted | +0.0%  | +9.3% unlisted | -0.0%  |
| first_gear | +0.7% unlisted | +0.0%  | +0.9% ok | +0.0%  | -39.5% unlisted | +0.0%  |
| ground_clearance | -0.1%  | -0.0%  | +0.0%  | +0.0% ok | +2.3% ok | -0.0%  |
| mass | +7.5% ok | -0.5% WRONG (table: +) | -1.9% ok | +0.0%  | +0.0%  | +0.0%  |
| ride_frequency | +0.0%  | -0.1%  | +0.0%  | -0.1%  | -46.5% unlisted | +0.0% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | -0.2% ok | +0.0%  | +0.3% WRONG (table: +) |
| tyre_mu | +0.0%  | -1.9% ok | +4.7% unlisted | +6.6% unlisted | +0.0%  | +7.1% ok |

### scout_4x4

| lever | B1 | B4 | B6 | B7 | B11 | B12 |
|---|---|---|---|---|---|---|
| brake_capacity | +0.0%  | -5.3% ok | +0.0%  | +0.0%  | +0.0%  | +0.0%  |
| com_height | +0.2%  | +1.5% ok | +0.0%  | +0.0% ok | +0.0%  | -34.1% ok |
| engine_power | -9.3% ok | +0.1%  | +10.6% ok | +0.0%  | +7.1% unlisted | -0.1%  |
| final_drive | -3.0% ok | +0.1%  | +10.6% unlisted | +0.0%  | +7.1% unlisted | -0.1%  |
| first_gear | -1.6% unlisted | -0.1%  | +10.6% ok | +0.0%  | +7.1% unlisted | +0.0%  |
| ground_clearance | -0.4%  | +0.0%  | +0.0%  | +0.0% ok | +0.0% WRONG (table: +) | -0.0%  |
| mass | +7.3% ok | -0.4% WRONG (table: +) | -8.5% ok | +0.1%  | +0.0%  | +0.1%  |
| ride_frequency | +0.0%  | -0.3%  | +0.0%  | +0.0%  | +0.0%  | -33.7% WRONG (table: +) |
| track_gauge | +0.0%  | +0.0%  | +0.0%  | -0.3% ok | +0.0%  | +0.6% ok |
| tyre_mu | +0.0%  | -1.2% ok | +0.0%  | +6.5% unlisted | +0.0%  | +8.0% ok |

**Right signs: 37 of 54 scored (69%).**

Dead levers (moved no runnable benchmark): [("hauler_4x4", "ground_clearance"), ("mule_4x4", "track_gauge"), ("scout_4x4", "ground_clearance")]

Orphan benchmarks (no lever moved them): []

## What the wrong signs and dead levers say (each is for the lane that owns the model; none is "fixed" here)
1. **Mass does not lengthen a brake-limited stop (all three, -0.1% to -0.5%; table: `+`).** FORGE's brake lever is `service_decel_g`, a deceleration, which does not depend on mass by construction. The table's reasoning assumes a brake *torque* (more mass, same torque, less deceleration). Either the lever should be a torque (a real brake is) or the table row is wrong for this model. For FORGE and DRIVE.
2. **Torque does not help gradeability on the Hauler and the Mule (power and first gear: 0.0%; table: `+`).** Both are well below the traction limit (`mu f` = 0.85 to 0.9; Hauler 0.38, Mule 0.63), yet more engine torque or a lower first gear changes nothing; the Scout (0.28) responds fully (+10.6%). Something other than torque or traction limits those two (launch or converter logic). The ledger regime check cannot explain it. For DRIVE and ARCH.
3. **Ground clearance is nearly a dead lever.** It moves the step climb only on the Mule (+2.3%), and not the Scout or the Hauler (0.0%; table: `+`); on the Hauler, which tips (`mode = roll`), it does not raise the side-slope limit's centre of mass either (0.0%; table: `-`), because `ground_clearance_m` and `com_height_m` are separate fields in the `VehicleDef`. The step test also has no belly or ground collision (ARCH gaps, item 4). For FORGE and WORLD.
4. **A stiffer ride cuts the Scout's skidpad limit by a third (-33.7%; table: `+`) and does nothing to the Mule's (0.0%).** A 10% frequency change should not move the limit by that much: a suspension-travel or roll-stiffness effect to explain from the ledger. For CHASSIS.
5. **The Hauler's skidpad is labelled power-limited but moves with track (+9.4%) and centre-of-mass height (-7.8%).** Either the label is wrong or the limit is roll-related. For CHASSIS and ARCH.
6. **A numerically higher final drive slows the Hauler's launch (+1.0%; table: `-`)** (DRIVE: shift schedule).
7. **The Mule's track gauge moves nothing** (dead on this vehicle): it slides before it tips and its skidpad is grip-limited. That is correct physics, so the regime override says `~0` is right; it is listed because a lever with no effect anywhere on a vehicle is always reported.
8. **Large unlisted effects on the step climb** (engine power -51% and first gear -39% on the Mule; ride frequency -47%): the step test is decided by run-up momentum (ARCH gap 4), not by the quasi-static traction limit the table assumes. The table needs momentum rows, or the test needs a controlled approach speed.

## Orphan benchmarks
None: every benchmark that was run is moved by at least one lever.

## About the regime rules
The first run scored 65%. Three table rows were regime-dependent in a way the table did not say: the side-slope formula applies only to a vehicle that tips (the Mule and Scout slide first), and a power-limited skidpad cannot depend on chassis levers. The "Regimes" section in `IMPACT-MATRIX.md` says so, from the physics; the rows without a regime keep their signs. That moved the score from 65% to 69%; most of the remaining 17 wrong signs are items 1 to 6.
