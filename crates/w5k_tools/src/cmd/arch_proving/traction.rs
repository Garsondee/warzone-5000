//! The grip proving tests: `skidpad` and `gradeability` (docs/validation/proving-ground.md sections 3d and 3c).

use w5k_chassis::bench::{com_height_m, half_track_m, skidpad as bench_skidpad, Skidpad};
use w5k_contract::{DriveInputs, GearRequest};
use w5k_drive::powertrain::{Powertrain, Tunings};
use w5k_math::{scalar, StateHasher, Vec3};

use super::facts::{wheel_torque_crawl_nm, Facts};
use super::longitudinal::ticks;
use super::sim::Sim;
use super::{flat_world, Ctx, Outcome};

/// Constant-steer skidpad on CHASSIS's bench: speed creeps up until the truck can no longer hold the circle. The inputs echo the radius
/// the truck actually turned at its limit (`v / yaw rate` there), its track and centre-of-mass height, and the static front load share.
/// The bench returns no frames, so there is no replay; the determinism hash is taken over the two measurements.
pub fn skidpad(c: &Ctx) -> Result<Outcome, String> {
    let cfg = c.cfg;
    let mut level = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    level.park(cfg.settle_s.v);
    let facts = Facts::read(&level);
    // the steer command that turns the wheels to the Ackermann angle of the chosen circle: atan(wheelbase / R), as a share of full lock
    let z = c.rig.stations.iter().map(|s| s.rest_pos_m.z);
    let wheelbase_m = z.clone().fold(f64::MIN, f64::max) - z.fold(f64::MAX, f64::min);
    let lock_rad = c.rig.stations.iter().filter_map(|s| s.steer.as_ref()).map(|d| d.max_angle_rad).fold(0.0, f64::max);
    let pad = Skidpad {
        steer: scalar::clamp(scalar::atan(wheelbase_m / cfg.skid_radius_m.v) / lock_rad, 0.0, 1.0),
        start_speed_m_s: cfg.skid_start_speed_m_s.v,
        ramp_m_s2: cfg.skid_ramp_m_s2.v,
        speed_gain_per_m_s: cfg.speed_gain_per_m_s.v,
        warmup_s: cfg.skid_warmup_s.v,
        max_s: cfg.skid_max_s.v,
        slide_out_frac: cfg.skid_slide_out_frac.v,
    };
    let mut drive =
        Powertrain::new(&c.rig.drivetrain, &Tunings::shipped()).map_err(|e| format!("DRIVE refused: {e}"))?;
    let run = bench_skidpad(c.rig, c.tuning, level.world.as_ref(), &mut drive, &pad)
        .map_err(|e| format!("CHASSIS refused: {e:?}"))?;
    // The limit is the highest *median* lateral acceleration over `skid_window_s`. A median ignores disturbances shorter than half the
    // window: a bump of the tail, and the last second before the truck spins out, where v times yaw rate no longer measures the path and
    // the swinging tail reads as grip that is not there (a maximum over samples or over a mean would take it for the limit).
    let n = ticks(cfg.skid_window_s.v).max(1) as usize;
    let median = |end: usize| {
        let mut w: Vec<f64> = run.points[end + 1 - n..=end].iter().map(|p| p.lateral_acc_m_s2).collect();
        w.sort_by(f64::total_cmp);
        w[n / 2]
    };
    let end = (n.saturating_sub(1)..run.points.len()).max_by(|&a, &b| median(a).total_cmp(&median(b)));
    let limit_m_s2 = end.map_or(0.0, median);
    let best = end.map(|e| &run.points[e]);
    let mut out = Outcome::default();
    out.inputs.extend([
        ("mu", facts.mu),
        ("radius_m", best.map_or(0.0, |p| p.speed_m_s / p.yaw_rate_rad_s.max(f64::MIN_POSITIVE))),
        ("track_m", 2.0 * half_track_m(c.rig)),
        ("cg_height_m", com_height_m(c.rig)),
        ("front_load_fraction", facts.front_load_fraction),
        ("mass_kg", facts.mass_kg),
    ]);
    out.labels.extend([("surface", "dry_asphalt"), ("method", "constant steer, speed ramped (CHASSIS bench)")]);
    let last = run.points.last();
    let slid = last.is_some_and(|p| p.lateral_acc_m_s2 < cfg.skid_slide_out_frac.v * run.max_lateral_acc_m_s2);
    if run.points.len() < n {
        out.ended_early = Some("the run ended before one averaging window of samples".into());
        return Ok(out);
    }
    // how the run ended: slid out (the bench's own rule); or the lateral acceleration plateaued while the speed kept rising (the truck
    // ploughs on at its grip limit: an understeering truck never spins); or the speed stopped rising first (the engine, not the tyres, ended it)
    let rose_m_s = last.zip(best).map_or(0.0, |(l, b)| l.speed_m_s - b.speed_m_s);
    let limit = if slid {
        "slide-out"
    } else if rose_m_s >= cfg.skid_plateau_gain_m_s.v {
        "grip plateau (speed rose, lateral acceleration did not)"
    } else {
        "power (speed stopped rising below the grip limit)"
    };
    out.labels.insert("limited_by", limit);
    let measured = [
        ("max_lat_accel_g", limit_m_s2 / scalar::G),
        ("understeer_gradient_rad_per_g", run.understeer_gradient_rad_per_m_s2 * scalar::G),
    ];
    out.measured.extend(measured);
    let mut h = StateHasher::new();
    measured.iter().for_each(|m| h.write_f64(m.1));
    out.hash = h.finish();
    Ok(out)
}

/// One start on a slope of the given grade (rise over run): gravity is rotated about the lateral axis (the physics of an inclined plane),
/// the truck is held with the parking brake, then released with full throttle in drive. It holds the grade if it has climbed
/// `grade_progress_m` uphill after `grade_hold_s`.
fn holds_grade<'a>(c: &Ctx<'a>, grade: f64) -> Result<(bool, Sim<'a>), String> {
    let cfg = c.cfg;
    let mut sim = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    let angle = scalar::atan(grade);
    sim.chassis.gravity_m_s2 = Vec3::new(0.0, -scalar::cos(angle), scalar::sin(angle)) * scalar::G;
    sim.park(cfg.settle_s.v);
    let start = sim.chassis.datum_m();
    let go = DriveInputs { throttle: 1.0, gear: GearRequest::Auto, ..DriveInputs::default() };
    for _ in 0..ticks(cfg.grade_hold_s.v) {
        if !sim.tick(&go) {
            break;
        }
    }
    let climbed_m = start.z - sim.chassis.datum_m().z;
    Ok((sim.early.is_none() && climbed_m >= cfg.grade_progress_m.v, sim))
}

/// Bisect the steepest grade the truck can start on from rest and hold for `grade_hold_s`, to `grade_resolution`. Inputs echo what the
/// oracle `min(mu f, torque)` needs: friction and level static load share on the driven wheels, the crawl torque the driveline can put on
/// the wheels (an upper bound), the mean driven wheel radius and the rolling resistance coefficient the sim applies (tyre plus surface).
pub fn gradeability(c: &Ctx) -> Result<Outcome, String> {
    let cfg = c.cfg;
    let (mut lo, mut hi, mut best) = (0.0, cfg.grade_search_max.v, None);
    while hi - lo > cfg.grade_resolution.v {
        let mid = 0.5 * (lo + hi);
        let (ok, sim) = holds_grade(c, mid)?;
        if ok {
            (lo, best) = (mid, Some(sim));
        } else {
            hi = mid;
        }
    }
    let sim = match best {
        Some(s) => s,
        None => holds_grade(c, hi)?.1,
    };
    let mut level = Sim::new(c.rig, c.tuning, flat_world(), cfg.frame_every_ticks)?;
    level.park(cfg.settle_s.v);
    let facts = Facts::read(&level);
    let driven: Vec<_> = c.rig.stations.iter().filter(|s| s.drive_output.is_some()).collect();
    let n = driven.len().max(1) as f64;
    let radius_m = driven.iter().map(|s| s.wheel.radius_m).sum::<f64>() / n;
    let tyre_rolling = driven.iter().map(|s| s.wheel.tyre.as_ref().map_or(0.0, |t| t.rolling_coeff)).sum::<f64>() / n;
    let surface_rolling = level.world.materials().get(level.chassis.stations[0].report.material).rolling_coeff;
    let mut out = Outcome::from_sim(sim);
    out.inputs.extend([
        ("mass_kg", facts.mass_kg),
        ("mu", facts.mu),
        ("driven_load_fraction", facts.driven_load_fraction),
        ("wheel_torque_crawl_nm", wheel_torque_crawl_nm(c.rig)),
        ("wheel_radius_m", radius_m),
        ("rolling_resistance_coeff", tyre_rolling + surface_rolling),
    ]);
    out.labels.extend([
        ("surface", "dry_asphalt"),
        ("method", "gravity rotated about the lateral axis, start from rest, bisected"),
    ]);
    out.measured.extend([("max_grade_ratio", lo), ("bracket_lo", lo), ("bracket_hi", hi)]);
    Ok(out)
}
