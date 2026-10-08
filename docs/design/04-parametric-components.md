# Parametric Components

*Status: **complete catalogue** in `crates/w5k_forge/src/family/`: five hulls, seven kinds of running gear, three
weapons, a sensor, a repair rig and an engine, all mounting on one standard socket vocabulary, with an auto-fitter and
a design roller that mix them (see [07 Possibility space](07-possibility-space.md)). Renders:
[atlas of every hull with every gear](../assets/forge/atlas.png), [fun mixes](../assets/forge/showcase.png),
[sweep](../assets/forge/turret_gun_sweep.png), [coupling](../assets/forge/turret_gun_coupling.png),
[army lineup](../assets/forge/lineup_vanguard.png).*

## The idea
Instead of a tech tree full of fixed parts ("75 mm turret", "88 mm turret", "heavy 88 mm turret"...), each component is
a **family with sliders**. One "gun turret" card covers everything from a rotary machine gun to a battleship gun; the
player decides where on that range their design sits. This gives:
- **fewer tech items**: cards unlock families, materials and mechanisms rather than numbered variants;
- **far more choices**: the possibility space is continuous;
- **a real design problem**: sliders compete, so every design is a balance the player chose.

## How a family works
A family is a **generator**: slider values in, an ordinary part out (shapes, sockets, function). Everything downstream is
unchanged. The forge meshes the part, measures its mass, inertia and internal volume from voxels, and fires rays for its
armour table ([03-part-forge.md](03-part-forge.md)). So **stats stay honest**: the armour slider makes the plates
thicker and the turret squatter, and the armour rating rises *because the rays see thicker, steeper plates*. No
formula says "armour +10%".

```
sliders --(family generator)--> shape tree --(forge)--> mesh + mass + inertia + armour table --(family)--> performance
```

The family also turns the slider values plus the measured part into **performance figures** (penetration, reload,
traverse time...).

## Three kinds of sliders
| Kind | Behaviour | Examples |
|---|---|---|
| **Free** | A matter of taste or an even trade. The solver never moves it. | slope (protection per mm against interior room), style |
| **Budgeted** | Competes for the part's budget (mass). With *hold mass* on, moving one makes the unlocked others give way. | calibre, barrel length, armour, loader, ammunition |
| **Opposed** | Two quantities linked by physics, so raising one directly costs the other. Not a separate mechanism: it falls out of the generator. | calibre against reload time; barrel length against traverse speed |

### The coupling solver (`family::set_slider`)
When the player drags a budgeted slider with *hold mass* on:
1. The budget is the part's current mass, estimated quickly from exact piece volumes (no voxels).
2. Every **unlocked** budgeted slider shifts by the same amount of slider travel. Each moves in whichever direction
   compensates (lighter if the moved slider made the part heavier, heavier if it freed mass). The amount is found by
   bisection, since mass changes monotonically with the shift.
3. If the others run out of travel, the moved slider **stops where the budget runs out**.
4. Locked sliders never move. If everything else is locked, the part simply gets heavier.

Measured in the prototype: starting from a 4.4 t, 75 mm turret, asking for 120 mm with everything free gives 120 mm
with 5 mm armour and 7 rounds. With armour locked, the calibre stops at 115 mm, the barrel shrinks to 15 calibres
(a stubby howitzer) and penetration falls from 70 to 43 mm. Mass is held within 0.5%.

> **Theory: why equal slider travel?** It is the simplest rule a player can predict. Every free slider gives up
> the same *share of its range*. Proportional-to-cost rules make the cheapest slider collapse first, which feels
> arbitrary. We can revisit this once players use it; the solver is twenty lines.

## Physics inside the gun turret family
The relationships are real scaling laws, so the trade-offs are ones a player can reason about:

| Quantity | Law | Why | Check |
|---|---|---|---|
| Shell mass | m = 7 kg x (d / 75 mm)^3 | geometric similarity: double the bore, eight times the shell | 7.62 mm: 7 g; 406 mm: 1.1 t (both close to real) |
| Muzzle velocity | v = 1200 (1 - e^(-L/35)) m/s, L in calibres | gas does work along the barrel with diminishing returns | L45: 870 m/s |
| Gun mass | 550 kg x (E / 1 MJ)^0.9 | a gun must contain the energy it releases | 75 mm: 1.3 t; 406 mm: 140 t (Iowa: 121 t) |
| Penetration | de Marre: P ~ v^1.43 m^0.71 / d^1.07 | empirical, from naval armour trials | 7.62 mm: 11 mm; 75 mm: 100 mm; 406 mm: about 600 mm |
| Velocity at range | v e^(-x / λ), λ grows with m / d^2 | sectional density: heavy, thin shots keep their speed | a rifle round loses ~90% by 1 km; a 406 mm shell ~5% |
| Reload | action cycle + handling time x m^0.85 | humans handle a few kg/s, machines far more | 75 mm by hand: 4 s; 406 mm by hand: minutes |
| Recoil | J = 1.3 m v | momentum of shot and gases | sets a minimum hull mass (below) |
| Turret size | smallest shape whose interior holds crew, breech recoil space, ammunition and loader | found by bisection, using the geometry kernel to compute exact interior volume | more slope or armour gives less room, so the turret grows |
| Traverse | t180 = 2 sqrt(pi I / tau), tau ~ ring area | constant-torque rotation, accelerating half way and braking half way; I is measured from the geometry | 75 mm: 4 s; 406 mm: about a minute (Iowa: about 45 s) |

### Gates that fall out of the physics
- **Recoil sets the hull you need.** A hull must not be kicked back faster than about 0.5 m/s, so the minimum hull mass is
  J / 0.5. That works out to about 20 kg for a machine gun (a drone can carry one), about 16 t for a 75 mm gun, and
  about 2,600 t for a 406 mm gun. That last figure is why railway guns existed, and why a battleship gun needs a train or
  a titan.
- **Turret ring diameter** grows with what the turret holds. A hull's ring socket limits it, so a big gun needs a big hull.
- **Traverse time** grows with turret inertia. Small, fast units can outrun a heavy turret's tracking by circling it.

## Making size visible
- Crew fittings (cupolas, hatches) are **human-sized** whatever the turret's scale, so a battleship turret's tiny
  hatches tell you how big it is.
- Chamfers grow with plate thickness, and heavy armour also makes the body squatter, so heavy armour *looks* heavy.
- Small rotary guns get a remote station with a sensor head; crewed turrets get cupolas; big guns get muzzle brakes and fume
  extractors.

## Armour semantics (shared with all parts)
- **Hulls and turrets have vital interiors**: a shot that reaches the inside counts as a penetration. Legs, wheels and
  masts are hollow but not vital.
- **Materials can be non-protective** (`armour: false`): guns, engines and electronics. A shot hitting them damages
  that component, so it is not counted as armour. Without this, a ray down the length of a barrel would read as
  20 m of steel.

## Shape is a trade-off, not just a look
The owner asked whether shapes can carry real trade-offs. Several shape axes have them, from physics:

| Axis | One end | Other end | Why |
|---|---|---|---|
| **Profile** (low or tall) | small silhouette, hard to hit | room inside; gun can dip further | hit chance follows silhouette area; a gun pivots on its trunnions, so dipping the barrel swings the breech up into the roof (max depression = asin(headroom / lever)); a loader needs about 1.7 m to stand |
| **Slope** | upright: roomy, cheap | steep: more protection per mm, less room, so a bigger, heavier turret | a plate crossed at angle a is t / cos a thick; slanted walls eat interior volume |
| **Length : width** | short and wide: turns well, takes big turret rings | long and narrow: more turrets and track, turns badly | tracked vehicles turn by skidding; the torque needed grows with track length over track spacing, and skid steering fails beyond about 1.8:1 |
| **Height of a hull** | low: hard to see and hit | tall: room for engines and crew, sensors see further | silhouette and line of sight |
| **Sponsons** (overhang) | narrow, light | wide upper body: more room, wider target | volume against frontal area |
| **Nose and bow** | blunt: room in front | pointed: front plates angled against side shots too | slope seen from more directions |

Some axes stay **free** (taste): wheel count, nose style. The rule of thumb: if two settings are equally good, the slider is free;
if one end buys something the other pays for, physics makes it a trade.

In the turret family, **profile** is implemented: low turrets measure a smaller frontal area, load more slowly by hand
(not with an autoloader) and depress the gun less (about 8 degrees at the lowest against 20 at the tallest).

## Families so far
| Family | Main sliders | What the physics says | Sweep |
|---|---|---|---|
| `turret_gun` | calibre, barrel length, front armour, slope, profile, loader, ammunition | de Marre penetration, shell mass with the cube of the calibre, sized to fit crew, breech, ammunition and loader | [sweep](../assets/forge/turret_gun_sweep.png) |
| `turret_missile` | missile calibre, tubes, housing armour, seeker, warhead share, elevation | mass with d^3; rocket equation trades warhead for range; shaped charge about 6 calibres | [sweep](../assets/forge/turret_missile_sweep.png) |
| `turret_beam` | beam power, aperture, housing armour, dwell | capacitors set the mass; the aperture sets the spot, so range and burn-through | [sweep](../assets/forge/turret_beam_sweep.png) |
| `sensor` | mast height, aperture, radar share | the horizon grows with sqrt(height); the radar equation; a tall mast is heavy | [sweep](../assets/forge/sensor_sweep.png) |
| `repair` | reach, repair rate | a cantilever arm (heavy with reach); 25 kW per kg/s | [sweep](../assets/forge/repair_sweep.png) |
| `engine` | power, technology (diesel 0.22, turbine 0.75, fusion 2.5 kW/kg) | hidden in the hull, but its mass and bay are real | |
| `track` | width, skirt armour, road wheels | wider spreads the weight (ground pressure) | [sweep](../assets/forge/track_sweep.png) |
| `wheel` | diameter, width, tyre pressure, tread | load = pressure x contact patch; big wheels roll over obstacles | [sweep](../assets/forge/wheel_sweep.png) |
| `legs` | stance, leg strength, foot radius, splay, knee bend, material | Euler buckling against crushing, hip torque, Froude gait; square-cube law | [sweep](../assets/forge/legs_sweep.png) |
| `rail` | wheel diameter, axle load, axles, gauge | tiny rolling resistance, low adhesion; only goes where the rails go | [sweep](../assets/forge/rail_sweep.png) |
| `hover` | skirt height, thrust fans, cushion pressure | lift power grows as W^1.5 at a fixed footprint | [sweep](../assets/forge/hover_sweep.png) |
| `antigrav` | rated lift, field cost (kW/t), ride height | power linear in the weight; a cheaper field needs heavier coils | [sweep](../assets/forge/antigrav_sweep.png) |
| `rotor` | radius, blades, tip speed, altitude | actuator-disc hover power, blade-stall thrust limit | [sweep](../assets/forge/rotor_sweep.png) |
| `hull_lancer` | length, width, height, front and side armour, glacis, nose, stations | low sharp wedge, one turret | |
| `hull_bastion` | the same plus turrets (1-2), sponsons | tall box, room inside, two turrets | |
| `hull_dreadnought` | the same plus turrets (1-6), stations (2-10) | long deck, row of turrets, a keel for rails | |
| `hull_strider` | radius, height, armour, leg pairs, sensor head, dome | round carapace on a ring of hips | |
| `hull_skiff` | length, width, height, armour, sweep, cabin, stations | flat arrowhead of composite: the body of cushions, rotors and anti-gravity | |

Sweeps are `w5k sweeps`: each family on its host hull, the engine sized to the whole vehicle, red text where the
physics objects.

## Mounts: why any gear fits any hull
Hulls publish a **standard vocabulary of sockets**, and families declare which kinds they fit:

| Socket | Where | Used by |
|---|---|---|
| `gear_r/l` | one long mount on each side | tracks |
| `station_r1..n / station_l1..n` | evenly spaced along each side (kind Station, or Hip on a round body) | wheels, legs, anti-gravity pods, rotor arms, tracks (not on hips) |
| `keel_1..n` | centre line under the hull | rail bogies |
| `belly` | the whole underside | air cushions, anti-gravity plates |
| `turret`, `turret_k` | rings on the roof | gun, missile and beam turrets |
| `mast_1/2`, `hub` | the roof | sensors, repair rigs; the `hub` also takes a rotor |
| `engine` | inside | engines (bay size checked) |

Every socket carries **hints** (`ctx.*`): the room along the hull, the height above the ground, the largest turret ring,
the mast width, the engine bay. A family reads them next to its sliders, so a wheel sizes itself to the room between
stations and a tyre touches the ground whatever the hull. `Forge::instantiate` generates the parts, refuses a family on a
socket kind it does not fit, mirrors the left side, and **raises the hull** so legs, cushions, pods and rotors reach the
ground (or their altitude): the design's `lift_m`. Fit problems (turret ring too wide, engine too big for its bay,
sensor head wider than its mast) appear on the vehicle sheet. Nothing is a rule; a combination simply fails when the
numbers say so.

## Mixing and auto-fit
Because mounts are standard, the explorer (`w5k_forge::explore`) can combine anything. It rolls a design from a seed
(`Pcg32::derive`, so the same seed gives the same design), then **auto-fit** sizes the parts: each running gear gets its
share of the weight (`Family::fit_to_load`), the engine gets what the vehicle draws, lifts and wants for speed, and the loop
repeats because every change moves the mass. If the gear cannot carry the vehicle even at its limits, the fitter puts the
vehicle on a **diet** (thinner armour, step by step), because mass is what every kind of lift is short of. What is left
over is either valid or reports why not. This is the seed of an AI designer: a search proposes a skeleton; the fitter
makes it physically coherent. Used from the command line: `w5k fit --spec content/specs/x.ron --out content/vehicles/x.ron`.

## Decisions taken
- Every piece of running gear and equipment mounts on the standard sockets; physics, not rules, gates combinations
  ([D7](../planning/decisions.md)).
- *Hold mass* is on by default (owner, 2026-10-08).
- Slope stays a free slider, with profile as a separate shape slider; heavy armour also pulls the turret a little
  lower and its chamfers heavier, so armour reads visually.

## Open questions
- How many sliders per component before it stops being fun? The turret has seven.
- Should hull families also expose per-face armour (front, side, rear, roof) or keep two values?
