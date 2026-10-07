# Damage, Armour and Counters

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`, multiplayer ruleset unless stated. **Evidence:** the combat code was read by
an analysis agent (paths below are relative to the Warzone 2100 checkout) and the central formulas were re-read by me
(`src/projectile.cpp:1694-1717`, `src/combat.cpp:518-525`); the tables are recomputed by
`tools/research/wz_combat.py`. Tables: [`modifiers`](../data/modifiers.md), [`weapons-dps`](../data/weapons-dps.md),
[`damage-vs-targets`](../data/damage-vs-targets.md), [`upgrade-race`](../data/upgrade-race.md).
Weapon families and stats: [05-weapons-and-turrets.md](05-weapons-and-turrets.md).

## 1. Summary

1. Damage is **base damage x (effect vs target propulsion) - flat armour**, with a **33% floor**. The two modifiers (propulsion,
   body size) are **added**, not multiplied.
2. Armour is **one number per damage class on the body** (kinetic, thermal). It is a *flat subtraction*, so it punishes small hits
   hard: a Machinegun does **1 damage per hit** to a Cobra on tracks.
3. **Propulsion is the armour class.** The 6 weapon effects x 7 propulsion types matrix is the main source of counter-play.
4. Research makes **flat armour grow faster than small weapons' damage**, so low-damage weapons stay pinned to the floor
   at every tech level.
5. Experience is a strong, simple multiplier: at rank 8 a unit takes **0.52x** damage and gets +/-40% accuracy.
6. Visibility is a **gate, not a modifier** (you cannot shoot what you cannot see), and indirect fire depends on allied sensors.

## 2. The pipeline of one direct hit

1. **Hit roll** succeeds (section 5). The projectile flies and the first non-allied object on its path takes the impact
   (not necessarily the intended target).
2. **Base damage** = the weapon's per-player upgraded `damage`. Research adds `ceil(base x value / 100)` per step (the
   default "compat" rounding), so a +25% line on a 10-damage weapon goes 10, 13, 16, 19, 22.
3. **Effect modifiers** (`calcDamage`). Droid target:
   `base x (100 + (propulsion modifier - 100) + (size modifier - 100)) / 100`, at least 1. The *size* modifier is read from the
   same file but **no shipped data defines it**, so it is always 100. Structure target: `base x strength modifier / 100`.
4. **x3 damage** if the target is a landed VTOL or a landed multiplayer transporter.
5. **Rank reduction**: x `(100 - 6 x level) / 100`.
6. **Armour**: `max(damage - armour, damage x minimumDamage / 100)`; armour is the body's kinetic value for KINETIC weapons and
   thermal value for HEAT weapons. Almost every multiplayer weapon has `minimumDamage` 33.
7. **At least 1.** Hit points are subtracted (one pool per droid, however many parts it has).

> **Theory note: additive vs. multiplicative modifiers.** Adding the propulsion and size modifiers (instead of multiplying
> them) keeps the result bounded and predictable: two "70%" modifiers give 40%, not 49%. It also has an edge case: if the two
> deficits sum past 100% the result would go negative (the engine's unsigned arithmetic would wrap); it is latent only because
> no shipped data uses the body-size axis.

## 3. The counter matrices

### 3.1 Weapon effect against the target's propulsion (percent of base damage)

| Weapon effect | Half-tracks | Hover | Legged | Lift (air) | Propellor | Tracked | Wheeled |
|---|---:|---:|---:|---:|---:|---:|---:|
| ALL ROUNDER | 115 | 120 | 65 | 40 | 105 | 105 | 125 |
| ANTI PERSONNEL | 50 | 110 | **150** | 60 | 50 | **40** | 100 |
| ANTI TANK | 125 | 90 | **30** | 80 | 120 | 120 | 130 |
| ARTILLERY ROUND | 65 | 110 | 130 | **25** | 65 | **30** | 90 |
| BUNKER BUSTER | 40 | 20 | 30 | 30 | 50 | 50 | 30 |
| FLAMER | 100 | 130 | 130 | **25** | 90 | 90 | 110 |

### 3.2 Weapon effect against structure strength

| Weapon effect | SOFT | MEDIUM | HARD | BUNKER |
|---|---:|---:|---:|---:|
| ALL ROUNDER | 130 | 100 | 100 | 75 |
| ANTI PERSONNEL | 160 | 65 | 40 | 50 |
| ANTI TANK | 75 | 50 | 25 | 60 |
| ARTILLERY ROUND | 200 | 120 | 100 | 20 |
| BUNKER BUSTER | 100 | 120 | **300** | **400** |
| FLAMER | 150 | 60 | **10** | **300** |

What the matrices say (the intended counter-play):
- **Anti-personnel** (machine guns) shred cyborgs (150%) and are nearly useless against tracks (40%).
- **Anti-tank** (rockets, missiles) is great against wheels, half-tracks and tracks (120-130%) and nearly useless against
  cyborgs (30%).
- **Artillery** is strong against cyborgs (130%) and soft structures (200%), and weak against tracks (30%) and bunkers (20%).
- **Bunker busters** do 300-400% against hard structures and bunkers but only 20-50% against units.
- **Air** takes only 25-80% from most ground effects, which is why dedicated anti-air exists (VTOLs also take x3 when landed).
- **Hover** is a soft target for artillery (110%) and anti-personnel (110%) but resists anti-tank (90%).

The same table is read by the AI's target scoring, so weapon-versus-propulsion matchups also bias *whom units shoot at*.

## 4. Armour and the 33% floor

Armour is **one kinetic and one thermal value, taken from the body only** (propulsion and turret add nothing, other than hit
points). It is **subtracted** from each hit. The floor means a hit always does at least 33% of the modifier-adjusted
damage (and at least 1).

Worked examples (multiplayer data, rank 0, no research). A Cobra on tracks has 130 x 400/100 + 75 (Machinegun HP) = 595 HP
and kinetic armour 15; a light cyborg has 300 HP and armour 12:

| Weapon (base damage, effect) | vs Cobra on tracks | vs light cyborg |
|---|---|---|
| Machinegun (10, anti-personnel) | 10 x 40% = 4, minus 15 is negative, floor to **1** per hit (2 HP/s) | 10 x 150% = 15, minus 12 = 3, floor 4 gives **4** per hit |
| Heavy Cannon (120, all-rounder) | 126, minus 15 = **111** | 78 minus 12 = **66** |
| Lancer (105, anti-tank) | 126 minus 15 = **111** | 31 minus 12 = **19** |

Floor-pinned weapons (per-hit damage at the 33% floor) among the 41 ground-capable (non-AA, non-EMP/electronic) weapons:

| Target (armour kin/heat) | Weapons pinned to the floor |
|---|---|
| Cobra / Wheels (15/6) | 4 (the machine guns) |
| Cobra / Tracks (15/6) | **10** (all five machine guns, Mortar, Pepperpot, both rocket arrays, Ripple Rockets) |
| Python / Tracks (20/9) | 11 |
| Vengeance / Tracks (28/25) | **17** (adds Light Cannon, flamers, Seraph, Bombard...) |
| Light cyborg (12/6) | 2 |

### The upgrade race: armour outruns small weapons

If both sides take the same number of research steps (weapon damage +25% of base per step; kinetic armour +30% of base per
step; rounding up each step), per-hit damage over steps 0 to 9 is:

| Weapon vs target | Step 0 | 3 | 6 | 9 | Pinned at the floor? |
|---|---:|---:|---:|---:|---|
| Machinegun (10) vs Cobra tracks (arm 15) | 1 | 2 | 3 | 4 | **always** |
| Heavy Machinegun (17) vs Cobra tracks | 1 | 3 | 5 | 7 | **always** |
| Assault Gun (19) vs Python tracks (arm 20) | 2 | 4 | 6 | 8 | **always** |
| Mortar (60, artillery) vs Cobra tracks | 5 | 10 | 14 | 19 | **always** |
| Light Cannon (35) vs Cobra tracks | 21 | 35 | 48 | 61 | no |
| Heavy Cannon (120) vs Python tracks (arm 20) | 106 | 182 | 259 | 335 | no |
| Lancer (105) vs Cobra tracks | 111 | 193 | 275 | 357 | no |
| Tank Killer (180) vs Python tracks | 196 | 340 | 484 | 628 | no |

A +30% armour step on a base armour of 15 is +5 points per step, while a +25% damage step on a 10-damage weapon is +3. Small
weapons can never catch up; large weapons' damage grows many times faster than armour (+30 or +45 per step). This is a **structural
property of flat armour plus percentage upgrades**: it creates a permanent "small weapons are cannon fodder vs armoured
tanks" tier regardless of research.

> **Theory note: flat vs. percentage mitigation.** Percentage mitigation (armour removes x% of damage) scales with hit size,
> so all weapons keep the same *relative* effectiveness as armour grows; flat mitigation (armour subtracts n points) favours
> large hits and makes small weapons obsolete against heavy armour. A floor (33%) stops the obsolescence from becoming total
> immunity. WZ2100's choice makes weapon *size* a first-class counter, in addition to the effect matrix.

## 5. Accuracy, misses and projectiles

- **Hit chance** uses two flat bands (no interpolation): `shortHit` between minimum and short range, `longHit` out to long range;
  beyond long range the weapon does not fire. The roll is `random(0..99) <= chance`, so a listed 75 hits 76% of the time.
- **Not in the formula:** elevation, target speed, propulsion, body size or ECM. Visibility is a gate.
- **Rank** changes the chance by +5% x attacker level and -5% x defender level (relative to the base band value; commander bonus
  included). Accuracy research adds +10% of the base to both bands per topic.
- **Misses** land near the predicted point (offset grows with distance and low chance), continue, and can hit anything
  non-allied on the way; they stop at 120% (machine guns, command), 100% (AA) or 150% (others) of max range.
- **Flight models:** DIRECT (straight), INDIRECT (ballistic arc, gravity 1000 u/s^2, speed raised to reach the target), HOMING-DIRECT
  (re-aims every tick), HOMING-INDIRECT (terrain-following). Droid targets are *led* by speed x flight time.
- Weapons with `fireOnMove` false cannot fire while moving and root the unit for 1.5 s after firing.
- Steep indirect arcs forced by terrain reduce range and chance.

## 6. Rate of fire, ammunition and nominal DPS

- Times in the JSON are in **tenths of a second**. Single-shot weapons fire once per `firePause`. **Salvo weapons**
  (`reloadTime` > 0): `numRounds` shots with `firePause` between them, then wait for `reloadTime`; the cycle is
  `(N-1) x pause + reload`.
- Ground units have **unlimited ammunition**. VTOLs have a sortie of `numAttackRuns` (times rounds for salvos) and rearm on pads
  (the pad rearms until points equal the droid's weight, so heavier VTOLs rearm slower).
- Research reduces `firePause` and `reloadTime` by a percentage of the base value.

Nominal DPS (damage x shots / cycle; no modifiers; 52 ground weapons incl. AA): median about **25**, range from
1.4 (Heavy Plasma Launcher, a 35 s cycle) to 643 (Stormbringer AA Laser, anti-air). Among weapons that can hit ground:
Scourge Missile 71, Twin Assault Gun 70, Heavy Laser 63, Seraph Missile Array 54; the Machinegun is 20 DPS at 10 power (the best
cost efficiency of any weapon that can hit ground, at 2.0 DPS per power), while Ripple Rockets, Ground Shaker and the EMP and plasma launchers
sit near 0.02 DPS per power and are valued for range and effect, not damage.

## 7. Area damage, burn, penetration, EMP and electronic warfare

- **Splash** (`radius`, `radiusDamage`): every live object whose *centre* is within the radius takes a full, **flat** hit (no
  falloff with distance), friendlies included unless the weapon has `NoFriendlyFire`. The primary target is excluded from the
  splash. Misses that hit terrain still splash.
- **Burn zones** (`periodicalDamage`): per-second damage in a radius for a duration (5 weapons: incendiary bombs, howitzer, mortar).
  Overlapping zones do not stack (the highest rate wins). Droids also burn for 10 s after leaving a fire.
- **Penetration:** flamers and gauss weapons (12 weapons) continue through droids, hitting each once.
- **EMP:** any EMP hit stops a droid from acting or moving for 10 s after its last hit; `empRadius` hits do no damage. Structures
  are not disabled.
- **Electronic warfare** (the Nexus Link): bypasses armour entirely. The hit's damage reduces the target's `resistance`; at 0 the
  target is captured. Droid resistance starts at 150 (more with experience) and regenerates slowly (+1 per 0.83 s on droids,
  +1 per 2 s on structures); research raises it (+90). This is a complete third damage layer beside kinetic and thermal.

## 8. Targeting, sensors and visibility

**Target choice** (`src/ai.cpp`): candidates must be fully visible to the player or allies and pass an air/ground check. Each gets a
score. Against a droid: `effect-vs-propulsion modifier + effect-vs-body-size modifier - 13 x tiles + 130 x damage fraction +
type bonus` (armed units 52, commanders 78, builders/repair 65). Against a structure: `strength modifier - 13 x tiles +
91 x damage fraction + bonus` (defences 52, derricks 65, factories 13). Penalties: /8 if unbuilt, /10 if not visible to the
attacker itself, /20 inside minimum range, /4 with no line of fire. "Doomed" targets (incoming damage over 120% of HP) are
scored /10. A switch needs +52 points. Armour is ignored. 100 modifier points are worth about 7.7 tiles, so matchup biases
targeting but does not override proximity.

**Indirect fire** needs team vision, not the shooter's own sensor: artillery takes targets observed by any allied sensor and prefers
counter-battery targets. Which sensor pairs with which weapon is hard-coded (ground indirect with standard, CB or wide-spectrum
sensors; VTOLs with VTOL strike, VTOL CB or wide-spectrum sensors, or a commander).

**Sensors:** every unit has a default sensor (1,024 range, upgraded +25%, +15%, +10%); sensor turret 1,536; wide-spectrum 2,260
(acts as all types); CB 2,048 (reveals enemy artillery whose *hit* lands within range of the victim); VTOL strike 2,048; radar
detector 1,024 (shows active radars within 10x range as blips).

**Visibility** is a per-player byte on every object (255 = seen, 127 = blip), fading in at 510/s and out at 50/s; allies share vision;
a satellite uplink sees everything; sensors use terrain-occluded wavecasts. Anything within 4 tiles of a viewer cannot be jammed.

**ECM** is a binary jam over its range (a jammed target seen only by a long-range sensor is a blip and cannot be shot); VTOLs ignore
it. **It is dormant in shipped data:** no research references ECM, and the ECM turret is not designable (only a Jammer
Tower structure uses it).

## 9. Experience, commanders and repair

- **Experience** is a continuous accumulator: the fraction of the victim's max HP dealt, a kill giving the remainder, scaled by
  cost and build-point ratios between 0.5x and 2x. It is shared with the commander and credited to sensor designators. Recycled
  units bank their experience for the next build.
- **Rank** (0-8, "Rookie" to "Hero") by kill thresholds (ordinary units 0, 2, 4 ... 16; commanders 0, 12 ... 96). Per level:
  **-6% damage taken**, **+5% accuracy** (and -5% enemy accuracy), **+5% speed**, more electronic resistance; **no extra outgoing
  damage**. At rank 8: damage taken x0.52, accuracy +/-40%: roughly **2.7x fight value** (1/0.52 x 1.4).
- **Commanders:** a group of 6 + 2 per rank non-artillery units; attached units get effective level max(own+1, commander level)
  within a range of 512-1,536 units, and the commander receives every unit's experience. Its turret is a damage-4 heat designator.
- **Repair:** a repair turret heals its `repairPoints` per second within 2 tiles (light 15, heavy 30, cyborg 10); the repair facility
  heals 50 HP/s, one droid at a time; **auto-repair** heals about 13 HP/s on droids (not VTOLs); structures self-heal about 3.3 HP/s.

## 10. Hard-coded versus data-driven

All of the fixed lists below are compiled into the engine; a modder can change values but not add members without C++:
weapon classes (2), sub-classes (16), effects (6), movement models (4), structure strengths (4), propulsion types (7), body sizes
(4), droid types (15), component types (8), armour channels (2), weapon flags (8), `MAX_WEAPONS` (3). The modifier tables are
statically sized by those enums. The **research-upgrade filter system is generic** (any exposed stat can be filtered). Details and
effort estimates are in [07-limits-and-extensibility.md](07-limits-and-extensibility.md).

## 11. Findings for our design work

1. **Real counter-play exists** where an effect meets a propulsion/structure class (anti-personnel vs. cyborgs, anti-tank vs. tanks,
   bunker busters vs. bunkers, anti-air vs. VTOLs, CB vs. artillery, EMP). These are the system's best features.
2. **Coarse keys.** Six effects and seven propulsion types are all that the matrix can express. A richer game wants more armour
   types (and per-component armour), more damage types, and traits instead of a closed enum; a single damage chokepoint
   (`objDamage`) makes armour generalisation easy.
3. **Flat armour + percentage upgrades = permanent cannon-fodder weapons.** Decide deliberately whether that is a feature
   (weapon size as a counter) or a trap (weapons that are strictly dead late).
4. **Sensors, visibility and indirect fire form a clean information layer**, but ECM and electronic warfare are mostly
   dormant or niche. The takeover weapon shows how a third, armour-bypassing damage layer can coexist.
5. **Experience is a strong, hidden multiplier** (2.7x at max rank) with no counter-play; any future design that includes
   experience should expose and budget it.
6. **Latent hooks worth studying:** the unused body-size modifier axis, the dormant `weaponSize` rule, and per-turret hit points.
