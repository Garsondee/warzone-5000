//! `w5k_combat`: lane COMBAT.
//!
//! Ballistics, penetration, damage, targets, turret and gun servos, recoil.
//!
//! Status: `ballistics` is built (slice 2). The rest waits for a later slice; see `docs/swarm/status/combat.md` and `docs/lanes/combat/design-note.md`.

pub mod ballistics;
