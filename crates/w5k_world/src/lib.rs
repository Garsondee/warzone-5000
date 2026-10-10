//! `w5k_world`: lane WORLD.
//!
//! The obstacle course: heightfield, materials, props, procedural course generator, WorldQuery.
//!
//! Status: SETTLING. `spike_w` is the throwaway risk spike S-W (docs/lanes/world/spike-w.md); M1 splits it into
//! `heightfield`, `props`, `noise` and `generator` modules.

pub mod spike_w;
pub mod strip;
