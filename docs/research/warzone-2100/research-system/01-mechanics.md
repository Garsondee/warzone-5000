# Research Mechanics

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`. **Evidence:** engine source read by an analysis agent (cited as
`path:line`, relative to the Warzone 2100 checkout), plus the stat files; the key lab-rate and damage code was spot-checked by me.
Statistical shape of the tree is in [03-tree-analysis.md](03-tree-analysis.md); how upgrades are applied is in
[02-upgrade-engine.md](02-upgrade-engine.md).

## 1. What a research topic is

A topic is a record in `research.json` (390 in multiplayer). The engine's index for a topic is its **position in ASCII-sorted
id order**, not file order (`research.cpp:458-501`). Each player has per-topic state: points accumulated, status bits
(started, cancelled, researched), and a tri-state `possible` flag.

| Field | Meaning |
|---|---|
| `requiredResearch` | **AND** of completed topics. There is no OR. Unknown ids are logged and dropped; a cycle fails the load. An empty list means "never auto-available" (only via script/`possible`). |
| `requiredStructures` | AND of *built* structures of that exact type; checked **only when a topic is picked**, never again. Used by one multiplayer topic (Tower 01 needs the HQ). |
| `researchPoints` | points needed (a 16-bit field, max 65,535). 0 completes instantly and unpaid. |
| `researchPower` | power paid **once, at the start** (when the lab first adds points). |
| `disabledWhen` | bitmask (1 tanks, 2 cyborgs, 4 VTOL, 8 uplink, 16 LasSat, 32 forced limits). When the lobby applies the matching limits, the topic **and all its dependants** are disabled for everyone. 14 multiplayer topics. |
| `keyTopic` | campaign only (137 topics): never available through prerequisites; a script must enable it. |
| `techCode` | major/minor flag; only chooses the intel message and sound. |
| `category` | multiplayer only (200 topics); the loader requires each category to be one linear prerequisite chain, else discards it. UI numbering only. |
| `statID`, icons, `msgName`, `name` | cosmetic/UI (icon names must be one of 20 hard-coded ids). |
| `resultComponents` | components that become AVAILABLE (one global id namespace across all 8 component types). |
| `resultStructures` | structures that become AVAILABLE. |
| `redComponents`, `redStructures` | set REDUNDANT (obsolete). |
| `replacedComponents` | `old:new` pairs: on completion the old part is swapped in every existing droid, template, queued production and structure weapon (used for Auto-Repair). |
| `results` | the upgrade list (see [02-upgrade-engine.md](02-upgrade-engine.md)). |

## 2. Is a topic available? (decision order)

1. Cancelled earlier: available again (progress kept).
2. Disabled: no.
3. Marked `possible` and not done/started: **yes, skipping prerequisites and structures**.
4. Campaign `keyTopic`: no.
5. Done, or already started (unless its lab is still being built): no.
6. Otherwise yes iff it has prerequisites, **all** are completed, and all required structures are built.

What the player sees: the classic lab menu lists only currently available topics. A new full-screen **research-tree UI**
(in this snapshot) shows a laid-out graph with six node states, inferred "progression tracks", a "focus" filter (remaining cost and
critical path), a lab-assignment helper and generated effect text. A post-game research timeline viewer also exists. Campaigns
may hide or grey undiscovered topics.

**Component and structure availability** is a per-player state (AVAILABLE, UNAVAILABLE, FOUND, REDUNDANT and variants).
On completion the engine, in order: marks the topic done; makes structures available; applies component replacement; makes
components available (a sensor/ECM/repair marked as default becomes the player's default); sets redundancy; sends the message;
frees the lab; **applies the upgrade results**; fires `eventResearched`. Redundant is an order-independent flag. "FOUND" gates
nothing in the engine (only the guide screen reads it), although the docs imply otherwise.

## 3. How a lab advances a topic

- Each tick, a **built** lab adds points. The rate is `facility points + module points x modules`, in points per second, reading
  the live per-player upgrade, so research upgrades speed up existing labs immediately.
- **Numbers (multiplayer):** facility 14 points/s; one module adds 7; so **21/s** per equipped lab. The module structure's own
  `researchPoints: 12` is never read. A lab costs 100 power, 500 build points and has 800 HP (400 in the campaign). Per-player limit
  20 in data, **5 in skirmish rules**.
- **Nine research-speed upgrades** (+30% of the *base* 14 each, rounded up to +5 each) take a lab to 59/s; the module's +7 is
  not upgradable by the shipped data, so the maximum is **66/s per lab**.

Typical times (points / rate):

| Topic (points) | no module (14/s) | module (21/s) | module + 9 upgrades (66/s) |
|---|---:|---:|---:|
| Fuel Injection Engine 1 (1,200) | 86 s | 57 s | 18 s |
| Turbo-Charged Engine 2 (9,000) | 643 s | 429 s | 136 s |
| Gas Turbine Engine 3 (17,000) | 1,214 s | 810 s | 258 s |
| A 43,200-point late topic | 51 min | 34 min | 11 min |

### Pacing in context (derived)

- The whole tree is **3.59 million points**. Five fully upgraded labs (330 points/s) would need about **3 hours** of
  perfectly parallel research; five basic equipped labs (105/s) about 9.5 hours. In one hour, five fully upgraded labs complete about
  **1.2 million points, one third of the tree**. So a game never completes the tree: the choice of *where to spend the first million
  points* is the real research decision.
- Chains are **strictly sequential**: extra labs cannot shorten a chain. The nine engine topics (80,400 points) take about
  64 minutes at 21/s or 20 minutes at 66/s in one lab.
- **Power** is the second budget: the tree costs 85,414 power in total; a derrick yields roughly 0.55 power/s (0.83 with the power
  module) at a 100% power modifier, 4 derricks per generator.

## 4. Paying, holding, cancelling

- **Power** is charged once, when the topic has 0 points and its lab first can add some. It goes through a **per-player FIFO queue**:
  a request succeeds only if all earlier queued requests plus its own fit in current power. A lab waiting for power makes no
  progress.
- **Hold** freezes progress. It is automatic while a module is being added, and when leaving for an off-world mission. In
  multiplayer a lab whose electronic-warfare resistance is below max does nothing.
- **Cancel:** if the topic has points, it becomes CANCELLED and **keeps its points** (no refund, and no re-payment on resume). If it has
  zero points it is simply reset. A destroyed lab behaves the same way.
- **No queue:** a lab has exactly one current topic. Picking another cancels the current one (progress kept). Only a JS helper
  (`pursueResearch`) walks a path automatically.
- **The same topic cannot run in two labs** for one player (progress is per player-topic; a start request cancels the other lab). In
  alliances with shared research, one payment covers the team, progress is the maximum across allies, and results go to every ally.
- **Losing a required structure** never undoes research or stops a running topic.

## 5. Hidden and obscure behaviours (from the analysis)

- The research module's +7 is never upgraded, and compat rounding gives about 13% more lab speed than the data's percentages suggest.
- The engine-upgrade bonus threshold (see [../unit-design/04-engine-and-speed.md](../unit-design/04-engine-and-speed.md)) ignores research.
- Redundant items are hidden unless a toggle is on; power is paid at start and **cancelling gives no refund**.
- In multiplayer, stealing tech: destroying an enemy factory drops an **artifact**; picking it up scans topics from the highest
  index (ASCII-last id) down and grants the first the victim finished and the picker lacks, favouring ASCII-last ids. Electronic
  takeover of a lab instantly completes its most expensive finished topic.

## 6. Findings for our design work

1. **A topic costs two budgets** (time via lab points, and power at start), and the power budget serialises through a FIFO queue.
   That interplay is a quiet source of strategy.
2. **Research is one lab, one topic, one chain**, and chains are long and sequential. The number of labs is limited (5 in skirmish),
   so choice is about *which chain to push*, not breadth.
3. **Cancel-keeps-progress** makes switching free of waste; that is player-friendly but removes commitment.
4. The research-tree UI work in this snapshot (progression tracks, critical path, focus filter) shows that **discoverability** of the
   tree has been a long-standing weakness (consistent with forum comments that you need to know prerequisite chains in advance).
5. A richer system could add: OR/exclusive branches, queues, partial-refund cancels, lab specialisation (labs with different
   strengths per research domain), and research that costs materials as well as time.
