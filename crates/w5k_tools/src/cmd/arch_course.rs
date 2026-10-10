//! `w5k scenario mule-course --out DIR`: ARCH's integration glue. FORGE's Mule 4x4 drives WORLD's slice course with DRIVE's real powertrain.
//!
//! The pieces (all built here, none edited): `w5k_forge::compile` gives the rig; `w5k_drive::powertrain::Powertrain` is built from the rig's
//! drivetrain; `w5k_world::course::generate` gives a `GridWorld` (a `WorldQuery`) and the road centreline; `w5k_chassis::WheeledChassis`
//! integrates the truck. A scripted driver closes the loop: a speed controller on the throttle and brake pedals and pure-pursuit steering
//! along the centreline. The run is done twice and the state hashes compared (determinism, printed). Outputs in DIR: `replay.w5kr`,
//! `replay.json`, `rig.json` (for `w5k viewer render`) and CSV traces (`speed.csv`, `rpm.csv`, `gear.csv`, `track.csv`) for `w5k viewer plot`.
//!
//! Every driver number lives in `content/physics/arch/mule_course.ron` with provenance; the code holds only unit conversions and maths.

use std::fmt::Write as _;
use std::path::Path;

use serde::Deserialize;
use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::frame::{Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::{
    ContactFrame, DriveInputs, DrivePort, DriveTelemetry, GearRequest, LimitingFactor, Param, VehicleFrame, WorldQuery,
};
use w5k_drive::powertrain::{Powertrain, Tunings};
use w5k_math::{scalar, Quat, StateHasher, Vec3};
use w5k_replay::ReplayFile;
use w5k_world::course::{generate, CourseDef};
use w5k_world::grid::GridWorld;

const USAGE: &str = "usage: w5k scenario mule-course [--scenario FILE.ron] --out DIR";
const DEFAULT_SCENARIO: &str = "content/physics/arch/mule_course.ron";
const KN_PER_N: f64 = 1e-3; // const-ok: unit conversion for the CSV traces
const MM_PER_M: f64 = 1e3; // const-ok: unit conversion for the CSV traces
const PLAN_EPS: f64 = 1e-9; // const-ok: guard against dividing by a zero-length chord
const SEARCH_SAMPLES: usize = 400; // const-ok: how many centreline samples ahead of the last fix the nearest-point search looks at

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MuleCourse {
    vehicle: String,
    extras: String,
    course: String,
    chassis_tuning: String,
    cruise_speed_m_s: Param,
    max_lateral_accel_m_s2: Param,
    speed_gain_per_m_s: Param,
    brake_gain_per_m_s: Param,
    brake_deadband_m_s: Param,
    lookahead_base_m: Param,
    lookahead_per_m_s: Param,
    curvature_window_m: Param,
    final_decel_m_s2: Param,
    finish_stop_margin_m: Param,
    stopped_below_m_s: Param,
    stall_window_s: Param,
    stall_progress_m: Param,
    off_road_limit_m: Param,
    max_seconds: Param,
    /// Cap the route at this distance along the road (m) when the truck cannot drive the whole course; `None` = the whole road.
    #[serde(default)]
    route_end_m: Option<Param>,
    frame_every_ticks: u32,
}

impl MuleCourse {
    fn check(&self) -> Result<(), String> {
        let ps = [
            ("cruise_speed_m_s", &self.cruise_speed_m_s),
            ("max_lateral_accel_m_s2", &self.max_lateral_accel_m_s2),
            ("speed_gain_per_m_s", &self.speed_gain_per_m_s),
            ("brake_gain_per_m_s", &self.brake_gain_per_m_s),
            ("brake_deadband_m_s", &self.brake_deadband_m_s),
            ("lookahead_base_m", &self.lookahead_base_m),
            ("lookahead_per_m_s", &self.lookahead_per_m_s),
            ("curvature_window_m", &self.curvature_window_m),
            ("final_decel_m_s2", &self.final_decel_m_s2),
            ("finish_stop_margin_m", &self.finish_stop_margin_m),
            ("stopped_below_m_s", &self.stopped_below_m_s),
            ("stall_window_s", &self.stall_window_s),
            ("stall_progress_m", &self.stall_progress_m),
            ("off_road_limit_m", &self.off_road_limit_m),
            ("max_seconds", &self.max_seconds),
        ];
        for (name, p) in ps {
            p.check(&format!("mule_course.{name}"))?;
        }
        if let Some(p) = &self.route_end_m {
            p.check("mule_course.route_end_m")?;
        }
        Ok(())
    }
}

/// The road centreline as a polyline with arclength.
pub(crate) struct RoadPath {
    pts: Vec<(f64, f64)>,
    s: Vec<f64>,
}

impl RoadPath {
    fn new(road: &[(f64, f64, f64)]) -> Result<RoadPath, String> {
        let mut pts: Vec<(f64, f64)> = Vec::new();
        let mut s: Vec<f64> = Vec::new();
        for &(x, z, _y) in road {
            match pts.last() {
                Some(&(px, pz)) => {
                    let d = scalar::hypot(x - px, z - pz);
                    if d > PLAN_EPS {
                        s.push(s[s.len() - 1] + d);
                        pts.push((x, z));
                    }
                }
                None => {
                    pts.push((x, z));
                    s.push(0.0);
                }
            }
        }
        if pts.len() < 2 {
            return Err("the road centreline has fewer than two distinct points".into());
        }
        Ok(RoadPath { pts, s })
    }

    fn length_m(&self) -> f64 {
        self.s[self.s.len() - 1]
    }

    /// The plan position at arclength `s` (clamped to the road).
    fn point_at(&self, s: f64) -> (f64, f64) {
        let s = scalar::clamp(s, 0.0, self.length_m());
        let k = self.s.partition_point(|&v| v <= s).clamp(1, self.s.len() - 1);
        let t = scalar::inv_lerp(self.s[k - 1], self.s[k], s);
        (scalar::lerp(self.pts[k - 1].0, self.pts[k].0, t), scalar::lerp(self.pts[k - 1].1, self.pts[k].1, t))
    }

    /// Heading (yaw convention: positive = nose left) of the road at `s`, looking `probe_m` ahead.
    fn heading_at(&self, s: f64, probe_m: f64) -> f64 {
        let (a, b) = (self.point_at(s), self.point_at(s + probe_m));
        scalar::atan2(-(b.0 - a.0), -(b.1 - a.1))
    }

    /// Largest path curvature (1/m) in the next `window_m`, from heading change over chords of a quarter window up to the whole window.
    fn upcoming_curvature(&self, s: f64, window_m: f64) -> f64 {
        let step = window_m * 0.25; // const-ok: the window is sampled in quarters
        let h0 = self.heading_at(s, step);
        (1..=4)
            .map(|k| {
                let d = f64::from(k) * step;
                scalar::wrap_pi(self.heading_at(s + d, step) - h0).abs() / d
            })
            .fold(0.0, f64::max)
    }

    /// Nearest point of the polyline to `(x, z)` searching from sample `hint`: `(index, arclength, distance)`.
    fn locate(&self, x: f64, z: f64, hint: usize) -> (usize, f64, f64) {
        let lo = hint.saturating_sub(SEARCH_SAMPLES / 4); // const-ok: mostly look ahead, a little behind
        let hi = (hint + SEARCH_SAMPLES).min(self.pts.len() - 1);
        let mut best = (hint, self.s[hint.min(self.pts.len() - 1)], f64::INFINITY);
        for k in lo..hi {
            let (a, b) = (self.pts[k], self.pts[k + 1]);
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let t = scalar::clamp(((x - a.0) * dx + (z - a.1) * dz) / (dx * dx + dz * dz), 0.0, 1.0);
            let d = scalar::hypot(x - (a.0 + t * dx), z - (a.1 + t * dz));
            if d < best.2 {
                best = (k, scalar::lerp(self.s[k], self.s[k + 1], t), d);
            }
        }
        best
    }
}

/// One trace sample (one per replay frame, taken before the tick): what `course-compare` computes its report from.
#[derive(Clone, Debug)]
pub(crate) struct Sample {
    pub t_s: f64,
    pub s_m: f64,
    pub speed_m_s: f64,
    pub target_m_s: f64,
    pub cross_track_m: f64,
    pub steer_angle_rad: f64,
    /// Centripetal acceleration `v * yaw rate` (positive = turning left), m/s^2.
    pub lat_accel_m_s2: f64,
    /// Rate of change of the hull datum's vertical velocity between samples, m/s^2 (0 on flat ground at constant speed).
    pub vert_accel_m_s2: f64,
    pub gear: i8,
    pub fuel_kg: f64,
    /// Any tyre patch at its friction or shear limit.
    pub tyre_limit: bool,
}

/// What a finished run leaves behind.
#[derive(Clone)]
pub(crate) struct RunResult {
    pub frame_every_ticks: u32,
    pub frames: Vec<Frame>,
    pub samples: Vec<Sample>,
    pub hashes: Vec<u64>,
    pub final_hash: u64,
    pub stop_reason: String,
    pub stopped_at_s_m: f64,
    pub route_m: f64,
    pub time_s: f64,
    max_speed_m_s: f64,
    max_cross_track_m: f64,
    gears_used: Vec<i8>,
    fuel_kg: f64,
    pub speed: String,
    pub rpm: String,
    pub gear: String,
    pub track: String,
    pub loads: String,
}

/// The driver scenario, the chassis tuning and the generated course: everything a run needs that is not the vehicle.
pub(crate) struct Setup {
    sc: MuleCourse,
    tuning: ChassisTuning,
    world: GridWorld,
    path: RoadPath,
    pub course_name: String,
    pub course_seed: u64,
}

impl Setup {
    /// Read the scenario (paths relative to `root`); `course` replaces the scenario's course file when given.
    pub(crate) fn load(root: &Path, scenario: &str, course: Option<&str>) -> Result<Setup, String> {
        let sc: MuleCourse = ron::from_str(&read(root, scenario)?).map_err(|e| format!("{scenario}: {e}"))?;
        sc.check()?;
        let tuning = ChassisTuning::from_ron(&read(root, &sc.chassis_tuning)?)
            .map_err(|e| format!("{}: {e}", sc.chassis_tuning))?;
        let course_def = CourseDef::from_ron(&read(root, course.unwrap_or(&sc.course))?)?;
        let generated = generate(&course_def)?;
        let path = RoadPath::new(&generated.road)?;
        Ok(Setup {
            sc,
            tuning,
            world: generated.world,
            path,
            course_name: course_def.name,
            course_seed: course_def.seed,
        })
    }

    /// Run `rig` over the course twice from scratch and demand identical hashes (determinism).
    pub(crate) fn run_checked(&self, rig: &PhysRig) -> Result<RunResult, String> {
        let run = simulate(&self.sc, rig, &self.tuning, &self.world, &self.path)?;
        let again = simulate(&self.sc, rig, &self.tuning, &self.world, &self.path)?;
        println!("{}: state hash, run 1: {:016x}, run 2: {:016x}", rig.id, run.final_hash, again.final_hash);
        if run.final_hash != again.final_hash || run.hashes != again.hashes {
            return Err(format!("{}: the two runs differ: the simulation is not deterministic", rig.id));
        }
        println!("{}: deterministic: identical ({} per-second hashes compared)", rig.id, run.hashes.len());
        Ok(run)
    }
}

/// Everything a scenario produces before it is written to disk.
struct Outcome {
    run: RunResult,
    rig: PhysRig,
    hull_size_m: Vec3,
    vehicle_id: String,
    course_name: String,
    course_seed: u64,
}

/// Load the scenario (paths relative to `root`), compile the truck, generate the course, run it twice and demand identical hashes.
fn run_scenario(root: &Path, scenario: &str) -> Result<Outcome, String> {
    let setup = Setup::load(root, scenario, None)?;
    let def = w5k_forge::compile::parse_def(&read(root, &setup.sc.vehicle)?)?;
    let extras = w5k_forge::compile::parse_extras(&read(root, &setup.sc.extras)?)?;
    let compiled =
        w5k_forge::compile::compile(&def, &extras).map_err(|e| format!("FORGE refused {}: {e:?}", setup.sc.vehicle))?;
    let run = setup.run_checked(&compiled.rig)?;
    Ok(Outcome {
        run,
        rig: compiled.rig,
        hull_size_m: compiled.hull_size_m,
        vehicle_id: def.id,
        course_name: setup.course_name,
        course_seed: setup.course_seed,
    })
}

/// Entry point for `w5k scenario mule-course`.
pub fn mule_course(args: &[String]) -> Result<(), String> {
    let opt = |key: &str| args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str);
    let out = Path::new(opt("--out").ok_or(USAGE)?);
    let Outcome { run: a, rig, hull_size_m, vehicle_id, course_name, course_seed } =
        run_scenario(Path::new("."), opt("--scenario").unwrap_or(DEFAULT_SCENARIO))?;
    let frame_every = a.frame_every_ticks;

    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let replay = ReplayFile {
        header: ReplayHeader {
            version: REPLAY_VERSION,
            scenario: format!("mule-course: {} ({})", rig.id, a.stop_reason),
            frame_dt_s: f64::from(frame_every) / TICK_HZ,
            vehicles: vec![VehicleHeader {
                name: vehicle_id,
                rig_id: rig.id.clone(),
                joint_names: rig.joint_names(),
                contact_names: rig.contact_names(),
                livery: None,
            }],
            world: WorldHeader { course: course_name, seed: course_seed, terrain: None },
            state_hashes: a.hashes.clone(),
        },
        frames: a.frames,
    };
    w5k_replay::write_json(&out.join("replay.json"), &replay)?;
    w5k_replay::write_bin(&out.join("replay.w5kr"), &replay)?;
    let render = w5k_forge::render::render_rig(&rig, hull_size_m);
    std::fs::write(out.join("rig.json"), serde_json::to_string(&render).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for (name, text) in [
        ("speed.csv", &a.speed),
        ("rpm.csv", &a.rpm),
        ("gear.csv", &a.gear),
        ("track.csv", &a.track),
        ("loads.csv", &a.loads),
    ] {
        std::fs::write(out.join(name), text).map_err(|e| format!("cannot write {name}: {e}"))?;
    }
    println!(
        "{}: {}; reached {:.1} m of {:.1} m of road in {:.1} s, top speed {:.1} km/h, worst cross-track {:.2} m, gears used {:?}, fuel {:.2} kg, {} frames",
        rig.id,
        a.stop_reason,
        a.stopped_at_s_m,
        a.route_m,
        a.time_s,
        scalar::ms_to_kmh(a.max_speed_m_s),
        a.max_cross_track_m,
        a.gears_used,
        a.fuel_kg,
        replay.frames.len()
    );
    println!("wrote {}/replay.w5kr, replay.json, rig.json and the CSV traces", out.display());
    Ok(())
}

pub(crate) fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path)).map_err(|e| format!("cannot read {path}: {e}"))
}

/// One complete run: build the chassis and the powertrain, drive the scripted driver until a stop rule fires.
fn simulate(
    sc: &MuleCourse,
    rig: &PhysRig,
    tuning: &ChassisTuning,
    world: &dyn WorldQuery,
    path: &RoadPath,
) -> Result<RunResult, String> {
    // Geometry the pursuit needs: wheelbase and the rear (unsteered) reference axle, from the rig's stations.
    let unsteered: Vec<f64> = rig.stations.iter().filter(|s| s.steer.is_none()).map(|s| s.rest_pos_m.z).collect();
    let steered: Vec<(f64, f64)> =
        rig.stations.iter().filter_map(|s| s.steer.as_ref().map(|d| (s.rest_pos_m.z, d.max_angle_rad))).collect();
    if unsteered.is_empty() || steered.is_empty() {
        return Err(format!("{}: the pursuit driver needs both steered and unsteered axles", rig.id));
    }
    let z_ref = unsteered.iter().sum::<f64>() / unsteered.len() as f64;
    let z_front = steered.iter().map(|s| s.0).sum::<f64>() / steered.len() as f64;
    let wheelbase_m = z_ref - z_front;
    let max_steer_rad = steered.iter().map(|s| s.1).fold(0.0, f64::max);
    if wheelbase_m <= 0.0 || max_steer_rad <= 0.0 {
        return Err(format!("{}: wheelbase {wheelbase_m} m or steer lock {max_steer_rad} rad is not positive", rig.id));
    }

    let route_end_m = sc.route_end_m.as_ref().map_or(path.length_m(), |p| p.v.min(path.length_m()));
    // Start with the rear axle on the first road point, heading along the road (the hull datum is `z_ref` ahead of the axle line).
    let yaw0 = path.heading_at(0.0, sc.lookahead_base_m.v);
    let axle_offset = Quat::from_yaw(yaw0).rotate(Vec3::new(0.0, 0.0, z_ref));
    let (x0, z0) = (path.pts[0].0 - axle_offset.x, path.pts[0].1 - axle_offset.z);
    let mut chassis = WheeledChassis::new(rig, tuning, world, x0, z0, yaw0)
        .map_err(|e| format!("CHASSIS refused {}: {e:?}", rig.id))?;
    let mut drive = Powertrain::new(&rig.drivetrain, &Tunings::shipped())
        .map_err(|e| format!("DRIVE refused the drivetrain of {}: {e}", rig.id))?;

    let dt = 1.0 / TICK_HZ;
    let ticks = (sc.max_seconds.v * TICK_HZ) as u64;
    let every = u64::from(sc.frame_every_ticks.max(1));
    let stall_ticks = (sc.stall_window_s.v * TICK_HZ) as usize;
    let mut r = RunResult {
        frame_every_ticks: sc.frame_every_ticks.max(1),
        frames: Vec::new(),
        samples: Vec::new(),
        hashes: Vec::new(),
        final_hash: 0,
        stop_reason: format!("ran the full {} s", sc.max_seconds.v),
        stopped_at_s_m: 0.0,
        route_m: route_end_m,
        time_s: 0.0,
        max_speed_m_s: 0.0,
        max_cross_track_m: 0.0,
        gears_used: Vec::new(),
        fuel_kg: 0.0,
        speed: String::from("t_s,speed_kmh,target_kmh,throttle_x10,brake_x10\n"),
        rpm: String::from("t_s,engine_rpm,gear_x1000\n"),
        gear: String::from("t_s,gear\n"),
        track: String::from("t_s,s_m,cross_track_mm,steer_x10,height_m\n"),
        loads: String::from("t_s"),
    };
    for s in &chassis.stations {
        let _ = write!(r.loads, ",{}_kn", s.name);
    }
    r.loads.push('\n');
    let mut progress: Vec<f64> = Vec::new();
    let mut prev_vy: Option<(f64, f64)> = None;
    let mut hint = 0usize;
    let mut s_now = 0.0;
    for n in 0..ticks {
        // --- the scripted driver -------------------------------------------------------------------------------------------------
        let v = chassis.forward_speed_m_s();
        let datum = chassis.datum_m();
        let fwd = chassis.hull.rot.rotate(Vec3::FORWARD);
        let rgt = chassis.hull.rot.rotate(Vec3::RIGHT);
        let rear = datum + chassis.hull.rot.rotate(Vec3::new(0.0, 0.0, z_ref));
        let (idx, s_fix, cross_m) = path.locate(rear.x, rear.z, hint);
        hint = idx;
        s_now = s_fix;
        let remaining_m = route_end_m - sc.finish_stop_margin_m.v - s_now;

        let ld = sc.lookahead_base_m.v + sc.lookahead_per_m_s.v * v.abs();
        let (tx, tz) = path.point_at((s_now + ld).min(route_end_m));
        let (dx, dz) = (tx - rear.x, tz - rear.z);
        let (ex, ey) = (dx * fwd.x + dz * fwd.z, dx * rgt.x + dz * rgt.z);
        let curvature = 2.0 * ey / (ex * ex + ey * ey).max(PLAN_EPS); // pure pursuit, positive = right
        let steer = scalar::clamp(scalar::atan(wheelbase_m * curvature) / max_steer_rad, -1.0, 1.0);

        let kappa = path.upcoming_curvature(s_now, sc.curvature_window_m.v);
        let v_corner = if kappa > PLAN_EPS { scalar::sqrt(sc.max_lateral_accel_m_s2.v / kappa) } else { f64::INFINITY };
        let v_end = scalar::sqrt(2.0 * sc.final_decel_m_s2.v * remaining_m.max(0.0));
        let target = sc.cruise_speed_m_s.v.min(v_corner).min(v_end);
        let finishing = remaining_m <= 0.0;
        let throttle = if finishing { 0.0 } else { scalar::clamp((target - v) * sc.speed_gain_per_m_s.v, 0.0, 1.0) };
        let brake = if finishing {
            1.0
        } else {
            scalar::clamp((v - target - sc.brake_deadband_m_s.v) * sc.brake_gain_per_m_s.v, 0.0, 1.0)
        };
        let inputs = DriveInputs { throttle, brake, steer, gear: GearRequest::Auto, ..DriveInputs::default() };

        if n % every == 0 {
            let tel = drive.telemetry();
            let (t0, vy) = (chassis.time_s, chassis.hull.vel_m_s.y);
            let vert_accel = prev_vy.map_or(0.0, |(pt, pv)| (vy - pv) / (t0 - pt));
            prev_vy = Some((t0, vy));
            r.samples.push(Sample {
                t_s: t0,
                s_m: s_now,
                speed_m_s: v,
                target_m_s: target,
                cross_track_m: cross_m,
                steer_angle_rad: chassis.stations.iter().map(|s| s.steer_rad.abs()).fold(0.0, f64::max),
                lat_accel_m_s2: v * chassis.hull.omega_rad_s().y,
                vert_accel_m_s2: vert_accel,
                gear: tel.gear,
                fuel_kg: tel.fuel_used_kg,
                tyre_limit: chassis.stations.iter().any(|s| s.report.contact.saturated),
            });
        }

        // --- the physics ---------------------------------------------------------------------------------------------------------
        chassis.tick(dt, &inputs, world, &mut drive);
        if !chassis.is_finite() {
            return Err(format!("non-finite state at t = {:.3} s", chassis.time_s));
        }
        let t = chassis.time_s;
        let tel = drive.telemetry();
        r.max_speed_m_s = r.max_speed_m_s.max(v.abs());
        r.max_cross_track_m = r.max_cross_track_m.max(cross_m);
        if !r.gears_used.contains(&tel.gear) {
            r.gears_used.push(tel.gear);
        }
        progress.push(s_now);

        if n % every == 0 {
            r.frames.push(Frame {
                t_s: t,
                vehicles: vec![vehicle_frame(&chassis, rig, &tel)],
                events: Vec::new(),
                projectiles: Vec::new(),
            });
            let _ = writeln!(
                r.speed,
                "{t:.4},{:.3},{:.3},{:.2},{:.2}",
                scalar::ms_to_kmh(v),
                scalar::ms_to_kmh(if target.is_finite() { target } else { 0.0 }),
                throttle * PEDAL_PLOT_SCALE,
                brake * PEDAL_PLOT_SCALE
            );
            let _ = writeln!(r.rpm, "{t:.4},{:.1},{}", tel.engine_rpm, f64::from(tel.gear) * GEAR_PLOT_SCALE);
            let _ = writeln!(r.gear, "{t:.4},{}", tel.gear);
            let _ = writeln!(
                r.track,
                "{t:.4},{s_now:.2},{:.1},{:.2},{:.2}",
                cross_m * MM_PER_M,
                steer * PEDAL_PLOT_SCALE,
                datum.y
            );
            let _ = write!(r.loads, "{t:.4}");
            for s in &chassis.stations {
                let _ = write!(r.loads, ",{:.3}", s.report.contact.fz_n * KN_PER_N);
            }
            r.loads.push('\n');
        }
        if n % TICK_HZ as u64 == 0 {
            r.hashes.push(state_hash(&chassis, &drive));
        }
        r.time_s = t;
        r.fuel_kg = tel.fuel_used_kg;

        // --- stop rules, each says why ---------------------------------------------------------------------------------------------
        if finishing && v.abs() < sc.stopped_below_m_s.v {
            r.stop_reason = format!(
                "finished: stopped at {:.1} m along the road, {:.1} m from its end",
                s_now,
                route_end_m - s_now
            );
            break;
        }
        if cross_m > sc.off_road_limit_m.v {
            r.stop_reason = format!("left the road: {cross_m:.1} m from the centreline at {s_now:.1} m along it");
            break;
        }
        if progress.len() > stall_ticks && s_now - progress[progress.len() - 1 - stall_ticks] < sc.stall_progress_m.v {
            r.stop_reason = format!(
                "stalled at {s_now:.1} m along the road (under {} m of progress in {} s; speed {:.2} m/s, gear {}, engine {:.0} rpm)",
                sc.stall_progress_m.v, sc.stall_window_s.v, v, tel.gear, tel.engine_rpm
            );
            break;
        }
    }
    r.stopped_at_s_m = s_now;
    r.final_hash = state_hash(&chassis, &drive);
    Ok(r)
}

const GEAR_PLOT_SCALE: f64 = 1e3; // const-ok: draws the gear number on the rpm axis as 1000 rpm per gear
const PEDAL_PLOT_SCALE: f64 = 10.0; // const-ok: draws a 0..1 pedal on the km/h axis as 0..10

fn state_hash(chassis: &WheeledChassis, drive: &Powertrain) -> u64 {
    let mut h = StateHasher::new();
    chassis.hash_state(&mut h);
    drive.hash_state(&mut h);
    h.finish()
}

/// The replay frame: hull datum pose, joints in `PhysRig::joint_names()` order (spin, steer of steered stations, travel), one contact per
/// station. (Same layout as `w5k chassis strip`; CHASSIS owns that copy, see docs/swarm/requests/arch-glue-gaps.md.)
pub(crate) fn vehicle_frame(c: &WheeledChassis, rig: &PhysRig, telemetry: &DriveTelemetry) -> VehicleFrame {
    let mut joints: Vec<f32> = c.stations.iter().map(|s| s.spin_angle_rad as f32).collect();
    joints.extend(
        c.stations.iter().zip(&rig.stations).filter(|(_, d)| d.steer.is_some()).map(|(s, _)| s.steer_rad as f32),
    );
    joints.extend(c.stations.iter().map(|s| s.travel_m as f32));
    let contacts = c
        .stations
        .iter()
        .map(|s| {
            let k = &s.report.contact;
            let flags =
                u8::from(k.fz_n > 0.0) | (u8::from(k.saturated) << 1) | (u8::from(s.report.suspension.at_limit) << 3);
            ContactFrame {
                flags,
                normal_force_n: k.fz_n as f32,
                sinkage_m: k.sinkage_m as f32,
                slip: k.slip_ratio as f32,
                material: s.report.material.0,
            }
        })
        .collect();
    VehicleFrame {
        vehicle: 0,
        pos_m: c.datum_m(),
        rot: c.hull.rot,
        lin_vel_m_s: c.hull.vel_m_s,
        ang_vel_rad_s: c.hull.omega_rad_s(),
        joints,
        engine_rpm: telemetry.engine_rpm as f32,
        gear: telemetry.gear,
        contacts,
        ledger_n: Vec::new(),
        limiting: LimitingFactor::None,
        weapons: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_mule_with_the_real_powertrain_drives_the_slice_road_and_stops_at_its_end() {
        let o = run_scenario(&root(), DEFAULT_SCENARIO).expect("scenario runs, twice, with identical hashes");
        let r = &o.run;
        assert!(r.stop_reason.starts_with("finished"), "{}", r.stop_reason);
        assert!(r.stopped_at_s_m > r.route_m - 3.0, "stopped at {} of {} m", r.stopped_at_s_m, r.route_m);
        assert!(r.max_cross_track_m < 3.0, "worst cross-track {} m (the road is 6 m wide)", r.max_cross_track_m);
        assert!(r.gears_used.len() > 1, "the automatic box never shifted: {:?}", r.gears_used);
        assert!(r.frames.iter().all(|f| f.vehicles.iter().all(|v| v.is_finite())));
    }

    #[test]
    fn a_circular_road_of_radius_r_has_curvature_one_over_r() {
        let r = 50.0; // const-ok: test radius, m
        let road: Vec<(f64, f64, f64)> = (0..=100) // const-ok: test sampling
            .map(|k| {
                let a = f64::from(k) * 0.01; // const-ok: 1 rad over 100 samples
                (r * scalar::sin(a), -r * (1.0 - scalar::cos(a)), 0.0)
            })
            .collect();
        let path = RoadPath::new(&road).expect("path");
        let kappa = path.upcoming_curvature(10.0, 12.0); // const-ok: test station and window
        assert!((kappa - 1.0 / r).abs() < 0.02 / r, "curvature {kappa}, expected {}", 1.0 / r);
        // const-ok: 2 % tolerance
    }
}
