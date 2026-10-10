//! The straight-line proving tests: `braking_50kmh` and `accel_0_48kmh` (docs/validation/proving-ground.md sections 3a and 3b).

use w5k_contract::rig::TICK_HZ;
use w5k_contract::{DriveInputs, GearRequest};
use w5k_math::scalar;

use super::facts::Facts;
use super::sim::Sim;
use super::{flat_world, Ctx, Outcome};

pub fn ticks(seconds: f64) -> u64 {
    (seconds * TICK_HZ).round() as u64
}

/// Full brake from a steady 50 km/h on dry asphalt, straight, no ABS, engine disconnected (neutral, as a brake-type test is run).
/// Measures the distance from the first tick of full brake to standstill and the peak deceleration averaged over `decel_window_s`.
/// The inputs echo the speed actually reached at brake onset and the friction the tyres actually have.
pub fn braking(c: &Ctx) -> Result<Outcome, String> {
    let (cfg, dt) = (c.cfg, 1.0 / TICK_HZ);
    let target_m_s = scalar::kmh_to_ms(cfg.braking_speed_kmh.v);
    let mut sim = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    sim.park(cfg.settle_s.v);
    let facts = Facts::read(&sim);

    // drive up to the entry speed from rest (the box and the converter reach it the way they would on the road) and hold it steady
    let (mut error_integral_m, mut steady_ticks, mut held) = (0.0, 0, false);
    for _ in 0..ticks(cfg.braking_approach_max_s.v) {
        let error_m_s = target_m_s - sim.speed_m_s();
        let demand = error_m_s * cfg.speed_gain_per_m_s.v + error_integral_m * cfg.speed_integral_per_m.v;
        if (0.0..=1.0).contains(&demand) {
            error_integral_m += error_m_s * dt; // conditional integration: no wind-up while the pedal is saturated
        }
        let throttle = scalar::clamp(demand, 0.0, 1.0);
        if !sim.tick(&DriveInputs { throttle, gear: GearRequest::Auto, ..DriveInputs::default() }) {
            break;
        }
        steady_ticks = if error_m_s.abs() <= cfg.braking_hold_tol_m_s.v { steady_ticks + 1 } else { 0 };
        if steady_ticks >= ticks(cfg.braking_hold_s.v) {
            held = true;
            break;
        }
    }
    let v0 = sim.speed_m_s();

    // full brake until stopped
    let start = sim.chassis.datum_m();
    let mut speeds = vec![v0];
    let brake = DriveInputs { brake: 1.0, gear: GearRequest::Neutral, ..DriveInputs::default() };
    let t_brake = sim.chassis.time_s;
    let mut stopped = false;
    for _ in 0..ticks(cfg.braking_max_s.v) {
        if !sim.tick(&brake) {
            break;
        }
        speeds.push(sim.speed_m_s());
        if sim.speed_m_s() < cfg.stopped_below_m_s.v {
            stopped = true;
            break;
        }
    }
    let end = sim.chassis.datum_m();
    let n = ticks(cfg.decel_window_s.v).max(1) as usize;
    let peak_decel_m_s2 = (n..speeds.len()).map(|i| (speeds[i - n] - speeds[i]) / (n as f64 * dt)).fold(0.0, f64::max);

    let mut out = Outcome::from_sim(sim);
    out.inputs.extend([("speed_m_s", v0), ("mu", facts.mu), ("mass_kg", facts.mass_kg)]);
    out.labels.extend([("surface", "dry_asphalt"), ("abs", "off"), ("driveline", "neutral while braking")]);
    if out.ended_early.is_none() && !held {
        out.ended_early = Some(format!("could not hold {target_m_s:.2} m/s before braking (at {v0:.2} m/s)"));
    }
    if out.ended_early.is_none() && !stopped {
        out.ended_early = Some(format!("did not stop within {} s of full brake", cfg.braking_max_s.v));
    }
    if out.ended_early.is_none() {
        let t_stop_s = out.time_s - t_brake;
        out.measured.extend([
            ("stop_distance_m", scalar::hypot(end.x - start.x, end.z - start.z)),
            ("peak_decel_g", peak_decel_m_s2 / scalar::G),
            ("stop_time_s", t_stop_s),
        ]);
    }
    Ok(out)
}

/// Full throttle from rest on dry asphalt with the automatic gearbox; the time to 48 km/h, interpolated inside the tick that crosses it.
pub fn acceleration(c: &Ctx) -> Result<Outcome, String> {
    let cfg = c.cfg;
    let target_m_s = scalar::kmh_to_ms(cfg.accel_target_kmh.v);
    let mut sim = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    sim.park(cfg.settle_s.v);
    let facts = Facts::read(&sim);

    let go = DriveInputs { throttle: 1.0, gear: GearRequest::Auto, ..DriveInputs::default() };
    let t0 = sim.chassis.time_s;
    let mut reached = None;
    for _ in 0..ticks(cfg.accel_max_s.v) {
        let (v_prev, t_prev) = (sim.speed_m_s(), sim.chassis.time_s);
        if !sim.tick(&go) {
            break;
        }
        let v = sim.speed_m_s();
        if v >= target_m_s {
            reached = Some(t_prev - t0 + (sim.chassis.time_s - t_prev) * scalar::inv_lerp(v_prev, v, target_m_s));
            break;
        }
    }

    let mut out = Outcome::from_sim(sim);
    out.inputs.extend([
        ("mass_kg", facts.mass_kg),
        ("power_w", facts.power_w),
        ("mu", facts.mu),
        ("driven_load_fraction", facts.driven_load_fraction),
        ("speed_m_s", target_m_s),
    ]);
    out.labels.extend([
        ("surface", "dry_asphalt"),
        ("gearbox", "auto"),
        ("power_basis", "engine peak shaft power x gearbox efficiency (an upper bound)"),
    ]);
    match (out.ended_early.is_some(), reached) {
        (false, Some(t)) => out.measured.extend([("t_0_48_s", t)]),
        (false, None) => out.ended_early = Some(format!("did not reach 48 km/h within {} s", cfg.accel_max_s.v)),
        _ => {}
    }
    Ok(out)
}
