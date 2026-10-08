# Documentation Index

Project: **warzone-5000**, a design-first async auto-battler that grew out of Warzone 2100's unit-design ideas. Players draft
parametric components, tune their sliders, test designs against bots, then send them into hands-off battles against other
players' armies. See the [vision](design/00-vision.md).

## Directory layout

| Path | Purpose |
|---|---|
| `docs/research/` | Findings about *existing* games, tech and theory. Facts about the outside world, with sources. |
| `docs/research/warzone-2100/` | Research dossier on Warzone 2100 (the reference game): overview, history, mechanics, deep dives, data. |
| `docs/design/` | *Our* game's design: mechanics, units, factions, UI. Decisions we own. |
| `docs/planning/` | Roadmap, milestones, task breakdowns, tech-stack decisions. |
| `docs/notes/` | Theory notes for each milestone, scratch notes, ideas not yet promoted to design. |
| `docs/assets/` | Figures and diagrams used by the docs (generated figures live here). |
| `tools/` | Scripts that produce the tables and figures (see [tools/README.md](../tools/README.md)). |

### Convention: research vs. design
Research documents record what *is* (Warzone 2100 as it exists). Design documents record what *we will do*. Keeping them apart means we
can always tell "this is how the original works" from "this is our choice".

### Convention: GPL hygiene
Warzone 2100's code and data are GPL-2.0-or-later. We **study** them (read-only checkout kept *outside* this repository) but do not copy
code or assets. Tables in `docs/research/warzone-2100/data/` are derived reference notes; do not paste them into our own game data.

## Our game: design and planning
| Document | Contents |
|---|---|
| [00 Vision](design/00-vision.md) | what the game must be: pillars and cut list |
| [01 Technical architecture](design/01-technical-architecture.md) | Godot presentation over a deterministic Rust core |
| [02 Determinism rules](design/02-determinism-rules.md) | fixed point, seeded randomness, ordered iteration |
| [03 Part Forge](design/03-part-forge.md) | authoring parts, how mass and armour are measured, vehicles, tools |
| [04 Parametric components](design/04-parametric-components.md) | slider families, the coupling solver, gun physics |
| [05 Game loop](design/05-game-loop.md) | draft, design, deploy, battle, shop; Lanchester and terramechanics |
| [06 Art direction](design/06-art-direction.md) | colour theory, faction palettes, stylised light, the shared design language |
| [07 Possibility space](design/07-possibility-space.md) | the whole catalogue mixed and sampled: atlas, fun mixes, viability, the trade triangle, corners |
| [08 Time trials](design/08-time-trial.md) | one vehicle on a hill-and-valley course: real physics, soft earth that sinks and bogs, wheels that roll and legs that walk, every outcome explained |
| [Roadmap](planning/roadmap.md) | milestones and status |
| [Decision log](planning/decisions.md) | D1 to D11 |
| [Development environment](planning/dev-environment.md) | Rust, Godot, Blender, cloud sessions |
| [Theory note: Part Forge](notes/m1-part-forge-theory.md) | half-spaces, sloped armour, sampling, composition, power balance, hover, scaling |
| [Theory note: movement, weapons and mixing](notes/m1b-movement-and-mixing-theory.md) | square-cube law, buckling, rotors, cushions, rockets, beams, the horizon, auto-fit, stratified sampling |
| [Theory note: the physics of a time trial](notes/m1c-the-physics-of-a-time-trial.md) | the felt slope, power and the lowest gear, grip, coherent units, soil as a spring and a wedge, why tracks float, legs, replays as the interface |

Renders: [atlas: every hull with every gear](assets/forge/atlas.png), [fun mixes](assets/forge/showcase.png),
[possibility space](design/07-possibility-space.md), [overview](assets/forge/overview.png), [medium tank](assets/forge/tank_medium.png),
[spider walker](assets/forge/walker_spider.png), [scout drone](assets/forge/drone_scout.png), [6x6 APC](assets/forge/apc_6x6.png),
[turret sweep](assets/forge/turret_gun_sweep.png), [slider coupling](assets/forge/turret_gun_coupling.png),
[army lineup](assets/forge/lineup_vanguard.png), [factions](assets/forge/factions_bastion_twin.png),
[hull variants](assets/forge/hull_variants.png). Time trials: [results](assets/trial/results.png), [speed traces](assets/trial/traces.png),
[soil ladders](assets/trial/soil_ladders_lancer_mk1.png), [a bog](assets/trial/bog_wheel_scout.png), [a gait](assets/trial/gait.png).

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
