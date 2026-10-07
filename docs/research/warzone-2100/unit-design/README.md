# Unit Design System (deep dive)

How Warzone 2100 lets players build units, and what the numbers underneath really do. Built from the game's stat data, its engine
source (read-only, never copied) and community material. **Snapshot:** `master` @ `d7ce18df8d` (2026-10-07).

## How evidence was gathered (and how far to trust it)

| Evidence | Confidence | How |
|---|---|---|
| Stat files (bodies, propulsion, weapons, research...) | **High** | parsed by scripts in `tools/research`; tables in [`../data/`](../data/) |
| Engine formulas (speed, weight, HP, damage, armour, lab rate) | **High** | read in the source by analysis agents; the key formulas were re-read by me line by line |
| Other engine behaviour (targeting, visibility, VTOL rules) | Medium-high | agent reports with line citations; not all re-verified |
| Community material (mods, forums, reviews) | Low-medium | search-result excerpts only; the sites were blocked |

## A unit in one paragraph
A design is a **body** (HP, armour, *engine power*, weight, weapon slots) plus a **propulsion** (wheels, half-tracks, tracks, hover, VTOL;
its weight, cost and HP are *percentages of the body's*) plus a **turret** (1-3 weapons, or one sensor, repair, truck or command unit).
Any body takes any propulsion and any ground weapon. Speed is `multiplier x engine power / total weight`, capped per propulsion. Damage is
`base x (weapon effect vs target propulsion) - flat armour`, with a 33% floor. Research changes numbers through a generic
percent-of-base upgrade engine, and gates new bodies behind the engine and armour lines.

## Key numbers

| Quantity | Value |
|---|---|
| Designable bodies / propulsions / weapons | 14 / 5 usable (6 defined) / 79 (52 ground, 27 VTOL) |
| Nominal valid designs | about 9,700 |
| Body engine power | 5,000 (Light) to 30,000 (Dragon); research +50% over 9 steps |
| Ground designs whose speed is pinned at the cap | 44% (no research), 62% (all engine research), 72% on roads |
| Weapon weight versus body weight spread | 150x versus 10x |
| Strong or weak cells in the weapon-effect by propulsion matrix | 26 of 42 |
| Weapons pinned to the 33% damage floor by a tracked Vengeance | 17 of 41 |
| Rank-8 unit | takes 0.52x damage, +/-40% accuracy |

## Ten findings
1. There is **no engine component**; "engine" is a body number plus a +50% research line ([04](04-engine-and-speed.md)).
2. Propulsion weight, cost and HP are **percentages of the hull**; weapons are fixed ([01](01-design-pipeline.md)).
3. **Any body takes any propulsion**; the validator checks almost nothing about fit ([01](01-design-pipeline.md)).
4. The **speed cap makes the engine invisible** for roughly half the designs; a hidden x1.5 weight cliff exists ([04](04-engine-and-speed.md)).
5. Wheels, half-tracks and tracks are **one machine with different numbers**; pathfinding ignores terrain speed ([03](03-propulsion.md)).
6. The body list is a **quality ladder**; only the three heavy bodies are cost-efficient on every propulsion ([02](02-bodies.md)).
7. **Propulsion doubles as the armour class** for the damage matrix, the source of most counter-play ([06](06-damage-and-counters.md)).
8. **Flat armour outruns small weapons' upgrades**, creating permanent cannon fodder ([06](06-damage-and-counters.md)).
9. Utility turrets (sensors, repair, command, ECM) are **thin**; a droid has one turret role ([05](05-weapons-and-turrets.md)).
10. Propulsion, component types, damage effects and terrain are **closed enumerations**; engines or new propulsion need engine changes ([07](07-limits-and-extensibility.md)).

## Documents

| # | Document | Contents |
|---|---|---|
| 01 | [Design pipeline](01-design-pipeline.md) | anatomy of a design, validity rules, availability, derived-stat formulas, design screen, closed subsystems |
| 02 | [Bodies](02-bodies.md) | the 14 chassis, engine power by class, ladder analysis, tech gating, campaign vs multiplayer |
| 03 | [Propulsion](03-propulsion.md) | the 7 types, stats, terrain table, special cases in code, campaign tiers, community notes |
| 04 | [Engine and speed](04-engine-and-speed.md) | the speed/engine model, worked examples, saturation analysis, engine research, quirks |
| 05 | [Weapons and turrets](05-weapons-and-turrets.md) | weapon axes, sub-class x effect matrix, weight and range, non-weapon turrets |
| 06 | [Damage and counters](06-damage-and-counters.md) | damage pipeline, matrices, armour and floor, upgrade race, accuracy, sensors, experience |
| 07 | [Limits and extensibility](07-limits-and-extensibility.md) | what is data versus code, effort to extend, latent hooks, principles |

Related: [research system](../research-system/README.md), [community precedents](../community-precedents.md),
[expansion analysis](../expansion-analysis.md), [generated data tables](../data/).
