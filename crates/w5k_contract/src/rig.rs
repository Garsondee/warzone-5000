//! `PhysRig`: the solver-level description of a vehicle, compiled by lane FORGE from a [`crate::def::VehicleDef`] and consumed by
//! the dynamics lanes (CHASSIS, DRIVE, TRACKS), COMBAT and the AI. Plain numbers in SI: no sliders, no pedigree (that lives in the
//! `VehicleDef`).
//!
//! Structure (reduced coordinates, not a general multibody): one 6-DoF **hull**; a list of **stations** (a wheel or road wheel
//! on its own suspension travel, optionally steered, optionally driven); optional **linkages** (several stations on one spring:
//! bogies, walking beams, inboard leaf springs); optional **tracks** that loop over stations; a **drivetrain**; an
//! **articulation** tree (turret yaw, gun pitch, recoil slide) hanging off the hull; aero; collision proxies; muzzle frames.
//! Frame: hull frame, +X right, +Y up, -Z forward, origin at the design datum.
//!
//! Conventions that every lane relies on (the details are in `docs/architecture/CONTRACTS.md`):
//! * **Compression** of a station is measured from its rest position along `bump_dir`: positive toward the hull, negative in droop.
//!   For a swinging arm it is the *vertical rise* of the wheel centre.
//! * **Spring forces are totals**: the force a spring returns at compression 0 is `preload_n`.
//! * `radius_m` of a tyre or road wheel is the **free (unloaded)** radius; the wheel centre `rest_pos_m` carries the static deflection.
//! * **Gear ratios and `ratio` fields are reductions** (input speed / output speed), multiplicative down the driveline tree.
//! * Optional extensions (everything marked `#[serde(default)]`) are *off* at their default; a solver that does not implement
//!   one must refuse the rig ([`PhysRig::required_features`]) instead of ignoring it.

use serde::{Deserialize, Serialize};
use w5k_math::{Mat3, StateHasher, Transform, Vec3};

use crate::combat::CombatDef;

/// The most substeps a rig may declare (per 60 Hz tick). A sanity bound for `validate()`; the design tripwire is lower (a lane whose rig
/// needs more than 8 stops and writes it up, and spike S1 settles the budget for heavy tracked vehicles).
pub const MAX_SUBSTEPS: u32 = 16;

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysRig {
    pub id: String,
    /// Sprung hull: everything rigidly attached above the suspension, including the unsprung-mass-free share of crew and fuel, the
    /// top run of every track belt and every rigid (spin-only) station.
    pub hull: BodyDef,
    /// Height of the hull datum above the ground plane at the design pose, m. The ground plane is therefore at `y = -ride_height_m` in
    /// the hull frame. With the *free* wheel radius and `rest_pos_m` it fixes the static tyre deflection.
    #[serde(default)]
    pub ride_height_m: f64,
    pub stations: Vec<StationDef>,
    /// Stations that share one spring system (bogie, walking beam, inboard leaf springs). Member stations have `SpringKind::Rigid`.
    #[serde(default)]
    pub linkages: Vec<LinkageDef>,
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
    /// Armour, damage modules, removable mass items and sights (M3; empty until COMBAT's lane fills it). Addressed by `Option<usize>` joint
    /// indices (`None` = the hull). **A body's mass always includes its removable items at as-loaded fill.**
    #[serde(default)]
    pub combat: CombatDef,
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
    /// Mass that moves with the wheel (wheel, tyre, hub, brake, the moving share of the arm or axle), kg. Zero for a rigid (spin-only)
    /// station that is not in a linkage: its mass is part of `hull.mass_kg`.
    pub unsprung_mass_kg: f64,
    pub suspension: SuspensionDef,
    pub steer: Option<SteerDef>,
    pub wheel: WheelDef,
    /// Index into `drivetrain.outputs` if this station is driven.
    pub drive_output: Option<usize>,
    /// Pivot of a swinging arm (torsion-bar road wheel, trailing arm), hull frame. The wheel centre then moves on a circle about it;
    /// `bump_dir` is the direction of the wheel's vertical motion at rest. `None` = the wheel moves on the straight line `bump_dir`.
    #[serde(default)]
    pub arm_pivot_m: Option<Vec3>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WheelDef {
    pub kind: WheelKind,
    /// The **free (unloaded)** outer radius of a tyre, road wheel, idler or return roller; the **pitch radius** of a sprocket (belt
    /// speed = spin x pitch radius). The loaded rolling radius is the solver's business.
    pub radius_m: f64,
    pub width_m: f64,
    /// Rotational inertia of wheel, tyre, hub and brake disc about the axle, kg m^2 (on the wheel side of the final drive).
    pub inertia_kg_m2: f64,
    /// Present for `WheelKind::Tyre`.
    pub tyre: Option<TyreDef>,
    /// Lateral offsets (hull X, m) of every contact patch of this station from the wheel's centre plane. Empty = one centred patch. A dual
    /// wheel on one hub is `[-0.15, 0.15]`; each patch has `width_m` and the same tyre, and all patches share travel, spin and drive.
    /// A frame's `contacts` then lists one entry per patch (`contact_names`: `<station>` or `<station>.<k>`).
    #[serde(default)]
    pub patches_x_m: Vec<f64>,
}

/// Tyre model parameters (a deliberately small set; lane CHASSIS extends it by CCR).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TyreDef {
    pub vertical_stiffness_n_m: f64,
    pub vertical_damping_ns_m: f64,
    /// Grip multiplier: the tyre's peak friction on a surface is `Material::mu_peak * mu_scale` (no other factor, so nothing counts twice).
    /// 1 = the reference rubber tyre the surface table is written for. FORGE compiles it from the `VehicleDef`'s `mu_peak_ref` (the tyre's own
    /// peak friction on dry hard ground) as `mu_peak_ref / mu_peak of the table's dry hard reference surface`.
    pub mu_scale: f64,
    /// Longitudinal slip stiffness: force per unit load per unit slip ratio at zero slip (dimensionless).
    pub slip_stiffness: f64,
    /// Cornering stiffness: lateral force per unit load per radian of slip angle (1/rad).
    pub cornering_stiffness_per_rad: f64,
    /// Relaxation length: the distance a tyre rolls for its force to build up (m); also what tames low-speed jitter.
    pub relaxation_length_m: f64,
    pub rolling_coeff: f64,
    /// Inflation pressure, Pa (sets the contact patch area on soft ground).
    pub inflation_pa: f64,
    /// Contact patch length at the load that gives the static deflection of this rig, m.
    pub patch_length_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SteerDef {
    /// Maximum steering angle of this wheel at full lock, rad (positive = left, as for any yaw). Where Ackermann geometry makes the inner and
    /// outer wheel differ this is the **outer** wheel's angle; `Command::steer = +1` (full right) gives `-max_angle_rad`.
    pub max_angle_rad: f64,
    /// 0 = parallel steer, 1 = full Ackermann geometry about the reference axle (the mean longitudinal position of the unsteered axles).
    pub ackermann: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuspensionDef {
    pub spring: SpringKind,
    /// Total spring force at the rest position (compression 0), N, along `bump_dir`: the static load share of this station (or of this
    /// linkage). For a `Table` or `Hydropneumatic` spring it must agree with the spring's own value at rest (`validate()` checks).
    pub preload_n: f64,
    pub damper: DamperDef,
    pub bump_stop: BumpStopDef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpringKind {
    /// No suspension (a solid mount, a sprocket, a member of a linkage).
    Rigid,
    /// A constant wheel rate (coil, or any linearised spring), N/m at the wheel: `F = preload_n + rate * compression`.
    Linear { rate_n_m: f64 },
    /// A tabulated **total** force against compression from the rest position (leaf, rubber, air): `(compression_m, force_n)` strictly
    /// ascending in compression, which may be negative (droop). The force at compression 0 is `preload_n`. Beyond the ends the nearest
    /// segment is extended linearly; the force is never negative (a spring cannot pull).
    Table { points: Vec<(f64, f64)> },
    /// A torsion bar acting through an arm of length `arm_length_m` that stands `rest_arm_angle_rad` below the horizontal at rest. With
    /// compression `c` (the wheel's vertical rise) the arm angle `phi` follows from `sin(phi) = sin(phi0) - c / arm_length_m`; the bar
    /// torque is `T0 + rate_nm_rad * (phi0 - phi)` with `T0 = preload_n * arm_length_m * cos(phi0)`, and the vertical force at the wheel is
    /// `T / (arm_length_m * cos(phi))`.
    Torsion { rate_nm_rad: f64, arm_length_m: f64, rest_arm_angle_rad: f64 },
    /// A hydropneumatic strut: a polytropic gas spring `p V^gamma = const` acting on a piston. `gas_pressure_pa` (absolute) and
    /// `gas_volume_m3` are the values at rest; compression `c` moves the piston by `c`, so `V = V0 - piston_area_m2 * c`, `F = p * piston_area_m2`.
    Hydropneumatic { gas_pressure_pa: f64, gas_volume_m3: f64, piston_area_m2: f64, gamma: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DamperDef {
    /// Damping coefficient in compression and in rebound, N s/m.
    pub bump_ns_m: f64,
    pub rebound_ns_m: f64,
    /// Above this piston speed the damper becomes digressive (m/s); 0 disables the knee.
    pub knee_speed_m_s: f64,
    /// Above the knee the *incremental* coefficient is this fraction of the base one (1 = linear); the force is continuous at the knee.
    pub post_knee_ratio: f64,
    /// Coulomb (dry) friction of the strut or leaf pack, N, regularised by the solver. 0 = none. (A viscous stand-in for 3 kN of friction
    /// would need 14 substeps; a friction element does not.)
    #[serde(default)]
    pub friction_n: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BumpStopDef {
    /// Compression beyond which the bump stop acts, measured from the rest position, m.
    pub engage_m: f64,
    /// Stiffness once engaged, N/m. Progressive: the **stiffness** is multiplied by `1 + progression * penetration / engage_m`.
    pub rate_n_m: f64,
    pub progression: f64,
    /// Damping of the stop while engaged, N s/m.
    #[serde(default)]
    pub damping_ns_m: f64,
    /// At `bump_travel_m` the wheel meets a rigid stop: the compression is clamped there and the approach velocity is reflected with
    /// `restitution` (0..1) instead of the stop spring stiffening without bound.
    #[serde(default)]
    pub hard_limit: bool,
    #[serde(default)]
    pub restitution: f64,
}

/// An anti-roll bar: a torsion spring between two stations on opposite sides. The force on each end is `rate_n_m` times the *difference*
/// of the two compressions (opposite signs on the two ends), so a positive rate resists body roll. Negative rates are not allowed: use a
/// [`LinkageDef`] for stations that share a spring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AntiRollDef {
    pub left_station: usize,
    pub right_station: usize,
    /// Force per metre of differential travel, N/m.
    pub rate_n_m: f64,
}

/// A rigid beam or lever carrying several stations on **one** spring system: a walking-beam tandem, a bogie of road wheels, a solid
/// axle whose leaf springs sit inboard of its wheels. The pivot (the point the spring acts at) moves by `h = sum w_i c_i`, where `c_i` is
/// the compression of member `i` and the weights sum to 1; the spring system returns the force `F(h, h')` and member `i` receives `w_i F`
/// along its `bump_dir`. The member stations keep their own travel coordinate (so frames, render rigs and `joint_names()` do not change)
/// but their own spring is `Rigid`; the spring, damper and bump stop live here. A solid axle with two spring seats is two linkages.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkageDef {
    pub name: String,
    /// `(station index, weight)`.
    pub members: Vec<(usize, f64)>,
    /// Spring, damper and bump stop acting on the pivot coordinate `h` (compression measured from the design pose).
    pub suspension: SuspensionDef,
    /// Largest allowed difference between any two members' compressions (the beam's rocking stop), m. `None` = unlimited.
    #[serde(default)]
    pub rock_limit_m: Option<f64>,
}

/// Vertical compliance of a road wheel (rubber and pad) in series with the belt: the number the substep count is baked from. A track
/// sample is **massless**; its only stiffness is this one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WheelContact {
    pub vertical_stiffness_n_m: f64,
    pub vertical_damping_ns_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackDef {
    pub name: String,
    pub side: Side,
    /// Every wheel the belt wraps, **in loop order**: starting at the sprocket, along the top run through the return rollers (if any) to
    /// the idler, then back along the ground run through the road wheels to the sprocket. Return rollers are stations of kind
    /// `ReturnRoller` and belong in the list (they spin and are drawn).
    pub stations: Vec<usize>,
    pub sprocket: usize,
    pub idler: usize,
    pub width_m: f64,
    pub pitch_m: f64,
    /// Design length of belt in contact with firm ground, m.
    pub contact_length_m: f64,
    pub mass_per_m_kg: f64,
    /// Contact samples along the footprint (8-16 is the working range). Samples are massless.
    pub samples: u16,
    /// Shoe friction relative to rubber on the same surface (1 = rubber pads, >1 = grousers on soft ground, <1 = steel on road).
    pub shoe_mu_scale: f64,
    /// A second value for soft ground when it differs from the hard-ground one (a steel grouser: 0.50 on asphalt, 1.44 on mud).
    #[serde(default)]
    pub shoe_mu_scale_soft: Option<f64>,
    /// Track tension at rest, N (the idler's spring preload carries it; not part of the vertical equilibrium).
    pub tension_n: f64,
    /// Closed belt length along the pitch line, m. 0 = unknown: `contact_length_m` is used. The belt's kinetic energy is `m v^2` for the
    /// whole loop (the ground run has none, the rest has twice the translational share), so reflected inertia uses this length.
    #[serde(default)]
    pub belt_length_m: f64,
    /// Belt (shoe and pad) thickness under the road wheels, m: the ground plane lies this far below the lowest road-wheel bottoms.
    #[serde(default)]
    pub thickness_m: f64,
    /// Running resistance of the track system on hard ground: `F = (c0 + c1 v) N` with `N` the track's normal load (dimensionless; s/m).
    #[serde(default)]
    pub resist_c0: f64,
    #[serde(default)]
    pub resist_c1_s_m: f64,
    /// Sprocket teeth; the pitch radius is `pitch_m / (2 sin(pi / n))` (that is `WheelDef::radius_m` of the sprocket). 0 = unknown.
    #[serde(default)]
    pub sprocket_teeth: u8,
    #[serde(default)]
    pub wheel_contact: WheelContact,
}

impl TrackDef {
    /// The belt length that counts: `belt_length_m` if known, else the contact length.
    pub fn effective_belt_length_m(&self) -> f64 {
        if self.belt_length_m > 0.0 {
            self.belt_length_m
        } else {
            self.contact_length_m
        }
    }

    pub fn belt_mass_kg(&self) -> f64 {
        self.mass_per_m_kg * self.effective_belt_length_m()
    }
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
    /// Selectable driveline states (high and low range, front-axle declutch, diff lock). Empty = one fixed state.
    #[serde(default)]
    pub modes: Vec<DriveModeDef>,
    /// The mode active at the start (index into `modes`).
    #[serde(default)]
    pub default_mode: u8,
}

/// One selectable driveline state, chosen by `Command::drive_mode`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriveModeDef {
    /// "H4", "L4", "6x4", ...
    pub name: String,
    /// Multiplies the `ratio` of the root `driveline` node (the transfer case): low range is a scale above 1.
    #[serde(default = "one")]
    pub ratio_scale: f64,
    /// Outputs disconnected from the driveline in this mode (a declutched front axle): they roll freely.
    #[serde(default)]
    pub declutched_outputs: Vec<usize>,
    /// Groups of outputs forced to one speed (a differential lock).
    #[serde(default)]
    pub locked_groups: Vec<Vec<usize>>,
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
    /// Full-load torque against speed, `(rpm, N m)` ascending in rpm. (rpm appears in engine data because that is how engines are
    /// published; the solver converts to rad/s once.)
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
    /// A free-turbine output: the shaft may stall at 0 rpm (no idle floor) and the gas generator behind it lags the throttle.
    #[serde(default)]
    pub free_output: bool,
    /// Throttle-to-torque response time constant (spool-up of a turbine, turbo lag), s. 0 = instantaneous.
    #[serde(default)]
    pub response_time_s: f64,
    /// Fuel burned at idle, kg/s (a turbine burns fuel while standing).
    #[serde(default)]
    pub idle_fuel_kg_s: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CouplingDef {
    Direct,
    /// A friction clutch of capacity `max_torque_nm`. `engage_rpm` is the engine speed at which an automatic clutch (centrifugal) starts to
    /// engage; a manual clutch follows `Command::clutch`.
    Clutch {
        max_torque_nm: f64,
        engage_rpm: f64,
    },
    /// A hydrodynamic converter: pump torque `T = (omega_pump / K)^2` with `K = k_factor_rpm_per_sqrt_nm` (rpm per square root of N m, at
    /// speed ratio 0), torque ratio falling from `stall_ratio` at stall to 1 at the coupling point; an optional lock-up above `lockup_speed_ratio`.
    TorqueConverter {
        stall_ratio: f64,
        k_factor_rpm_per_sqrt_nm: f64,
        lockup_speed_ratio: Option<f64>,
    },
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
    /// Splits torque by `split` (equal by default) and lets the two sides turn at different speeds; creates no torque of its own.
    Open,
    /// Forces its children to one speed.
    Locked,
    /// Torque-sensing / limited slip: the faster side may carry at most `bias` times the torque of the slower one.
    LimitedSlip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteerUnitKind {
    ClutchBrake,
    ControlledDifferential,
    DoubleDifferential,
    Hydrostatic,
}

/// How a tracked vehicle's steering unit turns the two outputs against each other. A kinematic unit (clutch-brake, double differential) fixes
/// a speed *ratio* (one turn radius per gear); a hydrostatic unit fixes a speed *difference* (the radius grows with gear and it works in
/// neutral); a controlled differential steers by brake torque. All fields are optional: the default is the kind's own default behaviour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SteerLaw {
    /// Fractional speed difference between the outputs at full demand, `(v_outer - v_inner) / (v_outer + v_inner)`, one entry per forward gear
    /// (index = gear - 1, the last entry repeats). Empty = the kind's default.
    #[serde(default)]
    pub diff_ratio_by_gear: Vec<f64>,
    /// Discrete steering stages as fractions of the full-demand difference (a double-differential Tiger II has two radii per gear: `[0.5, 1.0]`).
    /// Empty = continuous.
    #[serde(default)]
    pub detents: Vec<f64>,
    /// Hydrostatic: the speed difference between the outputs (rad/s at the sprockets) at full demand, independent of output speed.
    #[serde(default)]
    pub diff_speed_rad_s: Option<f64>,
    /// The unit is fed from the engine side and so steers with the gearbox in neutral (a pivot turn).
    #[serde(default)]
    pub works_in_neutral: bool,
    /// Largest torque the unit can put across the two outputs, N m at the sprockets (pump and motor limit, or steering-brake capacity). 0 = unknown.
    #[serde(default)]
    pub max_steer_torque_nm: f64,
    /// For clutch-brake and controlled-differential units: the brakes (indices into `DrivetrainDef::brakes`) that realise the steering, [left, right].
    #[serde(default)]
    pub steer_brakes: Option<[usize; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DriveNode {
    /// A differential or gear stage (axle, centre, transfer case) splitting torque between its children. `ratio` is the stage's reduction
    /// (input speed / output speed, 1 = none) and compounds multiplicatively down the tree.
    Diff {
        kind: DiffKind,
        ratio: f64,
        /// `LimitedSlip` only: the torque-bias ratio (>= 1).
        bias: f64,
        /// Open differential torque split between the children (fractions summing to 1). Empty = equal.
        #[serde(default)]
        split: Vec<f64>,
        /// Mechanical efficiency of this stage (0..1].
        #[serde(default = "one")]
        efficiency: f64,
        children: Vec<DriveNode>,
    },
    /// The steering unit of a tracked vehicle: a driven pair with a steering demand mixed in. `children` = [left, right], checked against the
    /// sides of the stations they drive. `ratio` is the unit's own input reduction.
    SteerUnit {
        kind: SteerUnitKind,
        ratio: f64,
        #[serde(default)]
        law: SteerLaw,
        children: Vec<DriveNode>,
    },
    /// A driven shaft: index into `DrivetrainDef::outputs`.
    Output(usize),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputDef {
    /// The station this shaft drives (wheel hub or sprocket).
    pub station: usize,
    /// Final-drive (hub or portal reduction) ratio, input speed / output speed.
    pub final_drive_ratio: f64,
    pub efficiency: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrakeLocation {
    /// On the driven station's shaft after the final drive (at the hub). The default.
    #[default]
    AtStation,
    /// On the shaft before the output's `final_drive_ratio` (an inboard brake, a Sherman steering brake): at the wheel it is multiplied by that ratio.
    BeforeFinalDrive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrakeSite {
    /// Acts on `station`'s shaft.
    #[default]
    Wheel,
    /// Acts on the gearbox output shaft (a transmission or transfer-case parking drum); `station` is ignored.
    Driveline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrakeDef {
    pub station: usize,
    /// Peak torque at the reference shaft named by `location` (and `site`), N m.
    pub max_torque_nm: f64,
    pub thermal_mass_j_k: f64,
    /// Convective cooling, W/K, at rest and per m/s of vehicle speed.
    pub cooling_w_k: f64,
    pub cooling_per_ms_w_k: f64,
    /// Temperature range over which friction fades from full to `fade_floor`, K.
    pub fade_start_k: f64,
    pub fade_end_k: f64,
    pub fade_floor: f64,
    /// Applied by `Command::parking_brake`.
    pub parking: bool,
    /// Applied by the brake pedal (`Command::brake`). A drum used only as a parking brake is `parking: true, service: false`.
    #[serde(default = "yes")]
    pub service: bool,
    #[serde(default)]
    pub location: BrakeLocation,
    #[serde(default)]
    pub site: BrakeSite,
    /// A steering brake of a clutch-brake or controlled-differential unit: the steering demand applies it on its side.
    #[serde(default)]
    pub steering: bool,
    /// Torque factor for the direction of rotation that de-energises a band brake (1 = a disc).
    #[serde(default = "one")]
    pub reverse_torque_factor: f64,
    /// Time from command to full torque and back (hydraulic ~0.1 s, air ~0.25 s), s. 0 = instantaneous.
    #[serde(default)]
    pub apply_time_s: f64,
    #[serde(default)]
    pub release_time_s: f64,
    /// Hydraulic or air circuit (a failed circuit takes its brakes out).
    #[serde(default)]
    pub circuit: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AeroDef {
    pub drag_coeff: f64,
    pub frontal_area_m2: f64,
    /// Where the drag acts, hull frame (height of the centre of pressure matters for pitch).
    pub centre_of_pressure_m: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JointKind {
    Revolute,
    Prismatic,
}

/// What a joint is for. Roles name the *kind* of motion; which demand moves the joint is its `aim_channel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JointRole {
    /// A yaw ring (main turret, cupola, a sight head): follows the `yaw_rad` of its aim channel.
    TurretYaw,
    /// An elevation joint (gun cradle, a sight's pitch): follows the `pitch_rad` of its aim channel.
    GunPitch,
    /// A passive recoil slide.
    Recoil,
    Other,
}

/// A joint in the articulation tree and the body it carries. The tree may branch (a cupola ring beside the gun cradle); parents precede
/// children. **Frame convention:** the body of joint `j` has its origin at `anchor_m` (in the parent body's frame, parent axes at joint
/// coordinate 0) and is rotated about `axis` by `q` (revolute) or slid along `axis` by `q` (prismatic); `BodyDef::com_m` and
/// `BodyDef::inertia_kg_m2` are in that moving frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointDef {
    pub name: String,
    pub role: JointRole,
    /// Index of the parent joint in `articulation`, or `None` for the hull.
    pub parent: Option<usize>,
    pub kind: JointKind,
    /// Joint origin and axis in the parent's frame (a unit axis).
    pub anchor_m: Vec3,
    pub axis: Vec3,
    /// Motion limits, rad or m, `(min, max)`; `None` = unlimited (turret ring). Enforced as stiff penalty springs whose reaction goes to the
    /// parent (so momentum is conserved), never as a rigid clamp that deletes velocity.
    pub limits: Option<(f64, f64)>,
    pub body: BodyDef,
    /// What moves this joint.
    #[serde(default)]
    pub drive: JointDrive,
    /// Which `Command::aim` channel commands this joint (0 = the main turret and gun). Ignored for joints that are not driven by a servo.
    #[serde(default)]
    pub aim_channel: u8,
}

/// What moves a joint. `_si` in a field name means the SI unit of the joint's own coordinate: rad, rad/s, N m, ... for a revolute joint;
/// m, m/s, N, ... for a prismatic one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JointDrive {
    /// Nothing drives it: it moves under the loads on it (a hinge, a free sight), with Coulomb friction `friction_si` (N m or N).
    Free { friction_si: f64 },
    /// A position servo with limits, a PD law and an optional stabiliser.
    Servo(ServoDef),
    /// A passive recoil mechanism.
    Recoil(RecoilDef),
}

impl Default for JointDrive {
    fn default() -> Self {
        JointDrive::Free { friction_si: 0.0 }
    }
}

/// Who or what turns a servo joint (it sets latency and what can disable it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Actuator {
    #[default]
    Hydraulic,
    Electric,
    /// A crew member turns it (hand crank, shoulder rest): a dead crew member disables the joint, and the rate falls to `fallback_rate_si`
    /// when the power drive is lost.
    Human(CrewRole),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CrewRole {
    Commander,
    Gunner,
    Loader,
    Driver,
    Assistant,
}

/// A servo. The slew acceleration it can reach is `alpha = min(max_accel_si, max_effort_si / I)` with `I` the inertia of the joint's whole
/// subtree about the axis; `validate()` rejects a rig whose `max_accel_si` exceeds what the effort allows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServoDef {
    pub max_rate_si: f64,
    pub max_accel_si: f64,
    /// Peak drive torque (N m) or force (N).
    pub max_effort_si: f64,
    /// PD gains: effort per unit of position error (`kp`) and per unit of rate error (`kd`).
    pub kp_si: f64,
    pub kd_si: f64,
    pub actuator: Actuator,
    /// Delay from demand to effect: a person ~0.25 s, hydraulic ~0.05 s, electric ~0.01 s.
    pub latency_s: f64,
    /// Rate available when the power drive is lost (hand crank), same unit as `max_rate_si`. 0 = none.
    pub fallback_rate_si: f64,
    /// Stabiliser: rejection of the parent's motion (see `StabiliserDef`). `None` = unstabilised (a WW2 gun).
    pub stabiliser: Option<StabiliserDef>,
}

/// A stabiliser rejects the motion of the *parent* body along the joint axis. **The law** (pinned here so that FORGE, COMBAT, VALIDATION and
/// the AI mean the same thing): the fraction of the parent's absolute angular rate (or velocity, for a prismatic joint) along the joint axis that
/// still reaches the payload at frequency `f` is `|1 - r / (1 + j f / f_b)|`, with `r = rejection` (the rejection at DC, 0..1) and
/// `f_b = bandwidth_hz`; `latency_s` adds a pure delay. A stabiliser can only cancel what its own axis can reach (a hull pitched 90 degrees
/// puts the gun's roll out of reach of an elevation loop).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StabiliserDef {
    pub rejection: f64,
    pub bandwidth_hz: f64,
    pub latency_s: f64,
}

/// A passive recoil mechanism on a prismatic joint (positive recoil = rearward). The slide is a spring, a damper and a stop, not a servo.
/// `stroke_m` equals the joint's `limits.1`. The preload must hold the barrel in battery at the highest elevation (`preload_n > m g sin(el_max)`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoilDef {
    pub stroke_m: f64,
    /// The recuperator: any `SpringKind` (a hydropneumatic recuperator is `Hydropneumatic`), compression = recoil distance.
    pub spring: SpringKind,
    /// Force holding the barrel in battery, N (the spring's total force at zero recoil).
    pub preload_n: f64,
    /// Buffer: `F = c v + q v |v|` (linear and quadratic damping).
    pub damper_ns_m: f64,
    pub damper_quad_ns2_m2: f64,
}

/// One shape vocabulary for collision proxies, armour solids and module volumes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProxyShape {
    Sphere {
        radius_m: f64,
    },
    Capsule {
        a: Vec3,
        b: Vec3,
        radius_m: f64,
    },
    Box {
        half_m: Vec3,
    },
    /// A convex solid as half-spaces `n . x <= d` (unit normals): wedges, glacis plates, turret faces.
    Convex {
        planes: Vec<(Vec3, f64)>,
    },
}

/// What a collision proxy is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyRole {
    /// The hull body against terrain and props.
    #[default]
    Hull,
    /// The plate that ploughs soil and sets the ground clearance: clearance and plan area come from this proxy's lowest face and footprint.
    Belly,
    /// A side skirt or fender.
    Skirt,
    Other,
}

/// A collision shape the physics uses against terrain and props (cheap, convex, a few per vehicle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollisionProxy {
    pub name: String,
    pub shape: ProxyShape,
    /// Pose in the frame of the body it is attached to.
    pub pose: Transform,
    /// Index into `articulation`, or `None` for the hull. Exclusive with `attached_station`.
    pub attached_to: Option<usize>,
    /// Carried by a station (an axle housing or diff nose that rides the unsprung axle): the pose is in the station's frame at travel 0.
    #[serde(default)]
    pub attached_station: Option<usize>,
    #[serde(default)]
    pub role: ProxyRole,
}

/// A weapon's muzzle. Bore direction is the -Z axis of `pose`; a barrel that recoils has its recoil axis along +Z of the joint's frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MuzzleDef {
    pub name: String,
    /// Articulation joint carrying the barrel; `None` = fixed to the hull (a bow machine gun, a casemate gun with no moving part).
    pub joint: Option<usize>,
    /// Muzzle position and firing direction in that body's frame (the hull frame when `joint` is `None`).
    pub pose: Transform,
    pub caliber_m: f64,
    /// The numbers the momentum balance needs. Ballistics proper (drag, penetration) live in `content/combat/`, named by `catalogue_id`.
    pub weapon: WeaponDef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeaponDef {
    /// Key into COMBAT's weapon catalogue (`content/combat/`).
    pub catalogue_id: String,
    /// The `Command::fire` bit (0..7) that fires this weapon.
    pub trigger: u8,
    pub projectile_mass_kg: f64,
    pub muzzle_velocity_m_s: f64,
    /// Net impulse on the gun `J = factor * m * v`: 1.3 to 1.9 for a gun with propellant gas (per gun), below 1 with a muzzle brake, about 0.05 for a recoilless weapon.
    pub recoil_impulse_factor: f64,
    pub dispersion_mrad: f64,
    pub cycle: FireCycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FireCycle {
    /// One round per trigger pull, then a reload (loader or autoloader).
    Single { reload_s: f64 },
    /// Automatic fire while the trigger is held.
    Auto { rounds_per_min: f64 },
    /// An autoloader that needs the gun at a fixed loading elevation.
    Autoloader { cycle_s: f64, loading_pitch_rad: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrationDef {
    /// Substeps per 60 Hz tick, set when the rig is baked from its stiffest mode (the rule is spike S1's to settle; see CONTRACTS.md).
    pub substeps: u32,
}

/// Optional rig features that a solver must implement or refuse: see [`PhysRig::required_features`].
pub mod feature {
    pub const LINKAGES: &str = "linkages";
    pub const MULTI_PATCH_WHEELS: &str = "multi-patch wheels";
    pub const DRIVE_MODES: &str = "drive modes";
    pub const STEER_LAW: &str = "steering-unit law";
    pub const DIFF_SPLIT_OR_EFFICIENCY: &str = "differential split or efficiency";
    pub const BRAKE_LAG: &str = "brake apply and release lag";
    pub const BRAKE_DRIVELINE_SITE: &str = "driveline brake";
    pub const BRAKE_STEERING: &str = "steering brakes";
    pub const BRAKE_REVERSE_FACTOR: &str = "band-brake reverse factor";
    pub const DRY_FRICTION: &str = "dry friction in the strut";
    pub const HARD_BUMP_LIMIT: &str = "hard bump limit";
    pub const SWING_ARM: &str = "swinging arms";
    pub const FREE_TURBINE: &str = "free turbine";
    pub const STATION_PROXIES: &str = "proxies on stations";
    pub const TRACK_RESISTANCE: &str = "track running resistance";
}

impl PhysRig {
    /// The names of the joint coordinates a [`crate::frame::VehicleFrame`] carries for this rig, in order. This order is part of
    /// the contract: first every station's wheel spin (`<station>.spin`, rad, positive = rolling forward), then every *steered*
    /// station's steer angle (`<station>.steer`, rad, positive = left), then every station's suspension travel
    /// (`<station>.travel`, m, positive = compressed), then the articulation joints in rig order (turret yaw rad positive left, gun
    /// pitch rad positive up, recoil m positive rearward). Spin and every other revolute coordinate is **continuous** (never wrapped):
    /// viewers interpolate it linearly.
    pub fn joint_names(&self) -> Vec<String> {
        let mut n: Vec<String> = self.stations.iter().map(|s| format!("{}.spin", s.name)).collect();
        n.extend(self.stations.iter().filter(|s| s.steer.is_some()).map(|s| format!("{}.steer", s.name)));
        n.extend(self.stations.iter().map(|s| format!("{}.travel", s.name)));
        n.extend(self.articulation.iter().map(|j| j.name.clone()));
        n
    }

    /// The names of the contact entries a frame carries, in order: one per contact patch of a tyred station (`<station>` for one centred
    /// patch, `<station>.<k>` per patch of `patches_x_m`), then one per sample of every track (`<track>.<k>`).
    pub fn contact_names(&self) -> Vec<String> {
        let mut n = Vec::new();
        for s in self.stations.iter().filter(|s| s.wheel.kind == WheelKind::Tyre) {
            if s.wheel.patches_x_m.is_empty() {
                n.push(s.name.clone());
            } else {
                n.extend((0..s.wheel.patches_x_m.len()).map(|k| format!("{}.{k}", s.name)));
            }
        }
        for t in &self.tracks {
            n.extend((0..t.samples).map(|k| format!("{}.{k}", t.name)));
        }
        n
    }

    /// Height of the ground plane in the hull frame at the design pose, m.
    pub fn ground_y_m(&self) -> f64 {
        -self.ride_height_m
    }

    /// Total mass: the hull (which includes the top run of every belt and every rigid station), every station's unsprung mass, the
    /// articulated bodies, and each belt's ground run (`mass_per_m_kg * contact_length_m`, supported by the ground).
    pub fn total_mass_kg(&self) -> f64 {
        self.hull.mass_kg
            + self.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()
            + self.articulation.iter().map(|j| j.body.mass_kg).sum::<f64>()
            + self.tracks.iter().map(|t| t.mass_per_m_kg * t.contact_length_m).sum::<f64>()
    }

    /// The optional features this rig uses. A solver lane that does not implement one yet must refuse the rig with this list instead of
    /// silently ignoring the field.
    pub fn required_features(&self) -> Vec<&'static str> {
        let mut f: Vec<&'static str> = Vec::new();
        let mut add = |cond: bool, name: &'static str| {
            if cond && !f.contains(&name) {
                f.push(name);
            }
        };
        add(!self.linkages.is_empty(), feature::LINKAGES);
        add(self.stations.iter().any(|s| !s.wheel.patches_x_m.is_empty()), feature::MULTI_PATCH_WHEELS);
        add(!self.drivetrain.modes.is_empty(), feature::DRIVE_MODES);
        add(self.stations.iter().any(|s| s.arm_pivot_m.is_some()), feature::SWING_ARM);
        add(self.stations.iter().any(|s| s.suspension.damper.friction_n > 0.0), feature::DRY_FRICTION);
        add(self.linkages.iter().any(|l| l.suspension.damper.friction_n > 0.0), feature::DRY_FRICTION);
        add(
            self.stations.iter().any(|s| s.suspension.bump_stop.hard_limit)
                || self.linkages.iter().any(|l| l.suspension.bump_stop.hard_limit),
            feature::HARD_BUMP_LIMIT,
        );
        add(self.drivetrain.engine.free_output || self.drivetrain.engine.response_time_s > 0.0, feature::FREE_TURBINE);
        add(self.proxies.iter().any(|p| p.attached_station.is_some()), feature::STATION_PROXIES);
        add(self.tracks.iter().any(|t| t.resist_c0 > 0.0 || t.resist_c1_s_m > 0.0), feature::TRACK_RESISTANCE);
        for b in &self.drivetrain.brakes {
            add(b.apply_time_s > 0.0 || b.release_time_s > 0.0, feature::BRAKE_LAG);
            add(b.site == BrakeSite::Driveline, feature::BRAKE_DRIVELINE_SITE);
            add(b.steering, feature::BRAKE_STEERING);
            add((b.reverse_torque_factor - 1.0).abs() > 1e-12, feature::BRAKE_REVERSE_FACTOR);
        }
        fn walk(n: &DriveNode, add: &mut dyn FnMut(bool, &'static str)) {
            match n {
                DriveNode::Diff { split, efficiency, children, .. } => {
                    add(!split.is_empty() || (*efficiency - 1.0).abs() > 1e-12, feature::DIFF_SPLIT_OR_EFFICIENCY);
                    for c in children {
                        walk(c, add);
                    }
                }
                DriveNode::SteerUnit { law, children, .. } => {
                    add(*law != SteerLaw::default(), feature::STEER_LAW);
                    for c in children {
                        walk(c, add);
                    }
                }
                DriveNode::Output(_) => {}
            }
        }
        walk(&self.drivetrain.driveline, &mut add);
        f
    }

    /// A stable 64-bit hash of the rig's content (the canonical JSON text of the whole rig). A damaged rig has a different hash; a repaired
    /// one returns to the original. `CapabilityTable::rig_hash` is this value.
    pub fn rig_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        match serde_json::to_vec(self) {
            Ok(bytes) => h.write_bytes(&bytes),
            Err(_) => h.write_u8(0xff),
        }
        h.finish()
    }
}

pub(crate) fn collect_outputs(n: &DriveNode, out: &mut Vec<usize>) {
    match n {
        DriveNode::Output(i) => out.push(*i),
        DriveNode::Diff { children, .. } | DriveNode::SteerUnit { children, .. } => {
            for c in children {
                collect_outputs(c, out);
            }
        }
    }
}
