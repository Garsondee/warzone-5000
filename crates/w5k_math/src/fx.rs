//! Q32.32 fixed-point scalar.
//!
//! An `Fx` is a 64-bit signed integer holding the value multiplied by 2^32. Every operation is integer
//! arithmetic, so results are identical on every machine and compiler. Range is about +/-2.1e9 with a
//! resolution of about 2.3e-10.

use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Number of fractional bits.
pub const FRAC_BITS: u32 = 32;
const ONE_RAW: i64 = 1 << FRAC_BITS;

/// Q32.32 fixed-point number.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fx(i64);

/// Saturate an i128 into the i64 range. Overflow is a bug in gameplay code, so debug builds panic.
#[inline]
fn narrow(v: i128) -> i64 {
    debug_assert!(v >= i64::MIN as i128 && v <= i64::MAX as i128, "Fx overflow: {v}");
    v.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// `n / d` rounded to nearest, ties away from zero.
#[inline]
const fn round_div(n: i128, d: i128) -> i128 {
    let half = if d < 0 { -d / 2 } else { d / 2 };
    if (n < 0) == (d < 0) {
        if d < 0 { (n - half) / d } else { (n + half) / d }
    } else if d < 0 {
        (n + half) / d
    } else {
        (n - half) / d
    }
}

/// Integer square root of a u128 (largest r with r*r <= n).
pub fn isqrt_u128(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // Initial guess from the bit length, then Newton's method; converges in a few iterations.
    let bits = 128 - n.leading_zeros();
    let mut x: u128 = 1u128 << bits.div_ceil(2);
    loop {
        let y = (x + n / x) >> 1;
        if y >= x {
            break;
        }
        x = y;
    }
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

impl Fx {
    pub const ZERO: Fx = Fx(0);
    pub const ONE: Fx = Fx(ONE_RAW);
    pub const HALF: Fx = Fx(ONE_RAW / 2);
    pub const TWO: Fx = Fx(2 * ONE_RAW);
    pub const MAX: Fx = Fx(i64::MAX);
    pub const MIN: Fx = Fx(i64::MIN);
    /// Smallest positive value (2^-32).
    pub const EPSILON: Fx = Fx(1);
    /// pi, rounded to the nearest representable value.
    pub const PI: Fx = Fx(13_493_037_705);
    pub const HALF_PI: Fx = Fx(6_746_518_852);
    pub const QUARTER_PI: Fx = Fx(3_373_259_426);
    pub const TWO_PI: Fx = Fx(2 * 13_493_037_705);

    /// Build from the raw Q32.32 representation.
    #[inline]
    pub const fn from_raw(raw: i64) -> Fx {
        Fx(raw)
    }

    /// The raw Q32.32 representation.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[inline]
    pub const fn from_int(i: i32) -> Fx {
        Fx((i as i64) << FRAC_BITS)
    }

    /// `num / den`, rounded to the nearest representable value (ties away from zero).
    pub const fn from_ratio(num: i64, den: i64) -> Fx {
        assert!(den != 0, "Fx::from_ratio: zero denominator");
        Fx(round_div((num as i128) << FRAC_BITS, den as i128) as i64)
    }

    /// Convert from `f64` for **offline baking and tooling only**. The conversion of a given `f64` is exact
    /// IEEE arithmetic and therefore reproducible, but gameplay code must not produce floats in the first place.
    pub fn from_f64(f: f64) -> Fx {
        let scaled = f * ONE_RAW as f64;
        Fx(scaled.round().clamp(i64::MIN as f64, i64::MAX as f64) as i64)
    }

    /// Convert to `f64` for presentation and tests only.
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / ONE_RAW as f64
    }

    /// Largest integer not greater than `self`.
    #[inline]
    pub const fn floor_int(self) -> i64 {
        self.0 >> FRAC_BITS
    }

    #[inline]
    pub const fn floor(self) -> Fx {
        Fx(self.0 & !(ONE_RAW - 1))
    }

    #[inline]
    pub fn ceil(self) -> Fx {
        let f = self.floor();
        if f == self { f } else { f + Fx::ONE }
    }

    /// Round to nearest integer value, ties away from zero.
    pub fn round(self) -> Fx {
        if self.0 >= 0 { (self + Fx::HALF).floor() } else { -((-self) + Fx::HALF).floor() }
    }

    #[inline]
    pub fn frac(self) -> Fx {
        Fx(self.0 & (ONE_RAW - 1))
    }

    #[inline]
    pub fn abs(self) -> Fx {
        Fx(self.0.saturating_abs())
    }

    #[inline]
    pub fn signum(self) -> Fx {
        Fx::from_int(self.0.signum() as i32)
    }

    #[inline]
    pub fn min(self, o: Fx) -> Fx {
        if self <= o { self } else { o }
    }

    #[inline]
    pub fn max(self, o: Fx) -> Fx {
        if self >= o { self } else { o }
    }

    #[inline]
    pub fn clamp(self, lo: Fx, hi: Fx) -> Fx {
        self.max(lo).min(hi)
    }

    /// Multiply by an integer exactly.
    #[inline]
    pub fn mul_int(self, k: i64) -> Fx {
        Fx(narrow(self.0 as i128 * k as i128))
    }

    /// Square root, rounded down. Negative input returns zero (and panics in debug builds).
    pub fn sqrt(self) -> Fx {
        debug_assert!(self.0 >= 0, "Fx::sqrt of negative value");
        if self.0 <= 0 {
            return Fx::ZERO;
        }
        // sqrt(raw / 2^32) * 2^32 == sqrt(raw * 2^32)
        Fx(isqrt_u128((self.0 as u128) << FRAC_BITS) as i64)
    }

    /// Linear interpolation `self + (other - self) * t`.
    pub fn lerp(self, other: Fx, t: Fx) -> Fx {
        self + (other - self) * t
    }

    /// Sine of an angle in radians. Deterministic polynomial approximation, error below 1e-9.
    pub fn sin(self) -> Fx {
        // Reduce to [0, 2pi) with a 64-fractional-bit 2*pi so the error does not grow with |angle|,
        // then to [-pi, pi).
        let q64 = (self.0 as i128) << FRAC_BITS;
        let r = q64.rem_euclid(TWO_PI_Q64);
        let mut x = Fx(((r + (1i128 << (FRAC_BITS - 1))) >> FRAC_BITS) as i64);
        if x >= Fx::PI {
            x -= Fx::TWO_PI;
        }
        // Reduce to [-pi/2, pi/2] using sin(pi - x) = sin(x).
        if x > Fx::HALF_PI {
            x = Fx::PI - x;
        } else if x < -Fx::HALF_PI {
            x = -Fx::PI - x;
        }
        // Reduce to [-pi/4, pi/4]: beyond that use cos of the complement.
        if x > Fx::QUARTER_PI {
            cos_poly(Fx::HALF_PI - x)
        } else if x < -Fx::QUARTER_PI {
            -cos_poly(Fx::HALF_PI + x)
        } else {
            sin_poly(x)
        }
    }

    /// Cosine of an angle in radians.
    pub fn cos(self) -> Fx {
        (self + Fx::HALF_PI).sin()
    }

    /// Four-quadrant arctangent of `y / x`, in radians in (-pi, pi]. Uses CORDIC vectoring.
    pub fn atan2(y: Fx, x: Fx) -> Fx {
        if x == Fx::ZERO && y == Fx::ZERO {
            return Fx::ZERO;
        }
        // Rotate into the right half-plane first; CORDIC converges for angles within +/- 99 degrees.
        let (mut xr, mut yr, mut angle): (i128, i128, i64) = if x.0 < 0 {
            if y.0 >= 0 {
                (y.0 as i128, -(x.0 as i128), Fx::HALF_PI.0)
            } else {
                (-(y.0 as i128), x.0 as i128, -Fx::HALF_PI.0)
            }
        } else {
            (x.0 as i128, y.0 as i128, 0)
        };
        for (i, &a) in ATAN_TABLE.iter().enumerate() {
            let (nx, ny);
            if yr > 0 {
                nx = xr + (yr >> i);
                ny = yr - (xr >> i);
                angle += a;
            } else {
                nx = xr - (yr >> i);
                ny = yr + (xr >> i);
                angle -= a;
            }
            xr = nx;
            yr = ny;
        }
        if angle <= -Fx::PI.0 {
            angle += Fx::TWO_PI.0;
        }
        Fx(angle)
    }
}

/// 2*pi with 64 fractional bits (computed offline from 60 decimal digits of pi).
const TWO_PI_Q64: i128 = 115_904_311_329_233_965_478;

/// atan(2^-i) in Q32.32 for i = 0..33 (computed once offline; exact integers, so deterministic).
const ATAN_TABLE: [i64; 33] = [
    3373259426, 1991351318, 1052175346, 534100635, 268086748, 134174063, 67103403, 33553749, 16777131,
    8388597, 4194303, 2097152, 1048576, 524288, 262144, 131072, 65536, 32768, 16384, 8192, 4096, 2048,
    1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1,
];

/// Taylor series for sin on [-pi/4, pi/4] (terms to x^13; truncation error below 1e-12).
fn sin_poly(x: Fx) -> Fx {
    let x2 = x * x;
    // x - x^3/3! + x^5/5! - ... evaluated by Horner's rule in the form x*(1 - x2/(2*3)*(1 - x2/(4*5)*(...)))
    let mut acc = Fx::ONE;
    for k in (1..=6).rev() {
        let d = (2 * k) * (2 * k + 1);
        acc = Fx::ONE - (x2 * acc).div_int(d as i64);
    }
    x * acc
}

/// Taylor series for cos on [-pi/4, pi/4] (terms to x^14).
fn cos_poly(x: Fx) -> Fx {
    let x2 = x * x;
    let mut acc = Fx::ONE;
    for k in (1..=7).rev() {
        let d = (2 * k - 1) * (2 * k);
        acc = Fx::ONE - (x2 * acc).div_int(d as i64);
    }
    acc
}

impl Fx {
    /// Divide by an integer, rounding to nearest.
    #[inline]
    pub fn div_int(self, k: i64) -> Fx {
        assert!(k != 0, "Fx::div_int by zero");
        Fx(narrow(round_div(self.0 as i128, k as i128)))
    }
}

impl Add for Fx {
    type Output = Fx;
    #[inline]
    fn add(self, o: Fx) -> Fx {
        Fx(narrow(self.0 as i128 + o.0 as i128))
    }
}

impl Sub for Fx {
    type Output = Fx;
    #[inline]
    fn sub(self, o: Fx) -> Fx {
        Fx(narrow(self.0 as i128 - o.0 as i128))
    }
}

impl Neg for Fx {
    type Output = Fx;
    #[inline]
    fn neg(self) -> Fx {
        Fx(self.0.saturating_neg())
    }
}

impl Mul for Fx {
    type Output = Fx;
    /// Product rounded to nearest (ties toward +infinity).
    #[inline]
    fn mul(self, o: Fx) -> Fx {
        let p = self.0 as i128 * o.0 as i128;
        Fx(narrow((p + (1i128 << (FRAC_BITS - 1))) >> FRAC_BITS))
    }
}

impl Div for Fx {
    type Output = Fx;
    /// Quotient rounded to nearest (ties away from zero). Division by zero panics.
    fn div(self, o: Fx) -> Fx {
        assert!(o.0 != 0, "Fx division by zero");
        Fx(narrow(round_div((self.0 as i128) << FRAC_BITS, o.0 as i128)))
    }
}

macro_rules! assign_op {
    ($tr:ident, $f:ident, $op:tt) => {
        impl $tr for Fx {
            #[inline]
            fn $f(&mut self, o: Fx) {
                *self = *self $op o;
            }
        }
    };
}
assign_op!(AddAssign, add_assign, +);
assign_op!(SubAssign, sub_assign, -);
assign_op!(MulAssign, mul_assign, *);
assign_op!(DivAssign, div_assign, /);

impl fmt::Debug for Fx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fx({:.6})", self.to_f64())
    }
}

impl fmt::Display for Fx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}", self.to_f64())
    }
}
