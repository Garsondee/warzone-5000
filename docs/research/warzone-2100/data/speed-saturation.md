# How often the speed cap hides the engine

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generated with: `wz_speed_model.py <wz_checkout> saturation`; `wz_speed_model.py <wz_checkout> engine`

## Cap saturation

Share of designs (every body x propulsion x weapon, single weapon) whose speed cap binds, by terrain context.

| Terrain | Engine research | Class | Designs | Cap binds | Share |
|---|---|---|---:|---:|---:|
| neutral | none | Ground | 2912 | 1279 | 44% |
| neutral | none | VTOL | 378 | 210 | 56% |
| neutral | all 9 | Ground | 2912 | 1794 | 62% |
| neutral | all 9 | VTOL | 378 | 298 | 79% |
| road | none | Ground | 2912 | 1794 | 62% |
| road | none | VTOL | 378 | 210 | 56% |
| road | all 9 | Ground | 2912 | 2090 | 72% |
| road | all 9 | VTOL | 378 | 298 | 79% |
| offroad | none | Ground | 2912 | 1091 | 37% |
| offroad | none | VTOL | 378 | 210 | 56% |
| offroad | all 9 | Ground | 2912 | 1687 | 58% |
| offroad | all 9 | VTOL | 378 | 298 | 79% |

By propulsion (ground weapons, neutral terrain):

| Propulsion | Cap binds, no research | Cap binds, all 9 | Gets x1.5 weight bonus |
|---|---:|---:|---:|
| Half-tracks | 51% | 70% | 70% |
| Hover | 45% | 72% | 81% |
| Tracks | 26% | 28% | 28% |
| Wheels | 53% | 76% | 76% |

## What the engine research line changes

Effect of the full engine research line on ground designs (single weapon):

- designs: 2912
- speed improves with engine research: 1633 (56%); median gain x1.50, max x1.55
- already at the speed cap with no research (engine research changes nothing on flat ground): 1279 (44%)
- other / no change: 0

