# The brief

*One document every session reads first. If it conflicts with anything else, this wins, and the conflict is a bug to report.*

## What we are building
A **realistic ground-vehicle simulator**: wheeled and tracked vehicles (1-80 tonnes) that drive, ride, brake, steer, climb and bog on a
rough obstacle course (hills, roads and mud, barricades, buildings, trees), and later shoot targets scattered along it while an AI
drives and fights. It grows into an **auto-battler**: the player designs vehicles from parametric parts, and a strong AI, which
understands its own vehicle's limits and adapts, runs the units.

## Why the physics is the product
If a design choice does not change what happens, the simulations are not interesting. So the vehicle model has to be deep enough that
suspension, engine and gearbox, brakes, running gear, hull and turret each have **measurable consequences**, and honest enough to be
checked against real vehicles. The test of "deep enough" is the **Design Impact Matrix** (`docs/validation/IMPACT-MATRIX.md`): every
design lever must move at least one benchmark by a plausible amount, and every benchmark must have a lever that moves it. A lever that
changes nothing is a dead lever; an effect nobody can influence is wasted complexity.

## What "realistic" means here
- **Validated against real vehicles.** A reference garage of parametric builds that mimic real ones (M998 HMMWV, M113, M4A3 Sherman for
  calibration; M1A1, Leopard 2A5, T-72B, M35, Tiger II held out). Published figures within stated bands; every number carries its
  provenance (`SPEC / MEASURED / ESTIMATE / TUNED`). The claim we make is "inside the envelope of what is published", not "exact".
- **Dynamics we write ourselves**: a 6-degree-of-freedom hull, per-wheel suspension, tyre and track contact (including soft ground),
  a real powertrain (engine torque map, clutch or torque converter, gearbox, differentials or steering unit, brakes with heat),
  and an articulated turret and gun (the turret rotates while the cannon pitches and recoils). Everything is explainable through the
  force ledger.
- **Art direction**: realistic camouflage and weathering, realistic hull geometry and proportions, articulated parts driven by the
  physics. No toy colours, no neon faction palettes (factions are told apart by camo and markings).

## What we decided (the owner's answers; ADRs record them)
| Question | Decision |
|---|---|
| Scope | Realistic ground vehicles (wheeled and tracked). Legs, hover, anti-gravity, rotors, rail, beams are parked, not deleted |
| Numbers | Plain 64-bit floats with strict discipline: fixed step, `libm`, no fast-math, golden hashes on Linux and Windows CI (replaces Q32.32 fixed point) |
| Old code | Clean slate: new workspace; the 2854baf prototype is a read-only reference toolbox |
| Battle size | A handful, up to about 20 vehicles, every one at full fidelity |
| Multiplayer | Not for a long time. Determinism stays for testing and replays |
| Control | We own the important code (dynamics, contact, powertrain, AI); third-party code only for I/O and queries |

## Assumed until the owner says otherwise (each is a decision card with this default)
Era: modern-ish (c. 1950s to today). Product order: vehicle lab, then proving ground and course, then targets, then AI skirmish; the
draft / command-point / shop loop is parked. The owner can drive any vehicle through the same `Command` interface the AI uses. World:
about 2 km square, 1 m heightfield, courses are data made by a procedural generator first. Art: procedural PBR camo and weathering plus
a lofted hull-geometry kit. "Adapts" means tactics and learned capability estimates, not machine learning.

## The path (milestones; numeric exit criteria in `docs/swarm/RULES.md` and each lane brief)
1. **M1 First light**: an M998-class 4x4 truck on a bump strip: engine map, torque converter, 4-speed automatic, diffs, brakes with
   heat, steering, four independent suspensions, tyres; design to rig to step to replay to viewer to validation. Contracts freeze here.
2. **M2 Mud and steel**: tracked vehicles and soft ground; a 2 km mixed course; five or more reference vehicles; camo and hull kit v1;
   Godot drive mode and a downloadable Windows package; held-out scoring.
3. **M3 Turret and targets**: articulation, stabiliser, recoil, ballistics, penetration, damage, targets along the course.
4. **M4 Thinking vehicles**: perception, capability table, planner, driver, gunner, tactician, stuck recovery.
5. **M5 Game loop**: revisit the product shape (skirmishes, then the auto-battler loop).

## How we avoid spinning out
The guardrails live in the repo, not in prompts: this brief, `CLAUDE.md`, the lane briefs, the contracts, CI that rejects drift (lane
guard, no bare constants, no non-portable maths, no new dependencies, goldens only on purpose), numeric gates with the owner's sign-off,
and decision cards instead of silent choices. Parallel lanes build against contracts and stand-ins; the integration spine (the
first-light scenario) runs on every merge. See `docs/swarm/RULES.md`.

## Who decides what
The **owner** decides scope, priorities, the decision cards and every milestone sign-off. **ARCH** (the coordinator session) owns the
contracts, ADRs, CI and merges, and writes no feature code. **Lanes** own their paths and nothing else.
