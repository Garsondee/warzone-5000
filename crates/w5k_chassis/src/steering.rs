//! Steering geometry: from `Command::steer` (+1 = full right) to each steered wheel's angle (positive = left, as for any yaw).
//!
//! `SteerDef::max_angle_rad` is the **outer** wheel at full lock. With Ackermann geometry the inner wheel turns further so both
//! wheels roll about one turn centre on the line of the reference axle: for wheelbase `L` (from the reference axle) and track `t`,
//! `R = L / tan(delta_outer) - t/2` (turn radius of the centreline) and `tan(delta_inner) = L / (R - t/2)`. `ackermann` blends from
//! parallel steer (0) to exact Ackermann (1). A wheel behind the reference axle steers the other way (rear steer).

use w5k_contract::rig::SteerDef;
use w5k_math::scalar;

/// The geometry of one steered wheel relative to the reference axle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SteerGeometry {
    /// Longitudinal distance from the reference axle, m: positive ahead of it (hull -Z is forward).
    pub ahead_of_ref_m: f64,
    /// Lateral position, m: positive on the right (hull +X).
    pub lateral_m: f64,
}

/// Wheel angle (rad, positive = left) for a steer command in `[-1, 1]` (positive = right).
pub fn wheel_angle_rad(def: &SteerDef, g: &SteerGeometry, steer: f64) -> f64 {
    let s = scalar::clamp(steer, -1.0, 1.0);
    let outer = s.abs() * def.max_angle_rad;
    if outer <= 0.0 {
        return 0.0;
    }
    let l = g.ahead_of_ref_m.abs();
    let half_track = g.lateral_m.abs();
    // the wheel is on the inside of the turn when it is on the side the vehicle turns toward
    let inside = (s > 0.0) == (g.lateral_m > 0.0);
    let angle = if inside && l > 0.0 {
        let r_centre = l / scalar::tan(outer) - half_track;
        let r_inner = r_centre - half_track;
        let exact_inner = if r_inner > 0.0 { scalar::atan(l / r_inner) } else { core::f64::consts::FRAC_PI_2 };
        outer + def.ackermann * (exact_inner - outer)
    } else {
        outer
    };
    // right turn (s > 0) is a negative (clockwise from above) wheel angle; rear steer is opposite
    let dir = if g.ahead_of_ref_m >= 0.0 { 1.0 } else { -1.0 };
    -scalar::sign(s) * dir * angle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_ackermann_wheels_share_one_turn_centre() {
        let def = SteerDef { max_angle_rad: 0.5, ackermann: 1.0 };
        let (l, t2) = (3.3, 0.9);
        let right = SteerGeometry { ahead_of_ref_m: l, lateral_m: t2 };
        let left = SteerGeometry { ahead_of_ref_m: l, lateral_m: -t2 };
        // full right: right wheel is inner and turns further; both negative
        let (dr, dl) = (wheel_angle_rad(&def, &right, 1.0), wheel_angle_rad(&def, &left, 1.0));
        assert!(dr < dl && dl < 0.0);
        assert!((dl + 0.5).abs() < 1e-12);
        // both wheel axes meet on the reference axle line at the same turn radius
        let r_outer = l / scalar::tan(-dl) - t2;
        let r_inner = l / scalar::tan(-dr) + t2;
        assert!((r_outer - r_inner).abs() < 1e-9, "{r_outer} vs {r_inner}");
        // mirror for a left turn, and parallel steer gives equal angles
        assert!((wheel_angle_rad(&def, &left, -1.0) + dr).abs() < 1e-12);
        let par = SteerDef { ackermann: 0.0, ..def.clone() };
        assert!((wheel_angle_rad(&par, &right, 0.4) - wheel_angle_rad(&par, &left, 0.4)).abs() < 1e-12);
    }
}
