//! From parts to a `RenderRig` (stand-in node layout of `testing::box_truck()`: hull; per station travel > steer (steered axles) > wheel) and
//! a binary glTF, so VIEWER, LOOK and Blender can load the vehicle before FORGE assembles real rigs. f64 inside, f32 once, here.

use crate::flags::{bake, FlagParams, Flags};
use crate::mesh::Mesh;
use crate::part::{Part, Side};
use w5k_contract::render::{
    JointAxisKind, JointBinding, MaterialSlot, MeshPart, NodeRole, RenderNode, RenderRig, SlotKind,
};
use w5k_math::{scalar, Transform, Vec3};

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

fn node_of(station: u8, side: Side) -> usize {
    // fl, fr, rl, rr in the order of the stand-in rig
    usize::from(station) * 2 + usize::from(side == Side::Right)
}

/// Build the rig for a 2-axle, 4-station vehicle whose front axle steers. Flags are baked here, on the assembled parts.
pub fn render_rig(id: &str, parts: &[Part], p: &FlagParams) -> RenderRig {
    let hulls: Vec<Mesh> = parts.iter().map(Part::in_hull_frame).collect();
    let baked = bake(&hulls.iter().collect::<Vec<_>>(), p);
    let hub = |i: usize| {
        parts
            .iter()
            .find(|q| q.role == NodeRole::Wheel && node_of(q.station.unwrap_or(0), q.side) == i)
            .map_or(Vec3::ZERO, |q| q.pose.pos)
    };
    let names = ["fl", "fr", "rl", "rr"];
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
        joint_count: 10, // const-ok: 4 spins, 2 steers, 4 travels, as the stand-in rig
    };
    let mut wheel_node = [0usize; 4];
    let mut steer_node = [None; 4];
    for (i, name) in names.iter().enumerate() {
        let travel = rig.nodes.len();
        let joint = |kind, axis, index| Some(JointBinding { kind, axis, index });
        rig.nodes.push(RenderNode {
            name: format!("{name}.travel"),
            parent: Some(0),
            role: NodeRole::SuspensionArm,
            rest: Transform::from_pos(hub(i)),
            joint: joint(JointAxisKind::Prismatic, Vec3::Y, 6 + i), // const-ok: joint layout of the stand-in rig
        });
        let mut parent = travel;
        if i < 2 {
            steer_node[i] = Some(rig.nodes.len());
            rig.nodes.push(RenderNode {
                name: format!("{name}.steer"),
                parent: Some(travel),
                role: NodeRole::SteerKnuckle,
                rest: Transform::IDENTITY,
                joint: joint(JointAxisKind::Revolute, Vec3::Y, 4 + i), // const-ok: joint layout of the stand-in rig
            });
            parent = rig.nodes.len() - 1;
        }
        wheel_node[i] = rig.nodes.len();
        rig.nodes.push(RenderNode {
            name: format!("{name}.wheel"),
            parent: Some(parent),
            role: NodeRole::Wheel,
            rest: Transform::IDENTITY,
            joint: joint(JointAxisKind::Revolute, -Vec3::X, i),
        });
    }
    let smooth = p.smooth_angle_deg * std::f64::consts::PI / 180.0; // const-ok: degrees to radians at the authoring edge
    for (part, (mesh, flags)) in parts.iter().zip(&baked) {
        let (node, offset) = match part.role {
            NodeRole::Wheel => (wheel_node[node_of(part.station.unwrap_or(0), part.side)], part.pose.pos),
            NodeRole::SteerKnuckle => {
                (steer_node[node_of(part.station.unwrap_or(0), part.side)].unwrap_or(0), part.pose.pos)
            }
            _ => (0, Vec3::ZERO),
        };
        rig.meshes.push(mesh_part(part, node, mesh, flags, offset, smooth));
    }
    rig
}

fn mesh_part(part: &Part, node: usize, mesh: &Mesh, flags: &Flags, offset: Vec3, smooth_rad: f64) -> MeshPart {
    let (pos, nrm, src, indices) = split_normals(mesh, smooth_rad);
    let f3 = |v: Vec3| [v.x as f32, v.y as f32, v.z as f32];
    MeshPart {
        name: part.name.clone(),
        node,
        material_slot: SLOTS.iter().position(|s| s.0 == part.slot).unwrap_or(0),
        positions: pos.iter().map(|&p| f3(p - offset)).collect(),
        normals: nrm.iter().map(|&n| f3(n)).collect(),
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
