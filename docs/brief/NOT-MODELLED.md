# Not modelled (yet)

The **design-relevance rule**: model an effect only if a design lever the player controls can change an outcome that matters.
Everything else is listed here with the reason and the trigger to revisit, so leaving it out is a decision, not an oversight.
Add a row whenever a lane consciously skips something; remove it when the effect is modelled.

| Effect | Why it is out | Revisit when |
|---|---|---|
| Wind and weather on vehicles | No design lever changes how a vehicle responds to wind that matters at our scale | A scenario needs gusts for gun-laying error |
| Tyre temperature and wear | Grip varies mainly with surface and load; no lever depends on it | Long-endurance scenarios |
| Track-link dynamics (catenary sag, track throw, link-level wear) | Belt is modelled as contact samples plus a belt speed; links are visual | Throwing a track becomes a damage mode (M3) |
| Driveline torsional wobble, gear-lash clunk | Fast, small, no design lever | Wheel-hop or shunt complaints in validation |
| Crew (fatigue, bounce injury, comfort) | Ride roughness is reported as vertical acceleration; no human model | A crew-comfort speed limit becomes a design driver |
| Engine and transmission thermal limits | Brakes have heat; powertrain thermals add cooling-system design levers | M3, with fuel and cooling |
| Fuel slosh and fuel mass change | Small next to hull mass; range is computed from consumption | Long-range scenarios |
| Persistent ruts and multi-pass soil deformation | Needs a deformable-terrain store; single-pass sinkage first | After M3 |
| Water (fording, swimming) beyond a depth limit | Depth limit in the capability table only | A course with water features |
| Tyre pressure control and run-flat behaviour | Inflation is a fixed design parameter | A central-tyre-inflation lever is wanted |
| Aerodynamic lift and downforce | Negligible below 100 km/h | Never for this speed range |
| Vehicle-vehicle collision response detail | Rare; a single convex proxy per vehicle first | Close-quarters scenarios (M4) |
| Ammunition handling, reload ergonomics | Reload is a timer in combat | M3 |
| Terrain below 1 m features | Unresolved relief enters as micro-roughness per material | The heightfield resolution is revisited |
