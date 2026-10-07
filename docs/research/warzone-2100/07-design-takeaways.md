# Design Takeaways for warzone-5000 (first pass)

> **Status: superseded in depth by [expansion-analysis.md](expansion-analysis.md).** This page keeps the first-pass summary; the deep
> dives are in [unit-design/](unit-design/README.md) and [research-system/](research-system/README.md).

These are *interpretations*, not facts about Warzone 2100. They are candidate pillars to evaluate when we start `docs/design/`.
Nothing here is a decision yet.

## Pillars that make Warzone 2100 distinctive
1. **Component-based unit design.** Players author units. Counters come from component choices (and, as the deep dive shows, from
   *propulsion acting as the armour class*).
2. **Research as the main progression**, with artifacts gating tech in the campaign and armour/engine lines gating chassis.
3. **Fixed-point economy** (oil derricks plus generator ratio) rewarding map control over micro.
4. **Information warfare** (sensors, counter-battery, VTOL rearming) layered over classic RTS combat.
5. **Persistent-base campaign with timers and away missions.**
6. **Data-driven and scriptable**, so AI and content are community-extensible.

## What the deep dive added (the main surprises)
- There is **no engine component**: speed is `power / weight` capped per propulsion, and the cap hides the engine for 44-62% of designs.
- More than half of research is **uniform percentage upgrades**; there are no exclusive choices.
- Three of the five propulsions are **one ladder**; pathfinding ignores terrain speed.
- Flat armour makes small weapons permanently ineffective against heavy bodies.
- Most structure (propulsion, damage effects, component types, terrain) is in **closed enumerations**, the main source of rigidity.

## Complexity warnings
- The design screen, orders menu, commanders and sensor assignment give a steep learning curve. The project added an in-game
  guide only in 4.5 and a research-tree screen only recently.
- Hotkey and order breadth is a feature for experts and a barrier for newcomers.
- Hidden mechanics (the x1.5 weight bonus, per-step rounding, floor damage) erode trust; show derived stats.

## Next steps
1. Decide the questions listed in [expansion-analysis.md](expansion-analysis.md) section 7 (variety philosophy, propulsion scale, constraint style, technical foundation).
2. Decide engine/language and licensing stance (Warzone 2100's code and data are GPL; we study but do not copy).
3. Pick a vertical slice: e.g. one map, 3 bodies, 3 propulsion traits, an engine component, 6 weapons, derricks and a factory.
4. Write design docs under `docs/design/` and a roadmap under `docs/planning/`; rebuild the analysis scripts for our own data to
   measure the instruments in expansion-analysis section 6.
