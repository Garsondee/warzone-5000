//! Stand-ins for every neighbour, so each lane can start on day one and the integration spine runs before any real physics exists.
//!
//! * [`world`]: `FlatPlane`, `BumpStrip` (a `WorldQuery`), `standard_materials()`.
//! * [`script`]: `ScriptedCommands` (a driver), `ConstantTorquePowertrain` (a `DrivePort`).
//! * [`vehicle`]: `RigidBoxVehicle` (a kinematic `VehicleModel`: it drives, steers, rides bumps and swings a turret, with no real
//!   physics; it exists so viewers and the AI have something that moves).
//! * [`rigs`]: `box_truck()` and `box_tank()` (a valid `PhysRig` + `RenderRig` pair each, with the full node chains
//!   hull > travel > steer > wheel and hull > turret > gun > recoil), and `dummy_vehicle_def()`.
//! * [`replay`]: `truck_over_bumps()` and `tank_slew_and_pitch()` (canned replays) and the small runner that makes them.
//!
//! Nothing here is physics. Real lanes replace these one by one; the first-light scenario keeps passing as they do.

pub mod replay;
pub mod rigs;
pub mod script;
pub mod vehicle;
pub mod world;

pub use replay::{run_stand_in, tank_slew_and_pitch, truck_over_bumps};
pub use rigs::{box_tank, box_truck, dummy_vehicle_def};
pub use script::{ConstantTorquePowertrain, ScriptedCommands};
pub use vehicle::RigidBoxVehicle;
pub use world::{standard_materials, BumpStrip, FlatPlane};
