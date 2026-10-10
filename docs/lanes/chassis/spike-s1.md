# Spike S1: stiffness versus step size (CHASSIS)

Script: `spikes/chassis/s1/s1.py` (standard library only), raw output: `spikes/chassis/s1/output.txt`.
Rig: quarter car, sprung 400 kg, unsprung 50 kg, spring 30 kN/m, damper 2 kN s/m, tyre 250 kN/m, semi-implicit Euler (velocity first, then position).

## The principle in one paragraph
A spring-mass pair is an oscillator with angular frequency w = sqrt(k/m). A time stepper samples it every dt. Semi-implicit Euler is *symplectic*:
it conserves a slightly distorted ("shadow") energy exactly, so energy wobbles but never drifts, provided w*dt < 2. Past 2 the sampled motion
grows every step and the sim explodes. Accuracy needs far smaller steps than stability does: the *frequency* error grows as (w*dt)^2 / 24.
Graphics analogy: it is the same reason a stiff cloth spring in a game explodes at 30 fps but is fine at 120.

## Results (closed form from the 4th-order characteristic polynomial)
| quantity | closed form | simulated |
|---|---|---|
| body mode | 1.322 Hz, zeta 0.246 (damped); 1.301 Hz undamped | frequency error 0.08% at 60 Hz, below 0.02% from 120 Hz |
| wheel-hop mode | 11.74 Hz, zeta 0.277 (damped); 11.92 Hz undamped | (see rule below) |
| body damping ratio (log decrement) | 0.2455 | 0.80% error at 60 Hz, 0.22% at 240 Hz (limit: 5%) |
| energy, undamped, frictionless | conserved | bounded oscillation (24.7% of the initial energy at 60 Hz, 2.8% at 300 Hz, 0.8% at 1 kHz); **drift below 0.04% per minute at every rate** (limit: 0.1%) |
| stability limit | | explodes at 45 Hz, stable at 60 Hz: the limit is about 50 Hz, i.e. w_hop*dt of about 1.5 (the damper pulls it below the undamped 2) |
| speed hump, 20 km/h, 50 mm | | 60 Hz: peak body excursion off by 4.1%; 120 Hz: 0.1%; 180 Hz and up: under 0.2% |

The oscillating energy error is not drift: semi-implicit Euler conserves the shadow energy, and the real energy wobbles around it by an amount that
shrinks as dt^2. The drift column is the honest test.

## Substep rule
Stability only needs 60 Hz here; accuracy and bump-stop impacts need more. The hump result says about 3 substeps (180 Hz) are accurate to 0.1% for a mode of 12 Hz,
which is w*dt = 0.42. The rule proposed in the brief, `substeps = ceil(20 * f_max_hz / 60)`, gives 4 for this hop mode (11.7 Hz), 5 on the bump stop (12.8 Hz): w*dt = 0.25 to 0.27,
conservative and cheap. **Adopt it, with `f_max` defined as the stiffest *mechanical* mode (wheel hop including the bump stop engaged; the stop's stiffness at full travel).**
The slip oscillator (wheel spin against the tyre's relaxation) would need 9 substeps on `box_truck` (25 Hz): do **not** pay for it with substeps. The tyre
update uses the exact exponential (`state += (target - state) * (1 - exp(-v dt / sigma))`), which is unconditionally stable, so that mode leaves the rule.
Kill criterion (more than 8 substeps) is not triggered by any wheeled rig seen so far (M998 5, M35 5).

## Stopped truck on a 10% grade (2500 kg, mu 0.8, 5 s)
| model | result |
|---|---|
| Coulomb with `sign(v)` (friction is zero at v = 0) | **creeps**: 1.6 mm in the last second; it never holds because a stationary tyre gets zero friction |
| bristle / relaxation-length (sigma 0.05, 0.2, 0.5 m) | settles to the static stretch sigma * tan(slope) / mu (6.25, 25, 62.5 mm: equals the closed form) and moves 0.0 um in the last second; no jitter |

The tyre force is a spring on the *contact patch stretch*, not a function of velocity, so zero speed has a well-defined, nonzero force. Consequence for the design:
`sigma` is also how far a "stopped" truck visibly rolls back when it settles on a slope (25 mm for a 0.2 m relaxation length): keep `relaxation_length_m` realistic (0.1 to 0.3 m).

## Caveat
The grade test is a one-dimensional point mass; it proves the friction model holds at rest, not that the full 6-DoF vehicle rests without pitch jitter (that is test `stopped_tyre_on_a_grade_does_not_creep` and `truck_rests_at_the_design_ride_height_within_5mm` later).
