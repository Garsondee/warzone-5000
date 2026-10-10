# FORGE design note: mass as a consequence (D2a the component mass budget, D2b the design audit)

Source: `docs/architecture/DESIGN-MODEL.md` (owner direction, card C-020; read it first). This note says how the compile gets there with small PRs, and what I decided where the doc leaves room. PROVISIONAL(C-020, C-021): the budget lives in the extras sidecar until contract 0.4 graduates it with the other extras.

## 1. The rule in one line
`total mass = structure + sum of the parts`, each part's mass a function of the choice that sizes it, each part with a position, so **mass, COM, inertia, axle loads, spring rates and ground pressure all follow the configuration**. The authored `hull.mass_kg` stops being the truth: it becomes the *design-point total* the structure is calibrated against.

## 2. D2a: the component mass budget (extras, `mass_budget`)
| Part | Mass model (a `Param` each: value, band, source, ESTIMATE) | Position | Sized by |
|---|---|---|---|
| Engine | `specific_mass_kg_per_kw x peak_power_w` (catalogue by engine type later; diesel truck about 2 to 4, petrol about 1 to 2, gas turbine lower, electric by motor and pack) | x, y, z in the hull frame (from front, height above ground, lateral) | `engine.peak_power_w` |
| Transmission (gearbox, transfer case, steer unit) | `kg_per_nm x peak_torque_nm` (torque-linked) | position | peak torque |
| Final drives (per axle) | `kg_per_nm x (peak_torque x first_gear x final_drive x axle share)` (the torque the axle must carry) | at each axle | torque, ratios |
| Fuel | `tank_litres x density_kg_l x fill_fraction + tank_kg_per_litre x tank_litres` (curb and combat fill are two named fractions; the compile uses `loading`) | position | tank size |
| Crew | `count x kg_each` | position | count |
| **Structure** | authored `structure_mass_kg` and `structure_position_m`: the hull, armour (until D3), body and everything not in the list above | position | the hull's dimensions (later: the loft's area, D3) |

Compile rule: sprung mass = structure + parts. **COM** = mass-weighted mean of the positions (a small helper shared by the wheeled and tracked compiles). **Inertia** = the structure as a uniform box about its own centre (as today) plus each part as a point mass by the parallel-axis theorem (`I = sum m (|d|^2 1 - d d^T)`), which replaces today's single lump. Unsprung and rigid-station masses are untouched. Everything downstream already follows the sprung mass: axle loads by statics, spring rates (`k = m (2 pi f)^2`), dampers, preloads, ride height and the tyre sink.

**Baseline stays put.** For each of the four vehicles the extras carry `structure_mass_kg` and `structure_position_m` *derived once* so that structure + parts reproduce the old all-in `hull.mass_kg` and COM exactly (source text says so: "design point: stated mass minus the budget"). Total mass and COM, hence axle loads, preloads, spring rates and ride height, are identical at the baseline. The hull **inertia tensor changes slightly** (a box plus point masses against one uniform box): this is a deliberate model improvement, stated in the PR with the before and after, with a Golden-Change line if any golden moves. Absent `mass_budget`: the old behaviour (`hull.mass_kg` lump), so nothing else breaks.

**What moves when a choice moves.** `engine_peak_power` (the lever scales power and torque) now changes engine, transmission and final-drive masses, so total mass, COM, axle loads and spring rates move; fuel tank size moves mass and COM; the Workshop's engine slider finally has a price. The Impact Matrix keeps `mass` as a probe: with a budget it perturbs **structure** mass (documented in the lever list); it is not a player lever.

**Rejections.** Only what the solver cannot represent: a negative or zero part mass, a COM outside the support (the existing statics error), a missing position. A heavy engine on a light hull builds and is flagged (D2b).

## 3. D2b: the design audit (`w5k.audit.v1`, JSON)
`w5k_forge::audit::audit(&VehicleDef, &Extras, &Compiled) -> Audit`, written by `w5k forge audit <def> --out FILE`. Each entry: `{ id, label, value, unit, because, provenance, band? }`; `provenance` is the weakest provenance among the inputs the entry used (SPEC, MEASURED, ESTIMATE, TUNED), `band` is propagated where inputs have bands. Groups: **mass** (by system: structure, engine, transmission, final drives, running gear, fuel, crew; sprung, unsprung, total), **balance** (COM xyz, axle shares, inertia diag), **ground** (nominal ground pressure from the tyre patch or the track footprint, wheel load against tyre rating, obstacle angles), **power** (power-to-weight, torque-to-weight), **ride** (ride frequency, damping ratio, static suspension margin), **stability** (static stability factor). Each a read-only consequence with a one-line "because" naming the formula and the choices it came from.
**Flags**: `{ id, level: amber | red, cause, thresholds, lever }` from the table in DESIGN-MODEL.md, each threshold a `Param` in `content/parts/envelope.ron` (ESTIMATE with source, so it is data and can be tuned or cited later): power-to-weight against a minimum, ground pressure against a soil bearing figure (cohesive and sandy reference soils from WORLD's table), wheel load against the tyre rating (a `rated_load_n` extra), static suspension margin (travel left at rest), static stability factor against plausible lateral g, obstacle angles, and where the model has no number yet a flag says **"not modelled"** rather than staying silent (cooling, driveline torque rating, brake heat soak: later PRs add them as their inputs exist). A flag never blocks a build.

## 4. PR plan (each under 400 non-test lines)
1. This note.
2. **D2a-1**: `mass_budget` extras, the shared COM and inertia helper, the wheeled compile on it; the three trucks calibrated (baseline total mass and COM identical, tested); lever updates (`mass` probes structure); tests: `baseline_total_mass_and_com_are_unchanged`, `engine_power_moves_mass_com_axle_loads_and_spring_rates`, `parallel_axis_matches_a_hand_calculation_with_parts`, `fuel_tank_size_moves_mass_and_com`, rejections.
3. **D2a-2**: the tracked compile on the same budget (carrier), a `loading` switch (curb or combat).
4. **D2b-1**: `audit.rs` (mass, balance, power, ride, stability groups) + `w5k forge audit` + the JSON schema doc; **D2b-2**: ground, tyre rating, obstacle angles, flags and `envelope.ron`.
5. After D2a-1 lands: tell VALIDATION (via ARCH) to rerun the matrix.

## 5. Decisions taken by default (reversible; say so and I change them)
- Parts' masses are linear in their sizing variable (specific mass, kg per N m): the simplest honest law, stated in the `because` line; nonlinear scaling (a gearbox grows faster than torque) is a later refinement with its own source.
- Structure mass is authored per vehicle for now (D3 derives it from armour and the loft).
- Impact `mass` lever probes structure mass; `engine_peak_power` is the price-bearing lever.
- Audit provenance is the weakest input's; bands propagate by interval arithmetic where the formula is monotone, else are omitted.
- Part positions are authored (ESTIMATE) until GEOMETRY's modules carry component volumes.
