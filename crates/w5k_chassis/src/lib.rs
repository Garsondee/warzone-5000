//! `w5k_chassis`: lane CHASSIS.
//!
//! Hull dynamics, suspension, tyres, steering and aero: a wheeled vehicle that rides, grips and handles.
//!
//! Status: SKELETON, created by the Launch Kit. Read your lane brief first:
//! `docs/swarm/lanes/chassis.md`. Only lane CHASSIS edits this crate (`docs/swarm/ownership.toml`).

/// DELIBERATELY BAD (guard test, to be closed unmerged): a bare physical constant and a banned std maths call.
pub const SPRING_RATE_N_M: f64 = 31415.9;

pub fn bad_ride_frequency(mass_kg: f64) -> f64 {
    let ratio = (SPRING_RATE_N_M / mass_kg).powf(0.5);
    ratio / 6.2831853
}
