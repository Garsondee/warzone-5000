//! Ring mount oracles: the generated geometry has the stated dimensions, every part is a closed outward-facing solid, the mount publishes
//! its ring and its cradle, a roof takes it only if the ring fits, and the turntable and what stands on it yaw about the collar's top face.

use w5k_contract::render::NodeRole;
use w5k_geo::module::{frame, Assembly, Module, ModuleKind, Socket, SocketKind};
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::{Part, Side};
use w5k_math::Vec3;

fn find<'a>(m: &'a Module, name: &str) -> &'a Part {
    m.parts.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no part {name}"))
}

/// A bare roof: a Ring socket 0.9 m wide, to hang the mount on without a hull.
fn roof() -> Assembly {
    let hull = Module {
        name: "roof".into(),
        kind: ModuleKind::Hull,
        parts: Vec::new(),
        sockets: vec![Socket {
            name: "roof".into(),
            kind: SocketKind::Ring,
            side: Side::Centre,
            pose: frame(Vec3::new(0.0, 1.5, 0.3), Vec3::Y, -Vec3::Z),
            size_m: 0.9,
            station: None,
            carrier: NodeRole::Hull,
            owner: None,
            hints: Vec::new(),
        }],
        mount: None,
        symmetric: true,
    };
    Assembly::new(hull)
}

#[test]
fn every_part_of_the_ring_mount_is_a_closed_outward_solid_at_every_detail() {
    for detail in 0..=2 {
        for p in &ring_mount(&RingMountDims::standard(), detail).parts {
            assert!(p.mesh.check_closed().is_ok(), "{} is not closed at detail {detail}", p.name);
            assert!(p.mesh.signed_volume() > 0.0, "{} faces inward at detail {detail}", p.name);
        }
    }
}

#[test]
fn the_collar_is_as_wide_and_as_high_as_stated_and_the_mount_publishes_its_ring_and_cradle() {
    let d = RingMountDims::standard();
    let m = ring_mount(&d, 1);
    let collar = find(&m, "collar");
    let (lo, hi) = collar.mesh.bounds();
    assert!((hi.x - lo.x - d.ring_m).abs() < 1e-9 && (hi.z - lo.z - d.ring_m).abs() < 1e-9, "outer diameter");
    assert!(lo.y.abs() < 1e-9 && (hi.y - d.collar_height_m).abs() < 1e-9, "height");
    assert!(collar.fitting && collar.role == NodeRole::Hull, "a fixed fitting, not part of the hull box");
    assert!(m.parts.iter().filter(|p| p.name != "collar").all(|p| p.role == NodeRole::Turret), "everything else yaws");
    let mount = m.mount.as_ref().unwrap();
    assert_eq!((mount.kind, mount.size_m), (SocketKind::Ring, d.ring_m));
    let t = &m.sockets[0];
    assert_eq!(
        (t.kind, t.size_m, t.hint("cradle_w_m")),
        (SocketKind::Trunnion, d.cradle_width_m, Some(d.cradle_width_m))
    );
    assert!((t.pose.pos - Vec3::new(0.0, d.collar_height_m + d.cradle_height_m, d.cradle_z_m)).length() < 1e-12);
    assert!((t.pose.apply_dir(Vec3::Y) + Vec3::Z).length() < 1e-12, "the trunnion faces forward");
}

#[test]
fn the_uprights_stand_exactly_the_cradle_width_apart_and_the_shield_wings_leave_room_for_the_gun() {
    let d = RingMountDims::standard();
    let m = ring_mount(&d, 1);
    let (r_lo, _) = find(&m, "upright.r").mesh.bounds();
    let (_, l_hi) = find(&m, "upright.l").mesh.bounds();
    assert!((r_lo.x - l_hi.x - d.cradle_width_m).abs() < 1e-9, "clear width between the uprights");
    let (wing_lo, _) = find(&m, "shield.r").mesh.bounds();
    assert!(
        wing_lo.x > d.cradle_width_m / 2.0 - d.shield_thickness_m,
        "the right wing starts beside the cradle, not across the gun"
    );
}

#[test]
fn a_ring_wider_than_the_roof_is_refused_and_one_exactly_as_wide_fits() {
    let mut wide = RingMountDims::standard();
    wide.ring_m = 1.2;
    let err = roof().attach("roof", &ring_mount(&wide, 1), 0.0, "ring").unwrap_err();
    assert!(err.reason.contains("needs 1.200 m, the socket offers 0.900 m"), "{err}");
    wide.ring_m = 0.9;
    assert!(roof().attach("roof", &ring_mount(&wide, 1), 0.0, "ring").is_ok());
}

#[test]
fn the_collar_is_baked_into_the_hull_and_the_turntable_yaws_about_the_top_face_of_the_collar() {
    let (mut asm, d) = (roof(), RingMountDims::standard());
    asm.attach("roof", &ring_mount(&d, 1), 0.0, "ring").unwrap();
    let collar = asm.parts.iter().find(|p| p.name == "collar.ring").unwrap();
    let (lo, hi) = collar.mesh.bounds();
    assert!(
        collar.pose.pos.length() < 1e-12 && (lo.y - 1.5).abs() < 1e-9 && (hi.y - 1.5 - d.collar_height_m).abs() < 1e-9
    );
    assert!((lo.z - (0.3 - d.ring_m / 2.0)).abs() < 1e-9, "centred on the socket");
    let turntable = asm.parts.iter().find(|p| p.name == "turntable.ring").unwrap();
    assert_eq!(turntable.role, NodeRole::Turret);
    assert!((turntable.pose.pos - Vec3::new(0.0, 1.5 + d.collar_height_m, 0.3)).length() < 1e-12);
    assert!(
        (turntable.pose.apply_dir(Vec3::Y) - Vec3::Y).length() < 1e-12
            && (turntable.pose.apply_dir(-Vec3::Z) + Vec3::Z).length() < 1e-12
    );
    assert!(asm.parts.iter().all(|p| p.pose.is_finite()));
}
