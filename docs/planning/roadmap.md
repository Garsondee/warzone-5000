# Roadmap

*Re-planned 2026-10-08 for the design-first async auto-battler ([D5, D6](decisions.md)).* Milestones retire the biggest
risks first. Each ships a short theory note in `docs/notes/` and has a check Claude can run in a cloud session.

| # | Milestone | Risk it retires | Verified by | Status |
|---|---|---|---|---|
| M0 | Workspace; `w5k_math` (Q32.32 fixed point, sqrt/trig, RNG, state hashing); design docs | Determinism foundation | property tests; golden hash | **done** |
| M1 | Part Forge: convex primitives, mesher, vertex attributes, voxel mass, exact armour rays, sockets and assembly, budgets, previews, GLB; starter parts and vehicles (tank, drone, spider walker, 6x6 APC) | Procedural look; honest stats | analytic tests (volume, mass, inertia, t/cos armour, hover power); owner art review | **done** |
| M1b | Parametric families: gun turret, three hull shapes, tracks, engines (done); wheels, legs, rail, sensors (next); slider solver with locks; family-based designs with socket hints (done); shared art direction (done) | Is slider design rich and legible? | family tests; sweep, coupling, lineup and faction sheets reviewed by the owner | **in progress** |
| M2 | Godot bridge and design bench prototype: gdext crate, vertex-colour shader, part viewer, sliders that regenerate the model live | Engine integration; interactive forge | in-engine screenshots (cloud via Xvfb, or owner); slider drag stays responsive | planned |
| M3 | Battle simulation core: deterministic movement, ballistics with flight time, armour-table hits, part damage, replays and hashes, headless runner | Deterministic combat at speed | identical hashes across runs and builds; analytic ballistics checks; a 1,000-unit battle within budget | planned |
| M4 | Test arena in the design bench: one design against dropped-in bots, in real time | The core "tune and test" feel | owner playtest | planned |
| M5 | Battle AI and terrain: attack-move, targeting, behaviour sliders, heightfield, soft ground and bogging, line of sight | The auto-battler is worth watching | scripted scenario suites; owner watches replays | planned |
| M6 | Command-point pricing by benchmark gauntlet; balance tooling | Swarms against titans stays a real choice | win-share reports across seeds | planned |
| M7 | Run loop vertical slice: draft, design, deploy, battle against local snapshot pool, shop, licences | Is the loop fun? | owner playtest | planned |
| M8 | Walkers with IK legs, ropes and toppling, rail transport, titans | The spectacle | invariant tests; filmstrips | planned |
| M9 | Online async: snapshot pool and result verification service | Multiplayer | replay verification round-trip | planned |
| M10 | Scale, performance and polish | The top end | benchmarks on the owner's GPU | planned |
