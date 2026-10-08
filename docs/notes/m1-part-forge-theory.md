# Theory Note: the ideas behind the Part Forge (M1)

A short tour of the principles the forge is built on, in the order the data flows. Each one is a small idea with a
large effect.

## 1. Shapes as half-spaces
A convex shape can be stored as a list of planes, each saying "inside is on this side": n · x <= d. Then:
- **Is a point inside?** Check it against every plane: a handful of dot products. Voxelisation asks this millions of
  times.
- **Moving a shape** moves its planes. From computer graphics you will know the catch: *normals transform by the
  inverse transpose* of the matrix, not the matrix itself, or they stop being perpendicular under non-uniform scale or
  mirroring. The same rule applies here (`Plane::transformed`).
- **A chamfer** is just one extra plane laid across an edge.
- **The visible faces** fall out by intersecting planes three at a time and keeping the points inside all of them.

Why convex? Because it makes every one of those operations exact and simple. Concave shapes are built as unions of
convex pieces ("kitbashing"), which suits a machined, faceted look.

## 2. Sloped armour emerges; it is not a rule
A shot crossing a plate of thickness t that is tilted by angle a from the shot's path travels t / cos a through steel.
At 60 degrees that is twice the plate's thickness. The forge never applies this as a formula: it fires rays and measures how
much steel each one crosses. Sloping, overlapping plates, spaced armour (tracks in front of the hull) and weak spots
(a thin cupola on a squat turret) all come out of the same measurement. The test
`armour_follows_thickness_over_cosine` checks the result against t / cos a.

## 3. Sampling, aliasing, and keeping the total right
Voxelising is **point sampling**: each cell asks "is my centre inside?". As in rendering, point sampling aliases. A
65 mm barrel in 70 mm cells can come out twice as heavy, or vanish, depending on where the grid falls. We know each
piece's *exact* volume (the divergence theorem gives a polyhedron's volume from its faces), so each piece's cells are
rescaled until they add up to the exact amount. The grid keeps the job it is good at, deciding where overlapping pieces
share space, while the totals stay exact. It is the same idea as an energy-conserving filter: the shape of the
distribution may be approximate, but the integral is right.

## 4. Mass properties compose
Mass adds. The centre of mass is the mass-weighted average of the parts' centres. Inertia (how hard something is to
spin) adds too, once each part's inertia is rotated into the common frame (R I R^T) and moved to the common centre with
the **parallel-axis theorem**: I = I_centre + m (|r|^2 1 - r r^T). That is why a vehicle's mass is exactly the sum of its
parts' masses, and why a turret's traverse time can come from its *measured* inertia.

## 5. Why top speed is the root of a cubic
Driving at speed v costs power against two forces: rolling resistance (C_rr m g, so power grows with v) and air drag
(1/2 rho C_d A v^2, so power grows with v^3). Top speed is where the engine's power equals the sum:
P = C_rr m g v + 1/2 rho C_d A v^3. Heavy, slow vehicles are limited by the first term, light and fast ones by
the second. There are no hard caps except what the running gear allows (`max_kmh`).

## 6. Hovering costs power even standing still
A rotor holds a weight W up by pushing air down. Momentum theory (the "actuator disc") gives the ideal power:
P = W^1.5 / sqrt(2 rho A), where A is the total rotor disc area. Double the weight and the power rises 2.8 times;
double the disc area and it falls by a factor of 1.4. Small drones hover on a few hundred watts; a flying battleship
needs enormous rotors, or anti-gravity research.

## 7. Scaling laws (the square-cube law)
Make something s times bigger and its area grows by s^2, its volume and mass by s^3. That one fact drives a lot of the
design:
- shell mass grows with calibre cubed (a 406 mm shell is 158 times a 75 mm one);
- ground pressure grows with size, so giants sink ([05-game-loop.md](../design/05-game-loop.md));
- leg strength grows with cross-section (s^2) but must carry weight (s^3), so giant walkers need better materials.
Good game physics leans on such laws because players can learn them and then reason with them.
