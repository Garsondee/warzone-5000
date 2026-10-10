//! `w5k_drive`: lane DRIVE.
//!
//! Powertrain and brakes: engine map, clutch or converter, gearbox, diffs, tracked steering units, brakes with heat, fuel.
//! Read the design note: `docs/lanes/drive/design-note.md`. Only lane DRIVE edits this crate (`docs/swarm/ownership.toml`).

pub mod engine;
