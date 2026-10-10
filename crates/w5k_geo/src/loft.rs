//! Lofts: a ship's lines plan. Station sections (rings of x-y points at a z) joined by ruled strips, capped, closed, outward.

use crate::mesh::Mesh;
use w5k_math::Vec3;

/// A ring of (x, y) points at station `z_m`. Every section of one loft has the same number of points, in the same order.
#[derive(Clone, Debug)]
pub struct Section {
    pub z_m: f64,
    pub ring: Vec<[f64; 2]>,
}

/// A plain rectangle, centred at `(0, y_m)`.
pub fn rect_ring(w_m: f64, h_m: f64, y_m: f64) -> Vec<[f64; 2]> {
    let (x, h) = (w_m / 2.0, h_m / 2.0);
    vec![[-x, y_m - h], [-x, y_m + h], [x, y_m + h], [x, y_m - h]]
}

/// A rectangle with its four corners cut by `corner_m` (so the corner edges are 45 degree bevels), and two extra vertices on each straight
/// side at `band_m` from its ends, so the `edge` flag can fall to zero over `band_m` (spike S-G). Always 16 points whatever the sizes
/// (the loop distance shrinks to a third of the side on a short side), so rings of different sizes loft together.
pub fn bevel_ring(w_m: f64, h_m: f64, y_m: f64, corner_m: f64, band_m: f64) -> Vec<[f64; 2]> {
    let (x, h) = (w_m / 2.0, h_m / 2.0);
    let k = corner_m.min(x).min(h);
    // corner points, in the order of `rect_ring`: -x -y, -x +y, +x +y, +x -y, each cut into two
    let sides = [
        ([-x, y_m - h + k], [-x, y_m + h - k]),
        ([-x + k, y_m + h], [x - k, y_m + h]),
        ([x, y_m + h - k], [x, y_m - h + k]),
        ([x - k, y_m - h], [-x + k, y_m - h]),
    ];
    let mut r = Vec::new();
    for (a, b) in sides {
        let len = w5k_math::scalar::hypot(b[0] - a[0], b[1] - a[1]);
        let t = (band_m / len.max(1e-12)).min(1.0 / 3.0); // const-ok: a third of the side at most, so the points stay in order
        let lerp = |s: f64| [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
        r.extend([a, lerp(t), lerp(1.0 - t), b]);
    }
    r
}

/// Loft the sections (increasing z) into a closed mesh. Strips longer than `max_step_m` are split by linear interpolation of the rings
/// (the surface does not change; the vertices let a flag vary along an edge). The end caps are fans, so end rings must be convex.
pub fn loft(sections: &[Section], max_step_m: f64) -> Mesh {
    let mut rings: Vec<(f64, Vec<[f64; 2]>)> = Vec::new();
    for pair in sections.windows(2) {
        let n = ((pair[1].z_m - pair[0].z_m) / max_step_m).ceil().max(1.0) as usize;
        for s in 0..n {
            let t = s as f64 / n as f64;
            let ring = pair[0]
                .ring
                .iter()
                .zip(&pair[1].ring)
                .map(|(a, b)| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
                .collect();
            rings.push((pair[0].z_m + (pair[1].z_m - pair[0].z_m) * t, ring));
        }
    }
    let last = sections.last().expect("a loft needs two sections");
    rings.push((last.z_m, last.ring.clone()));
    let n = last.ring.len() as u32;
    let mut m = Mesh::default();
    for (z, ring) in &rings {
        m.v.extend(ring.iter().map(|p| Vec3::new(p[0], p[1], *z)));
    }
    for r in 0..rings.len() as u32 - 1 {
        for i in 0..n {
            let (a, b) = (r * n + i, r * n + (i + 1) % n);
            m.t.push([a, b, b + n]);
            m.t.push([a, b + n, a + n]);
        }
    }
    let top = (rings.len() as u32 - 1) * n;
    for i in 1..n - 1 {
        m.t.push([0, i + 1, i]);
        m.t.push([top, top + i, top + i + 1]);
    }
    m.orient_outward();
    m
}
