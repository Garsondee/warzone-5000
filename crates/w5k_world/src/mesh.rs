//! A decimated terrain mesh (V1 of `docs/lanes/world/visual-target.md`): the heightfield as a few thousand flat triangles instead of
//! millions, for the viewers and the preview.
//!
//! The method is **RTIN** (right-triangle irregular network). Cut the square along its diagonal into two right triangles; a triangle can be
//! split by the line from its right-angle corner to the midpoint of its long side (the hypotenuse) into two smaller right triangles, and so on
//! down to one-cell triangles. A triangle is kept whole when the ground inside it stays within a tolerance of its plane. Two triangles that
//! share a hypotenuse must split together or a crack opens along it, so the error is stored once per hypotenuse midpoint and read by both,
//! and a parent's error is never less than its children's (so the tree is consistent).
//!
//! Here the error of a triangle is *exact*: the largest vertical distance from its plane to any grid node inside it. A triangle that holds
//! more than one material (a road edge, a river bank) is never kept whole, so material boundaries stay sharp. The square is padded to
//! `2^k + 1` nodes (the tree needs a power of two) by repeating the edge row and column, and the padding is clipped away at export.
//! Presentation only: the collision world is the heightfield.

use std::collections::BTreeMap;

use crate::grid::{GridWorld, CELL_M};

#[derive(Clone, Debug)]
pub struct TerrainMesh {
    /// Plan x, height, plan z of each vertex, m.
    pub vertices: Vec<[f64; 3]>,
    /// Counter-clockwise seen from above (normal up).
    pub triangles: Vec<[u32; 3]>,
    /// Splat id of each triangle.
    pub material: Vec<u8>,
    pub tolerance_m: f64,
}

type P = (i64, i64);
type Tri = (P, P, P);

struct Field<'a> {
    w: &'a GridWorld,
    n: i64,
}

impl Field<'_> {
    fn h(&self, p: P) -> f64 {
        self.w.height_at_node(p.0.min(self.n - 1) as usize, p.1.min(self.n - 1) as usize)
    }

    fn m(&self, p: P) -> u8 {
        self.w.splat_at_node(p.0.min(self.n - 1) as usize, p.1.min(self.n - 1) as usize)
    }
}

fn edge(a: P, b: P, c: P) -> i64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn mid(a: P, b: P) -> P {
    ((a.0 + b.0) / 2, (a.1 + b.1) / 2)
}

/// A triangle (right angle at `c`, long side `a`-`b`) can split while the long side's midpoint is a grid node: a one-cell diagonal cannot.
fn splittable((a, b, _): Tri) -> bool {
    (a.0 - b.0) % 2 == 0 && (a.1 - b.1) % 2 == 0 // const-ok: a midpoint is a node when both differences are even
}

fn children((a, b, c): Tri) -> [Tri; 2] {
    let m = mid(a, b);
    [(c, a, m), (b, c, m)]
}

/// Every real node (`0..n` on both axes) inside the triangle, edges included, with its barycentric weights.
fn for_nodes(n: i64, (a, b, c): Tri, mut f: impl FnMut(P, [f64; 3])) {
    let area = edge(a, b, c);
    if area == 0 {
        return;
    }
    let (x0, x1) = (a.0.min(b.0).min(c.0).max(0), a.0.max(b.0).max(c.0).min(n - 1));
    let (y0, y1) = (a.1.min(b.1).min(c.1).max(0), a.1.max(b.1).max(c.1).min(n - 1));
    let s = area.signum();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = (x, y);
            let w = [edge(b, c, p) * s, edge(c, a, p) * s, edge(a, b, p) * s];
            if w.iter().all(|&v| v >= 0) {
                let t = area.abs() as f64;
                f(p, w.map(|v| v as f64 / t));
            }
        }
    }
}

fn visit(t: Tri, depth: u32, target: u32, f: &mut impl FnMut(Tri)) {
    if depth == target {
        f(t);
    } else if splittable(t) {
        for c in children(t) {
            visit(c, depth + 1, target, f);
        }
    }
}

/// The two triangles that tile a square of side `size` (a power of two) nodes.
fn roots(size: i64) -> [Tri; 2] {
    [((0, 0), (size, size), (size, 0)), ((size, size), (0, 0), (0, size))]
}

/// Build the mesh: every grid node is within `tolerance_m` of it vertically.
pub fn build(world: &GridWorld, tolerance_m: f64) -> TerrainMesh {
    let n = world.n() as i64;
    let size = ((n - 1) as u64).next_power_of_two() as i64;
    let side = (size + 1) as usize;
    let field = Field { w: world, n };
    let mut err = vec![0.0f64; side * side];
    let at = |p: P| p.1 as usize * side + p.0 as usize;
    let depth = size.trailing_zeros() * 2; // a split turns a diagonal long side into an axis one and back: the size halves every two levels
                                           // Smallest triangles first, so a parent can read its children's errors, and both triangles on a shared long side update one entry.
    for level in (0..depth).rev() {
        for root in roots(size) {
            visit(root, 0, level, &mut |t| {
                let m = mid(t.0, t.1);
                let (ha, hb, hc) = (field.h(t.0), field.h(t.1), field.h(t.2));
                let (mut dev, mut first, mut mixed) = (0.0f64, None, false);
                for_nodes(n, t, |p, w| {
                    dev = dev.max((field.h(p) - (ha * w[0] + hb * w[1] + hc * w[2])).abs());
                    let id = field.m(p);
                    mixed |= *first.get_or_insert(id) != id;
                });
                let mut e = err[at(m)].max(if mixed { f64::INFINITY } else { dev });
                for c in children(t) {
                    if splittable(c) {
                        e = e.max(err[at(mid(c.0, c.1))]);
                    }
                }
                err[at(m)] = e;
            });
        }
    }
    let mut leaves: Vec<Tri> = Vec::new();
    fn collect(t: Tri, tol: f64, err: &[f64], at: &impl Fn(P) -> usize, n: i64, out: &mut Vec<Tri>) {
        let outside = |f: fn(P) -> i64| f(t.0) >= n - 1 && f(t.1) >= n - 1 && f(t.2) >= n - 1;
        if outside(|p| p.0) || outside(|p| p.1) {
            return; // only padding (or just touching the border)
        }
        if splittable(t) && err[at(mid(t.0, t.1))] > tol {
            for c in children(t) {
                collect(c, tol, err, at, n, out);
            }
        } else {
            out.push(t);
        }
    }
    for root in roots(size) {
        collect(root, tolerance_m, &err, &at, n, &mut leaves);
    }
    assemble(world, &field, &leaves, tolerance_m)
}

/// Clip the padded triangles to the real square, share vertices, and orient every face up.
fn assemble(world: &GridWorld, field: &Field, leaves: &[Tri], tolerance_m: f64) -> TerrainMesh {
    let n = field.n;
    let top = (n - 1) as f64;
    let half = top * CELL_M * 0.5;
    let mut mesh = TerrainMesh { vertices: Vec::new(), triangles: Vec::new(), material: Vec::new(), tolerance_m };
    let mut index: BTreeMap<(i64, i64), u32> = BTreeMap::new();
    let micro = |v: f64| (v * 1e6).round() as i64; // const-ok: vertices closer than a micrometre are one vertex
    for t in leaves {
        // Plan position (in nodes) and height of each corner; padded corners keep the repeated edge height, so the plane is the one the error used.
        let mut poly: Vec<[f64; 3]> = [t.0, t.1, t.2].iter().map(|&p| [p.0 as f64, p.1 as f64, field.h(p)]).collect();
        for axis in 0..2 {
            let src = std::mem::take(&mut poly);
            for k in 0..src.len() {
                let (p, q) = (src[k], src[(k + 1) % src.len()]);
                let (pin, qin) = (p[axis] <= top, q[axis] <= top);
                if pin {
                    poly.push(p);
                }
                if pin != qin {
                    // Compute from the lexicographically smaller end so two triangles sharing the edge get the identical point.
                    let (lo, hi) = if (p[0], p[1]) <= (q[0], q[1]) { (p, q) } else { (q, p) };
                    let s = (top - lo[axis]) / (hi[axis] - lo[axis]);
                    poly.push([0, 1, 2].map(|i| lo[i] + (hi[i] - lo[i]) * s));
                }
            }
        }
        let ids: Vec<u32> = poly
            .iter()
            .map(|p| {
                *index.entry((micro(p[0]), micro(p[1]))).or_insert_with(|| {
                    mesh.vertices.push([p[0] * CELL_M - half, p[2], p[1] * CELL_M - half]);
                    (mesh.vertices.len() - 1) as u32
                })
            })
            .collect();
        for k in 1..ids.len().saturating_sub(1) {
            let mut tri = [ids[0], ids[k], ids[k + 1]];
            let v = tri.map(|i| mesh.vertices[i as usize]);
            let normal_y = (v[1][2] - v[0][2]) * (v[2][0] - v[0][0]) - (v[1][0] - v[0][0]) * (v[2][2] - v[0][2]);
            if normal_y.abs() < 1e-12 {
                // const-ok: a sliver with no area
                continue;
            }
            if normal_y < 0.0 {
                tri.swap(1, 2);
            }
            let c = [0, 1, 2].map(|i| (v[0][i] + v[1][i] + v[2][i]) / 3.0); // const-ok: centroid
            let node = |q: f64| ((q / CELL_M + top * 0.5).round().clamp(0.0, top)) as usize;
            mesh.material.push(world.splat_at_node(node(c[0]), node(c[2])));
            mesh.triangles.push(tri);
        }
    }
    mesh
}

/// The largest vertical distance from any grid node to the mesh, and how many nodes the mesh covers (all of them, if it has no holes).
pub fn check(world: &GridWorld, mesh: &TerrainMesh) -> (f64, usize) {
    let (n, half) = (world.n() as i64, (world.n() - 1) as f64 * CELL_M * 0.5);
    let (mut worst, mut covered) = (0.0f64, vec![false; (n * n) as usize]);
    for t in &mesh.triangles {
        let v = t.map(|i| mesh.vertices[i as usize]);
        let g = |p: [f64; 3]| ((p[0] + half) / CELL_M, (p[2] + half) / CELL_M);
        let (a, b, c) = (g(v[0]), g(v[1]), g(v[2]));
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        let eps = 1e-9; // const-ok: tolerance on the inside test, nodes on a shared edge belong to both triangles
        for y in
            (a.1.min(b.1).min(c.1).floor().max(0.0) as i64)..=(a.1.max(b.1).max(c.1).ceil().min((n - 1) as f64) as i64)
        {
            for x in (a.0.min(b.0).min(c.0).floor().max(0.0) as i64)
                ..=(a.0.max(b.0).max(c.0).ceil().min((n - 1) as f64) as i64)
            {
                let p = (x as f64, y as f64);
                let w0 = ((b.0 - p.0) * (c.1 - p.1) - (b.1 - p.1) * (c.0 - p.0)) / area;
                let w1 = ((c.0 - p.0) * (a.1 - p.1) - (c.1 - p.1) * (a.0 - p.0)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 >= -eps && w1 >= -eps && w2 >= -eps {
                    covered[(y * n + x) as usize] = true;
                    let h = w0 * v[0][1] + w1 * v[1][1] + w2 * v[2][1];
                    worst = worst.max((h - world.height_at_node(x as usize, y as usize)).abs());
                }
            }
        }
    }
    (worst, covered.iter().filter(|&&c| c).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::world::MaterialTable;
    use w5k_math::scalar;

    const N: usize = 65;

    /// Rolling ground with a stripe of material 1 down the middle.
    fn field(stripe: bool) -> GridWorld {
        let half = (N - 1) as f64 * 0.5;
        let (mut h, mut s) = (vec![0f32; N * N], vec![0u8; N * N]);
        for j in 0..N {
            for i in 0..N {
                let (x, z) = (i as f64 - half, j as f64 - half);
                h[j * N + i] = (3.0 * scalar::sin(x / 9.0) * scalar::cos(z / 7.0)) as f32;
                s[j * N + i] = u8::from(stripe && x.abs() <= 3.0);
            }
        }
        GridWorld::from_arrays(N, h, s, MaterialTable::default())
    }

    #[test]
    fn every_grid_node_is_within_the_tolerance_of_the_mesh_and_the_mesh_has_no_holes() {
        let w = field(false);
        for tol in [0.02, 0.1, 0.5] {
            let (worst, covered) = check(&w, &build(&w, tol));
            assert!(worst <= tol + 1e-6, "tolerance {tol}: worst {worst}");
            assert_eq!(covered, N * N, "tolerance {tol}: every node is under a triangle");
        }
    }

    #[test]
    fn the_triangle_count_falls_as_the_tolerance_rises() {
        let w = field(false);
        let counts: Vec<usize> = [0.01, 0.1, 1.0, 10.0].map(|t| build(&w, t).triangles.len()).to_vec();
        assert!(counts.windows(2).all(|p| p[0] > p[1]), "{counts:?}");
        assert_eq!(counts[3], 2, "a tolerance above the relief is two triangles");
        assert!(counts[0] <= 2 * (N - 1) * (N - 1), "never more than one per half cell");
    }

    #[test]
    fn the_mesh_is_watertight_with_every_face_up_and_its_boundary_on_the_border() {
        let w = field(true);
        let m = build(&w, 0.2);
        let mut edges: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for t in &m.triangles {
            let v = t.map(|i| m.vertices[i as usize]);
            let ny = (v[1][2] - v[0][2]) * (v[2][0] - v[0][0]) - (v[1][0] - v[0][0]) * (v[2][2] - v[0][2]);
            assert!(ny.abs() > 1e-12, "no sliver");
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let half = (N - 1) as f64 * 0.5;
        for ((a, b), count) in edges {
            assert!(count <= 2, "an edge belongs to at most two faces");
            if count == 1 {
                let (p, q) = (m.vertices[a as usize], m.vertices[b as usize]);
                let on = |axis: usize| (p[axis].abs() - half).abs() < 1e-6 && (q[axis].abs() - half).abs() < 1e-6;
                assert!(on(0) || on(2), "a free edge {p:?} {q:?} lies on the border");
            }
        }
    }

    #[test]
    fn a_material_boundary_stays_sharp_even_where_the_ground_is_flat_enough_to_merge() {
        let w = field(true);
        let m = build(&w, 10.0); // far above the relief: only the material forces triangles
        let half = (N - 1) as f64 * 0.5;
        let face_material = |x: f64, z: f64| {
            m.triangles.iter().zip(&m.material).find_map(|(t, &id)| {
                let v = t.map(|i| m.vertices[i as usize]);
                let e = |a: [f64; 3], b: [f64; 3]| (b[0] - a[0]) * (z - a[2]) - (b[2] - a[2]) * (x - a[0]);
                let s = [e(v[0], v[1]), e(v[1], v[2]), e(v[2], v[0])];
                (s.iter().all(|&q| q >= -1e-9) || s.iter().all(|&q| q <= 1e-9)).then_some(id)
            })
        };
        assert_eq!(face_material(0.0, 0.0), Some(1), "the middle of the stripe");
        assert_eq!(face_material(1.0, 10.0), Some(1), "and further along it");
        assert_eq!(face_material(-half + 3.0, 5.0), Some(0), "the ground beside it");
        assert!(m.triangles.len() > 2, "the stripe forced a split");
    }

    #[test]
    fn the_slice_course_meshes_to_a_few_thousand_triangles_within_a_quarter_metre() {
        let def = crate::course::CourseDef::from_ron(include_str!("../../../content/world/courses/slice.ron"))
            .expect("course");
        let course = crate::course::generate(&def).expect("generate");
        let m = build(&course.world, 0.25);
        let (worst, covered) = check(&course.world, &m);
        let n = course.world.n();
        assert!(worst <= 0.25 + 1e-6 && covered == n * n, "worst {worst}, covered {covered} of {}", n * n);
        assert!(
            m.triangles.len() < 2 * (n - 1) * (n - 1) / 10,
            "{} triangles of {}",
            m.triangles.len(),
            2 * (n - 1) * (n - 1)
        );
        assert!(m.material.contains(&(course.road_material.0 as u8)), "the road survives the decimation");
    }
}
