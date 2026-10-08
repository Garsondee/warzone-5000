# Theory Note: the physics of a time trial (M1c)

The ideas behind the first vehicles that really move, one small principle at a time. Each section says what the principle is,
where it shows up in the code, and what a player will feel. The design is in [08-time-trial.md](../design/08-time-trial.md).

## 1. A vehicle on a hill is a ball on a smoothed hill
Newton's second law along the slope: `m a = F_thrust - W sin(theta) - resistances`. The pull of the slope is the weight times the
sine of the angle: 8 % downhill is `sin(theta) ~ 0.08`, so gravity alone accelerates a vehicle at 0.78 m/s^2 (a standing start of
8 m/s in ten seconds, with no engine at all).

A vehicle is not a point. It rests on two or more support points and feels the slope *between* them: the height difference over
the span, `(h(s + L/2) - h(s - L/2)) / L`. That is the terrain **averaged over the vehicle's length with a box filter**, and it has a
lovely consequence. Write the box-filtered height `Phi(s)`; the force is then exactly the slope of `Phi`, so it is a *conservative* force
and energy is conserved in `Phi`: `v^2 / 2 = g (Phi(start) - Phi(now))`. A test checks the simulation against that law (and against an
independent integration in doubles) to 1 %. The start of a ramp is a good example: with a 4 m span the first 2 m of the ramp are felt
at half strength, and `Phi(0) = grade / 2`, not 0.
> **From graphics:** this is texture filtering. A long vehicle samples the terrain at a coarser mip level than a short one; the
> bumps shorter than its span vanish. The same reason a dreadnought glides over ground that shakes a scout.

*What a player feels:* long vehicles are smooth over rough ground and slow to react to a short hill; short ones follow every
undulation.

## 2. Power, force and speed: a hyperbola
`P = F v`. An engine delivers *power*, not force: at speed `v` the most it can push with is `F = P / v`. That is a hyperbola: lots
of force at low speed, little at high, and **infinite at a standstill**, which is absurd (no car pulls with a million newtons from
rest). Real drives have a **lowest gear**: below some speed the force stops growing and stays constant. In the simulation the lowest
gear is a fixed fraction of the rated speed (0.2, a 5:1 spread): `F = P / max(v, 0.2 v_rated)`.

Two limits on acceleration follow, and **the lower one wins**: the *engine* (`P / v`) and the *ground* (`mu N`: you cannot push on the
ground harder than it pushes back). A powerful vehicle is ground-limited at the start (the wheels would spin) and engine-limited
once it is moving: the "limited by" ribbon in the viewer switches from amber to cyan as it launches.

A **governor** keeps a vehicle at the speed its running gear is rated for. A hard clamp (stop pushing at exactly 70 km/h) would make
the engine irrelevant above that and creates a kink; we multiply the thrust by `1 - (v / v_rated)^32`, which is 1 until about 90 %
of the rating and falls smoothly to 0 at the rating, computed with five squarings (no `pow`).
> **From graphics:** a smooth clamp instead of a hard clip is the *shoulder of a tone-mapping curve*. ACES, Reinhard and friends
> all do the same: compress smoothly toward the limit instead of clipping, so that nothing is lost abruptly and the curve is
> differentiable (which a physics integrator also likes).

## 3. Top speed is a power balance
At a steady speed the thrust equals the resistance and the engine's power equals what the resistance costs: `P = c1 v + c3 v^3`
(rolling resistance `c1 = C_rr m g` costs power in proportion to speed, air drag `c3 = 1/2 rho C_d A` in proportion to its *cube*).
The vehicle sheet solves this with Newton's method; the simulation arrives at the same speed by integrating the second law. Making
two independent calculations agree (a test requires 5 %, and most designs agree to 1 %) is the strongest check we have that both are
right.

The approach takes time: near the balance, `dv/dt = -(P/v^2 + 2 c3 v) / m x (v - v_top)`, a time constant of `m / (P/v^2 + 2 c3 v)`. A
20 tonne tank needs about 50 s to settle; the 403 tonne Sky Battery about 300 s, so on a 370 m course it never reaches its top speed.
*Heavy things are slow to get going*, and the sheet's "top speed" is only the end of a long road.

## 4. Grip, slip and the steepest hill
The ground can push back with at most `mu N` (`N` the load on it, `W cos(theta)` on a slope). A standing start on a slope has the
engine force at its lowest-gear value and needs `F >= W sin(theta) + C_rr N`: with `F = mu N` this gives the steepest hill a vehicle can
start on, `tan(theta) = mu - C_rr` (a test finds the edge at 0.84 for `mu = 0.9`, `C_rr = 0.06`). Concrete is 0.8-0.9; a lugged
tyre on loose ground much less, which is why soil thrust (section 6) is a different law.

## 5. Coherent units
Work in **tonnes, kilonewtons, kilowatts, metres and seconds**. Then 1 kN / 1 t = 1 m/s^2 and 1 kN x 1 m/s = 1 kW: forces, powers and
accelerations never need a conversion factor, and the numbers stay small. (In newtons and joules a 1,400 tonne walker has a weight of
14 million, and 14 million squared overflows a 32-bit integer part.) A Q32.32 fixed-point number holds up to 2 billion with a
resolution of 0.2 nanounits; every quantity in a run fits with room to spare, and a debug build (which panics on overflow) runs the
same hashes as a release build, which is the proof.
> **From graphics:** coherent units are to physics what *linear colour space* is to lighting. Do the arithmetic in the space where it
> is simple and exact; convert only at the edges (loading content, drawing the HUD).

## 6. Soft ground is a spring and a wedge
Two laws, each in its simplest form.

**The spring (Bekker).** Press a footprint into soil and it sinks until the soil pushes back: `p = k z`, with a stiffness
`k = k_c / b + k_phi` that depends on the footprint's width `b`: a *narrow* footprint is stiffer, because the soil beside it helps
carry the load. The sinkage is `z = p / k`, with `p = load / contact area`. A 50 kPa tank on 0.6 m tracks sinks 19 cm; a 100 kPa one 37 cm.

**The wedge (Mohr-Coulomb).** Soil gives way when its shear stress passes `c + p tan(phi)`: cohesion (grains stuck together) plus
friction (grows with the pressing load). So the most thrust a footprint can get is `H = A c + N tan(phi)`: *not* a fixed fraction of the
weight, and for the same soil it is a *higher fraction of the weight for a light vehicle* (`c / p + tan(phi)`, which falls as the
pressure rises). Real soil also needs shearing before it gives its strength; the exponential law is `1 - (1 - e^-x)/x`; we use the
hyperbola `x / (x + 2)`, same slope at the start and within about 12 % after, with `x = 0.6 L / K`: a long footprint shears far and gets
nearly all of it (a track: 98 %), a short one less (a tyre patch: 84 %).
> **From graphics:** `x / (1 + x)` is the **Reinhard tone-mapping curve**. The soil's thrust law and the way a monitor compresses a
> bright pixel are the same saturating hyperbola, because both describe a response that starts linear and approaches a limit.

## 7. Why tracks float and tyres bog
Sinking costs work: pressing the soil down by `z` under a footprint of width `b` takes `b * integral(p dz) = 1/2 b p z` (the area of the
triangle under `p = k z`), the **compaction resistance**. Per unit of weight it is `R_c / W = z / (2 L)` for a footprint of length `L`.
A 5 m track: `0.37 / 10 = 3.7 %` of the weight even at 100 kPa. A 0.4 m tyre patch at `z = 0.19 m`: `0.19 / 0.8 = 24 %`. That is the whole
reason tracks are used on mud, and it is *derived*, not assumed: the long footprint is a long ramp that the vehicle climbs out of
gently, the short patch is a step it must climb continuously.

A tracked vehicle therefore bogs a different way: when the sinkage reaches the **ground clearance** the belly meets the ground
(*high-centring*) and drags with a force we ramp up as `(z / clearance)^3` rather than switching on suddenly, so a design near the edge
slows before it stops. Heavier plating on the same tracks and engine shows the whole story in one chart: smooth slowdown, then a cliff
(the ladder chart in the design doc).

## 8. Legs pay inside the machine
A walker's **cost of transport** is the power it spends per unit of weight per unit of speed: about 0.6 for our walkers (ten times a
track's rolling resistance). It is spent in the actuators, not against the ground, so it comes off the engine's power *before* the grip
limit: `F = (P - c W v) / v`. At the speed where this reaches zero the walker stops accelerating: `v_top = P / (c W)`.

It is tempting to treat a walker like a cart: let gravity accelerate it downhill. A free-rolling body on an 8 % slope gains 0.78 m/s^2 for
as long as the slope lasts; a walker does not roll, its legs carry it. The honest model is an **energy balance**: downhill, the slope
pays part of the cost, so the steady speed is `P / (W (c cos(theta) - sin(theta)))`, about 15 % faster on 8 %, not the 2.4 times a cart
would reach. (The first version of the simulation got this wrong, and the spider walkers finished in 100 s instead of 154 s: a test now
checks both the downhill and the uphill balance in closed form.) In soft ground each foot is pressed in once per stride, which costs
`N z / stride` of force: the weight times the sinkage, per stride.

## 9. A replay is the interface; determinism is the contract
The simulation writes down what happened: 20 bytes per tick, in millimetres, milliradians and percent of weight, plus *what limited
the vehicle that tick*. Viewers only read it. The browser page, the MP4 and the future Godot player are three different ways to look at
the same record, the way a deferred renderer writes a G-buffer once and lets many passes shade it.
> **From graphics:** the replay is a *G-buffer for time*. Because every viewer reads the same buffer, adding a viewer never touches
> the physics, and a change to the physics shows up in every viewer at once.

For that to hold the simulation must be **bit-identical everywhere** (lockstep multiplayer, replays you can verify, desync
detection). Floating-point maths is not: the same code can give different last bits on different compilers and CPUs. So the run is
Q32.32 integers, with floats appearing only when a file is *loaded* (one correctly-rounded conversion each), no `pow`, `exp` or `ln`
(those are libraries and differ), integer sub-steps (three 60 Hz steps per 20 Hz tick) and a state hash chained every second. The
golden test replays every design, in debug and release, on one thread and eight, and on Linux and Windows in CI, and compares hashes.

## 10. Animation derives from the replay, so it costs nothing in determinism
A rolling wheel turns by the distance it has rolled over its radius, `theta = s / r`; if it slips, the rim runs faster than the ground
moves under it: `theta = s / (r (1 - i))`. A walker's gait is a cycle that advances with the *distance travelled* (a stride is 1.6
times the stance): stand still and it stands, slow down and it steps slowly. During the stance the foot must stay put on the ground
while the body passes over it, so the hip swings through `+-amp` with `2 R sin(amp) = duty x stride` (the chord of the foot's arc is
the 60 % of a stride the foot is down), which gives `amp = asin(0.6 stride / (2 R))` and no foot-sliding. Alternate legs half a cycle apart
and the feet on the ground always form a tripod.
> **From graphics:** at 24 frames a second a wheel turning 0.37 revolutions per frame can look like it turns backwards: the
> wagon-wheel effect is aliasing, the sampling theorem applied to rotation. No fix is wanted; the same thing happens in every film.

## 11. Reading a bog
A bogged vehicle is stopped by a contest between what pushes it and what resists, and the explanation says which side was short:
* **the engine was short** (the soil would have given more than the drive could deliver at this speed): more engine power, or a lower
  speed rating (gearing: the force at a crawl is `P / (0.2 v_rated)`, so a vehicle geared for 140 km/h pulls half as hard at a crawl as one geared for 70);
* **the ground was short** (the drive was asking for all the soil would give): a longer footprint shears more of the soil, a wider one
  sinks less; more power does not help (a test shows 3,000 kW cannot pull a hull off the ground).

The remedy is a design change in either case, which is the point: the bog is the game telling the designer where the design is weak.
