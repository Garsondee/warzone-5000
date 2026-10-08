//! Fixed-point 3D vectors.

use crate::fx::{isqrt_u128, Fx};
use core::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// A 3D vector of `Fx`. Coordinates are metres; +Y is up (matching Godot).
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct FxVec3 {
    pub x: Fx,
    pub y: Fx,
    pub z: Fx,
}

impl FxVec3 {
    pub const ZERO: FxVec3 = FxVec3 { x: Fx::ZERO, y: Fx::ZERO, z: Fx::ZERO };
    pub const X: FxVec3 = FxVec3 { x: Fx::ONE, y: Fx::ZERO, z: Fx::ZERO };
    pub const Y: FxVec3 = FxVec3 { x: Fx::ZERO, y: Fx::ONE, z: Fx::ZERO };
    pub const Z: FxVec3 = FxVec3 { x: Fx::ZERO, y: Fx::ZERO, z: Fx::ONE };

    #[inline]
    pub const fn new(x: Fx, y: Fx, z: Fx) -> FxVec3 {
        FxVec3 { x, y, z }
    }

    pub const fn from_ints(x: i32, y: i32, z: i32) -> FxVec3 {
        FxVec3::new(Fx::from_int(x), Fx::from_int(y), Fx::from_int(z))
    }

    #[inline]
    pub fn dot(self, o: FxVec3) -> Fx {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[inline]
    pub fn cross(self, o: FxVec3) -> FxVec3 {
        FxVec3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }

    /// Exact squared length in raw units, computed in 128-bit to avoid overflow.
    fn length_sq_raw(self) -> u128 {
        let (x, y, z) = (self.x.raw() as i128, self.y.raw() as i128, self.z.raw() as i128);
        (x * x + y * y + z * z) as u128
    }

    /// Length, rounded down. Safe for coordinates up to about a million metres.
    pub fn length(self) -> Fx {
        Fx::from_raw(isqrt_u128(self.length_sq_raw()) as i64)
    }

    pub fn length_sq(self) -> Fx {
        self.dot(self)
    }

    pub fn distance(self, o: FxVec3) -> Fx {
        (self - o).length()
    }

    /// Unit vector in the same direction; the zero vector stays zero.
    pub fn normalize(self) -> FxVec3 {
        let len = self.length();
        if len == Fx::ZERO {
            return FxVec3::ZERO;
        }
        FxVec3::new(self.x / len, self.y / len, self.z / len)
    }

    pub fn lerp(self, o: FxVec3, t: Fx) -> FxVec3 {
        self + (o - self) * t
    }

    pub fn to_f64(self) -> [f64; 3] {
        [self.x.to_f64(), self.y.to_f64(), self.z.to_f64()]
    }
}

impl Add for FxVec3 {
    type Output = FxVec3;
    fn add(self, o: FxVec3) -> FxVec3 {
        FxVec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for FxVec3 {
    type Output = FxVec3;
    fn sub(self, o: FxVec3) -> FxVec3 {
        FxVec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Neg for FxVec3 {
    type Output = FxVec3;
    fn neg(self) -> FxVec3 {
        FxVec3::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<Fx> for FxVec3 {
    type Output = FxVec3;
    fn mul(self, s: Fx) -> FxVec3 {
        FxVec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl AddAssign for FxVec3 {
    fn add_assign(&mut self, o: FxVec3) {
        *self = *self + o;
    }
}

impl SubAssign for FxVec3 {
    fn sub_assign(&mut self, o: FxVec3) {
        *self = *self - o;
    }
}
