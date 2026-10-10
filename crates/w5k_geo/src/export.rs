//! From parts to a `RenderRig` (stand-in node layout of `testing::box_truck()`: hull; per station travel > steer (steered axles) > wheel) and
//! a binary glTF, so VIEWER, LOOK and Blender can load the vehicle before FORGE assembles real rigs. f64 inside, f32 once, here.

use crate::flags::{bake, FlagParams, Flags};
use crate::mesh::Mesh;
use crate::part::{Part, Side};
use std::collections::BTreeSet;
use w5k_contract::render::{
    JointAxisKind, JointBinding, MaterialSlot, MeshPart, NodeRole, RenderNode, RenderRig, SlotKind,
};
use w5k_math::{scalar, Quat, Transform, Vec3};

const PALETTE: &str = include_str!("../shapes/preview_palette.ron");
const SLOTS: [(SlotKind, &str); 7] = [
    (SlotKind::Paint, "paint"),
    (SlotKind::Metal, "metal"),
    (SlotKind::Rubber, "rubber"),
    (SlotKind::Glass, "glass"),
    (SlotKind::Canvas, "canvas"),
    (SlotKind::Optics, "optics"),
    (SlotKind::Track, "track"),
];

/// Preview colour of a slot kind (`shapes/preview_palette.ron`).
pub fn slot_colour(k: SlotKind) -> [f64; 3] {
    let table: Vec<(SlotKind, (f64, f64, f64))> = ron::from_str(PALETTE).expect("preview_palette.ron parses");
    table.iter().find(|e| e.0 == k).map_or([0.5, 0.5, 0.5], |e| [e.1 .0, e.1 .1, e.1 .2])
    // const-ok: mid grey for an unlisted kind
}

/// Normals smoothed across faces within `smooth_rad` of each other and split beyond it: returns (positions, normals, source vertex, indices).
fn split_normals(m: &Mesh, smooth_rad: f64) -> (Vec<Vec3>, Vec<Vec3>, Vec<u32>, Vec<u32>) {
    let mut incident: Vec<Vec<usize>> = vec![Vec::new(); m.v.len()];
    for (k, t) in m.t.iter().enumerate() {
        for &i in t {
            incident[i as usize].push(k);
        }
    }
    let face: Vec<(Vec3, f64)> = (0..m.t.len())
        .map(|k| {
            let [a, b, c] = m.tri(k);
            let n = (b - a).cross(c - a);
            (n.normalized_or_zero(), n.length())
        })
        .collect();
    let cos_limit = scalar::cos(smooth_rad);
    let mut key: std::collections::BTreeMap<(u32, [i64; 3]), u32> = std::collections::BTreeMap::new();
    let (mut pos, mut nrm, mut src, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (k, t) in m.t.iter().enumerate() {
        for &i in t {
            let sum = incident[i as usize]
                .iter()
                .filter(|&&j| face[j].0.dot(face[k].0) >= cos_limit)
                .fold(Vec3::ZERO, |a, &j| a + face[j].0 * face[j].1);
            let n = sum.normalized_or_zero();
            let q = [(n.x * 1e4).round() as i64, (n.y * 1e4).round() as i64, (n.z * 1e4).round() as i64]; // const-ok: normals quantised to 1e-4 to merge equal ones
            idx.push(*key.entry((i, q)).or_insert_with(|| {
                pos.push(m.v[i as usize]);
                nrm.push(n);
                src.push(i);
                pos.len() as u32 - 1
            }));
        }
    }
    (pos, nrm, src, idx)
}

/// The first axle is `f`, the last `r`, the ones between `m` (`m1`, `m2` when there are several): `fl`, `fr`, `rl`, `rr` on a 4x4.
fn axle_name(axle: usize, axles: usize) -> String {
    match axle {
        0 => "f".into(),
        a if a + 1 == axles => "r".into(),
        _ if axles == 3 => "m".into(), // const-ok: one axle between the first and the last
        a => format!("m{a}"),
    }
}

/// The articulation chain of a mount, in the order each joint hangs from the one before: node name, role, joint kind and axis (the stand-in
/// layout of `testing::box_tank()`: turret yaw about +Y, gun pitch about +X, recoil along +Z).
const CHAIN: [(NodeRole, &str, JointAxisKind, Vec3); 3] = [
    (NodeRole::Turret, "turret_yaw", JointAxisKind::Revolute, Vec3::Y),
    (NodeRole::GunPitch, "gun_pitch", JointAxisKind::Revolute, Vec3::X),
    (NodeRole::Recoil, "gun_recoil", JointAxisKind::Prismatic, Vec3::Z),
];

/// A point in the frame of a node whose frame in the hull is `node` (a rotation-free node subtracts its position exactly).
fn to_local(node: &Transform, p: Vec3) -> Vec3 {
    if node.rot == Quat::IDENTITY {
        p - node.pos
    } else {
        node.rot.inverse_rotate(p - node.pos)
    }
}

/// Build the rig for a wheeled vehicle of any number of axles, with any number of weapon mounts. Nodes: the hull; per station travel > steer
/// (stations with a steering knuckle) > wheel; per mount turret yaw > gun pitch > recoil, hung from the node that carries the socket the
/// module sits on. Joint coordinates in the layout of the contract: spin, then steer, then travel, then the articulation chains. Flags are
/// baked here, on the assembled parts.
pub fn render_rig(id: &str, parts: &[Part], p: &FlagParams) -> RenderRig {
    render_rig_with(id, parts, p, &Overrides::default())
}

/// What a physics rig knows and the parts do not: the axis each station's suspension travel moves along (`bump_dir`; vertical when absent)
/// and how many joint coordinates its frames carry (an unarmed skin of an armed rig leaves the articulation coordinates unbound).
#[derive(Clone, Debug, Default)]
pub struct Overrides {
    pub travel_axes: Vec<Vec3>,
    pub joint_count: Option<usize>,
}

/// The wheel roles of a tracked vehicle: they make stations the way `Wheel` does, with one global index per station (`Part::station`, in the
/// physics rig's order) instead of an axle index shared by the two sides.
const TRACKED: [NodeRole; 4] = [NodeRole::RoadWheel, NodeRole::Sprocket, NodeRole::Idler, NodeRole::ReturnRoller];

fn is_wheel(role: NodeRole) -> bool {
    role == NodeRole::Wheel || TRACKED.contains(&role)
}

/// The node name of a tracked station: `<l|r>_idler`, `<l|r>_spr`, `<l|r>_r<k>` (road wheel k from the front) and `<l|r>_rr<k>` (return roller).
fn tracked_name(parts: &[Part], station: u8, right: bool) -> String {
    let wheel = |q: &&Part| is_wheel(q.role) && q.side == if right { Side::Right } else { Side::Left };
    let Some(me) = parts.iter().filter(wheel).find(|q| q.station == Some(station)) else { return String::new() };
    let mut mates: Vec<(f64, u8)> = parts
        .iter()
        .filter(wheel)
        .filter(|q| q.role == me.role)
        .filter_map(|q| q.station.map(|s| (q.pose.pos.z, s)))
        .collect();
    mates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    mates.dedup();
    let k = mates.iter().position(|m| m.1 == station).unwrap_or(0) + 1;
    let tag = match me.role {
        NodeRole::Idler => "idler".to_string(),
        NodeRole::Sprocket => "spr".to_string(),
        NodeRole::ReturnRoller => format!("rr{k}"),
        _ => format!("r{k}"),
    };
    format!("{}_{tag}", if right { "r" } else { "l" })
}

/// `render_rig` with what the physics rig says about the stations (`Overrides`).
pub fn render_rig_with(id: &str, parts: &[Part], p: &FlagParams, over: &Overrides) -> RenderRig {
    let hulls: Vec<Mesh> = parts.iter().map(Part::in_hull_frame).collect();
    let baked = bake(&hulls.iter().collect::<Vec<_>>(), p);
    // stations: (axle, right?) sorted axle first, left before right: fl, fr, rl, rr
    let mut stations: Vec<(u8, bool)> = parts
        .iter()
        .filter(|q| is_wheel(q.role))
        .filter_map(|q| q.station.map(|a| (a, q.side == Side::Right)))
        .collect();
    stations.sort_unstable();
    stations.dedup();
    let index_of =
        |part: &Part| stations.iter().position(|&(a, r)| Some(a) == part.station && r == (part.side == Side::Right));
    let steered: Vec<usize> = parts
        .iter()
        .filter(|q| q.role == NodeRole::SteerKnuckle)
        .filter_map(index_of)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let (n, axles) = (stations.len(), stations.iter().map(|s| usize::from(s.0)).max().map_or(0, |a| a + 1));
    let hub =
        |i: usize| parts.iter().find(|q| is_wheel(q.role) && index_of(q) == Some(i)).map_or(Vec3::ZERO, |q| q.pose.pos);
    let mut rig = RenderRig {
        id: id.into(),
        nodes: vec![RenderNode {
            name: "hull".into(),
            parent: None,
            role: NodeRole::Hull,
            rest: Transform::IDENTITY,
            joint: None,
        }],
        meshes: vec![],
        material_slots: SLOTS.iter().map(|&(kind, name)| MaterialSlot { name: name.into(), kind }).collect(),
        joint_count: 0,
        track_runs: Vec::new(),
    };
    let joint = |kind, axis, index| Some(JointBinding { kind, axis, index });
    let (mut wheel_node, mut steer_node) = (vec![0usize; n], vec![None; n]);
    for (i, &(axle, right)) in stations.iter().enumerate() {
        let role =
            parts.iter().find(|q| is_wheel(q.role) && index_of(q) == Some(i)).map_or(NodeRole::Wheel, |q| q.role);
        let name = if role == NodeRole::Wheel {
            format!("{}{}", axle_name(usize::from(axle), axles), if right { "r" } else { "l" })
        } else {
            tracked_name(parts, axle, right)
        };
        let travel = rig.nodes.len();
        rig.nodes.push(RenderNode {
            name: format!("{name}.travel"),
            parent: Some(0),
            role: NodeRole::SuspensionArm,
            rest: Transform::from_pos(hub(i)),
            joint: joint(
                JointAxisKind::Prismatic,
                over.travel_axes.get(i).copied().unwrap_or(Vec3::Y),
                n + steered.len() + i,
            ),
        });
        let mut parent = travel;
        if let Some(k) = steered.iter().position(|&s| s == i) {
            steer_node[i] = Some(rig.nodes.len());
            rig.nodes.push(RenderNode {
                name: format!("{name}.steer"),
                parent: Some(travel),
                role: NodeRole::SteerKnuckle,
                rest: Transform::IDENTITY,
                joint: joint(JointAxisKind::Revolute, Vec3::Y, n + k),
            });
            parent = rig.nodes.len() - 1;
        }
        wheel_node[i] = rig.nodes.len();
        rig.nodes.push(RenderNode {
            name: format!("{name}.wheel"),
            parent: Some(parent),
            role,
            rest: Transform::IDENTITY,
            joint: joint(JointAxisKind::Revolute, -Vec3::X, i),
        });
    }
    // the belt of each side: one jointless node on the hull at the track centre line, as in `testing::box_tank()`
    let mut belt_node = std::collections::BTreeMap::new();
    for q in parts.iter().filter(|q| q.role == NodeRole::Track) {
        let name = format!("track_{}", if q.side == Side::Right { "r" } else { "l" });
        belt_node.insert(q.name.clone(), rig.nodes.len());
        rig.nodes.push(RenderNode { name, parent: Some(0), role: NodeRole::Track, rest: q.pose, joint: None });
    }
    // articulation chains: one node per (placement, role), the placement's own chain first, each hung from the node that carries its socket
    let mut chain_nodes: Vec<(String, NodeRole, usize, Transform)> = Vec::new();
    let mut labels: Vec<&str> = Vec::new();
    for q in parts {
        let Some(pl) = &q.placement else { continue };
        if CHAIN.iter().any(|c| c.0 == q.role) && !labels.contains(&pl.label.as_str()) {
            labels.push(&pl.label);
        }
    }
    let mut next_joint = 2 * n + steered.len();
    for label in labels {
        for (k, &(role, base, kind, axis)) in CHAIN.iter().enumerate() {
            let Some(q) =
                parts.iter().find(|q| q.role == role && q.placement.as_ref().is_some_and(|pl| pl.label == label))
            else {
                continue;
            };
            let carrier = q.placement.as_ref().and_then(|pl| pl.carrier.as_ref());
            let own = chain_nodes.iter().rev().find(|c| c.0 == label && CHAIN[..k].iter().any(|e| e.0 == c.1));
            let host = own.or_else(|| carrier.and_then(|(l, r)| chain_nodes.iter().find(|c| &c.0 == l && c.1 == *r)));
            let (parent, parent_pose) = host.map_or((0, Transform::IDENTITY), |c| (c.2, c.3));
            let same_role = chain_nodes.iter().filter(|c| c.1 == role).count();
            let name = if same_role == 0 { base.to_string() } else { format!("{base}.{same_role}") };
            chain_nodes.push((label.to_string(), role, rig.nodes.len(), q.pose));
            rig.nodes.push(RenderNode {
                name,
                parent: Some(parent),
                role,
                rest: parent_pose.inverse().compose(&q.pose),
                joint: joint(kind, axis, next_joint),
            });
            next_joint += 1;
        }
    }
    rig.joint_count = over.joint_count.map_or(next_joint, |c| c.max(next_joint));
    let smooth = p.smooth_angle_deg * std::f64::consts::PI / 180.0; // const-ok: degrees to radians at the authoring edge
    for (part, (mesh, flags)) in parts.iter().zip(&baked) {
        let chain =
            part.placement.as_ref().and_then(|pl| chain_nodes.iter().find(|c| c.0 == pl.label && c.1 == part.role));
        let (node, frame) = match (part.role, chain) {
            (role, _) if is_wheel(role) => (index_of(part).map_or(0, |i| wheel_node[i]), part.pose),
            (NodeRole::Track, _) => (belt_node.get(&part.name).copied().unwrap_or(0), part.pose),
            (NodeRole::SteerKnuckle, _) => (index_of(part).and_then(|i| steer_node[i]).unwrap_or(0), part.pose),
            (_, Some(c)) => (c.2, c.3),
            _ => (0, Transform::IDENTITY),
        };
        rig.meshes.push(mesh_part(part, node, mesh, flags, &frame, smooth));
    }
    rig
}

fn mesh_part(part: &Part, node: usize, mesh: &Mesh, flags: &Flags, frame: &Transform, smooth_rad: f64) -> MeshPart {
    let (pos, nrm, src, indices) = split_normals(mesh, smooth_rad);
    let f3 = |v: Vec3| [v.x as f32, v.y as f32, v.z as f32];
    let dir = |n: Vec3| if frame.rot == Quat::IDENTITY { n } else { frame.rot.inverse_rotate(n) };
    MeshPart {
        name: part.name.clone(),
        node,
        material_slot: SLOTS.iter().position(|s| s.0 == part.slot).unwrap_or(0),
        positions: pos.iter().map(|&p| f3(to_local(frame, p))).collect(),
        normals: nrm.iter().map(|&n| f3(dir(n))).collect(),
        edge: src.iter().map(|&i| flags.edge[i as usize] as f32).collect(),
        cavity: src.iter().map(|&i| flags.cavity[i as usize] as f32).collect(),
        indices,
    }
}

fn pad4(b: &mut Vec<u8>, with: u8) {
    while !b.len().is_multiple_of(4) {
        b.push(with);
    }
}

/// A binary glTF 2.0 of the rest pose: the node tree, one mesh per part with `POSITION`, `NORMAL`, `COLOR_0` = (edge, cavity, 0), and a
/// material per slot (preview colours). Joints are not animated.
pub fn glb(rig: &RenderRig) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let (mut views, mut accessors, mut meshes, mut nodes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut push_view = |bin: &mut Vec<u8>, bytes: &[u8], target: u32| {
        pad4(bin, 0);
        views.push(format!(
            "{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":{target}}}",
            bin.len(),
            bytes.len()
        ));
        bin.extend_from_slice(bytes);
        views.len() - 1
    };
    for (k, m) in rig.meshes.iter().enumerate() {
        let f32s = |v: &[[f32; 3]]| v.iter().flatten().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        let colour: Vec<[f32; 3]> = (0..m.positions.len())
            .map(|i| [m.edge.get(i).copied().unwrap_or(0.0), m.cavity.get(i).copied().unwrap_or(0.0), 0.0])
            .collect();
        let (lo, hi) = m.positions.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(l, h), p| {
            (std::array::from_fn(|i| l[i].min(p[i])), std::array::from_fn(|i| h[i].max(p[i])))
        });
        let n = m.positions.len();
        let (vp, vn, vc) = (
            push_view(&mut bin, &f32s(&m.positions), 34962),
            push_view(&mut bin, &f32s(&m.normals), 34962),
            push_view(&mut bin, &f32s(&colour), 34962),
        ); // const-ok: glTF ARRAY_BUFFER
        let vi = push_view(&mut bin, &m.indices.iter().flat_map(|i| i.to_le_bytes()).collect::<Vec<u8>>(), 34963); // const-ok: glTF ELEMENT_ARRAY_BUFFER
        let a0 = accessors.len();
        accessors.push(format!("{{\"bufferView\":{vp},\"componentType\":5126,\"count\":{n},\"type\":\"VEC3\",\"min\":[{},{},{}],\"max\":[{},{},{}]}}", lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]));
        accessors.push(format!("{{\"bufferView\":{vn},\"componentType\":5126,\"count\":{n},\"type\":\"VEC3\"}}"));
        accessors.push(format!("{{\"bufferView\":{vc},\"componentType\":5126,\"count\":{n},\"type\":\"VEC3\"}}"));
        accessors.push(format!(
            "{{\"bufferView\":{vi},\"componentType\":5125,\"count\":{},\"type\":\"SCALAR\"}}",
            m.indices.len()
        ));
        meshes.push(format!(
            "{{\"name\":\"{}\",\"primitives\":[{{\"attributes\":{{\"POSITION\":{a0},\"NORMAL\":{},\"COLOR_0\":{}}},\"indices\":{},\"material\":{}}}]}}",
            m.name, a0 + 1, a0 + 2, a0 + 3, m.material_slot
        ));
        let _ = k;
    }
    // node i of the rig is glTF node i; each mesh gets a child node under its rig node, after the rig nodes
    for (i, n) in rig.nodes.iter().enumerate() {
        let kids: Vec<String> = rig
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.parent == Some(i))
            .map(|(j, _)| j.to_string())
            .chain(
                rig.meshes
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.node == i)
                    .map(|(k, _)| (rig.nodes.len() + k).to_string()),
            )
            .collect();
        let (p, q) = (n.rest.pos, n.rest.rot);
        nodes.push(format!(
            "{{\"name\":\"{}\",\"translation\":[{},{},{}],\"rotation\":[{},{},{},{}],\"children\":[{}]}}",
            n.name,
            p.x,
            p.y,
            p.z,
            q.x,
            q.y,
            q.z,
            q.w,
            kids.join(",")
        ));
    }
    for (k, m) in rig.meshes.iter().enumerate() {
        nodes.push(format!("{{\"name\":\"{}\",\"mesh\":{k}}}", m.name));
    }
    let materials: Vec<String> = rig
        .material_slots
        .iter()
        .map(|s| {
            let c = slot_colour(s.kind);
            format!("{{\"name\":\"{}\",\"pbrMetallicRoughness\":{{\"baseColorFactor\":[{},{},{},1],\"metallicFactor\":0,\"roughnessFactor\":0.8}}}}", s.name, c[0], c[1], c[2]) // const-ok: preview roughness
        })
        .collect();
    let mut json = format!(
        "{{\"asset\":{{\"version\":\"2.0\",\"generator\":\"w5k geometry\"}},\"scene\":0,\"scenes\":[{{\"nodes\":[0]}}],\"nodes\":[{}],\"meshes\":[{}],\"materials\":[{}],\"accessors\":[{}],\"bufferViews\":[{}],\"buffers\":[{{\"byteLength\":{}}}]}}",
        nodes.join(","), meshes.join(","), materials.join(","), accessors.join(","), views.join(","), bin.len()
    )
    .into_bytes();
    pad4(&mut json, b' ');
    pad4(&mut bin, 0);
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}
