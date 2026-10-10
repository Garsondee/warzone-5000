# Lane VALIDATION: the truth lane: is the simulated vehicle like the real one, and do design choices matter?

## Mission
When you have succeeded, for each reference vehicle there is a dossier of real figures with their sources and uncertainty, a harness that runs the simulated vehicle through the same tests the real one was subjected to, and a dashboard of traffic lights (green, amber, red, honestly red where the model is wrong),
plus a Design Impact Matrix that proves every design lever changes some outcome by a plausible amount. You never tune the model; you measure it and say what you see. The owner will see the dashboard, a tornado chart of what each lever moves, and for each real vehicle a table: published value, simulated value, band, verdict.

## You own  (the CI lane guard enforces it)
`crates/w5k_validate/**`, `content/dossier/**`, `docs/validation/**`, `crates/w5k_tools/src/cmd/validation.rs`; always `docs/swarm/status/validation.md`, `docs/swarm/requests/validation-*.md`, `docs/theory/validation.md`, `docs/lanes/validation/**`, `spikes/validation/**`.

## You read, never edit
`crates/w5k_contract` (`Param`, `Provenance`, `VehicleDef`, `VehicleModel`, `Command`, `Frame`, `CapabilityTable`), `docs/decisions/ADR-0007-validation-method.md`, `docs/validation/METHOD.md` and `IMPACT-MATRIX.md` (ARCH's first drafts; they are yours to improve from now on), `w5k_sim`'s scenario runner.

## Stand-ins you start on
`RigidBoxVehicle` over `FlatPlane` and `BumpStrip` as the "model under test" (it will fail the physics checks, which is the point: a pipeline that reports red for a fake model is a pipeline that works), `ScriptedCommands`, `truck_over_bumps()`.

## Settling round (first hours; then stop for review)
1. **Spike S9, data availability and reachability** (`docs/lanes/validation/spike-s9.md`): can a lane session reach the sources? Build dossiers for the M998 HMMWV, M113A3, M4A3 Sherman, M1A1 Abrams and Leopard 2A5 with at least 25 quantities each across: static (mass, dimensions, wheelbase and track or track length and width, ground pressure, clearance, estimated centre-of-mass height), powertrain (power, peak torque and rpm, gear ratios, speed per gear, governed top speed, 0-32 km/h time, fuel and range), dynamics (braking from 32 km/h, turning radius or pivot, estimated ride frequency, turret 360-degree time, gun elevation limits), mobility (gradient, side slope, trench, vertical step, fording depth). Every figure has `{value, band, provenance, source (title, page or URL)}`. Public data is uneven (an Abrams is quoted between 62 and 73 t; gear ratios and torque curves are mostly unpublished): say so, widen the band, mark ESTIMATE. **Kill criterion:** fewer than 15 sourced quantities for a vehicle means widen the reference set or ask the owner to drop manuals into `content/dossier/sources/`.
   Good sources, in order: US Army technical manuals and field manuals (TM 9-2320-387-10 for the HMMWV is the operator manual; public-domain status to be checked per document), manufacturer data sheets, published test reports, Wong's *Theory of Ground Vehicles* for soil and track maths, open vehicle models such as Project Chrono's HMMWV and M113 parameter sets as plausibility cross-checks (read for numbers, never imported), then Wikipedia only as a cross-check for headline figures. Never Jane's paywalled data; never figures from memory.
2. **Design note:** the dossier RON schema (use `Param`; add `unit`, `category`, `notes`); the scenario catalogue (standing start to speed, top speed, braking, gradient climb, side slope, step, trench, fording, pivot turn, turret traverse) as scripts using `ScriptedCommands` and `WorldQuery` constructions; how the harness measures each; the Monte Carlo over ESTIMATE inputs (seeded `Pcg32`) and the envelope; the tolerance classes of ADR-0007; the dashboard layout; the impact-matrix runner (finite differences at +-10%) with negative controls.
3. **CCRs** you expect: a way for a `VehicleModel` to expose telemetry the harness needs; scenario terrain constructors.

## Build order  (named tests)
1. Dossier loading: `dossier_ron_round_trips_and_every_quantity_passes_param_check`, `every_dossier_quantity_has_a_source_or_is_an_estimate_with_a_band`, `no_figure_is_duplicated_with_a_different_value`.
2. Scenarios and measurement on the stand-in model: `standing_start_scenario_measures_time_to_32kmh_within_one_tick`, `braking_scenario_measures_distance_from_the_driven_trace`, `gradient_scenario_finds_the_threshold_by_bisection`.
3. Verdicts: `tolerance_classes_match_adr_0007`, `a_value_inside_the_envelope_and_within_tolerance_is_green`, `mobility_limit_bracketing_is_published_le_sim_le_1_25_published`, `harness_reports_red_for_a_deliberately_wrong_model` (a model with twice the power must fail the top-speed check: **negative control**).
4. Monte Carlo and envelopes: `monte_carlo_is_deterministic_for_a_seed`, `envelope_widens_when_a_band_widens`.
5. Impact matrix: `impact_matrix_flags_a_dead_lever` (negative control: a model that ignores a lever), `impact_matrix_flags_an_orphan_effect`, `signs_agree_with_the_expected_table` (the table in `IMPACT-MATRIX.md`), run on the real model as it lands.
6. Fuzz: `w5k validation fuzz --designs 1000` (random in-range sliders through FORGE's compile: reject with a reason or run NaN-free and stable).
7. Dashboard: `w5k validation dashboard --out DIR` writes one self-contained HTML page (no external URLs, inline SVG): traffic lights per vehicle and quantity, tornado charts, the matrix, a provenance meter (how much of each vehicle is SPEC, MEASURED, ESTIMATE, TUNED); `dashboard_is_self_contained`.

## Acceptance for M1
Dossiers for M998, M113A3 and M4A3 (each at least 25 quantities, most with sources), the harness and dashboard v0 running on the stand-in and then on whatever the physics lanes have landed, impact matrix v0 with the sign table, negative controls passing, nightly CI job wired (`w5k validation nightly`). The owner will see: the dashboard, one vehicle's published-versus-simulated table, and the tornado of what each lever moves.
M2: held-out scoring (M1A1, Leopard 2A5, T-72B, M35, Tiger II): median error at most 12% and maximum 25% over at least 20 quantities.

## Theory to explain in `docs/theory/validation.md`
Calibration set versus held-out set (train and test: tuning on the test set is cheating); envelopes from uncertain inputs (Monte Carlo); sensitivity analysis and why a tornado chart is the right picture; provenance (asset versioning for numbers); why negative controls matter (a test that cannot fail proves nothing).

## Non-goals
Tuning the model (physics lanes tune global constants only, logged; you report); inventing numbers; UI polish; the vehicle designs themselves (FORGE).

## Needs from others / gives to others
Needs: the model APIs and rigs from the physics lanes and FORGE; the PNG plotter from VIEWER (SVG inline until then). Gives: dossier numbers to FORGE, GEOMETRY, CHASSIS, DRIVE and TRACKS; the scorecard and the dashboard to ARCH and the owner.

## Tripwires specific to this lane
A figure from memory; a source you could not open (mark it UNVERIFIED, do not cite it as if read); tuning anything per vehicle; hiding or softening a red; a dashboard needing the network; a tolerance changed without a card.

## Owner's answers that apply to you (2026-10-08, cards C-001 and C-002)
- Dossiers stay real vehicles: that is where published numbers live. The game never shows real names, so keep the mapping from a dossier to a game vehicle's fictional id in `content/dossier/`, not in the vehicle files.
- The garage should end up covering every role (recon, personnel carrier, light, medium, heavy and main battle tank, assault gun or tank destroyer, self-propelled artillery, recovery, utility truck), not only assault vehicles. After the first three dossiers, spread the next ones across roles and eras (very late WW2 to today); it is the vehicle builder that is being exercised.

## Owner's direction 2026-10-10 (card C-020, `docs/architecture/DESIGN-MODEL.md`)
- Mass breakdown and envelope thresholds need sources: dossiers should carry the component masses of the reference vehicles (engine, transmission, armour, weapon) so FORGE's budget can be checked, and each envelope flag's threshold needs a citation or an `ESTIMATE` tag. The Impact Matrix levers `mass`, `com_height`, `tyre_friction` stay as **probes**, not player levers.

## Done
M1 acceptance passes in CI, the theory note is written, the status file has the handoff note, you have idled; M2 held-out scoring is a follow-up task.
