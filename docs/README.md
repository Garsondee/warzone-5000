# Documentation Index

Project: **warzone-5000**, an attempt to create a game in the style of Warzone 2100, with the unit-design and research systems made
bigger, bolder and more impactful on gameplay (more component types such as engines, many more propulsion varieties, deeper unit building).

## Directory layout

| Path | Purpose |
|---|---|
| `docs/research/` | Findings about *existing* games, tech and theory. Facts about the outside world, with sources. |
| `docs/research/warzone-2100/` | Research dossier on Warzone 2100 (the reference game): overview, history, mechanics, deep dives, data. |
| `docs/design/` | *Our* game's design: mechanics, units, factions, UI. Decisions we own. (empty for now) |
| `docs/planning/` | Roadmap, milestones, task breakdowns, tech-stack decisions. (empty for now) |
| `docs/notes/` | Scratch notes, session logs, ideas not yet promoted to design. (empty for now) |
| `docs/assets/` | Figures and diagrams used by the docs (generated figures live here). |
| `tools/` | Scripts that produce the tables and figures (see [tools/README.md](../tools/README.md)). |

### Convention: research vs. design
Research documents record what *is* (Warzone 2100 as it exists). Design documents record what *we will do*. Keeping them apart means we
can always tell "this is how the original works" from "this is our choice".

### Convention: GPL hygiene
Warzone 2100's code and data are GPL-2.0-or-later. We **study** them (read-only checkout kept *outside* this repository) but do not copy
code or assets. Tables in `docs/research/warzone-2100/data/` are derived reference notes; do not paste them into our own game data.

## Warzone 2100 research dossier

### First pass: the game overall
| # | Document | Contents |
|---|---|---|
| 00 | [Overview](research/warzone-2100/00-overview.md) | What the game is, feature summary, why it matters as a reference |
| 01 | [History](research/warzone-2100/01-history.md) | Pumpkin Studios, Eidos, open-sourcing, community era |
| 02 | [Gameplay mechanics](research/warzone-2100/02-gameplay-mechanics.md) | Economy, research, design system, combat, artillery, VTOLs, commanders, controls |
| 03 | [Campaign and story](research/warzone-2100/03-campaign-and-story.md) | Plot, factions, campaign structure, mission flow |
| 04 | [Multiplayer and AI](research/warzone-2100/04-multiplayer-and-ai.md) | Skirmish, lobby, ratings, spectators, bundled bots |
| 05 | [Modding, scripting and tech](research/warzone-2100/05-modding-scripting-tech.md) | Engine, JS API, mods, build info |
| 06 | [Version history](research/warzone-2100/06-version-history.md) | 4.0 to 4.7 release timeline |

### Second pass: unit design and research (deep dives)
| Document | Contents |
|---|---|
| [**Expansion analysis**](research/warzone-2100/expansion-analysis.md) | **Start here for design thinking**: engines, propulsion, unit building and research, with theory, openings, pitfalls and metrics |
| [Unit design system](research/warzone-2100/unit-design/README.md) | design pipeline, bodies, propulsion, **engine and speed model**, weapons, damage and counters, limits |
| [Research system](research/warzone-2100/research-system/README.md) | mechanics, upgrade engine, tree analysis, campaign vs multiplayer, scripting surface |
| [Community precedents](research/warzone-2100/community-precedents.md) | what mods (Contingency, Enhanced Balance, Battleplan) and players already tried |
| [Generated data tables](research/warzone-2100/data/) | bodies, propulsion, weapons, modifiers, terrain, research tree, speed and DPS tables, frontier analyses |

### Bookkeeping
| Document | Contents |
|---|---|
| [07 Design takeaways](research/warzone-2100/07-design-takeaways.md) | first-pass takeaways (superseded in depth by the expansion analysis) |
| [08 Gaps and open questions](research/warzone-2100/08-gaps-and-open-questions.md) | what is unverified or conflicting; next research steps |
| [Sources](research/warzone-2100/sources.md) | every source consulted, with reliability notes |

## Figures
- [Speed versus turret weight](assets/wz-speed-vs-turret-weight.svg) (cap, 1/x fall, x1.5 cliff)
- [Share of designs at the speed cap](assets/wz-cap-binding-dumbbell.svg)
- [Research tree shape](assets/wz-research-tree-shape.svg)

## Research status
- **First pass** (2026-10-07): game overview, history, mechanics, campaign, multiplayer, tech, versions.
- **Second pass** (2026-10-07): unit design and research deep dives from the stat data and engine source (snapshot `d7ce18df8d`),
  community precedents, expansion analysis, generated tables and figures.
- Several primary web sources (Wikipedia, wz2100.net, the project wiki and forums) were blocked by the sandbox proxy; see
  [08 Gaps](research/warzone-2100/08-gaps-and-open-questions.md).
