# DRIVE design note (settling round)

Lane DRIVE builds `DrivePort` from any `DrivetrainDef`. Principle: **DRIVE returns torques; it never integrates a wheel** (CHASSIS does). It owns the engine, coupling and
gearbox states, the brake temperatures and the fuel integral. Everything is `f64`, SI, ordered, `libm` via `w5k_math::scalar`, constants from the rig or RON `Param`s.

## 1. Engine
`T(w, thr) = thr T_full(w) - (1 - thr) T_drag(w)`, `T_full` linearly interpolated from `torque_curve` (rpm converted once), `T_drag = drag_const + drag_per_rpm rpm` (rig fields).
- Idle controller: a PI on `idle_rpm` adds throttle only when `thr` is small; it is bounded so it cannot exceed full load. A free turbine (`free_output`) has no idle floor and may stall.
- Rev limiter: torque faded to zero over the last 3% below `redline_rpm` (a smooth cut, no chatter); fuel cut above it (a governor/limiter is one mechanism here; the fade width is a `Param`, ESTIMATE).
- Response: first-order lag with `response_time_s` on the *torque demand* (turbo lag, turbine spool).
- Dynamics: `J_e dw/dt = T_engine - T_coupling`, integrated semi-implicitly (the drag term is linear in `w`, so it is solved implicitly: exact energy decay).
- Fuel: `mdot = P_mech BSFC(w, T)`; the map is a quadratic bowl around the best point `bsfc_best_g_kwh` (at ~ 75% of peak torque, mid-speed) with a floor at idle (`idle_fuel_kg_s`). Range = integral of distance over fuel. The bowl shape is an ESTIMATE (card needed only if the owner wants real maps).

## 2. Coupling (from spike S-D, `docs/lanes/drive/spike-d.md`)
- **Clutch / lock-up**: *implicit stick/slip*. Each step compute the torque that would lock engine and input shaft at the end of the step, `T_lock = J_eff (slip/dt + ...)` (with the load stiffness from `ShaftState`), clamp to `+-capacity`. Stable at every rate tested (60 Hz to 1 kHz), exact stick, no energy creation.
  Automatic clutch capacity ramps with engine rpm above `engage_rpm`; manual follows `DriveInputs.clutch`. Clutch heat is booked (slip x torque), reported only at M3.
- **Torque converter**: `T_pump = (w_p/K)^2` linearised implicitly; `T_turb = TR(sr) T_pump`, `TR` linear from `stall_ratio` at `sr = 0` to 1 at the coupling point (a `Param`-banded 0.85 default); lock-up above `lockup_speed_ratio` becomes the stick/slip clutch with converter capacity.
- Substep needed: **one**, at the chassis rate. DRIVE does not substep.

## 3. Gearbox
Ratios from `GearboxDef`, `efficiency` per mesh. Output shaft speed = input / ratio; torque x ratio x efficiency (efficiency applied in the *direction of power flow*: driving loses torque, overrunning loses braking). Automatic: upshift when engine rpm > `upshift_rpm` (scaled by throttle: 60% of the way to the limit at part throttle), downshift below `downshift_rpm`, with a hysteresis band and a minimum dwell (a Schmitt trigger). A shift opens the coupling for `shift_time_s` (torque interruption; exact per the spike), then re-engages. Kick-down on full throttle. `GearRequest` manual selects directly; reverse uses `reverse_ratios`; neutral decouples.

## 4. Driveline tree
Walk `DriveNode` once at build into a flat list of (output, cumulative ratio, cumulative efficiency).
- **Open diff**: input torque split by `split` (equal default), creates no torque; each output integrates its own wheel (CHASSIS). The torque available at each side is capped by what the gearbox/coupling supplies; with no torque reaction on the slow side an open diff spins the free wheel (that emerges, it is not coded).
- **Locked / mode `locked_groups`**: the group is treated as one inertia; torque is shared in proportion to the implicit solution using `load_stiffness` so equal speed is enforced without a stiff spring.
- **Limited slip**: open split, then torque moved from the faster to the slower side up to `bias` x slower torque (bounded, tested).
- **Transfer case / modes**: `ratio_scale` multiplies the root ratio; `declutched_outputs` get zero drive torque (brakes still act); mode changes only at low speed or neutral (a `Param`-banded rule, else refuse and say so in telemetry).
- **Tracked steering units** as torque splits with the steer demand `s`: *clutch-brake* sends full torque to both sides and applies the inner steering brake in proportion to `|s|` (`steer_brakes`), the brake force producing the yaw (heat goes in that brake); *controlled differential* the same torque bias via brake torque; *double differential/hydrostatic* impose a speed *difference* `diff_ratio_by_gear` x mean or `diff_speed_rad_s` (hydrostatic, works in neutral): realised as an antisymmetric torque pair `+-T_s`, `T_s = clamp(k (dw_target - dw), +-max_steer_torque_nm)` with implicit gain (no extra stiffness). Detents quantise `|s|`.

## 5. Brakes
Torque `= max_torque * pedal_lag * fade(T_disc)`; `fade` linear from 1 at `fade_start_k` to `fade_floor` at `fade_end_k`. Temperature: `C dT/dt = P_brake - (cooling + cooling_per_ms v)(T - T_amb)`, advanced exactly for the linear cooling term. Parking brake on `parking: true` brakes. Apply/release lags per `apply_time_s`/`release_time_s`. `BeforeFinalDrive` multiplies by the output ratio; `Driveline` site brakes the gearbox output. **Never reverse a stopped shaft**: brake torque is `-sign(w) min(|T_b|, |w| J_shaft/dt + |other torques|)`-style clamped (implicit stick), so a stopped shaft holds if the net drive torque is below the brake capacity and a held shaft needs no sign.
Band brakes use `reverse_torque_factor` when `w < 0`.

## 6. Does the contract suffice?
`DriveInputs` (throttle, brake, steer, gear, parking, clutch, mode) and `ShaftState` (speed, inertia, vehicle speed, load torque and stiffness) are enough for everything above. `DriveTelemetry` lacks only what is below. Nothing blocks the settling round.

## 7. CCRs expected (none blocking; text only, to be filed after review)
1. `DriveTelemetry` additions: `clutch_heat_j`, per-output `torque_nm` split (for the ledger), `fuel_used_kg` cumulative. Additive, defaulted.
2. `ShaftState` has no ambient temperature; brake cooling needs one. Proposal: `DriveInputs.ambient_k` (default 288.15 K) or a world value passed by the glue. Default is acceptable until M3.
3. Engine start/stop (key on) for turbine/idle-off: `DriveInputs.engine_on: bool`, default true. Not needed for M1.
4. Clarify in CONTRACTS.md whether `torque_nm_out` for a **declutched** output may be exactly 0 while its brake acts (assumed yes).

## 8. Risks
Implicit lock with a speed-dependent load (spike used a constant) is the open numerical risk; first test in the lumped bench. A fuzz of 200 random valid `DrivetrainDef`s must give no NaN: every divide is guarded by validated positives.
