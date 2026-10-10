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

/// One trial of the step: the truck drives straight at walking pace at a vertical step `height_m` high; true if the rear axle gets over it.
fn clears_step<'a>(c: &Ctx<'a>, height_m: f64, radius_m: f64) -> Result<(bool, Sim<'a>), String> {
    let cfg = c.cfg;
    let mut world = BumpStrip::standard();
    let z_step = -cfg.step_distance_m.v;
    world.features = vec![Feature::Plateau {
        z_m: z_step,
        ramp_m: cfg.step_ramp_m.v,
        hold_m: cfg.step_distance_m.v * 4.0,
        height_m,
    }]; // const-ok: the plateau outlasts the truck
    let mut sim = Sim::new(c.rig, c.tuning, Box::new(world), cfg.frame_every_ticks)?;
    sim.park(cfg.settle_s.v);
    let rear_z = c.rig.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MIN, f64::max);
    let mut cleared = false;
    for _ in 0..ticks(cfg.step_max_s.v) {
        let throttle = scalar::clamp((cfg.step_speed_m_s.v - sim.speed_m_s()) * cfg.speed_gain_per_m_s.v, 0.0, 1.0);
        if !sim.tick(&DriveInputs { throttle, gear: GearRequest::Auto, ..DriveInputs::default() }) {
            break;
        }
        if sim.chassis.datum_m().z + rear_z < z_step - radius_m {
            cleared = true;
            break;
        }
    }
    Ok((cleared, sim))
}

/// Bisect the highest vertical step the truck clears at `step_speed_m_s` (the search bracket is `[0, step_search_max_m]`, resolution
/// `step_resolution_m`). The replay is the trial at the best cleared height.
pub fn step_climb(c: &Ctx) -> Result<Outcome, String> {
    let cfg = c.cfg;
    let front = c.rig.stations.iter().map(|s| (s.rest_pos_m.z, s.wheel.radius_m)).fold((f64::MAX, 0.0), |a, b| {
        if b.0 < a.0 {
            b
        } else {
            a
        }
    });
    let (mut lo, mut hi, mut best) = (0.0, cfg.step_search_max_m.v, None);
    while hi - lo > cfg.step_resolution_m.v {
        let mid = 0.5 * (lo + hi);
        let (ok, sim) = clears_step(c, mid, front.1)?;
        if ok {
            (lo, best) = (mid, Some(sim));
        } else {
            hi = mid;
        }
    }
    let sim = match best {
        Some(s) => s,
        None => clears_step(c, hi, front.1)?.1,
    };
    let mut level = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    level.park(cfg.settle_s.v);
    let facts = Facts::read(&level);
    let mut out = Outcome::from_sim(sim);
    out.inputs.extend([("wheel_radius_m", front.1), ("mu", facts.mu), ("mass_kg", facts.mass_kg)]);
    out.labels.extend([("surface", "dry_asphalt"), ("step", "vertical (Plateau with a short ramp), straight on")]);
    out.measured.insert("step_height_m", lo);
    Ok(out)
}
