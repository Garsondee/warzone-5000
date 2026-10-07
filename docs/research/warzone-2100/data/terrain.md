# Terrain speed factors

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generated with: `wz_components.py <wz_checkout> terrain`

### Speed factor by propulsion type and terrain (percent)

| Terrain | wheeled | tracked | legged | hover | lift | propellor | half-tracked |
|---|---|---|---|---|---|---|---|
| Sand | 100 | 100 | 100 | 150 | 250 | 100 | 100 |
| Sandy Brush | 100 | 100 | 100 | 80 | 250 | 100 | 100 |
| Baked Earth | 80 | 90 | 100 | 100 | 250 | 100 | 80 |
| Green Mud | 80 | 100 | 100 | 150 | 250 | 100 | 100 |
| Red Brush | 100 | 100 | 100 | 80 | 250 | 100 | 100 |
| Pink Rock | 80 | 100 | 100 | 50 | 250 | 100 | 90 |
| Road | 150 | 120 | 100 | 150 | 250 | 100 | 135 |
| Water | 60 | 60 | 60 | 150 | 250 | 100 | 60 |
| Cliff Face | 60 | 60 | 60 | 80 | 250 | 100 | 60 |
| Rubble | 80 | 80 | 100 | 80 | 250 | 100 | 50 |
| Sheet Ice | 70 | 90 | 100 | 150 | 250 | 100 | 100 |
| Slush | 60 | 100 | 75 | 80 | 250 | 100 | 80 |

Average across terrain types:
| Type | Mean | Min | Max |
|---|---|---|---|
| wheeled | 85 | 60 | 150 |
| tracked | 92 | 60 | 120 |
| legged | 91 | 60 | 100 |
| hover | 108 | 50 | 150 |
| lift | 250 | 250 | 250 |
| propellor | 100 | 100 | 100 |
| half-tracked | 88 | 50 | 135 |

