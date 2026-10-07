# The Design Pipeline: How a Unit Is Defined, Validated and Costed

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`. **Evidence:** engine source read by an analysis agent, with the key
formulas (weight, hit points, build points, speed) re-checked by me directly in `src/droid.cpp`; data tables from the
stat files. Paths below are relative to the Warzone 2100 checkout. Units: 1 tile = 128 world units; the JSON uses 0.1 s
for time values.

## 1. Anatomy of a design

In the game a design is called a **template**. It is a record of component ids:

| Slot | Count | Notes |
|---|---|---|
| Body | 1 | Provides HP, armour, **engine power**, weight, size class, number of weapon slots. |
| Propulsion | 1 | Wheels, half-tracks, tracks, hover, VTOL (and legged for cyborgs). |
| Turret | 1 to 3 weapons, **or** one system turret | A system turret is a sensor, ECM, repair unit, construction unit, or a command brain. A brain forces exactly one weapon: its own pseudo-weapon. |

The engine knows **8 component types** (body, propulsion, brain, sensor, ECM, repair, construct, weapon) and a droid has
one slot of each type except weapons (up to `MAX_WEAPONS` = 3, `src/basedef.h:76`). Every designable body in the
multiplayer data has **1 weapon slot** except the Dragon with **2**. The design UI has three weapon buttons.

## 2. Validity: one gate

Every route that creates a template (the design screen, stored designs, network production orders, scripts) passes through
one validity function (`intValidTemplate`, `src/design.cpp:2795-2947`). It rejects a design when:

- the body or propulsion is missing (index 0 is a mandatory "null" placeholder in every component list);
- there is no turret at all (transporters are exempt);
- a weapon slot is empty, or a weapon's size class mismatches the body (this rule is **dormant**: no shipped data sets a
  weapon size);
- **VTOL rules:** "VTOL" means propulsion type Lift. A VTOL weapon must have attack runs; a non-VTOL design must not use
  one; a VTOL with no weapon is rejected (so VTOL tanks cannot carry sensors/repair through this path);
- more weapons than the body's slots;
- weapons mixed with system turrets;
- a brain with anything but its own turret.

### What is NOT checked

- **Body-propulsion compatibility.** *Any body takes any propulsion.* A 450-weight Bug body can be given tracks; a Dragon
  can be given wheels.
- **Mixed system turrets** (sensor plus construct). The UI prevents it by clearing slots; the engine otherwise resolves the
  droid type by a fixed priority (brain > sensor > ECM > construct > repair > weapon).
- **Weapon-versus-chassis fit** beyond VTOL: any ground weapon goes on any ground body, however heavy (see weight, below).

> **Theory note: permissive vs. constrained design spaces.** A design system can limit choices with *hard slots and
> mounts* (this weapon only fits that hull), with *budgets* (weight, power, volume), or with nothing but cost. Warzone 2100
> uses almost only the third: nearly everything fits everywhere and the penalty is price and speed. That keeps the UI
> simple and the combinatorics huge, but it also means few combinations are *forbidden*, and "good" combinations are
> found by arithmetic rather than by rules. A richer system usually adds at least one hard constraint.

## 3. Which parts the player may use

- A part must be flagged `designable` in the stats **and** *available* to that player. Unavailable parts never show.
  Cyborg parts are deliberately non-designable, which makes cyborgs **prefabricated-only** (see [02-bodies.md](02-bodies.md)).
- Per player and per component there is a state: AVAILABLE, UNAVAILABLE, FOUND, REDUNDANT, plus two combined redundant
  states. Parts become available through research (`resultComponents`), through scripts (`makeComponentAvailable`), or in
  skirmish by destroying an enemy factory (which can grant its best propulsion, body or weapon).
- **REDUNDANT** (set by newer research via `redComponents`) means "obsolete": hidden from the design lists unless an "obsolete"
  toggle is on, but it can still be used and built; nothing is removed from existing units.
- `replacedComponents` is a stronger mechanism: it swaps the old part in all existing droids, templates, queued
  production and structure weapons. The shipped data uses it for Auto-Repair.
- **"Naval" propulsion is designable in the multiplayer data but no research unlocks it**, so it is unreachable.

### Factories

A factory accepts a template only if: cyborg designs go to cyborg factories only; Lift (VTOL) designs to VTOL factories
only; in multiplayer an HQ must exist (except trucks); and the body size does not exceed the factory's modules
(0 / 1 / 2 modules allow Light / Medium / Heavy). This size check is enforced by the UI list and by scripts; the
simulation-level production request does not re-check it.

## 4. Derived stats: the formulas

All arithmetic is integer and truncating. Re-verified in `src/droid.cpp:1742-1850`.

| Stat | Formula (in words) |
|---|---|
| **Weight** | body weight x (100 + propulsion weight) / 100, plus the weights of all turret parts. **Propulsion "weight" is a percentage of the body's weight.** |
| **Hit points** | body HP x (100 + propulsion "hitpointPctOfBody") / 100, plus propulsion's own HP, plus the HP of each turret/brain; then scaled by (100 + sum of all HP-percentage upgrades) / 100. |
| **Build points** (production time) | body build points x (100 + propulsion build points) / 100, plus the others. Propulsion is a percentage of the body again. |
| **Build power** (price) | same pattern with `buildPower`. |
| **Armour (kinetic, thermal)** | Body only (plus research). Neither propulsion nor turret contributes. |
| **Weapon range** | The weapon only (plus research). |
| **Sensor range** | An ECM's range if fitted, otherwise the sensor's. |
| **Speed** | See [04-engine-and-speed.md](04-engine-and-speed.md). |

> **Key design fact:** for propulsion, `weight`, `buildPoints`, `buildPower` and `hitpointPctOfBody` are **percentages of the
> body's values**, while the same keys are *absolute* for every other component. A "tracks" option therefore scales with
> the hull it is bolted to: tracks add +650% of the hull's weight and 300% of its HP. This makes propulsion *proportional*
> and weapons *fixed*, which is a different economics from either "everything is additive" or "everything multiplies".

### Worked example (multiplayer data, no research)

Cobra (body: 130 HP, 2,000 weight, 46 power, 250 build pts) + Tracks (+650% weight, 300% HP, +125% cost, +125% build pts) +
Medium Cannon (5,000 weight, 350 HP, 150 power, 500 build pts):

| Quantity | Calculation | Result |
|---|---|---:|
| Weight | 2,000 x 750 / 100 + 5,000 | **20,000** |
| Hit points | 130 x 400 / 100 + 350 | **870** |
| Build points | 250 x 225 / 100 + 500 | **1,062** |
| Power cost | 46 x 225 / 100 + 150 | **253** |
| Armour | body only | **15 kinetic / 6 thermal** |

Factories produce **10 build points per second plus 10 per module**, so this tank takes about 106 s in a bare factory and
about 35 s in a factory with two modules (before production research; the factory-upgrade research chain adds +30%, +90%,
+90%, +60% in four steps).

## 5. The design screen

**Shown:** body armour (both), "Engine Output" (the body's upgraded power), the body's own weight; for propulsion, the
road speed, off-road speed and water speed *of the whole current design with that propulsion* (VTOLs show one air speed);
for weapons, range, damage, rate of fire, weight. Totals: **"Total Power Required"** and **"Total Body Points"** as bars.
Hovering a candidate shows a shadow bar with plus/minus deltas.

**Not shown:** total weight, build points or time, and the final speed as a single number beside the totals (the speeds
are on the propulsion panel).

**Constraints are hidden, not explained.** There is no warning text. Weapons are filtered by VTOL status; switching between
ground and VTOL propulsion silently clears the turret; the Systems button is hidden for VTOLs; invalid designs simply do not
save; a flashing button marks a missing part.

## 6. Closed sub-systems: cyborgs, transporters, commanders

- **Cyborgs** use two special bodies (light and "super"), legged propulsion and cyborg-only weapons, and exist only as
  **28 prefab templates**. They are built in a cyborg factory (cyborg designs can *only* be built there).
- **Transporters** use a non-designable body plus VTOL propulsion; campaign players cannot build them.
- **Commanders** are a normal tank body with a command turret (a pseudo-weapon plus a brain).

These three categories show where the design system is *bypassed*: when the developers wanted a unit category with its own
rules, they made a closed, non-designable set rather than extending the general system.

## 7. Stored designs and scripts

Designs are stored per player (separately for skirmish and campaign) and re-validated on load. Scripts can enumerate
(`enumTemplates`), create (`makeTemplate`, returning null if the player lacks the research), and order production
(`buildDroid`). The design screen also raises events (`eventDesignCreated`, etc.). So the AI and the human use the same
templates.

## 8. Findings for our design work

1. **Proportional propulsion is a neat idea** worth considering (a propulsion type is a *multiplier profile* applied to a
   hull). It automatically scales cost and HP with size and avoids a separate light/medium/heavy propulsion list.
2. **There is no compatibility graph.** Anything fits anything, so the number of designs is just a product, and the
   design screen cannot teach the player anything about *why* a design is poor (no total weight, no warnings).
3. **Derived stats are cached at production time.** Swapping parts via research (`replacedComponents`) does not recompute
   them, which is a known looseness; a future design should recompute from one source of truth.
4. **Cyborgs show the cost of a closed side-system**: a second, parallel set of bodies, propulsion, weapons, factory
   and templates. An extensible engine should treat "infantry" as just another body/propulsion class.
