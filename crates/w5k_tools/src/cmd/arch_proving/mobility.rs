//! The terrain-limit proving tests: `side_slope_rollover` and `step_climb` (docs/validation/proving-ground.md sections 3e and 3g).

use w5k_chassis::bench::{com_height_m, half_track_m};
use w5k_contract::rig::TICK_HZ;
use w5k_contract::testing::world::{BumpStrip, Feature};
use w5k_contract::{DriveInputs, GearRequest};
use w5k_math::{scalar, Vec3};

use super::facts::Facts;
use super::longitudinal::ticks;
use super::sim::Sim;
use super::{flat_world, Ctx, Outcome};

/// Tilt table: park the truck, then rotate gravity about the forward axis (the same physics as tilting the ground, and what CHASSIS's
/// `bench::tilt_table` does) until an uphill wheel carries no load (`roll`) or every loaded tyre patch is at its friction limit, which is the onset of sliding (`slide`), whichever comes
/// first. CHASSIS's bench reports only the lift, so the slide needs this loop; a test checks the lift angles agree.
pub fn side_slope(c: &Ctx) -> Result<Outcome, String> {
    let (cfg, dt) = (c.cfg, 1.0 / TICK_HZ);
    let mut sim = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    sim.park(cfg.settle_s.v);
    let facts = Facts::read(&sim);
    let parked = DriveInputs { gear: GearRequest::Neutral, parking_brake: true, ..DriveInputs::default() };
    let (mut angle, mut found) = (0.0, None);
    while angle < cfg.tilt_max_rad.v && found.is_none() {
        angle += cfg.tilt_rate_rad_s.v * dt;
        sim.chassis.gravity_m_s2 = Vec3::new(scalar::sin(angle), -scalar::cos(angle), 0.0) * scalar::G;
        if !sim.tick(&parked) {
            break;
        }
        let lifted = sim
            .chassis
            .stations
            .iter()
            .zip(&c.rig.stations)
            .any(|(s, d)| d.rest_pos_m.x < 0.0 && s.report.contact.fz_n <= 0.0);
        if lifted {
            found = Some("roll");
        } else if sim.chassis.stations.iter().all(|s| s.report.contact.fz_n <= 0.0 || s.report.contact.saturated) {
            found = Some("slide");
        }
    }
    let mut out = Outcome::from_sim(sim);
    out.inputs.extend([
        ("track_m", 2.0 * half_track_m(c.rig)),
        ("cg_height_m", com_height_m(c.rig)),
        ("mu", facts.mu),
        ("mass_kg", facts.mass_kg),
    ]);
    out.labels.extend([("surface", "dry_asphalt"), ("method", "gravity rotated about the forward axis, parked")]);
    match (out.ended_early.is_some(), found) {
        (false, Some(mode)) => {
            out.labels.insert("mode", mode);
            out.measured.insert("slope_angle_rad", angle);
        }
        (false, None) => {
            out.ended_early = Some("neither a wheel lifted nor the truck slid before the table limit".into())
        }
        _ => {}
    }
    Ok(out)
}

/// What one trial of the step leaves behind: whether the rear axle got over it, the run, and the speed at which the front tyres met the face.
struct Trial<'a> {
    cleared: bool,
    sim: Sim<'a>,
    impact_m_s: f64,
}

/// One trial of the step: the truck drives straight at walking pace for `distance_m` at a vertical step `height_m` high; `cleared` if the
/// rear axle gets over it. The speed is held by the same proportional-integral driver as the straight-line tests, so every truck meets the
/// step at the spec speed whatever its rolling resistance and gearing (a proportional driver alone arrived 14 % slow on the Hauler, and by an
/// amount that moved with the engine, the final drive and the first gear, which made the climb depend on the driver and not on the design).
fn clears_step<'a>(c: &Ctx<'a>, distance_m: f64, height_m: f64, radius_m: f64) -> Result<Trial<'a>, String> {
    let (cfg, dt) = (c.cfg, 1.0 / TICK_HZ);
    let mut world = BumpStrip::standard();
    let z_step = -distance_m;
    world.features =
        vec![Feature::Plateau { z_m: z_step, ramp_m: cfg.step_ramp_m.v, hold_m: distance_m * 4.0, height_m }]; // const-ok: the plateau outlasts the truck
    let mut sim = Sim::new(c.rig, c.tuning, Box::new(world), cfg.frame_every_ticks)?;
    sim.park(cfg.settle_s.v);
    let front_z = c.rig.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MAX, f64::min);
    let rear_z = c.rig.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MIN, f64::max);
    let (mut error_integral_m, mut impact_m_s, mut met, mut cleared) = (0.0, 0.0, false, false);
    for _ in 0..ticks(cfg.step_max_s.v) {
        let error_m_s = cfg.step_speed_m_s.v - sim.speed_m_s();
        let demand = error_m_s * cfg.speed_gain_per_m_s.v + error_integral_m * cfg.speed_integral_per_m.v;
        if (0.0..=1.0).contains(&demand) {
            error_integral_m += error_m_s * dt; // conditional integration: no wind-up while the pedal is saturated
        }
        let throttle = scalar::clamp(demand, 0.0, 1.0);
        if !sim.tick(&DriveInputs { throttle, gear: GearRequest::Auto, ..DriveInputs::default() }) {
            break;
        }
        let datum_z = sim.chassis.datum_m().z;
        if !met && datum_z + front_z - radius_m <= z_step {
            (met, impact_m_s) = (true, sim.speed_m_s());
        }
        if datum_z + rear_z < z_step - radius_m {
            cleared = true;
            break;
        }
    }
    Ok(Trial { cleared, sim, impact_m_s })
}

/// One run-up's answer: the highest step cleared, the width of the final bisection bracket, and the trial to replay (the best cleared one,
/// or the lowest failed one when nothing was cleared).
struct Climb<'a> {
    height_m: f64,
    bracket_m: f64,
    trial: Trial<'a>,
}

/// Bisect the highest vertical step the truck clears after a run-up of `distance_m` (the search bracket is `[0, step_search_max_m]`, the
/// resolution `step_resolution_m`).
fn bisect_step<'a>(c: &Ctx<'a>, distance_m: f64, radius_m: f64) -> Result<Climb<'a>, String> {
    let (mut lo, mut hi, mut best) = (0.0, c.cfg.step_search_max_m.v, None);
    while hi - lo > c.cfg.step_resolution_m.v {
        let mid = 0.5 * (lo + hi);
        let trial = clears_step(c, distance_m, mid, radius_m)?;
        if trial.cleared {
            (lo, best) = (mid, Some(trial));
        } else {
            hi = mid;
        }
    }
    let trial = match best {
        Some(t) => t,
        None => clears_step(c, distance_m, hi, radius_m)?,
    };
    Ok(Climb { height_m: lo, bracket_m: hi - lo, trial })
}

/// The highest vertical step the truck clears at `step_speed_m_s`: the bisection of [`bisect_step`] repeated for `step_run_ups` run-ups
/// (`step_distance_m`, then `step_run_up_stride_m` longer each time), reporting the **median**. With a single ray under each tyre the
/// face is felt as an impulse when the wheel centre passes the edge, so the outcome depends on the phase of the body's bounce on arrival:
/// one run-up can land 30 % off the others. The median ignores a stray run-up and the inputs echo the spread, the width of the final
/// bracket and the speed at the face, so a reader can tell a real change from a change of the dice. The replay is the median run-up.
pub fn step_climb(c: &Ctx) -> Result<Outcome, String> {
    let cfg = c.cfg;
    let front = c.rig.stations.iter().map(|s| (s.rest_pos_m.z, s.wheel.radius_m)).fold((f64::MAX, 0.0), |a, b| {
        if b.0 < a.0 {
            b
        } else {
            a
        }
    });
    let mut climbs = Vec::new();
    for k in 0..(cfg.step_run_ups.v.round() as usize).max(1) {
        climbs.push(bisect_step(c, cfg.step_distance_m.v + k as f64 * cfg.step_run_up_stride_m.v, front.1)?);
    }
    climbs.sort_by(|a, b| a.height_m.total_cmp(&b.height_m));
    let spread_m = climbs[climbs.len() - 1].height_m - climbs[0].height_m;
    let median = climbs.swap_remove(climbs.len() / 2);
    let mut level = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    level.park(cfg.settle_s.v);
    let facts = Facts::read(&level);
    let mut out = Outcome::from_sim(median.trial.sim);
    out.inputs.extend([
        ("wheel_radius_m", front.1),
        ("mu", facts.mu),
        ("mass_kg", facts.mass_kg),
        ("approach_speed_m_s", median.trial.impact_m_s),
        ("step_resolution_m", median.bracket_m),
        ("step_run_up_spread_m", spread_m),
    ]);
    out.labels.extend([("surface", "dry_asphalt"), ("step", "vertical (Plateau with a short ramp), straight on")]);
    out.measured.insert("step_height_m", median.height_m);
    Ok(out)
}
