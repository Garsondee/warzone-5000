# ADR-0003: Clean slate, with the prototype as a reference toolbox

**Status:** Accepted (owner decision, 2026-10-08: "Clean slate"). The mechanics of leaving the old code behind are in the Consequences.

## Context
The prototype (commit `2854baf`, about 16.8k lines of Rust) proved the pipeline (design, physics, replay, viewer) but is a 1-D point-mass
simulation: joints are animation labels only, there is no sideways motion, the design-to-physics bridge is a flat bag of numbers, forge depends
on sim, and the numerics are fixed-point. It is not the foundation for 6-DoF vehicles with suspension, powertrain and articulation. The owner
chose a clean slate over migrating the good parts.

## Decision
1. New workspace and new foundation (see `docs/architecture/LAYERS.md`). **Nothing is imported from the prototype.** Anything reused is re-read, re-written against the new contracts and re-tested.
2. The prototype stays **reachable**: in git history at `2854baf`, with `docs/archive/prototype-v0/` for the superseded documents and, until the owner says to delete it, `reference/prototype-v0/` for the code (read-only; excluded from the workspace and from CI; no lane may edit it).
3. The **port ledger** (`docs/archive/prototype-v0/PROTOTYPE-INDEX.md`) lists, per piece, QUARRY (re-read and re-write), KEEP (still valid as theory), DROP, or ARCHIVE:
   - QUARRY: `Pcg32` and `StateHasher` (ported into `w5k_math` already), the voxel kernel (mass, centre of mass, armour rays) which FORGE must re-write for inertia tensors and armour thickness, the software rasteriser and chart plotting (dev tools), the three.js viewer shell and the MP4 capture pipeline, the Linux + Windows CI matrix.
   - KEEP: `docs/theory/the-physics-of-a-time-trial.md` (valid physics; its determinism section is superseded by ADR-0002).
   - DROP: fixed-point maths, the 1-D mover, soil, course, trial, spec, the sheet as a source of truth, the sci-fi families.
   - ARCHIVE: prototype content, design documents and decisions.

## Consequences
- The voxel kernel is re-written (a known cost, accepted for full understanding and control).
- Lessons kept as principles: coherent units, explainable outcomes (the force ledger), analytic-oracle tests, replay as the interface between simulation and viewers.
- Deleting `reference/prototype-v0/` entirely is a single `git rm -r` the owner can order at any time.
