# Research tree analysis (mp)

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generator: `python3 -I tools/research/wz_research_graph.py <wz_checkout> mp`.

- Research items: **390**
- Dangling prerequisite ids (not in file): none
- Root items (no prerequisites): 6; leaf items (nothing depends on them): 118
- Longest prerequisite chain: **19 items** (depth 0..18)
- Prerequisites per item: mean 1.40, max 4; items with >=2 prereqs: 142 (36%)
- Direct dependents per item: mean 1.40, max 13; items gating nothing: 118
- Items with `requiredStructures`: 1; with `disabledWhen`: 14
- Items that make components redundant (`redComponents`): 28; structures redundant: 26; `replacedComponents`: 1
- Research points: total 3,590,910, mean 9207, median 6000, max 56,600
- Research power cost: total 85,414, mean 219, max 450
- Most expensive path (sum of researchPoints along the costliest chain): 220,510

## What each research item does

| Effect | Items | Share |
|---|---:|---:|
| upgrade | 204 | 52% |
| component | 96 | 25% |
| structure | 86 | 22% |
| component+structure | 2 | 1% |
| component+upgrade | 1 | 0% |
| structure+upgrade | 1 | 0% |

## Categories

| Category | Items | Avg points | Avg depth | Deepest | Effect mix |
|---|---:|---:|---:|---:|---|
| (none) | 190 | 7223 | 9.0 | 17 | component:96, structure:86, upgrade:5, component+structure:2, structure+upgrade:1 |
| Wall | 12 | 11433 | 11.0 | 18 | upgrade:12 |
| MG Damage | 10 | 6580 | 6.6 | 12 | upgrade:10 |
| Cyborg Thermal | 9 | 10267 | 11.7 | 16 | upgrade:9 |
| Cyborg Alloys | 9 | 7867 | 8.7 | 13 | upgrade:8, component+upgrade:1 |
| Research | 9 | 6889 | 8.0 | 12 | upgrade:9 |
| Tank Thermal | 9 | 12978 | 11.7 | 16 | upgrade:9 |
| Engine | 9 | 8933 | 8.2 | 15 | upgrade:9 |
| Tank Alloys | 9 | 9600 | 8.7 | 13 | upgrade:9 |
| Cannon Damage | 9 | 6806 | 7.0 | 11 | upgrade:9 |
| Flamer Damage | 9 | 8611 | 7.3 | 14 | upgrade:9 |
| Rocket Damage | 9 | 6000 | 7.7 | 12 | upgrade:9 |
| Power | 6 | 8500 | 10.5 | 13 | upgrade:6 |
| VTOL Rearming | 6 | 9133 | 10.5 | 13 | upgrade:6 |
| Cannon Reload | 6 | 9000 | 9.5 | 12 | upgrade:6 |
| Howitzers Damage | 6 | 19133 | 13.5 | 16 | upgrade:6 |
| Mortar Damage | 6 | 6750 | 7.5 | 10 | upgrade:6 |
| Production | 4 | 11500 | 9.5 | 14 | upgrade:4 |
| Howitzers Reload | 4 | 36850 | 15.5 | 17 | upgrade:4 |
| Mortar Reload | 4 | 5175 | 9.5 | 11 | upgrade:4 |
| Rocket Accuracy | 4 | 5400 | 8.0 | 10 | upgrade:4 |
| Buildings | 3 | 6800 | 9.0 | 14 | upgrade:3 |
| Engineering | 3 | 5200 | 6.7 | 12 | upgrade:3 |
| Sensor | 3 | 4200 | 9.3 | 11 | upgrade:3 |
| Bomb Damage | 3 | 9200 | 9.0 | 10 | upgrade:3 |
| Laser Damage | 3 | 28800 | 13.0 | 14 | upgrade:3 |
| Laser Reload | 3 | 28800 | 14.0 | 15 | upgrade:3 |
| Flamer Reload | 3 | 4967 | 6.3 | 8 | upgrade:3 |
| Howitzers Accuracy | 3 | 9200 | 12.0 | 13 | upgrade:3 |
| MG Reload | 3 | 4800 | 6.3 | 8 | upgrade:3 |
| Missile Damage | 3 | 28800 | 14.0 | 15 | upgrade:3 |
| Missile Reload | 3 | 28800 | 13.0 | 14 | upgrade:3 |
| Mortar Accuracy | 3 | 4800 | 7.0 | 8 | upgrade:3 |
| Rail Damage | 3 | 28800 | 13.3 | 15 | upgrade:3 |
| Rail Reload | 3 | 28800 | 15.0 | 16 | upgrade:3 |
| Rocket Reload | 3 | 5600 | 6.0 | 7 | upgrade:3 |
| Cannon Accuracy | 2 | 4200 | 6.5 | 8 | upgrade:2 |
| Missile Accuracy | 2 | 21600 | 14.5 | 15 | upgrade:2 |
| Laser Accuracy | 1 | 14400 | 12.0 | 12 | upgrade:1 |
| Rail Accuracy | 1 | 14400 | 13.0 | 13 | upgrade:1 |

## Depth profile (items per prerequisite-depth)

| Depth | Items | Avg points | Of which unlock components | Of which unlock structures | Of which upgrades |
|---:|---:|---:|---:|---:|---:|
| 0 | 6 | 662 | 5 | 0 | 1 |
| 1 | 10 | 840 | 4 | 5 | 2 |
| 2 | 12 | 1100 | 2 | 6 | 5 |
| 3 | 13 | 1569 | 2 | 5 | 6 |
| 4 | 16 | 2278 | 5 | 1 | 10 |
| 5 | 19 | 3074 | 3 | 3 | 13 |
| 6 | 20 | 4280 | 8 | 2 | 10 |
| 7 | 31 | 4977 | 12 | 6 | 13 |
| 8 | 35 | 4817 | 4 | 9 | 23 |
| 9 | 27 | 6430 | 8 | 3 | 17 |
| 10 | 30 | 7796 | 11 | 3 | 16 |
| 11 | 37 | 7770 | 9 | 11 | 17 |
| 12 | 40 | 10578 | 10 | 11 | 19 |
| 13 | 32 | 13462 | 5 | 8 | 19 |
| 14 | 24 | 22617 | 5 | 4 | 15 |
| 15 | 19 | 22374 | 3 | 5 | 11 |
| 16 | 11 | 25091 | 2 | 3 | 6 |
| 17 | 7 | 32114 | 1 | 4 | 2 |
| 18 | 1 | 24000 | 0 | 0 | 1 |

## Components unlocked by research

| Kind | Designable? | Count |
|---|---|---:|
| Body | no | 4 |
| Body | yes | 14 |
| Brain | yes | 1 |
| Construct | no | 1 |
| Construct | yes | 1 |
| Propulsion | no | 1 |
| Propulsion | yes | 5 |
| Repair | no | 2 |
| Repair | yes | 2 |
| Sensor | yes | 6 |
| Weapon | no | 18 |
| Weapon | yes | 77 |

Components unlocked by more than one research item: 0

## Upgrade effects (`results`) by class and parameter

| Class | Parameter | Uses |
|---|---|---:|
| Body | Thermal | 27 |
| Body | Armour | 27 |
| Body | HitPointPct | 27 |
| Body | Power | 9 |
| Body | Resistance | 1 |
| Brain | BaseCommandLimit | 1 |
| Brain | CommandLimitByLevel | 1 |
| Brain | RankThresholds | 1 |
| Brain | HitPoints | 1 |
| Building | Armour | 15 |
| Building | HitPoints | 15 |
| Building | ResearchPoints | 9 |
| Building | PowerPoints | 7 |
| Building | RearmPoints | 6 |
| Building | ProductionPoints | 4 |
| Building | RepairPoints | 3 |
| Building | Resistance | 1 |
| Construct | ConstructorPoints | 3 |
| Repair | RepairPoints | 9 |
| Sensor | Range | 3 |
| Weapon | Damage | 71 |
| Weapon | RadiusDamage | 52 |
| Weapon | FirePause | 35 |
| Weapon | ReloadTime | 35 |
| Weapon | RepeatDamage | 24 |
| Weapon | HitChance | 16 |
| Weapon | ShortHitChance | 16 |

Filter parameters used: Weapon.ImpactClass (249), Body.BodyClass (91), Building.Type (31), Repair.Id (9), Brain.Id (2)

## Longest upgrade chains (same class/parameter/filter)

| Class | Parameter | Filter | Steps | Sum of values | Value range per step | Cumulative research points |
|---|---|---|---:|---:|---|---:|
| Building | Armour | Wall | 12 | 420 | 35..35 | 137,200 |
| Building | HitPoints | Wall | 12 | 360 | 30..30 | 137,200 |
| Weapon | Damage | MACHINE GUN | 10 | 250 | 25..25 | 65,800 |
| Weapon | Damage | A-A GUN | 10 | 210 | 5..25 | 65,800 |
| Weapon | RadiusDamage | A-A GUN | 10 | 210 | 5..25 | 65,800 |
| Body | Thermal | Cyborgs | 9 | 405 | 45..45 | 92,400 |
| Body | Armour | Cyborgs | 9 | 315 | 35..35 | 70,800 |
| Body | HitPointPct | Cyborgs | 9 | 315 | 35..35 | 70,800 |
| Building | ResearchPoints | - | 9 | 270 | 30..30 | 62,000 |
| Body | Thermal | Droids | 9 | 360 | 40..40 | 116,800 |
| Body | Thermal | Transports | 9 | 360 | 40..40 | 116,800 |
| Body | Power | Droids | 9 | 50 | 5..8 | 80,400 |
| Body | Armour | Droids | 9 | 270 | 30..30 | 86,400 |
| Body | HitPointPct | Droids | 9 | 270 | 30..30 | 86,400 |
| Body | Armour | Transports | 9 | 135 | 15..15 | 86,400 |
| Body | HitPointPct | Transports | 9 | 135 | 15..15 | 86,400 |
| Weapon | Damage | CANNON | 9 | 225 | 25..25 | 61,250 |
| Weapon | RadiusDamage | CANNON | 9 | 225 | 25..25 | 61,250 |
| Weapon | Damage | FLAME | 9 | 225 | 25..25 | 77,500 |
| Weapon | RepeatDamage | FLAME | 9 | 225 | 25..25 | 77,500 |
| Weapon | Damage | ROCKET | 9 | 225 | 25..25 | 54,000 |
| Weapon | RadiusDamage | ROCKET | 9 | 225 | 25..25 | 54,000 |
| Building | PowerPoints | - | 7 | 215 | 25..50 | 52,200 |
| Building | RearmPoints | - | 6 | 180 | 30..30 | 54,800 |
| Weapon | FirePause | CANNON | 6 | -60 | -10..-10 | 54,000 |

## Biggest gatekeepers (items whose completion unlocks the most of the tree)

| Item | Name | Transitive dependents | Depth |
|---|---|---:|---:|
| R-Sys-Sensor-Turret01 | Sensor Turret | 266 | 0 |
| R-Sys-Sensor-Tower01 | Sensor Tower | 265 | 1 |
| R-Struc-CommandRelay | Command Relay Post | 263 | 2 |
| R-Sys-Engineering01 | Engineering | 261 | 0 |
| R-Struc-Research-Module | Research Module | 261 | 3 |
| R-Struc-Research-Upgrade01 | Synaptic Link Data Analysis | 236 | 4 |
| R-Struc-Research-Upgrade02 | Synaptic Link Data Analysis Mk2 | 222 | 5 |
| R-Struc-Research-Upgrade03 | Synaptic Link Data Analysis Mk3 | 218 | 6 |
| R-Struc-Research-Upgrade04 | Dedicated Synaptic Link Data Analysis | 208 | 7 |
| R-Vehicle-Engine01 | Fuel Injection Engine | 201 | 1 |
| R-Struc-Research-Upgrade05 | Dedicated Synaptic Link Data Analysis Mk2 | 150 | 8 |
| R-Wpn-MG1Mk1 | Machinegun | 149 | 0 |
| R-Wpn-MG-Damage01 | Hardened MG Bullets | 147 | 1 |
| R-Struc-Research-Upgrade06 | Dedicated Synaptic Link Data Analysis Mk3 | 146 | 9 |
| R-Struc-PowerModuleMk1 | Power Module | 142 | 2 |

## Structures unlocked by research (by structure type)

| Type | Count |
|---|---:|
| DEFENSE | 72 |
| FORTRESS | 4 |
| WALL | 2 |
| GATE | 1 |
| CORNER WALL | 1 |
| COMMAND RELAY | 1 |
| CYBORG FACTORY | 1 |
| FACTORY MODULE | 1 |
| POWER MODULE | 1 |
| REPAIR FACILITY | 1 |
| RESEARCH MODULE | 1 |
| VTOL FACTORY | 1 |
| REARM PAD | 1 |
| SAT UPLINK | 1 |
| LASSAT | 1 |

## Items whose results touch body `Power` (engine output)

| Item | Name | Depth | Points | Power cost | Requires | Value(s) |
|---|---|---:|---:|---:|---|---|
| R-Vehicle-Engine01 | Fuel Injection Engine | 1 | 1200 | 37 | R-Sys-Engineering01 | 5% Droids |
| R-Vehicle-Engine02 | Fuel Injection Engine Mk2 | 2 | 2400 | 75 | R-Vehicle-Engine01 | 5% Droids |
| R-Vehicle-Engine03 | Fuel Injection Engine Mk3 | 3 | 4800 | 150 | R-Vehicle-Engine02 | 5% Droids |
| R-Vehicle-Engine04 | Turbo-Charged Engine | 8 | 7000 | 218 | R-Vehicle-Engine03, R-Struc-Research-Upgrade04 | 5% Droids |
| R-Vehicle-Engine05 | Turbo-Charged Engine Mk2 | 9 | 9000 | 281 | R-Vehicle-Engine04 | 5% Droids |
| R-Vehicle-Engine06 | Turbo-Charged Engine Mk3 | 10 | 11000 | 343 | R-Vehicle-Engine05 | 5% Droids |
| R-Vehicle-Engine07 | Gas Turbine Engine | 12 | 13000 | 406 | R-Vehicle-Engine06, R-Vehicle-Prop-VTOL, R-Vehicle-Metals07 | 5% Droids |
| R-Vehicle-Engine08 | Gas Turbine Engine Mk2 | 14 | 15000 | 450 | R-Vehicle-Body07, R-Vehicle-Engine07 | 7% Droids |
| R-Vehicle-Engine09 | Gas Turbine Engine Mk3 | 15 | 17000 | 450 | R-Vehicle-Engine08, R-Vehicle-Body10 | 8% Droids |

## Cross-family prerequisite edges (family of prerequisite -> family of dependent)

Shows which research lines gate which other lines. Edges within one family (upgrade chains) are excluded.

| Prerequisite family | Dependent family | Edges |
|---|---|---:|
| R-Wpn-Rocket | R-Defense | 9 |
| R-Vehicle-Metals | R-Vehicle-Body | 9 |
| R-Wpn-AAGun | R-Defense | 7 |
| R-Wpn-RailGun | R-Defense | 6 |
| R-Cyborg-Metals | R-Cyborg-Hvywpn | 5 |
| R-Wpn-Missile | R-Defense | 5 |
| R-Vehicle-Engine | R-Vehicle-Body | 5 |
| R-Sys-Engineering | R-Defense | 4 |
| R-Wpn-MG | R-Defense | 4 |
| R-Wpn-CannonMk | R-Defense | 4 |
| R-Struc-Research | R-Sys-Sensor | 4 |
| R-Wpn-HowitzerMk | R-Wpn-Howitzer | 4 |
| R-Struc-Research | R-Wpn-MG | 4 |
| R-Wpn-Laser | R-Defense | 3 |
| R-Defense | R-Struc-Materials | 3 |
| R-Struc-Research | R-Vehicle-Metals | 3 |
| R-Wpn-MGMk | R-Wpn-MG | 3 |
| R-Cyborg-Metals | R-Cyborg-Armor | 2 |
| R-Struc-Research | R-Cyborg-Metals | 2 |
| R-Wpn-CannonAMk | R-Defense | 2 |
| R-Wpn-MissileA | R-Defense | 2 |
| R-Wpn-Howitzer | R-Defense | 2 |
| R-Wpn-Mortar | R-Defense | 2 |
| R-Wpn-Cannon | R-Defense | 2 |
| R-Wpn-MGMk | R-Defense | 2 |
| R-Sys-Engineering | R-Struc-Factory | 2 |
| R-Struc-Research | R-Struc-Power | 2 |
| R-Struc-Factory | R-Struc-RprFac | 2 |
| R-Sys-Sensor | R-Sys-CBSensor | 2 |
| R-Struc-Research | R-Sys-Engineering | 2 |

## Which research lines gate each designable body

| Body research | Requires |
|---|---|
| Light Body - Viper |  |
| Medium Body - Cobra | Factory Module |
| Light Body - Bug | Composite Alloys |
| Medium Body - Scorpion | Light Body - Bug, Composite Alloys Mk2 |
| Heavy Body - Python | Medium Body - Cobra, Composite Alloys Mk2 |
| Light Body - Leopard | Composite Alloys Mk3 |
| Heavy Body - Mantis | Medium Body - Scorpion, Fuel Injection Engine Mk3 |
| Medium Body - Panther | Light Body - Leopard, Medium Body - Cobra, Dense Composite Alloys |
| Heavy Body - Tiger | Heavy Body - Python, Medium Body - Panther, Dense Composite Alloys Mk2 |
| Light Body - Retaliation | Superdense Composite Alloys, Turbo-Charged Engine Mk2 |
| Medium Body - Retribution | Light Body - Retaliation, Turbo-Charged Engine Mk3, Superdense Composite Alloys Mk2 |
| Heavy Body - Vengeance | Medium Body - Retribution, Superdense Composite Alloys Mk3, Gas Turbine Engine |
| Heavy Body - Wyvern | Gas Turbine Engine Mk3, High Intensity Thermal Armor Mk3 |
| Multi Turret Body - Dragon | Heavy Body - Wyvern |
