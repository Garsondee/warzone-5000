# Design Takeaways for warzone-5000

These are *interpretations*, not facts about Warzone 2100. They are candidate pillars to evaluate when we start
`docs/design/`. Nothing here is a decision yet.

## Pillars that make Warzone 2100 distinctive
1. **Component-based unit design.** Players author units. Counters come from component choices.
2. **Research as the main progression**, with artifacts gating tech in the campaign.
3. **Fixed-point economy** (oil derricks plus generator ratio) rewarding map control over micro.
4. **Information warfare** (sensors, counter-battery, VTOL rearming) layered over classic RTS combat.
5. **Persistent-base campaign with timers and away missions.**
6. **Data-driven and scriptable**, so AI and content are community-extensible.

## Complexity warnings
- The design screen, orders menu, commanders and sensor assignment give a steep learning curve. Warzone's own
  quick-start guide is long. Consider an in-game guide early (the project added one only in 4.5).
- Hotkey and order breadth is a feature for experts and a barrier for newcomers.

## Suggested next steps
1. Decide scope: spiritual successor, clone, or reimagining.
2. Decide engine/language and licensing stance (see GPL note in the overview).
3. Pick a vertical slice: e.g. one map, 3 bodies, 3 propulsions, 3 weapons, derricks and a factory.
4. Write design docs under `docs/design/` and a roadmap under `docs/planning/`.
