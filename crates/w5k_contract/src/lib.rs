//! `w5k_contract`: the interfaces every lane builds against (docs/architecture/CONTRACTS.md is the human-readable version).
//!
//! This crate holds **data and traits only**: no physics, no rendering. Producers and consumers of each contract are listed in
//! CONTRACTS.md. Lanes pin a tagged version of this crate (`contract-vX.Y`); breaking it is a *contract change request* (CCR).
//!
//! Naming rule for every physical field: the unit is the suffix (`mass_kg`, `length_m`, `torque_nm`, `power_w`, `angle_rad`,
//! `omega_rad_s`, `rate_n_m` for N/m). SI throughout; km/h and kW exist only at the edges (display, spec sheets).
//!
//! | Module | Contract |
//! |---|---|
//! | [`param`] | `Param`: a constant with its pedigree (SPEC / MEASURED / ESTIMATE / TUNED) |
//! | [`def`] | `VehicleDef`: the designer-level description (RON) |
//! | [`rig`] | `PhysRig`: the solver-level description compiled from a `VehicleDef` |
//! | [`render`] | `RenderRig`: meshes tagged by articulation node |
//! | [`command`] | `Command`: what a driver (human, AI, script) asks of a vehicle |
//! | [`frame`] | `Frame` and the replay header: the interface between the simulation and every viewer |
//! | [`world`] | `WorldQuery`, `Material`, `MaterialTable`: terrain, soil and props |
//! | [`ports`] | `DrivePort`, `SuspensionElement`, `ContactElement`: the seams between the physics lanes |
//! | [`ledger`] | `ForceLedger`: every force term, every tick, so any outcome is explainable |
//! | [`vehicle`] | `VehicleModel`: the stepping interface the simulation drives |
//! | [`capability`] | `CapabilityTable`: what a vehicle can do, for the AI |
//! | [`testing`] | stand-ins for every neighbour, so each lane can start on day one |

pub mod capability;
pub mod combat;
pub mod command;
pub mod def;
pub mod frame;
pub mod kinematics;
pub mod ledger;
pub mod param;
pub mod ports;
pub mod render;
pub mod rig;
pub mod testing;
mod validate;
pub mod vehicle;
pub mod world;

pub use capability::CapabilityTable;
pub use command::{AimDemand, AimFrame, Command, GearRequest, AIM_CHANNELS};
pub use frame::{ContactFrame, Event, Frame, ReplayHeader, VehicleFrame, VehicleHeader};
pub use ledger::{ForceLedger, ForceTerm};
pub use param::{Param, Provenance};
pub use ports::{
    ContactElement, ContactInput, ContactOutput, DriveInputs, DrivePort, DriveTelemetry, ShaftState, SuspensionElement,
    SuspensionOut, DEFAULT_AMBIENT_K,
};
pub use render::RenderRig;
pub use rig::PhysRig;
pub use vehicle::{LimitingFactor, StepReport, VehicleModel};
pub use world::{GroundSample, Material, MaterialId, MaterialTable, PropRef, RayHit, SoilParams, WorldQuery};

/// The version of this contract. Bump the minor for additive changes, the major for breaking ones; tag the commit `contract-vX.Y`.
pub const CONTRACT_VERSION: &str = "0.2.0";

/// Density of air at sea level, 15 C (ISA), in kg/m^3: a defined reference value.
pub const AIR_DENSITY_KG_M3: f64 = 1.225; // const-ok: ISA sea-level standard atmosphere, a defined reference value
