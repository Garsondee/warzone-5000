# FORGE to VALIDATION: the lever API for the Design Impact Matrix

`w5k_forge::levers::apply(&VehicleDef, lever: &str, factor: f64) -> Result<VehicleDef, String>`, then `w5k_forge::compile::compile(&def, &extras)`. `LEVERS` lists every lever (name, what it scales, which compiled number moves). Wheeled vehicles only; a tracked def returns an error until the tracked compile exists.

- A perturbed `Param` may leave its authored band: the band is widened to include the new value (it is an experiment, not a design claim). Provenance and source are kept.
- `factor` must be finite and positive. An unknown lever lists the valid names. A perturbation the physics cannot honour compiles to an error with a reason (for example `brake_torque` above the tyre's friction, `engine_peak_power` pushing an axle ratio out of band).
- Each lever is tested (`crates/w5k_forge/tests/levers.rs`) on scout, mule and hauler at x1.1 and x0.9: the compiled number moves by exactly the factor given in the table; and every lever at x0.3 to x3 either compiles to a rig that passes `validate()` or fails with a reason.

| lever | what it scales | compiled number that moves |
|---|---|---|
| `engine_peak_power` | peak power and peak torque together (same curve shape) | peak torque x omega, x f |
| `torque_peak_rpm` | both rpm peaks, power following (same torque curve moved along rpm) | rpm of torque maximum, x f |
| `final_drive` | final drive ratio | axle differential ratio, x f |
| `first_gear` | first forward ratio | x f |
| `brake_torque` | `service_decel_g` | total brake torque capacity, x f |
| `brake_thermal_mass` | thermal mass | x f |
| `mass` | hull mass | hull mass x f; springs, dampers and preloads follow (ride frequency is a slider and holds) |
| `com_height` | COM height | x f |
| `ground_clearance` | clearance | hull underside height x f |
| `wheelbase` | axle span about the front axle; COM keeps its fraction; hull length keeps the rear overhang | station row spacing x f |
| `track_gauge` | track width of every axle | lateral station offset x f |
| `ride_frequency` | front and rear ride frequency | ride rate x f^2 |
| `spring_rate` | ride frequency by sqrt(f) | ride rate x f |
| `damping` | damping ratio | mean damper coefficient x f |
| `suspension_travel` | bump and droop travel | x f |
| `tyre_width` | section width | wheel width x f, contact patch length x 1/f |
| `tyre_pressure` | inflation pressure | contact patch length x 1/f |
| `tyre_friction` | `mu_peak_ref` | tyre `mu_scale` x f |
| `frontal_area`, `drag_coeff` | aero | x f |

## Matrix rows with no lever yet (and why)
- **Fuel tank capacity**: `VehicleDef` has no fuel-tank field (DRIVE owns fuel use; a tank needs a mass-item breakdown, the deferred M part of the CCR).
- **Gear count (closer ratios, same span)**: needs the `GearboxSliders::design` form (W6) compiled; today the gear list is explicit.
- **Track contact length, track gauge (tracked), track width, gun mass, turret mass, traverse effort, stabiliser rejection**: the tracked and turret compiles are not built.
- **Tyre pressure** only moves the contact patch: the tyre's vertical stiffness is its own number in the def (`vertical_stiffness_n_m`), so a lower pressure does not soften the tyre. If the matrix expects that, VALIDATION or ARCH should say so and FORGE will couple them with a stated law (a CCR for the coefficient).
- **Engine levers keep the curve's shape on purpose**: scaling only the power (or only one rpm peak) changes the shape and can make the two peaks inconsistent; the compile then rejects with the reason ("power would peak earlier than stated").

## Answers to the first Impact Matrix run (ARCH, impact-v0.md)
1. **Ground clearance.** What it changes in the compiled rig today: `ride_height_m` (the datum is `clearance + height / 2` above the ground) and the underside of the hull collision proxy, which sits exactly `ground_clearance_m` above the ground at the design pose; the compile report now also prints the obstacle angles it implies (approach, departure, ramp breakover). It does **not** move the COM (`com_height_m` is its own number, kept by the lever) and no belly plate is emitted. So a step or ridge test sees clearance only through the hull proxy's contact with the ground, which is the solver's call (CHASSIS and WORLD's queries); if their step test ignores the hull proxy on two of three vehicles, the lever is dead in that test, not in the compile. If the matrix wants a plough or belly-drag effect, the rig has `ProxyRole::Belly` and a `BellyDrag` ledger term, and the def needs a belly-plate extent (a def field FORGE would then compile into a Belly proxy): say so and I will put it in the next CCR.
2. **Brake lever.** The def states brake capability as a deceleration (`service_decel_g`), so the compile sizes torque as `m a r`. For a mass perturbation that would make heavier vehicles brake just as hard, which is wrong. Default taken now, inside the lever API: the **`mass` lever holds the total brake torque constant** by scaling `service_decel_g` by the inverse of the total-mass ratio (the achievable deceleration then falls with mass; test `the_mass_lever_holds_the_brake_torque_so_a_heavier_vehicle_brakes_less_hard`). `brake_torque` still scales the capacity itself. If DRIVE or ARCH prefer the def to carry brake torque per axle (`brake_torque_nm`), say so: it is an additive CCR and the compile change is small; until then the lever API gives the runner the right behaviour without a contract change.
