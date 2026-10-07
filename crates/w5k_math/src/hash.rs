//! Deterministic state hashing (FNV-1a, 64-bit) for desync detection and golden tests.

use crate::fx::Fx;
use crate::vec::FxVec3;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// Accumulates a hash of simulation state. Byte order is fixed (little-endian) so the result is the same
/// on every platform.
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

    pub fn write_fx(&mut self, v: Fx) {
        self.write_i64(v.raw());
    }

    pub fn write_vec(&mut self, v: FxVec3) {
        self.write_fx(v.x);
        self.write_fx(v.y);
        self.write_fx(v.z);
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}
