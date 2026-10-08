# Working notes for Claude on warzone-5000

## How the owner likes to work
- **Show the visuals.** Whenever something visible changes (parts, vehicles, previews, UI), render it and
  send the images to the owner (`SendUserFile`), not just a description. Contact sheets come from
  `cargo run --release -p w5k_tools --bin w5k -- render content --out <dir>`; also `lineup` (one army at true
  scale, `--palette`), `factions` (one design in every palette) and `family` (slider sweeps and coupling).
  Mixing tools (all `w5k <cmd> content --out ...`): `sweeps` (every family on its host hull), `view` (one design, `--zoom`),
  `gallery --only a,b` (labelled designs), `roll --seed N` (random auto-fitted designs), `atlas` (every hull with every
  gear), `ladder` (one archetype across sizes), `space` (sampled possibility-space charts and CSV), `fit --spec` (auto-fit a
  design file: specs live in `content/specs/`, fitted designs in `content/vehicles/`) and `rates`/`why` (debug the fitter).
  Time trials: `w5k trial content --out DIR` runs every design through the physics (`w5k_sim::mover`, fed by the forge's
  `mover_spec`) on `content/courses/hill_valley.ron` and writes `results.png`, `traces.png`, `course.png` and the replay (`--mode
  parade` is the physics-free reference lap); then `python3 -I tools/trial/build.py DIR page.html` builds the 3D replay page and
  `node tools/trial/capture.js` records an MP4 (see `tools/trial/README.md`; the sim is the `w5k_sim` crate).
  Every outcome must be explainable: the replay records what limited the vehicle each tick, and a run that ends early says why.
- **Art direction** is in `docs/design/06-art-direction.md`; every new family decorates itself through
  `crates/w5k_forge/src/family/style.rs` so the army stays consistent.
- **Teach the theory.** The owner has a computer graphics background and wants to learn: explain the principle
  behind each design or engineering decision (the physics, the maths, the trade-off), briefly and concretely.
- Design documents live in `docs/design/`, plans and decisions in `docs/planning/`, theory notes in `docs/notes/`.

## Conventions
- Units are metres, kilograms and seconds. +Y is up, forward is -Z, right is +X (Godot's convention).
- Gameplay simulation is deterministic: fixed point (`w5k_math`), seeded RNG streams, ordered iteration
  (`docs/design/02-determinism-rules.md`). Offline content baking (`w5k_forge`) may use floats.
- Content is RON under `content/`; every part and vehicle must pass `cargo test --workspace`.
- A new family needs: `fits()` (the socket kinds), `host()` (where sweeps show it), `fit_to_load` if it carries weight, a
  `style.rs`-based look, an entry in `family::all()`, physics tests in `crates/w5k_forge/tests/mixing.rs`, and a row in
  `docs/design/04-parametric-components.md`.
- Do not copy Warzone 2100 code or assets (GPL); research notes only.
- Commit trailers carry no model identifier.
