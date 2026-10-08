//! What the simulation needs to know about a vehicle, baked from its design.
//!
//! The forge measures a design in floats (mass, power, running gear...) and writes a [`MoverSpec`]; the simulation converts it
//! to fixed point once, when a run starts (`Mover::new`), and never sees a float again. Everything here is in the simulation's
//! coherent units: tonnes, kilowatts, metres, seconds (and kN for forces, which follows).

use serde::{Deserialize, Serialize};

/// The kind of running gear, which decides how the ground and the air push back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GearClass {
    Tracks,
    Wheels,
    Legs,
    /// An air cushion on skirts: no grip, a low static thrust, drag that grows with speed.
    Cushion,
    AntiGrav,
    Rotor,
    /// Needs rails; the courses do not have any.
    Rail,
}

impl GearClass {
    /// Flies or floats: the path it follows is the ground smoothed over its altitude, and the ground's material does not matter.
    pub fn airborne(self) -> bool {
        matches!(self, GearClass::AntiGrav | GearClass::Rotor)
    }

    /// Stands on the ground and pushes against it: the soil decides how far it sinks and how much it can push with.
    pub fn grounded(self) -> bool {
        matches!(self, GearClass::Tracks | GearClass::Wheels | GearClass::Legs)
    }

    pub fn name(self) -> &'static str {
        match self {
            GearClass::Tracks => "tracks",
            GearClass::Wheels => "wheels",
            GearClass::Legs => "legs",
            GearClass::Cushion => "air cushion",
            GearClass::AntiGrav => "anti-gravity",
            GearClass::Rotor => "rotor",
            GearClass::Rail => "rail",
        }
    }
}

fn default_launch() -> f64 {
    0.2
}

/// One vehicle, as the simulation sees it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoverSpec {
    pub id: String,
    pub class: GearClass,
    pub mass_t: f64,
    /// Power reaching the running gear (kW): engine power, less what other systems draw and what lift costs, times the
    /// efficiency of the drive.
    pub drive_kw: f64,
    /// The fastest the running gear allows (m/s).
    pub rated_ms: f64,
    /// Rolling resistance as a fraction of the load on the ground.
    pub c_roll: f64,
    /// Power the running gear spends per unit of weight per unit of speed, whatever the ground does: the cost of
    /// transport of a walker. It comes off the engine's force *before* the grip limit, because the ground does not pay it.
    pub c_internal: f64,
    /// Grip: the most force the ground lets the running gear push with, as a fraction of the load on it. Zero when the
    /// gear does not push against the ground (a cushion, anti-gravity, a rotor).
    pub grip_mu: f64,
    /// For gear that does not grip: the most thrust it can make, as a fraction of weight.
    pub thrust_w: f64,
    /// Rolling resistance that grows with speed (a skirt dragging over the ground) instead of being constant.
    #[serde(default)]
    pub skirt_drag: bool,
    /// Drag area, C_d x frontal area (m^2).
    pub cd_a_m2: f64,
    /// Distance between the points the body rests on (m): sets its pitch and the slope it feels.
    pub span_m: f64,
    /// Altitude it flies at, for the airborne classes (m): the slope it feels is averaged over twice this.
    #[serde(default)]
    pub altitude_m: f64,
    /// Below this fraction of the rated speed the drive pushes with constant force (the lowest gear), instead of
    /// force = power / speed, which would be infinite at a standstill.
    #[serde(default = "default_launch")]
    pub launch_floor: f64,
    /// How many separate contact units the running gear has (two tracks, six wheels, eight feet); zero for gear that does not
    /// touch the ground.
    #[serde(default)]
    pub contact_units: u32,
    /// Width of one contact unit (m): its smaller dimension, which sets how stiff the soil is under it.
    #[serde(default)]
    pub contact_width_m: f64,
    /// Length of one contact unit along the direction of travel (m): how far it shears the soil.
    #[serde(default)]
    pub contact_len_m: f64,
    /// Total ground contact area (m^2): the weight over it is the ground pressure.
    #[serde(default)]
    pub contact_area_m2: f64,
    /// Legs: distance travelled between two steps of one foot (m).
    #[serde(default)]
    pub stride_m: f64,
    /// Hull belly above the ground (m); zero when unknown (no hull dragging).
    #[serde(default)]
    pub clearance_m: f64,
    /// Why this vehicle cannot run the course at all, if it cannot.
    #[serde(default)]
    pub dns: Option<String>,
}

impl Default for MoverSpec {
    /// A 1 t tracked vehicle with nothing else set: a starting point for tests and tools (`..Default::default()`).
    fn default() -> Self {
        MoverSpec {
            id: String::new(),
            class: GearClass::Tracks,
            mass_t: 1.0,
            drive_kw: 0.0,
            rated_ms: 0.0,
            c_roll: 0.0,
            c_internal: 0.0,
            grip_mu: 0.0,
            thrust_w: 0.0,
            skirt_drag: false,
            cd_a_m2: 0.0,
            span_m: 1.0,
            altitude_m: 0.0,
            launch_floor: default_launch(),
            contact_units: 0,
            contact_width_m: 0.0,
            contact_len_m: 0.0,
            contact_area_m2: 0.0,
            stride_m: 0.0,
            clearance_m: 0.0,
            dns: None,
        }
    }
}
