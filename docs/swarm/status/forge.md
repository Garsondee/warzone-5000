# Status: FORGE

*Update with every PR.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/forge/build (then lane/forge/compile) | **Contract pinned:** `contract-v0.1` = commit `f8f5e5d` | **Phase:** building (owner instruction in force: no waiting for review)

## Done
- Settling round PR (this branch): spike S5 (`crates/w5k_forge/src/s5.rs`, `docs/lanes/forge/spike-s5.md`), design note (`docs/lanes/forge/design-note.md`), `VehicleDef` CCR as text (`docs/swarm/requests/forge-ccr-vehicledef.md`), theory starter (`docs/theory/forge.md`), image `docs/lanes/forge/media/s5-torque-curve.png`.

## In progress
- PR A (this branch): `curve.rs` (engine curve through the two peaks, promoted from spike S5), `extras.rs` (PROVISIONAL(CCR-forge) sidecar types), the first truck `content/vehicles/game/mule_4x4.ron` + `.extras.ron` (fictional "Mule 4x4", all ESTIMATE with bands, M998-like archetype, no real figures from memory).
- PR B (this branch `lane/forge/compile`, stacked on A): `compile.rs`, the wheeled compile of the truck to `PhysRig` (validated) with the report; 15 physics-sentence tests incl. determinism and a 1000-def fuzz.
- PR C (next): `render.rs`, the `RenderRig` for the same stations (split to stay under 400 non-test lines); the S5 spike is already removed.

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
