# Theory Note: movement, weapons and mixing (M1b)

The ideas behind the second batch of families and the explorer, one small principle at a time. Each section says
what the principle is, where it shows up in the code, and what a player will feel.

## 1. The square-cube law: why scale has physics
Double every length of a machine and its **surface and cross-sections grow by 4** (L^2) but its **volume and mass by
8** (L^3). Strength follows area; weight follows volume. So a bigger machine is *relatively weaker*.

- **Walkers.** A leg carries a load of mass m ~ L^3 on a section A ~ L^2, so stress ~ L. A 20 m walker cannot be a
  scaled-up 2 m walker: its legs must be proportionally fatter. The walker size ladder (`w5k ladder`) shows it: the
  fitted leg thickness grows from 0.03 to 0.11 of the leg length as the body grows 15 times.
- **Fliers.** Lift needs power that grows faster than weight (sections 5 and 6), so large rotor craft and cushions must
  be built lighter and lighter: the fitter's "diet" thins their armour as they grow (`space_scale_armour.png`).
- **Anti-gravity** is the exception we chose: its cost is linear in weight, so titans can float, at a flat price.

## 2. Legs: buckling beats strength
A column fails in one of two ways: it **crushes** (stress above the material's strength, F = sigma A) or it
**buckles** sideways (Euler, F = pi^2 E I / L^2). I is the second moment of the section, so for a tube it grows with
the *fourth* power of the section size, and a long thin leg fails by buckling long before it crushes. The Legs family
computes both for each segment of the leg, takes the smaller, knocks it down for bending, divides by a safety factor
and a dynamic factor (landing loads), and rates the leg. Stiffness (the modulus E), not strength, is what tall legs
need: composite is strong but flexible, steel is both.
The hip must then produce a torque F x reach, and actuator mass follows from torque per kilogram. The gait speed
limit is a Froude number, v^2 / (g h) <= 1 (people switch from walking to running at about 0.5): taller legs may go faster
(as sqrt(h)), exactly as animals do.
> **From graphics:** the same "second moment of area" governs the bending stiffness of a bone, a ray-traced
> cylinder's silhouette thickness, and a mast; it is the reason hollow tubes are efficient.

## 3. Wheels, tyres and ground pressure
A tyre carries a load F = p x (width x contact length), the inflation pressure times the contact patch, and
puts that same pressure p on the ground. A low-pressure tyre floats on soft soil and wastes energy on hard roads
(rolling resistance grows as the pressure falls); a big wheel rolls over a bump instead of climbing it. Soft ground
gives way at about 50-100 kPa: a tank presses about 90, an air cushion a few.

## 4. Rail: almost no resistance, almost no grip
Steel wheels on steel rails roll with C_rr of about 0.0015 (a twentieth of a tyre), so a train needs little power to
keep moving. But the tractive force is limited by adhesion, F_max = mu x (driven weight), and mu for steel on steel is
only about 0.25. Rail is fast, efficient and bound to the line.

## 5. The air cushion: leakage is the cost
The fans keep the cushion pressure p = W / A just above the weight. Air escapes under the skirt through a gap at the
speed Bernoulli gives, v = sqrt(2 p / rho), so the flow is Q = perimeter x gap x v and the fans must supply power P =
p Q / efficiency. At a fixed footprint p grows with W and v with sqrt(p), so **P grows as W^1.5**. A bigger footprint
lowers p (cheaper to float) but is a bigger, easier target. A taller skirt clears waves and ground clutter but leaks more.

## 6. Rotors: the actuator disc
Treat the rotor as a disc that pushes air down. To hold weight W the disc must give the air momentum, and the
cheapest way is a large mass of air moving slowly: the induced speed is v_i = sqrt(W / (2 rho A)) and the power is
P = W v_i = **W^1.5 / sqrt(2 rho A)**, divided by a "figure of merit" of about 0.7 for real blades. Doubling the disc
radius quadruples A and **halves** the power; this is why helicopters have huge slow rotors, and why quadcopters
(four small discs) are power-hungry. Separately, blades stall when they are asked for too much lift per area:
thrust T <= C_T rho A V_tip^2, with C_T of about 0.12 sigma (sigma is the solidity, blade area over disc area). So
**lift capacity** (rated lift) and **hover power** are different limits, and the family reports both. Arm rotors may not
overlap their neighbours, so their radius is capped by the room between stations; a single big mast rotor has no such cap.

## 7. Anti-gravity: a deliberate design choice
There is no real physics here, so the *cost model* is a design decision: power = k kW per tonne held, however big
the pod, times (1 + 4 % per metre of ride height). Linear in weight means titans can float; a flat price in kilowatts
means they pay it. A better field (smaller k) needs more coil turns, so a bigger and heavier pod.

## 8. Missiles: the rocket equation and similarity
A missile is a body whose mass splits into **warhead, propellant and structure**. The Tsiolkovsky equation gives its
burnout speed dv = Isp g ln(m0 / m_final): more warhead means less propellant, so less range. *Punch against reach* is
the family's central trade (the "warhead share" slider). Geometric similarity gives mass ~ d^3, so the warhead
and the range scale with the cube of the calibre; a shaped-charge jet penetrates about six calibres of steel (with
diminishing returns above 150 mm).

## 9. Beams: aperture is resolution
A beam leaves an aperture of diameter D and spreads by diffraction, with half-angle about 1.22 lambda / D (the Airy
disc). **A bigger aperture keeps the beam tight over a longer distance**, exactly as a bigger lens resolves finer
detail or a wider aperture lowers the diffraction limit in a camera. Turbulence adds a floor to the spot size, so we
model r(R) = 15 mm + 0.06 mrad x R / (D / 0.3 m). The depth a beam melts in time t is E x coupling / (pi r^2 x 10^10 J/m^3)
(10^10 J/m^3 is the energy to melt a cubic metre of steel), which falls with r^2: a tight beam is *enormously* more
destructive than a wide one. Energy is stored in capacitors (25 kJ/kg), which is where the mass goes.

## 10. Sight: the horizon and the radar equation
A sensor at height h sees to the horizon d = sqrt(2 R h) (plus the target's own horizon), so **height buys range
with a square root** and a mast pays for it in mass (a column that must stay stiff). Optics see detail but not far;
radar follows R^4 ~ P G^2 / ..., and since the antenna gain G grows with its area, range grows about linearly with the
aperture and with the fourth root of the power.

## 11. Auto-fit is a fixed-point iteration
The fitter (`Explorer::fit`) solves a coupled problem: the gear must carry the weight, but the gear is part of the
weight; the engine must power the vehicle, but the engine is part of the vehicle. We iterate: measure the mass, size
the gear and engine for it, measure again. The mass changes less on each pass (the engine is a small part of the
total), so the loop is a **contraction** and settles in five to ten passes. When no setting of the sliders can satisfy
the load, the fitter projects back into the feasible set the only way the physics allows: by shedding mass (the diet).
> **From graphics:** the same pattern appears in iterative solvers (Gauss-Seidel in radiosity, relaxation in cloth
> simulation, fixed-point refinement of a pose).

## 12. Sampling the possibility space
A uniform random sample over a huge product space (hull x gear x weapons x sizes) over-represents the common combinations
and under-represents the rare ones. We **stratify**: every hull-gear cell gets the same number of attempts, then the
sizes and weapons inside the cell are random. It is the same idea as stratified sampling of a pixel in a path tracer:
equal coverage first, randomness within, lower variance for the same budget.
To compare things measured in different units (kilowatts, millimetres, kilometres per hour) we use **percentile
ranks** (the fraction of designs below this one), the same trick as histogram equalisation in tone mapping: it turns any
distribution into a flat one so axes can be mixed. Spearman correlation is just the Pearson correlation of those ranks.

## 13. Shadow prices and frontiers
A **constraint only shapes decisions where it binds.** Every design has a mass budget, but nothing in the model makes spending
mass on speed *hurt*: the engine is a small share of the vehicle (roughly 2-15 %) and each kind of running gear tops out at a rated
speed. So speed does not compete with armour or firepower, and the sample shows rank correlations near zero. Economists call the
cost of a constraint its **shadow price**: how much better the objective gets if you relax the constraint by one unit (the
Lagrange multiplier of an optimisation). Speed's shadow price here is nearly zero, so every player will always buy it. A second
budget, a price in command points, makes the three traits compete. The sampler is how we notice a shadow price of zero.

The cloud is not the frontier. A random roll scatters designs everywhere, while a player (or the AI designer) wants the **Pareto
frontier**: the designs that no other design beats on every trait at once. In the armour-firepower chart it is the white line.
In two traits it is a skyline query (sort by one trait, sweep along it keeping the running best of the other); in more traits the
same idea gives a trade-off surface.
> **From graphics:** the frontier is the silhouette of the cloud seen from the corner where both traits are large, built by the
> same sweep as a one-dimensional z-buffer. The trade-off *is* the shape of that edge: steep means one trait is expensive in
> terms of the other, flat means it is nearly free.

## What to distrust
Every constant here (the tyre pressure limit, the melt energy, the rotor stall coefficient, the anti-gravity cost, the
leg safety factors) is a *hypothesis chosen to give a believable shape*. The charts show the structure the model implies;
they are for finding the places where a hypothesis breaks, not for balance.
