# Interface request: CHASSIS -> ARCH, a NOT-MODELLED row for tyre load sensitivity

**From:** CHASSIS · **To:** ARCH (owner of `docs/brief/NOT-MODELLED.md`) · **Date:** 2026-10-10 · **Asked for by:** ARCH (message of 15:17Z)

## What
Please add this row to `docs/brief/NOT-MODELLED.md`:

| Effect | Why it is out | Revisit when |
|---|---|---|
| Tyre load sensitivity (friction and cornering stiffness per unit load falling as the load rises) | The CHASSIS tyre is linear in load: `mu`, `slip_stiffness` and `cornering_stiffness_per_rad` are per unit load and constant, so a tyre at 4x its static load makes 4x the force. Load transfer therefore never costs total grip; the limit lateral acceleration reads high and lateral load transfer does not move the understeer balance (a design lever: anti-roll bar split front/rear, COM height). | VALIDATION's skidpad or limit-handling comparison against published data shows it, or the Hauler's cornering limit matters. Fix: a CCR adding `TyreDef.load_sensitivity` (e.g. `mu = mu0 (1 - k (Fz/Fz0 - 1))`, the same for the stiffnesses), about one CHASSIS PR. |

## Why it matters
It is a known gap, not a choice to leave out for good: the anti-roll split is a player lever whose main effect on handling is exactly this. Seen in practice: on the slice course's whoops the Scout's front tyres reach 11.5 kN (about 4x static) and, with no load sensitivity, they make 4x the cornering force, which inflates the yaw jolt (PR #76).

## Default if no answer
CHASSIS records it in `docs/swarm/status/chassis.md` as a known gap and carries on; the row is ARCH's to add.
