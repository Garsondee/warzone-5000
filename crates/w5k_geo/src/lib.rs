//! `w5k_geo`: lane GEOMETRY.
//!
//! Realistic hull, turret, wheel and track-link shape generators, tagged by articulation node.
//!
//! Status: SKELETON, created by the Launch Kit. Read your lane brief first:
//! `docs/swarm/lanes/geometry.md`. Only lane GEOMETRY edits this crate (`docs/swarm/ownership.toml`).
//!
//! Settling round: the indexed triangle `mesh`, the `edge` flag, a `bvh` and the `cavity` bake (spike S-G, `docs/lanes/geometry/spike-g.md`).

pub mod budget;
pub mod bvh;
pub mod cavity;
pub mod edge;
pub mod export;
pub mod flags;
pub mod gear;
pub mod hardware;
pub mod loft;
pub mod mass;
pub mod mesh;
pub mod module;
pub mod mount;
pub mod part;
pub mod raster;
pub mod skin;
pub mod truck;
pub mod weapon;
pub mod wheel;
