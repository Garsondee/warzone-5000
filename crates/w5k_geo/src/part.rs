//! A generated part: a closed mesh with the articulation node role that moves it, a material slot kind and the side it sits on.

use crate::mesh::Mesh;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::Transform;

pub use w5k_contract::rig::Side;

/// Where a part came from when a module was attached: the placement's label and the node that carries the socket it sits on (the label
/// of the placement that offered the socket and the role of its node; `None` when the socket is the hull's). The export builds the joint
/// tree from this: a gun on a ring mount's trunnion hangs from the mount's turntable.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub label: String,
    pub carrier: Option<(String, NodeRole)>,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub name: String,
    pub role: NodeRole,
    /// Axle or road-wheel index, for parts that belong to a station.
    pub station: Option<u8>,
    pub side: Side,
    pub slot: SlotKind,
    /// Mirrors, handles, lamps: excluded from the hull box that `HullDef` dimensions measure.
    pub fitting: bool,
    /// The mesh in the part's node frame (a wheel: hub at the origin, axle along X).
    pub mesh: Mesh,
    /// The node frame in the hull frame, in the vehicle's design pose (a hub position for a wheel part; identity for hull parts).
    pub pose: Transform,
    /// `None` for the hull module's own parts.
    pub placement: Option<Placement>,
}

impl Part {
    pub fn in_hull_frame(&self) -> Mesh {
        self.mesh.transformed(&self.pose)
    }
}
