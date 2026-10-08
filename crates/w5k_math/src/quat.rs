//! Unit quaternions, stored w-x-y-z. A quaternion maps *body* coordinates to *world* coordinates.
//!
//! Sign conventions (right-handed, +Y up, -Z forward, +X right), pinned by the tests below:
//! * **yaw** is a rotation about +Y; positive yaw turns the nose to the **left** (counter-clockwise seen from above);
//! * **pitch** is a rotation about +X; positive pitch raises the nose;
//! * **roll** is a rotation about the forward axis (-Z); positive roll lowers the **right** side.
//!
//! `from_ypr` composes them intrinsically in the order yaw, pitch, roll (the order a driver would describe a pose: heading,
//! then slope, then lean), and `to_ypr` inverts it.

use serde::{Deserialize, Serialize};

use crate::mat3::Mat3;
use crate::scalar;
use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Default for Quat {
    fn default() -> Quat {
        Quat::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Quat = Quat { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(w: f64, x: f64, y: f64, z: f64) -> Quat {
        Quat { w, x, y, z }
    }

    /// Rotation by `angle` radians about the (unit) `axis`, right-handed.
    pub fn from_axis_angle(axis: Vec3, angle: f64) -> Quat {
        let a = axis.normalized_or_zero();
        let (s, c) = scalar::sin_cos(0.5 * angle);
        Quat { w: c, x: a.x * s, y: a.y * s, z: a.z * s }
    }

    /// Rotation about +Y; positive turns the nose left.
    pub fn from_yaw(angle: f64) -> Quat {
        Quat::from_axis_angle(Vec3::Y, angle)
    }

    /// Rotation about +X; positive raises the nose.
    pub fn from_pitch(angle: f64) -> Quat {
        Quat::from_axis_angle(Vec3::X, angle)
    }

    /// Rotation about the forward axis (-Z); positive lowers the right side.
    pub fn from_roll(angle: f64) -> Quat {
        Quat::from_axis_angle(Vec3::FORWARD, angle)
    }

    /// Yaw, then pitch, then roll (intrinsic): `q = yaw * pitch * roll`.
    pub fn from_ypr(yaw: f64, pitch: f64, roll: f64) -> Quat {
        Quat::from_yaw(yaw) * Quat::from_pitch(pitch) * Quat::from_roll(roll)
    }

    /// The `(yaw, pitch, roll)` of this orientation (inverse of `from_ypr`; pitch in `[-pi/2, pi/2]`).
    pub fn to_ypr(self) -> (f64, f64, f64) {
        let f = self.rotate(Vec3::FORWARD);
        let pitch = scalar::asin(scalar::clamp(f.y, -1.0, 1.0));
        let yaw = scalar::atan2(-f.x, -f.z);
        // The roll-free frame for this yaw and pitch.
        let q0 = Quat::from_yaw(yaw) * Quat::from_pitch(pitch);
        let r0 = q0.rotate(Vec3::RIGHT);
        let u0 = q0.rotate(Vec3::UP);
        let up = self.rotate(Vec3::UP);
        let roll = scalar::atan2(up.dot(r0), up.dot(u0));
        (yaw, pitch, roll)
    }

    pub fn conjugate(self) -> Quat {
        Quat { w: self.w, x: -self.x, y: -self.y, z: -self.z }
    }

    pub fn dot(self, o: Quat) -> f64 {
        self.w * o.w + self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn length(self) -> f64 {
        scalar::sqrt(self.dot(self))
    }

    /// Unit length again (identity for a degenerate input).
    pub fn normalized(self) -> Quat {
        let l = self.length();
        if l > 1e-300 {
            let k = 1.0 / l;
            Quat { w: self.w * k, x: self.x * k, y: self.y * k, z: self.z * k }
        } else {
            Quat::IDENTITY
        }
    }

    /// Rotate a vector by this (unit) quaternion.
    pub fn rotate(self, v: Vec3) -> Vec3 {
        // v' = v + 2 w (u x v) + 2 u x (u x v), with u the vector part.
        let u = Vec3::new(self.x, self.y, self.z);
        let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }

    /// Rotate by the inverse (world vector to body vector).
    pub fn inverse_rotate(self, v: Vec3) -> Vec3 {
        self.conjugate().rotate(v)
    }

    pub fn to_mat3(self) -> Mat3 {
        let Quat { w, x, y, z } = self;
        Mat3 {
            m: [
                [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y)],
                [2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x)],
                [2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y)],
            ],
        }
    }

    /// Advance the orientation by the world-frame angular velocity `omega` (rad/s) over `dt` (s), using the exact exponential map
    /// of the rotation vector `omega * dt` (so a constant spin is integrated without drift). Returns a unit quaternion.
    pub fn integrate_world(self, omega: Vec3, dt: f64) -> Quat {
        let rv = omega * dt;
        let angle = rv.length();
        if angle < 1e-300 {
            return self;
        }
        (Quat::from_axis_angle(rv * (1.0 / angle), angle) * self).normalized()
    }

    /// The smallest rotation angle (rad) between two orientations.
    pub fn angle_to(self, o: Quat) -> f64 {
        let d = scalar::clamp(self.dot(o).abs(), 0.0, 1.0);
        2.0 * scalar::acos(d)
    }

    /// Spherical interpolation along the shortest arc.
    pub fn slerp(self, o: Quat, t: f64) -> Quat {
        let mut d = self.dot(o);
        let mut b = o;
        if d < 0.0 {
            d = -d;
            b = Quat { w: -o.w, x: -o.x, y: -o.y, z: -o.z };
        }
        if d > 0.9995 {
            // Nearly parallel: a normalised lerp is accurate and avoids dividing by sin(~0).
            return Quat {
                w: scalar::lerp(self.w, b.w, t),
                x: scalar::lerp(self.x, b.x, t),
                y: scalar::lerp(self.y, b.y, t),
                z: scalar::lerp(self.z, b.z, t),
            }
            .normalized();
        }
        let theta = scalar::acos(d);
        let s = scalar::sin(theta);
        let ka = scalar::sin((1.0 - t) * theta) / s;
        let kb = scalar::sin(t * theta) / s;
        Quat {
            w: ka * self.w + kb * b.w,
            x: ka * self.x + kb * b.x,
            y: ka * self.y + kb * b.y,
            z: ka * self.z + kb * b.z,
        }
    }

    pub fn is_finite(self) -> bool {
        self.w.is_finite() && self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl core::ops::Mul for Quat {
    type Output = Quat;
    /// Hamilton product: `(a * b).rotate(v) == a.rotate(b.rotate(v))`.
    fn mul(self, o: Quat) -> Quat {
        Quat {
            w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scalar::{approx_eq, PI};

    fn near(a: Vec3, b: Vec3, tol: f64) -> bool {
        (a - b).length() <= tol
    }

    #[test]
    fn positive_yaw_turns_the_nose_to_the_left() {
        let q = Quat::from_yaw(PI / 2.0);
        assert!(near(q.rotate(Vec3::FORWARD), Vec3::new(-1.0, 0.0, 0.0), 1e-12), "forward must go to -X (left)");
    }

    #[test]
    fn positive_pitch_raises_the_nose() {
        let q = Quat::from_pitch(PI / 6.0);
        let f = q.rotate(Vec3::FORWARD);
        assert!(f.y > 0.0 && approx_eq(f.y, 0.5, 1e-12));
    }

    #[test]
    fn positive_roll_lowers_the_right_side() {
        let q = Quat::from_roll(PI / 6.0);
        let r = q.rotate(Vec3::RIGHT);
        assert!(r.y < 0.0 && approx_eq(r.y, -0.5, 1e-12));
    }

    #[test]
    fn ypr_round_trips() {
        for &(y, p, r) in &[(0.3, 0.2, -0.1), (-2.0, -0.7, 0.4), (3.0, 0.0, 0.0), (0.0, 0.0, 1.2), (-0.5, 1.2, -1.0)] {
            let q = Quat::from_ypr(y, p, r);
            let (y2, p2, r2) = q.to_ypr();
            assert!(approx_eq(scalar::wrap_pi(y2 - y), 0.0, 1e-9), "yaw {y} -> {y2}");
            assert!(approx_eq(p2, p, 1e-9), "pitch {p} -> {p2}");
            assert!(approx_eq(scalar::wrap_pi(r2 - r), 0.0, 1e-9), "roll {r} -> {r2}");
        }
    }

    #[test]
    fn product_composes_rotations_in_order() {
        let a = Quat::from_yaw(0.7);
        let b = Quat::from_pitch(-0.3);
        let v = Vec3::new(0.2, -1.0, 0.5);
        assert!(near((a * b).rotate(v), a.rotate(b.rotate(v)), 1e-12));
    }

    #[test]
    fn matrix_and_quaternion_agree() {
        let q = Quat::from_ypr(0.4, -0.2, 0.9);
        let v = Vec3::new(1.0, 2.0, -3.0);
        assert!(near(q.to_mat3().mul_vec(v), q.rotate(v), 1e-12));
    }

    #[test]
    fn constant_spin_integrates_without_drift() {
        // 1 rad/s about +Y for 10 s in 6000 steps of 1/600 s must give exactly 10 rad of yaw (mod 2 pi).
        let omega = Vec3::new(0.0, 1.0, 0.0);
        let mut q = Quat::IDENTITY;
        for _ in 0..6000 {
            q = q.integrate_world(omega, 1.0 / 600.0);
        }
        assert!(approx_eq(q.length(), 1.0, 1e-12));
        assert!(q.angle_to(Quat::from_yaw(10.0)) < 1e-9);
    }

    #[test]
    fn slerp_hits_the_ends_and_the_middle() {
        let a = Quat::from_yaw(0.0);
        let b = Quat::from_yaw(1.0);
        assert!(a.slerp(b, 0.0).angle_to(a) < 1e-12);
        assert!(a.slerp(b, 1.0).angle_to(b) < 1e-12);
        assert!(a.slerp(b, 0.5).angle_to(Quat::from_yaw(0.5)) < 1e-12);
    }
}
