# Spike S5: design-to-physics coherence

**Question.** Can a hand-written `VehicleDef` (hull, wheels, suspension, engine) compile into a `PhysRig` whose numbers agree with the design and with each other, and pass `PhysRig::validate()`? Any failure is a contract problem to fix before the M1 freeze.
**Answer.** Yes for the four checks the brief names, but only after finding twelve problems (table below: missing `VehicleDef` fields, one inconsistent stand-in, three decisions the contract text leaves open). Code: `crates/w5k_forge/src/s5.rs` (throwaway; M1 replaces it). Run: `cargo test -p w5k_forge`.

**Method.** The spike compiles the stand-in 4x4 (`dummy_vehicle_def()`; real-vehicle figures wait for VALIDATION's dossier, because writing an M998 from memory is a lane tripwire). The stations, hull, ride height, tyre, spring, damper and engine curve are rebuilt from the def; the drivetrain tree and brakes are grafted from `box_truck()` so that the *whole* rig is validated. `*Extras` structs in the spike stand for the missing def fields.

| Check (test name) | Result |
|---|---|
| Masses sum: `the_compiled_rig_validates_and_masses_sum_to_the_def` | `total_mass_kg = hull + 4 x unsprung` exactly; `validate()` passes on rig and render rig |
| Radii agree: `wheel_radius_in_rig_and_mesh_agree_to_a_millimetre` | rig radius = `outer_diameter / 2` (to 1e-12); the mesh's farthest vertex is within 1 mm (one source, two consumers) |
| Equilibrium at ride height: `static_equilibrium_sits_at_the_design_ride_height` | tyre deflection implied by the rig (`R - ride_height - rest.y`) times tyre stiffness equals preload plus unsprung weight within 1%; ground clearance, COM height and COM position match the def to 1 mm |
| Ride frequency: `ride_frequency_slider_gives_the_stated_natural_frequency` | `f` recomputed from the rig matches the slider within 0.5% **when the tyre is in series** (decision D1) |
| Damping: `damping_ratio_slider_gives_the_stated_zeta` | mean of bump and rebound gives the stated zeta (decision D2) |
| Engine: `torque_curve_peaks_match_the_def_peaks` | both peaks exact in value and rpm (`media/s5-torque-curve.png`) |

## Findings (each is a contract or content problem; the CCR file has the fields)
| # | Finding | Evidence | Fix |
|---|---|---|---|
| F1 | `TyreSliders` has no vertical stiffness, vertical damping, slip stiffness, relaxation length or wheel inertia; the rig needs all five | the compile had nowhere to read them | CCR W1 |
| F2 | `SuspensionSliders` has one damping ratio: no bump/rebound split, no bump-stop position or rate | rig `DamperDef`/`BumpStopDef` | CCR W2 |
| F3 | `EngineSliders` has no closed-throttle drag, and no idle-torque shape handle | rig `drag_*` | CCR W3 |
| F4 | `AxleDef` has no max steer angle or Ackermann | rig `SteerDef` | CCR W4 |
| F5 | **The contract stand-in def is physically inconsistent**: 110 kW at 3500 rpm is exactly 300 N m, equal to the torque peak, so power would still be rising there. `compile` rejects it with a reason. | test `the_contract_stand_in_def_has_inconsistent_engine_peaks_and_is_rejected` | `validate` rule in the compile; `dummy_vehicle_def` should use <= 100 kW (ARCH's file; asked in the CCR text) |
| F6 | The datum of the hull frame is undefined in `def.rs` (`com_height_m` is above the ground; the rig wants COM relative to a datum and `ride_height_m` of that datum) | needed to write `ride_height_m` | Decision D3: datum = hull box centre; the compile derives `ride_height_m = ground_clearance + height / 2` |
| F7 | A first torque template (a cubic through both peaks) **blew up on the low-rpm side** (torque 400 N m at idle for a 300 N m peak) | first spike run | two-piece curve (parabola below the peak, cubic above); the idle fraction is a template `Param` (F3) |
| F8 | Statics for more than two axles is indeterminate; the spike supports two | `front_share` | design note section 3: min-norm load share, linkages decide for tandems |
| F9 | `mu_scale` needs the table's reference surface `mu_peak`; it is an input to the compile, not data in the def | `ref_surface_mu_peak` | compile takes the terrain table (WORLD) as an argument |
| F10 | `patch_length_m` is *derivable* (contact area = load / inflation pressure, divided by width); it was one of the "missing" fields and now is not | 92 mm front, 82 mm rear | derived, no field |
| F11 | The ride rate with the tyre in series is 27% below the spring rate on the stand-in (spring 68.7 kN/m vs ride 53.9 kN/m): ignoring the tyre would be a visible error | report line `fl:` | decision D1 |
| F12 | Bump-stop engage fraction, rate and rebound ratio were bare constants in the first draft: tripwire "a rig field set from a bare constant" | review of the diff | now `Param`s (W2) |

## Decisions the spike forced (PROVISIONAL until ARCH or the owner objects)
- **D1. The ride-frequency slider is the *ride* frequency**: sprung corner mass on the suspension *in series with the tyre*, `k_ride = k_s k_t / (k_s + k_t)`, so `k_s = k_ride k_t / (k_t - k_ride)`. Rejected with a reason if `k_ride >= k_t`.
- **D2. The damping-ratio slider is the mean of bump and rebound damping**: `c_mean = 2 zeta sqrt(k_ride m)`, split by a `rebound_to_bump` ratio so that the mean is preserved.
- **D3. Hull datum = hull box centre**, +Z backward (front of hull at `z = -length/2`), as in the stand-in rigs.

## Result for the M1 freeze
No blocker in `PhysRig`: every failure was on the `VehicleDef` side, which is this lane's CCR. The rig contract already carries what the compile needs (`ride_height_m`, free radius, totals for spring forces, `mu_scale`).
