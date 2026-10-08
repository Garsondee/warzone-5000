//! `w5k_math`: the small maths library every other crate builds on.
//!
//! Plain `f64` with **strict discipline** (docs/architecture/DETERMINISM.md): every transcendental function comes from the
//! `libm` crate (pure Rust, identical on every platform) because the standard library's `f64::sin` and friends call the
//! platform's maths library and may differ in the last bit between Linux and Windows. Clippy bans the standard ones
//! (`clippy.toml`), so using them by accident fails CI. `sqrt`, `abs`, `min`, `max`, `floor`, `ceil` are exactly rounded by the
//! IEEE standard and are fine.
//!
//! Conventions (docs/architecture/UNITS-AND-FRAMES.md): SI units; right-handed; **+Y up, -Z forward, +X right**; quaternions
//! are stored w-x-y-z. Positive yaw (about +Y) turns the nose to the **left**; positive pitch (about +X) raises the nose;
//! positive roll (about the forward axis) lowers the **right** side. The tests in `quat.rs` pin all three.

pub mod hash;
pub mod mat3;
pub mod quat;
pub mod rng;
pub mod scalar;
pub mod transform;
pub mod vec3;

pub use hash::StateHasher;
pub use mat3::Mat3;
pub use quat::Quat;
pub use rng::Pcg32;
pub use transform::Transform;
pub use vec3::Vec3;
