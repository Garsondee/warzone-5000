//! `w5k_validate`: lane VALIDATION.
//!
//! Dossiers, validation harness, dashboard, design impact matrix, fuzz.
//!
//! Status: SKELETON, created by the Launch Kit. Read your lane brief first:
//! `docs/swarm/lanes/validation.md`. Only lane VALIDATION edits this crate (`docs/swarm/ownership.toml`).

pub mod dossier;

pub mod dashboard;
pub mod measure;
pub mod score;
pub mod verdict;

pub mod oracle;
pub mod proving;
