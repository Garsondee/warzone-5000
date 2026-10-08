//! Ports: the seams between the three physics lanes. They exist so CHASSIS, DRIVE and TRACKS can each build and test their half
//! against a stand-in for the other halves, then plug together without anyone editing anyone else's crate.
//!
//! * [`DrivePort`]: powertrain (DRIVE) <-> running gear (CHASSIS, TRACKS). Speeds go in, torques come out.
//! * [`SuspensionElement`]: a spring/damper/bump-stop at one station (CHASSIS), used by the glue for wheels and by TRACKS for road wheels.
//! * [`ContactElement`]: a patch of tyre or track on the ground (CHASSIS for tyres, TRACKS for belt samples).
//! * [`ArticulationPort`]: the turret, gun and recoil mechanisms (COMBAT implements it; the glue calls it).
//!
//! Sign conventions: positive shaft speed and torque drive the vehicle **forward**; forces on the vehicle are positive along
//! +X (right), +Y (up) and -Z; in a **contact frame** x is forward, **y is left** and z is up (right-handed, like the yaw convention); compression is positive.

use w5k_math::{Quat, StateHasher, Vec3};

use crate::command::{Command, GearRequest};
use crate::world::Material;

// ---------------------------------------------------------------------------------------------------------------- DrivePort

/// What the driver asks of the powertrain this step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DriveInputs {
    pub throttle: f64,
    pub brake: f64,
    /// -1 left .. +1 right. Wheeled vehicles ignore it (steering is on the stations); tracked steering units consume it.
    pub steer: f64,
    pub gear: GearRequest,
    pub parking_brake: bool,
    /// Clutch pedal of a manual gearbox, 0 (engaged) to 1 (depressed); `None` = automatic.
    pub clutch: Option<f64>,
    /// The selected driveline state (index into `DrivetrainDef::modes`).
    pub drive_mode: u8,
}

/// The state of one driven output shaft (wheel hub or sprocket) as the powertrain sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShaftState {
    /// Angular speed, rad/s (positive = forward).
    pub omega_rad_s: f64,
    /// Rotational inertia of everything on the shaft (wheel, tyre, hub, track drive share), kg m^2.
    pub inertia_kg_m2: f64,
    /// Vehicle speed, m/s (brakes cool with airflow).
    pub vehicle_speed_m_s: f64,
    /// The load the running gear puts on this shaft: the reaction torque of the last substep (positive resists forward rolling), N m.
    pub load_torque_nm: f64,
    /// How that load changes with shaft speed (N m per rad/s), so a locked driveline can be solved implicitly.
    pub load_stiffness_nm_s_rad: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DriveTelemetry {
    pub engine_rpm: f64,
    /// 0 neutral, positive forward, negative reverse.
    pub gear: i8,
    pub engine_torque_nm: f64,
    pub fuel_rate_kg_s: f64,
    /// Clutch or converter slip speed, rad/s.
    pub coupling_slip_rad_s: f64,
    /// Brake disc temperatures, K, one per brake.
    pub brake_temps_k: Vec<f64>,
    pub shifting: bool,
    /// The selected driveline state.
    pub drive_mode: u8,
}

/// Powertrain port. One call per substep; `torque_nm_out` is filled with the net torque (drive minus brake) each output shaft
/// receives. Implementations must never produce a torque that reverses a stopped shaft by braking alone.
pub trait DrivePort {
    /// Number of driven output shafts (matches `DrivetrainDef::outputs`).
    fn output_count(&self) -> usize;
    fn step(&mut self, dt_s: f64, inputs: &DriveInputs, shafts: &[ShaftState], torque_nm_out: &mut [f64]);
    fn telemetry(&self) -> DriveTelemetry;
    /// Feed the state into a hasher (golden tests).
    fn hash_state(&self, h: &mut w5k_math::StateHasher);
}

// ----------------------------------------------------------------------------------------------------- SuspensionElement

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuspensionOut {
    /// Force along the strut axis, N; positive pushes the wheel and the hull apart.
    pub force_n: f64,
    /// Spring part of `force_n` (for the ledger).
    pub spring_n: f64,
    /// Damper part of `force_n`.
    pub damper_n: f64,
    /// Bump-stop part of `force_n`.
    pub bump_stop_n: f64,
    /// True when the travel limit was reached this step.
    pub at_limit: bool,
}

pub trait SuspensionElement {
    /// `compression_m` is positive when compressed from the rest position; `rate_m_s` positive while compressing.
    fn step(&mut self, compression_m: f64, rate_m_s: f64, dt_s: f64) -> SuspensionOut;
}

// ---------------------------------------------------------------------------------------------------------- ContactElement

/// A contact patch's kinematics for one substep, in the *contact frame*: x along the rolling direction (forward positive),
/// y lateral (**left** positive, so the frame is right-handed), z along the surface normal (up positive). A positive self-aligning moment
/// about z turns the nose left; the slip angle is positive when the velocity points left of the wheel's heading.
#[derive(Clone, Copy, Debug)]
pub struct ContactInput<'a> {
    /// Penetration of the undeformed patch into the ground, m (>= 0 when touching; 0 or negative = lifted off).
    pub penetration_m: f64,
    pub penetration_rate_m_s: f64,
    /// Velocity of the patch over the ground, m/s.
    pub vel_long_m_s: f64,
    pub vel_lat_m_s: f64,
    /// Surface speed of the rolling member at the patch (wheel `omega * radius`, or belt speed), m/s, forward positive.
    pub surface_speed_m_s: f64,
    pub ground: &'a Material,
    pub dt_s: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContactOutput {
    /// Forces on the vehicle at the patch, contact frame, N.
    pub fx_n: f64,
    pub fy_n: f64,
    pub fz_n: f64,
    /// Self-aligning moment about the surface normal, N m.
    pub mz_nm: f64,
    /// Reaction torque on the rolling member's shaft from the longitudinal force, N m (positive resists forward rolling).
    pub shaft_reaction_nm: f64,
    /// Sinkage into soft ground, m (0 on rigid ground).
    pub sinkage_m: f64,
    /// Normalised slip, -1..1: `(v_wheel - v_ground) / max(|v_wheel|, |v_ground|, eps)`; positive when the wheel is faster than the ground (driving),
    /// -1 when it is locked and sliding. (For a track sample driving, this is `1 - v / v_belt`.)
    pub slip_ratio: f64,
    pub slip_angle_rad: f64,
    /// True when the patch is saturated at the friction (or soil shear) limit.
    pub saturated: bool,
}

pub trait ContactElement {
    fn step(&mut self, input: &ContactInput) -> ContactOutput;
    /// Forget the internal state (relaxation, sinkage history), e.g. after a teleport.
    fn reset(&mut self);
}

// ------------------------------------------------------------------------------------------------------ ArticulationPort

/// The motion of the hull the articulation reacts against, world frame, at the start of a substep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HullMotion {
    pub rot: Quat,
    pub pos_m: Vec3,
    pub vel_m_s: Vec3,
    pub acc_m_s2: Vec3,
    pub omega_rad_s: Vec3,
    pub alpha_rad_s2: Vec3,
}

/// The reaction of the articulated bodies on the hull for this substep: a force at the hull's centre of mass and a torque about it (world frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArticulationWrench {
    pub force_n: Vec3,
    pub torque_about_hull_com_nm: Vec3,
}

/// A shell leaving a muzzle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub muzzle: u32,
    pub origin_m: Vec3,
    pub dir: Vec3,
    /// Velocity of the muzzle point (hull motion plus joint rates): the shell inherits it.
    pub muzzle_point_vel_m_s: Vec3,
}

/// Turret, gun and recoil mechanisms (COMBAT implements it; the glue `w5k_vehicle` calls it once per substep). Servos follow the aim channels of
/// the `Command`; the returned wrench is what the moving bodies do to the hull, so momentum is conserved.
pub trait ArticulationPort {
    fn step(&mut self, dt_s: f64, hull: &HullMotion, cmd: &Command, wrench: &mut ArticulationWrench);
    /// Joint coordinates (rad or m), in `PhysRig::articulation` order.
    fn joint_positions(&self) -> &[f64];
    /// Move the shots fired since the last call into `out`.
    fn drain_shots(&mut self, out: &mut Vec<Shot>);
    fn hash_state(&self, h: &mut StateHasher);
}
