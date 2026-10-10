//! `w5k drive`: the command line of lane DRIVE (only that lane edits this file).
//!
//! `w5k drive bench <engine|shift|launch|brake|fuel|skidpad> [--vehicle a.ron,b.ron,...] [--out DIR] [--tank-l N]` runs one bench per vehicle
//! (the garage's three trucks by default) on DRIVE's real powertrain and a lumped point-mass vehicle (`w5k_drive::bench`), prints a summary
//! and writes CSVs (first column x, the others series: `w5k viewer plot file.csv --out chart.png` draws them):
//! * `engine`: the dyno curve, torque, power and specific fuel consumption against rpm (`engine_<id>.csv`);
//! * `shift`: speed per gear at idle, at the upshift point and at the redline (`shift_<id>.csv`);
//! * `launch`: a flat-out acceleration trace, with the 0-32 and 0-48 km/h times (`launch_<id>.csv`);
//! * `brake`: a long descent at a held speed, the disc temperature and the pedal it takes (`brake_<id>.csv`);
//! * `fuel`: consumption at a steady cruise on level ground, and the range of a tank (`fuel.csv`).
//!
//! The world's constants (gravity, air density, rolling resistance, cruise speed) are `Param`s in `content/physics/drive/bench.ron`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use w5k_chassis::bench::{skidpad as chassis_skidpad, Skidpad};
use w5k_chassis::tuning::ChassisTuning;
use w5k_contract::ports::DrivePort;
use w5k_contract::rig::{DiffKind, DriveNode, PhysRig};
use w5k_contract::{DriveInputs, DriveTelemetry, ShaftState};
use w5k_drive::benches::{self, Bed, BenchWorld, Report};
use w5k_drive::powertrain::{Powertrain, Tunings};

const USAGE: &str =
    "usage: w5k drive bench <engine|shift|launch|brake|fuel|skidpad> [--vehicle a.ron,b.ron,...] [--out DIR] [--tank-l N]";
const GARAGE_FILES: [&str; 3] = [
    "content/vehicles/game/scout_4x4.ron",
    "content/vehicles/game/mule_4x4.ron",
    "content/vehicles/game/hauler_4x4.ron",
];
const WORLD_FILE: &str = "content/physics/drive/bench.ron";
const CHASSIS_TUNING_FILE: &str = "content/physics/chassis/tuning.ron";

/// The repository root: the working directory when the command runs from it, else (under `cargo test`, which runs in the crate) two levels up.
fn root() -> PathBuf {
    if Path::new(WORLD_FILE).exists() {
        PathBuf::from(".")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
}

fn load_world() -> Result<BenchWorld, String> {
    let text = std::fs::read_to_string(root().join(WORLD_FILE)).map_err(|e| format!("{WORLD_FILE}: {e}"))?;
    BenchWorld::parse(&text)
}

fn compile(path: &str) -> Result<PhysRig, String> {
    let e = super::arch_compare::load_entry(&root(), path)?;
    w5k_forge::compile::compile(&e.def, &e.extras).map(|c| c.rig).map_err(|r| format!("FORGE refused {path}: {r:?}"))
}

fn write_csv(dir: &Path, name: &str, header: &str, rows: &[Vec<f64>]) -> Result<(), String> {
    let mut text = format!("{header}\n");
    for r in rows {
        let cells: Vec<String> = r.iter().map(|v| format!("{v:.4}")).collect();
        let _ = writeln!(text, "{}", cells.join(","));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("  wrote {}", path.display());
    Ok(())
}

fn emit(r: &Report, out: &Path) -> Result<(), String> {
    println!("{}", r.summary);
    write_csv(out, &r.file, r.header, &r.rows)
}

/// A powertrain that counts how often its gear changes (a hunting box shows up as a high count in a steady turn).
struct GearLog {
    inner: Powertrain,
    last: i8,
    changes: usize,
}

impl DrivePort for GearLog {
    fn output_count(&self) -> usize {
        self.inner.output_count()
    }
    fn step(&mut self, dt_s: f64, inputs: &DriveInputs, shafts: &[ShaftState], torque_nm_out: &mut [f64]) {
        self.inner.step(dt_s, inputs, shafts, torque_nm_out);
        let g = self.inner.telemetry().gear;
        if g != self.last {
            self.changes += 1;
            self.last = g;
        }
    }
    fn telemetry(&self) -> DriveTelemetry {
        self.inner.telemetry()
    }
    fn hash_state(&self, h: &mut w5k_math::StateHasher) {
        self.inner.hash_state(h);
    }
}

/// Set the centre and axle differentials of a four-wheel-drive tree (a root differential over two axle differentials); a two-wheel-drive
/// root is an axle differential.
fn set_diffs(node: &mut DriveNode, centre: (DiffKind, f64), axle: (DiffKind, f64)) {
    if let DriveNode::Diff { kind, bias, children, .. } = node {
        if children.iter().all(|c| matches!(c, DriveNode::Diff { .. })) {
            (*kind, *bias) = centre;
            children.iter_mut().for_each(|c| set_diffs(c, centre, axle));
        } else {
            (*kind, *bias) = axle;
        }
    }
}

/// What a skidpad run found.
struct Skid {
    max_lat_g: f64,
    shifts: usize,
    end_speed_m_s: f64,
}

/// The skidpad of ARCH's proving runner (constant steer for the circle, speed ramped) on `rig` with the given centre and axle differentials,
/// with the gear changes counted. `ramp` and `max_s` override the world's (a zero ramp holds the start speed: a steady turn).
fn skid(
    rig: &PhysRig,
    world: &BenchWorld,
    centre: (DiffKind, f64),
    axle: (DiffKind, f64),
    start_m_s: f64,
    ramp: f64,
    max_s: f64,
) -> Result<Skid, String> {
    let mut rig = rig.clone();
    set_diffs(&mut rig.drivetrain.driveline, centre, axle);
    let text =
        std::fs::read_to_string(root().join(CHASSIS_TUNING_FILE)).map_err(|e| format!("{CHASSIS_TUNING_FILE}: {e}"))?;
    let tuning = ChassisTuning::from_ron(&text).map_err(|e| format!("{CHASSIS_TUNING_FILE}: {e:?}"))?;
    let z = rig.stations.iter().map(|s| s.rest_pos_m.z);
    let wheelbase = z.clone().fold(f64::MIN, f64::max) - z.fold(f64::MAX, f64::min);
    let lock = rig.stations.iter().filter_map(|s| s.steer.as_ref()).map(|d| d.max_angle_rad).fold(0.0, f64::max);
    let pad = Skidpad {
        steer: w5k_math::scalar::clamp(w5k_math::scalar::atan(wheelbase / world.skid_radius_m.v) / lock, 0.0, 1.0), // Ackermann angle of the circle as a share of full lock
        start_speed_m_s: start_m_s,
        ramp_m_s2: ramp,
        speed_gain_per_m_s: world.skid_speed_gain_per_m_s.v,
        warmup_s: world.skid_warmup_s.v,
        max_s,
        slide_out_frac: world.skid_slide_out_frac.v,
    };
    let mut log = GearLog { inner: Powertrain::new(&rig.drivetrain, &Tunings::shipped())?, last: 1, changes: 0 };
    let flat = super::arch_proving::flat_world();
    let run =
        chassis_skidpad(&rig, &tuning, flat.as_ref(), &mut log, &pad).map_err(|e| format!("CHASSIS refused: {e:?}"))?;
    Ok(Skid {
        max_lat_g: run.max_lateral_acc_m_s2 / world.gravity_m_s2.v,
        shifts: log.changes,
        end_speed_m_s: run.points.last().map_or(0.0, |p| p.speed_m_s),
    })
}

/// The skidpad with each centre differential kind on every vehicle (axle differentials open): the diff as a design lever.
fn skidpad_bench(rigs: &[PhysRig], world: &BenchWorld, out: &Path) -> Result<(), String> {
    let kinds = [
        ("open", DiffKind::Open, 1.0),
        ("limited slip x3", DiffKind::LimitedSlip, 3.0),
        ("locked", DiffKind::Locked, 1.0),
    ];
    let mut rows = Vec::new();
    for (i, rig) in rigs.iter().enumerate() {
        for (j, (name, kind, bias)) in kinds.iter().enumerate() {
            let r = skid(
                rig,
                world,
                (*kind, *bias),
                (DiffKind::Open, 1.0),
                world.skid_start_speed_m_s.v,
                world.skid_ramp_m_s2.v,
                world.skid_max_s.v,
            )?;
            println!(
                "{}: centre {name}: {:.3} g, ended at {:.1} m/s, {} gear changes",
                rig.id, r.max_lat_g, r.end_speed_m_s, r.shifts
            );
            rows.push(vec![i as f64, j as f64, r.max_lat_g, r.end_speed_m_s, r.shifts as f64]);
        }
    }
    write_csv(
        out,
        "skidpad_diffs.csv",
        "vehicle_index,centre_diff(0 open 1 lsd 2 locked),max_lat_g,end_speed_m_s,gear_changes",
        &rows,
    )
}

fn bench(args: &[String]) -> Result<(), String> {
    let what = args.first().ok_or(USAGE)?.as_str();
    let (mut vehicles, mut out, mut tank) =
        (GARAGE_FILES.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(), PathBuf::from("out/drive-bench"), None);
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        let v = it.next().ok_or(format!("{a} needs a value; {USAGE}"))?;
        match a.as_str() {
            "--vehicle" => vehicles = v.split(',').map(str::to_string).collect(),
            "--out" => out = PathBuf::from(v),
            "--tank-l" => tank = Some(v.parse::<f64>().map_err(|e| format!("--tank-l: {e}"))?),
            _ => return Err(format!("unknown option {a}; {USAGE}")),
        }
    }
    let world = load_world()?;
    if what == "skidpad" {
        let rigs = vehicles.iter().map(|v| compile(v)).collect::<Result<Vec<_>, _>>()?;
        return skidpad_bench(&rigs, &world, &out);
    }
    let mut fuel_rows = Vec::new();
    for v in &vehicles {
        let b = Bed::new(compile(v)?, &world)?;
        match what {
            "engine" => emit(&benches::engine(&b)?, &out)?,
            "shift" => emit(&benches::shift(&b), &out)?,
            "launch" => emit(&benches::launch(b), &out)?,
            "brake" => emit(&benches::brake(b, &world), &out)?,
            "fuel" => {
                let f = benches::fuel(b, &world, tank)?;
                println!("{}", f.summary);
                fuel_rows.push(vec![fuel_rows.len() as f64, f.cruise_kmh, f.kg_per_km, f.l_per_100km, f.range_km]);
            }
            _ => return Err(USAGE.to_string()),
        }
    }
    if what == "fuel" {
        write_csv(&out, "fuel.csv", "vehicle_index,cruise_kmh,kg_per_km,l_per_100km,range_km", &fuel_rows)?;
    }
    Ok(())
}

/// Entry point for `w5k drive <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("bench") => bench(&args[1..]),
        _ => Err(USAGE.to_string()),
    }
}

/// Every vehicle in the garage must be shiftable by the powertrain: the shift schedule has to suit the engine it is fitted to.
/// These run here, not in `w5k_drive`, because compiling a garage vehicle needs FORGE, which a lane crate must not depend on.
#[cfg(test)]
mod tests {
    use std::path::Path;

    use w5k_contract::ports::DrivePort;
    use w5k_contract::DriveInputs;
    use w5k_drive::bench::LumpedVehicle;
    use w5k_drive::powertrain::{Powertrain, Tunings};

    const GARAGE: [&str; 3] = ["scout_4x4", "mule_4x4", "hauler_4x4"];

    fn compile(name: &str) -> w5k_contract::PhysRig {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/vehicles/game");
        let read = |f: String| std::fs::read_to_string(root.join(&f)).unwrap_or_else(|e| panic!("{f}: {e}"));
        let def = w5k_forge::compile::parse_def(&read(format!("{name}.ron"))).expect("def parses");
        let extras = w5k_forge::compile::parse_extras(&read(format!("{name}.extras.ron"))).expect("extras parse");
        w5k_forge::compile::compile(&def, &extras).expect("FORGE compiles it").rig
    }

    /// A flat-out run from rest on a long straight, as a lumped vehicle built from the compiled rig.
    fn flat_out(name: &str, secs: f64) -> (Vec<(f64, i8)>, f64, f64, f64, usize) {
        let rig = compile(name);
        let mut pt = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).expect("powertrain builds");
        let wheel = rig.stations.iter().find(|s| s.drive_output.is_some()).expect("a driven station").wheel.clone();
        let mass = rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
        let mut v = LumpedVehicle::new(mass, wheel.radius_m, wheel.inertia_kg_m2, pt.output_count());
        v.rolling_coeff = 0.015;
        v.drag_n_s2_m2 = 0.5 * 1.2 * rig.aero.drag_coeff * rig.aero.frontal_area_m2;
        v.gravity_m_s2 = 9.81;
        let dt = 1.0 / 240.0;
        let (mut gears, mut peak_rpm, mut late_accel) = (Vec::new(), 0.0_f64, 0.0_f64);
        let mut v_prev: Option<f64> = None;
        for k in 0..((secs / dt) as usize) {
            v.step(dt, &mut pt, &DriveInputs { throttle: 1.0, ..Default::default() });
            let t = pt.telemetry();
            peak_rpm = peak_rpm.max(t.engine_rpm);
            if gears.last().map(|&(_, g)| g) != Some(t.gear) {
                gears.push((k as f64 * dt, t.gear));
            }
            if k as f64 * dt > secs - 5.0 && k % 240 == 0 {
                if let Some(prev) = v_prev {
                    late_accel = late_accel.max((v.speed_m_s - prev).abs());
                }
                v_prev = Some(v.speed_m_s);
            }
        }
        (gears, v.speed_m_s, peak_rpm, late_accel, rig.drivetrain.gearbox.forward_ratios.len())
    }

    #[test]
    fn every_garage_vehicle_accelerates_flat_out_to_its_top_gear_without_hunting() {
        for name in GARAGE {
            let (gears, v, peak_rpm, _, n_gears) = flat_out(name, 120.0);
            let rig = compile(name);
            // reaches the top gear
            assert_eq!(gears.last().unwrap().1 as usize, n_gears, "{name} ended in the wrong gear: {gears:?}");
            // never shifts down on the way up, and never skips a long way back: a monotone gear history is "no hunting"
            assert!(gears.windows(2).all(|w| w[1].1 > w[0].1), "{name} hunted: {gears:?}");
            // within the limiter
            let redline = rig.drivetrain.engine.redline_rpm;
            assert!(peak_rpm <= redline * 1.005, "{name} reached {peak_rpm} rpm, redline {redline}");
            assert!(v > 10.0, "{name} only reached {v} m/s");
        }
    }

    #[test]
    fn every_garage_vehicle_settles_at_a_top_speed_set_by_power_or_the_limiter() {
        for name in GARAGE {
            let (_, v, _, late_accel, _) = flat_out(name, 200.0);
            assert!(late_accel < 0.5, "{name} is still accelerating at {v} m/s at the end ({late_accel} m/s per s)");
        }
    }

    /// Engine speed and torque with the vehicle held at rest, full throttle in first gear (what a steep start asks for).
    fn stall(name: &str) -> (f64, f64, f64, f64) {
        let rig = compile(name);
        let mut pt = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).unwrap();
        let n = pt.output_count();
        let shafts = vec![w5k_contract::ShaftState { inertia_kg_m2: 1.0e6, ..Default::default() }; n];
        let mut out = vec![0.0; n];
        let go = DriveInputs { throttle: 1.0, gear: w5k_contract::GearRequest::Gear(1), ..Default::default() };
        for _ in 0..(8 * 240) {
            pt.step(1.0 / 240.0, &go, &shafts, &mut out);
        }
        let t = pt.telemetry();
        let curve = &rig.drivetrain.engine.torque_curve;
        let (peak_rpm, peak) = curve.iter().fold((0.0, 0.0), |m: (f64, f64), &(r, q)| if q > m.1 { (r, q) } else { m });
        (t.engine_rpm / peak_rpm, t.engine_torque_nm / peak, out.iter().sum(), peak)
    }

    #[test]
    fn a_clutch_launch_holds_the_engine_at_its_peak_torque_speed_when_the_vehicle_is_held() {
        // a driver pulling away on a steep hill slips the clutch near the engine's torque peak; the scout and hauler have clutches
        for name in ["scout_4x4", "hauler_4x4"] {
            let (speed_frac, torque_frac, wheel_nm, _) = stall(name);
            assert!((speed_frac - 1.0).abs() < 0.03, "{name}: engine at {speed_frac} of its peak-torque speed");
            assert!(torque_frac > 0.97, "{name}: engine torque {torque_frac} of its peak");
            assert!(wheel_nm > 0.0);
        }
    }

    #[test]
    fn a_loaded_truck_on_a_steep_grade_stays_in_first_instead_of_shifting_up_and_rolling_back() {
        let rig = compile("hauler_4x4");
        let mut pt = Powertrain::new(&rig.drivetrain, &Tunings::shipped()).unwrap();
        let wheel = rig.stations.iter().find(|s| s.drive_output.is_some()).unwrap().wheel.clone();
        let mass = rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
        let mut v = LumpedVehicle::new(mass, wheel.radius_m, wheel.inertia_kg_m2, pt.output_count());
        v.rolling_coeff = 0.02;
        v.gravity_m_s2 = 9.81;
        v.grade_rad = w5k_math::scalar::atan(0.25);
        let dt = 1.0 / 240.0;
        let mut gears = Vec::new();
        for k in 0..(10 * 240) {
            v.step(dt, &mut pt, &DriveInputs { throttle: 1.0, ..Default::default() });
            let g = pt.telemetry().gear;
            if gears.last() != Some(&g) {
                gears.push(g);
            }
            assert!(
                k < 3 * 240 || v.speed_m_s > 0.5,
                "rolling back or stalled at {} m/s after {} s",
                v.speed_m_s,
                k / 240
            );
        }
        assert_eq!(gears, vec![1], "the 1-2 shift (0.8 s without drive) on this grade must not happen: {gears:?}");
        assert!(v.distance_m > 20.0, "climbed only {} m", v.distance_m);
    }

    #[test]
    fn cruise_fuel_per_km_is_at_least_the_best_point_bsfc_times_the_road_work_and_not_absurdly_more() {
        let world = super::load_world().unwrap();
        for file in super::GARAGE_FILES {
            let rig = super::compile(file).unwrap();
            let b = super::Bed::new(rig.clone(), &world).unwrap();
            let v = world.cruise_kmh.v / 3.6;
            let road_force =
                world.rolling_resistance_coeff.v * b.veh.mass_kg * world.gravity_m_s2.v + b.veh.drag_n_s2_m2 * v * v;
            let best_kg_per_km = road_force * 1000.0 / 3.6e6 * rig.drivetrain.engine.bsfc_best_g_kwh / 1000.0;
            let kg_km = super::benches::fuel(b, &world, None).unwrap().kg_per_km;
            assert!(kg_km >= best_kg_per_km, "{file}: {kg_km} kg/km is below the thermodynamic floor {best_kg_per_km}");
            assert!(kg_km <= 4.0 * best_kg_per_km, "{file}: {kg_km} kg/km is more than 4 x the floor {best_kg_per_km}");
        }
    }

    #[test]
    fn the_bench_commands_write_their_csv_files() {
        let dir = std::env::temp_dir().join(format!("w5k-drive-bench-{}", std::process::id()));
        let args = |what: &str| {
            vec![
                "bench".to_string(),
                what.to_string(),
                "--vehicle".into(),
                super::GARAGE_FILES[1].into(),
                "--out".into(),
                dir.display().to_string(),
            ]
        };
        for (what, file, header) in [
            ("engine", "engine_mule_4x4.csv", "rpm,torque_nm,power_kw,bsfc_g_kwh"),
            ("shift", "shift_mule_4x4.csv", "gear,overall_ratio"),
            ("launch", "launch_mule_4x4.csv", "t_s,speed_kmh"),
        ] {
            super::run(&args(what)).unwrap();
            let text = std::fs::read_to_string(dir.join(file)).unwrap();
            assert!(text.starts_with(header), "{file}: {}", text.lines().next().unwrap_or(""));
            assert!(text.lines().count() > 3);
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(super::run(&["bench".to_string(), "nonsense".to_string()]).is_err());
    }

    #[test]
    fn a_turn_with_spinning_inner_wheels_does_not_make_the_box_hunt() {
        // the Hauler on the skidpad circle, speed ramped from 4 m/s: the inner wheels unload and spin on the open differentials, which used to
        // inflate the carrier speed the shift map reads (29 gear changes in a ramped run); now it reads the slowest wheel
        let world = super::load_world().unwrap();
        let rig = super::compile(super::GARAGE_FILES[2]).unwrap();
        let open = (w5k_contract::rig::DiffKind::Open, 1.0);
        let r = super::skid(
            &rig,
            &world,
            open,
            open,
            world.skid_start_speed_m_s.v,
            world.skid_ramp_m_s2.v,
            world.skid_max_s.v,
        )
        .unwrap();
        assert!(r.shifts <= 6, "{} gear changes in a slowly accelerating turn", r.shifts);
    }

    #[test]
    fn a_limited_slip_and_a_lock_raise_the_haulers_cornering_speed_on_the_skidpad() {
        use w5k_contract::rig::DiffKind;
        let world = super::load_world().unwrap();
        let rig = super::compile(super::GARAGE_FILES[2]).unwrap();
        let axle = (DiffKind::Open, 1.0);
        let run = |c: (DiffKind, f64)| {
            super::skid(&rig, &world, c, axle, world.skid_start_speed_m_s.v, world.skid_ramp_m_s2.v, world.skid_max_s.v)
                .unwrap()
        };
        let (open, lsd, locked) =
            (run((DiffKind::Open, 1.0)), run((DiffKind::LimitedSlip, 3.0)), run((DiffKind::Locked, 1.0)));
        // the Hauler is power-limited in the turn (its unloaded inner wheels spin): a torque bias or a lock sends the drive to the wheels that grip
        assert!(lsd.max_lat_g > 1.03 * open.max_lat_g, "open {} g, limited slip {} g", open.max_lat_g, lsd.max_lat_g);
        assert!(
            locked.max_lat_g > 1.03 * lsd.max_lat_g,
            "limited slip {} g, locked {} g",
            lsd.max_lat_g,
            locked.max_lat_g
        );
    }
}
