//! `w5k drive`: the command line of lane DRIVE (only that lane edits this file).
//!
//! `w5k drive bench <engine|shift|launch|brake|fuel> [--vehicle a.ron,b.ron,...] [--out DIR] [--tank-l N]` runs one bench per vehicle
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

use w5k_contract::rig::PhysRig;
use w5k_drive::benches::{self, Bed, BenchWorld, Report};

const USAGE: &str =
    "usage: w5k drive bench <engine|shift|launch|brake|fuel> [--vehicle a.ron,b.ron,...] [--out DIR] [--tank-l N]";
const GARAGE_FILES: [&str; 3] = [
    "content/vehicles/game/scout_4x4.ron",
    "content/vehicles/game/mule_4x4.ron",
    "content/vehicles/game/hauler_4x4.ron",
];
const WORLD_FILE: &str = "content/physics/drive/bench.ron";

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
}
