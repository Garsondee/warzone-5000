# Design Impact Matrix

The north-star test of the project: **every design lever must move some outcome by a plausible amount, and every outcome must have a lever that moves it.** This is ARCH's first draft of the expected table from first-order engineering; lane VALIDATION owns it from now on, refines it,
and runs it (`w5k validation impact`). Method: perturb a lever by +10% on a baseline vehicle, rerun the benchmark scenarios, compare the **sign** and, where a first-order estimate is given, the **size within a factor of 2** against the table. "~0" rows are checks too: they catch spurious coupling.
A lever that moves nothing anywhere is a **dead lever**; a benchmark nothing moves is an **orphan effect**; both are failures. Where the sign depends on the regime (grip-limited versus brake-limited) the harness first detects the regime from the force ledger.

## Benchmarks (scenario ids are VALIDATION's; units at the edges only)
| Id | Benchmark | Measured by |
|---|---|---|
| B1 | 0-32 km/h time (s) | standing start, full throttle, level hard ground |
| B2 | top speed (km/h) | level hard ground, 60 s |
| B3 | fuel range (km) | steady 50 km/h cruise on level hard ground, tank to empty (integral of the fuel rate) |
| B4 | braking distance from 50 km/h (m) | full brake, level hard ground |
| B5 | repeated-stop distance, 5th stop (m) | five stops from 50 km/h with 30 s between |
| B6 | maximum gradient (%) | bisection on a ramp, start and hold |
| B7 | side-slope limit (deg) | tilt table; first of rollover or slide |
| B8 | soft-ground mobility | maximum mass (or minimum track width) that crosses the standard mud patch without bogging |
| B9 | washboard ride roughness (m/s^2 RMS) | vertical acceleration at the hull datum, 30 km/h over the washboard |
| B10 | hump settling time (s) | time for the pitch oscillation after the speed hump to fall below 10% of its peak |
| B11 | obstacle capability | vertical step height (m) and trench width (m) |
| B12 | cornering limit (g) | steady circle, increasing speed until slide or rollover |
| B13 | pivot-turn power or minimum radius at 20 km/h | tracked: pivot on firm ground; wheeled: steady circle at full lock |
| B14 | ground pressure (kPa) and sinkage in mud (m) | static weight over contact area; Bekker sinkage |
| B15 | turret 360-degree traverse time (s) | full-rate slew (M3) |
| B16 | gun-laying error while moving (mrad) | fixed target, 20 km/h over the washboard (M3) |

## Expected effects of +10% on a lever
Sign: `-` the benchmark decreases, `+` it increases, `~0` essentially no change. First-order estimate in the last column where one exists.

### Powertrain and brakes
| Lever | Benchmark | Sign | Why (first-order physics) |
|---|---|---|---|
| Engine peak power | B1 | `-` | constant-power launch: `t = m v^2 / (2 P)`, so about -9% (less when grip- or torque-limited) |
| Engine peak power | B2 | `+` | `P = c1 v + c3 v^3`: +3% if drag-dominated, up to +10% if rolling-dominated |
| Engine peak power | B6 | `+` | tractive force at crawl speed scales with torque (not power): small, +0 to +10% |
| Engine peak power | B3 | `-` | more power available is not more fuel burned at a cruise, but a bigger engine runs further from its best point: weak, sign to be confirmed by the fuel map |
| Torque curve shape (peak moved to lower rpm) | B6 | `+` | more torque at crawl speed in first gear |
| First-gear ratio | B6 | `+` | wheel force = engine torque x ratio / wheel radius, until grip-limited |
| First-gear ratio | B2 | `~0` | top speed does not depend on first gear |
| Final-drive ratio (numerically higher) | B1 | `-` | more wheel torque per engine torque until grip-limited |
| Final-drive ratio (numerically higher) | B2 | `-` or `~0` | `-` if redline-limited (`v = omega_red r / (G_top FD)`), `~0` if power-limited |
| Gear count (closer ratios, same span) | B3 | `+` | the engine stays nearer its best-efficiency point |
| Brake torque capacity | B4 | `-` or `~0` | `-` (about -9%) if brake-limited, `~0` if tyre-limited: **a deceleration of `mu g` does not depend on brake size once the tyres are the limit** |
| Brake thermal mass | B5 | `-` | later fade onset; `~0` on the first stop |
| Fuel tank capacity | B3 | `+` | range = fuel / consumption; mass penalty on B1 is `+` and small |

### Mass, geometry and aerodynamics
| Lever | Benchmark | Sign | Why |
|---|---|---|---|
| Vehicle mass (armour added) | B1 | `+` | `t ~ m`: about +10% when power-limited |
| Vehicle mass | B4 | `~0` or `+` | `d = v^2 / (2 mu g)` is independent of mass when tyre-limited; `+` if brake-limited |
| Vehicle mass | B8 | `-` | higher ground pressure, deeper sinkage, earlier bogging |
| Vehicle mass | B6 | `-` | `sin(theta)` load grows with mass while engine torque does not |
| Centre-of-mass height | B7 | `-` | static stability: `tan(theta) = (track/2) / h`, so about -9% |
| Centre-of-mass height | B12 | `-` | rollover-limited lateral acceleration `a_y = g (track/2) / h` |
| Centre-of-mass height | B4 | `~0` or weak `+` | load transfer moves load between axles; total grip is unchanged to first order |
| Wheelbase | B13 | `+` | larger minimum turning radius `R = L / tan(delta)` for wheeled vehicles |
| Track gauge (width between wheels) | B7 | `+` | wider base resists rollover |
| Track gauge | B12 | `+` | same |
| Frontal area x drag coefficient | B2 | `-` | up to -3% when drag-dominated |
| Frontal area x drag coefficient | B3 | `-` | drag power grows with `v^3` |
| Ground clearance | B11 | `+` | belly clears obstacles; also delays belly drag in mud |
| Ground clearance | B7 | `-` | the centre of mass rises with it |

### Suspension and tyres
| Lever | Benchmark | Sign | Why |
|---|---|---|---|
| Ride frequency (stiffer springs) | B9 | `+` | above the body resonance the transmissibility grows with natural frequency: a stiffer ride is rougher |
| Ride frequency | B12 | `+` | less roll, more even tyre loads |
| Damping ratio | B10 | `-` | faster decay of the oscillation after the hump |
| Damping ratio | B9 | `+` | above resonance a damper transmits more: the classic damper trade-off (a tuned value near 0.3 balances B9 and B10) |
| Suspension travel | B9 | `-` | fewer bump-stop hits on the washboard |
| Tyre pressure (lower) | B8 | `+` | larger contact patch, lower ground pressure |
| Tyre pressure (lower) | B3 | `-` | higher rolling resistance on hard ground |
| Tyre width | B8 | `+` | the `kc/b` term stiffens the soil under a narrow tyre and softens it under a wide one: wider floats better |
| Tyre peak friction | B4 | `-` | `d = v^2 / (2 mu g)`, about -9% when tyre-limited |
| Tyre peak friction | B12 | `+` | slide-limited lateral acceleration |

### Tracks and soft ground
| Lever | Benchmark | Sign | Why |
|---|---|---|---|
| Track width | B14 | `-` (ground pressure about -9%, sinkage by at least as much) | `p = W / A`; sinkage `z = (p / (kc/b + kphi))^(1/n)` |
| Track width | B8 | `+` | floats better |
| Track width | B13 | `~0` on firm ground | turning-resistance moment `mu W L / 4` does not depend on width; on soft ground bulldozing adds a `+` |
| Track contact length | B14 | `-` | larger area |
| Track contact length | B13 | `+` | pivot moment grows with `L` |
| Track contact length | B8 | `+` | lower pressure, longer shear path |

### Turret and gun (M3)
| Lever | Benchmark | Sign | Why |
|---|---|---|---|
| Turret mass | B15 | `+` | more inertia for the same drive |
| Turret mass | B7 | `-` | higher centre of mass |
| Traverse drive effort | B15 | `-` or `~0` | `-` if acceleration-limited, `~0` if rate-limited |
| Stabiliser rejection | B16 | `-` | the gun stays on target while the hull moves |
| Gun mass (barrel) | B16 | `~0` or `+` | heavier barrel, harder to stabilise |

## Regimes (where the sign depends on what limits the vehicle)
The runner reads the regime from the baseline run's labels and applies it before comparing: the `-`/`+` rows above hold **in the regime that the row's "Why" assumes**.
- **Braking (B4):** `peak_decel_g` at 95% of `mu` or more is tyre-limited: a lever that only changes brake size or mass must give `~0`; below that it is brake-limited and the table's non-zero sign applies.
- **Side slope (B7):** `mode = slide` (the vehicle slides before it tips): centre-of-mass height, track gauge and ground clearance must give `~0`; `mode = roll` (a wheel lifts): the table's signs apply.
- **Skidpad (B12):** `limited_by` naming power: the chassis levers (centre-of-mass height, track, ride frequency, tyre friction) must give `~0`.
Added 2026-10-10 by VALIDATION after the first runs, from the physics (the tipping formula applies only when tipping happens first), not from the results' direction: the rows without a regime keep their signs.

## Coverage rule
Each lever above appears in at least one row with a non-`~0` sign (no dead levers), and every benchmark B1-B16 appears in at least one such row (no orphan effects). VALIDATION extends the table as lanes add levers; adding a lever without a row is a merge-gate failure.
