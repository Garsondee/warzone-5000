//! `Frame` and the replay header: the interface between the simulation and every viewer (the three.js reference viewer, the Godot
//! player, the validation harness). A viewer is a pure function of a header plus a sequence of frames.
//!
//! These are the *data model*; lane VIEWER owns the compact encoding (`w5k_replay`: quantised, ~30 Hz, viewers interpolate). The quantum of an
//! aim joint must be finer than the gun-laying error a plot shows (16 bits over 2 pi is 0.1 mrad, the minimum).

use serde::{Deserialize, Serialize};
use w5k_math::{Quat, Vec3};

use crate::vehicle::LimitingFactor;

/// One simulation snapshot of every vehicle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub t_s: f64,
    pub vehicles: Vec<VehicleFrame>,
    /// Events since the previous frame.
    #[serde(default)]
    pub events: Vec<Event>,
    /// Shells in flight (tracers).
    #[serde(default)]
    pub projectiles: Vec<ProjectileFrame>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VehicleFrame {
    /// Index into `ReplayHeader::vehicles`.
    pub vehicle: u32,
    /// Hull datum position and orientation (body to world), world frame.
    pub pos_m: Vec3,
    pub rot: Quat,
    /// Velocity of the hull's centre of mass and angular velocity, **world frame**.
    pub lin_vel_m_s: Vec3,
    pub ang_vel_rad_s: Vec3,
    /// Joint coordinates in the order of `VehicleHeader::joint_names`: wheel spin and steer (rad), suspension travel (m), turret
    /// yaw, gun pitch (rad), recoil (m). **Revolute coordinates are continuous (never wrapped): viewers interpolate them linearly.**
    pub joints: Vec<f32>,
    pub engine_rpm: f32,
    /// Engaged gear: 0 neutral, positive forward, negative reverse.
    pub gear: i8,
    /// One entry per contact patch of a tyred station, then one per sample of each track, in `PhysRig::contact_names()` order.
    pub contacts: Vec<ContactFrame>,
    /// A compact summary of the force ledger for this frame: one value per `ForceTerm` (N, magnitude of the net force by term).
    #[serde(default)]
    pub ledger_n: Vec<f32>,
    /// What limited the vehicle this frame (the explanation behind "why did it not climb").
    #[serde(default)]
    pub limiting: LimitingFactor,
    /// State of each weapon (HUD, AI), in `PhysRig::muzzles` order.
    #[serde(default)]
    pub weapons: Vec<WeaponFrame>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactFrame {
    /// Bit 0: touching ground; bit 1: slipping; bit 2: on soft ground; bit 3: bottomed out.
    pub flags: u8,
    pub normal_force_n: f32,
    pub sinkage_m: f32,
    /// Slip ratio, normalised to -1..1 (see `ContactOutput::slip_ratio`).
    pub slip: f32,
    /// The ground material under the patch (`MaterialId`): dust, mud spray and tracks look different on asphalt, gravel and mud.
    #[serde(default)]
    pub material: u16,
}

/// A weapon's state in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeaponFrame {
    pub ready: bool,
    pub reload_s: f32,
    pub rounds: u16,
    /// Angle between the bore and the sight line, rad (the gun-laying error).
    pub aim_error_rad: f32,
}

/// A shell in flight.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectileFrame {
    pub id: u32,
    pub pos_m: Vec3,
    pub vel_m_s: Vec3,
}

/// Things that happened since the last frame. New variants are added at the end (the enum is `#[non_exhaustive]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Event {
    Fired {
        vehicle: u32,
        muzzle: u32,
    },
    /// A shell hit something: a vehicle (`target` = its index in the header) or a prop.
    Hit {
        target: u32,
        point: Vec3,
        normal: Vec3,
        penetrated: bool,
        /// The module that took the damage, if the shell reached one (index into `CombatDef::modules`).
        module: Option<u32>,
        energy_j: f32,
    },
    Destroyed {
        what: String,
    },
    Stuck {
        vehicle: u32,
        cause: String,
    },
    Rollover {
        vehicle: u32,
    },
    BottomedOut {
        vehicle: u32,
        station: u32,
    },
    Note {
        text: String,
    },
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
    /// Names of the entries of `VehicleFrame::contacts` (`PhysRig::contact_names()`).
    pub contact_names: Vec<String>,
    /// Paint scheme and wear of this vehicle (several vehicles may share a rig and differ in livery). `None` = the viewer's default.
    #[serde(default)]
    pub livery: Option<Livery>,
}

/// How one vehicle is painted: a camo scheme from LOOK's catalogue, the seed that places its blobs, and the amount of wear 0..1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Livery {
    pub camo: String,
    pub seed: u64,
    pub wear: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldHeader {
    pub course: String,
    pub seed: u64,
    /// Where the terrain export of this world is (`w5k world export`), relative to the replay: lets a viewer draw the ground. `None` = flat.
    #[serde(default)]
    pub terrain: Option<String>,
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
