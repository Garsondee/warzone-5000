//! Property tests: fixed-point operations agree with exact or f64 references within tight tolerances.
//! (Floats appear here only as a test oracle.)

use w5k_math::{Fx, FxVec3, Pcg32};

const RES: f64 = 1.0 / 4_294_967_296.0; // one Q32.32 step

fn rand_fx(r: &mut Pcg32, mag: i32) -> Fx {
    r.range_fx(Fx::from_int(-mag), Fx::from_int(mag))
}

#[test]
fn constants_and_conversions() {
    assert_eq!(Fx::from_int(3).to_f64(), 3.0);
    assert_eq!(Fx::from_ratio(1, 2), Fx::HALF);
    assert_eq!(Fx::from_ratio(-1, 4).to_f64(), -0.25);
    assert!((Fx::PI.to_f64() - std::f64::consts::PI).abs() < RES);
    assert_eq!(Fx::from_f64(2.5), Fx::from_ratio(5, 2));
    assert_eq!(Fx::from_ratio(7, 2).floor_int(), 3);
    assert_eq!(Fx::from_ratio(-7, 2).floor_int(), -4);
    assert_eq!(Fx::from_ratio(-7, 2).round(), Fx::from_int(-4));
    assert_eq!(Fx::from_ratio(7, 2).round(), Fx::from_int(4));
    assert_eq!(Fx::from_ratio(13, 4).ceil(), Fx::from_int(4));
}

#[test]
fn arithmetic_matches_reference() {
    let mut r = Pcg32::new(1, 1);
    for _ in 0..20_000 {
        // |a*b| must stay inside the Q32.32 range (about 2.1e9).
        let a = rand_fx(&mut r, 40_000);
        let b = rand_fx(&mut r, 40_000);
        assert_eq!(a + b, b + a);
        assert_eq!(a * b, b * a);
        assert!(((a + b).to_f64() - (a.to_f64() + b.to_f64())).abs() < RES);
        let p = (a * b).to_f64();
        assert!((p - a.to_f64() * b.to_f64()).abs() <= 1.0 * RES + 1e-15 * p.abs(), "mul {a:?} {b:?}");
        if b.abs() > Fx::from_ratio(1, 100) {
            let q = (a / b).to_f64();
            assert!((q - a.to_f64() / b.to_f64()).abs() <= RES + 1e-12 * q.abs(), "div {a:?} {b:?}");
        }
    }
}

#[test]
fn sqrt_is_floor_exact() {
    let mut r = Pcg32::new(2, 1);
    for _ in 0..20_000 {
        let a = r.range_fx(Fx::ZERO, Fx::from_int(1_000_000));
        let s = a.sqrt();
        // s is the largest representable value whose square (at full precision) does not exceed a.
        let s_raw = s.raw() as u128;
        let target = (a.raw() as u128) << 32;
        assert!(s_raw * s_raw <= target);
        assert!((s_raw + 1) * (s_raw + 1) > target);
        assert!((s.to_f64() - a.to_f64().sqrt()).abs() < 2.0 * RES + 1e-12 * a.to_f64());
    }
    assert_eq!(Fx::from_int(16).sqrt(), Fx::from_int(4));
    assert_eq!(Fx::ZERO.sqrt(), Fx::ZERO);
}

#[test]
fn trig_matches_reference() {
    let mut r = Pcg32::new(3, 1);
    for _ in 0..20_000 {
        let a = rand_fx(&mut r, 100);
        let (s, c) = (a.sin().to_f64(), a.cos().to_f64());
        let af = a.to_f64();
        assert!((s - af.sin()).abs() < 2e-9, "sin({af}) = {s}, expected {}", af.sin());
        assert!((c - af.cos()).abs() < 2e-9, "cos({af}) = {c}, expected {}", af.cos());
    }
    for (x, y) in [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1), (3, -7), (-5, 2)] {
        let got = Fx::atan2(Fx::from_int(y), Fx::from_int(x)).to_f64();
        let want = (y as f64).atan2(x as f64);
        assert!((got - want).abs() < 1e-8, "atan2({y},{x}) = {got}, expected {want}");
    }
    for _ in 0..20_000 {
        let y = rand_fx(&mut r, 1000);
        let x = rand_fx(&mut r, 1000);
        let got = Fx::atan2(y, x).to_f64();
        let want = y.to_f64().atan2(x.to_f64());
        assert!((got - want).abs() < 1e-8, "atan2({y},{x}) = {got}, expected {want}");
    }
}

#[test]
fn vectors() {
    let v = FxVec3::from_ints(3, 4, 12);
    assert_eq!(v.length(), Fx::from_int(13));
    let n = v.normalize();
    assert!((n.length().to_f64() - 1.0).abs() < 1e-8);
    assert_eq!(FxVec3::X.cross(FxVec3::Y), FxVec3::Z);
    assert_eq!(FxVec3::from_ints(1, 2, 3).dot(FxVec3::from_ints(4, 5, 6)), Fx::from_int(32));
}

#[test]
fn rng_is_uniform_and_reproducible() {
    let mut a = Pcg32::derive(42, &[1, 2]);
    let mut b = Pcg32::derive(42, &[1, 2]);
    let mut c = Pcg32::derive(42, &[1, 3]);
    let sa: Vec<u32> = (0..100).map(|_| a.next_u32()).collect();
    let sb: Vec<u32> = (0..100).map(|_| b.next_u32()).collect();
    let sc: Vec<u32> = (0..100).map(|_| c.next_u32()).collect();
    assert_eq!(sa, sb);
    assert_ne!(sa, sc);
    let mut counts = [0u32; 10];
    let mut r = Pcg32::new(7, 7);
    for _ in 0..100_000 {
        counts[r.below(10) as usize] += 1;
    }
    for c in counts {
        assert!((9_500..10_500).contains(&c), "bucket count {c}");
    }
}
