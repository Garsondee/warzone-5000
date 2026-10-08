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
