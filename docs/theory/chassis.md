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
