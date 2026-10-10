//! Weapon-mount families. A ring mount is a collar fixed to the hull (role `Hull`, a fitting), a turntable that yaws on it, a cradle of two
//! uprights that holds a gun's trunnion, and a shield of two swept wings either side of the gun (all role `Turret`, one node frame at the
//! centre of the ring's top face). As a module it mounts on a `Ring` socket (sized by its ring diameter, so a roof takes it only if the ring
//! fits) and offers one `Trunnion` socket (sized by the cradle's clear width, which it also publishes as the hint `cradle_w_m` for the gun
//! to read). It is authored standing up: mount normal down, reference forward.

use crate::hardware::{bevel_box, cyl_y, revolve_y};
use crate::mesh::Mesh;
use crate::module::{frame, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::segments_for;
use serde::Deserialize;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{scalar, Quat, Transform, Vec3};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RingMountDims {
    pub ring_m: f64,
    pub collar_height_m: f64,
    pub collar_width_m: f64,
    pub turntable_m: f64,
    pub cradle_width_m: f64,
    pub cradle_height_m: f64,
    pub cradle_z_m: f64,
    pub upright_thickness_m: f64,
    pub upright_length_m: f64,
    pub shield_width_m: f64,
    pub shield_height_m: f64,
    pub shield_thickness_m: f64,
    pub shield_sweep_deg: f64,
    pub shield_z_m: f64,
    pub overlap_m: f64,
    pub corner_m: f64,
    pub band_m: f64,
}

impl RingMountDims {
    /// `shapes/ring_mount.ron`.
    pub fn standard() -> RingMountDims {
        ron::from_str(include_str!("../shapes/ring_mount.ron")).expect("ring_mount.ron parses")
    }
}

/// The ring mount module (frame: origin at the centre of the collar's bottom face on the hull, +Y up, -Z forward).
pub fn ring_mount(d: &RingMountDims, detail: u8) -> Module {
    let (n, r, e, o) = (segments_for(detail) / 2, d.ring_m / 2.0, d.corner_m, d.overlap_m); // const-ok: half the wheel segments for a ring this size
    let (ch, cw) = (d.collar_height_m, d.collar_width_m);
    let part = |name: &str, role: NodeRole, slot: SlotKind, side: Side, mesh: Mesh, pose: Transform| Part {
        name: name.into(),
        role,
        station: None,
        side,
        slot,
        fitting: role == NodeRole::Hull,
        mesh: mesh.finished(),
        pose,
        placement: None,
    };
    // the collar: a stepped ring, a chamfer on its top outer edge
    let collar = revolve_y(&[(r - cw, 0.0), (r, 0.0), (r, ch - e), (r - e, ch), (r - cw, ch)], n);
    let node = Transform::from_pos(Vec3::new(0.0, ch, 0.0)); // the turntable's frame: the collar's top face
    let (t, pivot) = (d.turntable_m, Vec3::new(0.0, d.cradle_height_m, d.cradle_z_m));
    let mut parts = vec![
        part("collar", NodeRole::Hull, SlotKind::Metal, Side::Centre, collar, Transform::IDENTITY),
        part(
            "turntable",
            NodeRole::Turret,
            SlotKind::Metal,
            Side::Centre,
            cyl_y(0.0, 0.0, r - cw / 2.0, -o, t, n),
            node,
        ),
    ];
    // right-hand upright and shield wing, then their mirror images
    let (ut, ul) = (d.upright_thickness_m, d.upright_length_m);
    let top = pivot.y + ut; // the uprights stand a little above the pivot
    let upright = bevel_box(
        Vec3::new(d.cradle_width_m / 2.0 + ut / 2.0, (top - o) / 2.0, pivot.z),
        [ut, top + o, ul],
        e,
        d.band_m,
    );
    let sweep = d.shield_sweep_deg * scalar::PI / 180.0; // const-ok: degrees to radians at the authoring edge
    let half = d.shield_width_m / 2.0;
    let wing = bevel_box(Vec3::ZERO, [d.shield_width_m, d.shield_height_m, d.shield_thickness_m], e, d.band_m)
        .transformed(&Transform::new(
            Vec3::new(
                d.cradle_width_m / 2.0 + half * scalar::cos(sweep),
                t - o + d.shield_height_m / 2.0,
                d.shield_z_m + half * scalar::sin(sweep),
            ),
            Quat::from_yaw(-sweep),
        ));
    for (name, slot, mesh) in [("upright", SlotKind::Metal, upright), ("shield", SlotKind::Paint, wing)] {
        parts.push(part(&format!("{name}.r"), NodeRole::Turret, slot, Side::Right, mesh.clone(), node));
        parts.push(part(&format!("{name}.l"), NodeRole::Turret, slot, Side::Left, mesh.mirrored_x(), node));
    }
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Ring,
        side: Side::Centre,
        pose: frame(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        size_m: d.ring_m,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    let trunnion = Socket {
        name: "trunnion".into(),
        kind: SocketKind::Trunnion,
        side: Side::Centre,
        pose: frame(Vec3::new(0.0, ch + pivot.y, pivot.z), -Vec3::Z, Vec3::Y),
        size_m: d.cradle_width_m,
        station: None,
        carrier: NodeRole::Turret, // the trunnion rides on the turntable
        owner: None,
        hints: vec![("cradle_w_m".into(), d.cradle_width_m)],
    };
    Module {
        name: "ring_mount".into(),
        kind: ModuleKind::Mount,
        parts,
        sockets: vec![trunnion],
        mount: Some(mount),
        symmetric: true,
    }
}
