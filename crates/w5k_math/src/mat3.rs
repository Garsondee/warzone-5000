//! A 3x3 matrix, row-major: `m[row][col]`. Used for inertia tensors and rotations.

use serde::{Deserialize, Serialize};

use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mat3 {
    pub m: [[f64; 3]; 3],
}

impl Default for Mat3 {
    fn default() -> Mat3 {
        Mat3::IDENTITY
    }
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 { m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] };
    pub const ZERO: Mat3 = Mat3 { m: [[0.0; 3]; 3] };

    pub fn diagonal(x: f64, y: f64, z: f64) -> Mat3 {
        Mat3 { m: [[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, z]] }
    }

    /// The matrix of `v x (.)`, so `skew(a).mul_vec(b) == a.cross(b)`.
    pub fn skew(v: Vec3) -> Mat3 {
        Mat3 { m: [[0.0, -v.z, v.y], [v.z, 0.0, -v.x], [-v.y, v.x, 0.0]] }
    }

    /// The outer product `a b^T`.
    pub fn outer(a: Vec3, b: Vec3) -> Mat3 {
        let (a, b) = (a.as_array(), b.as_array());
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, e) in row.iter_mut().enumerate() {
                *e = a[i] * b[j];
            }
        }
        Mat3 { m }
    }

    pub fn mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3 {
            x: m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            y: m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            z: m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        }
    }

    pub fn mul_mat(&self, o: &Mat3) -> Mat3 {
        let mut m = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, e) in row.iter_mut().enumerate() {
                *e = self.m[i][0] * o.m[0][j] + self.m[i][1] * o.m[1][j] + self.m[i][2] * o.m[2][j];
            }
        }
        Mat3 { m }
    }

    pub fn transpose(&self) -> Mat3 {
        let m = &self.m;
        Mat3 { m: [[m[0][0], m[1][0], m[2][0]], [m[0][1], m[1][1], m[2][1]], [m[0][2], m[1][2], m[2][2]]] }
    }

    pub fn scaled(&self, s: f64) -> Mat3 {
        let mut m = self.m;
        for row in m.iter_mut() {
            for e in row.iter_mut() {
                *e *= s;
            }
        }
        Mat3 { m }
    }

    pub fn add(&self, o: &Mat3) -> Mat3 {
        let mut m = self.m;
        for (i, row) in m.iter_mut().enumerate() {
            for (j, e) in row.iter_mut().enumerate() {
                *e += o.m[i][j];
            }
        }
        Mat3 { m }
    }

    pub fn det(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// The inverse, or `None` when the determinant is (near) zero relative to the matrix scale.
    pub fn inverse(&self) -> Option<Mat3> {
        let m = &self.m;
        let det = self.det();
        let scale = m.iter().flatten().fold(0.0f64, |a, &e| a.max(e.abs()));
        if det.abs() <= 1e-14 * scale * scale * scale {
            return None;
        }
        let k = 1.0 / det;
        let c = |a: usize, b: usize, c2: usize, d: usize| m[a][b] * m[c2][d] - m[a][d] * m[c2][b];
        Some(Mat3 {
            m: [
                [c(1, 1, 2, 2) * k, -c(0, 1, 2, 2) * k, c(0, 1, 1, 2) * k],
                [-c(1, 0, 2, 2) * k, c(0, 0, 2, 2) * k, -c(0, 0, 1, 2) * k],
                [c(1, 0, 2, 1) * k, -c(0, 0, 2, 1) * k, c(0, 0, 1, 1) * k],
            ],
        })
    }

    /// `R I R^T`: an inertia tensor given in a rotated frame, re-expressed in the other frame.
    pub fn rotate_inertia(&self, rot: &Mat3) -> Mat3 {
        rot.mul_mat(self).mul_mat(&rot.transpose())
    }

    pub fn is_finite(&self) -> bool {
        self.m.iter().flatten().all(|e| e.is_finite())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quat::Quat;
    use crate::scalar::approx_eq;

    #[test]
    fn inverse_times_matrix_is_identity() {
        let a = Mat3 { m: [[4.0, 1.0, 0.5], [1.0, 3.0, 0.25], [0.5, 0.25, 2.0]] };
        let i = a.inverse().expect("invertible").mul_mat(&a);
        for r in 0..3 {
            for c in 0..3 {
                let want = if r == c { 1.0 } else { 0.0 };
                assert!(approx_eq(i.m[r][c], want, 1e-12));
            }
        }
        assert!(Mat3::ZERO.inverse().is_none());
    }

    #[test]
    fn skew_is_the_cross_product() {
        let a = Vec3::new(1.0, -2.0, 0.5);
        let b = Vec3::new(0.3, 0.9, -1.1);
        assert!((Mat3::skew(a).mul_vec(b) - a.cross(b)).length() < 1e-15);
    }

    #[test]
    fn rotated_inertia_keeps_the_trace_and_the_determinant() {
        let i = Mat3::diagonal(10.0, 20.0, 30.0);
        let r = Quat::from_ypr(0.4, 0.2, -0.7).to_mat3();
        let j = i.rotate_inertia(&r);
        let tr = |m: &Mat3| m.m[0][0] + m.m[1][1] + m.m[2][2];
        assert!(approx_eq(tr(&j), 60.0, 1e-12));
        assert!(approx_eq(j.det(), i.det(), 1e-8));
    }
}
