# Prototype v0: index and port ledger

The first prototype (about 16.8k lines of Rust, commit `2854baf` on branch `claude/sharp-babbage-d702f7`) built a parametric part forge, 18 component families, a 1-D point-mass "time trial" simulation with soft-earth soil, a replay format and a browser viewer.
It proved the pipeline and taught what is hard; it is **not** the foundation for 6-DoF vehicles (ADR-0003). Nothing is imported from it. This page says what is worth re-reading and where it lives.

**Where the code is:** in git history (`git show 2854baf:<path>`), and, until the owner orders it deleted, in `reference/prototype-v0/` (read-only; excluded from the workspace and from CI; no lane may edit it).
**Where the documents are:** `docs/archive/prototype-v0/` (design documents `design/`, theory notes `notes/`, the decision log `decisions-v0.md`, the roadmap `roadmap-v0.md`, images `assets/`); the one theory note that stays live is `docs/theory/the-physics-of-a-time-trial.md`.

## Port ledger (starter; the Launch Kit confirms it)
Verdicts: **QUARRY** re-read, re-write against the new contracts, re-test. **KEEP** still valid as written. **DROP** superseded. **ARCHIVE** historical.

| Piece | Where (`git show 2854baf:...`) | Verdict | Why / what to take |
|---|---|---|---|
| `Pcg32`, `StateHasher` | `crates/w5k_math/src/{rng,hash}.rs` | QUARRY (done) | integer-only and generic; ported into `w5k_math`, with new `f64` helpers |
| Q32.32 `Fx` and fixed-point trig | `crates/w5k_math/src/{fx,vec}.rs` | DROP | ADR-0002: `f64` + `libm` |
| Voxel kernel (mass, COM, armour rays), primitive trees | `crates/w5k_forge/src/{voxel,armour,geom,convex}.rs` | QUARRY | ideas for armour thickness by ray casting; mass properties now come from closed-mesh integrals (GEOMETRY) |
| The 18 families and `style.rs` | `crates/w5k_forge/src/family/*.rs` | DROP / REWRITE | sci-fi parked; ground families are rewritten for realism by FORGE and GEOMETRY |
| The vehicle sheet, explorer, possibility-space tools | `crates/w5k_forge/src/{sheet,explore}.rs`, `crates/w5k_tools/src/space_cmd.rs` | DROP | the sheet becomes a cached surrogate of proving-ground runs; the sampling idea survives in the fuzz |
| The 1-D mover, soil, course, trial, spec | `crates/w5k_sim/src/{mover,soil,course,trial,spec}.rs` | DROP (read `soil.rs` for TRACKS) | keep the *ideas*: coherent units, explainable outcomes (now the force ledger), analytic-oracle tests, replay as the interface |
| Replay (20-byte frames, limiting-factor codes) | `crates/w5k_sim/src/replay.rs` | QUARRY | the concept (replay as the interface); the format is replaced by replay v2 |
| Software rasteriser and chart plotting | `crates/w5k_forge/src/{raster,preview}.rs`, `crates/w5k_tools/src/{plot,trial_plots}.rs` | QUARRY | charts and previews without a browser: VIEWER re-writes a small plotter |
| three.js viewer, build and capture scripts | `tools/trial/{template.html,build.py,capture.js,frames.js,shot.js}` | QUARRY | viewer shell and MP4 pipeline: VIEWER re-writes for replay v2 |
| `tools/explorer/` | `tools/explorer/*` | DROP | the possibility-space page |
| CI matrix (Linux + Windows) | `.github/workflows/ci.yml` | QUARRY (done) | replaced by the three-tier workflows in `.github/workflows/` |
| Golden roster | `crates/w5k_sim/tests/fixtures/roster.json` | DROP | per-scenario goldens now |
| Content: parts, vehicles, materials, specs | `content/{parts,vehicles,materials.ron,specs}` | ARCHIVE | sci-fi |
| Content: `terrain.ron`, `courses/hill_valley.ron` | `content/` | ARCHIVE | the hill-and-valley course may return as a regression course; the soil numbers were tuned for a game, not measured |
| Theory notes | `docs/notes/*` | KEEP `m1c` (now `docs/theory/the-physics-of-a-time-trial.md`); ARCHIVE `m1`, `m1b` | the m1c physics is valid; its determinism section is superseded by ADR-0002 |

## What the prototype taught (kept as principles)
- Coherent units keep numbers honest; SI with unit-suffixed names does the same job for `f64`.
- Every outcome must be explainable: record what limited the vehicle each tick (now the force ledger and `LimitingFactor`).
- Two independent calculations that agree (the closed-form sheet and the integrated simulation) are the strongest check; this is now the analytic-oracle rule.
- Tracks float where tyres bog is a *derived* result of Bekker's law, not an assumption: keep deriving, not asserting.
- A replay is a G-buffer for time: write once, view anywhere.
- It was 1-D with no sideways dynamics, cosmetic animation and a lumped bag of numbers between design and physics. Those are exactly what the new contracts fix.
