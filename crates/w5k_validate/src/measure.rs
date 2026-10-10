//! Measurement: numbers read from a replay's driven trace (never from inside a model).

use w5k_contract::frame::Frame;
use w5k_math::scalar;

/// Speed a standing start is timed to: 32 km/h, the Army's usual acceleration benchmark.
pub const TARGET_SPEED_M_S: f64 = 32.0 / 3.6; // const-ok: 32 km/h converted to m/s
const REST_SPEED_M_S: f64 = 0.05; // const-ok: below this the vehicle counts as standing (a creep of 5 cm/s)

#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub peak_speed_m_s: f64,
    /// Seconds from leaving rest to first reaching 32 km/h; `None` if it started moving or never got there.
    pub time_to_32kmh_s: Option<f64>,
    pub distance_m: f64,
    pub frame_dt_s: f64,
}

fn speed(f: &Frame, vehicle: usize) -> Option<f64> {
    let v = f.vehicles.get(vehicle)?;
    Some(scalar::sqrt(v.lin_vel_m_s.length_sq()))
}

/// Measure vehicle `vehicle` from a trace sampled every `frame_dt_s`.
pub fn from_frames(frames: &[Frame], vehicle: usize, frame_dt_s: f64) -> Measured {
    let speeds: Vec<f64> = frames.iter().filter_map(|f| speed(f, vehicle)).collect();
    let peak = speeds.iter().copied().fold(0.0, f64::max);
    let distance = speeds.iter().sum::<f64>() * frame_dt_s;
    let time = if speeds.first().is_some_and(|&s| s < REST_SPEED_M_S) {
        // the start of the pull-away is the last frame at rest before the target is crossed; the crossing is interpolated
        speeds.iter().position(|&s| s >= TARGET_SPEED_M_S).map(|hit| {
            let left_rest = speeds[..hit].iter().rposition(|&s| s < REST_SPEED_M_S).unwrap_or(0);
            let (a, b) = (speeds[hit.saturating_sub(1)], speeds[hit]);
            let frac = if b > a { (TARGET_SPEED_M_S - a) / (b - a) } else { 1.0 };
            (hit.saturating_sub(1) - left_rest) as f64 * frame_dt_s + frac * frame_dt_s
        })
    } else {
        None
    };
    Measured { peak_speed_m_s: peak, time_to_32kmh_s: time, distance_m: distance, frame_dt_s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::frame::VehicleFrame;
    use w5k_math::{Quat, Vec3};

    fn frame(t: f64, v: f64) -> Frame {
        let vf = VehicleFrame {
            vehicle: 0,
            pos_m: Vec3::ZERO,
            rot: Quat::IDENTITY,
            lin_vel_m_s: Vec3 { x: 0.0, y: 0.0, z: -v },
            ang_vel_rad_s: Vec3::ZERO,
            joints: vec![],
            engine_rpm: 0.0,
            gear: 1,
            contacts: vec![],
            ledger_n: vec![],
            limiting: Default::default(),
            weapons: vec![],
        };
        Frame { t_s: t, vehicles: vec![vf], events: vec![], projectiles: vec![] }
    }

    #[test]
    fn standing_start_scenario_measures_time_to_32kmh_within_one_tick() {
        let dt = 1.0 / 30.0; // const-ok: test sampling interval
        let accel = 2.0; // const-ok: test acceleration, m/s^2
        let frames: Vec<Frame> = (0..300).map(|i| frame(i as f64 * dt, accel * i as f64 * dt)).collect();
        let m = from_frames(&frames, 0, dt);
        let expected = TARGET_SPEED_M_S / accel;
        let got = m.time_to_32kmh_s.expect("reaches 32 km/h");
        assert!((got - expected).abs() <= dt, "{got} vs {expected}");
        assert!((m.peak_speed_m_s - accel * 299.0 * dt).abs() < 1e-9);
    }

    #[test]
    fn a_run_that_starts_moving_has_no_standing_start_time() {
        let frames: Vec<Frame> = (0..10).map(|i| frame(i as f64, 12.0)).collect();
        assert_eq!(from_frames(&frames, 0, 1.0).time_to_32kmh_s, None);
    }
}
