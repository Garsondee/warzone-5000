//! Small f64 geometry toolkit for offline content building (not used by the deterministic simulation).

use core::ops::{Add, AddAssign, Div, Index, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub fn from_arr(a: [f64; 3]) -> V3 {
        v3(a[0], a[1], a[2])
    }
    pub fn arr(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    pub fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 0.0 { self / l } else { V3::ZERO }
    }
    pub fn min(self, o: V3) -> V3 {
        v3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }
    pub fn max(self, o: V3) -> V3 {
        v3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
    pub fn mul_elem(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    /// Any unit vector perpendicular to `self` (which must be non-zero).
    pub fn any_perp(self) -> V3 {
        let a = if self.x.abs() < 0.9 { v3(1.0, 0.0, 0.0) } else { v3(0.0, 1.0, 0.0) };
        self.cross(a).norm()
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl AddAssign for V3 {
    fn add_assign(&mut self, o: V3) {
        *self = *self + o;
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl Mul<f64> for V3 {
    type Output = V3;
    fn mul(self, s: f64) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f64> for V3 {
    type Output = V3;
    fn div(self, s: f64) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Index<usize> for V3 {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            _ => &self.z,
        }
    }
}

/// 3x3 matrix, row-major. Used for rotations and reflections.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M3(pub [[f64; 3]; 3]);

impl M3 {
    pub const IDENTITY: M3 = M3([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    pub fn from_cols(a: V3, b: V3, c: V3) -> M3 {
        M3([[a.x, b.x, c.x], [a.y, b.y, c.y], [a.z, b.z, c.z]])
    }

    pub fn scale(s: V3) -> M3 {
        M3([[s.x, 0.0, 0.0], [0.0, s.y, 0.0], [0.0, 0.0, s.z]])
    }

    /// Rotation about a unit axis by `deg` degrees (right-handed).
    pub fn axis_angle(axis: V3, deg: f64) -> M3 {
        let (s, c) = deg.to_radians().sin_cos();
        let t = 1.0 - c;
        let V3 { x, y, z } = axis.norm();
        M3([
            [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
            [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
            [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
        ])
    }

    /// Euler angles in degrees, applied X then Y then Z (R = Rz * Ry * Rx).
    pub fn euler_deg(r: [f64; 3]) -> M3 {
        M3::axis_angle(v3(0.0, 0.0, 1.0), r[2]) * M3::axis_angle(v3(0.0, 1.0, 0.0), r[1]) * M3::axis_angle(v3(1.0, 0.0, 0.0), r[0])
    }

    pub fn transpose(self) -> M3 {
        let m = self.0;
        M3([[m[0][0], m[1][0], m[2][0]], [m[0][1], m[1][1], m[2][1]], [m[0][2], m[1][2], m[2][2]]])
    }

    pub fn det(self) -> f64 {
        let m = self.0;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Inverse (the matrix must be invertible).
    pub fn inverse(self) -> M3 {
        let m = self.0;
        let d = self.det();
        let inv = [
            [m[1][1] * m[2][2] - m[1][2] * m[2][1], m[0][2] * m[2][1] - m[0][1] * m[2][2], m[0][1] * m[1][2] - m[0][2] * m[1][1]],
            [m[1][2] * m[2][0] - m[1][0] * m[2][2], m[0][0] * m[2][2] - m[0][2] * m[2][0], m[0][2] * m[1][0] - m[0][0] * m[1][2]],
            [m[1][0] * m[2][1] - m[1][1] * m[2][0], m[0][1] * m[2][0] - m[0][0] * m[2][1], m[0][0] * m[1][1] - m[0][1] * m[1][0]],
        ];
        let mut r = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = inv[i][j] / d;
            }
        }
        M3(r)
    }
}

impl Mul for M3 {
    type Output = M3;
    fn mul(self, o: M3) -> M3 {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[i][k] * o.0[k][j]).sum();
            }
        }
        M3(r)
    }
}

impl Mul<V3> for M3 {
    type Output = V3;
    fn mul(self, v: V3) -> V3 {
        let m = self.0;
        v3(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }
}

/// An affine transform: `p -> m * p + t`. `m` may include reflection and uniform scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xform {
    pub m: M3,
    pub t: V3,
}

impl Xform {
    pub const IDENTITY: Xform = Xform { m: M3::IDENTITY, t: V3::ZERO };

    pub fn new(m: M3, t: V3) -> Xform {
        Xform { m, t }
    }

    pub fn point(&self, p: V3) -> V3 {
        self.m * p + self.t
    }

    pub fn dir(&self, d: V3) -> V3 {
        self.m * d
    }

    /// `self` applied after `inner`.
    pub fn compose(&self, inner: &Xform) -> Xform {
        Xform { m: self.m * inner.m, t: self.m * inner.t + self.t }
    }

    pub fn is_mirrored(&self) -> bool {
        self.m.det() < 0.0
    }
}

/// Half-space `dot(n, x) <= d` with unit normal `n` pointing outward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub n: V3,
    pub d: f64,
}

impl Plane {
    pub fn from_points(a: V3, b: V3, c: V3) -> Option<Plane> {
        let n = (b - a).cross(c - a);
        let l = n.len();
        if l < 1e-12 {
            return None;
        }
        let n = n / l;
        Some(Plane { n, d: n.dot(a) })
    }

    pub fn dist(&self, p: V3) -> f64 {
        self.n.dot(p) - self.d
    }

    /// Transform a plane by an affine map (handles rotation, reflection and uniform scale).
    pub fn transformed(&self, x: &Xform) -> Plane {
        // Normals transform by the inverse transpose.
        let n = (x.m.inverse().transpose() * self.n).norm();
        let p0 = x.point(self.n * self.d);
        Plane { n, d: n.dot(p0) }
    }
}

/// Solve the intersection point of three planes, if they meet in a single point.
pub fn intersect3(a: &Plane, b: &Plane, c: &Plane) -> Option<V3> {
    let n1 = a.n;
    let n2 = b.n;
    let n3 = c.n;
    let det = n1.dot(n2.cross(n3));
    if det.abs() < 1e-10 {
        return None;
    }
    Some((n2.cross(n3) * a.d + n3.cross(n1) * b.d + n1.cross(n2) * c.d) / det)
}
