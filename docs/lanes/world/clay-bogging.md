# Which vehicles should bog in the published clay? (answer to ARCH, SLICE-2 sentence 1)

Reproduce with `python3 -I spikes/world/clay_bogging.py`. Soil: the clay row of `materials.ron` (n 0.5, kc 13.19 kN/m^(n+1), kphi 692.15 kN/m^(n+2), c 4.14 kPa, phi 13 deg), a **secondary** source (arXiv:2603.28965 Table 2; Wong's book unopened, VALIDATION to verify). Vehicles: the design-sheet estimates in `content/vehicles/game/*.ron`. Nothing here tunes a soil number.

## The theory in four lines
1. **Sinkage.** A plate of width `b` pressed into soil sinks `z` where the soil pushes back with `p = (kc/b + kphi) z^n` (Bekker). So `z = (p / (kc/b + kphi))^(1/n)`: with n = 0.5, sinkage goes with the *square* of pressure, so doubling the pressure quadruples the rut.
2. **Compaction resistance.** Pressing the rut down costs work per metre of travel: `Rc = b (kc/b + kphi) z^(n+1) / (n+1)`. It is the energy the vehicle leaves in the ground.
3. **Thrust.** The soil can push back horizontally at most `H = c A + W tan(phi)` (Coulomb: cohesion over the contact area plus friction under the weight).
4. **Bogging** is `Rc > H`; the margin `(H - Rc) / W` is the slope (rise over run) the vehicle could still climb in that soil. A pit wall of grade `g` costs `g` of the margin.

## Which Bekker for a tyre?
A tyre either stays round (a *rigid* wheel, Wong's closed form `z = (3W / (b (3-n)(kc/b+kphi) sqrt(D)))^(2/(2n+1))`) or flattens to a footprint at its inflation pressure (the elastic case). It stays round when its inflation pressure exceeds the pressure the soil can carry at the resulting sinkage, which it does here (180 to 400 kPa inflation against 105 to 157 kPa under the wheel), and it is also what CHASSIS' `soil_wheel.rs` computes. (Using the inflation pressure as the soil pressure instead would say every wheeled truck bogs on flat clay, 15 times over: wrong regime.)

## Result (phi 13 degrees)
| vehicle | pressure under the soil | sinkage | Rc / W | thrust H / W | margin / W |
|---|---|---|---|---|---|
| Scout (1150 kg, 4 x 0.20 m tyres) | 105 kPa | 2.7 cm | 0.156 | 0.270 | **0.114** |
| Mule (2300 kg, 4 x 0.30 m) | 115 kPa | 3.4 cm | 0.163 | 0.267 | **0.104** |
| Hauler (5600 kg, 4 x 0.35 m) | 157 kPa | 6.3 cm | 0.196 | 0.257 | **0.061** |
| Carrier (7800 kg, 2 tracks 0.38 x 2.9 m) | 34.7 kPa | 0.23 cm | 0.001 | 0.350 | **0.350** |

(The carrier's ground pressure is `W / (2 b L)`; a track is a long plate, so it barely sinks, and what little it sinks costs almost nothing.)

## What this says
1. **On level published clay nothing bogs.** All three trucks have a positive margin; only the carrier is comfortable. The prediction is *not* "the wheeled trucks bog": it is "the wheeled trucks have a thin margin and the carrier a fat one".
2. **The slice pit does the separating through its walls, and the order matches the simulation.** The pit is 0.4 m deep with 6 m walls: average wall grade 6.7%, steepest point 10% (a smoothstep peaks at 1.5 times the average). The Hauler's margin is 6.1%: it cannot climb the wall and bogs. The Mule's (10.4%) and the Scout's (11.4%) just clear the 10% peak: they cross. The carrier (35%) crosses with room to spare. This is what ARCH sees (carrier and Mule cross, only the Hauler bogs), and it points to the model being right, not to a tyre-model fault: the Mule crosses because the published clay lets it, by 0.4 of a percentage point of grade at the steepest point.
3. **It is a knife edge, and the soil band is wide.** The clay's friction angle band (10 to 16 degrees, from the secondary table) moves the Mule's margin between 5.0% and 16.0%, and the Hauler's between 0.7% and 11.7%. At 10 degrees the Mule would bog on the wall; at 16 degrees the Hauler would cross. VALIDATION verifying the primary row is what pins this down.

## Options (a decision for ARCH and the owner; I changed nothing)
- **A (default, no content change).** Keep the published figures. Reword the acceptance sentence to what the physics says: "the tracked carrier crosses with a margin of 0.35 W; wheeled trucks have 0.06 to 0.11 W, and the heaviest bogs on the pit wall". Cheapest, honest, already true in the simulation.
- **B (geometry only, never a soil number).** Make the pit's walls steeper or deeper so the Mule also bogs: the Mule bogs when the steepest wall grade exceeds about 10.4%, i.e. a depth over about 0.42 m with the 6 m walls. But the Scout (11.4%) then bogs at nearly the same depth, so the pit stops separating wheeled trucks from each other and starts being a *ramp test*, not a soil test. Not recommended on its own.
- **C (needs a cited row).** A second, softer published soil (Wong's book lists several rows beyond sandy loam and clay: heavy clay, muskeg, snow, among others) for a second pit. I have not opened the book and will not quote numbers from memory; this is for VALIDATION to supply. A softer soil lowers every truck's margin together and keeps the carrier's far larger one.
