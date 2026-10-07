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

/// A vehicle: a hull part plus parts attached to its sockets (recursively).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignDef {
    pub id: String,
    pub name: String,
    pub hull: String,
    #[serde(default)]
    pub palette: Option<String>,
    #[serde(default)]
    pub attach: Vec<Attach>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attach {
    /// Socket on the parent part.
    pub socket: String,
    pub part: String,
    /// Mirror the child left-right before attaching (for symmetric pairs such as left/right tracks).
    #[serde(default)]
    pub mirror: bool,
    /// Extra rotation (degrees) about the socket normal, for poses such as turret yaw or leg splay.
    #[serde(default)]
    pub spin: f64,
    #[serde(default)]
    pub children: Vec<Attach>,
}
