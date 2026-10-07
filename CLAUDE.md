# Working notes for Claude on warzone-5000

## How the owner likes to work
- **Show the visuals.** Whenever something visible changes (parts, vehicles, previews, UI), render it and
  send the images to the owner (`SendUserFile`), not just a description. Contact sheets come from
  `cargo run --release -p w5k_tools --bin w5k -- render content --out <dir>`.
- **Teach the theory.** The owner has a computer graphics background and wants to learn: explain the principle
  behind each design or engineering decision (the physics, the maths, the trade-off), briefly and concretely.
- Design documents live in `docs/design/`, plans and decisions in `docs/planning/`, theory notes in `docs/notes/`.

## Conventions
- Units are metres, kilograms and seconds. +Y is up, forward is -Z, right is +X (Godot's convention).
- Gameplay simulation is deterministic: fixed point (`w5k_math`), seeded RNG streams, ordered iteration
  (`docs/design/02-determinism-rules.md`). Offline content baking (`w5k_forge`) may use floats.
- Content is RON under `content/`; every part and vehicle must pass `cargo test --workspace`.
- Do not copy Warzone 2100 code or assets (GPL); research notes only.
- Commit trailers carry no model identifier.
