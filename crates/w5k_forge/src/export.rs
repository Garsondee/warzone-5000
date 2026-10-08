//! Compact vehicle meshes for viewers (the time-trial page, and later the engine).
//!
//! A viewer needs far less than the forge keeps: no normals (the faces are flat, so a viewer recovers them from screen-space
//! derivatives), no colours (a palette plus the per-vertex *slot* reproduces them, which also keeps team colours a viewer
//! input). What it does need, per vertex: a position, the slot and edge flag, the ambient occlusion and the part index.
//!
//! The blob is little-endian and planar, so it compresses well (the page deflates it):
//!
//! | array | element | count |
//! |---|---|---|
//! | `pos` | 3 x u16, the position quantised over the bounding box (`lo` to `hi`) | `n_vertices` |
//! | `attr` | u8: colour slot in the low 3 bits, bit 3 = chamfer edge | `n_vertices` |
//! | `ao` | u8: ambient occlusion x 255 | `n_vertices` |
//! | `part` | u16: index into the assembly's parts | `n_vertices` |
//! | `idx` | u16, or u32 when `wide_indices` | `n_indices` |
//!
//! Quantising to 16 bits over the box costs under 1 mm for a vehicle up to 65 m long.

use crate::mesh::Mesh;

/// A mesh packed for transport.
#[derive(Clone, Debug)]
pub struct ExportMesh {
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub n_vertices: u32,
    pub n_indices: u32,
    /// Indices are u32 instead of u16 (more than 65,535 vertices).
    pub wide_indices: bool,
    /// Lowest point of the mesh: where the vehicle touches the ground.
    pub ground_y: f32,
    pub blob: Vec<u8>,
}

/// The arrays of an [`ExportMesh`], unpacked (for tests and tools).
#[derive(Clone, Debug, Default)]
pub struct Unpacked {
    pub positions: Vec<[f32; 3]>,
    pub slots: Vec<u8>,
    pub edge: Vec<bool>,
    pub ao: Vec<f32>,
    pub part: Vec<u16>,
    pub indices: Vec<u32>,
}

pub fn export_mesh(mesh: &Mesh) -> ExportMesh {
    let n = mesh.positions.len();
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in &mesh.positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    if n == 0 {
        lo = [0.0; 3];
        hi = [0.0; 3];
    }
    let wide = n > u16::MAX as usize;
    let mut blob = Vec::with_capacity(n * 10 + mesh.indices.len() * if wide { 4 } else { 2 });
    for p in &mesh.positions {
        for (k, &c) in p.iter().enumerate() {
            let extent = (hi[k] - lo[k]) as f64;
            let q = if extent > 0.0 { (((c - lo[k]) as f64 / extent) * 65535.0).round() } else { 0.0 };
            blob.extend_from_slice(&(q as u16).to_le_bytes());
        }
    }
    for i in 0..n {
        blob.push((mesh.slots[i] & 7) | if mesh.edge[i] > 0.5 { 8 } else { 0 });
    }
    for i in 0..n {
        blob.push((mesh.ao[i].clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    for i in 0..n {
        blob.extend_from_slice(&mesh.part[i].to_le_bytes());
    }
    for &ix in &mesh.indices {
        if wide {
            blob.extend_from_slice(&ix.to_le_bytes());
        } else {
            blob.extend_from_slice(&(ix as u16).to_le_bytes());
        }
    }
    ExportMesh { lo, hi, n_vertices: n as u32, n_indices: mesh.indices.len() as u32, wide_indices: wide, ground_y: lo[1], blob }
}

impl ExportMesh {
    /// Unpack the blob (what a viewer does).
    pub fn unpack(&self) -> Unpacked {
        let n = self.n_vertices as usize;
        let b = &self.blob;
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let mut out = Unpacked::default();
        for i in 0..n {
            let mut p = [0f32; 3];
            for (k, c) in p.iter_mut().enumerate() {
                let q = u16_at((i * 3 + k) * 2) as f32 / 65535.0;
                *c = self.lo[k] + q * (self.hi[k] - self.lo[k]);
            }
            out.positions.push(p);
        }
        let attr = 6 * n;
        let ao = attr + n;
        let part = ao + n;
        let idx = part + 2 * n;
        for i in 0..n {
            out.slots.push(b[attr + i] & 7);
            out.edge.push(b[attr + i] & 8 != 0);
            out.ao.push(b[ao + i] as f32 / 255.0);
            out.part.push(u16_at(part + 2 * i));
        }
        for i in 0..self.n_indices as usize {
            out.indices.push(if self.wide_indices {
                u32::from_le_bytes([b[idx + 4 * i], b[idx + 4 * i + 1], b[idx + 4 * i + 2], b[idx + 4 * i + 3]])
            } else {
                u16_at(idx + 2 * i) as u32
            });
        }
        out
    }
}
