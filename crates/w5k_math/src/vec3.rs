//! A 3-vector of `f64`. Frame: +X right, +Y up, -Z forward.

use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

use crate::scalar;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const X: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };
    /// Right, up and forward in the world and body frames.
    pub const RIGHT: Vec3 = Vec3::X;
    pub const UP: Vec3 = Vec3::Y;
    pub const FORWARD: Vec3 = Vec3 { x: 0.0, y: 0.0, z: -1.0 };

    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn splat(v: f64) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.y * o.z - self.z * o.y, y: self.z * o.x - self.x * o.z, z: self.x * o.y - self.y * o.x }
    }

    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        scalar::sqrt(self.length_sq())
    }

    /// The unit vector, or `None` when the length is below `eps` (never divides by ~0).
    pub fn try_normalize(self, eps: f64) -> Option<Vec3> {
        let l = self.length();
        if l > eps {
            Some(self * (1.0 / l))
        } else {
            None
        }
    }

    /// The unit vector, or zero for a (near) zero input.
    pub fn normalized_or_zero(self) -> Vec3 {
        self.try_normalize(1e-300).unwrap_or(Vec3::ZERO)
    }

    pub fn lerp(self, o: Vec3, t: f64) -> Vec3 {
        self + (o - self) * t
    }

    pub fn abs(self) -> Vec3 {
        Vec3 { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }

    pub fn min(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x.min(o.x), y: self.y.min(o.y), z: self.z.min(o.z) }
    }

    pub fn max(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x.max(o.x), y: self.y.max(o.y), z: self.z.max(o.z) }
    }

    /// Component-wise product.
    pub fn mul_elem(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x * o.x, y: self.y * o.y, z: self.z * o.z }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// The part of `self` along the unit vector `n`.
    pub fn project_on(self, n: Vec3) -> Vec3 {
        n * self.dot(n)
    }

    /// The part of `self` perpendicular to the unit vector `n`.
    pub fn reject_from(self, n: Vec3) -> Vec3 {
        self - self.project_on(n)
    }

    pub fn as_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z }
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z }
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3 { x: -self.x, y: -self.y, z: -self.z }
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
}
impl Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}
impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f64) -> Vec3 {
        Vec3 { x: self.x / s, y: self.y / s, z: self.z / s }
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}
impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}
impl MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, s: f64) {
        *self = *self * s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_of_right_and_up_is_back_in_a_right_handed_frame() {
        // +X right, +Y up, forward = -Z, so right x up = +Z = backward.
        assert_eq!(Vec3::RIGHT.cross(Vec3::UP), Vec3::Z);
        assert_eq!(Vec3::FORWARD, -Vec3::Z);
    }

    #[test]
    fn dot_length_and_normalise() {
        let v = Vec3::new(3.0, 0.0, 4.0);
        assert!(scalar::approx_eq(v.length(), 5.0, 1e-15));
        assert!(scalar::approx_eq(v.normalized_or_zero().length(), 1.0, 1e-15));
        assert_eq!(Vec3::ZERO.normalized_or_zero(), Vec3::ZERO);
        assert!(Vec3::ZERO.try_normalize(1e-9).is_none());
    }

    #[test]
    fn project_and_reject_split_a_vector() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let n = Vec3::Y;
        let sum = v.project_on(n) + v.reject_from(n);
        assert!(scalar::approx_eq((sum - v).length(), 0.0, 1e-15));
        assert!(scalar::approx_eq(v.reject_from(n).dot(n), 0.0, 1e-15));
    }
}
