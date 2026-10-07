//! Deterministic maths for warzone-5000.
//!
//! Everything that decides gameplay outcomes uses these types so that every machine in a lockstep match
//! computes bit-identical results. See `docs/design/02-determinism-rules.md`.

pub mod fx;
pub mod hash;
pub mod rng;
pub mod vec;

pub use fx::Fx;
pub use hash::StateHasher;
pub use rng::Pcg32;
pub use vec::FxVec3;
