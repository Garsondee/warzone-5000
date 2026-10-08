//! `Frame` and the replay header: the interface between the simulation and every viewer (the three.js reference viewer, the Godot
//! player, the validation harness). A viewer is a pure function of a header plus a sequence of frames.
//!
//! These are the *data model*; lane VIEWER owns the compact encoding (`w5k_replay`: quantised, ~30 Hz, viewers interpolate).

use serde::{Deserialize, Serialize};
use w5k_math::{Quat, Vec3};

/// One simulation snapshot of every vehicle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub t_s: f64,
    pub vehicles: Vec<VehicleFrame>,
    /// Events since the previous frame.
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleFrame {
    /// Index into `ReplayHeader::vehicles`.
    pub vehicle: u32,
    pub pos_m: Vec3,
    pub rot: Quat,
    pub lin_vel_m_s: Vec3,
    pub ang_vel_rad_s: Vec3,
    /// Joint coordinates in the order of `VehicleHeader::joint_names`: wheel spin and steer (rad), suspension travel (m), turret
    /// yaw, gun pitch (rad), recoil (m).
    pub joints: Vec<f32>,
    pub engine_rpm: f32,
    /// Engaged gear: 0 neutral, positive forward, negative reverse.
    pub gear: i8,
    pub contacts: Vec<ContactFrame>,
    /// A compact summary of the force ledger for this frame: one value per `ForceTerm` (N, magnitude of the net force by term).
    #[serde(default)]
    pub ledger_n: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactFrame {
    /// Bit 0: touching ground; bit 1: slipping; bit 2: on soft ground; bit 3: bottomed out.
    pub flags: u8,
    pub normal_force_n: f32,
    pub sinkage_m: f32,
    pub slip: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Fired { vehicle: u32, muzzle: u32 },
    Hit { target: String, damage: f32 },
    Destroyed { what: String },
    Stuck { vehicle: u32, cause: String },
    Rollover { vehicle: u32 },
    BottomedOut { vehicle: u32, station: u32 },
    Note { text: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub version: u32,
    pub scenario: String,
    /// Seconds between frames (constant within a replay).
    pub frame_dt_s: f64,
    pub vehicles: Vec<VehicleHeader>,
    pub world: WorldHeader,
    /// Hash chain of the simulation state, one entry per simulated second, for regression tests.
    #[serde(default)]
    pub state_hashes: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleHeader {
    pub name: String,
    /// Id of the `PhysRig` / `RenderRig` pair that was simulated.
    pub rig_id: String,
    pub joint_names: Vec<String>,
    pub contact_names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldHeader {
    pub course: String,
    pub seed: u64,
}

pub const REPLAY_VERSION: u32 = 2;

impl VehicleFrame {
    pub fn is_finite(&self) -> bool {
        self.pos_m.is_finite()
            && self.rot.is_finite()
            && self.lin_vel_m_s.is_finite()
            && self.ang_vel_rad_s.is_finite()
            && self.joints.iter().all(|j| j.is_finite())
            && self.engine_rpm.is_finite()
    }
}
