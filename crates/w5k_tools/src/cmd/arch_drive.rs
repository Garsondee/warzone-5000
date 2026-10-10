//! `w5k drive --vehicle FILE.ron --course FILE.ron [--vehicles-dir DIR] [--web DIR] [--port N] [--open] [--no-assist] [--record DIR]`:
//! the live test-drive server (ARCH). The real simulation (FORGE rig, DRIVE powertrain, CHASSIS vehicle, WORLD course) runs in real time on
//! its own thread; a tiny loopback HTTP server serves a prebuilt page and a small JSON API (`docs/swarm/requests/arch-drive-protocol.md`).
//! A five-year-old drives with arrow keys, a gamepad or big buttons; the kid assists live in `content/physics/arch/drive_assist.ron`.
//!
//! Stage 1 of the series: the headless session (scene, garage, assists as raw pass-through, stream frame); the loop and the server follow.
//!
//! This is tooling: it paces the simulation against the wall clock, which the simulation crates may not (the results of a tick never
//! depend on it).
#![allow(clippy::disallowed_methods)] // std::time::Instant::now paces the loop; no simulated quantity reads it
#![allow(dead_code)] // temporary: wired up by the later changes of this series (live loop, server)

mod assist;
mod session;
