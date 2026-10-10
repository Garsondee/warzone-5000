# The physics of a chassis (CHASSIS lane, in plain language)

*Grows with each PR. This first section covers the quarter car.*

## A corner of a car is two masses and two springs
Take one wheel's share of a truck: the **sprung mass** (hull share, about 400 kg), the **unsprung mass** (wheel, tyre, hub, about 50 kg), the **spring and damper**
between them, and the **tyre**, which is another spring to the road. Two masses means two ways to wobble: the **body mode** (the whole hull bouncing on the
suspension, about 1.3 Hz, like a boat on swell) and the **wheel hop** (the light wheel shaking between road and spring, about 12 Hz). A car that feels "floaty" has a low
body frequency; one that feels "harsh" has a high one. Designers pick **ride frequency** and **damping ratio** and the spring and damper rates follow:
`k = m (2 pi f)^2`, `c = 2 zeta sqrt(k m)`.

## Damping ratio, and the camera analogy
`zeta` is how much of the bounce survives each cycle: 0 never stops, 1 returns without overshoot (critical damping), cars sit at 0.2 to 0.4 so they absorb a bump but
do not feel dead. A critically damped spring is exactly the "smooth damp" every engine uses to make a camera follow a target. You can measure `zeta` from a ringing
trace: each peak is smaller than the last by a fixed ratio, and the natural log of that ratio (the **log decrement** `delta`) gives `zeta = delta / sqrt(4 pi^2 + delta^2)`.

## Why the simulation steps the way it does
We update velocity from force first, then position from the *new* velocity (**semi-implicit Euler**). Unlike plain Euler, this does not add energy every step: it conserves a
slightly distorted energy exactly, so an undamped oscillator rings forever instead of blowing up or dying. The bench test measures the energy after a minute of ringing and
requires less than 0.1% change. The price is that the step must be small compared with the fastest oscillation: stable below `omega dt = 2`, accurate with about 20 steps per period
(spike S1, `docs/lanes/chassis/spike-s1.md`).

## How we know it is right: oracles
The two-mass system has an exact answer. With `det [[k - ms w^2, -k], [-k, k + kt - mu w^2]] = 0` the natural frequencies come out of a quadratic in `w^2`. The tests
(`quarter_car_*`) simulate and compare with that closed form: frequency within 1%, damping ratio within 5% (in the limit where one mass is rigid and `zeta = c / (2 sqrt(k m))`
is exact), static deflection equals `m g / k`.

## The suspension element: spring, damper, stop
**Spring.** A coil is `F = F0 + k x`. Real suspensions are rarely that simple, so the element supports every kind in the rig: a *table* (leaf packs and rubber, force read off a curve); a
*torsion bar* (the bar twists through an arm, so the force at the wheel is `T / (L cos phi)`: as the arm rises its lever shortens and the preload's push on the wheel changes, so the wheel rate at
rest is `k / (L cos phi)^2 - P sin(phi) / (L cos^2 phi)`, not just `k / L^2`); and a *hydropneumatic* strut (a gas bag: `p V^gamma = const`, so it gets stiffer the more it is squeezed,
a spring that is progressive for free). A spring can push but never pull, so its force is floored at zero.
**Damper.** Force proportional to speed, `F = c v`, with a bigger coefficient in rebound than bump (so the wheel drops back slowly but the hull is not hammered by a bump), and a *knee*: above a
set speed the damper bleeds (digressive), because a valve blows off. Dry friction in a leaf pack is `F_f tanh(v / v_s)`: Coulomb friction smoothed over a small speed `v_s` so the
integrator does not chatter around zero speed.
**Bump stop.** A rubber block that engages near full travel, stiffening as it squashes. If a rig declares a *hard* limit, the travel is clamped and the approach speed is reflected with a
restitution coefficient: a bounce, not a spring that grows without bound.

## The tyre: a slip-force curve, a friction circle, and a patch that remembers
**Slip.** A tyre produces force only when its patch slides a little against the road. Longitudinal *slip ratio* `kappa = (wheel speed - ground speed) / speed`; sideways *slip angle*
`alpha`, the angle between where the wheel points and where it is going. For small slip the force is a straight line: `Fx = C_kappa Fz kappa`, `Fy = -C_alpha Fz alpha` (the
stiffness is "per unit load", which is why a heavy truck on the same tyres corners harder). The tests check those slopes against the stated stiffness.
**Friction circle.** The road can give at most `mu Fz` in total, in any direction. Spend it all on braking and nothing is left for steering: the force *vector* is capped on a circle of radius `mu Fz`.
**Relaxation length.** The rubber must roll about one relaxation length (0.1 to 0.3 m) before it builds its full force. We model the *stretch of the contact patch* as a state that grows
with slip speed and relaxes as the tyre rolls: `du/dt = slip_velocity - (speed / sigma) u`, force `= (C Fz / sigma) u`. At speed it settles to `sigma * slip`, the curve above.
**Why not a stick/slip switch.** The textbook Coulomb rule "friction = -mu N sign(v)" gives *zero* friction at exactly zero speed, so a parked truck creeps (spike S1 measured 1.6 mm per second on
a 10% grade). A stretched patch is a spring: at rest it keeps its stretch, supplies exactly the force needed to balance the slope, and nothing moves. Graphics analogy: it is the same trick as a
spring-based "grab" constraint instead of teleporting a held object. The tyre holds a small residual stretch (25 mm for a 0.2 m relaxation length on 10%): that is the visible price.
**Aligning moment.** The force acts a little behind the middle of the patch (the pneumatic trail), so a sideways-sliding tyre also tries to straighten itself; the trail shrinks to zero as the patch saturates, which is why steering goes "light" at the limit.
**Rolling resistance.** `Crr x Fz`, opposing motion, fading to zero at standstill over a small speed so a parked truck is not shoved backwards.

## The hull: a rigid body with six degrees of freedom
**Wrench.** Every force on the hull (a strut, a tyre reaction, gravity, drag) is added to one running total during a substep, together with its turning effect about the centre of mass,
`r x F` (the lever arm crossed with the force). A push up at the nose gives a torque about +X, which raises the nose: that is how a braking truck dives. The pair (total force, total torque) is the *wrench*.
**Why we integrate angular momentum, not angular velocity.** A spinning body's inertia tensor turns with it. In the world frame `I_world = R I_body R^T` changes every step, so `omega` can change
with no torque at all (the gyroscopic effect; it is why a thrown phone tumbles when spun about its middle axis). The quantity that does *not* change without torque is the angular momentum
`L = I omega`. So the state is `L`: `L += torque dt` (exact), then `omega = I_world^-1 L`, then the orientation advances by `omega dt` (the exact exponential map on the quaternion). With no torque
`L` stays constant to rounding error (the test checks 1e-12 over 20 s of tumbling about the unstable intermediate axis). Graphics analogy: storing `L` is like storing a world-space quantity and
deriving the local one each frame, instead of accumulating error in a value that the frame change keeps invalidating.
**How the tests know it is right.** Free fall: semi-implicit Euler drops exactly `g dt^2 n(n+1)/2`, which is `g t^2 / 2` plus a small `g t dt / 2` lag (0.2% after 1 s at 240 Hz). Energy: a hull
on four undamped corner springs, heaving, pitching and rolling, keeps its oscillation energy to 0.1% over a minute. As a check that the test has teeth, swapping to plain (explicit) Euler makes the
same test fail by a factor of about 3e8.
