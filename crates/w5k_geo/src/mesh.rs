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
