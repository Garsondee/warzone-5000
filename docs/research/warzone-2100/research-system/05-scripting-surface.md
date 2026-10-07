# The Scripting Surface for Research, Components and Design

What the engine lets JavaScript do in these areas. It shows how extensible the design and research layer already is, and where the
walls are. **Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`; function lists from the project's `doc/js-*.md`, checked against the
engine source by an analysis agent.

## 1. Research

| Function | What it does |
|---|---|
| `getResearch(name[, player])` | returns a research object (power, points, started, done, name, id, type, results). The code sets `name` to the **id** and adds an undocumented `fullname`, contrary to the docs. |
| `enumResearch()` | topics available right now |
| `findResearch(name[, player])` | the unfinished chain to a target |
| `pursueResearch(lab, name or [names])` | start the first available step toward a target (the only automatic path-walker) |
| `enableResearch(name)` | lift a topic from impossible to *possible* (bypasses prerequisites) |
| `completeResearch(name[, player[, force]])` | run the topic's results immediately (force re-applies) |
| `completeAllResearch()` | everything |
| `getMultiTechLevel()` | the lobby tech level (1-4) |

Event: `eventResearched(research, structure, player)`.

## 2. Structures and components

`enableStructure`, `isStructureAvailable` (research plus limits), `enableComponent` (sets FOUND only, gates nothing),
`makeComponentAvailable`, `componentAvailable`, `setStructureLimits`, `applyLimitSet`, `getStructureLimit`.

## 3. Templates and design

`enumTemplates`, `makeTemplate` (returns null if the research is missing), `buildDroid`, `addDroid` (desyncs in multiplayer),
`getDroidProduction`, `setDesign`, `enableTemplate`, `removeTemplate`, droid-limit and experience getters/setters,
`propulsionCanReach` (can this propulsion get from A to B), `getWeaponInfo` (deprecated).

Events for the design screen: `eventDesignCreated` and `eventDesignBody/Propulsion/Weapon/Command/System/Quit` (UI navigation),
`eventMenuResearch`, `eventMenuResearchSelected`.

## 4. Data access

- **`Stats`** is the *read-only base* data: Body, Sensor, ECM, Propulsion, Repair, Construct, Brain, Weapon, WeaponClass, Building,
  Research.
- **`Upgrades`** is the per-player *writable* closed set (see [02-upgrade-engine.md](02-upgrade-engine.md)). Direct writes are not
  restored on load.
- Others: `playerData` (faction, difficulty), `tweakOptions`, `baseType`, `alliancesType`, `powerType`, `scavengers`.

## 5. Campaign library

`camSetArtifacts`, `camAddArtifact`, `camDeleteArtifact`, `camAllArtifactsPickedUp`, `camGetArtifacts`, `camEnableRes`,
`camCompleteRequiredResearch`, `camClassicResearch`, `camUpgradeOnMapTemplates`, `camQueueDroidProduction`, `camGetRankThreshold`.

## 6. What is missing

- **No runtime creation or alteration of topics, components or schema.** Data is loaded once.
- **No "disable" or lock** for research and **no exclusive choice**; no lab-rate query.
- The shipped `eventResearched` handlers are **empty**: upgrades moved into the engine (C++) in version 4.1.

## 7. Takeaways

1. **Scriptable design and research is a strong precedent**: bots, campaigns and tech rules all run through the same public API.
2. The API shows the *types of control* worth designing in from the start: query the tree, grant/lock topics, read and write
   per-player stats, create templates and validate them, and receive events on research, pickups and design actions.
3. The gaps (no runtime schema, no exclusive choice, no lock) are exactly where a richer research system would need new primitives.
