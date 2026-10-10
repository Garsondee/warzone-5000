# Spike S6: articulation coupling (lane COMBAT)

**Verdict: PASS, with one contract change request (CCR-C1).** A reduced-coordinate solve of hull + turret + cradle + barrel conserves linear and
angular momentum to 1e-6 of the impulse from **2 substeps** (3e-7 at 3, 3e-8 at 6) with a 4th-order integrator. "Joints driven by the hull's
motion, reaction wrench added a substep late" (the glue pattern the 0.2 port implies) **diverges** at this mass ratio and loses 7% of the impulse
at best. Kill criteria (more than 8 substeps; neither scheme holds 1e-6) are not met. Code: `spikes/combat/s6` (about 900 lines, `cargo run --release`),
full printout `spikes/combat/s6/output.txt`, plots `docs/lanes/combat/media/s6-*.png`.

## What was built
A free hull (no gravity, suspension, ground) carrying the `box_tank()` chain turret (yaw) > cradle (pitch) > barrel (prismatic, +Z = rearward), the
`ServoDef`s and `RecoilDef` read from the rig, so the contract's numbers are the ones tested. The copy fixes the stand-in: recoiling mass 2.0 t (a 120 mm
tube and breech; 1.2 t gives 16.5 m/s and an impossible stroke), cradle 2.0 t, a metered buffer `c = 36 kN s/m, q = 1500 N s^2/m^2` (ESTIMATE,
UNVALIDATED). The shot is `J = 1.5 x 8 kg x 1650 m/s = 19.8 kN s` applied along the bore at the muzzle line; equal and opposite momentum, and its
angular momentum about the world origin, is booked to the **ejecta**. All momentum checks include the ejecta. Joint limits are penalty springs
(the reaction reaches the parent through `M`, so nothing is deleted).
- **A**: generalised velocity `u = [v_hull, w_hull, qd1, qd2, qd3]` (9); `M(q)` from the body Jacobians, bias from a zero-acceleration pass,
  one 9x9 direct solve; integrated with RK4 (or semi-implicit Euler for contrast). Servo effort and its latency are held over a substep; passive
  forces (recoil, stops) are evaluated inside the stages.
- **B**: the glue pattern: joints solved against the hull's previous acceleration, the reaction wrench handed to the hull next substep, Euler.

## Results (60 Hz tick, `n` substeps; fire at t = 1 s while the turret is slewing 0.8 rad and the gun elevating)
| scheme | n | max abs dP / J | max abs dL / (J x 1 m) |
|---|---|---|---|
| A, RK4 | 1 / 2 / 3 / 6 / 8 | 5.3e-6 / 4.4e-7 / 9.6e-8 / 1.0e-8 / 4.1e-9 | 7.1e-6 / 1.0e-6 / 2.5e-7 / 3.2e-8 / 1.3e-8 |
| A, Euler | 6 / 12 | 2.6e-3 / 1.3e-3 | 9.7e-3 / 4.8e-3 |
| B, Euler | any | diverged | diverged |

- **Why B fails.** With the hull 10 to 1000 times heavier than the chain it is stable but loses `7.06e-2 J` at every ratio: that is the peak
  mechanism force times the step (480 kN x 2.8 ms = 1.3 kN s). At the real ratio (chain 16 t, hull 28 t) the explicit lag is unstable (the
  "added-mass" instability of partitioned coupling), even with no shot, just a slew. It is not a tolerance to tighten; the scheme is wrong.
- **Why A-Euler fails.** `P = sum m_i v_i(u, q)` changes through `q` inside a step; first order leaves `O(dt)` drift. Fourth order does not.
- **Swivel chair** (coaxial copy, all COMs on the yaw axis): hull yaw equals `-q I_t/(I_h + I_t)` (I_h 135,333, I_t 25,387 kg m^2) to 1.3e-8 rad
  over the whole 14 s slew (the relation holds at every instant, since L = 0). `|P|`, `|L|` at the end: 1.5e-12 N s, 9.7e-12 N m s.
- **Slew only (no shot)**, 4 s: A-RK4 max `|P|` 1.1e-8 N s, `|L|` 9.2e-9 N m s; Euler 10 and 28; B infinite.
- **Recoil**: barrel 9.90 m/s after the shot (= J/m_barrel), peak stroke 0.310 m of 0.35, peak mechanism force 480 kN, back in battery at 0.39 s;
  the vehicle's centre of mass moves at J/M_total = 0.45 m/s.
- **Servo stability across mass** (fixed gains, limits off, held hull, 0.5 rad step): overshoot matches `exp(-pi z / sqrt(1 - z^2))`, `z = kd / (2 sqrt(kp I))`,
  to 0.05 points at 0.5x, 1x and 2x turret mass (stand-in gains: z 0.71, 0.65, 0.56; overshoot 4.4, 7.0, 12.0%). Stable and monotone in mass.
  On a **free** hull the joint feels the reduced inertia `I_t I_h/(I_t + I_h)` (41,774 against 63,943 kg m^2), so z is higher there.
- **Stand-in gains are soft**: zeta is fine (0.65 yaw, 0.56 pitch; the brief's 0.1 to 0.2 does not reproduce) but `wn = 0.97 rad/s` makes a 90
  degree slew 27% slower than `d/w + w/alpha` and overshoot 20% with real limits. A derived set (`kp = I wn^2`, `kd = 2 z I wn`, `wn = 4 rad/s`,
  `z = 0.8` at 1x) gives a slew within 0.2% (free hull) and 2.8% (held hull; the 50 ms latency shifts the whole move: `T = d/w + w/alpha + latency`).
  The midpoint crossing is the right measurement (the trapezoid is time-symmetric); "first within 1%" reads 14% early because the loop leads in the decel.
- **Stabiliser law** `|1 - r e^(-jwt)/(1 + jf/f_b)|`: holds to **1%** at 0.1, 0.25, 0.5 and 1 Hz when the rate loop is fast enough
  (`kd/I = 8 w_b`, latency 0.01 s, `f_b = 0.5 Hz`; the servo's position command must include the stabiliser's integrated rate, or the position loop fights
  it). With the stand-in (`f_b = 3 Hz`, hydraulic latency 0.05 s, `kd/I = 1.5 rad/s`) it misses by 26 to 285%: **a 3 Hz stabiliser is not reachable with a 50 ms
  servo delay.** The law is right; the data must be consistent with it (CCR-C3).

## Contract change requests implied (written as text; ARCH settles into v0.3)
- **CCR-C1 (`ports.rs`): the articulation integrates the hull's own six degrees of freedom with the chain.** The 0.2 port returns a wrench for the
  glue to apply later, which is scheme B. Change `ArticulationPort::step` to take the sum of all *other* loads on the hull for the substep
  (`ext: ArticulationLoads { force_n, torque_about_hull_com_nm }`, world, held over the substep) and write back the hull's state at the end of the
  substep (`HullMotion` out: rot, pos, vel, omega). The glue substitutes it for its own hull integration whenever a port is present and keeps
  `ArticulationWrench` only for the ledger (the average reaction over the substep). The glue's ground and suspension loads stay explicit over a substep, as today.
  `LAYERS.md` is unchanged (the port is still COMBAT's, the glue still owns the order). Needs `PhysRig::hull_inertia` access (already `rig.hull`).
- **CCR-C2 (`rig.rs`, documentation only): pin the servo law.** `effort = clamp(kp (q_cmd - q) + kd (qd_cmd - qd), +-max_effort_si)` after `latency_s`;
  `q_cmd`, `qd_cmd` from a trapezoid at `max_rate_si`, `alpha = min(max_accel_si, max_effort_si / I)`, `I` = the held-hull subtree inertia about the axis;
  the stabiliser adds `-r LP(parent rate)` to `qd_cmd` and its integral to `q_cmd`. Recoil: preload against a stop at 0, `F = -(preload + k x) - c v - q v|v|`,
  stops as penalty springs with `w_stop dt <= 1`.
- **CCR-C3 (FORGE `validate()` and derivation, no type change): derive, do not author, the gains.** `kp = I wn^2`, `kd = 2 z I wn`, `wn` from the
  slew accuracy (lag in time is `latency + alpha/(wn^2 w)`), `z` 0.8 at the nominal ammunition load. Stabiliser: the rate loop `kd/I >= 8 w_b` and
  `kd/I x (latency_servo + latency_stab) <= 0.5`, i.e. `f_b <= 1/(32 pi (t_servo + t_stab))`; the achieved bandwidth, not the dossier's wish, goes in
  `StabiliserDef`. Recoil: a bake check that integrates the slide (the 4/e rule of the red-team is for a linear buffer only) and requires 10% of
  the stroke in hand. The numbers above are UNVALIDATED (stand-in masses; the dossier gives real ones).
- Nothing else: the `_si` suffixes already in 0.2 are what S6 used; `Shot` carries what the ejecta bookkeeping needs.

## Limits of this spike
Instantaneous impulse (a 5 to 10 ms pressure pulse moves the barrel about 5 cm during the shot; ignored). No gravity (the preload against elevation is a
rig validation rule already). One tree (the branching tree with a hull-fixed MG is the same code with another parent index; not exercised). The hull
here floats; the effective-inertia and stabiliser numbers on tracks come when the real hull exists.
