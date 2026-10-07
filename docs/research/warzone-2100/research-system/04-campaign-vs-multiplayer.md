# Campaign Versus Multiplayer Research

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`. **Evidence:** the two research data sets (`data/base/stats` and `data/mp/stats`),
the engine source, and the bundled scripts, as read by an analysis agent (paths relative to the Warzone 2100 checkout).

## 1. Two different data sets for two different jobs

Loading the second research file **replaces** the first: the campaign and multiplayer trees are alternative datasets, not layers.

| | Campaign (`base`) | Multiplayer (`mp`) |
|---|---:|---:|
| Research topics | 413 | 390 |
| Topics with no prerequisite | **53** | 6 |
| `keyTopic` (script-enabled only) | **137** | 0 |
| Named categories | 0 | 200 topics |
| `disabledWhen` (tech-limit switches) | 0 | 14 |
| `requiredStructures` | 0 | 1 |
| Longest prerequisite chain | 16 | 19 |
| Topics that are pure upgrades | 62% | 52% |
| Engine upgrade line | nine steps of +5 (+45%) | +5 x7, +7, +8 (+50%) |
| Propulsion progression | Mk I, II, III components | one tier, percentage upgrades |
| Lab HP | 400 | 800 |
| Lab points | 14 + 7 per module (same) | 14 + 7 per module |

**Philosophy.** The campaign is **curated and scripted**: artifacts, hidden key topics, chapter tech resets and scripted AI tech.
Multiplayer is a **pure self-unlocking graph** plus a timeline table for catch-up. Bodies also have different stat values between
the two (see [../unit-design/02-bodies.md](../unit-design/02-bodies.md)).

## 2. The campaign: artifacts and scripts

- The engine only **detects the pickup** of an artifact and fires an event. Everything else is scripted: the campaign library places an
  artifact, and on pickup calls "enable research" for a topic. **The player must still research the opened topic** (it costs points
  and power like any other).
- Helper functions exist for the campaign: enable plus complete, "complete the whole required chain" (the library finds the chain
  to a target), and per-chapter baseline resets from scripted lists (so a new chapter starts with a defined tech set).
- **AIs never research in the campaign**; scripts grant their tech.
- Research doubles as a **script lever**: hidden key-topic pairs apply and undo a nerf (for example a -12/+12 flame-range pair used to
  weaken scavengers), an AI production boost, and a "tower wars" flag.

## 3. Multiplayer setup

- Skirmish rules load from a separate rules script set (the old single `rules.js` is gone for skirmish; the campaign still uses
  one). On game init, per player: **three root topics are enabled**; Wheels, the Truck and the Viper body are completed at time zero by the
  base setup, which gives the six zero-prerequisite topics.
- **Base level** completes every topic whose timestamp is at most 1 s (clean start), 180 s (base) or 384 s (advanced base).
- **Tech level** (host option, 1-4): T2 completes everything up to 1,020 s, T3 up to 1,560 s, T4 grants **all** research.
- The timestamp table has one entry per topic (390 entries, from 0 to 4,009 s). It is a **hand-maintained "typical time" table, not
  derived from the tree**.
- **Tech theft:** destroying an enemy factory drops an artifact; picking it up grants one topic the victim finished (the scan runs from
  the ASCII-last id downward). Electronic takeover of a lab completes its most expensive finished topic. Allies can gift all research.
- **Shared research** in alliances: one payment covers the team and results go to every ally.

## 4. Observations

1. **Two trees for two contexts is a sound idea**, but it doubles balance work: the same chassis list has different numbers in each
   (the changelog records repeated re-tuning of both).
2. **The multiplayer "tech level" is a timestamp table.** A tier tag *on each topic* would be simpler and derivable from the tree.
3. **Artifacts as research keys** (campaign) turn research into a *reward for exploration and combat* rather than a purely economic
   spend, and keep the player from out-teching a mission. This is a distinct design idea to keep in mind.
4. **Research is also a general scripting lever** (silent topics that flip numbers or flags). It is flexible but opaque to
   players and modders.
5. **Component generations (Mk I-III) vs percentage upgrades** is a real design fork, visible by comparing the two data sets: the
   campaign unlocks new variants with different trade-offs; multiplayer improves a single variant by percentages.
