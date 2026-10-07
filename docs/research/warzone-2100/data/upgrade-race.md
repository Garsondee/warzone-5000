# Weapon-damage research versus armour research

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generated with: `wz_combat.py <wz_checkout> race`

Per-hit damage as BOTH sides take the same number of research steps (weapon-damage line +25% of base per step; kinetic armour line +30% of base armour per step, compat rounding).

| Weapon vs target | Step 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | Weapon dmg step | Armour step |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Machinegun (10) vs Cobra / Tracks (arm 15) | 1* | 1* | 1* | 2* | 2* | 3* | 3* | 3* | 4* | 4* | +3 | +5 |
| Heavy Machinegun (17) vs Cobra / Tracks (arm 15) | 1* | 2* | 3* | 3* | 4* | 5* | 5* | 6* | 7* | 7* | +5 | +5 |
| Assault Gun (19) vs Python / Tracks (arm 20) | 2* | 2* | 3* | 4* | 4* | 5* | 6* | 6* | 7* | 8* | +5 | +6 |
| Light Cannon (35) vs Cobra / Tracks (arm 15) | 21 | 26 | 30 | 35 | 39 | 44 | 48 | 52 | 57 | 61 | +9 | +5 |
| Heavy Cannon (120) vs Python / Tracks (arm 20) | 106 | 131 | 157 | 182 | 208 | 233 | 259 | 284 | 310 | 335 | +30 | +6 |
| Lancer (105) vs Cobra / Tracks (arm 15) | 111 | 138 | 165 | 193 | 220 | 248 | 275 | 302 | 330 | 357 | +27 | +5 |
| Tank Killer (180) vs Python / Tracks (arm 20) | 196 | 244 | 292 | 340 | 388 | 436 | 484 | 532 | 580 | 628 | +45 | +6 |
| Mortar (60) vs Cobra / Tracks (arm 15) | 5* | 7* | 8* | 10* | 11* | 13* | 14* | 16* | 17* | 19* | +15 | +5 |

`*` = pinned to the minimum-damage floor at that step.

