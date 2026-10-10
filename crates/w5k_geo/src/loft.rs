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

fn cross2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn is_convex(pts: &[[f64; 2]]) -> bool {
    let n = pts.len();
    let ccw = (0..n).map(|i| pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1]).sum::<f64>() > 0.0;
    (0..n).all(|i| {
        let c = cross2(pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        if ccw {
            c >= -1e-12 // const-ok: collinear tolerance
        } else {
            c <= 1e-12 // const-ok: collinear tolerance
        }
    })
}

/// Triangles of a simple polygon given as a ring of (x, y) points, convex or not, with collinear points allowed (the loops of a bevel ring).
/// Every boundary edge is traversed by exactly one triangle in the direction the ring runs. Ear clipping on the corner points, then each
/// collinear point is put back by fanning the triangle on its edge from the opposite vertex, so no triangle has zero area.
pub fn triangulate_ring(pts: &[[f64; 2]]) -> Vec<[u32; 3]> {
    let n = pts.len();
    let ccw = (0..n).map(|i| pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1]).sum::<f64>() > 0.0;
    let order: Vec<usize> = if ccw { (0..n).collect() } else { (0..n).rev().collect() };
    let len = |a: [f64; 2], b: [f64; 2]| w5k_math::scalar::hypot(b[0] - a[0], b[1] - a[1]);
    let corner: Vec<bool> = (0..n)
        .map(|k| {
            let (p, q, r) = (pts[order[(k + n - 1) % n]], pts[order[k]], pts[order[(k + 1) % n]]);
            cross2(p, q, r).abs() > 1e-9 * len(p, q) * len(q, r) // const-ok: relative collinearity tolerance
        })
        .collect();
    let mut poly: Vec<usize> = (0..n).filter(|&k| corner[k]).map(|k| order[k]).collect();
    let mut tris: Vec<[usize; 3]> = Vec::new();
    while poly.len() > 3 {
        let k = poly.len();
        let ear = (0..k).find(|&i| {
            let (a, b, c) = (poly[(i + k - 1) % k], poly[i], poly[(i + 1) % k]);
            cross2(pts[a], pts[b], pts[c]) > 0.0
                && !poly.iter().any(|&q| {
                    q != a && q != b && q != c && {
                        let (d1, d2, d3) = (
                            cross2(pts[a], pts[b], pts[q]),
                            cross2(pts[b], pts[c], pts[q]),
                            cross2(pts[c], pts[a], pts[q]),
                        );
                        d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
                    }
                })
        });
        // numerical trouble: clip the most convex corner rather than loop for ever
        let i = ear.unwrap_or_else(|| {
            (0..k)
                .max_by(|&x, &y| {
                    let f = |i: usize| cross2(pts[poly[(i + k - 1) % k]], pts[poly[i]], pts[poly[(i + 1) % k]]);
                    f(x).total_cmp(&f(y))
                })
                .unwrap_or(0)
        });
        tris.push([poly[(i + k - 1) % k], poly[i], poly[(i + 1) % k]]);
        poly.remove(i);
    }
    if poly.len() == 3 {
        tris.push([poly[0], poly[1], poly[2]]);
    }
    // put the collinear points back: between consecutive corners a, b the removed points in ring order
    let corners: Vec<usize> = (0..n).filter(|&k| corner[k]).map(|k| order[k]).collect();
    for (ci, &a) in corners.iter().enumerate() {
        let b = corners[(ci + 1) % corners.len()];
        let (ka, kb) =
            (order.iter().position(|&x| x == a).unwrap_or(0), order.iter().position(|&x| x == b).unwrap_or(0));
        let run: Vec<usize> = (1..n).map(|d| order[(ka + d) % n]).take_while(|&x| x != b).collect();
        if run.is_empty() || ka == kb {
            continue;
        }
        let Some(t) = tris.iter().position(|t| (0..3).any(|r| t[r] == a && t[(r + 1) % 3] == b)) else { continue };
        let tri = tris.swap_remove(t);
        let r = (0..3).find(|&r| tri[r] == a && tri[(r + 1) % 3] == b).unwrap_or(0);
        let apex = tri[(r + 2) % 3];
        let chain: Vec<usize> = std::iter::once(a).chain(run).chain(std::iter::once(b)).collect();
        tris.extend(chain.windows(2).map(|w| [w[0], w[1], apex]));
    }
    tris.into_iter()
        .map(|[a, b, c]| if ccw { [a as u32, b as u32, c as u32] } else { [a as u32, c as u32, b as u32] })
        .collect()
}

/// A ring from the corner polygon `poly` (counter-clockwise or clockwise, no repeated points): each vertex with `chamfer_m[i] > 0` is cut
/// into two points that far along its two edges (a 45 degree bevel on a right angle; the cut is limited to 0.45 of either edge), and each
/// straight edge between the resulting points gets two loop points at `band_m` from its ends (a third of the edge at most), so the
/// `edge` flag can fall to zero over `band_m`. The point count depends only on how many chamfers are non-zero, so rings whose shape
/// changes along a loft still have matching topology.
pub fn polygon_ring(poly: &[[f64; 2]], chamfer_m: &[f64], band_m: f64) -> Vec<[f64; 2]> {
    let n = poly.len();
    let dist = |a: [f64; 2], b: [f64; 2]| w5k_math::scalar::hypot(b[0] - a[0], b[1] - a[1]);
    let mut cut: Vec<([f64; 2], [f64; 2])> = Vec::new(); // (arrival point, departure point) of each vertex
    for i in 0..n {
        let (p, q, r) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        if chamfer_m[i] > 0.0 {
            let c = chamfer_m[i].min(0.45 * dist(p, q)).min(0.45 * dist(q, r)); // const-ok: the two cuts of an edge never overlap
            let (lp, lr) = (dist(q, p), dist(q, r));
            cut.push((
                [q[0] + (p[0] - q[0]) / lp * c, q[1] + (p[1] - q[1]) / lp * c],
                [q[0] + (r[0] - q[0]) / lr * c, q[1] + (r[1] - q[1]) / lr * c],
            ));
        } else {
            cut.push((q, q));
        }
    }
    let mut ring = Vec::new();
    for i in 0..n {
        let (arrive, depart) = cut[i];
        ring.push(arrive);
        if chamfer_m[i] > 0.0 {
            ring.push(depart);
        }
        let next = cut[(i + 1) % n].0;
        let len = dist(depart, next);
        let t = (band_m / len.max(1e-12)).min(1.0 / 3.0); // const-ok: a third of the edge at most, so the points stay in order
        let lerp = |s: f64| [depart[0] + (next[0] - depart[0]) * s, depart[1] + (next[1] - depart[1]) * s];
        ring.extend([lerp(t), lerp(1.0 - t)]);
    }
    ring
}

/// Join rings of 3D points (all with the same number of points, in the same order) into a closed mesh with strips between neighbours and
/// a cap at each end. A convex end ring is capped by a fan from its centroid; a non-convex one is triangulated (`first` and `last` are
/// the 2D shapes of the end rings).
fn skin(rings: &[Vec<Vec3>], first: &[[f64; 2]], last: &[[f64; 2]]) -> Mesh {
    let n = rings[0].len() as u32;
    let mut m = Mesh::default();
    for ring in rings {
        m.v.extend_from_slice(ring);
    }
    for r in 0..rings.len() as u32 - 1 {
        for i in 0..n {
            let (a, b) = (r * n + i, r * n + (i + 1) % n);
            m.t.push([a, b, b + n]);
            m.t.push([a, b + n, a + n]);
        }
    }
    let top = (rings.len() as u32 - 1) * n;
    for (start, shape, flip) in [(0, first, true), (top, last, false)] {
        let tris: Vec<[u32; 3]> = if is_convex(shape) {
            let centroid = rings[(start / n) as usize].iter().fold(Vec3::ZERO, |a, &p| a + p) * (1.0 / f64::from(n));
            m.v.push(centroid);
            let centre = m.v.len() as u32 - 1;
            (0..n).map(|i| [centre, start + i, start + (i + 1) % n]).collect()
        } else {
            triangulate_ring(shape).into_iter().map(|t| t.map(|i| start + i)).collect()
        };
        for t in tris {
            m.t.push(if flip { [t[0], t[2], t[1]] } else { t });
        }
    }
    m.orient_outward();
    m
}

/// Loft the sections (increasing z) into a closed mesh. Strips longer than `max_step_m` are split by linear interpolation of the rings
/// (the surface does not change; the vertices let a flag vary along an edge).
pub fn loft(sections: &[Section], max_step_m: f64) -> Mesh {
    let mut rings: Vec<Vec<Vec3>> = Vec::new();
    for pair in sections.windows(2) {
        let n = ((pair[1].z_m - pair[0].z_m) / max_step_m).ceil().max(1.0) as usize;
        for s in 0..n {
            let t = s as f64 / n as f64;
            let z = pair[0].z_m + (pair[1].z_m - pair[0].z_m) * t;
            rings.push(
                pair[0]
                    .ring
                    .iter()
                    .zip(&pair[1].ring)
                    .map(|(a, b)| Vec3::new(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, z))
                    .collect(),
            );
        }
    }
    let last = sections.last().expect("a loft needs two sections");
    rings.push(last.ring.iter().map(|p| Vec3::new(p[0], p[1], last.z_m)).collect());
    skin(&rings, &sections[0].ring, &last.ring)
}

/// Sweep a cross-section ring of (x, rho) points around an axis along X through `(y_m, z_m)`: rho is the distance from the axis, and
/// `angle_rad` runs from straight up (+Y) towards +Z. The cross-section is the same at every angle. A wheel-arch lip is such a sweep.
pub fn sweep_arc(ring: &[[f64; 2]], axis_yz_m: (f64, f64), angles_rad: &[f64]) -> Mesh {
    let rings: Vec<Vec<Vec3>> = angles_rad
        .iter()
        .map(|&a| {
            let (s, c) = w5k_math::scalar::sin_cos(a);
            ring.iter().map(|p| Vec3::new(p[0], axis_yz_m.0 + p[1] * c, axis_yz_m.1 + p[1] * s)).collect()
        })
        .collect();
    skin(&rings, ring, ring)
}

/// Bounding box centre and half extents of a ring.
fn ring_box(ring: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let (lo, hi) = ring.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(l, h), p| {
        ([l[0].min(p[0]), l[1].min(p[1])], [h[0].max(p[0]), h[1].max(p[1])])
    });
    ([(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0], [(hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0])
}

/// A ring scaled about the centre of its bounding box so that each side moves in by `inset_m` (a 45 degree bevel when the sections are
/// `inset_m` apart in z). Collinear points stay collinear, so the ring keeps its topology.
fn inset_ring(ring: &[[f64; 2]], inset_m: f64) -> Vec<[f64; 2]> {
    let (c, half) = ring_box(ring);
    let s = [1.0 - inset_m / half[0].max(1e-9), 1.0 - inset_m / half[1].max(1e-9)]; // const-ok: guards a zero-size ring
    ring.iter().map(|p| [c[0] + (p[0] - c[0]) * s[0], c[1] + (p[1] - c[1]) * s[1]]).collect()
}

/// The ring of a section list at `z`, by linear interpolation between the sections around it (the surface does not change).
fn ring_at(sections: &[Section], z: f64) -> Vec<[f64; 2]> {
    let i = sections.partition_point(|s| s.z_m <= z).clamp(1, sections.len() - 1);
    let (a, b) = (&sections[i - 1], &sections[i]);
    let t = ((z - a.z_m) / (b.z_m - a.z_m)).clamp(0.0, 1.0);
    a.ring.iter().zip(&b.ring).map(|(p, q)| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]).collect()
}

/// A loft whose ends are bevelled and whose hard edges have vertex rows next to them, so that a per-vertex `edge` flag can fall to zero
/// over `band_m` instead of smearing across a large face (spike S-G): each end becomes a `bevel_m` chamfer (the end ring scaled inward),
/// then an inner ring `band_m` further in, in the plane of the end face (the cap is its triangulation); the sections `bevel_m` and
/// `bevel_m + band_m` from each end repeat the ring there; and every z in `crease_z` (a station where the surface bends) gets a section
/// `band_m` either side of it. Lengths shrink to fit short parts.
pub fn loft_beveled(sections: &[Section], bevel_m: f64, band_m: f64, crease_z: &[f64], max_step_m: f64) -> Mesh {
    let (a, b) = (&sections[0], &sections[sections.len() - 1]);
    let len = b.z_m - a.z_m;
    // an inset never takes more than 45% of a ring's smaller half extent, so a thin ring keeps its shape
    let room = [a, b]
        .iter()
        .map(|s| {
            let h = ring_box(&s.ring).1;
            h[0].min(h[1])
        })
        .fold(f64::MAX, f64::min)
        * 0.45; // const-ok: inset limit
    let bevel = bevel_m.min(len / 4.0).min(room / 2.0);
    let band = band_m.min((len - 2.0 * bevel) / 4.0).max(0.0);
    let inner = (bevel + band).min(room);
    let (lo, hi) = (a.z_m + bevel, b.z_m - bevel);
    let mut zs: Vec<f64> = sections.iter().map(|s| s.z_m).filter(|&z| z >= lo - 1e-9 && z <= hi + 1e-9).collect(); // const-ok: nothing closer than a nanometre
    let mut extra = vec![lo, lo + band, hi, hi - band];
    for &zc in crease_z.iter().filter(|&&z| z > lo + band && z < hi - band) {
        let (prev, next) = (
            zs.iter().copied().filter(|&z| z < zc - 1e-9).fold(lo, f64::max),
            zs.iter().copied().filter(|&z| z > zc + 1e-9).fold(hi, f64::min),
        ); // const-ok: nothing closer than a nanometre
        let d = band.min(0.45 * (zc - prev)).min(0.45 * (next - zc)); // const-ok: loops stay in their own interval
        extra.extend([zc - d, zc + d]);
    }
    for z in extra {
        // const-ok: a vertex row within 5 mm of another is redundant
        if z >= lo - 1e-9 && z <= hi + 1e-9 && zs.iter().all(|&e| (e - z).abs() > 0.005) {
            zs.push(z);
        }
    }
    zs.sort_by(f64::total_cmp);
    let loops = inner - bevel > 1e-3; // const-ok: a loop ring closer than a millimetre to the cap ring would be degenerate
    let mut out = Vec::new();
    if loops {
        out.push(Section { z_m: a.z_m, ring: inset_ring(&a.ring, inner) });
    }
    out.push(Section { z_m: a.z_m, ring: inset_ring(&a.ring, bevel) });
    out.extend(zs.iter().map(|&z| Section { z_m: z, ring: ring_at(sections, z) }));
    out.push(Section { z_m: b.z_m, ring: inset_ring(&b.ring, bevel) });
    if loops {
        out.push(Section { z_m: b.z_m, ring: inset_ring(&b.ring, inner) });
    }
    loft(&out, max_step_m)
}

/// The corner polygon with each corner cut by `chamfer_m` (limited to 0.45 of either edge): the plain outline of a rounded-corner shape.
pub fn chamfer_polygon(poly: &[[f64; 2]], chamfer_m: f64) -> Vec<[f64; 2]> {
    let n = poly.len();
    let dist = |a: [f64; 2], b: [f64; 2]| w5k_math::scalar::hypot(b[0] - a[0], b[1] - a[1]);
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q, r) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        let c = chamfer_m.min(0.45 * dist(p, q)).min(0.45 * dist(q, r)); // const-ok: the two cuts of an edge never overlap
        let (lp, lr) = (dist(q, p), dist(q, r));
        out.push([q[0] + (p[0] - q[0]) / lp * c, q[1] + (p[1] - q[1]) / lp * c]);
        out.push([q[0] + (r[0] - q[0]) / lr * c, q[1] + (r[1] - q[1]) / lr * c]);
    }
    out
}

/// Sweep a small profile around a closed, planar, convex outline (a window frame, a gasket): `profile` points are (s, t), s outward in the
/// plane of the outline from the outline itself, t along `normal`. At each corner the profile is mitered, so the frame has the same width
/// on every side. The result is a closed ring of material with no end caps.
pub fn sweep_loop(outline: &[Vec3], normal: Vec3, profile: &[[f64; 2]]) -> Mesh {
    let n = normal.normalized_or_zero();
    let count = outline.len();
    // wind the outline counter-clockwise about the normal so that `edge x normal` points outward
    let turn: f64 = (0..count).map(|i| n.dot(outline[i].cross(outline[(i + 1) % count]))).sum();
    let pts: Vec<Vec3> = if turn >= 0.0 { outline.to_vec() } else { outline.iter().rev().copied().collect() };
    let m = profile.len() as u32;
    let mut mesh = Mesh::default();
    for i in 0..count {
        let (p, q, r) = (pts[(i + count - 1) % count], pts[i], pts[(i + 1) % count]);
        let (o1, o2) = ((q - p).normalized_or_zero().cross(n), (r - q).normalized_or_zero().cross(n));
        let b = (o1 + o2).normalized_or_zero();
        let miter = 1.0 / b.dot(o1).max(0.3); // const-ok: caps the miter length at a corner sharper than about 70 degrees
        for &[s, t] in profile {
            mesh.v.push(q + b * (s * miter) + n * t);
        }
    }
    let c = count as u32;
    for i in 0..c {
        for k in 0..m {
            let (a, b) = (i * m + k, i * m + (k + 1) % m);
            let (a2, b2) = (((i + 1) % c) * m + k, ((i + 1) % c) * m + (k + 1) % m);
            mesh.t.push([a, b, b2]);
            mesh.t.push([a, b2, a2]);
        }
    }
    mesh.orient_outward();
    mesh
}
