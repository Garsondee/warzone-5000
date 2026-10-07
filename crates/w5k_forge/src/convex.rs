//! Convex pieces stored as half-spaces (planes).
//!
//! Every primitive in a part (box, wedge, cylinder, sphere, custom hull) becomes one convex piece. Keeping
//! pieces as planes makes the operations we need simple and robust:
//! * "is this point inside?" (voxelisation) is a few dot products;
//! * transforms, including mirroring, are plane transforms;
//! * chamfering an edge is adding one bevel plane;
//! * the visible faces fall out of vertex enumeration, already flat-shaded.

use crate::geom::{intersect3, Plane, Xform, V3};
use std::collections::BTreeMap;

/// Geometric tolerance in metres.
pub const EPS: f64 = 1e-7;

/// Where a face came from: the primitive's main surfaces or a chamfer bevel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceKind {
    Main,
    Bevel,
}

#[derive(Clone, Debug)]
pub struct Convex {
    pub planes: Vec<Plane>,
    pub kinds: Vec<FaceKind>,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub plane: Plane,
    pub kind: FaceKind,
    /// Vertex indices, counter-clockwise seen from outside.
    pub idx: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Polyhedron {
    pub verts: Vec<V3>,
    pub faces: Vec<Face>,
}

fn dedupe_points(points: &[V3], tol: f64) -> Vec<V3> {
    let mut out: Vec<V3> = Vec::new();
    for &p in points {
        if !out.iter().any(|q| (*q - p).len() < tol) {
            out.push(p);
        }
    }
    out
}

impl Convex {
    /// Convex hull of a point set as planes. Brute force over point triples, which is robust and fast enough
    /// for the small point sets primitives use (up to about 100 points).
    pub fn from_points(points: &[V3]) -> Convex {
        let pts = dedupe_points(points, 1e-9);
        let scale = pts.iter().fold(0.0f64, |m, p| m.max(p.len())).max(1.0);
        let tol = 1e-9 * scale;
        let mut planes: Vec<Plane> = Vec::new();
        let n = pts.len();
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let Some(mut pl) = Plane::from_points(pts[i], pts[j], pts[k]) else { continue };
                    let mut above = false;
                    let mut below = false;
                    for p in &pts {
                        let d = pl.dist(*p);
                        if d > tol {
                            above = true;
                        } else if d < -tol {
                            below = true;
                        }
                        if above && below {
                            break;
                        }
                    }
                    if above && below {
                        continue;
                    }
                    if above {
                        pl = Plane { n: -pl.n, d: -pl.d };
                    }
                    // Points that are coplanar only up to rounding give several almost identical planes; keep one.
                    if !planes.iter().any(|q| (q.n - pl.n).len() < 1e-6 && (q.d - pl.d).abs() < 1e-6 * scale) {
                        planes.push(pl);
                    }
                }
            }
        }
        let kinds = vec![FaceKind::Main; planes.len()];
        Convex { planes, kinds }
    }

    pub fn contains(&self, p: V3, tol: f64) -> bool {
        self.planes.iter().all(|pl| pl.dist(p) <= tol)
    }

    pub fn transformed(&self, x: &Xform) -> Convex {
        Convex { planes: self.planes.iter().map(|p| p.transformed(x)).collect(), kinds: self.kinds.clone() }
    }

    /// Enumerate vertices (all triple-plane intersections inside every half-space) and assemble faces.
    pub fn polyhedron(&self) -> Polyhedron {
        let np = self.planes.len();
        let mut verts: Vec<V3> = Vec::new();
        for i in 0..np {
            for j in (i + 1)..np {
                for k in (j + 1)..np {
                    let Some(p) = intersect3(&self.planes[i], &self.planes[j], &self.planes[k]) else { continue };
                    if !p.x.is_finite() || !self.contains(p, 1e-6) {
                        continue;
                    }
                    if !verts.iter().any(|q| (*q - p).len() < 1e-6) {
                        verts.push(p);
                    }
                }
            }
        }
        let mut faces = Vec::new();
        for (pi, pl) in self.planes.iter().enumerate() {
            let mut on: Vec<usize> = (0..verts.len()).filter(|&v| pl.dist(verts[v]).abs() < 1e-6).collect();
            if on.len() < 3 {
                continue; // redundant plane (touches only an edge or a point)
            }
            let c = on.iter().fold(V3::ZERO, |a, &v| a + verts[v]) / on.len() as f64;
            let u = (verts[on[0]] - c).norm();
            let w = pl.n.cross(u);
            on.sort_by(|&a, &b| {
                let da = verts[a] - c;
                let db = verts[b] - c;
                let aa = da.dot(w).atan2(da.dot(u));
                let ab = db.dot(w).atan2(db.dot(u));
                aa.partial_cmp(&ab).unwrap()
            });
            // Skip duplicate (or nearly coincident) planes that produce the same face.
            let mut key = on.clone();
            key.sort_unstable();
            if faces.iter().any(|f: &Face| {
                let mut k = f.idx.clone();
                k.sort_unstable();
                k == key || ((f.plane.n - pl.n).len() < 1e-6 && (f.plane.d - pl.d).abs() < 1e-6)
            }) {
                continue;
            }
            faces.push(Face { plane: *pl, kind: self.kinds[pi], idx: on });
        }
        Polyhedron { verts, faces }
    }

    /// Chamfer every edge whose dihedral angle exceeds `min_angle_deg` by `size` metres (measured across each
    /// adjacent face). Returns a new convex with extra bevel planes.
    pub fn beveled(&self, size: f64, min_angle_deg: f64) -> Convex {
        if size <= 0.0 {
            return self.clone();
        }
        let poly = self.polyhedron();
        let min_cos = min_angle_deg.to_radians().cos();
        let mut edges: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
        for (fi, f) in poly.faces.iter().enumerate() {
            for e in 0..f.idx.len() {
                let a = f.idx[e];
                let b = f.idx[(e + 1) % f.idx.len()];
                edges.entry((a.min(b), a.max(b))).or_default().push(fi);
            }
        }
        let mut out = self.clone();
        for ((a, b), fs) in edges {
            if fs.len() != 2 {
                continue;
            }
            let (f1, f2) = (&poly.faces[fs[0]], &poly.faces[fs[1]]);
            if f1.plane.n.dot(f2.plane.n) > min_cos {
                continue; // nearly flat edge (for example between cylinder segments): leave it sharp
            }
            let va = poly.verts[a];
            let vb = poly.verts[b];
            let e = (vb - va).norm();
            let c1 = f1.idx.iter().fold(V3::ZERO, |s, &v| s + poly.verts[v]) / f1.idx.len() as f64;
            let mut t1 = f1.plane.n.cross(e).norm();
            if t1.dot(c1 - va) < 0.0 {
                t1 = -t1;
            }
            let nb = (f1.plane.n + f2.plane.n).norm();
            let p = va + t1 * size;
            out.planes.push(Plane { n: nb, d: nb.dot(p) });
            out.kinds.push(FaceKind::Bevel);
        }
        out
    }
}

impl Polyhedron {
    /// Signed volume by the divergence theorem (positive when faces wind outward).
    pub fn volume(&self) -> f64 {
        let mut v = 0.0;
        for f in &self.faces {
            let p0 = self.verts[f.idx[0]];
            for i in 1..f.idx.len() - 1 {
                v += p0.dot(self.verts[f.idx[i]].cross(self.verts[f.idx[i + 1]]));
            }
        }
        v / 6.0
    }

    /// Exact volume and centroid (sum of tetrahedra from an interior point to each face triangle).
    pub fn volume_centroid(&self) -> (f64, V3) {
        if self.verts.is_empty() {
            return (0.0, V3::ZERO);
        }
        let r = self.verts.iter().fold(V3::ZERO, |a, v| a + *v) / self.verts.len() as f64;
        let mut vol = 0.0;
        let mut first = V3::ZERO;
        for f in &self.faces {
            let a = self.verts[f.idx[0]];
            for i in 1..f.idx.len() - 1 {
                let (b, c) = (self.verts[f.idx[i]], self.verts[f.idx[i + 1]]);
                let v = (a - r).dot((b - r).cross(c - r)) / 6.0;
                vol += v;
                first += (a + b + c + r) * (v / 4.0);
            }
        }
        if vol.abs() < 1e-18 {
            (0.0, r)
        } else {
            (vol, first / vol)
        }
    }

    pub fn aabb(&self) -> (V3, V3) {
        let mut lo = V3 { x: f64::MAX, y: f64::MAX, z: f64::MAX };
        let mut hi = V3 { x: f64::MIN, y: f64::MIN, z: f64::MIN };
        for v in &self.verts {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
        (lo, hi)
    }

    /// True when every edge is shared by exactly two faces in opposite directions (a closed, consistently
    /// wound surface).
    pub fn is_closed(&self) -> bool {
        let mut count: BTreeMap<(usize, usize), i32> = BTreeMap::new();
        for f in &self.faces {
            for e in 0..f.idx.len() {
                let a = f.idx[e];
                let b = f.idx[(e + 1) % f.idx.len()];
                *count.entry((a, b)).or_default() += 1;
            }
        }
        count.iter().all(|(&(a, b), &c)| c == 1 && count.get(&(b, a)) == Some(&1))
    }

    pub fn face_centroid(&self, f: &Face) -> V3 {
        f.idx.iter().fold(V3::ZERO, |s, &v| s + self.verts[v]) / f.idx.len() as f64
    }
}
