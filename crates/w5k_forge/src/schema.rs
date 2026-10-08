//! Content file formats (RON). See docs/design/03-part-forge.md for the authoring guide.
//!
//! Conventions: metres and kilograms; +Y is up, **forward is -Z** and right is +X (Godot's convention);
//! rotations are Euler angles in degrees applied X, then Y, then Z.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn zero3() -> [f64; 3] {
    [0.0, 0.0, 0.0]
}
fn up() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}
fn fwd() -> [f64; 3] {
    [0.0, 0.0, -1.0]
}
fn one() -> f64 {
    1.0
}
fn steel() -> String {
    "steel".into()
}
fn segs() -> u32 {
    12
}
fn tru() -> bool {
    true
}

/// Colour slots. The palette (and the team colour) decide the actual colours; geometry only names the role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    #[default]
    Primary,
    Secondary,
    Trim,
    Dark,
    Metal,
    Rubber,
    Glow,
    Glass,
}

impl Slot {
    pub const ALL: [Slot; 8] = [Slot::Primary, Slot::Secondary, Slot::Trim, Slot::Dark, Slot::Metal, Slot::Rubber, Slot::Glow, Slot::Glass];
    pub fn index(self) -> u8 {
        Slot::ALL.iter().position(|s| *s == self).unwrap() as u8
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    X,
    #[default]
    Y,
    Z,
}

impl Axis {
    pub fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    #[default]
    Hull,
    Turret,
    Weapon,
    Locomotion,
    Engine,
    Sensor,
    Utility,
    Leg,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeClass {
    Tiny,
    Small,
    #[default]
    Medium,
    Large,
    Huge,
    Titanic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketKind {
    /// Where this part attaches to its parent.
    #[default]
    Mount,
    TurretRing,
    Hardpoint,
    Locomotion,
    Utility,
    Internal,
    LegHip,
    /// Full-length mount on a hull side (track units).
    Gear,
    /// One of several discrete mounts along a hull side (wheels, legs, pods, rotor arms).
    Station,
    /// A radial hip on a round walker body (legs, pods, rotor arms).
    Hip,
    /// Centre-line mount under the hull (rail bogies).
    Keel,
    /// The whole underside (air cushions, anti-gravity plates).
    Belly,
    /// Top-side mount for masts and utility rigs.
    Mast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locomotion {
    Wheels,
    HalfTracks,
    Tracks,
    Hover,
    AntiGrav,
    Legs,
    Rotor,
    Jet,
    Rail,
}

/// One node in a part's shape tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Node {
    /// Box centred on `at`. `taper` scales the top face (x, z); `shift` moves the top face (x, z).
    Box {
        size: [f64; 3],
        #[serde(default)]
        taper: Option<[f64; 2]>,
        #[serde(default)]
        shift: Option<[f64; 2]>,
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// Box whose top front edge (toward -Z) slopes down: `slope` is the fraction of the length that slopes,
    /// `nose` the front height as a fraction of the full height.
    Wedge {
        size: [f64; 3],
        #[serde(default = "half")]
        slope: f64,
        #[serde(default)]
        nose: f64,
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// Faceted cylinder or prism along `axis`; `taper` scales the far end's radius (a frustum when not 1).
    Cylinder {
        radius: f64,
        length: f64,
        #[serde(default)]
        axis: Axis,
        #[serde(default = "segs")]
        segments: u32,
        #[serde(default = "one")]
        taper: f64,
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// Faceted sphere (or ellipsoid when `scale` is given).
    Sphere {
        radius: f64,
        #[serde(default = "segs")]
        segments: u32,
        #[serde(default)]
        scale: Option<[f64; 3]>,
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// A straight member from `from` to `to` (legs, struts, arms, rams): a box section `size` = (width,
    /// height) at `from`, tapering to `end` (default: the same) at `to`. The height is measured along `up`
    /// (made perpendicular to the beam); the width across it.
    Beam {
        from: [f64; 3],
        to: [f64; 3],
        size: [f64; 2],
        #[serde(default)]
        end: Option<[f64; 2]>,
        #[serde(default = "up")]
        up: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// Convex hull of explicit points.
    Hull {
        points: Vec<[f64; 3]>,
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "steel")]
        mat: String,
        #[serde(default)]
        slot: Slot,
        #[serde(default)]
        shell: Option<f64>,
        #[serde(default)]
        chamfer: f64,
    },
    /// Transform a group of children.
    Group {
        #[serde(default = "zero3")]
        at: [f64; 3],
        #[serde(default = "zero3")]
        rot: [f64; 3],
        #[serde(default = "one")]
        scale: f64,
        children: Vec<Node>,
    },
    /// The children plus their mirror image across the plane through the origin normal to `axis`.
    Mirror {
        #[serde(default = "x_axis")]
        axis: Axis,
        #[serde(default = "tru")]
        keep: bool,
        children: Vec<Node>,
    },
    /// `count` copies, each offset by `step`.
    Array { count: u32, step: [f64; 3], children: Vec<Node> },
    /// `count` copies rotated evenly about `axis` through the origin, starting at `phase` degrees.
    Radial {
        count: u32,
        #[serde(default)]
        axis: Axis,
        #[serde(default)]
        phase: f64,
        children: Vec<Node>,
    },
}

fn half() -> f64 {
    0.5
}
fn x_axis() -> Axis {
    Axis::X
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocketDef {
    pub name: String,
    #[serde(default)]
    pub kind: SocketKind,
    #[serde(default)]
    pub size: SizeClass,
    #[serde(default = "zero3")]
    pub at: [f64; 3],
    /// Outward direction of the socket (for a mount: the direction the part attaches toward its parent).
    #[serde(default = "up")]
    pub normal: [f64; 3],
    #[serde(default = "fwd")]
    pub forward: [f64; 3],
    /// Sizes for whatever attaches here (for example `ctx.length` for a track unit). A parametric family reads
    /// them alongside its slider values.
    #[serde(default)]
    pub hints: BTreeMap<String, f64>,
}

/// What a part does, beyond its shape. All fields optional.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Function {
    /// Mechanical power produced (kW).
    #[serde(default)]
    pub power_kw: f64,
    /// Power consumed when active (kW).
    #[serde(default)]
    pub draw_kw: f64,
    /// For locomotion parts: mass it can carry (kg).
    #[serde(default)]
    pub load_kg: f64,
    #[serde(default)]
    pub locomotion: Option<Locomotion>,
    /// Rolling-resistance coefficient for locomotion parts.
    #[serde(default)]
    pub rolling: Option<f64>,
    /// Traction coefficient (fraction of weight usable as driving force).
    #[serde(default)]
    pub traction: Option<f64>,
    /// Sensor range (m) for sensor parts.
    #[serde(default)]
    pub sensor_m: f64,
    /// For rotor parts: rotor radius (m). Hover power depends on the total disc area.
    #[serde(default)]
    pub rotor_radius_m: f64,
    /// For locomotion parts: the fastest this running gear allows (gearing, suspension or gait), km/h.
    #[serde(default)]
    pub max_kmh: Option<f64>,
    /// For ground locomotion: area in contact with the ground (m^2). Weight over contact area is ground pressure.
    #[serde(default)]
    pub contact_m2: f64,
    /// Height of the mount point above the ground when the vehicle operates (m). The assembler raises the hull to
    /// match: legs lift it by their stance, air cushions by their skirt, anti-gravity pods by their ride height.
    #[serde(default)]
    pub ride_height_m: Option<f64>,
    /// Tallest obstacle this running gear steps or floats over (m).
    #[serde(default)]
    pub step_m: f64,
    /// Rail-bound running gear: the vehicle can only go where rails go.
    #[serde(default)]
    pub rail_bound: bool,
    /// Air cushion: area, perimeter and leakage gap (m^2, m, m). Lift power depends on them and on the weight.
    #[serde(default)]
    pub cushion_area_m2: f64,
    #[serde(default)]
    pub cushion_perimeter_m: f64,
    #[serde(default)]
    pub cushion_gap_m: f64,
    /// Anti-gravity: power cost per tonne held up (kW/t).
    #[serde(default)]
    pub grav_kw_per_t: f64,
    /// Turret ring diameter (m); checked against the hull socket's `ctx.ring_max`.
    #[serde(default)]
    pub ring_m: f64,
    /// Length of the contact patch along the direction of travel (m), for turning and wheelbase estimates.
    #[serde(default)]
    pub contact_len_m: f64,
    /// Weapon summary.
    #[serde(default)]
    pub weapon: Option<WeaponFn>,
    /// Sensor summary.
    #[serde(default)]
    pub sensor: Option<SensorFn>,
    /// Repair rig: structure restored per second (kg/s) and reach (m).
    #[serde(default)]
    pub repair_kg_s: f64,
    #[serde(default)]
    pub repair_reach_m: f64,
}

/// What a weapon delivers, in physical units (the battle simulation and the vehicle sheet both read this).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WeaponFn {
    /// "gun", "missile" or "beam".
    pub kind: String,
    /// Energy delivered per shot (J): kinetic energy, warhead energy or beam burst energy.
    pub energy_j: f64,
    /// Shots per minute when the weapon can fire continuously (each missile counts as a shot).
    pub shots_per_min: f64,
    /// Armour penetration at 1 km (mm of steel).
    pub penetration_mm: f64,
    /// Longest effective range (m).
    pub range_m: f64,
    /// Recoil impulse per shot (N s).
    pub recoil_ns: f64,
    /// Power drawn while firing (kW): energy weapons.
    pub burst_kw: f64,
    /// Whether the weapon steers itself onto its target.
    pub guided: bool,
    /// Shots released by one trigger pull (a missile salvo); 0 counts as 1.
    pub salvo: f64,
}

/// What a sensor sees: its own limit and its height above the vehicle's mount (the horizon grows with height).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SensorFn {
    /// "optical", "radar" or "combined".
    pub kind: String,
    /// Detection range before the horizon limit (m).
    pub range_m: f64,
    /// Height of the sensor head above the part's mount (m).
    pub height_m: f64,
}

impl PartDef {
    pub fn vital_interior(&self) -> bool {
        self.vital.unwrap_or(matches!(self.category, Category::Hull | Category::Turret))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: Category,
    #[serde(default)]
    pub size: SizeClass,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub palette: Option<String>,
    /// Voxel cells along the longest axis for statistics (default 64).
    #[serde(default)]
    pub voxels: Option<u32>,
    /// Whether the inside of this part's shells is vital space (crew, ammunition, engine bay) that a shot must
    /// not reach. Defaults to true for hulls and turrets; the hollow of a leg or a mast is just air.
    #[serde(default)]
    pub vital: Option<bool>,
    pub shapes: Vec<Node>,
    #[serde(default)]
    pub sockets: Vec<SocketDef>,
    #[serde(default)]
    pub function: Function,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    /// kg per cubic metre.
    pub density: f64,
    /// Protection per metre relative to rolled homogeneous steel (1.0).
    #[serde(default)]
    pub hardness: f64,
    /// Whether this material protects what is behind it. Guns, engines and electronics do not: a shot that
    /// hits them damages them instead (`false` makes them transparent to the armour tables).
    #[serde(default = "tru")]
    pub armour: bool,
    /// Working strength (MPa), for structures that must carry loads (legs, masts). 0 means a weak 100 MPa.
    #[serde(default)]
    pub strength_mpa: f64,
    /// Stiffness (Young's modulus, GPa), for buckling. 0 means a soft 10 GPa.
    #[serde(default)]
    pub modulus_gpa: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Palette {
    /// sRGB hex colours, one per `Slot` name.
    pub colours: BTreeMap<Slot, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub materials: BTreeMap<String, Material>,
    pub palettes: BTreeMap<String, Palette>,
}

/// A vehicle: a hull plus parts attached to its sockets (recursively). The hull and each attachment are either a
/// fixed part (`part`) or a parametric family with slider values (`family` + `params`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignDef {
    pub id: String,
    pub name: String,
    /// A part id or a family id.
    pub hull: String,
    /// Slider values when `hull` names a family.
    #[serde(default)]
    pub hull_params: BTreeMap<String, f64>,
    #[serde(default)]
    pub palette: Option<String>,
    #[serde(default)]
    pub attach: Vec<Attach>,
    /// How far the hull is raised so the running gear reaches the ground (m). Computed when a design is
    /// instantiated from its running gear's `ride_height_m`; leave at 0 in content files.
    #[serde(default)]
    pub lift_m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attach {
    /// Socket on the parent part. A name with one `*` (such as `station_*`) attaches to every matching socket;
    /// sockets on the left side (negative x normal) are mirrored automatically.
    pub socket: String,
    /// A fixed part id (leave empty when `family` is given).
    #[serde(default)]
    pub part: String,
    /// A parametric family id, generated with `params` plus the parent socket's hints.
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    /// Mirror the child left-right before attaching (for symmetric pairs such as left/right tracks).
    #[serde(default)]
    pub mirror: bool,
    /// Extra rotation (degrees) about the socket normal, for poses such as turret yaw or leg splay.
    #[serde(default)]
    pub spin: f64,
    #[serde(default)]
    pub children: Vec<Attach>,
}
