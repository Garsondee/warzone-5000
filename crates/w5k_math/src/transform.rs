//! A rigid transform: rotation then translation. Maps *local* coordinates to the *parent* (or world) frame.

use serde::{Deserialize, Serialize};

use crate::quat::Quat;
use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
}

impl Transform {
    pub const IDENTITY: Transform = Transform { pos: Vec3::ZERO, rot: Quat::IDENTITY };

    pub const fn new(pos: Vec3, rot: Quat) -> Transform {
        Transform { pos, rot }
    }

    pub fn from_pos(pos: Vec3) -> Transform {
        Transform { pos, rot: Quat::IDENTITY }
    }

    /// A local point in the parent frame.
    pub fn apply_point(&self, p: Vec3) -> Vec3 {
        self.pos + self.rot.rotate(p)
    }

    /// A local direction (or velocity) in the parent frame.
    pub fn apply_dir(&self, d: Vec3) -> Vec3 {
        self.rot.rotate(d)
    }

    pub fn inverse(&self) -> Transform {
        let ri = self.rot.conjugate();
        Transform { pos: -ri.rotate(self.pos), rot: ri }
    }

    /// `self * o`: first `o`, then `self` (so `o` is expressed in `self`'s local frame).
    pub fn compose(&self, o: &Transform) -> Transform {
        Transform { pos: self.apply_point(o.pos), rot: (self.rot * o.rot).normalized() }
    }

    pub fn is_finite(&self) -> bool {
        self.pos.is_finite() && self.rot.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_undoes_the_transform() {
        let t = Transform::new(Vec3::new(1.0, 2.0, -3.0), Quat::from_ypr(0.5, 0.1, -0.2));
        let p = Vec3::new(0.3, -0.4, 5.0);
        assert!((t.inverse().apply_point(t.apply_point(p)) - p).length() < 1e-12);
    }

    #[test]
    fn compose_matches_applying_one_after_the_other() {
        let a = Transform::new(Vec3::new(1.0, 0.0, 0.0), Quat::from_yaw(0.6));
        let b = Transform::new(Vec3::new(0.0, 2.0, 1.0), Quat::from_pitch(0.3));
        let p = Vec3::new(0.5, 0.5, 0.5);
        assert!((a.compose(&b).apply_point(p) - a.apply_point(b.apply_point(p))).length() < 1e-12);
    }
}
