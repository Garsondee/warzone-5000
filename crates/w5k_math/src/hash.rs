//! Deterministic state hashing (FNV-1a, 64-bit) for golden tests and desync detection.
//!
//! Byte order is fixed (little-endian) so the result is the same on every platform. Floats are hashed by their bit patterns
//! after two normalisations: `-0.0` becomes `0.0`, and a non-finite value is a bug (debug builds panic; release builds hash a
//! fixed sentinel so the mismatch shows up in the golden rather than hiding).

use crate::mat3::Mat3;
use crate::quat::Quat;
use crate::vec3::Vec3;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;
const NON_FINITE: u64 = 0x7ff8_dead_beef_0001;

#[derive(Clone, Debug)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        StateHasher::new()
    }
}

impl StateHasher {
    pub fn new() -> StateHasher {
        StateHasher(OFFSET)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(PRIME);
        }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_i64(&mut self, v: i64) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_str(&mut self, s: &str) {
        self.write_u64(s.len() as u64);
        self.write_bytes(s.as_bytes());
    }

    pub fn write_f64(&mut self, v: f64) {
        debug_assert!(v.is_finite(), "non-finite value in simulation state: {v}");
        if !v.is_finite() {
            self.write_u64(NON_FINITE);
            return;
        }
        let v = if v == 0.0 { 0.0 } else { v };
        self.write_u64(v.to_bits());
    }

    pub fn write_vec3(&mut self, v: Vec3) {
        self.write_f64(v.x);
        self.write_f64(v.y);
        self.write_f64(v.z);
    }

    pub fn write_quat(&mut self, q: Quat) {
        self.write_f64(q.w);
        self.write_f64(q.x);
        self.write_f64(q.y);
        self.write_f64(q.z);
    }

    pub fn write_mat3(&mut self, m: &Mat3) {
        for row in &m.m {
            for &e in row {
                self.write_f64(e);
            }
        }
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_vector() {
        // FNV-1a 64 of "a" is af63dc4c8601ec8c.
        let mut h = StateHasher::new();
        h.write_bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn negative_zero_hashes_like_zero() {
        let mut a = StateHasher::new();
        let mut b = StateHasher::new();
        a.write_f64(0.0);
        b.write_f64(-0.0);
        assert_eq!(a.finish(), b.finish());
    }
}
