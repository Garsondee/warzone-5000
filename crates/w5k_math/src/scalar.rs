//! Scalar helpers. Transcendentals go through `libm` (see the crate docs for why).

/// Pi.
pub const PI: f64 = core::f64::consts::PI;
/// Two pi.
pub const TAU: f64 = core::f64::consts::TAU;
/// Standard gravity (m/s^2): a defined constant, not a measurement.
pub const G: f64 = 9.80665;

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}
pub fn asin(x: f64) -> f64 {
    libm::asin(x)
}
pub fn acos(x: f64) -> f64 {
    libm::acos(x)
}
pub fn atan(x: f64) -> f64 {
    libm::atan(x)
}
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
pub fn sinh(x: f64) -> f64 {
    libm::sinh(x)
}
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}
/// `x` to the power `y` (any real exponents; for small integer powers prefer plain multiplication).
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}
pub fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}
pub fn cosh(x: f64) -> f64 {
    libm::cosh(x)
}
pub fn asinh(x: f64) -> f64 {
    libm::asinh(x)
}
pub fn acosh(x: f64) -> f64 {
    libm::acosh(x)
}
pub fn atanh(x: f64) -> f64 {
    libm::atanh(x)
}
pub fn exp2(x: f64) -> f64 {
    libm::exp2(x)
}
/// `exp(x) - 1`, accurate for small `x`.
pub fn exp_m1(x: f64) -> f64 {
    libm::expm1(x)
}
/// `ln(1 + x)`, accurate for small `x`.
pub fn ln_1p(x: f64) -> f64 {
    libm::log1p(x)
}
pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}
/// Logarithm of `x` to the given `base`.
pub fn log(x: f64, base: f64) -> f64 {
    libm::log(x) / libm::log(base)
}
/// `x` to an integer power by repeated squaring: only multiplications, in a fixed order, so it is identical everywhere
/// (std `powi` is lowered differently by different compilers and targets, which is why it is banned).
pub fn powi(x: f64, n: i32) -> f64 {
    let mut base = x;
    let mut e = n.unsigned_abs();
    let mut acc = 1.0;
    while e > 0 {
        if e & 1 == 1 {
            acc *= base;
        }
        base *= base;
        e >>= 1;
    }
    if n < 0 {
        1.0 / acc
    } else {
        acc
    }
}
/// Fused multiply-add computed in software (`a * b + c` with a single rounding), so it is identical on every CPU.
/// Use only when you genuinely need the single rounding; plain `a * b + c` is the default.
pub fn fma(a: f64, b: f64, c: f64) -> f64 {
    libm::fma(a, b, c)
}
/// Square root (exactly rounded by IEEE 754, so identical everywhere).
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}
/// `(sin x, cos x)`.
pub fn sin_cos(x: f64) -> (f64, f64) {
    (libm::sin(x), libm::cos(x))
}

/// Clamp without the panics of `f64::clamp`: `lo <= result <= hi` whenever `lo <= hi`; NaN input stays NaN.
pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// Linear interpolation: `a` at `t = 0`, `b` at `t = 1`.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Where `x` sits between `a` and `b` (0 at `a`, 1 at `b`); 0 when `a == b`.
pub fn inv_lerp(a: f64, b: f64, x: f64) -> f64 {
    let d = b - a;
    if d.abs() < 1e-300 {
        0.0
    } else {
        (x - a) / d
    }
}

/// Hermite smooth step: 0 below `e0`, 1 above `e1`, smooth between.
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp(inv_lerp(e0, e1, x), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// -1, 0 or +1.
pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// An angle wrapped into `(-pi, pi]`.
pub fn wrap_pi(a: f64) -> f64 {
    let mut r = a % TAU;
    if r > PI {
        r -= TAU;
    } else if r <= -PI {
        r += TAU;
    }
    r
}

/// Degrees to radians.
pub fn deg_to_rad(d: f64) -> f64 {
    d * (PI / 180.0)
}
/// Radians to degrees.
pub fn rad_to_deg(r: f64) -> f64 {
    r * (180.0 / PI)
}
/// km/h to m/s (an edge conversion: the simulation never stores km/h).
pub fn kmh_to_ms(v: f64) -> f64 {
    v / 3.6
}
/// m/s to km/h (an edge conversion for display).
pub fn ms_to_kmh(v: f64) -> f64 {
    v * 3.6
}
/// rpm to rad/s.
pub fn rpm_to_rad_s(rpm: f64) -> f64 {
    rpm * (TAU / 60.0)
}
/// rad/s to rpm.
pub fn rad_s_to_rpm(w: f64) -> f64 {
    w * (60.0 / TAU)
}

/// `|a - b| <= tol`.
pub fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Replace a vanishingly small value by exactly zero. Call it on filtered or decaying state (lags, damped oscillations, relaxation
/// states) at the end of a step: a host process may run with a different denormal mode, and values near 1e-300 behave differently
/// there. Flushing on purpose at known points keeps the results identical everywhere.
pub fn flush_tiny(x: f64) -> f64 {
    if x.abs() < 1e-30 {
        0.0
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_pi_maps_into_the_half_open_interval() {
        assert!(approx_eq(wrap_pi(3.0 * PI), PI, 1e-12));
        assert!(approx_eq(wrap_pi(-3.0 * PI), PI, 1e-12));
        assert!(approx_eq(wrap_pi(0.5), 0.5, 1e-15));
        assert!(approx_eq(wrap_pi(TAU + 0.25), 0.25, 1e-12));
    }

    #[test]
    fn smoothstep_is_zero_one_and_half_at_the_middle() {
        assert!(approx_eq(smoothstep(1.0, 3.0, 0.0), 0.0, 1e-15));
        assert!(approx_eq(smoothstep(1.0, 3.0, 9.0), 1.0, 1e-15));
        assert!(approx_eq(smoothstep(1.0, 3.0, 2.0), 0.5, 1e-15));
    }

    #[test]
    fn unit_conversions_round_trip() {
        assert!(approx_eq(ms_to_kmh(kmh_to_ms(72.0)), 72.0, 1e-12));
        assert!(approx_eq(rad_s_to_rpm(rpm_to_rad_s(3000.0)), 3000.0, 1e-9));
        assert!(approx_eq(rad_to_deg(deg_to_rad(33.0)), 33.0, 1e-12));
    }

    #[test]
    fn flush_tiny_zeroes_only_the_negligible() {
        assert!(approx_eq(flush_tiny(4.2e-38), 0.0, 0.0));
        assert!(approx_eq(flush_tiny(-1e-31), 0.0, 0.0));
        assert!(approx_eq(flush_tiny(1e-20), 1e-20, 0.0));
        assert!(approx_eq(flush_tiny(3.0), 3.0, 0.0));
    }

    #[test]
    fn clamp_does_not_panic_and_keeps_order() {
        assert!(approx_eq(clamp(5.0, 0.0, 1.0), 1.0, 0.0));
        assert!(approx_eq(clamp(-5.0, 0.0, 1.0), 0.0, 0.0));
        assert!(clamp(f64::NAN, 0.0, 1.0).is_nan());
    }

    #[test]
    fn powi_matches_repeated_multiplication_and_negative_powers() {
        assert!(approx_eq(powi(1.5, 0), 1.0, 0.0));
        assert!(approx_eq(powi(1.5, 1), 1.5, 0.0));
        assert!(approx_eq(powi(2.0, 10), 1024.0, 0.0));
        assert!(approx_eq(powi(-3.0, 3), -27.0, 0.0));
        assert!(approx_eq(powi(2.0, -3), 0.125, 0.0));
        assert!(approx_eq(powi(1.1, 7), pow(1.1, 7.0), 1e-12));
        assert!(approx_eq(powi(2.0, i32::MIN + 1), 0.0, 0.0));
    }

    #[test]
    fn the_extra_transcendentals_obey_their_identities() {
        for &x in &[0.1, 0.5, 1.0, 2.5] {
            assert!(approx_eq(cosh(x) * cosh(x) - sinh(x) * sinh(x), 1.0, 1e-12));
            assert!(approx_eq(asinh(sinh(x)), x, 1e-12));
            assert!(approx_eq(acosh(cosh(x)), x, 1e-12));
            assert!(approx_eq(atanh(tanh(x.min(1.5))), x.min(1.5), 1e-12));
            assert!(approx_eq(exp2(x), pow(2.0, x), 1e-12));
            assert!(approx_eq(exp_m1(x), exp(x) - 1.0, 1e-12));
            assert!(approx_eq(ln_1p(x), ln(1.0 + x), 1e-12));
            assert!(approx_eq(log2(exp2(x)), x, 1e-12));
            assert!(approx_eq(log(pow(10.0, x), 10.0), x, 1e-12));
        }
        assert!(approx_eq(fma(2.0, 3.0, 4.0), 10.0, 0.0));
    }
}
