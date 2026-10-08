//! Deterministic random numbers: PCG32 (the "XSH RR" variant by M. E. O'Neill). Never seed from the clock or the OS.

use crate::hash::StateHasher;
use crate::scalar;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

const MULT: u64 = 6_364_136_223_846_793_005;

impl Pcg32 {
    /// A stream from a seed and a stream id (different ids give independent sequences).
    pub fn new(seed: u64, stream: u64) -> Pcg32 {
        let mut r = Pcg32 { state: 0, inc: (stream << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    /// A stream derived from a seed and a list of labels, e.g. `(scenario_seed, [vehicle, purpose])`.
    pub fn derive(seed: u64, labels: &[u64]) -> Pcg32 {
        let mut h = StateHasher::new();
        h.write_u64(seed);
        for &l in labels {
            h.write_u64(l);
        }
        let k = h.finish();
        Pcg32::new(k, k.rotate_left(17) ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform integer in `0..n` without modulo bias. `n` must be > 0.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        let threshold = n.wrapping_neg() % n;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % n;
            }
        }
    }

    /// Uniform `f64` in `[0, 1)` with 53 random bits (every value exactly representable).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Uniform `f64` in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Standard normal (Box-Muller; uses `libm`, so identical everywhere).
    pub fn normal(&mut self) -> f64 {
        let u1 = 1.0 - self.next_f64(); // (0, 1]
        let u2 = self.next_f64();
        scalar::sqrt(-2.0 * scalar::ln(u1)) * scalar::cos(scalar::TAU * u2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream_and_different_streams_differ() {
        let mut a = Pcg32::new(42, 7);
        let mut b = Pcg32::new(42, 7);
        let mut c = Pcg32::new(42, 8);
        let xs: Vec<u32> = (0..8).map(|_| a.next_u32()).collect();
        let ys: Vec<u32> = (0..8).map(|_| b.next_u32()).collect();
        let zs: Vec<u32> = (0..8).map(|_| c.next_u32()).collect();
        assert_eq!(xs, ys);
        assert_ne!(xs, zs);
    }

    #[test]
    fn unit_floats_are_in_range_and_have_a_sane_mean() {
        let mut r = Pcg32::new(1, 1);
        let n = 20_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let v = r.next_f64();
            assert!((0.0..1.0).contains(&v));
            sum += v;
        }
        assert!(scalar::approx_eq(sum / n as f64, 0.5, 0.01));
    }

    #[test]
    fn normal_has_roughly_zero_mean_and_unit_variance() {
        let mut r = Pcg32::new(9, 3);
        let n = 40_000;
        let (mut s, mut s2) = (0.0, 0.0);
        for _ in 0..n {
            let v = r.normal();
            s += v;
            s2 += v * v;
        }
        let mean = s / n as f64;
        let var = s2 / n as f64 - mean * mean;
        assert!(mean.abs() < 0.03 && scalar::approx_eq(var, 1.0, 0.05), "mean {mean} var {var}");
    }

    /// A pinned sequence: if this changes, every golden in the repository changes with it.
    #[test]
    fn the_first_outputs_are_pinned() {
        let mut r = Pcg32::new(42, 54);
        let v: Vec<u32> = (0..4).map(|_| r.next_u32()).collect();
        assert_eq!(v, vec![0xa15c_02b7, 0x7b47_f409, 0xba1d_3330, 0x83d2_f293]);
    }
}
