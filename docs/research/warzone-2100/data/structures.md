# Key structures

<!-- GENERATED FILE: do not edit by hand. Regenerate with tools/research/generate_tables.py -->
> **Provenance:** derived from the Warzone 2100 game data (`mp` stats), snapshot commit `d7ce18df8d`
> (2026-10-07). Warzone 2100 data is GPL-2.0-or-later. These tables are **research notes for reference only**; do not
> paste them into our own game data. Generator: `python3 -I tools/research/wz_components.py <wz_checkout> structures`.

| Type | Name | Power cost | Build pts | HP | Production pts | Module prod pts | Research pts | Module research pts | Power pts | Module power pts | Rearm pts | Repair pts | User limits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| COMMAND RELAY | Command Relay Center | 100 | 500 | 1000 |  |  |  |  |  |  |  |  | [0, 1, 1] |
| CYBORG FACTORY | Cyborg Factory | 100 | 500 | 1000 | 10 | 10 |  |  |  |  |  |  | [0, 5, 5] |
| FACTORY | Factory | 100 | 500 | 1000 | 10 | 10 |  |  |  |  |  |  | [0, 5, 5] |
| FACTORY MODULE | Factory Module | 100 | 500 | 500 |  |  |  |  |  |  |  |  |  |
| HQ | *CommandCenterNE* | 100 | 500 | 1000 |  |  |  |  |  |  |  |  |  |
| HQ | Collective Command Center | 100 | 500 | 1000 |  |  |  |  |  |  |  |  |  |
| HQ | Command Center | 100 | 500 | 1000 |  |  |  |  |  |  |  |  |  |
| HQ | New Paradigm Command Center | 100 | 500 | 1000 |  |  |  |  |  |  |  |  |  |
| POWER GENERATOR | Power Generator | 50 | 500 | 1000 |  |  |  |  | 55 | 28 |  |  | [0, 10, 10] |
| POWER MODULE | Power Module |  | 500 | 1000 |  |  |  |  |  |  |  |  |  |
| REARM PAD | VTOL Rearming Pad | 100 | 300 | 300 |  |  |  |  |  |  | 100 | 10 | [0, 50, 99] |
| REPAIR FACILITY | Repair Facility | 100 | 500 | 1000 |  |  |  |  |  |  |  | 50 | [0, 5, 5] |
| RESEARCH | Research Facility | 100 | 500 | 800 |  |  | 14 | 7 |  |  |  |  | [0, 5, 20] |
| RESEARCH MODULE | Research Module | 100 | 500 | 800 |  |  | 12 |  |  |  |  |  |  |
| RESOURCE EXTRACTOR | Oil Derrick | 0 | 100 | 600 |  |  |  |  |  |  |  |  |  |
| SAT UPLINK | Satellite Uplink Center | 1000 | 1250 | 1600 |  |  |  |  |  |  |  |  | [0, 1, 1] |
| VTOL FACTORY | VTOL Factory | 100 | 500 | 500 | 10 | 10 |  |  |  |  |  |  | [0, 5, 5] |
