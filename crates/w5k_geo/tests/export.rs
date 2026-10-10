//! The exported RenderRig and GLB: validity, flags in range, budget, node frames, any number of axles, the turret > gun > recoil chains in
//! the contract's joint layout, and forward kinematics at rest reproducing the assembled geometry.

use w5k_contract::render::{JointAxisKind, NodeRole, RenderNode, RenderRig};
use w5k_geo::export::{glb, render_rig};
use w5k_geo::flags::FlagParams;
use w5k_geo::mesh::Mesh;
use w5k_geo::module::{frame, Assembly, Module, ModuleKind, Socket, SocketKind};
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::{Part, Side};
use w5k_geo::truck::{utility_4x4, utility_assembly, utility_truck, UtilityDims};
use w5k_geo::weapon::{gun_module, GunDims};
use w5k_math::{Quat, Transform, Vec3};

fn rig() -> w5k_contract::render::RenderRig {
    render_rig("utility_4x4", &utility_4x4(&UtilityDims::placeholder(), 1), &FlagParams::default_params())
}

#[test]
fn exported_rig_validates_carries_flags_in_range_and_stays_inside_the_triangle_budget() {
    let r = rig();
    r.validate().expect("RenderRig::validate");
    let tris = r.triangle_count();
    println!("exported triangles: {tris}");
    let budget = w5k_geo::budget::wheeled_triangles();
    assert!(tris < budget, "{tris} triangles, budget {budget} for a wheeled vehicle");
    for m in &r.meshes {
        assert!(m.edge.len() == m.positions.len() && m.cavity.len() == m.positions.len(), "{}", m.name);
        assert!(m.edge.iter().chain(&m.cavity).all(|v| v.is_finite() && (0.0..=1.0).contains(v)), "{}", m.name);
        assert!(
            m.normals.iter().all(|n| (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] - 1.0).abs() < 1e-3),
            "{} has a non-unit normal",
            m.name
        );
    }
    assert!(
        r.meshes.iter().any(|m| m.edge.iter().any(|&e| e > 0.4))
            && r.meshes.iter().any(|m| m.cavity.iter().any(|&c| c > 0.1))
    );
}

#[test]
fn wheel_meshes_are_centred_on_their_hub_in_the_wheel_node_frame() {
    let r = rig();
    let mut wheels = 0;
    for m in r.meshes.iter().filter(|m| m.name.starts_with("tyre")) {
        assert_eq!(r.nodes[m.node].role, NodeRole::Wheel);
        let (lo, hi) = m.positions.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(l, h), p| {
            (std::array::from_fn(|i| l[i].min(p[i])), std::array::from_fn(|i| h[i].max(p[i])))
        });
        assert!((lo[1] + hi[1]).abs() < 2e-3 && (lo[2] + hi[2]).abs() < 2e-3, "{} is off its hub", m.name);
        wheels += 1;
    }
    assert_eq!(wheels, 4);
    for m in r.meshes.iter().filter(|m| m.name.starts_with("knuckle")) {
        assert_eq!(r.nodes[m.node].role, NodeRole::SteerKnuckle);
    }
}

#[test]
fn glb_chunks_are_consistent() {
    let b = glb(&rig());
    assert_eq!(&b[0..4], b"glTF");
    assert_eq!(u32::from_le_bytes(b[4..8].try_into().unwrap()), 2);
    assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize, b.len());
    let json_len = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
    assert_eq!(&b[16..20], b"JSON");
    let json = std::str::from_utf8(&b[20..20 + json_len]).unwrap();
    assert!(json.trim_end().starts_with('{') && json.trim_end().ends_with('}'));
    assert_eq!(json.matches('{').count(), json.matches('}').count());
    assert_eq!(json.matches('[').count(), json.matches(']').count());
    assert!(json.contains("COLOR_0") && json.contains("\"fl.wheel\""));
    let bin_at = 20 + json_len;
    assert_eq!(&b[bin_at + 4..bin_at + 8], b"BIN\0");
    assert_eq!(bin_at + 8 + u32::from_le_bytes(b[bin_at..bin_at + 4].try_into().unwrap()) as usize, b.len());
}

// ---- any number of axles, and the articulation chains

fn node<'a>(r: &'a RenderRig, name: &str) -> (usize, &'a RenderNode) {
    r.nodes.iter().enumerate().find(|(_, n)| n.name == name).unwrap_or_else(|| panic!("no node {name}"))
}

/// The 4x4 with the ring mount on its roof and the autocannon (or the machine gun) on the trunnion, as parts.
fn armed(gun: &str) -> Vec<Part> {
    let d = UtilityDims::placeholder();
    let z = d.wheelbase_m / 2.0;
    let mut asm = utility_assembly(&d, &[-z, z], &[true, false], 0);
    asm.attach("roof", &ring_mount(&RingMountDims::standard(), 0), 0.0, "ring").unwrap();
    let cradle = asm.socket("trunnion.ring").unwrap().hint("cradle_w_m").unwrap();
    asm.attach("trunnion.ring", &gun_module(&GunDims::preset(gun).unwrap(), cradle, 0), 0.0, "gun").unwrap();
    asm.parts
}

/// Forward kinematics as the contract defines it: revolute about the joint axis, prismatic along it, each node in its parent's frame.
fn world(r: &RenderRig, i: usize, q: &[f64]) -> Transform {
    let n = &r.nodes[i];
    let local = match n.joint {
        None => n.rest,
        Some(b) if b.kind == JointAxisKind::Revolute => {
            n.rest.compose(&Transform::new(Vec3::ZERO, Quat::from_axis_angle(b.axis, q[b.index])))
        }
        Some(b) => n.rest.compose(&Transform::from_pos(b.axis * q[b.index])),
    };
    n.parent.map_or(local, |p| world(r, p, q).compose(&local))
}

#[test]
fn a_three_axle_truck_exports_six_wheel_chains_in_the_contract_joint_layout() {
    let d = UtilityDims::placeholder();
    let parts = utility_truck(&d, &[-d.wheelbase_m / 2.0, 0.45, d.wheelbase_m / 2.0], &[true, false, false], 0);
    let r = render_rig("six_by_six", &parts, &FlagParams::default_params());
    r.validate().expect("RenderRig::validate");
    let (stations, steered) = (6, 2);
    assert_eq!(r.joint_count, 2 * stations + steered, "spin, steer and travel only: no mount");
    for (i, name) in ["fl", "fr", "ml", "mr", "rl", "rr"].iter().enumerate() {
        let (_, wheel) = node(&r, &format!("{name}.wheel"));
        assert_eq!((wheel.role, wheel.joint.unwrap().index), (NodeRole::Wheel, i), "spin joint of station {i}");
        let (_, travel) = node(&r, &format!("{name}.travel"));
        assert_eq!(travel.joint.unwrap().index, stations + steered + i, "travel joint of station {i}");
        let has_steer = r.nodes.iter().any(|n| n.name == format!("{name}.steer"));
        assert_eq!(has_steer, i < 2, "only the front axle steers");
    }
    assert_eq!(node(&r, "fl.steer").1.joint.unwrap().index, stations);
    assert_eq!(node(&r, "fr.steer").1.joint.unwrap().index, stations + 1);
    let mut used: Vec<usize> = r.nodes.iter().filter_map(|n| n.joint.map(|j| j.index)).collect();
    used.sort_unstable();
    assert_eq!(used, (0..r.joint_count).collect::<Vec<_>>(), "every joint coordinate has exactly one node");
}

#[test]
fn an_armed_truck_exports_turret_gun_and_recoil_nodes_chained_after_the_wheel_joints() {
    let parts = armed("autocannon_25");
    let r = render_rig("armed_utility", &parts, &FlagParams::default_params());
    r.validate().expect("RenderRig::validate");
    assert_eq!(r.joint_count, 13, "4 spins, 2 steers, 4 travels, then turret yaw, gun pitch, recoil");
    let ((t, turret), (g, gun), (c, recoil)) = (node(&r, "turret_yaw"), node(&r, "gun_pitch"), node(&r, "gun_recoil"));
    assert_eq!((turret.parent, gun.parent, recoil.parent), (Some(0), Some(t), Some(g)));
    assert_eq!((turret.role, gun.role, recoil.role), (NodeRole::Turret, NodeRole::GunPitch, NodeRole::Recoil));
    let bind = |n: &RenderNode| (n.joint.unwrap().kind, n.joint.unwrap().axis, n.joint.unwrap().index);
    assert_eq!(bind(turret), (JointAxisKind::Revolute, Vec3::Y, 10));
    assert_eq!(bind(gun), (JointAxisKind::Revolute, Vec3::X, 11));
    assert_eq!(bind(recoil), (JointAxisKind::Prismatic, Vec3::Z, 12));
    let d = RingMountDims::standard();
    assert!(
        (gun.rest.pos - Vec3::new(0.0, d.cradle_height_m, d.cradle_z_m)).length() < 1e-9,
        "the pivot, in the turntable's frame"
    );
    assert!(recoil.rest.pos.length() < 1e-12, "recoil hangs at the pivot");
    let on = |name: &str| r.meshes.iter().find(|m| m.name == name).unwrap_or_else(|| panic!("no mesh {name}")).node;
    assert_eq!((on("collar.ring"), on("turntable.ring"), on("upright.r.ring"), on("shield.l.ring")), (0, t, t, t));
    assert_eq!((on("receiver.gun"), on("pins.gun"), on("ammo0.r.gun"), on("sight.gun")), (g, g, g, g));
    assert_eq!((on("barrel.gun"), on("jacket.gun"), on("muzzle.gun")), (c, c, c));
}

#[test]
fn forward_kinematics_of_the_exported_rig_at_rest_reproduces_the_assembled_geometry() {
    let parts = armed("machine_gun_12_7");
    let r = render_rig("armed_utility", &parts, &FlagParams::default_params());
    let q = vec![0.0; r.joint_count];
    for part in &parts {
        let m = r.meshes.iter().find(|m| m.name == part.name).unwrap();
        let w = world(&r, m.node, &q);
        let exported: Vec<Vec3> = m
            .positions
            .iter()
            .map(|p| w.apply_point(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))))
            .collect();
        let truth: Mesh = part.in_hull_frame();
        let (blo, bhi) = truth.bounds();
        let (elo, ehi) =
            exported.iter().fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(l, h), &p| (l.min(p), h.max(p)));
        assert!(
            (blo - elo).length() < 1e-4 && (bhi - ehi).length() < 1e-4,
            "{} is not where the assembly put it",
            part.name
        );
    }
}

#[test]
fn positive_yaw_turns_the_barrel_left_positive_pitch_raises_it_and_recoil_slides_it_back() {
    let r = render_rig("armed_utility", &armed("autocannon_25"), &FlagParams::default_params());
    let muzzle = r.meshes.iter().find(|m| m.name == "muzzle.gun").unwrap();
    let tip = muzzle.positions.iter().min_by(|a, b| a[2].total_cmp(&b[2])).unwrap();
    let tip = Vec3::new(f64::from(tip[0]), f64::from(tip[1]), f64::from(tip[2]));
    let at = |yaw: f64, pitch: f64, slide: f64| {
        let mut q = vec![0.0; r.joint_count];
        (q[10], q[11], q[12]) = (yaw, pitch, slide);
        world(&r, muzzle.node, &q)
    };
    let rest = at(0.0, 0.0, 0.0);
    let (p0, bore0) = (rest.apply_point(tip), rest.apply_dir(-Vec3::Z));
    assert!((bore0 + Vec3::Z).length() < 1e-9, "the bore points forward (-Z) at rest");
    let yawed = at(0.3, 0.0, 0.0);
    assert!(yawed.apply_dir(-Vec3::Z).x < -0.29, "positive yaw turns the nose to the left");
    let pitched = at(0.0, 0.2, 0.0);
    assert!(
        pitched.apply_point(tip).y > p0.y + 0.2 && pitched.apply_dir(-Vec3::Z).y > 0.19,
        "positive pitch raises the muzzle"
    );
    let recoiled = at(0.0, 0.0, 0.1);
    assert!(
        (recoiled.apply_point(tip) - p0 - Vec3::new(0.0, 0.0, 0.1)).length() < 1e-9,
        "positive recoil slides the barrel back, towards +Z"
    );
}

#[test]
fn two_ring_mounts_export_two_independent_chains_each_gun_on_its_own_turntable() {
    let socket = |name: &str, z: f64| Socket {
        name: name.into(),
        kind: SocketKind::Ring,
        side: Side::Centre,
        pose: frame(Vec3::new(0.0, 1.0, z), Vec3::Y, -Vec3::Z),
        size_m: 0.9,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    let deck = Mesh::extrude_fan(&[[-1.0, -2.5], [-1.0, 2.5], [1.0, 2.5], [1.0, -2.5]], 0, 1.0);
    let body = Part {
        name: "deck".into(),
        role: NodeRole::Hull,
        station: None,
        side: Side::Centre,
        slot: w5k_contract::render::SlotKind::Paint,
        fitting: false,
        mesh: deck,
        pose: Transform::IDENTITY,
        placement: None,
    };
    let hull = Module {
        name: "deck".into(),
        kind: ModuleKind::Hull,
        parts: vec![body],
        sockets: vec![socket("front", -1.5), socket("rear", 1.5)],
        mount: None,
        symmetric: true,
    };
    let mut asm = Assembly::new(hull);
    for (seat, ring, gun) in [("front", "ring_f", "mg_f"), ("rear", "ring_r", "mg_r")] {
        asm.attach(seat, &ring_mount(&RingMountDims::standard(), 0), 0.0, ring).unwrap();
        let cradle = asm.socket(&format!("trunnion.{ring}")).unwrap().hint("cradle_w_m").unwrap();
        let g = gun_module(&GunDims::preset("machine_gun_12_7").unwrap(), cradle, 0);
        asm.attach(&format!("trunnion.{ring}"), &g, 0.0, gun).unwrap();
    }
    let r = render_rig("two_mounts", &asm.parts, &FlagParams::default_params());
    r.validate().expect("RenderRig::validate");
    assert_eq!(r.joint_count, 6, "no wheels: two chains of three joints");
    let names: Vec<&str> = r.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["hull", "turret_yaw", "gun_pitch", "gun_recoil", "turret_yaw.1", "gun_pitch.1", "gun_recoil.1"]);
    let parents: Vec<Option<usize>> = r.nodes.iter().map(|n| n.parent).collect();
    assert_eq!(
        parents,
        [None, Some(0), Some(1), Some(2), Some(0), Some(4), Some(5)],
        "each gun hangs from its own turntable"
    );
    let on = |name: &str| r.meshes.iter().find(|m| m.name == name).unwrap().node;
    assert_eq!(
        (on("turntable.ring_f"), on("receiver.mg_f"), on("turntable.ring_r"), on("receiver.mg_r")),
        (1, 2, 4, 5)
    );
    let z = |i: usize| world(&r, i, &[0.0; 6]).pos.z;
    assert!((z(1) + 1.5).abs() < 1e-9 && (z(4) - 1.5).abs() < 1e-9, "the two turntables are where their sockets are");
}
