//! Proving-ground benches the runner calls: a **tilt table** (how far a parked truck can lean before an uphill wheel lifts) and a
//! **skidpad** (constant steer, slowly rising speed: the lateral grip limit and the understeer gradient). Every number is the caller's.

use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::{DriveInputs, DrivePort, GearRequest, WorldQuery};
use w5k_math::{scalar, Vec3};

use crate::tuning::ChassisTuning;
use crate::wheeled::{ChassisRefusal, WheeledChassis};

/// Height of the whole vehicle's centre of mass (hull plus unsprung masses) above the ground at the design pose, m.
pub fn com_height_m(rig: &PhysRig) -> f64 {
    let m: f64 = rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let my: f64 = rig.hull.mass_kg * rig.hull.com_m.y
        + rig.stations.iter().map(|s| s.unsprung_mass_kg * s.rest_pos_m.y).sum::<f64>();
    my / m + rig.ride_height_m
}

/// Half the mean track width (mean |x| of the stations), m.
pub fn half_track_m(rig: &PhysRig) -> f64 {
    rig.stations.iter().map(|s| s.rest_pos_m.x.abs()).sum::<f64>() / rig.stations.len().max(1) as f64
}

#[derive(Clone, Copy, Debug)]
pub struct TiltTable {
    /// How fast the table tilts, rad/s (slow enough to stay quasi-static).
    pub rate_rad_s: f64,
    /// Settling time on the level table before tilting, s.
    pub settle_s: f64,
    /// Give up beyond this angle, rad.
    pub max_angle_rad: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct TiltResult {
    /// Table angle at which the first uphill (left) wheel carried no load, rad; `None` if it never lifted. With unequal front and rear
    /// roll stiffness one axle takes more of the load transfer and its uphill wheel lifts first.
    pub lift_angle_rad: Option<f64>,
    /// Table angle at which every uphill wheel carried no load (the truck is tipping), rad.
    pub tip_angle_rad: Option<f64>,
    /// The rigid-body answer `atan(half_track / h)` (the static stability factor), rad. Suspension and tyre compliance let the body roll
    /// toward the low side first, so a real truck lifts earlier.
    pub rigid_estimate_rad: f64,
}

/// Park the truck (parking brake) on a level table, then tilt it about the forward axis with its right side going down until an uphill
/// wheel lifts and on until every uphill wheel is off (the tip). The table is tilted by rotating gravity, which is the same physics as tilting the ground.
pub fn tilt_table(
    rig: &PhysRig,
    tuning: &ChassisTuning,
    world: &dyn WorldQuery,
    drive: &mut dyn DrivePort,
    t: &TiltTable,
) -> Result<TiltResult, ChassisRefusal> {
    let mut c = WheeledChassis::new(rig, tuning, world, 0.0, 0.0, 0.0)?;
    let parked = DriveInputs { gear: GearRequest::Neutral, parking_brake: true, ..DriveInputs::default() };
    let dt = 1.0 / TICK_HZ;
    for _ in 0..(t.settle_s * TICK_HZ) as usize {
        c.tick(dt, &parked, world, drive);
    }
    let rigid_estimate_rad = scalar::atan(half_track_m(rig) / com_height_m(rig));
    let (mut angle, mut lift_angle_rad): (f64, Option<f64>) = (0.0, None);
    while angle < t.max_angle_rad {
        angle += t.rate_rad_s * dt;
        c.gravity_m_s2 = Vec3::new(scalar::sin(angle), -scalar::cos(angle), 0.0) * scalar::G;
        c.tick(dt, &parked, world, drive);
        let mut uphill = c
            .stations
            .iter()
            .zip(&rig.stations)
            .filter(|(_, d)| d.rest_pos_m.x < 0.0)
            .map(|(s, _)| s.report.contact.fz_n <= 0.0);
        let (any, all) = uphill.clone().fold((false, true), |(a, b), off| (a || off, b && off));
        let _ = uphill.next();
        if any && lift_angle_rad.is_none() {
            lift_angle_rad = Some(angle);
        }
        if all {
            return Ok(TiltResult { lift_angle_rad, tip_angle_rad: Some(angle), rigid_estimate_rad });
        }
    }
    Ok(TiltResult { lift_angle_rad, tip_angle_rad: None, rigid_estimate_rad })
}

#[derive(Clone, Copy, Debug)]
pub struct Skidpad {
    /// Constant steer command (+1 = full right).
    pub steer: f64,
    pub start_speed_m_s: f64,
    /// Speed ramp, m/s^2 (slow: each moment is close to a steady turn).
    pub ramp_m_s2: f64,
    /// Throttle per m/s of speed error for the speed-holding driver.
    pub speed_gain_per_m_s: f64,
    /// Samples before this time are the start transient and are dropped, s.
    pub warmup_s: f64,
    pub max_s: f64,
    /// The run ends when the lateral acceleration falls below this fraction of the best so far (the truck has slid out).
    pub slide_out_frac: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkidpadPoint {
    pub speed_m_s: f64,
    pub yaw_rate_rad_s: f64,
    pub lateral_acc_m_s2: f64,
    /// Mean steered-wheel angle magnitude, rad.
    pub steer_angle_rad: f64,
    /// Lateral load transfer `(sum of outer-wheel loads - sum of inner-wheel loads) / 2`, N.
    pub lateral_transfer_n: f64,
    /// Body roll toward the outside of the turn, rad (positive = leaning out).
    pub roll_out_rad: f64,
}

#[derive(Clone, Debug)]
pub struct SkidpadResult {
    pub points: Vec<SkidpadPoint>,
    pub max_lateral_acc_m_s2: f64,
    /// `K` in `delta = L / R + K a_y` (rad per m/s^2), fitted over the linear range (a_y below half the limit). Positive = understeer.
    pub understeer_gradient_rad_per_m_s2: f64,
    /// Wheelbase from the steered axle to the reference axle, m.
    pub wheelbase_m: f64,
}

/// Constant-steer skidpad on `world` with the given powertrain.
pub fn skidpad(
    rig: &PhysRig,
    tuning: &ChassisTuning,
    world: &dyn WorldQuery,
    drive: &mut dyn DrivePort,
    s: &Skidpad,
) -> Result<SkidpadResult, ChassisRefusal> {
    let mut c = WheeledChassis::new(rig, tuning, world, 0.0, 0.0, 0.0)?;
    c.set_forward_speed(s.start_speed_m_s);
    let dt = 1.0 / TICK_HZ;
    let steered: Vec<usize> =
        rig.stations.iter().enumerate().filter(|(_, d)| d.steer.is_some()).map(|(i, _)| i).collect();
    let unsteered_z: Vec<f64> = rig.stations.iter().filter(|d| d.steer.is_none()).map(|d| d.rest_pos_m.z).collect();
    let z_ref = unsteered_z.iter().sum::<f64>() / unsteered_z.len().max(1) as f64;
    let wheelbase_m = steered.iter().map(|&i| (z_ref - rig.stations[i].rest_pos_m.z).abs()).sum::<f64>()
        / steered.len().max(1) as f64;
    let (mut points, mut best) = (Vec::new(), 0.0_f64);
    let mut t = 0.0;
    while t < s.max_s {
        let target = s.start_speed_m_s + s.ramp_m_s2 * t;
        let v = c.forward_speed_m_s();
        let throttle = scalar::clamp((target - v) * s.speed_gain_per_m_s, 0.0, 1.0);
        let inp = DriveInputs { gear: GearRequest::Auto, throttle, steer: s.steer, ..DriveInputs::default() };
        c.tick(dt, &inp, world, drive);
        t += dt;
        if t < s.warmup_s {
            continue;
        }
        let r = c.yaw_rate_rad_s().abs();
        let speed = c.hull.vel_m_s.length();
        let ay = speed * r;
        let delta = steered.iter().map(|&i| c.stations[i].steer_rad.abs()).sum::<f64>() / steered.len().max(1) as f64;
        // turning right (negative yaw rate) puts the left wheels (x < 0) on the outside
        let outer_sign = if c.yaw_rate_rad_s() < 0.0 { -1.0 } else { 1.0 };
        let transfer = c
            .stations
            .iter()
            .zip(&rig.stations)
            .map(|(st, d)| scalar::sign(d.rest_pos_m.x) * outer_sign * st.report.contact.fz_n)
            .sum::<f64>()
            / 2.0;
        points.push(SkidpadPoint {
            speed_m_s: speed,
            yaw_rate_rad_s: r,
            lateral_acc_m_s2: ay,
            steer_angle_rad: delta,
            lateral_transfer_n: transfer,
            roll_out_rad: c.hull.rot.to_ypr().2 * outer_sign, // positive roll lowers the right side: leaning out of a left turn
        });
        best = best.max(ay);
        if ay < s.slide_out_frac * best || !c.is_finite() {
            break;
        }
    }
    // least squares of L/R = delta - K a_y over the linear range
    let lin: Vec<&SkidpadPoint> =
        points.iter().filter(|p| p.lateral_acc_m_s2 < 0.5 * best && p.speed_m_s > 0.0).collect();
    let n = lin.len() as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0);
    for p in &lin {
        let (x, y) = (p.lateral_acc_m_s2, wheelbase_m * p.yaw_rate_rad_s / p.speed_m_s - p.steer_angle_rad);
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let denom = n * sxx - sx * sx;
    let slope = if denom.abs() > 0.0 { (n * sxy - sx * sy) / denom } else { 0.0 };
    Ok(SkidpadResult { points, max_lateral_acc_m_s2: best, understeer_gradient_rad_per_m_s2: -slope, wheelbase_m })
}
