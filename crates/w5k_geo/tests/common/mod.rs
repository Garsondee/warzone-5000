//! Helpers shared by the skin tests: ray crossings, inside tests, and the dimensions a set of generated parts measures.

#![allow(dead_code)]

use w5k_contract::render::NodeRole;
use w5k_geo::mesh::Mesh;
use w5k_geo::part::Part;
use w5k_geo::skin::Skin;
use w5k_math::{scalar, Vec3};

/// Crossings of a ray with a mesh: sorted distances.
pub fn crossings(m: &Mesh, o: Vec3, d: Vec3) -> Vec<f64> {
    let mut ts = Vec::new();
    for k in 0..m.t.len() {
        let t = m.tri(k);
        let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
        let p = d.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-14 {
            continue;
        }
        let s = o - t[0];
        let (u, q) = (s.dot(p) / det, s.cross(e1));
        let v = d.dot(q) / det;
        let at = e2.dot(q) / det;
        if u > 0.0 && v > 0.0 && u + v < 1.0 && at > 0.0 {
            ts.push(at);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts
}

/// Odd number of crossings along an irrational direction: the point is inside the closed mesh.
pub fn inside(m: &Mesh, p: Vec3) -> bool {
    crossings(m, p, Vec3::new(0.371_1, 0.568_3, 0.735_7).normalized_or_zero()).len() % 2 == 1
}

/// The vertices of a mesh and the midpoints of its edges: enough sample points to see a thin pin pass through a box.
pub fn samples(m: &Mesh) -> Vec<Vec3> {
    let mut s = m.v.clone();
    for t in &m.t {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            s.push((m.v[t[a] as usize] + m.v[t[b] as usize]) * 0.5);
        }
    }
    s
}

/// The two solids share volume: a sample point of one lies inside the other.
pub fn overlap(a: &Mesh, b: &Mesh) -> bool {
    let ((alo, ahi), (blo, bhi)) = (a.bounds(), b.bounds());
    let apart = alo.x > bhi.x || blo.x > ahi.x || alo.y > bhi.y || blo.y > ahi.y || alo.z > bhi.z || blo.z > ahi.z;
    !apart && (samples(a).iter().any(|&v| inside(b, v)) || samples(b).iter().any(|&v| inside(a, v)))
}

/// What a skin's parts measure, in the terms of the hull definition: the box of the non-fitting hull parts, the hub spacing, the tread.
#[derive(Clone, Copy, Debug)]
pub struct Measured {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub wheelbase_m: f64,
    pub track_m: f64,
    pub wheel_diameter_m: f64,
    pub clearance_m: f64,
}

pub fn measure(skin: &Skin, parts: &[Part]) -> Measured {
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| p.in_hull_frame().bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    let hubs: Vec<Vec3> =
        parts.iter().filter(|p| p.role == NodeRole::Wheel && p.name.starts_with("rim")).map(|p| p.pose.pos).collect();
    let (xmax, zmin, zmax) =
        hubs.iter().fold((0.0_f64, f64::MAX, f64::MIN), |(x, a, b), h| (x.max(h.x), a.min(h.z), b.max(h.z)));
    let tyre = parts
        .iter()
        .find(|p| p.name == "tread.0.r")
        .map_or(0.0, |p| p.mesh.v.iter().fold(0.0_f64, |r, v| r.max(scalar::hypot(v.y, v.z))));
    Measured {
        length_m: hi.z - lo.z,
        width_m: hi.x - lo.x,
        height_m: hi.y - lo.y,
        wheelbase_m: zmax - zmin,
        track_m: 2.0 * xmax,
        wheel_diameter_m: 2.0 * tyre,
        clearance_m: skin.dims.ride_height_m() + lo.y,
    }
}
