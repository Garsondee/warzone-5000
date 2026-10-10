//! `w5k_terramech`: lane TRACKS.
//!
//! Tracked running gear and soft ground: Bekker and Wong, skid-steer, sprocket coupling. Read `docs/swarm/lanes/tracks.md` and
//! `docs/theory/tracks.md`. Only lane TRACKS edits this crate (`docs/swarm/ownership.toml`).

pub mod belly;
pub mod gear;
pub mod ladder;
pub mod plan;
pub mod reference;
pub mod sample;
pub mod soil;
pub mod tuning;
pub mod vehicle;

pub use tuning::{TracksTuning, Tuning};
