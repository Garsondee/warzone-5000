//! `CapabilityTable`: what a vehicle can do, measured by running the vehicle itself through proving-ground scenarios (so it is
//! always consistent with the physics, never a hand-written guess). The AI plans with it and re-requests it when damage changes the
//! rig. Produced by the scenario runner (lane ARCH/VALIDATION), consumed by lane AI.

use serde::{Deserialize, Serialize};

use crate::world::MaterialId;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CapabilityTable {
    pub vehicle: String,
    /// Hash of the rig this table was measured on; a different rig invalidates it.
    pub rig_hash: u64,
    /// Steepest grade (rise/run) that can be climbed and held, per surface.
    pub max_grade: Vec<(MaterialId, f64)>,
    /// Highest sustainable speed per surface, m/s.
    pub top_speed_m_s: Vec<(MaterialId, f64)>,
    /// Surfaces on which the vehicle bogs (sinks past recovery) with a margin: `false` = no-go.
    pub soil_go: Vec<(MaterialId, bool)>,
    /// Steepest side slope (tilt) before rollover or sliding, rad.
    pub max_side_slope_rad: f64,
    /// Highest vertical step it can climb, m.
    pub max_step_m: f64,
    /// Widest trench it can cross, m.
    pub max_trench_m: f64,
    /// Deepest water it can ford, m.
    pub fording_depth_m: f64,
    /// Braking distance from a speed on level hard ground: `(speed_m_s, distance_m)`.
    pub braking: Vec<(f64, f64)>,
    /// Smallest turn radius at walking pace and at 20 km/h, m (0 for a pivot turn).
    pub min_turn_radius_slow_m: f64,
    pub min_turn_radius_fast_m: f64,
    /// Lateral acceleration at which it starts to slide or tip, m/s^2.
    pub max_lateral_accel_m_s2: f64,
    /// Turret traverse rate and gun elevation range, if any.
    pub turret_rate_rad_s: Option<f64>,
    pub gun_elevation_rad: Option<(f64, f64)>,
}

impl CapabilityTable {
    /// Interpolated braking distance for `speed_m_s` (linear between the measured points, clamped at the ends).
    pub fn braking_distance_m(&self, speed_m_s: f64) -> Option<f64> {
        let b = &self.braking;
        if b.is_empty() {
            return None;
        }
        // Braking distance goes with the square of speed (v^2 / 2 mu g), so extrapolate quadratically outside the table.
        let sq = |r: f64| r * r;
        if speed_m_s <= b[0].0 {
            return Some(b[0].1 * sq(speed_m_s / b[0].0.max(1e-9)));
        }
        for w in b.windows(2) {
            if speed_m_s <= w[1].0 {
                let t = (speed_m_s - w[0].0) / (w[1].0 - w[0].0);
                return Some(w[0].1 + (w[1].1 - w[0].1) * t);
            }
        }
        b.last().map(|l| l.1 * sq(speed_m_s / l.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braking_distance_interpolates_and_scales_quadratically_outside_the_table() {
        let t = CapabilityTable { braking: vec![(10.0, 8.0), (20.0, 30.0)], ..CapabilityTable::default() };
        assert!((t.braking_distance_m(15.0).unwrap() - 19.0).abs() < 1e-12);
        assert!((t.braking_distance_m(5.0).unwrap() - 2.0).abs() < 1e-12);
        assert!((t.braking_distance_m(40.0).unwrap() - 120.0).abs() < 1e-12);
        assert!(CapabilityTable::default().braking_distance_m(5.0).is_none());
    }
}
