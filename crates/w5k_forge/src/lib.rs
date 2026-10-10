//! `w5k_forge`: lane FORGE.
//!
//! Design to physics: `VehicleDef` compiles to `PhysRig` and `RenderRig`; the compile report says what every slider became.
//! Only lane FORGE edits this crate (`docs/swarm/ownership.toml`). Design note: `docs/lanes/forge/design-note.md`.

pub mod compile;
pub mod curve;
pub mod extras;
pub mod render;
