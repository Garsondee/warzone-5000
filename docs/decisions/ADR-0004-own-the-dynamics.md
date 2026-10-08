# ADR-0004: We write the dynamics ourselves

**Status:** Accepted (owner brief, 2026-10-08: "full control over all the important code ... so that we can edit them when things don't go to plan").

## Context
The vehicle physics is the product, must be explainable (a force ledger, a theory note per module) and must be checkable against real data. Tyre,
track and soil laws are exactly what general rigid-body engines do not provide. A black-box solver would make the first mysterious behaviour
undebuggable.

## Decision
1. We write: the 6-DoF hull integrator, suspension, tyre and track contact, soil laws, powertrain and brakes, articulation and servos, ballistics and penetration, and the AI. We use **penalty (spring-damper) contacts with regularised friction** and a fixed small step; no constraint or impulse solver.
2. Third-party code is allowed for I/O and helpers only: `serde`, `ron`, `serde_json`, `libm`, image and CLI helpers (pre-approved: `png`, for image output only: plots, contact sheets, previews; never used by a simulation crate). Nothing that decides simulation results (physics engines, maths or RNG crates) without an approved decision card.
3. Collision queries against props: our heightfield and primitive shapes (sphere, cylinder, box, capsule) first. `parry3d-f64` may be adopted **by card**, for queries only, never its solver, if the primitives prove insufficient.
4. Every dependency is declared once in the root `[workspace.dependencies]` (ARCH-owned); crates use `name.workspace = true`; CI rejects anything else.
5. Every module ships a plain-language theory note (`docs/theory/<lane>.md`) so the owner can follow and edit it.

## Consequences
- More code for us to write and validate, and complete control over every force. The S1 spike establishes the step-size rule; the S7 spike checks the cost of 20 vehicles.
- Rapier, Bullet, Godot physics and Unity physics are out for the solver.

## Alternatives considered
Rapier or Bullet for the hull with custom tyre forces (rejected: contact, friction and stacking behaviour would be opaque, and tracks and soil would fight the solver).
