# The design model: what you choose, and what follows

*Owner direction, 2026-10-10 (card C-020), written by ARCH. It applies to every lane that touches a design number. Where it conflicts with a lane brief, this wins and the conflict is a bug to report.*

## The rule
A vehicle design is a short list of **choices**. Everything else about the vehicle is a **consequence**: computed from the choices by physics, shown to the player, never set by hand.

| Kind | What it is | Examples | Shown as |
|---|---|---|---|
| **Choice** | a part picked from a catalogue, or a size within that part's valid band | engine type and power, armour thickness per facing, gun calibre and barrel length, tyre width and pressure, track width, wheelbase, spring stiffness, fuel tank size | a slider or a picker |
| **Consequence (by design)** | follows from the choices by arithmetic or statics, instantly | total mass and where it sits, centre-of-mass height, axle loads, ground pressure, power-to-weight, ride frequency, frontal area, ammunition count, what can and cannot be armoured | a read-only number with its units and a one-line "because" |
| **Consequence (measured)** | only appears when the vehicle is driven or shot at | top speed, braking distance, steepest grade, skidpad grip, bogging in clay, survival in a fight | the proving-ground scoreboard and, later, battle results |

**Mass is a consequence.** `hull.mass_kg` in the vehicle file today is a hand-authored all-in number for the three trucks; it is a stage-1 shortcut (the coherence tests in FORGE's brief, "node masses sum to the sheet mass", were always meant to make mass, inertia and centre of mass come from the parts and their geometry). It has to become the sum of the parts. The same holds for the centre-of-mass position, the inertia, the springs' working loads and the ground pressure.

## Why, in four principles (the theory, for the owner)
1. **Independent and dependent variables.** A design space has a small number of independent knobs; everything else is dependent. Graphics analogy: a rig gives the animator controls, and the vertex positions are the *result*; a modeller sets dimensions, not "polygon count". A mass slider makes mass independent, so you can fit a huge engine to a light hull and "tune" the weight away, which breaks the causal chain that makes a design choice matter.
2. **No free lunch.** Every slider must charge a price on some other axis, otherwise the best setting is always the end of the slider and the game collapses to "maximum everything". Engineers call the set of best compromises the *Pareto front*; our job is to make sure a real front exists. The price list below is that front.
3. **Scaling laws and coupling.** Make a vehicle twice as long and its surfaces grow by 4, its volumes and so its mass by 8 (the square-cube law: scale a mesh by 2 and the area quadruples but the volume goes up eightfold). Armour area grows like the square of the size, the mass it must protect like the cube. Couplings do the rest: a bigger gun gives a bigger recoil impulse (momentum is conserved, `v_hull = p / M`), a heavier turret, a higher centre of mass, more load on the front axle, more ground pressure, and a vehicle that sinks where a lighter one drove on. Wacky designs fail *by themselves* because the chain of consequences is honest.
4. **Fail honestly.** The compile **rejects only what the solver cannot represent** (a numerically unstable design, a negative volume, a missing part). A design that merely performs badly is **built, simulated and flagged**, with the physical cause. A vehicle with a huge gun on a small hull is allowed to exist; it stalls on the first slope, tips when it fires, or sinks in the mud, and the scoreboard says why. (FORGE already moved this way: a brake torque above the tyre's friction is now a warning, not a rejection.)

## The price list (what each choice costs)
| Choice | What it buys | What it costs (each is a derived number the player can see) |
|---|---|---|
| Engine power | acceleration, top speed, grade | engine **mass** (specific mass of the engine type) and **volume**, **fuel burn** (range falls), **heat to reject** (cooling air and the openings that carry it, see Vulnerability), stress on the driveline above its rating (reliability), signature |
| Engine rating above 100% (boost, overrating) | still more power from the same engine | more heat (bigger or extra radiators, charge-air cooling), shorter life, and **unarmourable weak points** (see below) |
| Armour thickness, per facing | protection | **mass** = thickness x area x density (steel and aluminium differ), so power-to-weight, ground pressure, axle loads and cost; raised centre of mass for the turret and the top |
| Gun calibre and barrel length | penetration, range | gun mass, mount and **turret ring** size, **recoil impulse**, shell size so **fewer rounds** for the same stowage, barrel **overhang** (gets stuck on slopes, hits the ground), a bigger weak point at the mantlet |
| Tyre width | grip, lower ground pressure | tyre and wheel mass, rolling resistance, steering effort, overall width |
| Tyre pressure | on soft ground: floatation; on roads: efficiency | the opposite on the other surface (hard road: soft tyres waste energy and wander; soft ground: hard tyres dig in) |
| Track width | lower ground pressure, floatation | track mass, steering resistance (the skid-steer moment grows), transport width |
| Wheelbase / hull length | stability, ride, room | mass, turning circle, breakover angle (grounding on crests) |
| Ground clearance | obstacle clearance | higher centre of mass (rollover), bigger silhouette |
| Spring stiffness and travel | ride comfort or body control | body roll, bottoming on big hits, space taken |
| Fuel tank size | range | mass, fire risk, space |
| Final drive and gear ratios | pull at low speed, or speed | engine rpm at cruise, shift behaviour, top speed |
| Brake size | stopping power | mass, heat soak, fade |

A slider that has no price in this table is a bug in the design, not a feature: report it.

## What "doesn't work" looks like (envelope flags)
Each flag is a number, a threshold with provenance (`ESTIMATE` until sourced), and a "because" line. The checks are statics and short formulas, computed the instant a slider moves; the proving battery confirms them by driving.

| Question | Check | The flag a player sees |
|---|---|---|
| Can it move? | power-to-weight against a minimum for the soil; ground pressure against the soil's bearing strength (Bekker) | "cannot climb 30%", "sinks to the hull in clay" |
| Can it carry itself? | axle and wheel load against the tyre rating; static suspension margin (travel left at rest) | "front tyres overloaded", "bottoms out at rest" |
| Does it stay upright? | static stability factor `(track / 2) / h_com` against plausible lateral g; recoil impulse against the weight and the traction | "tips at 0.4 g", "slides back 1.2 m when it fires" |
| Does it cool? | heat to reject against the radiator's capacity at sustained load | "overheats after 90 s at full power" |
| Does the driveline survive? | engine torque against the gearbox and final-drive rating | "gearbox torque 140% of rating: short life" |
| Can it stop? | kinetic energy at top speed against the brakes' thermal mass | "brake fade on a long descent" |
| Can it cross obstacles? | approach, departure and breakover angles; gun overhang | "gun digs in on a 20% crest" |

## Vulnerability: weak points that cannot be armoured
Owner idea: an engine can be pushed over 100%, and the price is weak points the player cannot cover with armour. The model that makes this *emergent* rather than a rule:
- Every **component** (engine, transmission, fuel tank, ammunition stowage, crew compartment, turret ring, tracks and suspension) has a position, a volume, a **criticality** (what losing it does: engine = mobility kill, ammunition = catastrophic) and a **protection** (the armour along the sight lines in front of it, from the armour geometry).
- Some things physically **need openings**: cooling air in and out and the exhaust, crew vision and hatches, the gap round the gun mantlet and the turret ring. Their **area is derived**: the cooling aperture area grows with the heat the engine rejects (roughly a third of the fuel energy leaves through the radiator at full power, so it scales with power), and an engine run above its rating adds an extra aperture (a bigger radiator or a charge-air cooler) and a durability penalty. These apertures are **unarmourable by construction**.
- A **hit map** shows each facing coloured by effective protection, with the weak points marked. COMBAT (milestone M3) uses the same data: the chance a shot reaches a component is its **exposed cross-section**, damage is a component state, and a damaged component changes mobility or firepower. "More engine, easier to kill" then falls out of the geometry; it is not a hand-written penalty.
- A larger engine is also a **bigger target** (its volume is larger), which the exposure calculation sees for free.

## What the Workshop shows
Four panels, in this order of importance for the player:
1. **Design**: the sliders and part pickers (choices only). No mass, no centre-of-mass height, no ground pressure, no top speed.
2. **Consequences**, instant: a **mass budget** (stacked bar: hull structure, armour, engine, drivetrain, running gear, weapon, ammunition, fuel, crew), balance (centre of mass in three axes, front and rear axle share, inertia), ground pressure per soil, power-to-weight, ride frequency, silhouette and frontal area, obstacle angles, the **hit map**, build cost.
3. **Measured**: the proving-ground scoreboard (about half a second to compute), with the limiting factor named ("grip", "wheel lift", "power", "traction").
4. **Flags**: every envelope check that is amber or red, each with the cause and the slider that most affects it.
Every number shows its uncertainty band where its inputs are estimates (the dossier plan's Monte Carlo envelope), so a prototype reads "42 km/h, give or take 6" instead of false precision.

### Information, not sliders: the catalogue
- **Mass and balance**: total mass (curb and combat loading), mass by system, centre of mass (x, y, z), inertia (roll, pitch, yaw), sprung and unsprung mass, axle and track load shares, static pitch and ride height.
- **Ground contact**: nominal and peak ground pressure, sinkage in each soil, footprint, wheel loads against tyre rating, approach, departure and breakover angles, fording depth, turning radius.
- **Powertrain**: power-to-weight, torque-to-weight, top speed and what limits it (power, gearing, tyre rating), 0 to 48 km/h, steepest grade and its limiter, fuel burn at cruise and in combat, range, cooling margin, transmission torque margin, engine rpm at cruise.
- **Handling and ride**: ride frequency and damping ratio (derived from the spring and the sprung mass), body roll at 0.3 g, understeer gradient, skidpad limit and its limiter, rollover threshold and side-slope angle, braking distance, roughness at speed, step and trench crossing.
- **Combat**: silhouette, effective armour by facing (with slope), vulnerable components and their exposed area, the unarmourable aperture share, crew survivability, ammunition count and the stowage's vulnerability, recoil impulse and the hull's recoil velocity, gun arcs and overhang, turret ring loading.
- **Signature and logistics**: heat and noise signature (engine power and cooling), build cost and build time (the rapid-prototyping programme of the long-term fiction), fuel and ammunition use per hour, reliability (stress against rating), maintenance load, transport class (width, bridge class from axle loads and mass), crew count and workload.
- **Confidence**: the band on each number, and which inputs are estimates.

## Staging (what changes, in order)
- **D1, now (Workshop honesty, VIEWER):** the Workshop drops the Mass slider (and any other consequence); the slot becomes **tyre pressure** (a real trade: soft ground against roads). Mass, power-to-weight, axle loads and ground pressure appear as read-only numbers as soon as FORGE exports them.
- **D2 (FORGE, small PRs):** a **component mass budget**: the engine, transmission and fuel (then crew) carry mass models tied to their sliders, so `engine power` moves the total mass, the centre of mass and the axle loads. The three trucks keep their present total mass and centre of mass at their design point (nothing else in the simulation changes); the authored `hull.mass_kg` becomes "structure plus the budget". FORGE also exports a **design audit** (derived numbers with units and provenance, plus the envelope flags with their causes) as JSON for the Workshop, the Control Room and later the AI's capability estimates. The budget lives in the extras sidecar until contract 0.4 carries it (a CCR that also graduates FORGE's other provisional extras).
- **D3 (FORGE, GEOMETRY, COMBAT):** armour and weapons join the budget (armour mass from the loft's facing areas, the gun by scaling law from calibre and length), with the recoil, stability and overhang flags.
- **D4 (COMBAT, M3):** components, exposure, apertures and the hit map; damage state feeds mobility.
- **Probes stay probes.** The Impact Matrix keeps its `mass`, `com_height` and `tyre_friction` levers: they perturb a number on purpose to test the physics (a sensitivity probe). They are test tools, not design levers, and are never shown to a player as sliders.

## Not decided here (defaults are in card C-021)
Per-facing armour sliders against one armour slider; how steeply an overrated engine adds weak points and loses life; whether parts are gated by an era or technology level; slider ranges (default: wide, the model does the punishing). All numbers in the price list are `ESTIMATE` with bands until the dossier lane cites sources (card C-017).
