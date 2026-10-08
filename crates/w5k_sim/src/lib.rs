//! Deterministic simulation core.
//!
//! Everything in this crate follows `docs/design/02-determinism-rules.md`: fixed-point `Fx` state, a fixed 20 Hz tick,
//! no floats in anything that is simulated, and a state hash recorded as the run goes. Floats appear only where
//! content is *loaded* (`Course::bake`, `Fx::from_f64`), which is exact IEEE arithmetic and therefore reproducible.
//!
//! **Units.** The simulation uses a coherent unit system: tonnes, kilonewtons, kilowatts, metres and seconds.
//! 1 kN / 1 t = 1 m/s^2 and 1 kN x 1 m/s = 1 kW, so forces, powers and accelerations never need a conversion factor, and
//! values stay far inside the range of Q32.32 (joules and newtons would overflow it for a 1,000 tonne vehicle).
//!
//! **Course geometry.** A time trial runs along a course: distance `s` along the track, height `h(s)` and a surface
//! (concrete, soft earth...) per stretch. Positions are metres from the start line; the vehicle drives towards -Z in the
//! viewers, which is +s here.

pub mod course;
pub mod replay;
pub mod trial;

pub use course::{Course, CourseDef, SegmentDef};
pub use replay::{Cause, Frame, Outcome, Run, HZ};
