//! Render mesh: flat-shaded triangles with per-vertex attributes for the procedural look.
//!
//! Per vertex we store a **colour slot** (the palette decides the colour), an **edge** flag (chamfer faces,
//! used for worn-edge highlights) and **ambient occlusion** (how enclosed the point is by the rest of the
//! part, sampled from the voxel grid). Colours are therefore not baked into geometry: team colours,
//! camouflage and damage are shader inputs.

use crate::build::Piece;
use crate::convex::FaceKind;
use crate::geom::{v3, V3};
use crate::schema::Slot;
use crate::voxel::Grid;

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub slots: Vec<u8>,
    pub edge: Vec<f32>,
    pub ao: Vec<f32>,
    /// Which part each vertex belongs to (vehicles).
    pub part: Vec<u16>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Tolerance for "touching counts as inside" when culling hidden faces (m).
const CULL_TOL: f64 = 1e-5;

/// Build the mesh of a set of pieces. Faces lying entirely inside or on another piece are hidden and dropped.
pub fn build_mesh(pieces: &[Piece], grid: &Grid) -> Mesh {
    let mut m = Mesh::default();
    let boxes: Vec<(V3, V3)> = pieces.iter().map(|p| p.poly.aabb()).collect();
    let inside_box = |b: &(V3, V3), p: V3| {
        p.x >= b.0.x - CULL_TOL && p.y >= b.0.y - CULL_TOL && p.z >= b.0.z - CULL_TOL
            && p.x <= b.1.x + CULL_TOL && p.y <= b.1.y + CULL_TOL && p.z <= b.1.z + CULL_TOL
    };
    let ao_reach = {
        let span = grid.max_corner() - grid.origin;
        span.x.max(span.y).max(span.z) * 0.12
    };
    for (pi, p) in pieces.iter().enumerate() {
        for f in &p.poly.faces {
            let centroid = p.poly.face_centroid(f);
            let covered = pieces.iter().enumerate().any(|(qi, q)| {
                qi != pi
                    && inside_box(&boxes[qi], centroid)
                    && q.convex.contains(centroid, CULL_TOL)
                    && f.idx.iter().all(|&v| q.convex.contains(p.poly.verts[v], CULL_TOL))
            });
            if covered {
                continue;
            }
            let n = f.plane.n;
            let base = m.positions.len() as u32;
            for &vi in &f.idx {
                let v = p.poly.verts[vi];
                m.positions.push([v.x as f32, v.y as f32, v.z as f32]);
                m.normals.push([n.x as f32, n.y as f32, n.z as f32]);
                m.slots.push(p.slot.index());
                m.edge.push(if f.kind == FaceKind::Bevel { 1.0 } else { 0.0 });
                m.ao.push(ambient_occlusion(grid, v, n, ao_reach));
                m.part.push(p.part);
            }
            for k in 1..(f.idx.len() as u32 - 1) {
                m.indices.extend_from_slice(&[base, base + k, base + k + 1]);
            }
        }
    }
    m
}

/// Fraction of open sky above a surface point (1 = fully open), from rays marched through the voxel grid
/// out to `reach` metres.
fn ambient_occlusion(g: &Grid, p: V3, n: V3, reach: f64) -> f32 {
    const RAYS: usize = 24;
    let start = p + n * (g.cell * 1.5);
    let t = n.any_perp();
    let b = n.cross(t);
    let mut open = 0.0;
    let mut total = 0.0;
    for k in 0..RAYS {
        // Cosine-weighted hemisphere directions from a golden-angle spiral (fixed, so results are repeatable).
        let fk = (k as f64 + 0.5) / RAYS as f64;
        let r = fk.sqrt();
        let phi = k as f64 * 2.399_963_229_728_653;
        let local = v3(r * phi.cos(), r * phi.sin(), (1.0 - fk).sqrt());
        let d = t * local.x + b * local.y + n * local.z;
        let mut blocked = 0.0;
        let steps = 16;
        for s in 1..=steps {
            let q = start + d * (reach * s as f64 / steps as f64);
            if g.occupied_at(q) {
                blocked = 1.0 - (s as f64 - 1.0) / steps as f64 * 0.5; // near occluders darken more
                break;
            }
        }
        open += 1.0 - blocked;
        total += 1.0;
    }
    (open / total) as f32
}

/// Colour slot of a mesh vertex.
pub fn slot_of(i: u8) -> Slot {
    Slot::ALL[i as usize]
}
