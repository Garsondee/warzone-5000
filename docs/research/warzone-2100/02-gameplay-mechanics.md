# Gameplay Mechanics

Primary source for most of this document is the official **Quick Start Guide** shipped in the repository
(`doc/quickstartguide.asciidoc`), which is high reliability. Items marked *(general knowledge)* are not from a
fetched source and should be verified.

## 1. The core loop
1. Build a **Factory** and make extra **trucks**.
2. Trucks claim **oil resources** with **Oil Derricks** and build **Power Generators**.
3. Build a **Research Facility**, research weapons (you start with none), build a **Command Center** to unlock
   the minimap and the Design screen.
4. Design and mass-produce units; scout; destroy enemy bases.
5. Keep upgrading through research. Better tech, not just more units, wins.

## 2. Economy: oil and power
- **Oil Derricks** must be placed on oil resources (free oil shows as blue minimap pulses). A burning oil
  resource cannot be built on until the fire ends.
- **Power Generators** convert derrick output into power. **One generator supports up to 4 derricks.**
  Extra generators beyond that ratio add nothing; generators with no derrick do nothing. Generators can be
  placed anywhere.
- A **Power Module** (needs research) boosts a generator's output. Other research also raises income.
- Power is **spent at click time** when you start building/researching/manufacturing. A red power bar means you
  cannot afford the item.
- Derricks reportedly cost 100 power each (community hint, lower reliability).
- Demolishing a structure refunds half its cost. Recycling a unit refunds half its power and passes its
  *experience* to the next unit the factory builds.

> **Theory note:** a fixed, map-bound resource (oil points) rather than a gathered resource (like minerals) means
> expansion is about *territory control*, not worker micromanagement. The 4:1 generator ratio is a simple
> "infrastructure tax" that creates a small build-order decision.

## 3. Structures
| Structure | Role |
|---|---|
| Factory | Builds wheeled/half-track/tracked/hover land units and trucks. Modules upgrade it. |
| Cyborg Factory | Builds cyborgs. |
| VTOL Factory | The only place VTOLs and transports can be built. Needs modules for some units. |
| Research Facility | One research topic at a time per facility. Research Module speeds it up (facility pauses while the module is being built). |
| Command Center (HQ) | Unlocks minimap and Design screen. Prerequisite for defensive-structure research. Acts as a sensor tower. |
| Power Generator / Oil Derrick | Economy (see above). |
| Repair Facility | Units retreat here for repairs. |
| VTOL Rearming Pad | VTOLs reload ammo here. |
| Sensor tower / CB tower / VTOL strike tower | Targeting for artillery, counter-battery and VTOLs. |
| Defensive structures, walls, hardpoints | Cannot be ordered to fire. Hardcrete and tank traps only block movement. |
| Satellite Uplink | Counts as a Wide Spectrum sensor and reveals the map *(Quick Start only says sensor)*. |

Construction: trucks and Combat Engineers build. Extra builders clicking the same structure speed it up.

## 4. Research
- Only Research Facilities research. Each handles one topic at a time.
- **Campaign:** drive a unit onto an **artifact** (dropped by destroyed enemy structures) to unlock a concept; you then
  research it. Example: a heavy-cannon artifact leads to a heavy-cannon turret and a heavy-cannon emplacement.
- **Skirmish/multiplayer:** you progress up the tech tree directly (no artifacts needed).
- 400+ technologies. You start with **no weapons**; Machinegun is typically the first research.

## 5. The unit design system
Requires a Command Center. Three steps:
1. **Body**: armor, hit points, size/weight.
2. **Propulsion**: wheels, half-tracks, tracks, hover, legs (cyborgs), VTOL. Affects speed on road, off-road, water.
3. **Turret**: weapons or systems (sensor, repair, commander, etc.).

The Design screen shows cost, hit points, speed (road/off-road/water) and weight. **Weight affects both speed and
durability.** Sources quote "over 2,000" possible combinations.

A factory only builds designs that match its propulsion type and installed modules. Queue up to 9 units; loop
production repeats the queue (right-click at zero for infinite). Units spawn beside the factory and go to its
rally point.

## 6. Weapons and damage *(partly general knowledge; verify)*
Weapon families found in sources: machine guns, cannons, rockets and missiles, mortars, howitzers, ripple rockets,
Archangel missiles, flamers, lasers, bunker busters, mini-rocket pods, sensors.

- A reference table groups weapons by **damage profile**: all-rounder, anti-personnel, anti-tank, artillery,
  bunker buster, flamer. Each has percentage modifiers against armour types (kinetic vs. thermal) and target
  classes. Weapons are tagged **direct vs. indirect fire** and **anti-air vs. anti-ground**.
- Machine guns are the earliest weapon: anti-personnel, kinetic, can hit air and ground.
- Developer-forum balance assumption: **cannons for buildings, rockets for tanks** (cannons mostly short range).
- **Bunker Busters** are for killing structures; mortars plus bunker busters plus heavy cover is a classic base assault.

> **Theory note:** the "damage type vs. armour type" matrix is the same rock-paper-scissors idea as in most RTS
> games, but because *you* assemble the unit, counters are about component choices, not unit names.

## 7. Artillery and sensors (a signature feature)
- Artillery = mortars, howitzers, ripple rockets, Archangel missiles. MRL and Angel missiles can fire unspotted.
- Unspotted artillery range is short (8 tiles unupgraded). A mortar reaches **18 tiles when a sensor spots**.
- **Sensor tower** (range 16) auto-targets the nearest enemy; artillery structures in range fire on its targets.
- **Sensor turret** (range 12) attacks only when given a manual target. Artillery tanks must be assigned to a
  sensor (marked with an asterisk).
- **Counter-battery (CB)** towers/turrets detect enemy artillery firing at you and take priority over normal targets.

> **Theory note:** splitting "seeing" from "shooting" creates an information-warfare layer. Killing the sensor
> blinds the artillery. This is a rich design pattern that many RTS games skip.

## 8. VTOLs and anti-air
- Needs VTOL propulsion research, VTOL Factory research, a built VTOL Factory and several rearming pads.
- VTOL weapons do roughly **2x damage** of ground versions but have **limited ammo**; they auto-return to pads.
- They do not auto-attack: order patrols or attacks. VTOL bombs behave like artillery.
- Counters: dedicated AA (SAM sites etc.). Versatile weapons (machine guns, lasers, mini-rocket pods, rockets on
  cyborgs) work but do less damage.

## 9. Commanders and experience
- Needs Commander research. Attach units by selecting them and clicking a commander; detach with Ctrl+new order.
- A commander starts with **6** non-artillery attackers, **+2 per rank**. Ranks at 2, 4, 8, 16, 32, 64... kills.
  Indirect-fire units are unlimited.
- Commanders give attached units accuracy, armor and speed bonuses.
- **Experience:** kills raise rank, improving accuracy, speed, armor.

## 10. Orders and unit control
- **Attack range:** Optimum (default), Short, Long.
- **Retreat:** Do or Die (default), Medium Damage, Heavy Damage.
- **Firing:** Fire at Will (default), Return Fire, Do Not Fire.
- **Movement:** Patrol, Pursue, Guard (default), Hold Position.
- **Return:** Repair Facility, HQ/Landing Zone, Transport.
- Shift/Ctrl for waypoints and queued orders; Ctrl+number groups.

## 11. Transports
- **Campaign:** load up to 10 units, launch to away missions.
- **Multiplayer:** carries units across the map.
- **Cyborg Transport:** cyborgs only, needs a VTOL Factory with 2 modules, unarmed and vulnerable.

## 12. Interface and hotkeys (summary)
Panels: F1 Manufacture, F2 Research, F3 Build, F4 Design, F5 Intelligence, F6 Commanders.
Selection: Ctrl+U all units, Ctrl+V VTOLs, Ctrl+A attack units, Ctrl+S on-screen, Ctrl+D heavily damaged.
Camera: B to Command Center, Backspace north, Space tracks a unit, right-mouse-drag rotates.
Minimap colours: green units, flashing red/white under attack; pulses: green artifact, red enemy base/beacon, blue free oil.
Full list in the Quick Start Guide.

## 13. Strategy tips (from guides)
- Keep generators at 4:1 with derricks; add power modules early.
- Pair artillery structures with sensor towers; add CB towers vs. artillery-heavy enemies.
- Always keep some AA with the army.
- Use commanders to enlarge groups, but mind the direct-fire cap.
- Early: factory first (more trucks), grab oil, start research immediately.
