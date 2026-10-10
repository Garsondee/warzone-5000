# Status: FORGE

*Update with every PR.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/forge/render | **Contract pinned:** `contract-v0.2` = commit `80adac2` | **Phase:** building (owner instruction in force: no waiting for review)

## Done
- Settling round PR (this branch): spike S5 (`crates/w5k_forge/src/s5.rs`, `docs/lanes/forge/spike-s5.md`), design note (`docs/lanes/forge/design-note.md`), `VehicleDef` CCR as text (`docs/swarm/requests/forge-ccr-vehicledef.md`), theory starter (`docs/theory/forge.md`), image `docs/lanes/forge/media/s5-torque-curve.png`.

## In progress
- PR A (#18, merged): `curve.rs` (engine curve through the two peaks, promoted from spike S5), `extras.rs` (PROVISIONAL(CCR-forge) sidecar types), the first truck `content/vehicles/game/mule_4x4.ron` + `.extras.ron` (fictional "Mule 4x4", all ESTIMATE with bands, M998-like archetype, no real figures from memory).
- PR B (this branch `lane/forge/compile`, stacked on A): `compile.rs`, the wheeled compile of the truck to `PhysRig` (validated) with the report; 15 physics-sentence tests incl. determinism and a 1000-def fuzz.
- PR C (`lane/forge/render`, stacked on B): `render.rs`, the `RenderRig` for the same stations; tests for joint layout and the 1 mm wheel-radius agreement; image `docs/lanes/forge/media/mule-side-view.png` and the compile report `mule-compile-report.txt`.
- 2026-10-10: merged contract-v0.2 (pin 80adac2); extras sidecar slimmed to what def.rs still cannot state; compile reads W1-W4 from the def (a missing optional is a rejection naming the field) and uses the contract substep rule (4 substeps for the Mule).
- Next: `w5k forge compile <def> --out DIR` command, then hull mass items and the tracked compile.

## Merge order for ARCH
#18 is merged; #20 (base lane/forge/build) then #24 (base lane/forge/compile); retarget each base as the one below lands. API: `w5k_forge::compile::{parse_def, parse_extras, compile}` and `w5k_forge::render::render_rig(&c.rig, c.hull_size_m)`; files `content/vehicles/game/mule_4x4{,.extras}.ron`. (A direct message to ARCH was blocked by the lane tool guard, as it should be; this file is the channel.)

## Garage (2026-10-10, ARCH request for three trucks)
`scout_4x4` (1.3 t, 52 kW petrol, clutch, soft long-travel coils f 1.1/1.2 Hz, zeta 0.25, 0.70 m tyres) and `hauler_4x4` (6.2 t laden, 135 kW diesel, clutch, 5 gears, leaf springs modelled linear f 1.9/2.2 Hz, zeta 0.35, 1.05 m tyres, COM 1.35 m) beside the Mule; same 10-joint layout; `tests/garage.rs`. Their extras sidecars start from the Mule's (brake fade, cooling and converter numbers are generic and still carry the Mule's source text). The engine-curve template became a quartic (the cubic made power a local minimum at the power peak for the scout's and hauler's peaks). A 6x6 compiles in FORGE today with independent axles (tested); tandem and solid-axle linkages are not compiled yet; whether `WheeledChassis` runs three axles is for CHASSIS to say.

## Substep bake
The compile bakes `integration.substeps` and `f_max_hz` from the stop-engaged wheel hop (incremental stop rate at full bump travel, as CHASSIS PR 59 reads it) and the hull heave; test `declared_substeps_cover_the_stop_engaged_wheel_hop_of_every_station`.

## Scout damping (CHASSIS #76 finding)
The scout's mean damping ratio 0.25 with rebound/bump 1.5 meant only 0.20 on the bump side (what the whoops excite). Now zeta 0.30, rebound/bump 1.2: bump side 0.27, rebound 0.33. Slice course: scout peak az 13.43 -> 11.57 m/s2; mule and hauler unchanged.

## Levers (slice 2, stage A)
`levers.rs` + `docs/swarm/requests/forge-levers.md` for VALIDATION: 20 levers, each tested on the three trucks. The engine-curve template changed again to a torque-space power law `T = T_pk - c d^k` (concave, one power maximum by construction; both peaks exact); peaks that would need k < 1 are rejected as inconsistent.

## Matrix feedback (ARCH, impact-v0)
Ground clearance: compile report prints approach, departure and ramp breakover angles; the clearance effect and options are in `forge-levers.md`. Brake: the `mass` lever holds brake torque fixed (default; a def field is the alternative, offered in the request file).

## Differentials and brake torque (DRIVE request)
Provisional extras now carry centre and axle differential kind and bias and the authored axle brake torque; `apply_both` and `set_diff` expose them as levers; the CCR text for `def.rs` is in `forge-levers.md`. Authored torque makes mass lengthen stops.

## Tracked T1 (this PR)
`tracked.rs` (stations in loop order, TrackDef per side with the computed belt length, road-wheel springs, steer unit, sprocket brakes), `TrackedExtras`, `carrier_tracked` def + extras (M113-class archetype, 10.6 t, 65 links, 8 substeps), `tests/carrier.rs`. Next: T2 (belly proxy, steer law extras), T3 (render rig with track_runs, picture).
## Tracked compile (slice 2 stage B)
Design note: `docs/lanes/forge/tracked-design-note.md` (stations and loop order, belt, steer unit, belly proxy, extras, test list, PR plan T1 to T3). Contract pin v0.3 = 0982843.

## Blocked
- Nothing blocking the settling round. Later: `w5k_geo` mass integrals (stub today); VALIDATION's M998 dossier; CHASSIS' spike S1 (substep constant).

## Next
1. After review: ARCH applies the CCR to `def.rs` (contract 0.1.2); I adapt `s5.rs` to the real fields.
2. Build order 1 to 3 (def RON round trip, `compile_is_deterministic`, masses, sliders).
3. Both rigs, rejection fuzz, then the reference garage (fictional ids; dossier links live in VALIDATION's mapping).

## Cards needed / PROVISIONAL decisions in force
- No card. Decisions D1 (ride frequency includes the tyre in series), D2 (damping ratio is the bump/rebound mean), D3 (datum = hull box centre) are in the spike note; reversible by ARCH or the owner.

## Evidence
- `cargo test -p w5k_forge`: 8 tests pass, among them `ride_frequency_slider_gives_the_stated_natural_frequency`, `damping_ratio_slider_gives_the_stated_zeta`, `static_equilibrium_sits_at_the_design_ride_height`, `wheel_radius_in_rig_and_mesh_agree_to_a_millimetre`, `torque_curve_peaks_match_the_def_peaks`; the grafted rig passes `PhysRig::validate()`.
- Found: the contract's `dummy_vehicle_def()` engine peaks are inconsistent (spike F5).
- Not run: Windows CI (Linux only here). Golden and impact matrix: unchanged (no contract or content touched).

## Owner instructions received
- 2026-10-10 (via ARCH, verified in `docs/swarm/STATE.md` on integration): continue into the build steps without waiting for review; priority a truck-class 4x4 that compiles for CHASSIS and DRIVE; keep provisional types local, tagged PROVISIONAL(CCR-forge).
- None directly otherwise. Applied from the brief: owner's answers C-001 and C-002 (fictional game ids; the reference garage spans all roles; new components as data).

## Handoff note (fill in when you stop)
- Not yet: the lane continues to M1 after review.
