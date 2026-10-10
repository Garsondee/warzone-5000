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

use serde::Deserialize;
use w5k_contract::ports::DrivePort;
use w5k_contract::rig::PhysRig;
use w5k_contract::{DriveInputs, GearRequest, Param};
use w5k_drive::bench::LumpedVehicle;
use w5k_drive::engine::Engine;
use w5k_drive::powertrain::{Powertrain, Tunings};

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
const RPM_TO_RAD_S: f64 = core::f64::consts::PI / 30.0; // const-ok: unit conversion, mathematical
const KMH: f64 = 3.6; // const-ok: unit conversion, m/s to km/h
const STEP_S: f64 = 1.0 / 240.0; // const-ok: the bench's fixed step, a numerical choice (one chassis substep)

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchWorld {
    gravity_m_s2: Param,
    air_density_kg_m3: Param,
    rolling_resistance_coeff: Param,
    fuel_density_kg_m3: Param,
    cruise_kmh: Param,
    descent_grade_rad: Param,
}

impl BenchWorld {
    fn load() -> Result<BenchWorld, String> {
        let text = std::fs::read_to_string(root().join(WORLD_FILE)).map_err(|e| format!("{WORLD_FILE}: {e}"))?;
        let w: BenchWorld = ron::from_str(&text).map_err(|e| format!("{WORLD_FILE}: {e}"))?;
        for (n, p) in [
            ("gravity_m_s2", &w.gravity_m_s2),
            ("air_density_kg_m3", &w.air_density_kg_m3),
            ("rolling_resistance_coeff", &w.rolling_resistance_coeff),
            ("fuel_density_kg_m3", &w.fuel_density_kg_m3),
            ("cruise_kmh", &w.cruise_kmh),
            ("descent_grade_rad", &w.descent_grade_rad),
        ] {
            p.check(n)?;
        }
        Ok(w)
    }
}

/// One vehicle on the bench: its compiled rig, the real powertrain and a lumped body of its mass.
struct Bed {
    id: String,
    rig: PhysRig,
    pt: Powertrain,
    veh: LumpedVehicle,
}

fn bed(rig: PhysRig, world: &BenchWorld) -> Result<Bed, String> {
    let pt = Powertrain::new(&rig.drivetrain, &Tunings::shipped())?;
    let wheel = rig.stations.iter().find(|s| s.drive_output.is_some()).ok_or("no driven station")?.wheel.clone();
    let mass = rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let mut veh = LumpedVehicle::new(mass, wheel.radius_m, wheel.inertia_kg_m2, pt.output_count());
    veh.rolling_coeff = world.rolling_resistance_coeff.v;
    veh.drag_n_s2_m2 = 0.5 * world.air_density_kg_m3.v * rig.aero.drag_coeff * rig.aero.frontal_area_m2;
    veh.gravity_m_s2 = world.gravity_m_s2.v;
    Ok(Bed { id: rig.id.clone(), rig, pt, veh })
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

/// The dyno: full-load torque, power and specific fuel consumption (at the map's best load) against rpm.
fn engine_bench(b: &Bed, out: &Path) -> Result<(), String> {
    let d = &b.rig.drivetrain.engine;
    let e = Engine::new(d, &Tunings::shipped().engine)?;
    let mut rows = Vec::new();
    let (mut peak_t, mut peak_p) = ((0.0, 0.0), (0.0, 0.0));
    let mut rpm = d.idle_rpm;
    while rpm <= d.redline_rpm {
        let w = rpm * RPM_TO_RAD_S;
        let (t, p) = (e.full_load_nm(w), e.power_w(w, 1.0) / 1000.0); // const-ok: W to kW
        if t > peak_t.0 {
            peak_t = (t, rpm);
        }
        if p > peak_p.0 {
            peak_p = (p, rpm);
        }
        rows.push(vec![rpm, t, p, e.bsfc_g_kwh(w, 0.75)]); // const-ok: the map's best load fraction is 0.75
        rpm += (d.redline_rpm - d.idle_rpm) / 40.0; // const-ok: 41 points along the curve
    }
    println!(
        "{}: peak torque {:.0} N m at {:.0} rpm, peak power {:.1} kW at {:.0} rpm",
        b.id, peak_t.0, peak_t.1, peak_p.0, peak_p.1
    );
    write_csv(out, &format!("engine_{}.csv", b.id), "rpm,torque_nm,power_kw,bsfc_g_kwh", &rows)
}

/// Speed in each gear at idle, at the (capped) upshift point and at the redline.
fn shift_bench(b: &Bed, out: &Path) -> Result<(), String> {
    let d = &b.rig.drivetrain;
    let wheel = b.veh.wheel_radius_m;
    let kmh = |rpm: f64, g: usize| {
        rpm * RPM_TO_RAD_S * wheel / (b.pt.gear_ratio(g).unwrap_or(1.0) * b.pt.driveline_ratio()) * KMH
    };
    let mut rows = Vec::new();
    println!("{}: gear  overall  km/h@idle  km/h@upshift  km/h@redline", b.id);
    for g in 1..=d.gearbox.forward_ratios.len() {
        let r = vec![
            g as f64,
            b.pt.gear_ratio(g).unwrap_or(1.0) * b.pt.driveline_ratio(),
            kmh(d.engine.idle_rpm, g),
            kmh(d.gearbox.shift.upshift_rpm, g),
            kmh(d.engine.redline_rpm, g),
        ];
        println!("  {:>4} {:>8.2} {:>10.1} {:>13.1} {:>13.1}", g, r[1], r[2], r[3], r[4]);
        rows.push(r);
    }
    write_csv(
        out,
        &format!("shift_{}.csv", b.id),
        "gear,overall_ratio,kmh_at_idle,kmh_at_upshift,kmh_at_redline",
        &rows,
    )
}

/// Flat out from rest for 60 s: the acceleration trace and the times to 32 and 48 km/h.
fn launch_bench(mut b: Bed, out: &Path) -> Result<(), String> {
    let go = DriveInputs { throttle: 1.0, ..Default::default() };
    let (mut rows, mut t32, mut t48) = (Vec::new(), None, None);
    for k in 0..(60 * 240) {
        let t = f64::from(k) * STEP_S;
        b.veh.step(STEP_S, &mut b.pt, &go);
        let v = b.veh.speed_m_s * KMH;
        t32 = t32.or((v >= 32.0).then_some(t)); // const-ok: the brief's 0-32 km/h benchmark
        t48 = t48.or((v >= 48.0).then_some(t)); // const-ok: the proving ground's 0-48 km/h benchmark
        if k % 60 == 0 {
            let tel = b.pt.telemetry();
            rows.push(vec![t, v, tel.engine_rpm, f64::from(tel.gear)]);
        }
    }
    let fmt = |t: Option<f64>| t.map_or("not reached".to_string(), |t| format!("{t:.1} s"));
    println!("{}: 0-32 km/h {}, 0-48 km/h {}, {:.0} km/h after 60 s", b.id, fmt(t32), fmt(t48), b.veh.speed_m_s * KMH);
    write_csv(out, &format!("launch_{}.csv", b.id), "t_s,speed_kmh,engine_rpm,gear", &rows)
}

/// A long descent at a held speed on the vehicle's own brakes and gearbox: disc temperature and the pedal it takes.
fn brake_bench(mut b: Bed, world: &BenchWorld, out: &Path) -> Result<(), String> {
    b.veh.grade_rad = -world.descent_grade_rad.v;
    let hold = 10.0; // const-ok: the held descent speed, m/s: a driver's choice on a mountain road
    b.veh.speed_m_s = hold;
    let (mut rows, mut peak) = (Vec::new(), 0.0_f64);
    for k in 0..(300 * 240) {
        let brake = (0.2 + 0.6 * (b.veh.speed_m_s - hold)).clamp(0.0, 1.0); // const-ok: a feed-forward and a trim, the driver
        b.veh.step(STEP_S, &mut b.pt, &DriveInputs { brake, ..Default::default() });
        let temp = b.pt.telemetry().brake_temps_k.iter().copied().fold(0.0, f64::max);
        peak = peak.max(temp);
        if k % 240 == 0 {
            rows.push(vec![f64::from(k) * STEP_S, b.veh.speed_m_s, temp, brake, f64::from(b.pt.telemetry().gear)]);
        }
    }
    let fade = b.rig.drivetrain.brakes.first().map_or(0.0, |x| x.fade_start_k);
    println!(
        "{}: 300 s down {:.0} degrees at {hold} m/s: hottest disc {peak:.0} K (fade starts at {fade:.0} K)",
        b.id,
        world.descent_grade_rad.v.to_degrees()
    );
    write_csv(out, &format!("brake_{}.csv", b.id), "t_s,speed_m_s,disc_temp_k,pedal,gear", &rows)
}

/// Consumption at a steady cruise on level ground: a PI driver settles for 90 s, then fuel and distance are integrated for 120 s.
fn fuel_bench(mut b: Bed, world: &BenchWorld, tank_l: Option<f64>) -> Result<Vec<f64>, String> {
    let target = world.cruise_kmh.v / KMH;
    let (mut integral, mut start) = (0.0, None);
    for k in 0..(210 * 240) {
        let err = target - b.veh.speed_m_s;
        integral = (integral + 0.05 * err * STEP_S).clamp(0.0, 1.0); // const-ok: driver gains
        let throttle = if k < 20 * 240 { 1.0 } else { (0.4 * err + integral).clamp(0.0, 1.0) }; // const-ok: driver gains and the launch
        b.veh.step(STEP_S, &mut b.pt, &DriveInputs { throttle, gear: GearRequest::Auto, ..Default::default() });
        if k == 90 * 240 {
            start = Some((b.pt.telemetry().fuel_used_kg, b.veh.distance_m));
        }
    }
    let (f0, d0) = start.ok_or("bench too short")?;
    let (fuel_kg, km) = (b.pt.telemetry().fuel_used_kg - f0, (b.veh.distance_m - d0) / 1000.0); // const-ok: m to km
    let kg_km = fuel_kg / km;
    let l_100 = kg_km / world.fuel_density_kg_m3.v * 1000.0 * 100.0; // const-ok: m3 to L and per 100 km
    let range = tank_l.map_or(f64::NAN, |l| l * world.fuel_density_kg_m3.v / 1000.0 / kg_km);
    println!(
        "{}: {:.1} km/h cruise burns {:.4} kg/km = {:.1} L/100 km{}",
        b.id,
        b.veh.speed_m_s * KMH,
        kg_km,
        l_100,
        tank_l.map_or(String::new(), |l| format!("; a {l} L tank lasts {range:.0} km"))
    );
    Ok(vec![b.veh.speed_m_s * KMH, kg_km, l_100, range])
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
    let world = BenchWorld::load()?;
    let mut fuel_rows = Vec::new();
    for v in &vehicles {
        let b = bed(compile(v)?, &world)?;
        match what {
            "engine" => engine_bench(&b, &out)?,
            "shift" => shift_bench(&b, &out)?,
            "launch" => launch_bench(b, &out)?,
            "brake" => brake_bench(b, &world, &out)?,
            "fuel" => fuel_rows.push(fuel_bench(b, &world, tank)?),
            _ => return Err(USAGE.to_string()),
        }
    }
    if what == "fuel" {
        let rows: Vec<Vec<f64>> = fuel_rows
            .iter()
            .enumerate()
            .map(|(i, r)| std::iter::once(i as f64).chain(r.iter().copied()).collect())
            .collect();
        write_csv(&out, "fuel.csv", "vehicle_index,cruise_kmh,kg_per_km,l_per_100km,range_km", &rows)?;
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
        let world = super::BenchWorld::load().unwrap();
        for file in super::GARAGE_FILES {
            let rig = super::compile(file).unwrap();
            let b = super::bed(rig.clone(), &world).unwrap();
            let v = world.cruise_kmh.v / 3.6;
            let road_force =
                world.rolling_resistance_coeff.v * b.veh.mass_kg * world.gravity_m_s2.v + b.veh.drag_n_s2_m2 * v * v;
            let best_kg_per_km = road_force * 1000.0 / 3.6e6 * rig.drivetrain.engine.bsfc_best_g_kwh / 1000.0;
            let row = super::fuel_bench(b, &world, None).unwrap();
            let kg_km = row[1];
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
