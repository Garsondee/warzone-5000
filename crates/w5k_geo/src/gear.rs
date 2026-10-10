//! Running-gear families: modules that go on a hull's `Station` sockets.
//!
//! `wheel_module` is one wheel of a sprung axle: tyre, tread, rim and nuts in the wheel's node frame (hub at the origin, axle along X, the
//! outer face towards +X) and, on a steered axle, the steering knuckle, a stub from the hub back towards the hull with its own node role.
//! It is authored for the right-hand side; the left-hand wheel is its mirror image, which `Assembly::attach` makes on a left socket.

use crate::mesh::Mesh;
use crate::module::{frame, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::{cylinder_x, wheel, WheelDims};
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{Transform, Vec3};

/// A steering knuckle: a stub of `radius_m` from the hub back towards the hull for `reach_m`.
#[derive(Clone, Copy, Debug)]
pub struct Knuckle {
    pub radius_m: f64,
    pub reach_m: f64,
}

/// One wheel (and its knuckle if the axle steers). The mount faces the hull and is as big as the wheel, so it only fits a station that was
/// cut for a wheel at least this large.
pub fn wheel_module(w: &WheelDims, segments: u32, knuckle: Option<Knuckle>) -> Module {
    let wh = wheel(w, segments);
    let part = |what: &str, role: NodeRole, slot: SlotKind, mesh: Mesh| Part {
        name: what.into(),
        role,
        station: None,
        side: Side::Right,
        slot,
        fitting: false,
        mesh: mesh.finished(),
        pose: Transform::IDENTITY,
        placement: None,
    };
    let mut parts = vec![
        part("tyre", NodeRole::Wheel, SlotKind::Rubber, wh.tyre),
        part("tread", NodeRole::Wheel, SlotKind::Rubber, wh.lugs),
        part("rim", NodeRole::Wheel, SlotKind::Metal, wh.rim),
        part("nuts", NodeRole::Wheel, SlotKind::Metal, wh.nuts),
    ];
    if let Some(k) = knuckle {
        let stub = cylinder_x(k.radius_m, -k.reach_m, 0.0, 16); // const-ok: 16 sides
        parts.push(part("knuckle", NodeRole::SteerKnuckle, SlotKind::Metal, stub));
    }
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Station,
        side: Side::Right,
        pose: frame(Vec3::ZERO, -Vec3::X, -Vec3::Z),
        size_m: w.outer_radius_m,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    Module {
        name: "wheel".into(),
        kind: ModuleKind::Gear,
        parts,
        sockets: Vec::new(),
        mount: Some(mount),
        symmetric: false,
    }
}
