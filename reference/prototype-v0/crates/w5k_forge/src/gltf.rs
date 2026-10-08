//! Minimal glTF 2.0 binary (`.glb`) writer, so parts can be opened in Blender or Godot.
//!
//! One mesh with one primitive:
//! * `POSITION`, `NORMAL` (flat, per face);
//! * `COLOR_0`: the palette colour with edge wear and ambient occlusion baked in, for viewers that only
//!   understand vertex colours;
//! * `_W5K`: the raw attributes the game shader uses (slot index, edge flag, ambient occlusion, part index),
//!   so nothing is lost.
//!
//! Sockets become empty child nodes named `socket_<name>`, oriented so their local +Y is the socket normal and
//! local -Z its forward direction (Godot's convention), which makes them easy to inspect and snap to.

use crate::geom::V3;
use crate::mesh::Mesh;
use crate::raster::{scale, Rgb};
use serde_json::{json, Value};

pub struct GlbSocket {
    pub name: String,
    pub at: V3,
    pub normal: V3,
    pub forward: V3,
}

/// Quaternion (x, y, z, w) of the rotation whose columns are the given orthonormal axes.
fn quat_from_axes(x: V3, y: V3, z: V3) -> [f64; 4] {
    let (m00, m01, m02) = (x.x, y.x, z.x);
    let (m10, m11, m12) = (x.y, y.y, z.y);
    let (m20, m21, m22) = (x.z, y.z, z.z);
    let tr = m00 + m11 + m22;
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s]
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        [0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        [(m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s]
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        [(m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s]
    };
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    q.map(|c| c / l)
}

fn socket_node(s: &GlbSocket) -> Value {
    let y = s.normal.norm();
    let mut z = -(s.forward - y * s.forward.dot(y));
    if z.len() < 1e-9 {
        z = y.any_perp();
    }
    let z = z.norm();
    let x = y.cross(z);
    json!({
        "name": format!("socket_{}", s.name),
        "translation": [s.at.x, s.at.y, s.at.z],
        "rotation": quat_from_axes(x, y, z),
    })
}

fn push_f32(bin: &mut Vec<u8>, v: f32) {
    bin.extend_from_slice(&v.to_le_bytes());
}

/// Encode a mesh (coloured with `colours`, one linear colour per slot) and its sockets as GLB bytes.
pub fn write_glb(name: &str, mesh: &Mesh, colours: &[Rgb; 8], sockets: &[GlbSocket]) -> Vec<u8> {
    let n = mesh.positions.len();
    let mut bin: Vec<u8> = Vec::new();
    let mut views: Vec<Value> = Vec::new();
    let mut view = |bin: &mut Vec<u8>, start: usize, target: u32| {
        views.push(json!({"buffer": 0, "byteOffset": start, "byteLength": bin.len() - start, "target": target}));
        views.len() - 1
    };

    let start = bin.len();
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in &mesh.positions {
        for k in 0..3 {
            push_f32(&mut bin, p[k]);
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let v_pos = view(&mut bin, start, 34962);

    let start = bin.len();
    for p in &mesh.normals {
        for &c in p {
            push_f32(&mut bin, c);
        }
    }
    let v_nrm = view(&mut bin, start, 34962);

    let start = bin.len();
    for i in 0..n {
        let base = colours[mesh.slots[i] as usize];
        let worn = if mesh.edge[i] > 0.5 { scale(base, 1.5) } else { base };
        let c = scale(worn, 0.35 + 0.65 * mesh.ao[i]);
        for k in 0..3 {
            push_f32(&mut bin, c[k].min(1.0));
        }
        push_f32(&mut bin, 1.0);
    }
    let v_col = view(&mut bin, start, 34962);

    let start = bin.len();
    for i in 0..n {
        push_f32(&mut bin, mesh.slots[i] as f32);
        push_f32(&mut bin, mesh.edge[i]);
        push_f32(&mut bin, mesh.ao[i]);
        push_f32(&mut bin, mesh.part[i] as f32);
    }
    let v_w5k = view(&mut bin, start, 34962);

    let start = bin.len();
    for &i in &mesh.indices {
        bin.extend_from_slice(&i.to_le_bytes());
    }
    let v_idx = view(&mut bin, start, 34963);

    let accessors = json!([
        {"bufferView": v_pos, "componentType": 5126, "count": n, "type": "VEC3", "min": lo, "max": hi},
        {"bufferView": v_nrm, "componentType": 5126, "count": n, "type": "VEC3"},
        {"bufferView": v_col, "componentType": 5126, "count": n, "type": "VEC4"},
        {"bufferView": v_w5k, "componentType": 5126, "count": n, "type": "VEC4"},
        {"bufferView": v_idx, "componentType": 5125, "count": mesh.indices.len(), "type": "SCALAR"},
    ]);
    let socket_nodes: Vec<Value> = sockets.iter().map(socket_node).collect();
    let children: Vec<usize> = (1..=socket_nodes.len()).collect();
    let mut nodes = vec![json!({"name": name, "mesh": 0, "children": children})];
    nodes.extend(socket_nodes);
    if children.is_empty() {
        nodes[0].as_object_mut().unwrap().remove("children");
    }
    let doc = json!({
        "asset": {"version": "2.0", "generator": "w5k_forge"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": nodes,
        "meshes": [{
            "name": name,
            "primitives": [{
                "attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2, "_W5K": 3},
                "indices": 4,
                "material": 0,
            }],
        }],
        "materials": [{
            "name": "w5k_vertex_colour",
            "pbrMetallicRoughness": {"baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.8},
        }],
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": bin.len()}],
    });

    let mut json_bytes = serde_json::to_vec(&doc).expect("glTF JSON serialises");
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&0x4654_6C67u32.to_le_bytes()); // "glTF"
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes()); // "JSON"
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942u32.to_le_bytes()); // "BIN\0"
    out.extend_from_slice(&bin);
    out
}
