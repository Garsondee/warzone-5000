# Research System (deep dive)

How research works in Warzone 2100: the data, the engine mechanics, the upgrade engine, and the shape of the tree. **Snapshot:**
`master` @ `d7ce18df8d` (2026-10-07). Evidence levels are as described in [../unit-design/README.md](../unit-design/README.md).

## In one paragraph
Research is a directed graph of topics (390 in multiplayer, 413 in the campaign) joined by AND-only prerequisites. A lab spends
time (points at 14 to 66 per second) and power (paid once at the start) on one topic at a time. A completed topic can unlock components
or structures, mark old ones obsolete, and apply **percentage-of-base upgrades** to a closed list of stats, which take effect immediately
for every existing unit. Multiplayer has an open tree plus tech-level shortcuts; the campaign gates topics with scripts and artifacts.

## Key numbers

| Quantity | Value |
|---|---|
| Topics (multiplayer / campaign) | 390 / 413 |
| Roots (no prerequisite) | 6 / 53 |
| Longest prerequisite chain | 19 / 16 |
| Pure percentage upgrades | 52% / 62% of topics |
| Corridor topics (one in, one out) | 22% |
| Total research points in the tree | 3.59 million (85,414 power) |
| Lab rate | 14/s, 21/s with module, 66/s with module and nine upgrades |
| Share of the tree finished in 1 hour by 5 fully upgraded labs | about one third |
| Engine research | 9 steps, +50% total, 80,400 points; gates 5 bodies |
| Armour research | 9 steps of +30% armour and +30% HP; gates 9 body edges |
| Exclusive-choice branches | none |

## Findings
1. **More than half of the multiplayer tree is a stat treadmill** with uniform steps; real choice is only order and timing
   ([03](03-tree-analysis.md), [../expansion-analysis.md](../expansion-analysis.md)).
2. **Armour and engine lines gate the chassis** (9 and 5 prerequisite edges): a cheap, effective cross-line coupling ([03](03-tree-analysis.md)).
3. **Whole-tree scarcity is built in**: a game finishes about a third of the tree per hour at best ([01](01-mechanics.md)).
4. The upgrade engine (immutable base, per-player copy, eager write, replay on load) is **clean and replayable**, but its parameter list and filter
   language are closed ([02](02-upgrade-engine.md)).
5. The campaign and multiplayer are **two separate datasets** with different philosophies (artifacts versus open graph) ([04](04-campaign-vs-multiplayer.md)).
6. Scripts can query, grant and complete research, but cannot **create** topics, lock them or define exclusive choices ([05](05-scripting-surface.md)).

## Documents

| # | Document | Contents |
|---|---|---|
| 01 | [Mechanics](01-mechanics.md) | data model, availability logic, lab progress, power payment, hold and cancel, pacing |
| 02 | [Upgrade engine](02-upgrade-engine.md) | storage, arithmetic and rounding, upgradable parameters, the engine upgrade |
| 03 | [Tree analysis](03-tree-analysis.md) | shape, composition, gatekeepers, coupling between lines, upgrade chains, costs, campaign comparison |
| 04 | [Campaign versus multiplayer](04-campaign-vs-multiplayer.md) | artifacts, scripts, tech levels, differences |
| 05 | [Scripting surface](05-scripting-surface.md) | JavaScript functions and events for research, components and design |

Related: [unit design](../unit-design/README.md), [community precedents](../community-precedents.md),
[expansion analysis](../expansion-analysis.md), full tables in [`../data/`](../data/).
