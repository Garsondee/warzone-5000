# Theory: the powertrain (lane DRIVE)

Plain-language notes on why `w5k_drive` is built the way it is. Graphics analogies where they help.

## Power is torque times speed; a gearbox is a lever
An engine delivers torque T at angular speed w, and power is P = T w. A vehicle needs *force at a speed*, F v. A gear ratio r (input speed / output speed) is a lever:
the output turns r times slower and pushes r times harder, P stays the same (less friction). So the gearbox does not make power; it chooses *where on the
engine's curve you work for a given road speed*. A flat diesel torque curve means usable pull across a wide rpm range; a peaky petrol engine needs more gears to stay
near its power peak. Plotted as force against speed, each gear is a hyperbola F = P / v clipped by the traction limit at low speed and by the redline at high speed; the
shift diagram is those hyperbolas laid side by side.

## A clutch is a constraint, not a spring (spike S-D)
Two spinning inertias joined by a friction clutch obey Coulomb friction: the clutch can carry *up to* a capacity, and either sticks (same speed) or slips (at capacity).
The tempting model, torque = capacity * tanh(slip / eps), is a very stiff spring: it needs a time step shorter than eps J / capacity or it chatters and invents energy.
Instead each step asks: "what torque would make both shafts equal at the end of this step?" and clamps the answer to the capacity. That is the same idea as a
contact solver with a friction cone: solve for the force that satisfies the constraint, then clip it. It is stable at any step and sticks exactly.
The converter is different: its torque is (w / K)^2, which *falls* as slip falls, so it is smooth and only needs a linearised implicit step. It also multiplies torque
at stall (the stall ratio) and falls to 1 at the coupling point; a lock-up clutch then removes the slip that otherwise wastes fuel.

## Why a diesel curve is flat and a petrol one peaky
A diesel runs lean and varies fuel only; the torque it can make is set by how much air it swallows, which is roughly constant per revolution (and boosted by a
turbo at low rpm), so torque is flat and power rises linearly with rpm until the governor. A petrol engine throttles its air; volumetric efficiency has a resonance peak,
so torque peaks in a band and power peaks later. Fuel use follows the same shape: specific consumption (g per kWh) is best at high load, mid speed.

## Shift logic is a Schmitt trigger
If the upshift and downshift points were the same, the box would flip back and forth at that speed. Like debouncing a button in a UI, the points are different
(hysteresis), and the change takes time (dwell). We add three more guards that came from watching real runs: the shift map reads a *smoothed* pedal (so a one-frame
blip cannot kick down), the downshift curve stays under the upshift curve at every pedal position (so a pedal swing cannot turn an upshift into a downshift), and a shift
is refused if it would land next to the opposite point. The map runs on the gearbox *input* speed in each candidate gear, which is exactly what the gear we would land in
will see; engine rpm is blurred by converter slip. A manual box under automatic control is shifted by the same rules with the clutch opened for the shift time (torque
interruption: the vehicle coasts, losing speed of load*time/inertia).

## Pulling away: the clutch and the hill
A clutch passes torque only up to its capacity, so a driver pulling away hard slips it and holds the engine near its torque peak: all the engine's pull reaches the wheels while the
car is still slow. An engine that is allowed to sag to idle-ish revs, or a clutch that grabs by engine speed alone, delivers only the torque found at those lower revs. Gearing makes
the same point from the other side: first gear has the most wheel force, and on a steep grade a gear change is expensive because for 0.5-0.8 s no drive reaches the wheels while
gravity pulls the truck back (about g sin(theta) x t = 2 m/s on a 25% grade). So the automatic refuses an upshift at low speed unless the higher gear could pull the present load with room
to spare, the same reasoning a production "grade logic" uses.

## Differentials: split torque, not speed
An open differential gives each side an equal share of the torque and lets the speeds differ. Its famous weakness follows directly: the torque on *both* sides is limited
by the side that can transmit least, because the shares are equal. A wheel on ice cannot push back, so it takes half the torque to spin freely and the other wheel gets the
same tiny amount. A locked differential adds the constraint "same speed", solved like the clutch above, so the grippy wheel gets whatever torque the vehicle needs.
A limited-slip differential is in between: it moves torque toward the slower (gripping) wheel, up to a bias ratio (for example 2.5 times the other side), growing with the
speed difference. It never creates torque; it only redistributes it. This is why the three trucks separate off road: the lock lets a stuck axle share the load.

## Brakes are a leaky bucket
Braking turns kinetic energy into heat in a small piece of metal. Heat in is the braking power; heat out is cooling, proportional to the temperature above ambient and
rising with airflow (vehicle speed). The temperature is the water level in a bucket with a hole: it rises quickly in a long stop and drains slowly. Above a threshold the friction
coefficient falls (fade), so the same pedal brakes less exactly when it is needed most; a long descent is the test. A brake must also never push a stopped shaft backwards:
it is solved as "the torque that leaves the shaft stopped, clamped to capacity" so a held shaft stays held.

## Why tracked vehicles need a steering unit and cars do not
A car steers by turning wheels and lets an open differential absorb the speed difference between the inside and outside wheels. A tracked vehicle cannot turn its tracks;
it must drive one side slower than the other, so it needs a mechanism that deliberately creates that difference. Four families, and what each one fixes:
- **Controlled differential** (brake steering): an open differential feeds both tracks equal torque; braking the inside track makes it slow and, because the differential
  keeps the *sum* of the two speeds, the outside track speeds up. The turn radius is whatever the brake gives you, and the brake pays in heat.
- **Clutch-brake** (Sherman, T-34): both tracks are driven straight through. A little stick disengages the inside clutch (the track coasts, the tank drifts wide); more stick
  brakes it (a tight turn). It wastes the power it brakes away, and a stuck-on clutch makes it a tank that cannot go straight.
- **Double differential** (Tiger II, Merritt-Brown): a second differential adds a speed difference proportional to the demand, so each gear has a fixed turn radius
  (the *ratio* of the track speeds is fixed). No power is thrown away: the slow track's power flows to the fast track.
- **Hydrostatic**: a pump and motor fix a speed *difference* (rad/s) regardless of the vehicle speed, so the turn radius grows with the speed, and, being fed from the engine
  side, it works with the gearbox in neutral (a pivot turn on the spot).
In the code the first steers through the sprocket brakes; the clutch-brake locks the two sides and frees the inner clutch (brakes after half stick); the other two are an implicit
servo on the speed difference (the same constraint-solving trick as the clutch and the locked differential), limited by the unit's torque capacity.
