//! The `edge` flag: sharpness 0..1 for edge wear. Convex dihedral only, a ramp between two turning angles, concave creases 0.
//! A vertex carries the largest value of the edges that meet there (the "smear" the spike measures).

use crate::mesh::Mesh;
use std::collections::BTreeMap;
use w5k_math::scalar;

/// Value of one edge from the turning angle between its face normals (rad): 0 below `lo_rad`, 1 above `hi_rad`, linear between.
pub fn ramp(turn_rad: f64, lo_rad: f64, hi_rad: f64) -> f64 {
    scalar::clamp((turn_rad - lo_rad) / (hi_rad - lo_rad), 0.0, 1.0)
}

/// Per-edge values `(a, b, value)` in sorted vertex order (a < b), and the per-vertex maximum. The mesh must be welded.
pub fn edge_values(m: &Mesh, lo_rad: f64, hi_rad: f64) -> (Vec<(u32, u32, f64)>, Vec<f64>) {
    let mut faces: BTreeMap<(u32, u32), Vec<(usize, u32)>> = BTreeMap::new(); // edge -> (triangle, opposite vertex)
    for (k, &[a, b, c]) in m.t.iter().enumerate() {
        for (p, q, o) in [(a, b, c), (b, c, a), (c, a, b)] {
            faces.entry((p.min(q), p.max(q))).or_default().push((k, o));
        }
    }
    let mut per_vertex = vec![0.0; m.v.len()];
    let mut per_edge = Vec::new();
    for (&(a, b), f) in &faces {
        let mut value = 0.0;
        if let [(k1, _), (k2, o2)] = f[..] {
            let (n1, n2) = (m.face_normal(k1), m.face_normal(k2));
            let convex = n1.dot(m.v[o2 as usize] - m.v[a as usize]) < 0.0; // the neighbour's far vertex lies behind this face
            if convex {
                value = ramp(scalar::acos(scalar::clamp(n1.dot(n2), -1.0, 1.0)), lo_rad, hi_rad);
            }
        }
        per_edge.push((a, b, value));
        for i in [a, b] {
            per_vertex[i as usize] = f64::max(per_vertex[i as usize], value);
        }
    }
    (per_edge, per_vertex)
}
