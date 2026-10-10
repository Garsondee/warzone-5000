# Theory: from a designer's choices to solver parameters (FORGE)

*Plain-language notes; grows with each milestone. Graphics analogy first, physics second.*

## Design levers versus solver parameters
A rig in your animation package has *controls* (a slider that curls a finger) and *joint rotations* it drives. The solver only understands joint rotations. FORGE is the rigging layer: the designer's controls (ride frequency, damping ratio, peak power) are the sliders, and spring N/m, damper N s/m and a torque table are the joints. Each slider must move exactly the right joints by the right amount, and the compile report is the "what did my slider do" panel. A slider that moves nothing is a bug, as it would be on a character rig.

## Why ride frequency is the handle on a spring
A mass on a spring bounces at `f = (1/2 pi) sqrt(k/m)`. Solving for the spring: `k = m (2 pi f)^2`. A designer feels *frequency* (1 Hz is a floaty ride, 2.5 Hz is a truck): it does not change when you load the vehicle in the way a spring rate would. So we store frequency and compute the rate from the sprung mass. Heavier vehicle, same feel, stiffer spring. Doubling `f` quadruples `k`.
**The tyre is a second spring in the same load path.** Two springs in series are softer than either alone: `1/k = 1/k_s + 1/k_t`. On our stand-in the tyre makes the ride 27% softer than the coil alone, so the compile solves backwards for the coil, otherwise the slider would lie by a quarter.
**Damping ratio** `zeta = c / (2 sqrt(k m))` is the fraction of the damping that makes a bounce just stop without overshoot (zeta = 1). Road cars sit near 0.3: a couple of visible bounces. Same logic as `k`: store the dimensionless handle, derive `c`.

## Fitting an engine curve through two numbers
Spec sheets give two peaks: maximum torque at one speed, maximum power at a higher one. Power is `P = T omega`, so the two peaks are not independent: at the power peak the torque is `P / omega`, and it must be *below* the torque peak, otherwise power would still be climbing. We fit a smooth curve that hits both exactly: flat-topped at the torque peak, then a cubic falling away so that the slope of power is zero at the power peak. (A first attempt with one cubic shot to 400 N m at idle, so the low side is a gentler parabola.) Think of it as two control points with tangents on a spline.

## The parallel-axis theorem (coming with the mass kernel)
Moving a pivot in a rig changes how hard a limb is to swing: mass far from the axis counts with the *square* of the distance. `I = I_cm + m d^2`. A battery pack put in the nose of a vehicle raises its pitch inertia far more than the same pack near the middle, so we sum every item's own inertia plus `m d^2` about the common COM.

## Provenance as asset versioning
Every number carries where it came from (SPEC, MEASURED, ESTIMATE with a band, TUNED), like a texture carries its source and licence. A reference vehicle with ESTIMATE where a SPEC exists is a known debt, shown in the design sheet.
