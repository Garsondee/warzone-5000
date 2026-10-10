# Status: FORGE

*Update with every PR.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/forge/settling | **Contract pinned:** `contract-v0.1` = commit `f8f5e5d` | **Phase:** settling (stopped for review)

## Done
- Settling round PR (this branch): spike S5 (`crates/w5k_forge/src/s5.rs`, `docs/lanes/forge/spike-s5.md`), design note (`docs/lanes/forge/design-note.md`), `VehicleDef` CCR as text (`docs/swarm/requests/forge-ccr-vehicledef.md`), theory starter (`docs/theory/forge.md`), image `docs/lanes/forge/media/s5-torque-curve.png`.

## In progress
- Nothing: stopped for review as the brief says.

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
- None directly. Applied from the brief: owner's answers C-001 and C-002 (fictional game ids; the reference garage spans all roles; new components as data).

## Handoff note (fill in when you stop)
- Not yet: the lane continues to M1 after review.
