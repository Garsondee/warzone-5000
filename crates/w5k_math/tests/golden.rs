//! Golden determinism test.
//!
//! Runs a small fixed-point "simulation" that exercises every operation (add, mul, div, sqrt, sin, cos,
//! atan2, RNG) for thousands of ticks and compares the final state hash with a constant. If this hash ever
//! changes, the arithmetic changed: on another platform that would be a desync. Only update the constant
//! deliberately, together with a note in docs/planning/decisions.md.

use w5k_math::{Fx, FxVec3, Pcg32, StateHasher};

const GOLDEN: u64 = 0x7ec1_f7d5_0e20_5295;

fn scenario() -> u64 {
    let mut rng = Pcg32::derive(0x5EED, &[2026]);
    let dt = Fx::from_ratio(1, 20);
    let gravity = FxVec3::new(Fx::ZERO, Fx::from_ratio(-981, 100), Fx::ZERO);
    let mut pos: Vec<FxVec3> = (0..256)
        .map(|_| FxVec3::new(rng.range_fx(Fx::from_int(-100), Fx::from_int(100)), rng.range_fx(Fx::ONE, Fx::from_int(50)), rng.range_fx(Fx::from_int(-100), Fx::from_int(100))))
        .collect();
    let mut vel: Vec<FxVec3> = vec![FxVec3::ZERO; pos.len()];
    let mut hasher = StateHasher::new();
    for tick in 0..5_000u32 {
        for i in 0..pos.len() {
            // Steer toward a point on a moving circle (sin/cos), with a heading computed by atan2.
            let phase = Fx::from_int(tick as i32) * Fx::from_ratio(1, 50) + Fx::from_int(i as i32);
            let target = FxVec3::new(phase.cos() * Fx::from_int(40), Fx::from_int(10), phase.sin() * Fx::from_int(40));
            let to = target - pos[i];
            let dist = to.length();
            let heading = Fx::atan2(to.z, to.x);
            let push = if dist > Fx::ONE { to * (Fx::from_int(3) / dist) } else { FxVec3::ZERO };
            let jitter = FxVec3::new(heading.cos(), Fx::ZERO, heading.sin()) * rng.range_fx(-Fx::HALF, Fx::HALF);
            vel[i] += (gravity + push + jitter) * dt;
            vel[i] = vel[i] * Fx::from_ratio(99, 100);
            pos[i] += vel[i] * dt;
            if pos[i].y < Fx::ZERO {
                pos[i].y = -pos[i].y;
                vel[i].y = -vel[i].y * Fx::from_ratio(7, 10);
            }
        }
        if tick % 500 == 0 {
            for p in &pos {
                hasher.write_vec(*p);
            }
        }
    }
    for (p, v) in pos.iter().zip(&vel) {
        hasher.write_vec(*p);
        hasher.write_vec(*v);
    }
    hasher.finish()
}

#[test]
fn scenario_is_repeatable() {
    assert_eq!(scenario(), scenario());
}

#[test]
fn scenario_matches_golden_hash() {
    let h = scenario();
    assert_eq!(h, GOLDEN, "golden hash changed: got {h:#018x}");
}
