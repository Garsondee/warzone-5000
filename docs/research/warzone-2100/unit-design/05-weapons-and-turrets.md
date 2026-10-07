# Weapons and Other Turrets

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`, multiplayer ruleset. Tables:
[`../data/weapons.md`](../data/weapons.md), [`../data/turrets.md`](../data/turrets.md),
[`../data/modifiers.md`](../data/modifiers.md). How damage is *computed* is in
[06-damage-and-counters.md](06-damage-and-counters.md).

## 1. The weapon universe

- `weapons.json` has **120** entries; **79 are designable** by the player. The rest belong to scavengers, structures
  (towers, fortresses, the laser satellite), campaign scripts and cyborgs.
- Of the 79 designable weapons, **27 are VTOL entries** (ids containing `VTOL`) and 52 are ground weapons.
  VTOL versions are *separate stat entries*, not a flag on the ground weapon, and each is tuned individually.
  Compared with its ground twin (12 pairs checked): damage x1.1 to x2.0 (Machinegun x1.5, Lancer x1.57, Light Cannon
  x2.0, Rail Gun x2.0, Bunker Buster x1.14), a much shorter fire pause for the single-shot guns (Light Cannon 30 to 4,
  Rail Gun 50 to 4), about 10-30% more range, sometimes a heavier weapon (Tank Killer 300 to 750, Needle Gun 1,000
  to 2,000), and a limited-ammunition salvo model (`numRounds` / `reloadTime`).
- Cyborgs have their own 17 weapon entries (10 "Cyborg", 7 "SuperCyborg" usage class) that normal tanks cannot
  mount, and they are used only in fixed templates.

## 2. The axes a weapon is described on

| Axis | Values in the data | Notes |
|---|---|---|
| **Sub-class** ("family") | MACHINE GUN, CANNON, ROCKET, MISSILE, MORTARS, HOWITZERS, FLAME, ENERGY (lasers/particle), GAUSS (rail), BOMB, A-A GUN, EMP, ELECTRONIC, plus COMMAND and LAS_SAT (non-designable) | 15 in total. Research upgrades are mostly keyed by sub-class (`ImpactClass`). |
| **Weapon effect** | ALL ROUNDER, ANTI PERSONNEL, ANTI TANK, ARTILLERY ROUND, BUNKER BUSTER, FLAMER | Only **6**. This is the key that selects a row in the damage-modifier tables (versus propulsion type and versus structure strength). |
| **Weapon class** | KINETIC (92 of 120), HEAT (28) | Selects which armour value (kinetic or thermal) resists it. |
| **Flight / movement** | DIRECT, HOMING-DIRECT, INDIRECT, HOMING-INDIRECT | Indirect weapons arc over obstacles and (for artillery) rely on a sensor to see the target. |
| **Flags** | ShootAir, AirOnly, NoFriendlyFire, TeleportCapture | `AirOnly` = anti-aircraft only. `TeleportCapture` is the unit-takeover mechanic. |
| **Numbers** | damage, firePause, reloadTime, numRounds, shortRange, longRange, minRange, shortHit, longHit, radius, radiusDamage, periodicalDamage (burn), weight, buildPower, buildPoints, hitpoints | See the full table. |

### Ground (non-VTOL) designable weapons: sub-class x effect

| Sub-class | All-rounder | Anti-personnel | Anti-tank | Artillery | Bunker buster | Flamer |
|---|---:|---:|---:|---:|---:|---:|
| Machine gun | | 5 | | | | |
| Cannon | 7 | | 2 (AA flak) | | | |
| Rocket | | | 4 | 3 | 1 | |
| Missile | | | 3 | 2 | | |
| Mortar | | | | 4 | | |
| Howitzer | | | | 4 | | |
| Flame | 1 (plasma launcher) | | | | | 3 |
| Energy (laser/particle) | 1 | 3 | 1 (AA) | | | |
| Gauss (rail) | 3 | | | | | |
| AA gun | | | 2 | | | |
| EMP | 1 | | | 1 | | |
| Electronic (Nexus Link) | | 1 | | | | |

Totals for the 52 ground weapons: kinetic 39, heat 13; flight: 23 direct, 14 homing-direct, 13 indirect,
2 homing-indirect; 26 weapons are indirect-fire or have a minimum range.

> **Observation:** the *effect* axis is shallow. Most sub-classes map to exactly one effect, so the effect tables
> mostly act as a second name for the sub-class. Real tactical diversity comes from range, rate of fire,
> salvo behaviour, area damage, and weight.

## 3. Weight and cost are very wide, and weight matters

| Quantity | Smallest | Largest | Spread |
|---|---|---|---:|
| Weapon weight | 200 (Machinegun) | 30,000 (Plasma Cannon, Heavy Plasma Launcher) | **150x** |
| Weapon power cost | 10 | 700 | 70x |
| Body weight (for comparison) | 450 (Bug, Retaliation) | 4,500 (Dragon) | 10x |
| Body power cost | 25 | 90 | 3.6x |

Weapon weight spans **150x**, while body weights span only 10x. Total vehicle weight is the divisor in the speed
formula ([04-engine-and-speed.md](04-engine-and-speed.md)). Because a propulsion's weight is a *percentage of the body's*
(tracks add +650%, hover +200%, VTOL +50%), the weapon is the dominant weight term only on light propulsion
(VTOL, hover) or when it is one of the very heavy weapons: a Ground Shaker (20,000) or Plasma Cannon (30,000) weighs
several bodies' worth. On a tracked Cobra (15,000 from body plus tracks) a 200-weight Machinegun is invisible,
while an 8,000 Heavy Cannon costs about a third of the speed. Weight is the only built-in "cost" of mounting a heavy
weapon on a small chassis.

Range bands (distance in tiles, 128 units per tile): the quartiles of long range for ground weapons are about
**9 / 12.5 / 18 tiles**. Artillery reaches much further (mortars 18 tiles; Archangel 120 tiles; Ripple Rockets 86
tiles), and artillery needs a sensor to find targets beyond its own sight.

## 4. Special-purpose weapons worth noting

| Weapon | What is unusual |
|---|---|
| **Nexus Link Turret** (`ELECTRONIC`, `TeleportCapture`) | Damage 2 but takes over enemy units (the "Captured Units Teleport" flag was added in 4.7). Researchable (12,000 pts, needs Sensor Upgrade 3 and Command Turret 2) and designable. Countered by a separate **Resistance** stat (see below). |
| EMP Cannon / EMP Mortar / VTOL EMP | Disable rather than destroy; `empRadius` in the data. |
| Plasma Cannon, Heavy Plasma Launcher | Heat weapons with enormous weight (30,000) and cost. |
| Bunker Buster | Only 2 entries (ground + VTOL) with the `BUNKER BUSTER` effect: 400% against bunkers, 300% against hard structures, 20-50% against units. |
| Flamers | `periodicalDamage` (burn over time) plus 25% effectiveness against VTOLs. Flame effect does 10% to hard structures and 300% to bunkers. |
| A-A guns / SAM | `AirOnly`; homing; "Anti-tank" effect internally (80% against lift propulsion). |

**Resistance** is a defensive stat on bodies and structures (150 on most structures, 300 on HQs and relays), raised by
the research *Nexus Resistance Circuits* (+90). It is the countermeasure to takeover weapons, i.e. a whole
secondary combat layer (electronic warfare) that exists beside kinetic and thermal damage.

## 5. Non-weapon turrets

A droid has exactly one "turret role": weapon(s) *or* sensor *or* repair *or* construct *or* command (brain) *or* ECM.
(Checked against the 280 predefined templates: apart from 3 special ones with no turret at all, each lists one of
`weapons`, `sensor`, `repair` or `construct`. The 7 commander templates list both a `brain` and a
`CommandTurret1` entry under `weapons`, because the command turret is itself implemented as a pseudo-weapon that
occupies the weapon slot. The only template with two weapons uses the Dragon body.)

| Kind | Designable entries | Notes |
|---|---|---|
| Sensors | Standard sensor turret (range 12 tiles), Wide Spectrum (17.7 tiles, 450 power, 1,200 build points), CB radar, VTOL CB, VTOL strike, radar detector | Ranges 1,024-2,260 units (8-17.7 tiles). Sensor type decides what it enables (standard, counter-battery, VTOL intercept, wide spectrum). |
| Repair | Repair Turret (15 pts), Heavy Repair Turret (30 pts) | Weight 800 / 2,000. |
| Construction | Truck (8 construct points) | Cyborg Combat Engineer is a separate non-designable entry (5 points). |
| Command | Command Turret (500 HP) | Brain with ranks, thresholds, and a unit-group cap (see the research and mechanics docs). |
| ECM | Jammer Turret (range 8 tiles, weight 10,000, 500 power) | Present in the stat files but **not designable and not unlocked** in multiplayer. |

Important: sensors, repair, construction and command are **mutually exclusive with weapons** on a standard body.
A "support" vehicle therefore loses all firepower; the only multi-turret body is the Dragon (2 slots).

## 6. Findings for our design work

1. Weapons are the *most developed* component class (79 designable, 15 sub-classes) and the other turret classes are
   thin (6 sensors, 2 repair, 1 truck, 1 command, no usable ECM). A broader "utility turret" class (sensors,
   jammers, repair, construction, shields, towing, transport, mines, EW) would add a lot of design space cheaply.
2. Only 6 weapon *effects* and 2 damage classes means counters are coarse. Richer damage types
   (more armour types, status effects) would multiply meaningful decisions.
3. The weapon weight scale (150x) shows the right instinct, i.e. that a heavy gun should cost mobility, but
   it is the **only** constraint on what can be mounted on what. There is no mount-size, power-draw,
   recoil-vs-chassis, ammo-storage or crew constraint.
4. Resistance/takeover is an example of an *orthogonal* defensive stat that already exists in the engine; it is
   worth studying how well it works before inventing similar layers.
