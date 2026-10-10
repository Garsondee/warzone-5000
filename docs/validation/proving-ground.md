# The proving ground: tests that model, simulate, and judge the vehicle and the terrain

Owner direction (via ARCH, 2026-10-10): tests that **model** a vehicle test, **simulate** it, and let us **judge** whether our vehicle model and our terrain are good.
Lane VALIDATION writes this spec and the scorer; ARCH builds the runners (`w5k scenario proving --vehicle <ron> --test <name>`), which write a result JSON in the shape of section 2.

## 1. The idea (and why it is shaped this way)
A test here is four things: a **scripted scenario**, an **expected value with its source**, a **tolerance band**, and a **traffic light**.
The expected value comes from one of three places, in order of strength:
1. **A closed-form oracle**: physics that has an exact answer for the inputs the simulator was given (braking distance `v^2 / (2 mu g)`, rigid rollover `atan(t / 2h)`). The oracle is evaluated on the inputs the *runner echoes back in the result*, so it judges the simulation against its own numbers: a model cannot pass by quietly using a different tyre friction than the spec sheet says. A disagreement with an oracle means a bug or a deliberate effect (load transfer, tyre compliance) that must be explained from the force ledger.
2. **A published figure** from the dossier (M998 60% grade, 40% side slope), with the dossier's evidence level shown on the light.
3. **A reasoned bound** tagged `UNVALIDATED` with the reasoning: not evidence, but it can still catch a model that is physically impossible (faster than the engine can supply energy).
Nothing in this spec is a number from memory: where a figure is needed and no source is reachable the cell says `SOURCE NEEDED`, and the light is `not measured` until one lands.

**Tolerance classes.** ADR-0007's classes apply to published figures (static 3%, power 5%, top speed 7%, acceleration and braking 15%, mobility bracketed). Oracle tests need their own class: **Oracle, green within 10%, amber within 20%** (PROVISIONAL(card to be raised by ARCH): an oracle is exact for an idealised vehicle, so the band only has to absorb real effects such as load transfer and tyre relaxation, not scatter in published data).
Lights: `green`, `amber`, `red`, `not measured`, as the dashboard has them. A run that ends early (`ended_early` set) or contains a NaN is **red for the whole test**, with the reason shown.

## 2. Result JSON: `w5k.proving.result.v1` (the contract between ARCH's runners and my scorer)
```json
{
  "schema": "w5k.proving.result.v1",
  "test": "braking_50kmh",
  "vehicle": "mule_4x4",
  "contract_pin": "contract-v0.1",
  "inputs":   { "mass_kg": 2300.0, "mu": 0.8, "speed_m_s": 13.89 },
  "labels":   { "surface": "dry_asphalt", "road_class": "C" },
  "measured": { "stop_distance_m": 14.9, "peak_decel_g": 0.81 },
  "ended_early": null,
  "replay": "out/proving/mule_4x4/braking_50kmh.replay.json"
}
```
- `inputs`: every number the oracle needs, **as the simulation used it** (SI, unit as the key suffix). Required keys per test are in section 3. Extra keys are allowed and ignored.
- `measured`: scalar results, SI, key suffix = unit. `labels`: free text (surface, road class, seed). Both maps are ordered (BTreeMap): the output is deterministic.
- `ended_early`: `null`, or the reason (rolled over, stuck, left the course, NaN). `replay`: path or `null`.
- Unknown `schema` or a missing required key is an error naming the key, never a silent pass. Rust type: `w5k_validate::proving::ProvingResult`.

## 3. Phase 1 tests
Notation: `m` mass, `mu` peak friction of the tyre on the test surface (as used by the sim), `g` gravity, `t` track, `h` centre-of-mass height, `r` loaded wheel radius, `P` engine power at the wheels.

| id | scenario (script) | `inputs` required | `measured` | expected value and source | band and light |
|---|---|---|---|---|---|
| **a. `accel_0_48kmh`** | dry asphalt, full throttle from rest, record time to 48 km/h (13.33 m/s) | `mass_kg`, `power_w`, `mu`, `driven_load_fraction` | `t_0_48_s` | Lower bound `t_min = max( m v^2 / (2 P), v / (mu f g) )`: the vehicle cannot beat the energy it is given or the traction it has (f = fraction of weight on driven wheels). UNVALIDATED (first-principles bound). Published 0-32 km/h for M998: SOURCE NEEDED. | **Red if `t < t_min`** (impossible). Green if `t_min <= t <= 3 t_min`, amber to `5 t_min`, else red (ESTIMATE: gearing, shift time and drag put real trucks at about this multiple; the band is a judgement and is to be replaced by the published figure). |
| **b. `braking_50kmh`** | dry asphalt, 50 km/h (13.89 m/s), full brake, straight, ABS off | `speed_m_s`, `mu` | `stop_distance_m`, `peak_decel_g` | `d = v^2 / (2 mu g)`. Ideal rigid-body friction stop; Wong, *Theory of Ground Vehicles*, braking chapter (page to be added when the book is read). Published M998-class stopping distance: SOURCE NEEDED (dossier slot `dynamics.braking_distance_50kmh_m` to be added with a source). | Oracle class on `d`. A sim that stops **shorter** than the oracle is red outright (it beat friction). Longer is plausible (brake lag, load transfer); amber from +10% to +20%. |
| **c. `gradeability`** | ramp of constant grade on a surface of friction `mu`; start from rest facing uphill; the **runner bisects** grade (to 0.01) for the steepest grade it can start on and hold 5 s | `mass_kg`, `mu`, `driven_load_fraction`, `wheel_torque_crawl_nm`, `wheel_radius_m`, `rolling_resistance_coeff` | `max_grade_ratio` (rise/run), `bracket_lo`, `bracket_hi` | `min( traction, power )`. Traction (all wheels driven, first order, ignoring slope-induced load transfer): `grade_t = mu f`. Torque: `F = T/r`, `F = m g (sin a + c cos a)`, solve `a`, grade = `tan a`. Compare with the **published M998 60%** (dossier `mobility.max_grade_ratio`, UNVERIFIED). | (i) oracle class on `min(traction, torque)`; (ii) **bracket vs published**: `published <= sim <= 1.25 published` (ADR-0007), light provisional while the dossier is secondary. The surface for the published rating is unknown: SOURCE NEEDED, so (ii) is shown with that caveat. |
| **d. `skidpad`** | flat dry asphalt, constant-radius circle (radius `R` chosen so the limit is reachable, `R` echoed), steering held, speed ramped slowly until the vehicle cannot hold the circle; **at three lower speeds** record steer angle for the understeer slope | `mu`, `radius_m`, `track_m`, `cg_height_m`, `front_load_fraction` | `max_lat_accel_g`, `understeer_gradient_rad_per_g` | Upper bound `a_y/g <= min( mu, t / (2h) )` (slide first or roll first). Understeer gradient sign: `K_us = W_f / C_af - W_r / C_ar` (Gillespie, *Fundamentals of Vehicle Dynamics*, steady-state cornering; page TBD); the sign needs tyre cornering stiffness, which the sim owns, so the expected sign is `UNVALIDATED`: heavy-front, equal-tyre light trucks are understeer (positive). | Oracle class on `max_lat_accel_g` against the bound (**red if above it**, amber if below 70% of it). Understeer sign: green if the sign matches the expected sign computed from the axle loads, red if opposite, *and the light says UNVALIDATED*. |
| **e. `side_slope_rollover`** | tilt table / side slope: slowly raise the cross-slope until the vehicle slides or tips; report which | `track_m`, `cg_height_m`, `mu` | `slope_angle_rad`, label `mode` = `roll` or `slide` | Rigid-vehicle tipping: `tan(angle) = t / (2h)`; sliding: `tan(angle) = mu`. The vehicle fails by the smaller. Suspension and tyre roll compliance **lower** the tipping angle, so `atan(t/2h)` is an **upper** bound. Compare with the published M998 40% side slope (21.8 degrees), UNVERIFIED. | **Red if above the bound** (+3%). Green if within 25% below it (ESTIMATE for compliance; the band is the reasoning, replaced by a measured compliance when CHASSIS gives a roll stiffness), amber to 40% below. Published rating: bracket as in (c). `mode` must equal the smaller of the two analytic angles. |
| **f. `ride_iso8608_c`, `ride_iso8608_d`, `ride_washboard`** | straight run over a seeded random road profile with ISO 8608 roughness class C (and D), at 5, 10, 15, 20 m/s; and over a sinusoidal washboard of wavelength `lambda` at the same speeds | `sprung_mass_kg`, `spring_rate_n_m`, `damping_n_s_m`, `unsprung_mass_kg`, `tyre_rate_n_m`, `class_gd_m3` (the PSD level used), `lambda_m`, `seed` | `rms_az_m_s2` per speed (keys `rms_az_m_s2_at_5`, `_10`, ...), optionally `rms_az_w_m_s2_at_*` (ISO 2631-1 Wk-weighted; unweighted first) | **Oracle: a quarter-car** (two-mass, linear) transfer function integrated numerically by the scorer over the PSD (`w5k_validate::quarter_car`, coded and tested in its own PR). The road PSD is `G_d(n) = G_d(n0) (n / n0)^-2`; vertical velocity input is white, so RMS is exactly computable. Washboard: body resonance at `v = f_n lambda`, `f_n = sqrt(k_eff / m) / 2 pi`; expected peak of `rms_az` near that speed. ISO 8608 class boundaries (m^3 at n0 = 0.1 cycles/m): UNVERIFIED, the standard is paywalled; a secondary source is needed before C and D are scored. | Oracle class on the RMS at each speed (quarter-car is only a four-wheel-averaged picture, so use 20% green / 40% amber, ESTIMATE). Washboard: the resonance speed must be within 15% of `f_n lambda`. Rising RMS with speed is required (monotone up to the resonance) else red. |
| **g. `step_climb`** | ramp the step height at 2 m/s (walking pace), driven straight on, bisect the highest step cleared | `wheel_radius_m`, `mu`, `driven_axle` (front, rear, all) | `step_height_m` | Rigid wheel kinematic limit: `h <= r` (the wheel centre cannot rise over a corner higher than its own radius without the hull pushing). The traction-limited value (wheel at a corner, friction cone against the corner normal) will be derived in `docs/theory/validation.md` and checked on a rigid-wheel toy before being used: until then **only the kinematic bound is scored**. Published M998 vertical obstacle: SOURCE NEEDED; M113A3 and M4A3 summaries have 24 in (0.61 m) for tracked vehicles, UNVERIFIED. | **Red if `h > r`** (rigid-wheel kinematic bound), green if `0.5 r <= h <= r` (ESTIMATE: low-pressure tyres climb close to a radius; below half a radius with traction available means the model is hiding a bug), amber 0.3-0.5 r. |

## 4. Terrain scoring (consumes WORLD's `w5k world stats` JSON)
The same traffic lights, applied to the **ground** rather than the vehicle: is the terrain a believable terrain? Expected schema from WORLD (`w5k.world.stats.v1`, WORLD to confirm; I add a scorer for whatever shape it commits to):
```json
{ "schema": "w5k.world.stats.v1", "course": "...",
  "slope_hist": { "bin_edges_rad": [...], "fraction": [...] },
  "road_grade_max_ratio": 0.12,
  "roughness": { "n_cpm": [...], "psd_m3": [...] },
  "materials": [ { "id": 0, "name": "...", "mu_peak": 0.8 } ] }
```
| check | rule | source |
|---|---|---|
| roughness PSD slope | fit `log PSD` vs `log n`; the exponent ("waviness") should be about -2, as ISO 8608 assumes. Green within 0.5 of -2 (ESTIMATE band), amber within 1.0. Fitted class (A to H) is **reported**, and compared with the class the course says it is | ISO 8608 (the standard itself is paywalled; UNVERIFIED) |
| roughness class | the level at n0 = 0.1 cycles/m falls in the class's published range | SOURCE NEEDED (class boundaries) |
| steepest road grade | the road must be drivable by the calibration vehicle: `road_grade_max_ratio` below the M998's published 60%; and a road above ~ half of it is amber (a road should not need crawl gear): ESTIMATE | dossier `mobility.max_grade_ratio` |
| slope histogram | fraction of ground steeper than any reference vehicle can climb is **reported** (impassable area), not judged | derived |
| friction vs Wong | each material's `mu_peak` inside Wong's published range for that surface class | Wong, *Theory of Ground Vehicles*: SOURCE NEEDED (book not reachable from a lane session; owner can drop the table into `content/dossier/sources/`) |
| physical sanity | `0 < mu_peak <= 1.2` for every material (a rubber tyre does not exceed this on any ground; ESTIMATE band) | engineering judgement |
Until a source lands the friction and class-range checks show **not measured**, never a pass.

## 5. What would convince us the model is wrong (falsification table)
| result | on which vehicle | falsifies | what we do |
|---|---|---|---|
| braking distance **shorter** than `v^2/(2 mu g)` | any | the tyre force model (friction exceeds its own peak: a missing clamp, or a double-counted brake force) | red; read the force ledger tyre terms |
| braking distance **more than 20% longer** at low `speed` | any | brake torque or tyre slip stiffness too soft; a bug in the brake model | red, bisect brake torque |
| skidpad `a_y` above `min(mu, t/2h)` | any | lateral tyre force or roll geometry (rollover not enforced) | red |
| understeer gradient negative on a front-heavy truck | scout/mule | cornering stiffness balance or load-transfer modelling | red, `UNVALIDATED` shown |
| side-slope tipping angle **above** `atan(t/2h)` | any | centre-of-mass height or track entered wrongly in the dynamics, or tilt applied incorrectly | red |
| side-slope mode is `slide` where `mu > t/2h` or the reverse | any | friction versus roll balance | red |
| gradeability above `mu f` | any | slope traction or gravity component; a hidden "stick" force | red |
| gradeability far below the torque bound while the engine has power | any | gearbox or clutch model (the crawl ratio is not reaching the wheels) | amber/red |
| sim 0-48 km/h time below `t_min` | any | the power or inertia model (energy is created) | red |
| ride RMS does not rise with speed, or has no resonance near `f_n lambda` on washboard | any | suspension or wheel-hop dynamics (a quarter-car sanity failure) | red |
| step climb above one wheel radius | any | tyre contact modelled as a point or ground collision missing | red |
| M998 grade or side slope far above the published rating (> 1.25 x) while mu is low | M998-like | the mobility *rating* semantics (published ratings are guarantees: the sim must meet them without wild excess) | amber/red, card if the tolerance is wrong |

## 6. Scored vehicles and dashboard
`scout_4x4`, `mule_4x4`, `hauler_4x4` (FORGE authors them). The dashboard gains a **per-vehicle by per-test grid** (one cell: light + measured value + expected value; click is not needed, the page is static) and a terrain panel. Nothing is tuned to improve a light; reds stay red.

## 7. Delivery order (small PRs)
1. This spec and `proving::ProvingResult` (the schema and its parse test).
2. Oracles and scorers for (b), (e), (g) (closed-form); lights and the negative controls (a model that beats its own friction is red).
3. Oracles for (a), (c), (d); the quarter-car oracle for (f).
4. Terrain scorer for the schema WORLD commits to.
5. Dashboard grid.
