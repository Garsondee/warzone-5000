//! A compact file for a detailed `RenderRig` (a "skin": GEOMETRY's truck), so the live page can fetch it and the repository does not carry
//! two megabytes of JSON for it. Layout, little-endian: `"W5KS" | u32 version | u32 header_len | header JSON | zero padding to 4 | blob`.
//! The header holds the nodes, material slots, joint count and a table with one entry per mesh (counts and blob offsets). In the blob:
//! positions as u16 inside the mesh's bounding box (a step of under 0.1 mm for a 5 m part), normals as three i8, edge and cavity as u8
//! (0..1), indices as u16 (u32 above 65535 vertices). Each section starts on a 4-byte boundary so a browser can view it in place.

use serde::{Deserialize, Serialize};
use w5k_contract::render::{MaterialSlot, MeshPart, RenderNode, RenderRig};

const MAGIC: &[u8; 4] = b"W5KS";
const VERSION: u32 = 1;
/// Quantisation steps of the file format. const-ok: format constants.
const POS_STEPS: f64 = 65535.0; // const-ok: quantisation step of the file format
const NORMAL_STEPS: f64 = 127.0; // const-ok: quantisation step of the file format
const UNIT_STEPS: f64 = 255.0; // const-ok: quantisation step of the file format

#[derive(Serialize, Deserialize)]
struct Header {
    id: String,
    nodes: Vec<RenderNode>,
    material_slots: Vec<MaterialSlot>,
    joint_count: usize,
    meshes: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    name: String,
    node: usize,
    material_slot: usize,
    vertices: usize,
    indices: usize,
    index_bytes: usize,
    min: [f32; 3],
    scale: [f32; 3],
    pos: usize,
    nor: usize,
    /// Offsets of the edge and cavity bytes; `None` when the mesh has no flags.
    edge: Option<usize>,
    cav: Option<usize>,
    idx: usize,
}

fn pad(blob: &mut Vec<u8>) -> usize {
    while !blob.len().is_multiple_of(4) {
        blob.push(0);
    }
    blob.len()
}

pub fn pack(rig: &RenderRig) -> Vec<u8> {
    let mut blob = Vec::new();
    let mut meshes = Vec::new();
    for m in &rig.meshes {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &m.positions {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        if m.positions.is_empty() {
            (lo, hi) = ([0.0; 3], [0.0; 3]);
        }
        let scale = [0, 1, 2].map(|a| (hi[a] - lo[a]) / POS_STEPS as f32);
        let pos = pad(&mut blob);
        for p in &m.positions {
            for a in 0..3 {
                let q =
                    if scale[a] > 0.0 { ((f64::from(p[a] - lo[a])) / f64::from(scale[a])).round() as u16 } else { 0 };
                blob.extend(q.to_le_bytes());
            }
        }
        let nor = pad(&mut blob);
        for n in &m.normals {
            blob.extend(
                n.map(|c| (f64::from(c) * NORMAL_STEPS).round().clamp(-NORMAL_STEPS, NORMAL_STEPS) as i8 as u8),
            );
        }
        let mut unit = |values: &[f32]| {
            if values.is_empty() {
                return None;
            }
            let at = pad(&mut blob);
            blob.extend(values.iter().map(|v| (f64::from(*v) * UNIT_STEPS).round().clamp(0.0, UNIT_STEPS) as u8));
            Some(at)
        };
        let (edge, cav) = (unit(&m.edge), unit(&m.cavity));
        let index_bytes = if m.positions.len() <= 65535 { 2 } else { 4 };
        let idx = pad(&mut blob);
        for &i in &m.indices {
            if index_bytes == 2 {
                blob.extend((i as u16).to_le_bytes());
            } else {
                blob.extend(i.to_le_bytes());
            }
        }
        meshes.push(Entry {
            name: m.name.clone(),
            node: m.node,
            material_slot: m.material_slot,
            vertices: m.positions.len(),
            indices: m.indices.len(),
            index_bytes,
            min: lo,
            scale,
            pos,
            nor,
            edge,
            cav,
            idx,
        });
    }
    let header = Header {
        id: rig.id.clone(),
        nodes: rig.nodes.clone(),
        material_slots: rig.material_slots.clone(),
        joint_count: rig.joint_count,
        meshes,
    };
    let json = serde_json::to_vec(&header).expect("a rig header serialises");
    let mut out = MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(json);
    while !out.len().is_multiple_of(4) {
        out.push(b' ');
    }
    out.extend(blob);
    out
}

/// The inverse (quantised), for tests and tools; the page has its own reader in `tools/viewer/src/skin.js`.
pub fn unpack(bytes: &[u8]) -> Result<RenderRig, String> {
    if bytes.get(..4) != Some(MAGIC) {
        return Err("not a W5KS skin".to_string());
    }
    let word = |at: usize| -> Result<usize, String> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .ok_or_else(|| "the skin is truncated".to_string())
    };
    if word(4)? != VERSION as usize {
        return Err(format!("skin version {} is not supported", word(4)?));
    }
    let hlen = word(8)?;
    let header: Header = serde_json::from_slice(bytes.get(12..12 + hlen).ok_or("the header is truncated")?)
        .map_err(|e| format!("bad header: {e}"))?;
    let blob = bytes.get((12 + hlen).div_ceil(4) * 4..).ok_or("the blob is missing")?;
    let take = |at: usize, n: usize| blob.get(at..at + n).ok_or_else(|| "a mesh section is truncated".to_string());
    let mut meshes = Vec::new();
    for e in &header.meshes {
        let p = take(e.pos, e.vertices * 6)?;
        let positions = (0..e.vertices)
            .map(|v| {
                [0, 1, 2].map(|a| {
                    e.min[a] + f32::from(u16::from_le_bytes([p[v * 6 + a * 2], p[v * 6 + a * 2 + 1]])) * e.scale[a]
                })
            })
            .collect();
        let n = take(e.nor, e.vertices * 3)?;
        let normals =
            (0..e.vertices).map(|v| [0, 1, 2].map(|a| f32::from(n[v * 3 + a] as i8) / NORMAL_STEPS as f32)).collect();
        let unit = |at: Option<usize>| -> Result<Vec<f32>, String> {
            at.map_or(Ok(Vec::new()), |a| {
                Ok(take(a, e.vertices)?.iter().map(|b| f32::from(*b) / UNIT_STEPS as f32).collect())
            })
        };
        let raw = take(e.idx, e.indices * e.index_bytes)?;
        let indices = (0..e.indices)
            .map(|i| {
                if e.index_bytes == 2 {
                    u32::from(u16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]))
                } else {
                    u32::from_le_bytes([raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]])
                }
            })
            .collect();
        meshes.push(MeshPart {
            name: e.name.clone(),
            node: e.node,
            material_slot: e.material_slot,
            positions,
            normals,
            edge: unit(e.edge)?,
            cavity: unit(e.cav)?,
            indices,
        });
    }
    Ok(RenderRig {
        id: header.id,
        nodes: header.nodes,
        meshes,
        material_slots: header.material_slots,
        joint_count: header.joint_count,
        track_runs: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    /// A rig with flags and a mesh above 65535 vertices would be slow to build by hand; the box truck plus one dressed mesh covers both paths.
    fn rig() -> RenderRig {
        let (_, mut rig) = box_truck();
        let m = &mut rig.meshes[0];
        m.edge = (0..m.positions.len()).map(|i| (i % 7) as f32 / 6.0).collect();
        m.cavity = (0..m.positions.len()).map(|i| (i % 5) as f32 / 4.0).collect();
        rig
    }

    #[test]
    fn a_packed_skin_keeps_its_structure_and_positions_within_a_tenth_of_a_millimetre() {
        let rig = rig();
        let back = unpack(&pack(&rig)).unwrap();
        assert_eq!(
            (back.id.as_str(), back.joint_count, back.nodes.len()),
            (rig.id.as_str(), rig.joint_count, rig.nodes.len())
        );
        assert_eq!(back.triangle_count(), rig.triangle_count());
        for (a, b) in rig.meshes.iter().zip(&back.meshes) {
            assert_eq!((&a.name, a.node, a.material_slot, &a.indices), (&b.name, b.node, b.material_slot, &b.indices));
            for (p, q) in a.positions.iter().zip(&b.positions) {
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-4, "{} off by {} m", a.name, (p[k] - q[k]).abs());
                }
            }
        }
        back.validate().unwrap();
    }

    #[test]
    fn packed_normals_stay_within_a_degree_and_flags_within_a_quantum() {
        let rig = rig();
        let back = unpack(&pack(&rig)).unwrap();
        for (a, b) in rig.meshes.iter().zip(&back.meshes) {
            for (n, m) in a.normals.iter().zip(&b.normals) {
                let (dot, ln, lm) =
                    (0..3).fold((0.0, 0.0, 0.0), |s, k| (s.0 + n[k] * m[k], s.1 + n[k] * n[k], s.2 + m[k] * m[k]));
                let cos = dot / (ln.sqrt() * lm.sqrt());
                assert!(cos > 0.9998, "a normal turned by more than a degree (cos {cos})");
                // const-ok: cos(1.1 degrees)
            }
            assert_eq!(a.edge.len(), b.edge.len());
            for (e, f) in a.edge.iter().chain(&a.cavity).zip(b.edge.iter().chain(&b.cavity)) {
                assert!((e - f).abs() <= 0.5 / 255.0 + 1e-6); // const-ok: half a u8 step
            }
        }
    }

    #[test]
    fn a_truncated_or_foreign_skin_is_refused_not_a_panic() {
        assert!(unpack(b"nope").is_err());
        let bytes = pack(&rig());
        assert!(unpack(&bytes[..bytes.len() / 2]).is_err());
    }
}
