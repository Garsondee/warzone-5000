//! Deterministic random numbers (PCG32, "XSH RR" variant by M. E. O'Neill).

use crate::fx::Fx;
use crate::hash::StateHasher;

/// A seeded random stream. Never seed from the clock or the OS in gameplay code.
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

    /// A stream derived from a seed and a list of labels, e.g. `(match_seed, [player, draft_index])`.
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

    /// Uniform integer in `0..n` without modulo bias (rejection sampling). `n` must be > 0.
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

    /// Uniform `Fx` in `[0, 1)`.
    pub fn next_fx01(&mut self) -> Fx {
        Fx::from_raw(self.next_u32() as i64)
    }

    /// Uniform `Fx` in `[lo, hi)`.
    pub fn range_fx(&mut self, lo: Fx, hi: Fx) -> Fx {
        lo + (hi - lo) * self.next_fx01()
    }
}
