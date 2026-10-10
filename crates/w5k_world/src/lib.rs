//! `w5k_world`: lane WORLD.
//!
//! The obstacle course: heightfield, materials, props, procedural course generator, WorldQuery.
//!
//! Status: building. `grid` is the spike S-W heightfield world (docs/lanes/world/spike-w.md) made size-generic; M1 splits it into
//! `heightfield`, `props`, `noise` and `generator` modules.

pub mod course;
pub mod grid;
pub mod strip;
