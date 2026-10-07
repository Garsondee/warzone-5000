# Research tree: full list in depth order

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generator: `python3 -I tools/research/wz_research_graph.py <wz_checkout> mp --list`.

| Depth | Id | Name | Category | Points | Power | Requires | Effect |
|---:|---|---|---|---:|---:|---|---|
| 0 | R-Sys-Engineering01 | Engineering | Engineering | 1200 | 37 |  | upgrade |
| 0 | R-Sys-Sensor-Turret01 | Sensor Turret |  | 900 | 28 |  | component |
| 0 | R-Sys-Spade1Mk1 | Construction Unit |  | 10 | None |  | component |
| 0 | R-Vehicle-Body01 | Light Body - Viper |  | 600 | 18 |  | component |
| 0 | R-Vehicle-Prop-Wheels | Wheeled Propulsion |  | 1200 | 37 |  | component |
| 0 | R-Wpn-MG1Mk1 | Machinegun |  | 60 | 1 |  | component |
| 1 | R-Defense-HardcreteWall | Hardcrete |  | 600 | 18 | R-Sys-Engineering01 | structure |
| 1 | R-Defense-TankTrap01 | Tank Traps |  | 300 | 9 | R-Sys-Engineering01 | structure |
| 1 | R-Defense-Tower01 | Heavy Machinegun Guard Tower |  | 600 | 18 | R-Wpn-MG1Mk1 | structure |
| 1 | R-Struc-Factory-Cyborg | Cyborg Factory |  | 1800 | 56 | R-Sys-Engineering01 | component+structure |
| 1 | R-Sys-MobileRepairTurret01 | Mobile Repair Turret |  | 600 | 18 | R-Sys-Engineering01 | component |
| 1 | R-Sys-Sensor-Tower01 | Sensor Tower |  | 900 | 28 | R-Sys-Sensor-Turret01 | structure |
| 1 | R-Vehicle-Engine01 | Fuel Injection Engine | Engine | 1200 | 37 | R-Sys-Engineering01 | upgrade |
| 1 | R-Vehicle-Prop-Halftracks | Half-tracked Propulsion |  | 1200 | 37 | R-Sys-Engineering01 | component |
| 1 | R-Wpn-Flamer01Mk1 | Flamer |  | 600 | 18 | R-Sys-Engineering01 | component |
| 1 | R-Wpn-MG-Damage01 | Hardened MG Bullets | MG Damage | 600 | 18 | R-Wpn-MG1Mk1 | upgrade |
| 2 | R-Defense-HardcreteGate | Hardcrete Gate |  | 600 | 18 | R-Defense-HardcreteWall | structure |
| 2 | R-Defense-Pillbox01 | Heavy Machinegun Bunker |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-MG-Damage01 | structure |
| 2 | R-Defense-Pillbox05 | Flamer Bunker |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-Flamer01Mk1 | structure |
| 2 | R-Defense-WallUpgrade01 | Improved Hardcrete | Wall | 1200 | 37 | R-Defense-HardcreteWall | upgrade |
| 2 | R-Struc-CommandRelay | Command Relay Post |  | 1200 | 37 | R-Sys-Sensor-Tower01 | structure |
| 2 | R-Struc-PowerModuleMk1 | Power Module |  | 1200 | 37 | R-Vehicle-Engine01 | structure+upgrade |
| 2 | R-Sys-Sensor-Tower02 | Hardened Sensor Tower |  | 600 | 18 | R-Defense-HardcreteWall, R-Sys-Sensor-Tower01 | structure |
| 2 | R-Vehicle-Engine02 | Fuel Injection Engine Mk2 | Engine | 2400 | 75 | R-Vehicle-Engine01 | upgrade |
| 2 | R-Wpn-Cannon1Mk1 | Light Cannon |  | 1800 | 56 | R-Wpn-MG-Damage01 | component |
| 2 | R-Wpn-Flamer-Damage01 | High Temperature Flamer Gel | Flamer Damage | 600 | 18 | R-Wpn-Flamer01Mk1 | upgrade |
| 2 | R-Wpn-MG-Damage02 | APDSB MG Bullets | MG Damage | 1200 | 37 | R-Wpn-MG-Damage01 | upgrade |
| 2 | R-Wpn-Rocket05-MiniPod | Mini-Rocket Pod |  | 1200 | 37 | R-Vehicle-Engine01 | component |
| 3 | R-Comp-CommandTurret01 | Command Turret |  | 600 | 18 | R-Struc-CommandRelay | component |
| 3 | R-Defense-Pillbox04 | Light Cannon Bunker |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-Cannon1Mk1 | structure |
| 3 | R-Defense-Tower06 | Mini-Rocket Tower |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-Rocket05-MiniPod | structure |
| 3 | R-Defense-WallTower02 | Light Cannon Hardpoint |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-Cannon1Mk1 | structure |
| 3 | R-Defense-WallUpgrade02 | Improved Hardcrete Mk2 | Wall | 2400 | 75 | R-Defense-WallUpgrade01 | upgrade |
| 3 | R-Struc-Factory-Module | Factory Module |  | 2400 | 75 | R-Struc-Factory-Cyborg, R-Struc-PowerModuleMk1 | structure |
| 3 | R-Struc-Materials01 | Reinforced Base Structure Materials | Buildings | 2400 | 75 | R-Defense-WallUpgrade01 | upgrade |
| 3 | R-Struc-Research-Module | Research Module |  | 1200 | 37 | R-Struc-CommandRelay | structure |
| 3 | R-Vehicle-Engine03 | Fuel Injection Engine Mk3 | Engine | 4800 | 150 | R-Vehicle-Engine02 | upgrade |
| 3 | R-Wpn-Cannon-Damage01 | HEAT Cannon Shells | Cannon Damage | 1200 | 37 | R-Wpn-Cannon1Mk1 | upgrade |
| 3 | R-Wpn-Flamer-Damage02 | High Temperature Flamer Gel Mk2 | Flamer Damage | 1200 | 37 | R-Wpn-Flamer-Damage01 | upgrade |
| 3 | R-Wpn-MG2Mk1 | Twin Machinegun |  | 1200 | 37 | R-Wpn-MG-Damage02 | component |
| 3 | R-Wpn-Rocket-Damage01 | HE Rockets | Rocket Damage | 1200 | 37 | R-Wpn-Rocket05-MiniPod | upgrade |
| 4 | R-Cyborg-Metals01 | Cyborg Composite Alloys | Cyborg Alloys | 1200 | 37 | R-Struc-Factory-Module, R-Struc-Research-Module | upgrade |
| 4 | R-Defense-WallUpgrade03 | Improved Hardcrete Mk3 | Wall | 3600 | 112 | R-Defense-WallUpgrade02 | upgrade |
| 4 | R-Struc-Factory-Upgrade01 | Automated Manufacturing | Production | 2400 | 75 | R-Struc-Factory-Module | upgrade |
| 4 | R-Struc-RepairFacility | Repair Facility |  | 2400 | 75 | R-Sys-MobileRepairTurret01, R-Struc-Factory-Module | structure |
| 4 | R-Struc-Research-Upgrade01 | Synaptic Link Data Analysis | Research | 1200 | 37 | R-Struc-Research-Module | upgrade |
| 4 | R-Sys-MobileRepairTurretHvy | Heavy Repair Turret |  | 2400 | 75 | R-Sys-MobileRepairTurret01, R-Struc-Factory-Module | component |
| 4 | R-Vehicle-Body05 | Medium Body - Cobra |  | 1200 | 37 | R-Struc-Factory-Module | component |
| 4 | R-Vehicle-Metals01 | Composite Alloys | Tank Alloys | 1800 | 56 | R-Struc-Factory-Module, R-Struc-Research-Module | upgrade |
| 4 | R-Vehicle-Prop-Hover | Hover Propulsion |  | 3600 | 112 | R-Vehicle-Engine02, R-Struc-Factory-Module | component |
| 4 | R-Wpn-Cannon-Damage02 | HEAT Cannon Shells Mk2 | Cannon Damage | 2450 | 76 | R-Wpn-Cannon-Damage01 | upgrade |
| 4 | R-Wpn-Cannon2Mk1 | Medium Cannon |  | 3600 | 112 | R-Struc-Factory-Module, R-Wpn-Cannon-Damage01 | component |
| 4 | R-Wpn-Flamer-Damage03 | High Temperature Flamer Gel Mk3 | Flamer Damage | 2500 | 75 | R-Wpn-Flamer-Damage02 | upgrade |
| 4 | R-Wpn-Flamer-ROF01 | Flamer Autoloader | Flamer Reload | 900 | 28 | R-Wpn-Flamer-Damage02 | upgrade |
| 4 | R-Wpn-MG-Damage03 | APDSB MG Bullets Mk2 | MG Damage | 2400 | 75 | R-Wpn-MG2Mk1 | upgrade |
| 4 | R-Wpn-Mortar01Lt | Mortar |  | 2400 | 75 | R-Wpn-Cannon-Damage01 | component |
| 4 | R-Wpn-Rocket-Damage02 | HE Rockets Mk2 | Rocket Damage | 2400 | 75 | R-Wpn-Rocket-Damage01 | upgrade |
| 5 | R-Cyborg-Metals02 | Cyborg Composite Alloys Mk2 | Cyborg Alloys | 2400 | 75 | R-Cyborg-Metals01 | upgrade |
| 5 | R-Defense-MortarPit | Mortar Pit |  | 600 | 18 | R-Defense-HardcreteWall, R-Wpn-Mortar01Lt | structure |
| 5 | R-Defense-WallTower03 | Medium Cannon Hardpoint |  | 800 | 25 | R-Wpn-Cannon2Mk1, R-Defense-WallTower02 | structure |
| 5 | R-Struc-Research-Upgrade02 | Synaptic Link Data Analysis Mk2 | Research | 2400 | 75 | R-Struc-Research-Upgrade01 | upgrade |
| 5 | R-Struc-RprFac-Upgrade01 | Automated Repair Facility |  | 2400 | 75 | R-Struc-Factory-Upgrade01, R-Struc-RepairFacility | upgrade |
| 5 | R-Struc-VTOLFactory | VTOL Factory |  | 6000 | 187 | R-Struc-Factory-Upgrade01, R-Vehicle-Prop-Hover, R-Vehicle-Engine03 | structure |
| 5 | R-Vehicle-Body04 | Light Body - Bug |  | 1200 | 37 | R-Vehicle-Metals01 | component |
| 5 | R-Vehicle-Metals02 | Composite Alloys Mk2 | Tank Alloys | 3600 | 112 | R-Vehicle-Metals01 | upgrade |
| 5 | R-Wpn-Cannon-Accuracy01 | Cannon Laser Rangefinder | Cannon Accuracy | 3600 | 112 | R-Wpn-Cannon-Damage02, R-Struc-Research-Upgrade01 | upgrade |
| 5 | R-Wpn-Cannon-Damage03 | HEAT Cannon Shells Mk3 | Cannon Damage | 3600 | 112 | R-Wpn-Cannon-Damage02 | upgrade |
| 5 | R-Wpn-Flamer-Damage04 | Superhot Flamer Gel | Flamer Damage | 7200 | 225 | R-Wpn-Flamer-Damage03 | upgrade |
| 5 | R-Wpn-MG-Damage04 | APDSB MG Bullets Mk3 | MG Damage | 6000 | 225 | R-Wpn-MG-Damage03 | upgrade |
| 5 | R-Wpn-MG-ROF01 | Chaingun Upgrade | MG Reload | 3600 | 112 | R-Struc-Factory-Upgrade01, R-Wpn-MG-Damage03 | upgrade |
| 5 | R-Wpn-MG3Mk1 | Heavy Machinegun |  | 1200 | 37 | R-Wpn-MG-Damage03 | component |
| 5 | R-Wpn-Mortar-Damage01 | HE Mortar Shells | Mortar Damage | 1800 | 56 | R-Wpn-Mortar01Lt | upgrade |
| 5 | R-Wpn-Rocket-Accuracy01 | Stabilized Rockets | Rocket Accuracy | 3600 | 112 | R-Struc-Research-Upgrade01, R-Wpn-Rocket-Damage02 | upgrade |
| 5 | R-Wpn-Rocket-Damage03 | HE Rockets Mk3 | Rocket Damage | 3600 | 112 | R-Wpn-Rocket-Damage02 | upgrade |
| 5 | R-Wpn-Rocket-ROF01 | Rocket Autoloader | Rocket Reload | 2400 | 75 | R-Wpn-Rocket-Damage02, R-Struc-Factory-Upgrade01 | upgrade |
| 5 | R-Wpn-Rocket02-MRL | Mini-Rocket Array |  | 2400 | 75 | R-Wpn-Rocket-Damage02 | component |
| 6 | R-Cyborg-Metals03 | Cyborg Composite Alloys Mk3 | Cyborg Alloys | 3600 | 112 | R-Cyborg-Metals02 | upgrade |
| 6 | R-Cyborg-Transport | Cyborg Transport |  | 3600 | 112 | R-Struc-VTOLFactory | component |
| 6 | R-Defense-MRL | Mini-Rocket Battery |  | 800 | 25 | R-Wpn-Rocket02-MRL, R-Defense-HardcreteWall | structure |
| 6 | R-Defense-WallTower01 | Heavy Machinegun Hardpoint |  | 600 | 18 | R-Defense-Pillbox01, R-Wpn-MG3Mk1 | structure |
| 6 | R-Struc-Research-Upgrade03 | Synaptic Link Data Analysis Mk3 | Research | 3600 | 112 | R-Struc-Research-Upgrade02 | upgrade |
| 6 | R-Vehicle-Body08 | Medium Body - Scorpion |  | 2400 | 75 | R-Vehicle-Body04, R-Vehicle-Metals02 | component |
| 6 | R-Vehicle-Body11 | Heavy Body - Python |  | 2400 | 75 | R-Vehicle-Body05, R-Vehicle-Metals02 | component |
| 6 | R-Vehicle-Metals03 | Composite Alloys Mk3 | Tank Alloys | 5400 | 168 | R-Vehicle-Metals02 | upgrade |
| 6 | R-Vehicle-Prop-Tracks | Tracked Propulsion |  | 2400 | 75 | R-Vehicle-Prop-Halftracks, R-Vehicle-Metals02 | component |
| 6 | R-Vehicle-Prop-VTOL | VTOL Propulsion |  | 6000 | 187 | R-Struc-VTOLFactory | component |
| 6 | R-Wpn-Cannon-Damage04 | APFSDS Cannon Rounds | Cannon Damage | 6000 | 187 | R-Wpn-Cannon-Damage03 | upgrade |
| 6 | R-Wpn-Cannon4AMk1 | Hyper Velocity Cannon |  | 4800 | 150 | R-Wpn-Cannon-Accuracy01 | component |
| 6 | R-Wpn-Flame2 | Heavy Flamer - Inferno |  | 7200 | 225 | R-Wpn-Flamer-Damage04 | component |
| 6 | R-Wpn-Flamer-Damage05 | Superhot Flamer Gel Mk2 | Flamer Damage | 9200 | 287 | R-Wpn-Flamer-Damage04 | upgrade |
| 6 | R-Wpn-MG-Damage05 | Tungsten-Tipped MG Bullets | MG Damage | 7200 | 287 | R-Wpn-MG-Damage04 | upgrade |
| 6 | R-Wpn-MG-ROF02 | Rapid Fire Chaingun | MG Reload | 4800 | 150 | R-Wpn-MG-ROF01 | upgrade |
| 6 | R-Wpn-Mortar-Acc01 | Mortar Targeting Computer | Mortar Accuracy | 3600 | 112 | R-Struc-Research-Upgrade02, R-Wpn-Mortar-Damage01 | upgrade |
| 6 | R-Wpn-Mortar-Damage02 | HE Mortar Shells Mk2 | Mortar Damage | 3600 | 112 | R-Wpn-Mortar-Damage01 | upgrade |
| 6 | R-Wpn-Rocket-ROF02 | Rocket Autoloader Mk2 | Rocket Reload | 4800 | 150 | R-Wpn-Rocket-ROF01, R-Wpn-Rocket-Damage03 | upgrade |
| 6 | R-Wpn-Rocket01-LtAT | Lancer AT Rocket |  | 3600 | 112 | R-Wpn-Rocket-Damage03 | component |
| 7 | R-Cyborg-Armor-Heat01 | Cyborg Thermal Armor | Cyborg Thermal | 3600 | 112 | R-Cyborg-Metals03 | upgrade |
| 7 | R-Defense-Emplacement-HPVcannon | Hyper Velocity Cannon Emplacement |  | 900 | 28 | R-Wpn-Cannon4AMk1 | structure |
| 7 | R-Defense-HvyFlamer | Inferno Bunker |  | 1000 | 31 | R-Wpn-Flame2 | structure |
| 7 | R-Defense-Pillbox06 | Lancer Tower |  | 900 | 28 | R-Defense-HardcreteWall, R-Wpn-Rocket01-LtAT | structure |
| 7 | R-Defense-WallTower-HPVcannon | Hyper Velocity Cannon Hardpoint |  | 900 | 28 | R-Wpn-Cannon4AMk1 | structure |
| 7 | R-Defense-WallTower06 | Lancer Hardpoint |  | 900 | 28 | R-Wpn-Rocket01-LtAT, R-Defense-HardcreteWall | structure |
| 7 | R-Struc-Factory-Upgrade04 | Robotic Manufacturing | Production | 9200 | 287 | R-Struc-Factory-Upgrade01, R-Struc-Research-Upgrade03 | upgrade |
| 7 | R-Struc-Research-Upgrade04 | Dedicated Synaptic Link Data Analysis | Research | 4800 | 150 | R-Struc-Research-Upgrade03 | upgrade |
| 7 | R-Struc-VTOLPad | VTOL Rearming Pad |  | 3600 | 112 | R-Vehicle-Prop-VTOL | structure |
| 7 | R-SuperTransport | Super Transport |  | 4000 | 125 | R-Cyborg-Transport | component |
| 7 | R-Vehicle-Armor-Heat01 | Thermal Armor | Tank Thermal | 4800 | 150 | R-Vehicle-Metals03 | upgrade |
| 7 | R-Vehicle-Body02 | Light Body - Leopard |  | 2400 | 75 | R-Vehicle-Metals03 | component |
| 7 | R-Vehicle-Body12 | Heavy Body - Mantis |  | 3600 | 112 | R-Vehicle-Body08, R-Vehicle-Engine03 | component |
| 7 | R-Wpn-AAGun01 | AA Cyclone Flak Cannon |  | 6000 | 187 | R-Wpn-Cannon4AMk1 | component |
| 7 | R-Wpn-AAGun03 | Hurricane AA Turret |  | 3600 | 112 | R-Wpn-MG-ROF02 | component |
| 7 | R-Wpn-Bomb01 | Cluster Bomb Bay |  | 5500 | 171 | R-Vehicle-Prop-VTOL | component |
| 7 | R-Wpn-Cannon-Damage05 | APFSDS Cannon Rounds Mk2 | Cannon Damage | 7200 | 225 | R-Wpn-Cannon-Damage04 | upgrade |
| 7 | R-Wpn-Cannon-ROF01 | Cannon Autoloader | Cannon Reload | 4000 | 125 | R-Wpn-Cannon-Damage04 | upgrade |
| 7 | R-Wpn-Flamer-Damage06 | Superhot Flamer Gel Mk3 | Flamer Damage | 11200 | 350 | R-Wpn-Flamer-Damage05 | upgrade |
| 7 | R-Wpn-Flamer-ROF02 | Flamer Autoloader Mk2 | Flamer Reload | 6000 | 187 | R-Wpn-Flame2, R-Wpn-Flamer-ROF01 | upgrade |
| 7 | R-Wpn-MG-Damage06 | Tungsten-Tipped MG Bullets Mk2 | MG Damage | 7800 | 350 | R-Wpn-MG-Damage05 | upgrade |
| 7 | R-Wpn-MG4 | Assault Gun |  | 6000 | 187 | R-Wpn-MG3Mk1, R-Wpn-MG-ROF02 | component |
| 7 | R-Wpn-Mortar-Acc02 | Thermal Imaging Mortar Shells | Mortar Accuracy | 4800 | 150 | R-Wpn-Mortar-Acc01 | upgrade |
| 7 | R-Wpn-Mortar-Damage03 | HE Mortar Shells Mk3 | Mortar Damage | 7200 | 225 | R-Wpn-Mortar-Damage02 | upgrade |
| 7 | R-Wpn-Mortar-Incendiary | Incendiary Mortar |  | 4800 | 150 | R-Wpn-Mortar-Damage02, R-Wpn-Flame2 | component |
| 7 | R-Wpn-Mortar02Hvy | Heavy Mortar - Bombard |  | 7200 | 225 | R-Vehicle-Metals01, R-Wpn-Mortar-Damage02 | component |
| 7 | R-Wpn-Mortar3 | Rotary Mortar - Pepperpot |  | 7200 | 225 | R-Vehicle-Metals01, R-Wpn-Mortar-Damage02 | component |
| 7 | R-Wpn-Rocket-Damage04 | HEAT Rocket Warhead | Rocket Damage | 4800 | 150 | R-Wpn-Rocket01-LtAT | upgrade |
| 7 | R-Wpn-Rocket-ROF03 | Rocket Autoloader Mk3 | Rocket Reload | 9600 | 300 | R-Wpn-Rocket-ROF02 | upgrade |
| 7 | R-Wpn-Rocket03-HvAT | Bunker Buster Rocket |  | 4800 | 150 | R-Struc-Research-Upgrade03, R-Wpn-Rocket01-LtAT | component |
| 7 | R-Wpn-Sunburst | Sunburst AA Rocket Array |  | 6000 | 187 | R-Wpn-Rocket01-LtAT | component |
| 8 | R-Cyborg-Armor-Heat02 | Cyborg Thermal Armor Mk2 | Cyborg Thermal | 4800 | 150 | R-Cyborg-Armor-Heat01 | upgrade |
| 8 | R-Cyborg-Metals04 | Cyborg Dense Composite Alloys | Cyborg Alloys | 5600 | 175 | R-Cyborg-Metals03, R-Struc-Research-Upgrade04 | component+upgrade |
| 8 | R-Defense-AASite-QuadBof | AA Cyclone Flak Cannon Emplacement |  | 1000 | 31 | R-Wpn-AAGun01 | structure |
| 8 | R-Defense-AASite-QuadMg1 | Hurricane AA Site |  | 1000 | 31 | R-Wpn-AAGun03 | structure |
| 8 | R-Defense-HvyMor | Bombard Pit |  | 1000 | 31 | R-Defense-MortarPit, R-Wpn-Mortar02Hvy | structure |
| 8 | R-Defense-MortarPit-Incendiary | Incendiary Mortar Pit |  | 1200 | 37 | R-Wpn-Mortar-Incendiary, R-Defense-MortarPit | structure |
| 8 | R-Defense-RotMG | Rotary MG Bunker |  | 1100 | 34 | R-Wpn-MG4 | structure |
| 8 | R-Defense-RotMor | Pepperpot Pit |  | 1000 | 31 | R-Defense-MortarPit, R-Wpn-Mortar3 | structure |
| 8 | R-Defense-Sunburst | Sunburst AA Site |  | 1000 | 31 | R-Wpn-Sunburst | structure |
| 8 | R-Defense-Wall-RotMg | Assault Gun Hardpoint |  | 1100 | 34 | R-Wpn-MG4 | structure |
| 8 | R-Defense-WallTower-DoubleAAgun | AA Cyclone Flak Cannon Hardpoint |  | 1000 | 31 | R-Wpn-AAGun01 | structure |
| 8 | R-Struc-Power-Upgrade01 | Gas Turbine Generator | Power | 6000 | 187 | R-Struc-Research-Upgrade04, R-Struc-PowerModuleMk1 | upgrade |
| 8 | R-Struc-Research-Upgrade05 | Dedicated Synaptic Link Data Analysis Mk2 | Research | 6000 | 187 | R-Struc-Research-Upgrade04 | upgrade |
| 8 | R-Struc-RprFac-Upgrade04 | Robotic Repair Facility |  | 6000 | 187 | R-Struc-RprFac-Upgrade01, R-Struc-Factory-Upgrade04 | upgrade |
| 8 | R-Struc-VTOLPad-Upgrade01 | Automated VTOL Rearming | VTOL Rearming | 4800 | 150 | R-Struc-VTOLPad | upgrade |
| 8 | R-Sys-Engineering02 | Improved Engineering | Engineering | 4800 | 150 | R-Sys-Engineering01, R-Struc-Research-Upgrade04 | upgrade |
| 8 | R-Sys-Sensor-Upgrade01 | Sensor Upgrade | Sensor | 1800 | 56 | R-Struc-Research-Upgrade04 | upgrade |
| 8 | R-Vehicle-Armor-Heat02 | Thermal Armor Mk2 | Tank Thermal | 7000 | 218 | R-Vehicle-Armor-Heat01 | upgrade |
| 8 | R-Vehicle-Engine04 | Turbo-Charged Engine | Engine | 7000 | 218 | R-Vehicle-Engine03, R-Struc-Research-Upgrade04 | upgrade |
| 8 | R-Vehicle-Metals04 | Dense Composite Alloys | Tank Alloys | 7600 | 237 | R-Struc-Research-Upgrade04, R-Vehicle-Metals03 | upgrade |
| 8 | R-Wpn-Bomb-Damage01 | HE Bomb Shells | Bomb Damage | 7200 | 225 | R-Wpn-Mortar-Damage02, R-Wpn-Bomb01, R-Struc-Research-Upgrade04 | upgrade |
| 8 | R-Wpn-Bomb03 | Phosphor Bomb Bay |  | 7200 | 225 | R-Wpn-Bomb01, R-Wpn-Flamer-Damage03 | component |
| 8 | R-Wpn-Cannon-Accuracy02 | Cannon Laser Designator | Cannon Accuracy | 4800 | 150 | R-Struc-Research-Upgrade04, R-Wpn-Cannon-Accuracy01 | upgrade |
| 8 | R-Wpn-Cannon-Damage06 | APFSDS Cannon Rounds Mk3 | Cannon Damage | 8400 | 262 | R-Wpn-Cannon-Damage05 | upgrade |
| 8 | R-Wpn-Cannon-ROF02 | Cannon Autoloader Mk2 | Cannon Reload | 6000 | 187 | R-Struc-Factory-Upgrade04, R-Wpn-Cannon-ROF01 | upgrade |
| 8 | R-Wpn-Cannon3Mk1 | Heavy Cannon |  | 8800 | 275 | R-Wpn-Cannon-Damage05, R-Vehicle-Metals03, R-Wpn-Cannon2Mk1 | component |
| 8 | R-Wpn-Cannon5 | Assault Cannon |  | 4800 | 150 | R-Wpn-Cannon-ROF01 | component |
| 8 | R-Wpn-Flamer-ROF03 | Flamer Autoloader Mk3 | Flamer Reload | 8000 | 250 | R-Wpn-Flamer-ROF02 | upgrade |
| 8 | R-Wpn-MG-Damage07 | Tungsten-Tipped MG Bullets Mk3 | MG Damage | 8800 | 412 | R-Wpn-MG-Damage06 | upgrade |
| 8 | R-Wpn-MG-ROF03 | Hyper Fire Chaingun Upgrade | MG Reload | 6000 | 187 | R-Wpn-MG4 | upgrade |
| 8 | R-Wpn-Mortar-Acc03 | Target Acquisition Mortar Shells | Mortar Accuracy | 6000 | 187 | R-Wpn-Mortar-Acc02 | upgrade |
| 8 | R-Wpn-Mortar-Damage04 | HEAP Mortar Shells | Mortar Damage | 9200 | 287 | R-Wpn-Mortar-Damage03 | upgrade |
| 8 | R-Wpn-Mortar-ROF01 | Mortar Autoloader | Mortar Reload | 1800 | 56 | R-Wpn-Mortar-Damage03 | upgrade |
| 8 | R-Wpn-Rocket-Accuracy02 | Improved Rocket Wire Guidance | Rocket Accuracy | 4800 | 150 | R-Wpn-Rocket-Damage04, R-Wpn-Rocket-Accuracy01 | upgrade |
| 8 | R-Wpn-Rocket-Damage05 | HEAT Rocket Warhead Mk2 | Rocket Damage | 6000 | 187 | R-Wpn-Rocket-Damage04 | upgrade |
| 9 | R-Cyborg-Armor-Heat03 | Cyborg Thermal Armor Mk3 | Cyborg Thermal | 6000 | 187 | R-Cyborg-Armor-Heat02 | upgrade |
| 9 | R-Cyborg-Hvywpn-Mcannon | Super Heavy-Gunner |  | 4000 | 125 | R-Cyborg-Metals04, R-Wpn-Cannon2Mk1 | component |
| 9 | R-Cyborg-Metals05 | Cyborg Dense Composite Alloys Mk2 | Cyborg Alloys | 7600 | 237 | R-Cyborg-Metals04 | upgrade |
| 9 | R-Defense-Wall-VulcanCan | Assault Cannon Hardpoint |  | 1100 | 34 | R-Wpn-Cannon5 | structure |
| 9 | R-Defense-WallTower04 | Heavy Cannon Hardpoint |  | 1200 | 37 | R-Defense-WallTower03, R-Wpn-Cannon3Mk1 | structure |
| 9 | R-Defense-WallUpgrade04 | Supercrete | Wall | 6000 | 187 | R-Sys-Engineering02, R-Defense-WallUpgrade03 | upgrade |
| 9 | R-Struc-Power-Upgrade01b | Gas Turbine Generator Mk2 | Power | 6000 | 187 | R-Struc-Power-Upgrade01 | upgrade |
| 9 | R-Struc-Research-Upgrade06 | Dedicated Synaptic Link Data Analysis Mk3 | Research | 8000 | 250 | R-Struc-Research-Upgrade05 | upgrade |
| 9 | R-Struc-VTOLPad-Upgrade02 | Automated VTOL Rearming Mk2 | VTOL Rearming | 6000 | 187 | R-Struc-VTOLPad-Upgrade01 | upgrade |
| 9 | R-Sys-CBSensor-Turret01 | CB Turret |  | 4800 | 150 | R-Sys-Sensor-Upgrade01, R-Wpn-Mortar-Acc01 | component |
| 9 | R-Sys-RadarDetector01 | Radar Detector |  | 2000 | 62 | R-Sys-Sensor-Upgrade01 | component+structure |
| 9 | R-Sys-Sensor-Upgrade02 | Sensor Upgrade Mk2 | Sensor | 3600 | 112 | R-Sys-Sensor-Upgrade01, R-Struc-Research-Upgrade05 | upgrade |
| 9 | R-Sys-VTOLStrike-Turret01 | VTOL Strike Turret |  | 7200 | 225 | R-Struc-VTOLPad, R-Sys-Sensor-Upgrade01 | component |
| 9 | R-Vehicle-Armor-Heat03 | Thermal Armor Mk3 | Tank Thermal | 9000 | 281 | R-Vehicle-Armor-Heat02 | upgrade |
| 9 | R-Vehicle-Body06 | Medium Body - Panther |  | 4800 | 150 | R-Vehicle-Body02, R-Vehicle-Body05, R-Vehicle-Metals04 | component |
| 9 | R-Vehicle-Engine05 | Turbo-Charged Engine Mk2 | Engine | 9000 | 281 | R-Vehicle-Engine04 | upgrade |
| 9 | R-Vehicle-Metals05 | Dense Composite Alloys Mk2 | Tank Alloys | 9600 | 300 | R-Vehicle-Metals04 | upgrade |
| 9 | R-Wpn-Bomb-Damage02 | Improved Bomb Warhead | Bomb Damage | 9200 | 287 | R-Wpn-Bomb-Damage01 | upgrade |
| 9 | R-Wpn-Bomb02 | HEAP Bomb Bay |  | 7200 | 225 | R-Wpn-Bomb-Damage01 | component |
| 9 | R-Wpn-Bomb04 | Thermite Bomb Bay |  | 9200 | 287 | R-Wpn-Flamer-Damage05, R-Wpn-Bomb03 | component |
| 9 | R-Wpn-Cannon-Damage07 | HVAPFSDS Cannon Rounds | Cannon Damage | 9600 | 300 | R-Wpn-Cannon-Damage06 | upgrade |
| 9 | R-Wpn-Cannon-ROF03 | Cannon Autoloader Mk3 | Cannon Reload | 8000 | 250 | R-Wpn-Cannon-ROF02 | upgrade |
| 9 | R-Wpn-Mortar-Damage05 | HEAP Mortar Shells Mk2 | Mortar Damage | 9300 | 290 | R-Wpn-Mortar-Damage04 | upgrade |
| 9 | R-Wpn-Mortar-ROF02 | Mortar Autoloader Mk2 | Mortar Reload | 6000 | 187 | R-Wpn-Mortar-ROF01 | upgrade |
| 9 | R-Wpn-Rocket-Damage06 | HEAT Rocket Warhead Mk3 | Rocket Damage | 7200 | 225 | R-Wpn-Rocket-Damage05 | upgrade |
| 9 | R-Wpn-Rocket02-MRLHvy | Heavy Rocket Array |  | 6000 | 187 | R-Wpn-Rocket-Damage05, R-Wpn-Rocket02-MRL | component |
| 9 | R-Wpn-RocketSlow-Accuracy01 | Rocket Laser Designator | Rocket Accuracy | 6000 | 187 | R-Wpn-Rocket-Accuracy02 | upgrade |
| 10 | R-Cyborg-Hvywpn-Acannon | Super Auto-Cannon Cyborg |  | 4000 | 125 | R-Wpn-Cannon5, R-Cyborg-Hvywpn-Mcannon | component |
| 10 | R-Cyborg-Hvywpn-HPV | Super HVC Cyborg |  | 4000 | 125 | R-Cyborg-Hvywpn-Mcannon, R-Wpn-Cannon4AMk1 | component |
| 10 | R-Cyborg-Metals06 | Cyborg Dense Composite Alloys Mk3 | Cyborg Alloys | 9600 | 300 | R-Cyborg-Metals05 | upgrade |
| 10 | R-Defense-MRLHvy | Heavy Rocket Battery |  | 1200 | 37 | R-Defense-HardcreteWall, R-Wpn-Rocket02-MRLHvy | structure |
| 10 | R-Defense-WallUpgrade05 | Supercrete Mk2 | Wall | 8000 | 250 | R-Defense-WallUpgrade04 | upgrade |
| 10 | R-Struc-Materials02 | Hardened Base Structure Materials | Buildings | 6000 | 187 | R-Defense-WallUpgrade04, R-Struc-Materials01 | upgrade |
| 10 | R-Struc-Power-Upgrade01c | Gas Turbine Generator Mk3 | Power | 6000 | 187 | R-Struc-Power-Upgrade01b | upgrade |
| 10 | R-Struc-Research-Upgrade07 | Neural Synapse Research Brain | Research | 10000 | 312 | R-Struc-Research-Upgrade06 | upgrade |
| 10 | R-Struc-VTOLPad-Upgrade03 | Automated VTOL Rearming Mk3 | VTOL Rearming | 8000 | 250 | R-Struc-VTOLPad-Upgrade02 | upgrade |
| 10 | R-Sys-CBSensor-Tower01 | CB Tower |  | 1100 | 34 | R-Sys-CBSensor-Turret01, R-Sys-Sensor-Tower02 | structure |
| 10 | R-Sys-VTOLCBS-Turret01 | VTOL CB Turret |  | 8400 | 262 | R-Sys-CBSensor-Turret01, R-Sys-VTOLStrike-Turret01, R-Sys-Sensor-Upgrade02 | component |
| 10 | R-Sys-VTOLStrike-Tower01 | VTOL Strike Tower |  | 1200 | 37 | R-Sys-VTOLStrike-Turret01, R-Sys-Sensor-Tower02 | structure |
| 10 | R-Vehicle-Body09 | Heavy Body - Tiger |  | 10800 | 337 | R-Vehicle-Body11, R-Vehicle-Body06, R-Vehicle-Metals05 | component |
| 10 | R-Vehicle-Engine06 | Turbo-Charged Engine Mk3 | Engine | 11000 | 343 | R-Vehicle-Engine05 | upgrade |
| 10 | R-Vehicle-Metals06 | Dense Composite Alloys Mk3 | Tank Alloys | 11600 | 362 | R-Vehicle-Metals05 | upgrade |
| 10 | R-Wpn-AAGun04 | Whirlwind AA Turret |  | 10800 | 337 | R-Wpn-MG-ROF03, R-Wpn-AAGun03, R-Struc-Research-Upgrade06 | component |
| 10 | R-Wpn-Bomb-Damage03 | Advanced Bomb Warhead | Bomb Damage | 11200 | 350 | R-Wpn-Bomb-Damage02 | upgrade |
| 10 | R-Wpn-Cannon-Damage08 | HVAPFSDS Cannon Rounds Mk2 | Cannon Damage | 10800 | 337 | R-Wpn-Cannon-Damage07 | upgrade |
| 10 | R-Wpn-Cannon-ROF04 | Cannon Rapid Loader | Cannon Reload | 10000 | 312 | R-Wpn-Cannon-Damage07, R-Wpn-Cannon-ROF03 | upgrade |
| 10 | R-Wpn-Cannon6TwinAslt | Twin Assault Cannon |  | 7050 | 225 | R-Wpn-Cannon5, R-Struc-Research-Upgrade06 | component |
| 10 | R-Wpn-HowitzerMk1 | Howitzer |  | 9640 | 300 | R-Wpn-Mortar-Damage04, R-Sys-Sensor-Upgrade01, R-Struc-Research-Upgrade06 | component |
| 10 | R-Wpn-MG-Damage08 | Depleted Uranium MG Bullets | MG Damage | 9800 | 450 | R-Wpn-MG-Damage07, R-Struc-Research-Upgrade06 | upgrade |
| 10 | R-Wpn-MG5 | Twin Assault Gun |  | 7200 | 225 | R-Struc-Research-Upgrade06, R-Wpn-MG4 | component |
| 10 | R-Wpn-Mortar-Damage06 | HEAP Mortar Shells Mk3 | Mortar Damage | 9400 | 293 | R-Wpn-Mortar-Damage05 | upgrade |
| 10 | R-Wpn-Mortar-ROF03 | Mortar Autoloader Mk3 | Mortar Reload | 6300 | 196 | R-Wpn-Mortar-ROF02 | upgrade |
| 10 | R-Wpn-PlasmaCannon | Plasma Cannon |  | 12000 | 375 | R-Wpn-Cannon4AMk1, R-Wpn-Flame2, R-Struc-Research-Upgrade06 | component |
| 10 | R-Wpn-Rocket-Damage07 | HESH Rocket Warhead | Rocket Damage | 8400 | 262 | R-Wpn-Rocket-Damage06 | upgrade |
| 10 | R-Wpn-Rocket06-IDF | Ripple Rockets |  | 7200 | 225 | R-Wpn-Rocket02-MRL, R-Sys-CBSensor-Turret01 | component |
| 10 | R-Wpn-Rocket07-Tank-Killer | Tank Killer Rocket |  | 6000 | 187 | R-Wpn-RocketSlow-Accuracy01, R-Wpn-Rocket-Damage05 | component |
| 10 | R-Wpn-RocketSlow-Accuracy02 | Thermal Imaging Rockets | Rocket Accuracy | 7200 | 225 | R-Wpn-RocketSlow-Accuracy01, R-Struc-Research-Upgrade04 | upgrade |
| 11 | R-Comp-CommandTurret02 | Command Turret Upgrade |  | 2500 | 78 | R-Comp-CommandTurret01, R-Sys-Sensor-Upgrade02, R-Struc-Research-Upgrade07 | upgrade |
| 11 | R-Cyborg-Armor-Heat04 | Cyborg High Intensity Thermal Armor | Cyborg Thermal | 8000 | 250 | R-Cyborg-Armor-Heat03, R-Cyborg-Metals06 | upgrade |
| 11 | R-Cyborg-Hvywpn-TK | Super Tank-Killer Cyborg |  | 4000 | 125 | R-Cyborg-Metals04, R-Wpn-Rocket07-Tank-Killer | component |
| 11 | R-Cyborg-Metals07 | Cyborg Superdense Composite Alloys | Cyborg Alloys | 11600 | 362 | R-Cyborg-Metals06 | upgrade |
| 11 | R-Defense-AASite-QuadRotMg | Whirlwind AA Site |  | 1300 | 40 | R-Wpn-AAGun04, R-Defense-AASite-QuadMg1 | structure |
| 11 | R-Defense-Cannon6 | Twin Assault Cannon Bunker |  | 1200 | 37 | R-Defense-WallUpgrade04, R-Wpn-Cannon6TwinAslt | structure |
| 11 | R-Defense-Howitzer | Howitzer Emplacement |  | 4800 | 150 | R-Wpn-HowitzerMk1 | structure |
| 11 | R-Defense-HvyA-Trocket | Tank Killer Emplacement |  | 1200 | 37 | R-Wpn-Rocket07-Tank-Killer | structure |
| 11 | R-Defense-IDFRocket | Ripple Rocket Battery |  | 4800 | 150 | R-Wpn-Rocket06-IDF, R-Defense-MRL | structure |
| 11 | R-Defense-PlasmaCannon | Plasma Cannon Emplacement |  | 1300 | 40 | R-Wpn-PlasmaCannon | structure |
| 11 | R-Defense-Super-Cannon | Cannon Fortress |  | 8400 | 262 | R-Wpn-Cannon-Damage05, R-Defense-WallUpgrade05 | structure |
| 11 | R-Defense-WallTower-HvyA-Trocket | Tank Killer Hardpoint |  | 1200 | 37 | R-Wpn-Rocket07-Tank-Killer | structure |
| 11 | R-Defense-WallTower-QuadRotAA | Whirlwind Hardpoint |  | 1300 | 40 | R-Wpn-AAGun04 | structure |
| 11 | R-Defense-WallTower-TwinAGun | Twin Assault Gun Hardpoint |  | 1200 | 37 | R-Wpn-MG5 | structure |
| 11 | R-Defense-WallUpgrade06 | Supercrete Mk3 | Wall | 10000 | 312 | R-Defense-WallUpgrade05 | upgrade |
| 11 | R-Struc-Power-Upgrade02 | Vapor Turbine Generator | Power | 9000 | 281 | R-Struc-Research-Upgrade07, R-Struc-Power-Upgrade01c | upgrade |
| 11 | R-Struc-Research-Upgrade08 | Neural Synapse Research Brain Mk2 | Research | 12000 | 375 | R-Struc-Research-Upgrade07 | upgrade |
| 11 | R-Struc-VTOLPad-Upgrade04 | Robotic VTOL Rearming | VTOL Rearming | 10000 | 312 | R-Struc-VTOLPad-Upgrade03, R-Sys-Engineering02 | upgrade |
| 11 | R-Sys-Sensor-Upgrade03 | Sensor Upgrade Mk3 | Sensor | 7200 | 225 | R-Sys-Sensor-Upgrade02, R-Struc-Research-Upgrade07 | upgrade |
| 11 | R-Sys-Sensor-WS | Wide Spectrum Sensor |  | 14400 | 450 | R-Struc-Research-Upgrade07, R-Sys-VTOLCBS-Turret01 | component |
| 11 | R-Sys-VTOLCBS-Tower01 | VTOL CB Tower |  | 1300 | 40 | R-Sys-VTOLCBS-Turret01, R-Sys-Sensor-Tower02 | structure |
| 11 | R-Vehicle-Armor-Heat04 | High Intensity Thermal Armor | Tank Thermal | 11000 | 343 | R-Vehicle-Armor-Heat03, R-Vehicle-Metals06 | upgrade |
| 11 | R-Vehicle-Metals07 | Superdense Composite Alloys | Tank Alloys | 13600 | 425 | R-Struc-Research-Upgrade07, R-Vehicle-Metals06 | upgrade |
| 11 | R-Wpn-AAGun02 | AA Tornado Flak Cannon |  | 10000 | 312 | R-Wpn-AAGun01, R-Wpn-Cannon3Mk1, R-Struc-Research-Upgrade07 | component |
| 11 | R-Wpn-Cannon-Damage09 | HVAPFSDS Cannon Rounds Mk3 | Cannon Damage | 12000 | 375 | R-Wpn-Cannon-Damage08 | upgrade |
| 11 | R-Wpn-Cannon-ROF05 | Cannon Rapid Loader Mk2 | Cannon Reload | 12000 | 375 | R-Wpn-Cannon-ROF04 | upgrade |
| 11 | R-Wpn-Howitzer-Accuracy01 | Target Acquisition Artillery Shells | Howitzers Accuracy | 7200 | 225 | R-Wpn-HowitzerMk1 | upgrade |
| 11 | R-Wpn-Howitzer-Damage01 | HE Howitzer Shells | Howitzers Damage | 6000 | 187 | R-Wpn-HowitzerMk1 | upgrade |
| 11 | R-Wpn-Howitzer-Incendiary | Incendiary Howitzer |  | 10000 | 312 | R-Wpn-Mortar-Incendiary, R-Wpn-HowitzerMk1 | component |
| 11 | R-Wpn-Howitzer03-Rot | Rotary Howitzer - Hellstorm |  | 10000 | 312 | R-Wpn-HowitzerMk1, R-Wpn-Mortar3 | component |
| 11 | R-Wpn-Laser01 | Laser - Flashlight |  | 14400 | 450 | R-Struc-Research-Upgrade07, R-Sys-Sensor-Upgrade02 | component |
| 11 | R-Wpn-MG-Damage09 | Depleted Uranium MG Bullets Mk2 | MG Damage | 10200 | 450 | R-Wpn-MG-Damage08, R-Struc-Research-Upgrade07 | upgrade |
| 11 | R-Wpn-Missile2A-T | Scourge Missile |  | 16600 | 450 | R-Wpn-Rocket07-Tank-Killer, R-Wpn-RocketSlow-Accuracy02, R-Struc-Research-Upgrade07 | component |
| 11 | R-Wpn-Mortar-ROF04 | Mortar Fast Loader | Mortar Reload | 6600 | 206 | R-Wpn-Mortar-ROF03, R-Wpn-Cannon-ROF03 | upgrade |
| 11 | R-Wpn-Plasmite-Flamer | Plasmite Flamer |  | 7200 | 225 | R-Wpn-Flame2, R-Struc-Research-Upgrade07 | component |
| 11 | R-Wpn-RailGun01 | Needle Gun |  | 14400 | 450 | R-Wpn-Cannon4AMk1, R-Struc-Research-Upgrade07, R-Wpn-Cannon-Damage07, R-Wpn-Cannon-Accuracy02 | component |
| 11 | R-Wpn-Rocket-Damage08 | HESH Rocket Warhead Mk2 | Rocket Damage | 9600 | 300 | R-Wpn-Rocket-Damage07 | upgrade |
| 12 | R-Cyborg-Armor-Heat05 | Cyborg High Intensity Thermal Armor Mk2 | Cyborg Thermal | 10000 | 312 | R-Cyborg-Armor-Heat04 | upgrade |
| 12 | R-Cyborg-Hvywpn-A-T | Super Scourge Cyborg |  | 5000 | 156 | R-Cyborg-Metals04, R-Wpn-Missile2A-T | component |
| 12 | R-Cyborg-Metals08 | Cyborg Superdense Composite Alloys Mk2 | Cyborg Alloys | 13600 | 425 | R-Cyborg-Metals07 | upgrade |
| 12 | R-Defense-AASite-QuadBof02 | AA Tornado Flak Cannon Emplacement |  | 1300 | 40 | R-Wpn-AAGun02 | structure |
| 12 | R-Defense-GuardTower-ATMiss | Scourge Missile Tower |  | 1400 | 43 | R-Wpn-Missile2A-T | structure |
| 12 | R-Defense-GuardTower-Rail1 | Needle Gun Tower |  | 1400 | 43 | R-Wpn-RailGun01 | structure |
| 12 | R-Defense-Howitzer-Incendiary | Incendiary Howitzer Emplacement |  | 4800 | 150 | R-Defense-Howitzer, R-Wpn-Howitzer-Incendiary | structure |
| 12 | R-Defense-PlasmiteFlamer | Plasmite Flamer Bunker |  | 1300 | 40 | R-Wpn-Plasmite-Flamer | structure |
| 12 | R-Defense-PrisLas | Flashlight Emplacement |  | 1400 | 43 | R-Wpn-Laser01 | structure |
| 12 | R-Defense-RotHow | Hellstorm Emplacement |  | 5000 | 156 | R-Defense-Howitzer, R-Wpn-Howitzer03-Rot | structure |
| 12 | R-Defense-Super-Rocket | Heavy Rocket Bastion |  | 12000 | 375 | R-Wpn-RocketSlow-Accuracy02, R-Defense-WallUpgrade05, R-Wpn-Rocket-Damage08 | structure |
| 12 | R-Defense-WallTower-A-Tmiss | Scourge Missile Hardpoint |  | 1400 | 43 | R-Wpn-Missile2A-T | structure |
| 12 | R-Defense-WallTower-DoubleAAgun02 | AA Tornado Flak Cannon Hardpoint |  | 1300 | 40 | R-Wpn-AAGun02 | structure |
| 12 | R-Struc-Power-Upgrade03 | Vapor Turbine Generator Mk2 | Power | 12000 | 375 | R-Struc-Power-Upgrade02 | upgrade |
| 12 | R-Struc-Research-Upgrade09 | Neural Synapse Research Brain Mk3 | Research | 14000 | 437 | R-Struc-Research-Upgrade08 | upgrade |
| 12 | R-Struc-VTOLPad-Upgrade05 | Robotic VTOL Rearming Mk2 | VTOL Rearming | 12000 | 375 | R-Struc-VTOLPad-Upgrade04 | upgrade |
| 12 | R-Sys-Autorepair-General | Auto-Repair |  | 14400 | 450 | R-Struc-Research-Upgrade08 | component |
| 12 | R-Sys-Engineering03 | Advanced Engineering | Engineering | 9600 | 300 | R-Struc-Research-Upgrade08, R-Sys-Engineering02 | upgrade |
| 12 | R-Sys-Sensor-WSTower | Wide Spectrum Sensor Tower |  | 1400 | 43 | R-Sys-Sensor-WS, R-Sys-Sensor-Tower02 | structure |
| 12 | R-Sys-SpyTurret | Nexus Link Turret |  | 12000 | 375 | R-Sys-Sensor-Upgrade03, R-Comp-CommandTurret02 | component |
| 12 | R-Vehicle-Armor-Heat05 | High Intensity Thermal Armor Mk2 | Tank Thermal | 13000 | 406 | R-Vehicle-Armor-Heat04 | upgrade |
| 12 | R-Vehicle-Body03 | Light Body - Retaliation |  | 7200 | 225 | R-Vehicle-Metals07, R-Vehicle-Engine05 | component |
| 12 | R-Vehicle-Engine07 | Gas Turbine Engine | Engine | 13000 | 406 | R-Vehicle-Engine06, R-Vehicle-Prop-VTOL, R-Vehicle-Metals07 | upgrade |
| 12 | R-Vehicle-Metals08 | Superdense Composite Alloys Mk2 | Tank Alloys | 15600 | 450 | R-Vehicle-Metals07 | upgrade |
| 12 | R-Wpn-Bomb05 | Plasmite Bomb |  | 20000 | 450 | R-Wpn-Plasmite-Flamer, R-Wpn-Bomb04 | component |
| 12 | R-Wpn-Bomb06 | EMP Missile Launcher |  | 8000 | 250 | R-Comp-CommandTurret02, R-Wpn-Bomb-Damage03 | component |
| 12 | R-Wpn-Cannon-ROF06 | Cannon Rapid Loader Mk3 | Cannon Reload | 14000 | 437 | R-Wpn-Cannon-ROF05 | upgrade |
| 12 | R-Wpn-EMPCannon | EMP Cannon |  | 14400 | 450 | R-Comp-CommandTurret02, R-Struc-Research-Upgrade08 | component |
| 12 | R-Wpn-Energy-Accuracy01 | Improved Laser Focusing | Laser Accuracy | 14400 | 450 | R-Wpn-Laser01 | upgrade |
| 12 | R-Wpn-Energy-Damage01 | Hi-Energy Laser Emitter | Laser Damage | 14400 | 450 | R-Wpn-Laser01 | upgrade |
| 12 | R-Wpn-Flamer-Damage07 | Superhot Plasmite Gel | Flamer Damage | 13200 | 412 | R-Wpn-Flamer-Damage06, R-Wpn-Plasmite-Flamer | upgrade |
| 12 | R-Wpn-Howitzer-Accuracy02 | Target Acquisition Artillery Shells Mk2 | Howitzers Accuracy | 9200 | 287 | R-Wpn-Howitzer-Accuracy01 | upgrade |
| 12 | R-Wpn-Howitzer-Damage02 | HE Howitzer Shells Mk2 | Howitzers Damage | 8000 | 250 | R-Wpn-Howitzer-Damage01 | upgrade |
| 12 | R-Wpn-Laser02 | Pulse Laser |  | 28800 | 450 | R-Wpn-Laser01 | component |
| 12 | R-Wpn-MG-Damage10 | Depleted Uranium MG Bullets Mk3 | MG Damage | 11800 | 450 | R-Wpn-MG-Damage09, R-Struc-Research-Upgrade08 | upgrade |
| 12 | R-Wpn-MdArtMissile | Seraph Missile Array |  | 28800 | 450 | R-Wpn-Missile2A-T, R-Wpn-Rocket02-MRL | component |
| 12 | R-Wpn-Missile-LtSAM | Avenger SAM |  | 14400 | 450 | R-Sys-Sensor-Upgrade02, R-Wpn-Missile2A-T | component |
| 12 | R-Wpn-Missile-ROF01 | Advanced Missile Allocation System | Missile Reload | 14400 | 450 | R-Wpn-Missile2A-T | upgrade |
| 12 | R-Wpn-Rail-Damage01 | Hardened Rail Dart | Rail Damage | 14400 | 450 | R-Wpn-RailGun01 | upgrade |
| 12 | R-Wpn-Rocket-Damage09 | HESH Rocket Warhead Mk3 | Rocket Damage | 10800 | 337 | R-Wpn-Rocket-Damage08 | upgrade |
| 13 | R-Cyborg-Armor-Heat06 | Cyborg High Intensity Thermal Armor Mk3 | Cyborg Thermal | 12000 | 375 | R-Cyborg-Armor-Heat05 | upgrade |
| 13 | R-Cyborg-Hvywpn-PulseLsr | Super Pulse Laser Cyborg |  | 5000 | 156 | R-Cyborg-Metals04, R-Wpn-Laser02 | component |
| 13 | R-Cyborg-Metals09 | Cyborg Superdense Composite Alloys Mk3 | Cyborg Alloys | 15600 | 450 | R-Cyborg-Metals08 | upgrade |
| 13 | R-Defense-EMPCannon | EMP Cannon Hardpoint |  | 1500 | 46 | R-Wpn-EMPCannon | structure |
| 13 | R-Defense-MdArtMissile | Seraph Missile Battery |  | 1500 | 46 | R-Wpn-MdArtMissile | structure |
| 13 | R-Defense-PulseLas | Pulse Laser Tower |  | 1500 | 46 | R-Wpn-Laser02 | structure |
| 13 | R-Defense-SamSite1 | Avenger SAM Site |  | 1500 | 46 | R-Wpn-Missile-LtSAM | structure |
| 13 | R-Defense-WallTower-PulseLas | Pulse Laser Hardpoint |  | 1500 | 46 | R-Wpn-Laser02 | structure |
| 13 | R-Defense-WallTower-SamSite | Avenger Hardpoint |  | 1500 | 46 | R-Wpn-Missile-LtSAM | structure |
| 13 | R-Defense-WallUpgrade07 | Plascrete | Wall | 12000 | 375 | R-Defense-WallUpgrade06, R-Sys-Engineering03 | upgrade |
| 13 | R-Struc-Factory-Upgrade07 | Advanced Manufacturing | Production | 15200 | 450 | R-Struc-Factory-Upgrade04, R-Sys-Engineering03 | upgrade |
| 13 | R-Struc-Power-Upgrade03a | Vapor Turbine Generator Mk3 | Power | 12000 | 375 | R-Struc-Power-Upgrade03 | upgrade |
| 13 | R-Struc-RprFac-Upgrade06 | Advanced Repair Facility |  | 10000 | 312 | R-Struc-RprFac-Upgrade04, R-Sys-Engineering03 | upgrade |
| 13 | R-Struc-VTOLPad-Upgrade06 | Robotic VTOL Rearming Mk3 | VTOL Rearming | 14000 | 437 | R-Struc-VTOLPad-Upgrade05 | upgrade |
| 13 | R-Sys-Resistance-Circuits | Nexus Resistance Circuits |  | 18000 | 450 | R-Sys-SpyTurret, R-Struc-Research-Upgrade09 | upgrade |
| 13 | R-Sys-Sensor-UpLink | Satellite Uplink Center |  | 28800 | 450 | R-Sys-Sensor-WS, R-Sys-Engineering03 | structure |
| 13 | R-Sys-SpyTower | Nexus Link Tower |  | 1400 | 43 | R-Sys-SpyTurret | structure |
| 13 | R-Vehicle-Armor-Heat06 | High Intensity Thermal Armor Mk3 | Tank Thermal | 15000 | 450 | R-Vehicle-Armor-Heat05 | upgrade |
| 13 | R-Vehicle-Body07 | Medium Body - Retribution |  | 14400 | 450 | R-Vehicle-Body03, R-Vehicle-Engine06, R-Vehicle-Metals08 | component |
| 13 | R-Vehicle-Metals09 | Superdense Composite Alloys Mk3 | Tank Alloys | 17600 | 450 | R-Vehicle-Metals08 | upgrade |
| 13 | R-Wpn-Energy-Damage02 | Hi-Energy Laser Emitter Mk2 | Laser Damage | 28800 | 450 | R-Wpn-Energy-Damage01 | upgrade |
| 13 | R-Wpn-Energy-ROF01 | Thermopole Energizer | Laser Reload | 14400 | 450 | R-Wpn-Energy-Damage01 | upgrade |
| 13 | R-Wpn-Flamer-Damage08 | Superhot Plasmite Gel Mk2 | Flamer Damage | 15200 | 450 | R-Wpn-Flamer-Damage07 | upgrade |
| 13 | R-Wpn-Howitzer-Accuracy03 | Target Prediction Artillery Shells | Howitzers Accuracy | 11200 | 350 | R-Wpn-Howitzer-Accuracy02 | upgrade |
| 13 | R-Wpn-Howitzer-Damage03 | HE Howitzer Shells Mk3 | Howitzers Damage | 10000 | 312 | R-Wpn-Howitzer-Damage02 | upgrade |
| 13 | R-Wpn-HvyHowitzer | Heavy Howitzer - Ground Shaker |  | 14400 | 450 | R-Wpn-Howitzer-Damage02, R-Wpn-Mortar02Hvy | component |
| 13 | R-Wpn-HvyLaser | Heavy Laser |  | 36000 | 450 | R-Wpn-Laser02 | component |
| 13 | R-Wpn-Missile-Damage01 | Advanced Missile Warhead | Missile Damage | 14400 | 450 | R-Wpn-Missile-ROF01 | upgrade |
| 13 | R-Wpn-Missile-ROF02 | Advanced Missile Allocation System Mk2 | Missile Reload | 28800 | 450 | R-Wpn-Missile-ROF01 | upgrade |
| 13 | R-Wpn-MortarEMP | EMP Mortar |  | 14400 | 450 | R-Comp-CommandTurret02, R-Sys-Sensor-Upgrade03, R-Struc-Research-Upgrade09 | component |
| 13 | R-Wpn-Rail-Accuracy01 | Rail Target Prediction Computer | Rail Accuracy | 14400 | 450 | R-Wpn-Rail-Damage01 | upgrade |
| 13 | R-Wpn-Rail-Damage02 | Hardened Rail Dart Mk2 | Rail Damage | 28800 | 450 | R-Wpn-Rail-Damage01 | upgrade |
| 14 | R-Cyborg-Armor-Heat07 | Cyborg Superdense Thermal Armor | Cyborg Thermal | 14000 | 437 | R-Cyborg-Armor-Heat06 | upgrade |
| 14 | R-Defense-EMPMortar | EMP Mortar Pit |  | 1500 | 46 | R-Wpn-MortarEMP | structure |
| 14 | R-Defense-HeavyLas | Heavy Laser Emplacement |  | 1500 | 46 | R-Wpn-HvyLaser | structure |
| 14 | R-Defense-HvyHowitzer | Ground Shaker Emplacement |  | 6000 | 187 | R-Defense-Howitzer, R-Wpn-HvyHowitzer | structure |
| 14 | R-Defense-WallUpgrade08 | Plascrete Mk2 | Wall | 14000 | 437 | R-Defense-WallUpgrade07 | upgrade |
| 14 | R-Struc-Factory-Upgrade09 | Self-Replicating Manufacturing | Production | 19200 | 450 | R-Struc-Factory-Upgrade07 | upgrade |
| 14 | R-Struc-Materials03 | Advanced Base Structure Materials | Buildings | 12000 | 375 | R-Defense-WallUpgrade07, R-Struc-Materials02 | upgrade |
| 14 | R-Vehicle-Armor-Heat07 | Vehicle Superdense Thermal Armor | Tank Thermal | 17000 | 450 | R-Vehicle-Armor-Heat06 | upgrade |
| 14 | R-Vehicle-Body10 | Heavy Body - Vengeance |  | 28800 | 450 | R-Vehicle-Body07, R-Vehicle-Metals09, R-Vehicle-Engine07 | component |
| 14 | R-Vehicle-Engine08 | Gas Turbine Engine Mk2 | Engine | 15000 | 450 | R-Vehicle-Body07, R-Vehicle-Engine07 | upgrade |
| 14 | R-Wpn-AALaser | Stormbringer AA Laser |  | 28800 | 450 | R-Wpn-Energy-ROF01 | component |
| 14 | R-Wpn-Energy-Damage03 | Hi-Energy Laser Emitter Mk3 | Laser Damage | 43200 | 450 | R-Wpn-Energy-Damage02 | upgrade |
| 14 | R-Wpn-Energy-ROF02 | Thermopole Energizer Mk2 | Laser Reload | 28800 | 450 | R-Wpn-Energy-ROF01 | upgrade |
| 14 | R-Wpn-Flamer-Damage09 | Superhot Plasmite Gel Mk3 | Flamer Damage | 17200 | 450 | R-Wpn-Flamer-Damage08 | upgrade |
| 14 | R-Wpn-HeavyPlasmaLauncher | Heavy Plasma Launcher |  | 28800 | 450 | R-Wpn-Flamer-Damage08, R-Wpn-PlasmaCannon | component |
| 14 | R-Wpn-Howitzer-Damage04 | HEAP Howitzer Shells | Howitzers Damage | 18800 | 450 | R-Wpn-Howitzer-Damage03, R-Wpn-Cannon-Damage07 | upgrade |
| 14 | R-Wpn-Howitzer-ROF01 | Howitzer Autoloader | Howitzers Reload | 18800 | 450 | R-Wpn-Howitzer-Damage03 | upgrade |
| 14 | R-Wpn-LasSat | Laser Satellite Command Post |  | 56600 | 450 | R-Struc-Research-Upgrade09, R-Sys-Sensor-UpLink | structure |
| 14 | R-Wpn-Missile-Accuracy01 | Target Prediction Missiles | Missile Accuracy | 14400 | 450 | R-Wpn-Missile-Damage01 | upgrade |
| 14 | R-Wpn-Missile-Damage02 | Advanced Missile Warhead Mk2 | Missile Damage | 28800 | 450 | R-Wpn-Missile-Damage01 | upgrade |
| 14 | R-Wpn-Missile-ROF03 | Advanced Missile Allocation System Mk3 | Missile Reload | 43200 | 450 | R-Wpn-Missile-ROF02 | upgrade |
| 14 | R-Wpn-ParticleGun | Particle Gun |  | 43200 | 450 | R-Wpn-HvyLaser | component |
| 14 | R-Wpn-Rail-ROF01 | Rail Gun ROF | Rail Reload | 14400 | 450 | R-Wpn-Rail-Accuracy01 | upgrade |
| 14 | R-Wpn-RailGun02 | Rail Gun |  | 28800 | 450 | R-Wpn-Rail-Damage02 | component |
| 15 | R-Cyborg-Armor-Heat08 | Cyborg Superdense Thermal Armor Mk2 | Cyborg Thermal | 16000 | 450 | R-Cyborg-Armor-Heat07 | upgrade |
| 15 | R-Cyborg-Hvywpn-RailGunner | Super Rail-Gunner |  | 5500 | 171 | R-Cyborg-Metals04, R-Wpn-RailGun02 | component |
| 15 | R-Defense-AA-Laser | Stormbringer Emplacement |  | 1500 | 46 | R-Wpn-AALaser | structure |
| 15 | R-Defense-HeavyPlasmaLauncher | Heavy Plasma Launcher Emplacement |  | 28800 | 450 | R-Wpn-HeavyPlasmaLauncher | structure |
| 15 | R-Defense-ParticleGun | Particle Gun Emplacement |  | 1500 | 46 | R-Wpn-ParticleGun | structure |
| 15 | R-Defense-Rail2 | Rail Gun Emplacement |  | 1500 | 46 | R-Wpn-RailGun02 | structure |
| 15 | R-Defense-WallTower-Rail2 | Rail Gun Hardpoint |  | 1500 | 46 | R-Wpn-RailGun02 | structure |
| 15 | R-Defense-WallUpgrade09 | Plascrete Mk3 | Wall | 16000 | 450 | R-Defense-WallUpgrade08 | upgrade |
| 15 | R-Vehicle-Armor-Heat08 | Vehicle Superdense Thermal Armor Mk2 | Tank Thermal | 19000 | 450 | R-Vehicle-Armor-Heat07 | upgrade |
| 15 | R-Vehicle-Engine09 | Gas Turbine Engine Mk3 | Engine | 17000 | 450 | R-Vehicle-Engine08, R-Vehicle-Body10 | upgrade |
| 15 | R-Wpn-Energy-ROF03 | Thermopole Energizer Mk3 | Laser Reload | 43200 | 450 | R-Wpn-Energy-ROF02 | upgrade |
| 15 | R-Wpn-Howitzer-Damage05 | HEAP Howitzer Shells Mk2 | Howitzers Damage | 28800 | 450 | R-Wpn-Howitzer-Damage04 | upgrade |
| 15 | R-Wpn-Howitzer-ROF02 | Howitzer Autoloader Mk2 | Howitzers Reload | 28800 | 450 | R-Wpn-Howitzer-ROF01 | upgrade |
| 15 | R-Wpn-HvArtMissile | Archangel Missile |  | 28800 | 450 | R-Wpn-MdArtMissile, R-Wpn-Missile-Damage02, R-Wpn-Rocket06-IDF | component |
| 15 | R-Wpn-Missile-Accuracy02 | Search & Destroy Missiles | Missile Accuracy | 28800 | 450 | R-Wpn-Missile-Accuracy01 | upgrade |
| 15 | R-Wpn-Missile-Damage03 | Advanced Missile Warhead Mk3 | Missile Damage | 43200 | 450 | R-Wpn-Missile-Damage02 | upgrade |
| 15 | R-Wpn-Rail-Damage03 | Hardened Rail Dart Mk3 | Rail Damage | 43200 | 450 | R-Wpn-RailGun02 | upgrade |
| 15 | R-Wpn-Rail-ROF02 | Rail Gun ROF Mk2 | Rail Reload | 28800 | 450 | R-Wpn-Rail-ROF01 | upgrade |
| 15 | R-Wpn-RailGun03 | Gauss Cannon |  | 43200 | 450 | R-Wpn-RailGun02 | component |
| 16 | R-Cyborg-Armor-Heat09 | Cyborg Superdense Thermal Armor Mk3 | Cyborg Thermal | 18000 | 450 | R-Cyborg-Armor-Heat08 | upgrade |
| 16 | R-Defense-HvyArtMissile | Archangel Missile Battery |  | 28800 | 450 | R-Wpn-HvArtMissile | structure |
| 16 | R-Defense-Rail3 | Gauss Cannon Emplacement |  | 1500 | 46 | R-Wpn-RailGun03 | structure |
| 16 | R-Defense-WallTower-Rail3 | Gauss Cannon Hardpoint |  | 1500 | 46 | R-Wpn-RailGun03 | structure |
| 16 | R-Defense-WallUpgrade10 | Plasteel | Wall | 18000 | 450 | R-Defense-WallUpgrade09 | upgrade |
| 16 | R-Vehicle-Armor-Heat09 | Vehicle Superdense Thermal Armor Mk3 | Tank Thermal | 21000 | 450 | R-Vehicle-Armor-Heat08 | upgrade |
| 16 | R-Vehicle-Body13 | Heavy Body - Wyvern |  | 28800 | 450 | R-Vehicle-Engine09, R-Vehicle-Armor-Heat06 | component |
| 16 | R-Wpn-Howitzer-Damage06 | HEAP Howitzer Shells Mk3 | Howitzers Damage | 43200 | 450 | R-Wpn-Howitzer-Damage05 | upgrade |
| 16 | R-Wpn-Howitzer-ROF03 | Howitzer Autoloader Mk3 | Howitzers Reload | 43200 | 450 | R-Wpn-Howitzer-ROF02 | upgrade |
| 16 | R-Wpn-Missile-HvSAM | Vindicator SAM |  | 28800 | 450 | R-Wpn-Missile-Damage03, R-Wpn-Missile-LtSAM | component |
| 16 | R-Wpn-Rail-ROF03 | Rail Gun ROF Mk3 | Rail Reload | 43200 | 450 | R-Wpn-Rail-ROF02 | upgrade |
| 17 | R-Defense-MassDriver | Mass Driver Fortress |  | 50000 | 450 | R-Wpn-RailGun03, R-Wpn-Rail-ROF03, R-Defense-WallUpgrade10 | structure |
| 17 | R-Defense-SamSite2 | Vindicator SAM Site |  | 1500 | 46 | R-Wpn-Missile-HvSAM | structure |
| 17 | R-Defense-Super-Missile | Missile Fortress |  | 50000 | 450 | R-Defense-WallUpgrade10, R-Wpn-Missile-ROF03 | structure |
| 17 | R-Defense-WallTower-SamHvy | Vindicator Hardpoint |  | 1500 | 46 | R-Wpn-Missile-HvSAM | structure |
| 17 | R-Defense-WallUpgrade11 | Plasteel Mk2 | Wall | 22000 | 450 | R-Defense-WallUpgrade10 | upgrade |
| 17 | R-Vehicle-Body14 | Multi Turret Body - Dragon |  | 43200 | 450 | R-Vehicle-Body13 | component |
| 17 | R-Wpn-Howitzer-ROF04 | Howitzer Fast Loader | Howitzers Reload | 56600 | 450 | R-Wpn-Howitzer-ROF03 | upgrade |
| 18 | R-Defense-WallUpgrade12 | Plasteel Mk3 | Wall | 24000 | 450 | R-Defense-WallUpgrade11 | upgrade |
