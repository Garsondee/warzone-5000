//! `RenderRig`: the meshes of a vehicle, tagged by articulation node, compiled by FORGE from a `VehicleDef` (shapes come from lane
//! GEOMETRY, materials slots are filled by lane LOOK). Viewers and Godot draw a vehicle by walking the node tree, applying the
//! joint coordinates of the current [`crate::frame::VehicleFrame`] to each node that has a joint binding, and drawing its meshes.

use serde::{Deserialize, Serialize};
use w5k_math::{Transform, Vec3};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderRig {
    pub id: String,
    pub nodes: Vec<RenderNode>,
    pub meshes: Vec<MeshPart>,
    pub material_slots: Vec<MaterialSlot>,
    /// Number of joint coordinates a frame carries for this rig: `PhysRig::joint_names().len()` (a coordinate may be unbound, e.g. a spare).
    pub joint_count: usize,
    /// Contract 0.3 (GEOMETRY CCR). One entry per track belt: what a viewer needs to move the belt's links round their path. Empty for a wheeled
    /// vehicle, and for a tracked one whose belt is drawn as a static mesh (the 0.2 behaviour).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub track_runs: Vec<TrackRun>,
}

/// A track belt, to be drawn as instanced links on a path. The path is the taut band round the circles of `wheels` (loop order, the
/// sprocket's pitch circle included); its length `L` is closed-form, and the `links` links sit at arc-length spacing `L / links`. A viewer
/// advances the links by the sprocket's rolling distance (`sprocket_joint` times the pitch radius, times `direction`), so the belt moves
/// exactly as the simulated sprocket turns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackRun {
    /// The `Track` node of this side; the links are drawn in its frame.
    pub node: usize,
    /// The wheels the belt wraps, in loop order (`TrackDef::stations`); the band round them is the belt path.
    pub wheels: Vec<TrackWheel>,
    /// Index into `RenderRig::meshes`: ONE link at the origin, +X along the direction of travel, +Y out of the belt (instanced).
    pub link_mesh: usize,
    /// Design link count `n`; the viewer uses the spacing `L / n` for the path length `L` it computes, so the links always close the loop.
    pub links: u16,
    /// Index into `wheels` of the driven wheel.
    pub sprocket: usize,
    /// Index into `VehicleFrame::joints` of the sprocket's spin (rad, positive = rolling forward).
    pub sprocket_joint: usize,
    /// +1 if a positive sprocket spin advances the links along the `wheels` order, -1 if against it (a front sprocket).
    pub direction: i8,
}

/// One wheel on a belt path.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackWheel {
    /// The node carrying the wheel's spin (its position follows the suspension).
    pub node: usize,
    /// The path radius, m: the wheel's tip radius plus half the belt thickness (the sprocket's pitch radius).
    pub radius_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeRole {
    Hull,
    Turret,
    GunPitch,
    Recoil,
    /// A wheel (tyre) at a station.
    Wheel,
    RoadWheel,
    Sprocket,
    Idler,
    ReturnRoller,
    /// Suspension arm or strut that visibly moves.
    SuspensionArm,
    /// Steering knuckle (rotates about the steer axis).
    SteerKnuckle,
    /// The track belt of one side (animated by belt speed).
    Track,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointAxisKind {
    /// Rotation about `axis` by the joint coordinate (rad).
    Revolute,
    /// Translation along `axis` by the joint coordinate (m).
    Prismatic,
}

/// Ties a node to one entry of `VehicleFrame::joints`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointBinding {
    pub kind: JointAxisKind,
    /// Axis in the node's parent frame.
    pub axis: Vec3,
    /// Index into `VehicleFrame::joints`.
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderNode {
    pub name: String,
    pub parent: Option<usize>,
    pub role: NodeRole,
    /// Pose in the parent's frame at joint coordinate zero.
    pub rest: Transform,
    pub joint: Option<JointBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SlotKind {
    /// Painted armour: takes the camo pattern.
    Paint,
    Metal,
    Rubber,
    Glass,
    Canvas,
    Track,
    Optics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialSlot {
    pub name: String,
    pub kind: SlotKind,
}

/// Triangle mesh in the *node's* local frame. No UVs by design: camo and weathering are triplanar, driven by position, normal and
/// the per-vertex flags below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeshPart {
    pub name: String,
    pub node: usize,
    pub material_slot: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Per-vertex edge sharpness 0..1 (for edge wear) and cavity/occlusion 0..1 (for dirt), baked by GEOMETRY. May be empty.
    pub edge: Vec<f32>,
    pub cavity: Vec<f32>,
    pub indices: Vec<u32>,
}

impl RenderRig {
    /// Structural checks (indices in range, node tree acyclic with parents first, joint indices dense).
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut e = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if let Some(p) = n.parent {
                if p >= i {
                    e.push(format!("node {i} ({}): parent {p} must precede it", n.name));
                }
            }
            if let Some(j) = n.joint {
                if j.index >= self.joint_count {
                    e.push(format!(
                        "node {i} ({}): joint index {} >= joint_count {}",
                        n.name, j.index, self.joint_count
                    ));
                }
            }
        }
        for (i, run) in self.track_runs.iter().enumerate() {
            let w = format!("track run {i}");
            if run.node >= self.nodes.len() || run.link_mesh >= self.meshes.len() {
                e.push(format!("{w}: node or link mesh out of range"));
            }
            if run.wheels.len() < 3
                || run
                    .wheels
                    .iter()
                    .any(|x| x.node >= self.nodes.len() || !(x.radius_m > 0.0 && x.radius_m.is_finite()))
            {
                e.push(format!("{w}: needs at least 3 wheels, each on an existing node with a positive radius"));
            }
            if run.sprocket >= run.wheels.len() {
                e.push(format!("{w}: sprocket {} is not one of the {} wheels", run.sprocket, run.wheels.len()));
            }
            if run.sprocket_joint >= self.joint_count {
                e.push(format!("{w}: sprocket joint {} >= joint_count {}", run.sprocket_joint, self.joint_count));
            }
            if run.links < 3 || !(run.direction == 1 || run.direction == -1) {
                e.push(format!("{w}: needs at least 3 links and a direction of +1 or -1"));
            }
        }
        for (i, m) in self.meshes.iter().enumerate() {
            if !(m.edge.is_empty() || m.edge.len() == m.positions.len())
                || !(m.cavity.is_empty() || m.cavity.len() == m.positions.len())
            {
                e.push(format!("mesh {i} ({}): edge and cavity must be empty or one value per vertex", m.name));
            }
            if m.node >= self.nodes.len() {
                e.push(format!("mesh {i} ({}): node {} out of range", m.name, m.node));
            }
            if m.material_slot >= self.material_slots.len() {
                e.push(format!("mesh {i} ({}): material slot {} out of range", m.name, m.material_slot));
            }
            if m.positions.len() != m.normals.len() {
                e.push(format!("mesh {i} ({}): positions and normals differ in length", m.name));
            }
            if m.indices.len() % 3 != 0 || m.indices.iter().any(|&k| k as usize >= m.positions.len()) {
                e.push(format!("mesh {i} ({}): bad index buffer", m.name));
            }
        }
        if e.is_empty() {
            Ok(())
        } else {
            Err(e)
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.meshes.iter().map(|m| m.indices.len() / 3).sum()
    }
}
