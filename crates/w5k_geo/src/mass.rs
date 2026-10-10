//! Exact mass properties of a closed mesh (divergence theorem): unit-density volume, centroid and inertia, with the algebra FORGE needs
//! to compose parts (translate, rotate, scale, sum by the parallel-axis theorem).

use crate::mesh::Mesh;
use w5k_math::{Mat3, Vec3};

/// Density 1: multiply by a density in kg/m^3 for kg and kg m^2 (`at_density`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassProps {
    pub volume_m3: f64,
    pub centroid_m: Vec3,
    /// Inertia tensor about the centroid, in the mesh frame axes, for density 1 (m^5).
    pub inertia_m5: Mat3,
}

impl Mesh {
    /// Sum over signed tetrahedra (apex at the origin). Second moments: for a tetrahedron with one vertex at the origin,
    /// integral of x x^T dV = V/20 (sum v_i v_i^T + S S^T), S = sum v_i; inertia = trace(C) 1 - C.
    pub fn mass_props(&self) -> MassProps {
        let (mut vol, mut first, mut second) = (0.0, Vec3::ZERO, Mat3::ZERO);
        for k in 0..self.t.len() {
            let [a, b, c] = self.tri(k);
            let v = a.dot(b.cross(c)) / 6.0; // const-ok: a tetrahedron is a sixth of the parallelepiped
            let s = a + b + c;
            vol += v;
            first += s * (v / 4.0); // const-ok: the centroid of a tetrahedron with an apex at the origin is S / 4
            let mut c2 = Mat3::outer(s, s);
            for p in [a, b, c] {
                c2 = c2.add(&Mat3::outer(p, p));
            }
            second = second.add(&c2.scaled(v / 20.0)); // const-ok: the tetrahedron second-moment factor
        }
        let centroid = first * (1.0 / vol);
        let cov = second.add(&Mat3::outer(centroid, centroid).scaled(-vol)); // about the centroid
        let tr = cov.m[0][0] + cov.m[1][1] + cov.m[2][2];
        MassProps {
            volume_m3: vol,
            centroid_m: centroid,
            inertia_m5: Mat3::diagonal(tr, tr, tr).add(&cov.scaled(-1.0)),
        }
    }
}

impl MassProps {
    pub fn translate(&self, d: Vec3) -> MassProps {
        MassProps { centroid_m: self.centroid_m + d, ..*self }
    }

    pub fn rotate(&self, r: &Mat3) -> MassProps {
        MassProps {
            volume_m3: self.volume_m3,
            centroid_m: r.mul_vec(self.centroid_m),
            inertia_m5: self.inertia_m5.rotate_inertia(r),
        }
    }

    /// Uniform scale: volume s^3, centroid s, inertia s^5.
    pub fn scale(&self, s: f64) -> MassProps {
        MassProps {
            volume_m3: self.volume_m3 * s * s * s,
            centroid_m: self.centroid_m * s,
            inertia_m5: self.inertia_m5.scaled(w5k_math::scalar::powi(s, 5)),
        }
    }

    /// Parts combined into one body: centroids by volume weight, inertias by the parallel-axis theorem.
    pub fn sum(parts: &[MassProps]) -> MassProps {
        let vol: f64 = parts.iter().map(|p| p.volume_m3).sum();
        let centroid = parts.iter().fold(Vec3::ZERO, |a, p| a + p.centroid_m * p.volume_m3) * (1.0 / vol);
        let mut inertia = Mat3::ZERO;
        for p in parts {
            let d = p.centroid_m - centroid;
            let shift =
                Mat3::diagonal(d.length_sq(), d.length_sq(), d.length_sq()).add(&Mat3::outer(d, d).scaled(-1.0));
            inertia = inertia.add(&p.inertia_m5.add(&shift.scaled(p.volume_m3)));
        }
        MassProps { volume_m3: vol, centroid_m: centroid, inertia_m5: inertia }
    }

    /// Mass in kg and inertia about the centroid in kg m^2 at a uniform density.
    pub fn at_density(&self, kg_m3: f64) -> (f64, Mat3) {
        (self.volume_m3 * kg_m3, self.inertia_m5.scaled(kg_m3))
    }
}
