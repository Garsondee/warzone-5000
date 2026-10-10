//! `w5k_chassis`: lane CHASSIS.
//!
//! Hull dynamics, suspension, tyres, steering and aero: a wheeled vehicle that rides, grips and handles.
//!
//! Status: SKELETON, created by the Launch Kit. Read your lane brief first:
//! `docs/swarm/lanes/chassis.md`. Only lane CHASSIS edits this crate (`docs/swarm/ownership.toml`).

pub mod bench;
pub mod hull;
pub mod modes;
pub mod quarter_car;
pub mod soil_wheel;
pub mod steering;
pub mod suspension;
pub mod tuning;
pub mod tyre;
pub mod wheeled;
