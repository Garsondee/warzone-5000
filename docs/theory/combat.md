# The physics of a gun that kicks (COMBAT lane, in plain language)

*Grows with each PR. This first section covers spike S6: turret, gun and recoil on a free hull.*

## Recoil is Newton's third law, not an animation curve
A gun fires by pushing a shell forward and the gun backward with equal and opposite impulses. Momentum is mass times velocity, so a light shell fast one way
(8 kg at 1650 m/s) is balanced by a heavy gun slowly the other way. Add the propellant gas (it also leaves the muzzle) and the gun's share is about 1.5 times
`m v`: here `J = 19.8 kN s`. A keyframed kick is a guess; this law sets its size. In the simulation the shell and gas are **ejecta**: they take `+J` along
the bore and the angular momentum `r x J` out of the system, and the test is that *vehicle + ejecta* stays at zero to one part in a million.

## The recoil mechanism turns a spike into a push
The barrel slides back at `J / m_barrel` (9.9 m/s for a 2 t barrel), faster than a sprinter, and a spring-and-damper (the **buffer** and **recuperator**) must stop it
within the stroke (0.35 m here) and bring it home. Stopping 98 kJ in 0.3 m needs about 330 kN on average: the hull feels that force for a few tenths of a second,
not the whole impulse in one frame. The ideal buffer holds a constant force (like braking at a steady deceleration); a plain linear damper pushes hardest at the
start and wastes stroke, so real ones meter the oil with a profiled rod. Energy `1/2 m v^2` goes to heat; momentum goes to the hull.

## Swivel-chair astronaut
Sit on a spinning chair, turn your torso and your legs go the other way: the total angular momentum stays zero, so the two bodies split the rotation in
inverse proportion to their inertias. A turret slewing 90 degrees on a free hull turns the hull back by `-q I_t / (I_h + I_t)`. On the ground the tracks
resist, but the same torque is there, which is why a slewing turret on a slope matters. The test: hull yaw matches that formula to 1e-8 rad.

## A servo is a spring and a damper you can steer
A PD servo pulls the joint toward a target with a spring (`kp`) and brakes it with a damper (`kd`). The **damping ratio** `z = kd / (2 sqrt(kp I))` says how much it
overshoots: 0.7 barely, 0.5 about 16%, 1 not at all (the camera "smooth damp" is the critical case). Gains are fixed hardware, but the inertia `I` changes with ammunition,
so doubling the turret mass cuts `z` by `1/sqrt(2)`: the spike measures overshoot at half, normal and double mass and the closed form predicts it within 0.05 points.
A turret on a free hull feels a smaller **effective** inertia, `I_t I_h / (I_t + I_h)`, because the hull gives way: a held hull is the stiffer, slower case.
The slew time is a trapezoid (accelerate, cruise at the rate limit, brake): `d/w + w/alpha`, plus the hydraulic delay. Stiffer gains (`kp = I wn^2`) keep the
real motion on the trapezoid; soft gains lag it by tens of percent.

## A stabiliser is a gimbal with a bandwidth
A camera gimbal measures how the handle moves and counters it. The gun stabiliser does the same with the hull's rate: it commands `-r x` (the hull's motion, smoothed at the
bandwidth `f_b`). Below `f_b` it rejects the fraction `r` of the hull motion; above, it cannot keep up. Its rejection can be no better than the servo's own speed
(the rate loop) and the delay in it: a 50 ms hydraulic servo cannot deliver a 3 Hz gimbal. The law `|1 - r/(1 + j f/f_b)|` is checked to 1% for a servo that can.

## Why the integrator matters (and why Euler is not enough)
Momentum is a sum over bodies of mass times velocity, but each body's velocity depends on the joint angles, and the angles change during a step. A first-order step
(semi-implicit Euler) ignores that change and leaks about `10^-3` of the impulse; a fourth-order step (RK4) leaks `10^-8`. Splitting hull and gun into two solves with a one-step
delay is worse: the explicit coupling goes unstable when the gun is heavy compared with the hull (the same reason a boat pulling a heavy barge by an elastic rope oscillates).
So the hull and the chain are solved together, as one mass matrix `M(q)`, a 9 by 9 system: a small cousin of the articulated-body solve in character rigs.
