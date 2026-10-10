//! A generated part: a closed mesh with the articulation node role that moves it, a material slot kind and the side it sits on.

use crate::mesh::Mesh;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::Transform;

pub use w5k_contract::rig::Side;

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
}

impl Part {
    pub fn in_hull_frame(&self) -> Mesh {
        self.mesh.transformed(&self.pose)
    }
}
