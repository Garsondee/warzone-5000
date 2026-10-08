//! `PhysRig`: the solver-level description of a vehicle, compiled by lane FORGE from a [`crate::def::VehicleDef`] and consumed by
//! the dynamics lanes (CHASSIS, DRIVE, TRACKS), COMBAT and the AI. Plain numbers in SI: no sliders, no pedigree (that lives in the
//! `VehicleDef`).
//!
//! Structure (reduced coordinates, not a general multibody): one 6-DoF **hull**; a list of **stations** (a wheel or road wheel
//! on its own suspension travel, optionally steered, optionally driven); optional **tracks** that loop over stations; a
//! **drivetrain**; an **articulation** tree (turret yaw, gun pitch, recoil slide) hanging off the hull; aero; collision proxies;
//! muzzle frames. Frame: hull frame, +X right, +Y up, -Z forward, origin at the design datum.

use serde::{Deserialize, Serialize};
use w5k_math::{Mat3, Transform, Vec3};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysRig {
    pub id: String,
    /// Sprung hull: everything rigidly attached above the suspension, including the unsprung-mass-free share of crew and fuel.
    pub hull: BodyDef,
    pub stations: Vec<StationDef>,
    #[serde(default)]
    pub anti_roll: Vec<AntiRollDef>,
    #[serde(default)]
    pub tracks: Vec<TrackDef>,
    pub drivetrain: DrivetrainDef,
    /// Turret / gun / recoil chain. Parents precede children.
    #[serde(default)]
    pub articulation: Vec<JointDef>,
    pub aero: AeroDef,
    pub proxies: Vec<CollisionProxy>,
    #[serde(default)]
    pub muzzles: Vec<MuzzleDef>,
    pub integration: IntegrationDef,
}

/// A rigid mass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyDef {
    pub name: String,
    pub mass_kg: f64,
    /// Centre of mass in the parent frame (hull datum for the hull, the joint frame for articulated bodies), m.
    pub com_m: Vec3,
    /// Inertia tensor about the centre of mass, in the body's axes, kg m^2.
    pub inertia_kg_m2: Mat3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
    Centre,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WheelKind {
    /// A pneumatic or solid tyre that touches the ground itself.
    Tyre,
    /// A track road wheel (loads the track; the track touches the ground).
    RoadWheel,
    Sprocket,
    Idler,
    ReturnRoller,
}

/// One wheel station: the wheel, its suspension, and how it is steered and driven.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationDef {
    pub name: String,
    pub side: Side,
    /// Axle index from the front, 0-based (wheeled vehicles) or road-wheel index (tracked).
    pub axle: u8,
    /// Wheel centre at the design ride height, hull frame, m.
    pub rest_pos_m: Vec3,
    /// Unit vector (hull frame) along which the wheel centre moves when the suspension compresses (normally +Y).
    pub bump_dir: Vec3,
    /// Available travel from the rest position: toward the hull (bump) and away (droop), m.
    pub bump_travel_m: f64,
    pub droop_travel_m: f64,
    pub unsprung_mass_kg: f64,
    pub suspension: SuspensionDef,
    pub steer: Option<SteerDef>,
    pub wheel: WheelDef,
    /// Index into `drivetrain.outputs` if this station is driven.
    pub drive_output: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WheelDef {
    pub kind: WheelKind,
    pub radius_m: f64,
    pub width_m: f64,
    /// Rotational inertia of wheel, tyre, hub and brake disc about the axle, kg m^2.
    pub inertia_kg_m2: f64,
    /// Present for `WheelKind::Tyre`.
    pub tyre: Option<TyreDef>,
}

/// Tyre model parameters (a deliberately small set; lane CHASSIS extends it by CCR).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TyreDef {
    pub vertical_stiffness_n_m: f64,
    pub vertical_damping_ns_m: f64,
    /// Peak friction coefficient on the reference (dry, hard) surface; scaled by `Material::mu_peak` at run time.
    pub mu_peak_ref: f64,
    /// Longitudinal slip stiffness: force per unit load per unit slip ratio at zero slip (dimensionless).
    pub slip_stiffness: f64,
    /// Cornering stiffness: lateral force per unit load per radian of slip angle (1/rad).
    pub cornering_stiffness_per_rad: f64,
    /// Relaxation length: the distance a tyre rolls for its force to build up (m); also what tames low-speed jitter.
    pub relaxation_length_m: f64,
    pub rolling_coeff: f64,
    /// Inflation pressure, Pa (sets the contact patch area on soft ground).
    pub inflation_pa: f64,
    /// Contact patch length at rated load, m.
    pub patch_length_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteerDef {
    /// Maximum steering angle of this wheel, rad (positive = left, as for any yaw).
    pub max_angle_rad: f64,
    /// 0 = parallel steer, 1 = full Ackermann geometry.
    pub ackermann: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuspensionDef {
    pub spring: SpringKind,
    /// Force at the rest position (the static load share), N, along `bump_dir`.
    pub preload_n: f64,
    pub damper: DamperDef,
    pub bump_stop: BumpStopDef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpringKind {
    /// No suspension (a solid mount, a sprocket).
    Rigid,
    /// A constant wheel rate (coil, or any linearised spring), N/m at the wheel.
    Linear { rate_n_m: f64 },
    /// A tabulated force against compression from rest (leaf, rubber, air), `(compression_m, force_n)` ascending.
    Table { points: Vec<(f64, f64)> },
    /// A torsion bar acting through a road-wheel arm: the wheel rate follows from the arm geometry.
    Torsion { rate_nm_rad: f64, arm_length_m: f64, rest_arm_angle_rad: f64 },
    /// A hydropneumatic strut: a polytropic gas spring `p V^gamma = const` acting on a piston.
    Hydropneumatic { gas_pressure_pa: f64, gas_volume_m3: f64, piston_area_m2: f64, gamma: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DamperDef {
    /// Damping coefficient in compression and in rebound, N s/m.
    pub bump_ns_m: f64,
    pub rebound_ns_m: f64,
    /// Above this piston speed the damper becomes digressive (m/s); 0 disables the knee.
    pub knee_speed_m_s: f64,
    /// Fraction of the coefficient that applies above the knee (1 = linear).
    pub post_knee_ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BumpStopDef {
    /// Compression beyond which the bump stop acts, measured from the rest position, m.
    pub engage_m: f64,
    /// Stiffness once engaged, N/m (progressive: multiplied by `1 + progression * penetration / engage`).
    pub rate_n_m: f64,
    pub progression: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AntiRollDef {
    pub left_station: usize,
    pub right_station: usize,
    /// Force per metre of differential travel, N/m.
    pub rate_n_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackDef {
    pub name: String,
    pub side: Side,
    /// Station indices in belt order, front to back (idler, road wheels, sprocket).
    pub stations: Vec<usize>,
    pub sprocket: usize,
    pub idler: usize,
    pub width_m: f64,
    pub pitch_m: f64,
    /// Design length of belt in contact with firm ground, m.
    pub contact_length_m: f64,
    pub mass_per_m_kg: f64,
    /// Contact samples along the footprint (8-16 is the working range).
    pub samples: u16,
    /// Shoe friction relative to rubber on the same surface (1 = rubber pads, >1 = grousers on soft ground, <1 = steel on road).
    pub shoe_mu_scale: f64,
    pub tension_n: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrivetrainDef {
    pub engine: EngineDef,
    pub coupling: CouplingDef,
    pub gearbox: GearboxDef,
    /// Torque distribution from the gearbox output to the driven shafts.
    pub driveline: DriveNode,
    pub outputs: Vec<OutputDef>,
    pub brakes: Vec<BrakeDef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    Petrol,
    Diesel,
    GasTurbine,
    Electric,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineDef {
    pub kind: EngineKind,
    /// Full-load torque against speed, `(rpm, N m)` ascending in rpm.
    pub torque_curve: Vec<(f64, f64)>,
    pub idle_rpm: f64,
    pub redline_rpm: f64,
    /// Rotational inertia of crankshaft, flywheel and clutch cover, kg m^2.
    pub inertia_kg_m2: f64,
    /// Closed-throttle drag (engine braking): `drag_const_nm + drag_per_rpm_nm * rpm`.
    pub drag_const_nm: f64,
    pub drag_per_rpm_nm: f64,
    /// Fuel consumption at the best point, g/kWh (BSFC); the fuel map is derived around it.
    pub bsfc_best_g_kwh: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CouplingDef {
    Direct,
    Clutch { max_torque_nm: f64, engage_rpm: f64 },
    TorqueConverter { stall_ratio: f64, k_factor_rpm_per_sqrt_nm: f64, lockup_speed_ratio: Option<f64> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GearboxDef {
    /// Forward ratios, first gear first (input speed / output speed).
    pub forward_ratios: Vec<f64>,
    pub reverse_ratios: Vec<f64>,
    pub efficiency: f64,
    pub inertia_kg_m2: f64,
    pub shift: ShiftDef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShiftDef {
    pub automatic: bool,
    pub upshift_rpm: f64,
    pub downshift_rpm: f64,
    pub shift_time_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    Open,
    Locked,
    /// Torque-sensing / limited slip with a bias ratio (see `bias`).
    LimitedSlip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteerUnitKind {
    ClutchBrake,
    ControlledDifferential,
    DoubleDifferential,
    Hydrostatic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DriveNode {
    /// A differential (axle, centre, transfer case) splitting torque between its children.
    Diff { kind: DiffKind, ratio: f64, bias: f64, children: Vec<DriveNode> },
    /// The steering unit of a tracked vehicle: a driven pair with a steering demand mixed in. `children` = [left, right].
    SteerUnit { kind: SteerUnitKind, ratio: f64, children: Vec<DriveNode> },
    /// A driven shaft: index into `DrivetrainDef::outputs`.
    Output(usize),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputDef {
    /// The station this shaft drives (wheel hub or sprocket).
    pub station: usize,
    /// Final-drive (hub reduction) ratio.
    pub final_drive_ratio: f64,
    pub efficiency: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrakeDef {
    pub station: usize,
    pub max_torque_nm: f64,
    pub thermal_mass_j_k: f64,
    /// Convective cooling, W/K, at rest and per m/s of vehicle speed.
    pub cooling_w_k: f64,
    pub cooling_per_ms_w_k: f64,
    /// Temperature range over which friction fades from full to `fade_floor`, K.
    pub fade_start_k: f64,
    pub fade_end_k: f64,
    pub fade_floor: f64,
    pub parking: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AeroDef {
    pub drag_coeff: f64,
    pub frontal_area_m2: f64,
    /// Where the drag acts, hull frame (height of the centre of pressure matters for pitch).
    pub centre_of_pressure_m: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointKind {
    Revolute,
    Prismatic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointRole {
    TurretYaw,
    GunPitch,
    Recoil,
    Other,
}

/// A joint in the articulation tree and the body it carries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointDef {
    pub name: String,
    pub role: JointRole,
    /// Index of the parent joint in `articulation`, or `None` for the hull.
    pub parent: Option<usize>,
    pub kind: JointKind,
    /// Joint origin and axis in the parent's frame.
    pub anchor_m: Vec3,
    pub axis: Vec3,
    /// Motion limits, rad or m, `(min, max)`; `None` = unlimited (turret ring).
    pub limits: Option<(f64, f64)>,
    pub body: BodyDef,
    pub servo: Option<ServoDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServoDef {
    pub max_rate: f64,
    pub max_accel: f64,
    /// Peak drive torque (N m) or force (N).
    pub max_effort: f64,
    pub kp: f64,
    pub kd: f64,
    /// Stabiliser: fraction of hull motion rejected (0 = none, 1 = perfect) and its bandwidth, Hz.
    pub stabiliser_rejection: f64,
    pub stabiliser_bandwidth_hz: f64,
    /// For a recoil slide: spring rate, damper rate, stroke.
    pub recoil: Option<RecoilDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoilDef {
    pub spring_n_m: f64,
    pub damper_ns_m: f64,
    pub stroke_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProxyShape {
    Sphere { radius_m: f64 },
    Capsule { a: Vec3, b: Vec3, radius_m: f64 },
    Box { half_m: Vec3 },
}

/// A collision shape the physics uses against terrain and props (cheap, convex, a few per vehicle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollisionProxy {
    pub name: String,
    pub shape: ProxyShape,
    /// Pose in the frame of the body it is attached to.
    pub pose: Transform,
    /// Index into `articulation`, or `None` for the hull.
    pub attached_to: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MuzzleDef {
    pub name: String,
    /// Articulation joint carrying the barrel.
    pub joint: usize,
    /// Muzzle position and firing direction (+(-Z) of the pose) in that joint's body frame.
    pub pose: Transform,
    pub caliber_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrationDef {
    /// Substeps per 60 Hz tick, set when the rig is baked from its stiffest mode.
    pub substeps: u32,
}

impl PhysRig {
    /// The names of the joint coordinates a [`crate::frame::VehicleFrame`] carries for this rig, in order. This order is part of
    /// the contract: first every station's wheel spin (`<station>.spin`, rad, positive = rolling forward), then every *steered*
    /// station's steer angle (`<station>.steer`, rad, positive = left), then every station's suspension travel
    /// (`<station>.travel`, m, positive = compressed), then the articulation joints in rig order (turret yaw rad positive left, gun
    /// pitch rad positive up, recoil m positive rearward).
    pub fn joint_names(&self) -> Vec<String> {
        let mut n: Vec<String> = self.stations.iter().map(|s| format!("{}.spin", s.name)).collect();
        n.extend(self.stations.iter().filter(|s| s.steer.is_some()).map(|s| format!("{}.steer", s.name)));
        n.extend(self.stations.iter().map(|s| format!("{}.travel", s.name)));
        n.extend(self.articulation.iter().map(|j| j.name.clone()));
        n
    }

    pub fn total_mass_kg(&self) -> f64 {
        self.hull.mass_kg
            + self.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()
            + self.articulation.iter().map(|j| j.body.mass_kg).sum::<f64>()
            + self.tracks.iter().map(|t| t.mass_per_m_kg * t.contact_length_m).sum::<f64>()
    }

    /// Structural checks every rig must pass before a solver sees it. Returns every problem found.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut e: Vec<String> = Vec::new();
        let mut chk = |ok: bool, msg: String| {
            if !ok {
                e.push(msg);
            }
        };
        chk(self.hull.mass_kg > 0.0 && self.hull.mass_kg.is_finite(), "hull mass must be positive".into());
        chk(self.hull.inertia_kg_m2.is_finite() && self.hull.inertia_kg_m2.det() > 0.0, "hull inertia must be a finite, positive definite tensor".into());
        chk(self.integration.substeps >= 1, "substeps must be at least 1".into());
        for (i, s) in self.stations.iter().enumerate() {
            chk(s.wheel.radius_m > 0.0, format!("station {i} ({}): wheel radius must be positive", s.name));
            chk(s.unsprung_mass_kg >= 0.0, format!("station {i} ({}): negative unsprung mass", s.name));
            chk(s.bump_dir.length() > 0.5, format!("station {i} ({}): bump_dir must be a unit vector", s.name));
            chk(s.bump_travel_m >= 0.0 && s.droop_travel_m >= 0.0, format!("station {i} ({}): negative travel", s.name));
            if let Some(o) = s.drive_output {
                chk(o < self.drivetrain.outputs.len(), format!("station {i} ({}): drive_output {o} out of range", s.name));
            }
            if s.wheel.kind == WheelKind::Tyre {
                chk(s.wheel.tyre.is_some(), format!("station {i} ({}): a Tyre wheel needs a TyreDef", s.name));
            }
        }
        for (i, o) in self.drivetrain.outputs.iter().enumerate() {
            chk(o.station < self.stations.len(), format!("output {i}: station {} out of range", o.station));
        }
        for (i, b) in self.drivetrain.brakes.iter().enumerate() {
            chk(b.station < self.stations.len(), format!("brake {i}: station {} out of range", b.station));
        }
        let mut leaves = Vec::new();
        collect_outputs(&self.drivetrain.driveline, &mut leaves);
        for o in leaves {
            chk(o < self.drivetrain.outputs.len(), format!("driveline refers to output {o}, which does not exist"));
        }
        for (i, t) in self.tracks.iter().enumerate() {
            chk(t.samples >= 2, format!("track {i} ({}): at least 2 samples", t.name));
            for &s in &t.stations {
                chk(s < self.stations.len(), format!("track {i} ({}): station {s} out of range", t.name));
            }
        }
        for (i, j) in self.articulation.iter().enumerate() {
            if let Some(p) = j.parent {
                chk(p < i, format!("joint {i} ({}): parent {p} must precede it", j.name));
            }
            chk(j.axis.length() > 0.5, format!("joint {i} ({}): axis must be a unit vector", j.name));
            chk(j.body.mass_kg > 0.0, format!("joint {i} ({}): body mass must be positive", j.name));
        }
        for m in &self.muzzles {
            chk(m.joint < self.articulation.len(), format!("muzzle {}: joint {} out of range", m.name, m.joint));
        }
        for a in &self.anti_roll {
            chk(a.left_station < self.stations.len() && a.right_station < self.stations.len(), "anti-roll bar refers to a missing station".into());
        }
        if e.is_empty() {
            Ok(())
        } else {
            Err(e)
        }
    }
}

fn collect_outputs(n: &DriveNode, out: &mut Vec<usize>) {
    match n {
        DriveNode::Output(i) => out.push(*i),
        DriveNode::Diff { children, .. } | DriveNode::SteerUnit { children, .. } => {
            for c in children {
                collect_outputs(c, out);
            }
        }
    }
}
