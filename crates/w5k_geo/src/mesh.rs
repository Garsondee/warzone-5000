//! Indexed triangle mesh in f64 (f32 only once, at export). Counter-clockwise triangles face outward.

use w5k_math::Vec3;

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub v: Vec<Vec3>,
    pub t: Vec<[u32; 3]>,
}

impl Mesh {
    pub fn tri(&self, k: usize) -> [Vec3; 3] {
        let [a, b, c] = self.t[k];
        [self.v[a as usize], self.v[b as usize], self.v[c as usize]]
    }

    /// Unit normal of triangle `k` (zero for a degenerate one).
    pub fn face_normal(&self, k: usize) -> Vec3 {
        let [a, b, c] = self.tri(k);
        (b - a).cross(c - a).normalized_or_zero()
    }

    /// Signed volume by the divergence theorem: the sum of signed tetrahedra against the origin. Positive when outward-facing.
    pub fn signed_volume(&self) -> f64 {
        (0..self.t.len())
            .map(|k| {
                let [a, b, c] = self.tri(k);
                a.dot(b.cross(c))
            })
            .sum::<f64>()
            / 6.0 // const-ok: a tetrahedron is a sixth of the parallelepiped
    }

    /// Area-weighted vertex normals.
    pub fn vertex_normals(&self) -> Vec<Vec3> {
        let mut n = vec![Vec3::ZERO; self.v.len()];
        for (k, idx) in self.t.iter().enumerate() {
            let [a, b, c] = self.tri(k);
            let w = (b - a).cross(c - a); // length = 2 x area
            for &i in idx {
                n[i as usize] += w;
            }
        }
        n.into_iter().map(Vec3::normalized_or_zero).collect()
    }

    /// Prism from a convex-or-fan-visible profile in the XZ plane (the order of `rect` in the tests: -x -z, -x +z, +x +z, +x -z), from y = 0 to `h`. Caps are fans
    /// from profile vertex `fan` (a vertex that sees the whole profile: any for a convex one, the inner corner for an L).
    pub fn extrude_fan(profile: &[[f64; 2]], fan: usize, h: f64) -> Mesh {
        let n = profile.len() as u32;
        let mut m = Mesh::default();
        for &[x, z] in profile {
            m.v.push(Vec3::new(x, 0.0, z));
        }
        for &[x, z] in profile {
            m.v.push(Vec3::new(x, h, z));
        }
        for i in 0..n {
            let j = (i + 1) % n;
            m.t.push([i, j, n + j]);
            m.t.push([i, n + j, n + i]);
        }
        for i in 0..n {
            let j = (i + 1) % n;
            if i != fan as u32 && j != fan as u32 {
                m.t.push([n + fan as u32, n + i, n + j]); // top faces +Y
                m.t.push([fan as u32, j, i]); // bottom faces -Y
            }
        }
        m
    }
}

impl Mesh {
    /// Merge vertices that round to the same `tol_m` cell (generated parts repeat the same expression, so equal points are bit-equal
    /// and this is exact for them), then drop triangles that collapse or whose area is under `min_area_m2`. Returns the number dropped.
    pub fn weld(&mut self, tol_m: f64, min_area_m2: f64) -> usize {
        let mut cell: std::collections::BTreeMap<[i64; 3], u32> = std::collections::BTreeMap::new();
        let mut verts = Vec::new();
        let remap: Vec<u32> = self
            .v
            .iter()
            .map(|p| {
                let key = [(p.x / tol_m).round() as i64, (p.y / tol_m).round() as i64, (p.z / tol_m).round() as i64];
                *cell.entry(key).or_insert_with(|| {
                    verts.push(*p);
                    verts.len() as u32 - 1
                })
            })
            .collect();
        self.v = verts;
        let before = self.t.len();
        self.t = self.t.iter().map(|t| t.map(|i| remap[i as usize])).collect();
        let v = &self.v;
        self.t.retain(|&[a, b, c]| {
            a != b
                && b != c
                && c != a
                && (v[b as usize] - v[a as usize]).cross(v[c as usize] - v[a as usize]).length() / 2.0 >= min_area_m2
        });
        before - self.t.len()
    }

    /// Closed and consistently oriented: every directed edge occurs once and its reverse occurs once.
    pub fn check_closed(&self) -> Result<(), String> {
        let mut dir: std::collections::BTreeMap<(u32, u32), u32> = std::collections::BTreeMap::new();
        for &[a, b, c] in &self.t {
            for e in [(a, b), (b, c), (c, a)] {
                *dir.entry(e).or_default() += 1;
            }
        }
        for (&(p, q), &n) in &dir {
            if n != 1 || dir.get(&(q, p)) != Some(&1) {
                return Err(format!(
                    "edge {p}-{q} is not shared by exactly two triangles traversed in opposite directions"
                ));
            }
        }
        Ok(())
    }

    /// Flip every triangle if the signed volume is negative (valid for a closed, consistently oriented mesh).
    pub fn orient_outward(&mut self) {
        if self.signed_volume() < 0.0 {
            for t in &mut self.t {
                t.swap(1, 2);
            }
        }
    }

    pub fn append(&mut self, o: &Mesh) {
        let base = self.v.len() as u32;
        self.v.extend_from_slice(&o.v);
        self.t.extend(o.t.iter().map(|t| t.map(|i| i + base)));
    }

    pub fn transformed(&self, x: &w5k_math::Transform) -> Mesh {
        Mesh { v: self.v.iter().map(|&p| x.apply_point(p)).collect(), t: self.t.clone() }
    }

    /// Mirror in the plane x = 0: negates x and reverses the winding so the result still faces outward.
    pub fn mirrored_x(&self) -> Mesh {
        Mesh {
            v: self.v.iter().map(|p| Vec3::new(-p.x, p.y, p.z)).collect(),
            t: self.t.iter().map(|&[a, b, c]| [a, c, b]).collect(),
        }
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.v.iter().fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(lo, hi), &p| (lo.min(p), hi.max(p)))
    }

    /// Area of the cross-section at height `z` (the mesh's z axis) by clipping every triangle against the plane: the 2D divergence theorem
    /// over the cut segments. A vertex exactly on the plane counts as above it, so a station cut returns the section that was lofted.
    pub fn slice_area_z(&self, z: f64) -> f64 {
        let mut twice = 0.0;
        for k in 0..self.t.len() {
            let p = self.tri(k);
            let above = p.map(|v| v.z >= z);
            let Some(i) = (0..3).find(|&i| above[i] != above[(i + 1) % 3] && above[i] != above[(i + 2) % 3]) else {
                continue;
            };
            let cut = |a: Vec3, b: Vec3| a + (b - a) * ((z - a.z) / (b.z - a.z));
            let (a, b) = (cut(p[i], p[(i + 1) % 3]), cut(p[(i + 2) % 3], p[i]));
            let n = self.face_normal(k);
            // outward in the plane is the segment direction turned by -90 degrees: pick the direction that points that way
            let d = (b.x - a.x, b.y - a.y);
            let (s, e) = if d.1 * n.x - d.0 * n.y > 0.0 { (a, b) } else { (b, a) };
            twice += s.x * e.y - e.x * s.y;
        }
        twice / 2.0
    }
}
