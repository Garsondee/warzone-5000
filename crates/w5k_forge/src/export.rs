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
//! | `joint` | u16: 0 if the vertex does not move, else the assembly's joint index + 1 | `n_vertices` |
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
    pub joint: Vec<u16>,
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
    let mut blob = Vec::with_capacity(n * 12 + mesh.indices.len() * if wide { 4 } else { 2 });
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
    for i in 0..n {
        blob.extend_from_slice(&mesh.joint.get(i).copied().unwrap_or(0).to_le_bytes());
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
        let joint = part + 2 * n;
        let idx = joint + 2 * n;
        for i in 0..n {
            out.slots.push(b[attr + i] & 7);
            out.edge.push(b[attr + i] & 8 != 0);
            out.ao.push(b[ao + i] as f32 / 255.0);
            out.part.push(u16_at(part + 2 * i));
            out.joint.push(u16_at(joint + 2 * i));
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

// ---------------------------------------------------------------------------------------------------------------------
// Joints: what moves, resolved for viewers.

use serde::Serialize;

use crate::assemble::Assembly;
use crate::geom::V3;
use crate::schema::Motion;

/// Share of a gait cycle a foot spends on the ground (the rest is the swing).
pub const GAIT_DUTY: f64 = 0.6;

/// A moving sub-assembly, in vehicle space and ready to animate. Every viewer animates the same way from the replay, so the angles
/// are never stored: `kind` says which law to use and the other fields are its parameters.
///
/// The `axis` is oriented so that a *positive* angle is always the natural one: a wheel rolling forward, a leg swinging forward, a
/// foot lifting. Forward is -Z, up is +Y (the engine's convention).
#[derive(Clone, Debug, Serialize)]
pub struct ExportJoint {
    pub name: String,
    /// Index of the part it belongs to.
    pub part: usize,
    /// The joint it hangs from (an index into the same list), or -1.
    pub parent: i32,
    pub pivot: [f64; 3],
    pub axis: [f64; 3],
    /// `"roll"` (angle = distance rolled / `radius`), `"spin"` (`rps` turns a second), `"hip"` (swings fore and aft by `amp` radians
    /// as the gait cycles every `stride` metres) or `"knee"` (lifts by up to `amp` radians in the swing).
    pub kind: &'static str,
    pub radius: f64,
    pub rps: f64,
    pub stride: f64,
    pub amp: f64,
    /// Where in the gait cycle this leg is when the vehicle starts (0 to 1): alternate legs half a cycle apart.
    pub phase: f64,
}

fn horizontal(v: V3) -> V3 {
    V3 { x: v.x, y: 0.0, z: v.z }
}

/// Turn `axis` (or leave it) so that rotating by a positive angle about it moves `point` (relative to the pivot) in direction `want`.
fn oriented(axis: V3, point: V3, want: V3) -> V3 {
    if axis.cross(point).dot(want) >= 0.0 {
        axis
    } else {
        -axis
    }
}

/// Resolve every joint of an assembly for viewers.
pub fn export_joints(asm: &Assembly) -> Vec<ExportJoint> {
    let (up, fwd) = (V3 { x: 0.0, y: 1.0, z: 0.0 }, V3 { x: 0.0, y: 0.0, z: -1.0 });
    let mut out: Vec<ExportJoint> = Vec::with_capacity(asm.joints.len());
    for j in &asm.joints {
        let mut e = ExportJoint {
            name: j.name.clone(),
            part: j.part,
            parent: j.parent.map(|p| p as i32).unwrap_or(-1),
            pivot: j.pivot.arr(),
            axis: j.axis.arr(),
            kind: "spin",
            radius: 0.0,
            rps: 0.0,
            stride: 0.0,
            amp: 0.0,
            phase: 0.0,
        };
        match &j.motion {
            Motion::Roll { radius } => {
                // Positive = the top of the wheel moves forward.
                e.kind = "roll";
                e.radius = *radius;
                e.axis = oriented(j.axis, up, fwd).arr();
            }
            Motion::Spin { rps } => {
                e.rps = *rps;
            }
            Motion::Hip { foot, stride } => {
                let reach = horizontal(V3::from_arr(*foot) - j.pivot);
                e.kind = "hip";
                e.stride = *stride;
                // The foot swings through +-amp so that it stays planted while the body passes over it: it moves back by `duty` of a
                // stride during the stance.
                e.amp = if reach.len() > 1e-3 { (GAIT_DUTY * stride / (2.0 * reach.len())).clamp(0.0, 0.9).asin() } else { 0.3 };
                e.axis = oriented(up, reach, fwd).arr();
            }
            Motion::Knee { foot, lift } => {
                let rel = V3::from_arr(*foot) - j.pivot;
                let h = horizontal(rel);
                e.kind = "knee";
                // The knee turns about the axis perpendicular to the leg's plane (the plane through the vertical and the foot); a positive
                // angle lifts the foot.
                let plane = if h.len() > 1e-3 { up.cross(h).norm() } else { j.axis };
                e.axis = oriented(plane, rel, up).arr();
                e.amp = (lift / h.len().max(0.3 * rel.len())).clamp(0.05, 0.6);
            }
        }
        out.push(e);
    }
    // Gait phases: along each side the legs alternate, and the two sides are half a cycle apart, so that the feet on the ground
    // always form a rough tripod (or quad) however many legs there are.
    let mut hips: Vec<usize> = (0..out.len()).filter(|&i| out[i].kind == "hip").collect();
    hips.sort_by(|&a, &b| out[a].pivot[2].total_cmp(&out[b].pivot[2]));
    let mut rank = [0usize; 2];
    for &i in &hips {
        let side = if out[i].pivot[0] > 0.0 { 0 } else { 1 };
        out[i].phase = 0.5 * ((rank[side] + side) % 2) as f64;
        rank[side] += 1;
    }
    for i in 0..out.len() {
        if out[i].kind == "knee" && out[i].parent >= 0 {
            out[i].phase = out[out[i].parent as usize].phase;
            out[i].stride = out[out[i].parent as usize].stride;
        }
    }
    out
}
