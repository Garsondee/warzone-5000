# ADR-0001: Realistic ground vehicles, simulator first

**Status:** Accepted (owner decision, 2026-10-08). Supersedes D5 (async auto-battler first) and the scope of `00-vision` in the archived prototype docs.

## Context
The prototype aimed at a stylised, sci-fi auto-battler with every kind of locomotion and asynchronous multiplayer. The owner's new brief
is a **realistic ground-vehicle simulator** whose value comes from deep vehicle physics ("if the design choices don't have real impact,
the end simulations won't be interesting"), validated against real vehicles, on an obstacle course with targets and a strong AI. The
auto-battler remains the destination, but the physics and the course come first.

## Decision
1. Scope is **wheeled and tracked ground vehicles**, 1-80 t, with realistic hulls, camouflage and articulated turrets. Legs, hover, anti-gravity, rotors, rail and beam weapons are **parked** (kept in the archive, not deleted).
2. Product order: **vehicle lab, then proving ground and course, then targets, then AI skirmish, then the auto-battler loop** (M5). The draft / command-point / shop / three-lives loop is parked.
3. Multiplayer, netcode and lockstep are off the table "for a long time".
4. A battle has **up to about 20 vehicles, each at full fidelity**; no level-of-detail until a scale spike says it is needed.

## Consequences
- The component catalogue is rebuilt for real vehicles (engines with torque maps, gearboxes, suspensions, tracks, turrets); the sci-fi families do not carry over.
- `docs/brief/NON-GOALS.md` lists what we are deliberately not doing; lanes treat it as a tripwire.
- Roadmap milestones M1-M5 replace the prototype's M0-M3 numbering.

## Alternatives considered
Keep the sci-fi families alongside realistic ones (rejected: each locomotion class is its own contact model and "match real vehicles" contradicts them); build the auto-battler loop first (rejected: it needs a trustworthy simulation to be fun).
