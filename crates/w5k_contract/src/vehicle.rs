//! `VehicleModel`: what the simulation steps. The stand-in `testing::RigidBoxVehicle` implements it today; `w5k_vehicle` (ARCH's
//! glue over CHASSIS, DRIVE and TRACKS) implements it for real. Everything the scheduler, the validation harness and the AI know about
//! a vehicle is on this trait.

use serde::{Deserialize, Serialize};
use w5k_math::StateHasher;

use crate::command::Command;
use crate::frame::{Event, VehicleFrame};
use crate::ledger::ForceLedger;
use crate::rig::PhysRig;
use crate::world::WorldQuery;

/// What limited the vehicle this step: the explanation behind "why did it not climb / accelerate / turn".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LimitingFactor {
    #[default]
    None,
    /// Engine power or torque (or the governor).
    Power,
    /// Tyre or track grip (friction limit).
    Grip,
    /// Soil shear strength (soft ground gave out).
    Soil,
    /// Sinkage: the hull is dragging.
    Sinkage,
    /// Brakes (or thermal fade of the brakes).
    Brake,
    /// Suspension travel (bottomed out, topped out).
    Suspension,
    /// Stalled or bogged (not moving under full throttle).
    Stuck,
    /// Rolled over or upside down.
    Overturned,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepReport {
    pub limiting: LimitingFactor,
    /// Speed over the ground, m/s.
    pub speed_m_s: f64,
    /// Substeps taken (diagnostics).
    pub substeps: u32,
}

pub trait VehicleModel {
    fn rig(&self) -> &PhysRig;
    /// Advance by `dt_s` (one 60 Hz tick, split into the rig's substeps by the implementation).
    fn step(&mut self, dt_s: f64, cmd: &Command, world: &dyn WorldQuery, ledger: &mut ForceLedger) -> StepReport;
    /// The current snapshot, with `vehicle` set to 0 (the scheduler renumbers).
    fn frame(&self) -> VehicleFrame;
    fn hash_state(&self, h: &mut StateHasher);
    /// Move the events raised since the last call (a shot fired, a rollover, ...) into `out`. The scheduler puts them in the next frame.
    fn drain_events(&mut self, _out: &mut Vec<Event>) {}
    /// Replace the rig with a numbers-only variant of it (damage: `degrade()` returns one). The topology must match; the dynamic state is kept.
    fn swap_rig(&mut self, _rig: PhysRig) {}
}
