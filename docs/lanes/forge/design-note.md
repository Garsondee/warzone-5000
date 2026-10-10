# FORGE design note (settling round)

**Purpose.** A designer states *intent* (ride frequency, damping ratio, peak power, service deceleration); the solver needs *parameters* (spring N/m, damper N s/m, torque table, brake N m). The compile is the bridge, and it must be deterministic, explain itself, and refuse nonsense instead of clamping it.

## 1. Pipeline (each stage is a pure function of the previous stage's output; ordered iteration, no `HashMap`)
1. **Check**: `VehicleDef::check()` (every `Param` inside its band, provenance sources) plus structural rules.
2. **Resolve**: derive the frame (datum = hull box centre, D3), axle positions, static axle loads from statics, tyre loads.
3. **Masses**: the breakdown composes into hull mass, COM and inertia (section 3).
4. **Running gear**: stations, tyres, springs, dampers, bump stops, linkages for bogies, tandems and inboard leaf springs; `ride_height_m`; track loop order from the sprocket.
5. **Powertrain**: torque curve, gear ratios, final drive, driveline tree, outputs, brakes.
6. **Articulation and weapons**: turret, gun and mount joints, servos, recoil, muzzles; armour onto GEOMETRY's shells.
7. **Render rig**: nodes and joint bindings from the same stations; meshes from GEOMETRY (wheel radius shared with the physics, so they cannot disagree).
8. **Integration**: substeps from the stiffest resolved mode.
9. **Validate and report**: `PhysRig::validate()` and `RenderRig::validate()` must pass; the compile report lists every slider and what it became.
Output is `Result<(PhysRig, RenderRig, Report), Vec<Rejection>>`; a `Rejection` names the field, the numbers and the reason.

## 2. Slider mappings (the formulas; spike S5 verified the first four)
| Slider | Becomes | Formula |
|---|---|---|
| ride frequency `f` | spring rate | sprung corner mass `m = share W / (n g)`; ride rate `k_r = m (2 pi f)^2 / MR^2`; spring `k_s = k_r k_t / (k_t - k_r)` (tyre in series, D1) |
| damping ratio `zeta` | damper | `c = 2 zeta sqrt(k_r m) / MR^2`, split `c_bump = 2c/(1+r)`, `c_reb = 2cr/(1+r)` (D2) |
| anti-roll | `AntiRollDef` | stiffness at the wheels, N/m of differential travel (as authored; `rate >= 0`) |
| ride height | `ride_height_m`, rest positions | `ride_height = clearance + height/2`; wheel centre = `R - load/k_t` above ground; preload = sprung weight share (validate checks 5%; the compile hits it exactly) |
| `mu_peak_ref` | `TyreDef::mu_scale` | `mu_peak_ref / mu_peak(dry hard reference surface of the terrain table)` |
| peaks of power and torque | torque table | vertex at the torque peak; parabola below (idle fraction), cubic above, solved so `T(n_P) = P / omega_P` and `dP/dn = 0` at `n_P`; rejected if torque goes non-positive or a higher peak appears |
| gear spread (W6) | ratios | first gear from launch: `g_1 = m g (sin(theta) + c_rr) r / (T_pk eta fd)` with a safety factor from a `Param`; final drive from top speed: `fd = omega_red r / (v_top g_n)`; between them geometric, `g_i = g_1 q^-(i-1)`, `q = (g_1/g_n)^(1/(n-1))` |
| service decel | brake torque | `F = m a`, `a = decel_g g`; axle share from `brake_share` or from dynamic axle loads (`N_i = N_i0 +- m a h / L`); torque per wheel `F share r / n`; rejected if `decel_g > mu_peak_ref` (the tyre cannot deliver it) |
| substeps | `IntegrationDef` | `n = ceil(omega_max dt / c)` over wheel hop `sqrt((k_s+k_t)/m_u)`, bump stops, slip oscillator `omega_n^2 = C_s F_z r^2/(sigma J)`; the constant `c` is settled by CHASSIS' spike S1; **reject above 8** naming the mode and the field that drives it |

## 3. Mass breakdown, COM and inertia
Items (structure, engine, transmission, fuel, crew, ammunition, turret, each with a position) sum to the mass: `M = sum m_i`, `r_com = sum m_i r_i / M`. Inertia about the COM: `I = sum (I_i + m_i (|d_i|^2 1 - d_i d_i^T))`, `d_i = r_i - r_com` (parallel-axis theorem). `I_i` comes from GEOMETRY's closed-mesh integrals where a mesh exists (`w5k_geo` is a stub today; until it lands, analytic boxes and cylinders), else from the item's primitive. Tested against a two-box hand calculation, positive-definiteness (all leading minors) and the triangle inequalities. For more than two axles the static load split is the minimum-norm solution of force and moment balance, except that a linkage (tandem, bogie) fixes its members' shares by its geometry.

## 4. Rejection rules (a reason, never a clamp)
Peaks inconsistent (power at `n_P` not below the torque peak, spike F5); `k_ride >= k_t`; COM outside the axles; decel above `mu`; negative torque in the working range; static tyre deflection outside 0 to 0.35 R; more than 8 substeps; any `Param` outside its band. A slider outside its band is a `check()` failure, not a clamp.

## 5. Risks and what is not decided
- `w5k_geo` delivery date (mass integrals); until then primitives.
- Substep constant `c` waits for S1.
- Armour zones need COMBAT's confirmation (CCR section A).
- The reference-vehicle figures wait for VALIDATION's dossier; none are written from memory.

## 6. Plan after this PR (not started; waits for review)
Build order 1 to 6 of the brief; first PR: `VehicleDef` RON round trip and `compile_is_deterministic`, then masses, then sliders, promoting `s5.rs` into the real compile.
