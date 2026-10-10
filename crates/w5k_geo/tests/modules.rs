//! Module and socket oracles: a socket frame has its normal on +Y and its reference on -Z; a module sits on a socket with its mount normal
//! opposed to the socket's and the two references agreeing; a left socket takes the mirror image; kind and size gate the fit and a refusal
//! leaves the assembly as it was; the sockets an attached module offers chain (hull, mount, weapon).

use w5k_contract::render::{NodeRole, SlotKind};
use w5k_geo::mesh::Mesh;
use w5k_geo::module::{frame, Assembly, Module, ModuleKind, Socket, SocketKind};
use w5k_geo::part::{Part, Side};
use w5k_math::{scalar, Transform, Vec3};

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-9
}

fn cuboid(centre: Vec3, size: Vec3) -> Mesh {
    let (x, z) = (size.x / 2.0, size.z / 2.0);
    Mesh::extrude_fan(&[[-x, -z], [-x, z], [x, z], [x, -z]], 0, size.y)
        .transformed(&Transform::from_pos(centre - Vec3::new(0.0, size.y / 2.0, 0.0)))
}

/// Not its own mirror image: an L-shaped prism whose long legs lie on +x and +z of its origin.
fn lump() -> Mesh {
    let l = |x: f64, z: f64| [0.4 * x, 0.4 * z];
    Mesh::extrude_fan(&[l(0.0, 0.0), l(0.0, 1.0), l(1.0, 1.0), l(1.0, 0.5), l(0.5, 0.5), l(0.5, 0.0)], 4, 0.3)
}

fn part(name: &str, role: NodeRole, mesh: Mesh, pose: Transform) -> Part {
    Part {
        name: name.to_string(),
        role,
        station: None,
        side: Side::Centre,
        slot: SlotKind::Paint,
        fitting: false,
        mesh,
        pose,
    }
}

fn marker(pose: Transform) -> Part {
    part("marker", NodeRole::Turret, cuboid(Vec3::ZERO, Vec3::splat(0.05)), pose)
}

fn socket(name: &str, kind: SocketKind, side: Side, pose: Transform, size_m: f64) -> Socket {
    Socket { name: name.to_string(), kind, side, pose, size_m, station: None, hints: Vec::new() }
}

fn hull(sockets: Vec<Socket>) -> Module {
    let body = cuboid(Vec3::new(0.0, 0.75, 0.0), Vec3::new(2.0, 1.5, 4.0));
    let parts = vec![part("body", NodeRole::Hull, body, Transform::IDENTITY)];
    Module { name: "hull".into(), kind: ModuleKind::Hull, parts, sockets, mount: None, symmetric: true }
}

fn module(
    name: &str,
    kind: ModuleKind,
    mount: Socket,
    parts: Vec<Part>,
    sockets: Vec<Socket>,
    symmetric: bool,
) -> Module {
    Module { name: name.into(), kind, parts, sockets, mount: Some(mount), symmetric }
}

fn find<'a>(asm: &'a Assembly, name: &str) -> &'a Part {
    asm.parts.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no part {name}"))
}

/// A ring mount: a collar fixed to the hull, a turntable that yaws, and a trunnion socket on the front of the turntable.
fn ring_mount(ring_m: f64) -> Module {
    let mount = socket("mount", SocketKind::Ring, Side::Centre, frame(Vec3::ZERO, -Vec3::Y, -Vec3::Z), ring_m);
    let collar =
        part("collar", NodeRole::Hull, cuboid(Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.9, 0.2, 0.9)), Transform::IDENTITY);
    let race = part(
        "race",
        NodeRole::Turret,
        cuboid(Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.7, 0.2, 0.7)),
        Transform::from_pos(Vec3::new(0.0, 0.2, 0.0)),
    );
    let trunnion = socket(
        "trunnion",
        SocketKind::Trunnion,
        Side::Centre,
        frame(Vec3::new(0.0, 0.5, -0.2), -Vec3::Z, Vec3::Y),
        0.3,
    );
    module("ring_mount", ModuleKind::Mount, mount, vec![collar, race], vec![trunnion], true)
}

/// A gun authored in its own frame: pivot at the origin, barrel along -Z, up +Y; its mount faces back.
fn gun(cradle_m: f64) -> Module {
    let mount = socket("mount", SocketKind::Trunnion, Side::Centre, frame(Vec3::ZERO, Vec3::Z, Vec3::Y), cradle_m);
    let barrel = part(
        "barrel",
        NodeRole::GunPitch,
        cuboid(Vec3::new(0.0, 0.0, -0.6), Vec3::new(0.1, 0.1, 1.2)),
        Transform::IDENTITY,
    );
    module("gun", ModuleKind::Weapon, mount, vec![barrel], vec![], true)
}

fn roof_hull() -> Assembly {
    let roof = frame(Vec3::new(0.0, 1.5, 0.3), Vec3::Y, -Vec3::Z);
    Assembly::new(hull(vec![socket("roof", SocketKind::Ring, Side::Centre, roof, 0.9)]))
}

#[test]
fn a_socket_frame_has_its_normal_on_plus_y_and_its_reference_on_minus_z() {
    let mut cases = vec![
        (Vec3::Y, -Vec3::Z),
        (-Vec3::Y, Vec3::Z),
        (Vec3::Y, Vec3::Z),
        (-Vec3::Y, -Vec3::Z),
        (Vec3::X, -Vec3::Z),
        (-Vec3::Z, Vec3::Y),
        (Vec3::new(1.0, 2.0, -3.0), Vec3::new(0.3, -0.4, -1.0)),
    ];
    for i in 0..200 {
        let (a, b) = (0.7 * f64::from(i), 0.31 * f64::from(i) - 2.0);
        let n = Vec3::new(scalar::cos(a) * scalar::cos(b), scalar::sin(b), scalar::sin(a) * scalar::cos(b));
        let r = Vec3::new(scalar::sin(1.3 * a + 0.4), scalar::cos(2.1 * b), scalar::sin(a - b) + 0.1);
        if n.normalized_or_zero().dot(r.normalized_or_zero()).abs() < 0.98 {
            cases.push((n, r));
        }
    }
    for (n, r) in cases {
        let f = frame(Vec3::new(1.0, 2.0, 3.0), n, r);
        let n_hat = n.normalized_or_zero();
        let r_hat = (r - n_hat * r.dot(n_hat)).normalized_or_zero();
        assert!(near(f.pos, Vec3::new(1.0, 2.0, 3.0)));
        assert!(near(f.apply_dir(Vec3::Y), n_hat), "normal {n:?}");
        assert!(near(f.apply_dir(-Vec3::Z), r_hat), "reference {r:?}");
        assert!(near(f.apply_dir(Vec3::X), n_hat.cross(-r_hat)), "right-handed");
        assert!((f.rot.length() - 1.0).abs() < 1e-12);
    }
}

#[test]
fn a_module_authored_standing_up_sits_upright_on_a_roof_socket() {
    let mut asm = roof_hull();
    asm.attach("roof", &ring_mount(0.8), 0.0, "ring").unwrap();
    let race = find(&asm, "race.ring");
    assert!(near(race.pose.pos, Vec3::new(0.0, 1.5 + 0.2, 0.3)), "the turntable sits 0.2 m above the roof socket");
    assert!(near(race.pose.apply_dir(Vec3::Y), Vec3::Y) && near(race.pose.apply_dir(-Vec3::Z), -Vec3::Z));
}

#[test]
fn an_attached_module_has_its_mount_normal_opposed_to_the_sockets_and_the_two_references_together() {
    let sockets = [
        frame(Vec3::new(0.2, 1.1, -0.7), Vec3::Y, -Vec3::Z),
        frame(Vec3::new(1.0, 0.4, 0.5), Vec3::X, -Vec3::Z),
        frame(Vec3::new(0.0, 1.0, -2.0), -Vec3::Z, Vec3::Y),
        frame(Vec3::new(-0.3, 0.8, 1.9), Vec3::new(0.2, 0.5, 1.0), Vec3::Y),
    ];
    let mounts = [
        frame(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        frame(Vec3::new(0.1, -0.2, 0.3), Vec3::Z, Vec3::Y),
        frame(Vec3::new(-0.4, 0.0, 0.1), Vec3::new(-1.0, 1.0, 0.3), Vec3::new(0.2, 0.1, -1.0)),
    ];
    for sf in sockets {
        for mf in mounts {
            let mut asm = Assembly::new(hull(vec![socket("s", SocketKind::Ring, Side::Centre, sf, 1.0)]));
            let m = module(
                "m",
                ModuleKind::Mount,
                socket("mount", SocketKind::Ring, Side::Centre, mf, 0.5),
                vec![marker(mf)],
                vec![],
                true,
            );
            asm.attach("s", &m, 0.0, "m").unwrap();
            let placed = find(&asm, "marker.m").pose;
            assert!(near(placed.pos, sf.pos), "the mount frame lies on the socket frame");
            assert!(near(placed.apply_dir(Vec3::Y), -sf.apply_dir(Vec3::Y)), "normals opposed");
            assert!(near(placed.apply_dir(-Vec3::Z), sf.apply_dir(-Vec3::Z)), "references together");
        }
    }
}

#[test]
fn a_gun_authored_in_its_own_frame_points_its_barrel_forward_on_a_trunnion_that_faces_forward() {
    let mut asm = roof_hull();
    asm.attach("roof", &ring_mount(0.8), 0.0, "ring").unwrap();
    let pivot = Vec3::new(0.0, 1.5 + 0.5, 0.3 - 0.2);
    assert!(near(asm.socket("trunnion.ring").unwrap().pose.pos, pivot));
    asm.attach("trunnion.ring", &gun(0.25), 0.0, "mg").unwrap();
    let barrel = find(&asm, "barrel.mg");
    assert!(
        near(barrel.pose.pos, pivot)
            && near(barrel.pose.apply_dir(Vec3::Y), Vec3::Y)
            && near(barrel.pose.apply_dir(-Vec3::Z), -Vec3::Z)
    );
    let (lo, hi) = barrel.in_hull_frame().bounds();
    assert!(
        (lo.z - (pivot.z - 1.2)).abs() < 1e-9 && (hi.z - pivot.z).abs() < 1e-9,
        "the muzzle is 1.2 m in front of the pivot"
    );
}

#[test]
fn a_chain_of_modules_names_its_parts_and_sockets_by_where_they_are_and_leaves_the_rest_open() {
    let mut asm = roof_hull();
    assert_eq!(asm.open_sockets(SocketKind::Ring).len(), 1);
    assert!(asm.open_sockets(SocketKind::Trunnion).is_empty());
    asm.attach("roof", &ring_mount(0.8), 0.0, "ring").unwrap();
    assert!(asm.open_sockets(SocketKind::Ring).is_empty(), "the roof is taken");
    let names: Vec<_> = asm.open_sockets(SocketKind::Trunnion).iter().map(|s| s.name.clone()).collect();
    assert_eq!(names, ["trunnion.ring"]);
    asm.attach("trunnion.ring", &gun(0.25), 0.0, "mg").unwrap();
    assert!(asm.open_sockets(SocketKind::Trunnion).is_empty());
    let log: Vec<_> = asm.log.iter().map(|l| (l.0.as_str(), l.1.as_str(), l.2.as_str())).collect();
    assert_eq!(log, [("ring", "roof", "ring_mount"), ("mg", "trunnion.ring", "gun")]);
    let parts: Vec<_> = asm.parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(parts, ["body", "collar.ring", "race.ring", "barrel.mg"]);
}

#[test]
fn a_part_fixed_to_the_hull_is_baked_into_the_hull_frame_and_a_moving_part_keeps_its_node_frame() {
    let mut asm = roof_hull();
    asm.attach("roof", &ring_mount(0.8), 0.0, "ring").unwrap();
    let collar = find(&asm, "collar.ring");
    assert!(
        near(collar.pose.pos, Vec3::ZERO)
            && near(collar.pose.apply_dir(Vec3::X), Vec3::X)
            && near(collar.pose.apply_dir(Vec3::Y), Vec3::Y)
    );
    let (lo, hi) = collar.mesh.bounds();
    assert!(
        (lo.y - 1.5).abs() < 1e-9 && (hi.y - 1.7).abs() < 1e-9 && (lo.z - (0.3 - 0.45)).abs() < 1e-9,
        "the collar sits on the roof"
    );
    let race = find(&asm, "race.ring");
    let (lo, hi) = race.mesh.bounds();
    assert!(lo.y.abs() < 1e-9 && (hi.y - 0.2).abs() < 1e-9, "the turntable's mesh stays in its own node frame");
    assert!(race.in_hull_frame().signed_volume() > 0.0);
}

#[test]
fn a_positive_spin_turns_a_module_counter_clockwise_about_the_socket_normal() {
    let mut asm = roof_hull();
    let mount = socket("mount", SocketKind::Ring, Side::Centre, frame(Vec3::ZERO, -Vec3::Y, -Vec3::Z), 0.5);
    let m = module(
        "m",
        ModuleKind::Mount,
        mount,
        vec![marker(Transform::from_pos(Vec3::new(0.5, 0.0, 0.0)))],
        vec![],
        true,
    );
    asm.attach("roof", &m, std::f64::consts::FRAC_PI_2, "m").unwrap();
    let placed = find(&asm, "marker.m").pose;
    assert!(near(placed.pos, Vec3::new(0.0, 1.5, 0.3 - 0.5)), "a point to the right of the mount goes to its front");
    assert!(near(placed.apply_dir(-Vec3::Z), -Vec3::X), "the nose turns to the left, like positive yaw");
}

#[test]
fn a_left_socket_takes_the_mirror_image_of_a_module_that_is_not_symmetric() {
    let right = frame(Vec3::new(1.0, 0.5, 0.2), Vec3::X, -Vec3::Z);
    let left = frame(Vec3::new(-1.0, 0.5, 0.2), -Vec3::X, -Vec3::Z);
    let sockets = vec![
        socket("right", SocketKind::Station, Side::Right, right, 0.5),
        socket("left", SocketKind::Station, Side::Left, left, 0.5),
    ];
    let mount = socket("mount", SocketKind::Station, Side::Centre, frame(Vec3::ZERO, -Vec3::X, -Vec3::Z), 0.4);
    let m = module(
        "lump",
        ModuleKind::Gear,
        mount,
        vec![part("lump", NodeRole::Hull, lump(), Transform::IDENTITY)],
        vec![],
        false,
    );
    assert!(
        m.parts[0].mesh.check_closed().is_ok() && m.parts[0].mesh.signed_volume() > 0.0,
        "the lump is a closed solid"
    );
    let mut asm = Assembly::new(hull(sockets));
    asm.attach("right", &m, 0.0, "r").unwrap();
    asm.attach("left", &m, 0.0, "l").unwrap();
    let (right_lump, left_lump) = (&find(&asm, "lump.r").mesh, &find(&asm, "lump.l").mesh);
    for m in [right_lump, left_lump] {
        assert!(m.check_closed().is_ok() && m.signed_volume() > 0.0, "still a closed, outward-facing solid");
    }
    assert!((right_lump.signed_volume() - left_lump.signed_volume()).abs() < 1e-12);
    for (a, b) in right_lump.v.iter().zip(&left_lump.v) {
        assert!(near(Vec3::new(-a.x, a.y, a.z), *b), "the left lump is the right lump reflected in the plane x = 0");
    }
    let (lo, hi) = right_lump.bounds();
    assert!((lo.x - 1.0).abs() < 1e-9 && (hi.x - 1.4).abs() < 1e-9, "the right lump stands out of the right wall");
}

#[test]
fn a_ring_wider_than_the_socket_is_refused_and_the_assembly_is_left_as_it_was() {
    let mut asm = roof_hull();
    let before = (asm.parts.len(), asm.sockets.len(), asm.log.len());
    let err = asm.attach("roof", &ring_mount(1.0), 0.0, "ring").unwrap_err();
    assert_eq!(err.socket, "roof");
    assert!(err.reason.contains("needs 1.000 m, the socket offers 0.900 m"), "{err}");
    assert_eq!((asm.parts.len(), asm.sockets.len(), asm.log.len()), before);
    assert_eq!(asm.open_sockets(SocketKind::Ring).len(), 1, "the roof is still free");
    assert!(asm.attach("roof", &ring_mount(0.9), 0.0, "ring").is_ok(), "a ring exactly as wide as the socket fits");
}

#[test]
fn an_occupied_socket_an_unknown_one_a_kind_mismatch_and_a_module_without_a_mount_are_refused() {
    let mut asm = roof_hull();
    asm.attach("roof", &ring_mount(0.8), 0.0, "ring").unwrap();
    assert!(asm.attach("roof", &ring_mount(0.8), 0.0, "again").unwrap_err().reason.contains("already taken"));
    assert!(asm.attach("nowhere", &ring_mount(0.8), 0.0, "x").unwrap_err().reason.contains("no such socket"));
    assert!(asm.attach("trunnion.ring", &ring_mount(0.8), 0.0, "x").unwrap_err().reason.contains("mounts on Ring"));
    let mut bare = gun(0.2);
    bare.mount = None;
    assert!(asm.attach("trunnion.ring", &bare, 0.0, "x").unwrap_err().reason.contains("no mount socket"));
    assert!(asm.attach("trunnion.ring", &gun(0.2), 0.0, "").unwrap_err().reason.contains("label is empty"));
    assert_eq!(asm.log.len(), 1, "five refusals changed nothing");
}

#[test]
fn a_name_already_in_the_assembly_is_refused_and_a_new_label_fixes_it() {
    let sockets = vec![
        socket("front", SocketKind::Ring, Side::Centre, frame(Vec3::new(0.0, 1.5, 0.3), Vec3::Y, -Vec3::Z), 0.9),
        socket("rear", SocketKind::Ring, Side::Centre, frame(Vec3::new(0.0, 1.5, -1.0), Vec3::Y, -Vec3::Z), 0.9),
    ];
    let mut asm = Assembly::new(hull(sockets));
    asm.attach("front", &ring_mount(0.8), 0.0, "ring").unwrap();
    let before = (asm.parts.len(), asm.sockets.len());
    let err = asm.attach("rear", &ring_mount(0.8), 0.0, "ring").unwrap_err();
    assert!(err.reason.contains("a part named collar.ring exists already"), "{err}");
    assert_eq!((asm.parts.len(), asm.sockets.len(), asm.log.len()), (before.0, before.1, 1));
    asm.attach("rear", &ring_mount(0.8), 0.0, "aft_ring").unwrap();
    assert!(asm.socket("trunnion.aft_ring").is_some() && asm.socket("trunnion.ring").is_some());
}

#[test]
fn parts_without_a_station_of_their_own_take_the_one_of_the_socket() {
    let mut s =
        socket("axle", SocketKind::Station, Side::Right, frame(Vec3::new(1.0, 0.5, 0.0), Vec3::X, -Vec3::Z), 0.5);
    s.station = Some(2);
    let mount = socket("mount", SocketKind::Station, Side::Centre, frame(Vec3::ZERO, -Vec3::X, -Vec3::Z), 0.4);
    let mut own = part("own", NodeRole::Wheel, lump(), Transform::IDENTITY);
    own.station = Some(7);
    let m = module(
        "wheel",
        ModuleKind::Gear,
        mount,
        vec![part("tyre", NodeRole::Wheel, lump(), Transform::IDENTITY), own],
        vec![],
        false,
    );
    let mut asm = Assembly::new(hull(vec![s]));
    asm.attach("axle", &m, 0.0, "2.r").unwrap();
    assert_eq!(find(&asm, "tyre.2.r").station, Some(2));
    assert_eq!(find(&asm, "own.2.r").station, Some(7), "a part that has an index keeps it");
    assert_eq!(find(&asm, "body").station, None);
}

#[test]
fn a_module_mirrored_twice_is_the_module_and_mirroring_swaps_left_and_right() {
    let mut m = ring_mount(0.8);
    m.sockets[0].pose = frame(Vec3::new(0.3, 0.5, -0.2), -Vec3::Z, Vec3::Y);
    m.sockets[0].side = Side::Left;
    m.parts.push(part("lump", NodeRole::Hull, lump(), Transform::from_pos(Vec3::new(0.2, 0.0, 0.1))));
    let once = m.mirrored_x();
    assert_eq!(once.sockets[0].side, Side::Right);
    assert!(near(once.sockets[0].pose.pos, Vec3::new(-0.3, 0.5, -0.2)));
    let twice = once.mirrored_x();
    assert_eq!(twice.sockets[0].side, Side::Left);
    for (a, b) in m.parts.iter().zip(&twice.parts) {
        assert_eq!(a.mesh.t, b.mesh.t);
        let bits =
            |p: &Part| p.mesh.v.iter().map(|v| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]).collect::<Vec<_>>();
        assert_eq!(bits(a), bits(b));
        assert!(
            near(a.pose.pos, b.pose.pos)
                && near(a.pose.apply_dir(Vec3::X), b.pose.apply_dir(Vec3::X))
                && near(a.pose.apply_dir(Vec3::Y), b.pose.apply_dir(Vec3::Y))
        );
    }
    assert!(
        near(m.sockets[0].pose.pos, twice.sockets[0].pose.pos)
            && near(m.sockets[0].pose.apply_dir(Vec3::Y), twice.sockets[0].pose.apply_dir(Vec3::Y))
    );
}

#[test]
fn the_same_assembly_built_twice_has_bit_identical_geometry() {
    let build = || {
        let mut asm = roof_hull();
        asm.attach("roof", &ring_mount(0.8), 0.4, "ring").unwrap();
        asm.attach("trunnion.ring", &gun(0.25), 0.0, "mg").unwrap();
        let mut bits = Vec::new();
        for p in &asm.parts {
            let m = p.in_hull_frame();
            bits.extend(m.v.iter().flat_map(|v| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]));
            bits.extend(m.t.iter().flatten().map(|&i| u64::from(i)));
        }
        bits
    };
    assert_eq!(build(), build());
}
