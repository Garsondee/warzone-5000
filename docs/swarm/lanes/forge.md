# Lane FORGE: from a designer's choices to a vehicle the solver can run

## Mission
When you have succeeded, a `VehicleDef` written the way a designer thinks (ride frequency, damping ratio, gear spread, peak power and torque, brake capability, wheelbase, masses) compiles into a `PhysRig` and a `RenderRig` whose numbers are consistent with each other and with the design, and the first reference vehicle, the M998 HMMWV, is a parametric build that mimics the real one with every figure carrying its provenance.
You are the bridge that makes design choices matter: each slider must turn into the right solver parameter. The owner will see the HMMWV's design sheet, its compile report (what each slider became) and the coherence checks passing.

## You own  (the CI lane guard enforces it)
`crates/w5k_forge/**`, `content/parts/**`, `content/vehicles/**`, `crates/w5k_tools/src/cmd/forge.rs`; always `docs/swarm/status/forge.md`, `docs/swarm/requests/forge-*.md`, `docs/theory/forge.md`, `docs/lanes/forge/**`, `spikes/forge/**`.

## You read, never edit
`crates/w5k_contract` (`VehicleDef` and everything in `def.rs`, `PhysRig`, `RenderRig`, `Param`), `crates/w5k_geo` (a library: its generators and closed-mesh mass properties), `docs/architecture/*`, `docs/validation/METHOD.md`, the M998 dossier when VALIDATION delivers it.

## Stand-ins you start on
`dummy_vehicle_def()` (a `VehicleDef` that checks clean), `box_truck()` and `box_tank()` (valid rigs to compare against), the meshes of `testing::rigs::{box_mesh, cylinder_mesh}` in place of GEOMETRY's until they land.

## Settling round (first hours; then stop for review)
**Contract 0.1.1 and `VehicleDef`.** The rig side is settled (`docs/architecture/redteam/wheeled.md`, `tracked.md`, `weapons.md`, `docs/architecture/CONTRACT-0.1.1-PLAN.md`). `def.rs` is not: your first CCR carries the `VehicleDef` additions the red-teams found, with their sketches as the baseline (`docs/architecture/redteam/sketches/`): per-axle layout (`Independent`, `Solid { spring_track_m }`, `Tandem`), dual tyres and portal hubs, per-axle tyre, unsprung mass and brake share; tracked bogies, return rollers, idler and tensioner, belt thickness and length, `shoe_mu_scale_soft`, wheel-contact stiffness, volute springs, steering-unit laws; guns, mounts and aim channels (`GunDef` projectile mass, recoiling mass, recoil factor, reload), turret ring position; and armour sliders (with COMBAT). Compile rules to implement and test: `mu_scale = VehicleDef mu_peak_ref / mu_peak of the table's dry hard reference surface`; `ride_height_m` and the free wheel radius (static deflection = load / vertical stiffness, validated 0 to 0.35 radius); preloads that carry the sprung weight (`validate()` checks 5%); Table springs as total forces; linkages for bogies, tandems and inboard leaf springs; track loop order from the sprocket; substeps from the stiffest resolved mode. `PhysRig::validate()` is strict: your compile output must pass it.
1. **Spike S5, design-to-physics coherence** (`docs/lanes/forge/spike-s5.md`): compile the hull, wheel, suspension and engine of the HMMWV from a hand-written `VehicleDef`; verify the masses sum, the wheel radii agree between rig and mesh to 1 mm, and the static equilibrium sits at the design ride height. Any failure is a contract problem to fix **before the M1 freeze**.
2. **Design note:** the compile pipeline stages; each slider's mapping, with the formula: ride frequency to spring rate (`k = m_corner (2 pi f)^2 / MR^2` for motion ratio `MR`), damping ratio to damper (`c = 2 zeta sqrt(k m)`), anti-roll; gear spread to ratios (first gear from a launch requirement, top gear from top speed at redline, a geometric progression between); final drive from top speed; torque-curve shape from the peaks (templates per engine kind scaled by peak torque and power and their rpm); brake torque from `service_decel_g` and axle loads; the mass breakdown (structure, engine, transmission, fuel, crew, ammunition, turret) and how it composes into the hull's centre of mass and inertia tensor (parallel-axis theorem), using GEOMETRY's closed-mesh integrals where meshes exist; the rejection rules ("numerically unstable design" when the rig needs too many substeps, with the reason).
3. **CCRs** you expect: `VehicleDef` field additions, `PhysRig` additions found by S5.

## Build order  (named tests)
1. `VehicleDef` checking and RON round trip on real files; `compile_is_deterministic` (same def, same rig bytes).
2. Hull and masses: `masses_sum_to_the_def_mass`, `com_matches_the_def_position`, `inertia_tensor_is_positive_definite_and_obeys_the_triangle_inequalities`, `parallel_axis_theorem_matches_a_two_box_hand_calculation`.
3. Sliders: `ride_frequency_slider_gives_the_stated_natural_frequency` (compute `f` back from the rig to 0.5%), `damping_ratio_slider_gives_the_stated_zeta`, `static_preload_carries_the_sprung_weight_share_within_1_percent`, `gear_ratios_follow_the_stated_spread_and_reach_top_speed_at_redline`, `torque_curve_peaks_match_the_def_peaks`, `brake_torque_gives_the_stated_service_decel_on_the_axle_loads`.
4. Both rigs: `rig_validates`, `render_rig_joint_layout_matches_the_rig`, `wheel_radius_in_rig_equals_tyre_diameter_over_two`, `track_and_station_indices_are_consistent` (for the tracked defs of M2).
5. Rejection: `fuzz_1000_random_defs_either_reject_with_a_reason_or_compile_and_validate` (sliders in range; no panics, no NaN).
6. The reference garage: `content/vehicles/reference/m998.ron` (`reference: Some("m998")`), every `Param` marked SPEC or MEASURED where the dossier has it, ESTIMATE with a band otherwise; M1 is the HMMWV; M2 adds the M113A3, M4A3 Sherman, M1A1, Leopard 2A5, T-72B, M35 and Tiger II (at least five by M2). `w5k forge compile <def> --out DIR` writes the rig JSON and a human-readable compile report.

## Acceptance for M1
The tests above pass on Linux and Windows; the HMMWV compiles to a valid `PhysRig` and `RenderRig`, and the first-light scenario can drive it (ARCH swaps the stand-in rig). The owner will see: the HMMWV's design sheet (a one-page table of each design number and its provenance) and the compile report showing what each slider became.

## Theory to explain in `docs/theory/forge.md`
Design levers versus solver parameters (the whole point of the project); `k = m (2 pi f)^2` and why ride frequency is the designer's handle on a spring; the parallel-axis theorem (graphics analogy: moving a pivot in a rig changes how hard it is to swing, and mass far from the axis counts squared); geometric gear progressions; provenance as asset versioning.

## Non-goals
New component families beyond the reference garage (after M2); sci-fi anything; the voxel kernel of the prototype (GEOMETRY's closed-mesh integrals replace it; revive it only if the design note shows they cannot do the job); UI.

## Needs from others / gives to others
Needs: GEOMETRY's shapes and mass integrals; VALIDATION's dossier numbers; CHASSIS, DRIVE and TRACKS to say what they read from the rig. Gives: compiled rigs to everyone, the reference garage to VALIDATION, the compile report to the owner.

## Tripwires specific to this lane
A slider that has no effect on any rig field (a dead lever: report it); a rig field set from a bare constant; a figure for a real vehicle written from memory instead of the dossier; a compile that silently clamps an out-of-range slider (reject with a reason instead).

## Done
M1 acceptance passes in CI, the theory note is written, the status file has the handoff note, you have idled.
