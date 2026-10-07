# Expansion Analysis: Engines, Propulsion, Unit Building and Research

**Purpose.** Bridge from "how Warzone 2100 works" to "where a bigger, bolder system could go". Everything under *What WZ does* and
*Evidence of limits* is backed by the dossier (links given). Everything under *Openings* and *Pitfalls* is **candidate thinking, not
a decision**: it exists to be argued with when we start `docs/design/`.

**The test we should apply to every idea ("the impact test").** Does this choice change *what the player does* (where they
move, what they build, what they shoot, what they research first), or only the numbers on a screen? Warzone 2100's weakest
mechanics (uniform upgrade steps, the speed cap) fail this test; its best (propulsion-versus-weapon counters, sensors and
artillery, gating chassis behind tech lines) pass it.

## 0. One-page summary

| Goal | What WZ2100 does | Evidence it is limited | The opening |
|---|---|---|---|
| **Engines** | One number on the body (`powerOutput`); +50% over a 9-step research line; speed = power / weight with a hard cap | The cap pins **44-62%** of ground designs; engine line is the stingiest (+5%/step); a hidden x1.5 cliff | A real component with several attributes and **no flat plateau** |
| **Propulsion variety** | 7-type closed enum; 3 of the 5 designable types are one ladder (wheels, half-tracks, tracks) | Terrain ignored by pathfinding; ~45 special cases per type; community "hover dominance" and "wheels have no role" | Propulsion as a **data-defined trait set**; terrain that steers routes |
| **Unit-building complexity** | 3 slots (body, propulsion, turret); no mount constraints; weight and cost the only budgets | "Five largely similar tanks"; only 12 of 56 chassis combos are efficient; thin utility-turret classes | **Budgeted slots** (weight, power, volume) with synergies; clearer roles; honest UI |
| **Research impact** | 390 topics; **52% pure percentage upgrades**; AND-only prerequisites; uniform steps | Choice is only order and timing; 22% of nodes are corridors; tree discoverability needed a new UI | Capability over percentage; exclusive branches; **component generations**; research that grows the *design space* |

## 1. Cross-cutting lessons

### 1.1 Nominal variety is not effective variety
WZ2100 has about **9,700 nominally valid designs** (14 bodies x 4 ground propulsions x (52 ground weapons + 10 system turrets), plus
VTOL designs, plus the dual-turret body's weapon pairs). But:
- On a medium cannon, only **12 of 56** body x propulsion combinations are efficient in (HP per cost, speed, armour); the three heavy
  bodies (Wyvern, Dragon, Vengeance) occupy the frontier on every propulsion.
- Add *absolute cost* as a fourth axis and **46 of 56 become "efficient"**: variety is mostly a **budget ladder** (more money, better
  numbers), not different roles.
- The original review's verdict was the same: "five tank bodies, each paired with several tracks and weapons, yield five largely similar tanks".

> **Theory note (Pareto dominance).** Option A *dominates* option B if A is at least as good on every axis that matters and better on
> one. A dominated option is a *trap* (never rational). The set of non-dominated options is the *Pareto front*. A design space is
> only as rich as its front is *thick across contexts*: many options should be best at something (a terrain, a budget, a role). Count
> the front, not the combinations.

### 1.2 Ladders versus sidegrades
Wheels to half-tracks to tracks step up toughness and down speed at a constant rate; the 14 bodies are one long quality ladder
(no body is strictly dominated once cost counts, but 12 of 13 beat the Viper on every raw stat). **Ladders are cheap to author and
easy to balance, but every step is a *replacement*, not a *choice*.** Sidegrades (better in one context, worse in another) create
decisions.

### 1.3 Plateaus and hidden cliffs
The speed cap (a plateau), the x1.5 weight bonus (a cliff), and the 33% damage floor (another plateau) are all places where a
number the player controls *stops mattering* or *jumps*. They are invisible on the design screen, which shows neither total weight
nor a warning. Rule of thumb: **every cliff and plateau should be visible and intentional.**

### 1.4 Flat armour and percentage upgrades diverge
Flat armour subtracts a fixed amount, so it favours big hits; percentage upgrades add a fixed *fraction of base*. The result: small
weapons never catch up with armour at any tech level (a Machinegun stays pinned at the floor against a Cobra after nine upgrades each).
Either keep this on purpose (weapon size as a counter) or use percentage armour.

### 1.5 Counter-graph density is decent, but the vocabulary is tiny
In the weapon-effect by propulsion matrix, **10 of 42 cells are strong (>=120%) and 16 are weak (<=60%)**: 62% of matchups matter. That
is why anti-personnel, anti-tank, artillery, bunker-buster, flamer and anti-air feel different. But there are only **6 effects** and
**7 propulsion types**, and both are closed lists.

### 1.6 Closed enumerations are the biggest rigidity
Propulsion types (7), component types (8), weapon effects (6), armour channels (2), terrain types (12), body sizes (4), structure
strengths (4) are all compiled in. Adding to any of them is a code change. See [unit-design/07-limits-and-extensibility.md](unit-design/07-limits-and-extensibility.md).

## 2. Engines

### What WZ does
- The engine is the body's `powerOutput`; research adds +5% for seven steps, then +7% and +8% (+50% in total). The engine line gates five chassis.
- **speed = multiplier x power / weight**, clamped by a per-propulsion cap, with a x1.5 bonus when base power exceeds weight.
- Excess power is not wasted entirely: **turning scales with the uncapped value**.
([04-engine-and-speed.md](unit-design/04-engine-and-speed.md))

### Evidence of limits
- 44% of ground designs (62% after full engine research) are at the cap: engine research is invisible for them.
- The upgrade line cannot move the x1.5 threshold; the design screen shows no total weight.
- Engine class equals size class (Light 5-6.5k, Medium 15-18k, Heavy 18-30k): there is **no engine decision**, only a hull decision.

### Openings
**A. Make the engine a component** with several attributes (each is a lever; pick only a few):

| Attribute | Gameplay effect |
|---|---|
| Power output | top speed and acceleration (the main axis) |
| Weight / size (slots) | competes with armour and weapons for the same budget |
| Torque versus power | acceleration and hill-climbing versus top speed: "tractor" or "sprinter" character |
| Efficiency / fuel | range, supply, endurance (only if logistics matter) |
| Heat / noise signature | detectability by sensors: couples to the information layer WZ already has |
| Durability, vulnerability | an engine hit slows or immobilises; an engine kill may explode |
| Compatibility | which propulsion classes it can drive (turbines for air and hover, diesels for tracks...) |
| Generation | Mk I-III (see research) |

**B. Replace the hard cap with a soft one.** *Theory:* a vehicle's top speed is where engine power equals resistance. Resistance
has a roughly linear term (rolling friction, proportional to weight and speed) and a cube term (fluid drag, proportional to
speed cubed). If propulsion types differ in those two coefficients, top speed emerges from power, weight and propulsion with **no
arbitrary cap**: more power always helps, with diminishing returns, and different locomotion types saturate differently
(drag-dominated hover/air benefit least per extra power, friction-dominated tracks most). *(General physics; not from WZ.)*

**C. Separate "how fast" from "how quickly".** Acceleration, top speed and hill-climbing can derive from different engine
attributes, so an engine has *character*, and WZ's hidden depth (agility from uncapped speed) becomes visible.

**D. Make it part of a power budget** (a reactor/engine supplies power that weapons, shields and sensors draw), so engine size
trades against firepower, not just speed.

### Pitfalls
- An engine slot becomes "always max" unless weight, volume, cost or heat make big engines *cost* something real.
- Hidden discontinuities (WZ's x1.5) erode trust; show final speed, total weight and the **limiting factor** ("engine-limited",
  "weight-limited", "locomotion-limited") in the design UI.
- Too many engine attributes = analysis paralysis; start with 2-3.
- Research should not make the engine the stingiest line; if engines gate hulls (as in WZ), the line must also create *capability*.

## 3. Propulsion variety

### What WZ does
- 7 closed types; 5 designable (wheels, half-tracks, tracks, hover, VTOL); legs for cyborgs only; "naval" defined but never unlocked.
- Propulsion is **four things at once**: a hull multiplier (weight, HP, cost), a speed class, a movement domain, and an armour class for
  the damage matrix. Its weight, cost, build points and HP are *percentages of the hull*.
- A 12-terrain speed table that **pathfinding ignores**; about 45 type-specific code branches; per-body x per-propulsion art models.
([03-propulsion.md](unit-design/03-propulsion.md))

### Evidence of limits
- Wheels, half-tracks and tracks are one machine with different numbers. Real behavioural differences exist only for hover (water),
  VTOL (air), legs (person physics) and the unreachable naval.
- Community: hover dominance, wheels lacking a role, half-tracks "unrealistic"; Battleplan added a weight rule and cut body classes
  to two.
- The engine already supports **component generations** (campaign Mk I-III) as pure data.

### Openings
**A. Propulsion as a trait set, not an enum.**

| Trait axis | Example values |
|---|---|
| Domain | land, water, air, amphibious, (subsurface) |
| Terrain profile | speed and passability per terrain type; slope limit; obstacle height; ground pressure (bogging) |
| Turning model | pivot, skid, turn radius |
| Load model | weight as a percentage of hull *or* absolute; cargo capacity |
| Signature | noise, dust, heat, radar cross-section |
| Upkeep | fuel, maintenance, range |
| Armour class | key into the damage matrix (or a separate armour layer) |
| Abilities | climb, hop, burrow, swim, tilt, tow |

**B. A candidate roster (illustrative only, none decided).**

| Propulsion idea | Tactical identity |
|---|---|
| Wheels | cheap, fast on roads, poor off-road |
| Half-tracks | the all-rounder |
| Tracks | heavy, slow, durable, ignores rough ground |
| Hover | fast, amphibious, fragile, drifts, weak to artillery |
| Light legs (walker) | slope- and rubble-capable, small signature |
| Heavy legs (mech) | steps over low walls, tall target, slow |
| Rotor | slow air, hovers, vulnerable to AA, carries cargo |
| Jet VTOL | fast strikes, limited ammunition, rearm cycle (WZ model) |
| Airship | slow, high-capacity, huge target |
| Surface naval / submarine | water control, bombardment, stealth |
| Crawler | extreme load and armour, extremely slow |
| Hopper / jump | short bursts over obstacles, noisy |
| Rail-bound | cheap and fast but only on built track (infrastructure game) |

**C. Let terrain steer routes.** Give each propulsion its own path cost per terrain so that hover seeks water, wheels seek roads and
legs cut across rubble. Without this, a rich terrain table does nothing (WZ's own situation).

**D. Avoid pure ladders with a rule:** every propulsion must be *best in at least one context* (terrain, mission, budget), checked
by the frontier analysis in section 6.

### Pitfalls
- **Art explosion:** WZ stores a model per body x propulsion. Use attachment points or modular art.
- **AI cost:** each domain needs pathing and tactics; a data-defined trait set keeps this generic.
- **Dominant propulsion:** hover shows how one option becomes universal. Counter with real weaknesses (terrain, cost, damage
  matrix) and test with simulation.
- Too many types dilute identity; WZ's 5 are already hard to tell apart for new players.

## 4. Unit-building complexity

### What WZ does
- Three choices (body, propulsion, one weapon; two on the Dragon). Any ground weapon fits any ground body. The only budgets are price
  and weight (indirectly speed). Armour comes only from the body. Utility turrets are thin (6 sensors, 2 repair, 1 truck, 1 command,
  no usable ECM).
- Derived stats are partly hidden (no total weight, no build time on the screen).
- Cyborgs live in a closed parallel system.
([01-design-pipeline.md](unit-design/01-design-pipeline.md), [05-weapons-and-turrets.md](unit-design/05-weapons-and-turrets.md))

### Evidence of limits
- "Five largely similar tanks"; efficient-chassis analysis (section 1.1).
- Weapon *weight* spans 150x but is the only mount constraint.
- Armour is not a component (it is the hull), so there is no armour-versus-speed trade-off *within* a design.

### Openings
**A. More slots.** Candidates: engine; armour plating (separate from hull); reactor/power core; locomotion; weapon mounts of different
sizes (hardpoint, turret, pintle); utility bays (sensor, jammer, repair drone, shield, cargo, smoke, mine layer); ammunition storage.

**B. Budgets create decisions.** Constraint types:

| Budget | Effect |
|---|---|
| Weight capacity (hull) | heavier loadout means slower or impossible |
| Power balance (generation vs. draw) | energy weapons, shields and sensors compete with the engine |
| Volume / slot count | hard limits on how much fits |
| Heat | sustained fire versus dissipation |
| Crew / command points (optional) | limits army size or complexity |

Hard constraints (this mount takes only that size) make choices *categorical*; soft ones (weight reduces speed) make choices
*continuous*. WZ uses only the soft kind.

**C. Synergies and anti-synergies** so that parts interact: a sensor boosts nearby artillery; heavy armour plus a small engine is
slow; shields draw power; stealth conflicts with noisy engines. Interactions are what make a *design* more than a sum of parts.

**D. Role clarity.** Offer named archetypes (scout, line tank, artillery, anti-air, carrier, support, siege) with sensible defaults, tags
and warnings. Let advanced players ignore them.

**E. Legibility.** Show total weight, final speed, acceleration, power balance, price, build time, a **matchup preview** against the
damage matrix, and the limiting factor. Provide design templates and refits.

### Pitfalls
- **Analysis paralysis and min-maxing:** more slots mean more optimal answers; use budgets so the optimum depends on context.
- **AI and balance cost:** build automated balance simulations early (our analysis scripts are a starting point).
- **False choices:** a slot where one option is always best is decoration. Test each slot for non-dominated options.
- **Progressive disclosure:** a *basic* and an *advanced* design mode so beginners are not overwhelmed (WZ added an in-game guide
  only in 4.5).
- **Production:** complex designs need production consequences (build time, modular factories, prototype costs) or players will
  design-spam.

## 5. Research

### What WZ does
- 390 topics (mp): 52% pure percentage upgrades, 25% unlock components, 22% unlock structures (72 of 90 structures are defences).
- AND-only prerequisites; steps are uniform (+25% damage, +30% armour, +5% engine); chains are strictly sequential; one lab, one topic.
- Armour and engine lines **gate chassis**; weapon lines gate defences; the campaign gates with artifacts and key topics.
- Percentage upgrades are **retroactive** (no refit decision); the whole tree cannot be finished in a game (about a third per hour with
  five fully upgraded labs).
([research-system/](research-system/README.md))

### Evidence of limits
- The only choices are order and timing; there are no exclusive branches.
- 22% of nodes are corridors (one in, one out): waiting, not choosing.
- The upgrade list is closed (speed, weight, cost cannot be upgraded); the filter language is one equality test.
- Community pain: tree navigation, mandatory upgrades, dead-end weapons; modders tried era-based progression, two-line prerequisites,
  and compounding curves.

### Openings
| Idea | What it adds |
|---|---|
| **Capability over percentage** | new rules and options (a new mount, a new mode), not just +25% |
| **Exclusive branches / doctrines** | real, committal choice; replay variety (needs an OR/exclusion primitive WZ lacks) |
| **Component generations (Mk I-III)** | new variants with different trade-offs; older units stay different, so refit becomes a decision (WZ campaign precedent) |
| **Non-uniform curves** | front-loaded, diminishing or compounding steps (Contingency: nine steps multiply by 4); different steps have different values |
| **Cross-line prerequisites** | WZ's armour/engine gating of bodies; Contingency's weapons needing two lines; creates interdependence |
| **Research that grows the design space** | new component *classes* (engine types, armour types, locomotion classes), so research changes what the design screen offers: the biggest "impact" lever |
| **Retrofit costs** | upgrades apply to new builds; old units are refitted at a cost, so timing and composition matter |
| **Research as exploration** | artifacts, salvage, captured sites (the campaign model) tie tech to map control |
| **Scarcity by design** | keep the tree larger than any game can finish (WZ already does) |
| **Discoverability** | path-to-target, effect text, critical-path filters (WZ added these in 4.x) |

### Pitfalls
- Exclusive branches multiply the balance matrix and can feel punishing if irreversible; consider limited respec.
- Snowballing: faster tech should not win outright (WZ's catch-up mechanism is tech theft).
- UI burden grows with branching; plan the tree view first.
- Retroactive upgrades are friendlier; refit-cost systems add bookkeeping.

## 6. How we will know it works: instruments

The scripts in `tools/research` can be rebuilt for our own data. Baselines from WZ2100:

| Instrument | WZ2100 baseline | What we want |
|---|---|---|
| Nominal design count | about 9,700 | not the goal by itself |
| Efficient share of chassis combos | 12 of 56 (efficiency axes) / 46 of 56 (with cost) | every propulsion and body on the frontier in some context |
| Speed-cap saturation | 44-62% of ground designs | low; no plateau at all if possible |
| Weapons pinned to the damage floor (vs a Vengeance on tracks) | 17 of 41 | deliberate, small, and visible in the UI |
| Counter-matrix density | 26 of 42 cells strong or weak; 6 effects | high, with a larger vocabulary |
| Upgrade-only share of research | 52% | lower, with capability-bearing upgrades |
| Corridor share of research | 22% | lower |
| Exclusive branch points | 0 | some |
| Tree completed per hour (5 upgraded labs) | about 33% | choice-forcing scarcity |

## 7. Decisions we will need to make (questions for the project owner)

1. **Variety philosophy:** *more, structured choices* (Contingency-like families and cross-line research) or *fewer, clearer choices*
   (Battleplan-like)? They are opposite answers to the same problem.
2. **Scale of propulsion expansion:** how many types are realistic given art, pathing and AI cost?
3. **Constraint style:** soft (weight, cost) only, or hard slot/mount constraints too?
4. **Technical foundation:** 2D or 3D; our own engine or an existing one; how the AI will consume a design system.
5. **Mode focus:** campaign (curated, artifact-driven research), skirmish/multiplayer (open tree), or both.
6. **Complexity budget:** how many decisions per unit are acceptable for a new player, and is there an advanced mode?

## 8. Outside precedents (general knowledge; not researched this session, verify before relying)

- **BattleTech-style construction** (tonnage, critical slots, heat) is the textbook case of *hard* slot and weight budgets plus a
  heat mechanic.
- **Mech-assembly games in the Armored Core style** use weight capacity and generator/energy limits.
- **Vehicle-builder sandboxes** (for example *From the Depths*, *Space Engineers*) offer block-level freedom with physical simulation,
  at the cost of very high complexity.

These are mentioned only to indicate where to look next if we decide to research constraint-based design beyond Warzone 2100.
