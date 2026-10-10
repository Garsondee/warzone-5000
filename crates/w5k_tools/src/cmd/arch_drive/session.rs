//! The simulated half of `w5k drive`: the scene (course, road, chassis tuning), the garage of drivable vehicles and one `Session`
//! (vehicle + powertrain + assists) that is stepped one fixed tick at a time. Nothing here knows about sockets or the wall clock.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde::Deserialize;
use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::{DrivePort, Param, VehicleFrame, WorldQuery};
use w5k_drive::powertrain::{Powertrain, Tunings};
use w5k_math::{scalar, Quat, StateHasher, Vec3};
use w5k_world::grid::GridWorld;

use super::assist::{Assist, AssistTuning, Obs, Raw, Recovery};
use crate::cmd::arch_course::{read, vehicle_frame};

/// Wording of the one-shot event after a recovery (a reset or an auto-recover).
pub(crate) const BACK_ON_ROAD: &str = "back on the road";
const JSON_DIGITS: f64 = 1e4; // const-ok: stream numbers are rounded to 0.1 mm / 0.0001 rad, plenty for drawing and short on the wire

/// `content/physics/arch/drive_assist.ron`: the live loop's numbers and (see `assist.rs`) the kid assists.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename = "DriveAssist")]
pub(crate) struct Tuning {
    pub chassis_tuning: String,
    pub publish_every_ticks: u32,
    pub max_ticks_per_wake: u32,
    pub input_timeout_s: Param,
    pub idle_finish_s: Param,
    pub road_probe_m: Param,
    pub assist: AssistTuning,
}

impl Tuning {
    pub(crate) fn load(root: &Path, file: &str) -> Result<Tuning, String> {
        let t: Tuning = ron::from_str(&read(root, file)?).map_err(|e| format!("{file}: {e}"))?;
        for (name, p) in [
            ("input_timeout_s", &t.input_timeout_s),
            ("idle_finish_s", &t.idle_finish_s),
            ("road_probe_m", &t.road_probe_m),
        ] {
            p.check(&format!("drive_assist.{name}"))?;
        }
        t.assist.check()?;
        if t.publish_every_ticks == 0 || t.max_ticks_per_wake == 0 {
            return Err("drive_assist: publish_every_ticks and max_ticks_per_wake must be at least 1".into());
        }
        Ok(t)
    }
}

/// The road centreline, `(x, z, y)` per point as WORLD generates it.
pub(crate) struct Road {
    pts: Vec<(f64, f64, f64)>,
}

impl Road {
    pub(crate) fn new(pts: Vec<(f64, f64, f64)>) -> Result<Road, String> {
        if pts.len() < 2 {
            return Err("the road has fewer than two points".into());
        }
        Ok(Road { pts })
    }

    /// Index of the road point nearest to `(x, z)` in plan.
    pub(crate) fn nearest(&self, x: f64, z: f64) -> usize {
        let d2 = |p: &(f64, f64, f64)| (p.0 - x) * (p.0 - x) + (p.1 - z) * (p.1 - z);
        (0..self.pts.len()).fold(0, |best, k| if d2(&self.pts[k]) < d2(&self.pts[best]) { k } else { best })
    }

    /// Plan position and heading (yaw, positive = nose left) at point `i`, looking `probe_m` further along the road.
    pub(crate) fn pose(&self, i: usize, probe_m: f64) -> (f64, f64, f64) {
        let p = self.pts[i];
        let far = (i + 1..self.pts.len()).find(|&k| scalar::hypot(self.pts[k].0 - p.0, self.pts[k].1 - p.1) >= probe_m);
        let (a, b) = match far {
            Some(k) => (p, self.pts[k]),
            None => (self.pts[self.pts.len().saturating_sub(2).min(i)], self.pts[self.pts.len() - 1]),
        };
        (p.0, p.1, scalar::atan2(-(b.0 - a.0), -(b.1 - a.1)))
    }
}

/// Everything a session needs that is not the vehicle.
pub(crate) struct Scene {
    pub world: GridWorld,
    pub road: Road,
    pub tuning: ChassisTuning,
    pub assist_tuning: Tuning,
    /// `terrain.json` (format `w5k-terrain-1`), served as `/api/world`.
    pub terrain_json: String,
    pub course: String,
    pub seed: u64,
}

impl Scene {
    /// Generate the course; the terrain file comes from WORLD's own exporter so the two never drift apart.
    pub(crate) fn load(root: &Path, tuning_file: &str, course_file: &str) -> Result<Scene, String> {
        let assist_tuning = Tuning::load(root, tuning_file)?;
        let def = w5k_world::course::CourseDef::from_ron(&read(root, course_file)?)?;
        let generated = w5k_world::course::generate(&def)?;
        // One folder per call: scenes may load in parallel threads (tests do).
        static LOADS: AtomicUsize = AtomicUsize::new(0);
        let tmp = std::env::temp_dir().join(format!(
            "w5k-drive-{}-{}",
            std::process::id(),
            LOADS.fetch_add(1, Ordering::Relaxed)
        ));
        let args = [
            "export".to_string(),
            root.join(course_file).to_string_lossy().into_owned(),
            "--out".into(),
            tmp.to_string_lossy().into_owned(),
        ];
        crate::cmd::world::run(&args)?;
        let terrain_json = std::fs::read_to_string(tmp.join("terrain.json")).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&tmp);
        let tuning = ChassisTuning::from_ron(&read(root, &assist_tuning.chassis_tuning)?)
            .map_err(|e| format!("{}: {e}", assist_tuning.chassis_tuning))?;
        Ok(Scene {
            world: generated.world,
            road: Road::new(generated.road)?,
            tuning,
            assist_tuning,
            terrain_json,
            course: def.name,
            seed: def.seed,
        })
    }
}

/// A drivable vehicle: the compiled rig plus what the page needs to list and draw it.
pub(crate) struct Car {
    pub id: String,
    pub name: String,
    pub mass_kg: f64,
    pub wheelbase_m: f64,
    pub rig: PhysRig,
    pub render_json: String,
    /// Longitudinal position of the rear (unsteered) axle line in the hull frame, m (the reference point placed on the road).
    z_ref_m: f64,
}

impl Car {
    /// Compile a vehicle file (with its `.extras.ron` sidecar); `Err` for anything the wheeled chassis would refuse.
    pub(crate) fn load(root: &Path, scene: &Scene, file: &str) -> Result<Car, String> {
        let e = crate::cmd::arch_compare::load_entry(root, file)?;
        let c = w5k_forge::compile::compile(&e.def, &e.extras).map_err(|r| format!("FORGE refused {file}: {r:?}"))?;
        let rig = c.rig;
        let z_of = |steered: bool| -> Vec<f64> {
            rig.stations.iter().filter(|s| s.steer.is_some() == steered).map(|s| s.rest_pos_m.z).collect()
        };
        let (rear, front) = (z_of(false), z_of(true));
        if rear.is_empty() || front.is_empty() {
            return Err(format!("{file}: a drivable vehicle needs steered and unsteered axles"));
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let (z_ref_m, z_front) = (mean(&rear), mean(&front));
        WheeledChassis::new(&rig, &scene.tuning, &scene.world, 0.0, 0.0, 0.0).map_err(|r| format!("{file}: {r:?}"))?;
        let render = w5k_forge::render::render_rig(&rig, c.hull_size_m);
        Ok(Car {
            id: e.def.id,
            name: e.def.name,
            mass_kg: rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>(),
            wheelbase_m: z_ref_m - z_front,
            render_json: serde_json::to_string(&render).map_err(|e| e.to_string())?,
            rig,
            z_ref_m,
        })
    }

    /// Every drivable `*.ron` (not `*.extras.ron`) in `dir`, sorted by file name; files that do not compile are named on stderr and skipped.
    pub(crate) fn load_dir(root: &Path, scene: &Scene, dir: &str) -> Result<Vec<Arc<Car>>, String> {
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir))
            .map_err(|e| format!("cannot read {dir}: {e}"))?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.ends_with(".ron") && !n.ends_with(".extras.ron"))
            .collect();
        names.sort();
        let mut cars = Vec::new();
        for n in names {
            match Car::load(root, scene, &format!("{dir}/{n}")) {
                Ok(c) => cars.push(Arc::new(c)),
                Err(e) => eprintln!("w5k drive: skipping {n}: {e}"),
            }
        }
        Ok(cars)
    }
}

/// One vehicle on the scene, stepped at the fixed tick.
pub(crate) struct Session {
    pub car: Arc<Car>,
    scene: Arc<Scene>,
    chassis: WheeledChassis,
    drive: Powertrain,
    pub assist: Assist,
    /// Ticks since the vehicle was selected (survives recoveries); time is `ticks / TICK_HZ`.
    pub ticks: u64,
    /// Set by a recovery, taken by the next published frame.
    pub event: Option<&'static str>,
    /// Why the last automatic recovery happened (`None` for a reset or a numerical failure).
    pub last_recovery: Option<Recovery>,
}

impl Session {
    pub(crate) fn new(scene: Arc<Scene>, car: Arc<Car>, assist_on: bool) -> Result<Session, String> {
        let (chassis, drive) = Session::build(&scene, &car, 0)?;
        Ok(Session {
            car,
            scene,
            chassis,
            drive,
            assist: Assist::new(assist_on),
            ticks: 0,
            event: None,
            last_recovery: None,
        })
    }

    /// A fresh chassis and powertrain standing on road point `idx`, rear axle on the point, facing along the road.
    fn build(scene: &Scene, car: &Car, idx: usize) -> Result<(WheeledChassis, Powertrain), String> {
        let (x, z, yaw) = scene.road.pose(idx, scene.assist_tuning.road_probe_m.v);
        let off = Quat::from_yaw(yaw).rotate(Vec3::new(0.0, 0.0, car.z_ref_m));
        let chassis = WheeledChassis::new(&car.rig, &scene.tuning, &scene.world, x - off.x, z - off.z, yaw)
            .map_err(|e| format!("CHASSIS refused {}: {e:?}", car.id))?;
        let drive = Powertrain::new(&car.rig.drivetrain, &Tunings::shipped())
            .map_err(|e| format!("DRIVE refused the drivetrain of {}: {e}", car.id))?;
        Ok((chassis, drive))
    }

    /// Put the vehicle back on the road point nearest to where it is, upright, at rest, facing along the road.
    pub(crate) fn recover(&mut self) {
        let p = self.chassis.datum_m();
        let idx = if p.x.is_finite() && p.z.is_finite() { self.scene.road.nearest(p.x, p.z) } else { 0 };
        // The rig built once already, so a refusal here would be a bug: keep the old state rather than crash a child's session.
        if let Ok((c, d)) = Session::build(&self.scene, &self.car, idx) {
            self.chassis = c;
            self.drive = d;
            self.assist.reset();
            self.event = Some(BACK_ON_ROAD);
        }
    }

    pub(crate) fn speed_m_s(&self) -> f64 {
        self.chassis.forward_speed_m_s()
    }

    pub(crate) fn time_s(&self) -> f64 {
        self.ticks as f64 / TICK_HZ
    }

    /// What the assists look at.
    fn obs(&self) -> Obs {
        let t = &self.scene.assist_tuning.assist;
        let up = self.chassis.hull.rot.rotate(Vec3::UP);
        let (p, (lo, hi)) = (self.chassis.datum_m(), self.scene.world.bounds());
        let m = t.bounds_margin_m.v;
        let inside =
            p.x > lo.x + m && p.x < hi.x - m && p.z > lo.z + m && p.z < hi.z - m && p.y > lo.y - t.fall_depth_m.v;
        Obs { speed_m_s: self.speed_m_s(), tilt_rad: scalar::acos(scalar::clamp(up.y, -1.0, 1.0)), off_map: !inside }
    }

    /// One tick with the raw pedals and wheel of the player.
    pub(crate) fn step(&mut self, raw: &Raw) {
        let (dt, scene) = (1.0 / TICK_HZ, self.scene.clone());
        let t = &scene.assist_tuning.assist;
        let obs = self.obs();
        let inputs = self.assist.apply(raw, &obs, t, dt);
        self.chassis.tick(dt, &inputs, &scene.world, &mut self.drive);
        self.ticks += 1;
        let why = self.assist.watch(&raw.clamped(), &self.obs(), t, dt);
        if why.is_some() || !self.chassis.is_finite() {
            self.last_recovery = why;
            if why.is_none() {
                eprintln!("w5k drive: non-finite state, recovering");
            }
            self.recover();
        }
    }

    pub(crate) fn frame(&self) -> VehicleFrame {
        vehicle_frame(&self.chassis, &self.car.rig, &self.drive.telemetry())
    }

    /// Hash of the whole simulated state (the replay's per-second chain).
    pub(crate) fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        self.chassis.hash_state(&mut h);
        self.drive.hash_state(&mut h);
        h.finish()
    }

    /// The stream frame (see `docs/swarm/requests/arch-drive-protocol.md`); the event is taken, so it is sent once.
    pub(crate) fn stream_json(&mut self) -> String {
        let (f, q) = (self.frame(), self.chassis.hull.rot);
        let r = |x: f64| (x * JSON_DIGITS).round() / JSON_DIGITS + 0.0; // + 0.0 turns -0.0 into 0.0
        let contacts: Vec<_> = self
            .car
            .rig
            .contact_names()
            .iter()
            .zip(&f.contacts)
            .map(|(n, c)| {
                serde_json::json!({"name": n, "normal_n": r(f64::from(c.normal_force_n)), "in_contact": c.flags & 1 == 1})
            })
            .collect();
        let mut v = serde_json::json!({
            "t_s": r(self.time_s()),
            "vehicle": self.car.id,
            "pos_m": [r(f.pos_m.x), r(f.pos_m.y), r(f.pos_m.z)],
            "rot": [r(q.w), r(q.x), r(q.y), r(q.z)],
            "joints": f.joints.iter().map(|j| r(f64::from(*j))).collect::<Vec<_>>(),
            "engine_rpm": r(f64::from(f.engine_rpm)),
            "gear": f.gear,
            "speed_m_s": r(self.speed_m_s()),
            "contacts": contacts,
            "assist": {"on": self.assist.on, "message": self.assist.message},
        });
        if let Some(e) = self.event.take() {
            v["message_event"] = e.into();
        }
        v.to_string()
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! A flat 400 m world with a straight road down -Z, and the three garage vehicles compiled once.
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use super::*;

    pub(crate) const TUNING: &str = "content/physics/arch/drive_assist.ron";

    pub(crate) fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    pub(crate) fn flat_scene(edit: impl FnOnce(&mut Tuning)) -> Arc<Scene> {
        let mut assist_tuning = Tuning::load(&root(), TUNING).expect("tuning");
        edit(&mut assist_tuning);
        let road = (0..=300).map(|k| (0.0, 150.0 - f64::from(k), 0.0)).collect(); // 1 m points down -Z
        let mut materials = w5k_contract::world::MaterialTable::default();
        materials.push(w5k_contract::world::Material {
            name: "asphalt".into(),
            mu_peak: 0.9,
            mu_slide: 0.8,
            rolling_coeff: 0.015,
            roughness_rms_m: 0.0,
            soil: None,
        });
        Arc::new(Scene {
            world: GridWorld::from_arrays(401, vec![0.0; 401 * 401], vec![0; 401 * 401], materials),
            road: Road::new(road).expect("road"),
            tuning: ChassisTuning::from_ron(&read(&root(), &assist_tuning.chassis_tuning).expect("file"))
                .expect("tuning"),
            assist_tuning,
            terrain_json: String::new(),
            course: "flat".into(),
            seed: 0,
        })
    }

    pub(crate) fn car(id: &str) -> Arc<Car> {
        static CARS: OnceLock<Vec<Arc<Car>>> = OnceLock::new();
        let cars =
            CARS.get_or_init(|| Car::load_dir(&root(), &flat_scene(|_| ()), "content/vehicles/game").expect("garage"));
        cars.iter().find(|c| c.id == id).cloned().unwrap_or_else(|| panic!("no vehicle {id}"))
    }

    pub(crate) fn session(id: &str, assist_on: bool, edit: impl FnOnce(&mut Tuning)) -> Session {
        Session::new(flat_scene(edit), car(id), assist_on).expect("session")
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    const GARAGE: [&str; 3] = ["scout_4x4", "mule_4x4", "hauler_4x4"];

    fn run(s: &mut Session, raw: Raw, secs: f64) {
        for _ in 0..(secs * TICK_HZ) as u32 {
            s.step(&raw);
        }
    }

    fn full_throttle() -> Raw {
        Raw { throttle: 1.0, ..Raw::default() }
    }

    #[test]
    fn full_throttle_on_flat_ground_holds_every_garage_vehicle_under_the_speed_cap() {
        for id in GARAGE {
            let mut s = session(id, true, |_| ());
            let cap = s.scene.assist_tuning.assist.speed_cap_m_s.v;
            let mut top = 0.0_f64;
            for _ in 0..(30.0 * TICK_HZ) as u32 {
                s.step(&full_throttle());
                top = top.max(s.speed_m_s());
            }
            eprintln!("{id}: top speed {top:.2} m/s ({:.1} km/h) against a cap of {cap} m/s", scalar::ms_to_kmh(top));
            assert!(top < cap + 0.5, "{id} reached {top} m/s against a cap of {cap}");
            assert!(top > 0.8 * cap, "{id} only reached {top} m/s: the cap should be a ceiling, not a brake");
        }
    }

    #[test]
    fn without_assists_full_throttle_goes_past_the_kid_cap() {
        let mut s = session("scout_4x4", false, |_| ());
        run(&mut s, full_throttle(), 12.0);
        assert!(s.speed_m_s() > 1.5 * s.scene.assist_tuning.assist.speed_cap_m_s.v, "{}", s.speed_m_s());
    }

    #[test]
    fn with_no_input_the_vehicle_stops_gently_and_stays_put() {
        let mut s = session("mule_4x4", true, |_| ());
        run(&mut s, full_throttle(), 8.0);
        assert!(s.speed_m_s() > 5.0, "{}", s.speed_m_s());
        let (v0, p0, t0) = (s.speed_m_s(), s.chassis.datum_m(), s.time_s());
        let mut stopped_at = None;
        for _ in 0..(12.0 * TICK_HZ) as u32 {
            s.step(&Raw::default());
            if stopped_at.is_none() && s.speed_m_s().abs() < 0.05 {
                stopped_at = Some((s.time_s() - t0, (s.chassis.datum_m() - p0).length()));
            }
        }
        eprintln!("released at {v0:.2} m/s: stopped after {stopped_at:?} (s, m)");
        let at = s.chassis.datum_m();
        assert!(s.speed_m_s().abs() < 0.05, "still rolling at {} m/s", s.speed_m_s());
        run(&mut s, Raw::default(), 6.0);
        let moved = (s.chassis.datum_m() - at).length();
        assert!(moved < 0.2, "crept {moved} m in 6 s with no pedal");
    }

    #[test]
    fn a_rolled_hull_is_back_on_the_road_upright_within_two_seconds() {
        let mut s = session("scout_4x4", true, |_| ());
        run(&mut s, Raw::default(), 1.0);
        s.chassis.hull.rot = Quat::from_roll(2.0); // 115 degrees: on its side and then some
        let started = s.ticks;
        let mut done = None;
        for _ in 0..(2.0 * TICK_HZ) as u32 {
            s.step(&Raw::default());
            if s.event.is_some() {
                done = Some(s.ticks - started);
                break;
            }
        }
        let n = done.expect("not recovered within 2 s");
        assert_eq!(s.last_recovery, Some(Recovery::RolledOver));
        assert!(n as f64 / TICK_HZ >= 0.99, "recovered after only {n} ticks: the dwell is 1 s");
        let up = s.chassis.hull.rot.rotate(Vec3::UP);
        assert!(up.y > 0.99, "not upright: {up:?}");
        assert!(s.chassis.datum_m().x.abs() < 1.0, "not on the road (the line x = 0): {:?}", s.chassis.datum_m());
    }

    #[test]
    fn a_vehicle_held_below_the_stuck_speed_with_the_throttle_down_is_recovered_after_3_s() {
        // The speed cap is a crawl, so full throttle cannot move the vehicle for long: the stuck rule is what fires. A clutch that launches at
        // the engine's torque peak lets the vehicle surge to about 1 m/s before the cap's throttle easing and brake catch it, so the stuck
        // speed is set above the surge (the default 0.3 m/s is for a vehicle that is really held).
        let mut s = session("scout_4x4", true, |t| {
            t.assist.speed_cap_m_s.v = 0.1;
            t.assist.stuck_speed_m_s.v = 2.0;
        });
        let mut at = None;
        for _ in 0..(5.0 * TICK_HZ) as u32 {
            s.step(&full_throttle());
            if s.event.is_some() {
                at = Some(s.time_s());
                break;
            }
        }
        let t = at.expect("never recovered");
        assert_eq!(s.last_recovery, Some(Recovery::Stuck));
        assert!((2.95..3.2).contains(&t), "recovered at {t} s");
    }

    #[test]
    fn leaving_the_terrain_puts_the_vehicle_back_on_the_nearest_road_point_facing_along_it() {
        let mut s = session("mule_4x4", true, |_| ());
        run(&mut s, Raw::default(), 1.0);
        s.chassis.hull.pos_m = Vec3::new(500.0, 1.0, -20.0); // far outside the 400 m world
        s.step(&Raw::default());
        assert_eq!(s.last_recovery, Some(Recovery::OffMap));
        let p = s.chassis.datum_m();
        assert!(p.x.abs() < 1.0 && (p.z + 20.0).abs() < 6.0, "put back at {p:?}, the nearest road point is at z = -20");
        let fwd = s.chassis.hull.rot.rotate(Vec3::FORWARD);
        assert!(fwd.z < -0.99, "not facing along the road (down -Z): {fwd:?}");
    }

    #[test]
    fn a_reset_recovers_even_with_assists_off() {
        let mut s = session("scout_4x4", false, |_| ());
        s.chassis.hull.pos_m = Vec3::new(30.0, 1.0, 0.0); // 30 m off the road
        s.recover();
        assert_eq!(s.event, Some(BACK_ON_ROAD));
        assert!(s.chassis.datum_m().x.abs() < 1.0);
    }

    #[test]
    fn the_reverse_request_backs_the_vehicle_up() {
        let mut s = session("scout_4x4", true, |_| ());
        run(&mut s, Raw { throttle: 0.8, reverse: true, ..Raw::default() }, 4.0);
        assert!(s.speed_m_s() < -0.5, "speed {}", s.speed_m_s());
    }

    #[test]
    fn road_poses_face_along_the_road() {
        let road =
            Road::new(vec![(0.0, 10.0, 0.0), (0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (20.0, 0.0, 0.0)]).expect("road");
        assert_eq!(road.nearest(0.5, 9.0), 0);
        assert!(road.pose(0, 4.0).2.abs() < 1e-9, "-Z is yaw 0");
        let east = road.pose(2, 4.0).2;
        assert!((east + std::f64::consts::FRAC_PI_2).abs() < 1e-9, "+X is yaw -90 degrees, got {east}");
        assert!((road.pose(3, 4.0).2 - east).abs() < 1e-9, "the last point reuses the heading of the last segment");
    }

    #[test]
    fn the_tuning_file_loads_and_the_cap_is_25_km_h() {
        let t = Tuning::load(&root(), TUNING).expect("loads and every Param checks");
        assert!((scalar::ms_to_kmh(t.assist.speed_cap_m_s.v) - 25.0).abs() < 0.2);
    }

    #[test]
    fn the_simulation_loop_runs_faster_than_real_time() {
        let mut s = session("hauler_4x4", true, |_| ());
        let started = std::time::Instant::now();
        run(&mut s, full_throttle(), 10.0);
        let factor = 10.0 / started.elapsed().as_secs_f64();
        eprintln!("real-time factor of the heaviest vehicle: {factor:.1}x");
        assert!(factor > 1.0, "{factor}x: the machine cannot keep up in real time");
    }
}
