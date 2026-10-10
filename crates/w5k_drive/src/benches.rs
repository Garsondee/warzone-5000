//! The named benches of `w5k drive bench` (the CLI in `w5k_tools` compiles the vehicles, calls these and writes the files): the dyno, the
//! speed per gear, a flat-out launch, a long descent and a steady cruise, each on the real [`Powertrain`] and a lumped point mass.
//! Each returns a [`Report`]: a summary line and a table (first column x, the others series). No file I/O here.

use serde::Deserialize;
use w5k_contract::ports::DrivePort;
use w5k_contract::rig::PhysRig;
use w5k_contract::{DriveInputs, GearRequest, Param};

use crate::bench::LumpedVehicle;
use crate::engine::Engine;
use crate::powertrain::{Powertrain, Tunings};

const RPM_TO_RAD_S: f64 = core::f64::consts::PI / 30.0; // const-ok: unit conversion, mathematical
const KMH: f64 = 3.6; // const-ok: unit conversion, m/s to km/h
const STEP_S: f64 = 1.0 / 240.0; // const-ok: the benches' fixed step, a numerical choice (one chassis substep)
const STEPS_PER_S: usize = 240; // const-ok: the same step, as a count

/// The world's constants (`content/physics/drive/bench.ron`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchWorld {
    pub gravity_m_s2: Param,
    pub air_density_kg_m3: Param,
    pub rolling_resistance_coeff: Param,
    pub fuel_density_kg_m3: Param,
    pub cruise_kmh: Param,
    pub descent_grade_rad: Param,
}

impl BenchWorld {
    /// Parse and check the file's text.
    pub fn parse(text: &str) -> Result<BenchWorld, String> {
        let w: BenchWorld = ron::from_str(text).map_err(|e| format!("bench.ron: {e}"))?;
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

/// What a bench found: one summary line, and a table to write as `<file>.csv`.
pub struct Report {
    pub summary: String,
    pub file: String,
    pub header: &'static str,
    pub rows: Vec<Vec<f64>>,
}

/// One vehicle on the bench: its compiled rig, the real powertrain and a lumped body of its mass.
pub struct Bed {
    pub id: String,
    pub rig: PhysRig,
    pub pt: Powertrain,
    pub veh: LumpedVehicle,
}

impl Bed {
    pub fn new(rig: PhysRig, world: &BenchWorld) -> Result<Bed, String> {
        let pt = Powertrain::new(&rig.drivetrain, &Tunings::shipped())?;
        let wheel = rig.stations.iter().find(|s| s.drive_output.is_some()).ok_or("no driven station")?.wheel.clone();
        let mass = rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
        let mut veh = LumpedVehicle::new(mass, wheel.radius_m, wheel.inertia_kg_m2, pt.output_count());
        veh.rolling_coeff = world.rolling_resistance_coeff.v;
        veh.drag_n_s2_m2 = 0.5 * world.air_density_kg_m3.v * rig.aero.drag_coeff * rig.aero.frontal_area_m2;
        veh.gravity_m_s2 = world.gravity_m_s2.v;
        Ok(Bed { id: rig.id.clone(), rig, pt, veh })
    }
}

/// The dyno: full-load torque, power and specific fuel consumption (at the map's best load) against rpm.
pub fn engine(b: &Bed) -> Result<Report, String> {
    let d = &b.rig.drivetrain.engine;
    let e = Engine::new(d, &Tunings::shipped().engine)?;
    let (mut rows, mut peak_t, mut peak_p) = (Vec::new(), (0.0, 0.0), (0.0, 0.0));
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
    let summary = format!(
        "{}: peak torque {:.0} N m at {:.0} rpm, peak power {:.1} kW at {:.0} rpm",
        b.id, peak_t.0, peak_t.1, peak_p.0, peak_p.1
    );
    Ok(Report { summary, file: format!("engine_{}.csv", b.id), header: "rpm,torque_nm,power_kw,bsfc_g_kwh", rows })
}

/// Speed in each gear at idle, at the (capped) upshift point and at the redline.
pub fn shift(b: &Bed) -> Report {
    let d = &b.rig.drivetrain;
    let kmh = |rpm: f64, g: usize| {
        rpm * RPM_TO_RAD_S * b.veh.wheel_radius_m / (b.pt.gear_ratio(g).unwrap_or(1.0) * b.pt.driveline_ratio()) * KMH
    };
    let mut rows = Vec::new();
    let mut summary = format!("{}: gear  overall  km/h@idle  km/h@upshift  km/h@redline", b.id);
    for g in 1..=d.gearbox.forward_ratios.len() {
        let r = vec![
            g as f64,
            b.pt.gear_ratio(g).unwrap_or(1.0) * b.pt.driveline_ratio(),
            kmh(d.engine.idle_rpm, g),
            kmh(d.gearbox.shift.upshift_rpm, g),
            kmh(d.engine.redline_rpm, g),
        ];
        summary += &format!("\n  {:>4} {:>8.2} {:>10.1} {:>13.1} {:>13.1}", g, r[1], r[2], r[3], r[4]);
        rows.push(r);
    }
    Report {
        summary,
        file: format!("shift_{}.csv", b.id),
        header: "gear,overall_ratio,kmh_at_idle,kmh_at_upshift,kmh_at_redline",
        rows,
    }
}

/// Flat out from rest for 60 s: the acceleration trace and the times to 32 and 48 km/h.
pub fn launch(mut b: Bed) -> Report {
    let go = DriveInputs { throttle: 1.0, ..Default::default() };
    let (mut rows, mut t32, mut t48) = (Vec::new(), None, None);
    for k in 0..(60 * STEPS_PER_S) {
        let t = k as f64 * STEP_S;
        b.veh.step(STEP_S, &mut b.pt, &go);
        let v = b.veh.speed_m_s * KMH;
        t32 = t32.or((v >= 32.0).then_some(t)); // const-ok: the brief's 0-32 km/h benchmark
        t48 = t48.or((v >= 48.0).then_some(t)); // const-ok: the proving ground's 0-48 km/h benchmark
        if k % (STEPS_PER_S / 4) == 0 {
            let tel = b.pt.telemetry();
            rows.push(vec![t, v, tel.engine_rpm, f64::from(tel.gear)]);
        }
    }
    let fmt = |t: Option<f64>| t.map_or("not reached".to_string(), |t| format!("{t:.1} s"));
    let summary = format!(
        "{}: 0-32 km/h {}, 0-48 km/h {}, {:.0} km/h after 60 s",
        b.id,
        fmt(t32),
        fmt(t48),
        b.veh.speed_m_s * KMH
    );
    Report { summary, file: format!("launch_{}.csv", b.id), header: "t_s,speed_kmh,engine_rpm,gear", rows }
}

/// A 300 s descent at a held speed on the vehicle's own brakes and gearbox: disc temperature and the pedal it takes.
pub fn brake(mut b: Bed, world: &BenchWorld) -> Report {
    b.veh.grade_rad = -world.descent_grade_rad.v;
    let hold = 10.0; // const-ok: the held descent speed, m/s: a driver's choice on a mountain road
    b.veh.speed_m_s = hold;
    let (mut rows, mut peak) = (Vec::new(), 0.0_f64);
    for k in 0..(300 * STEPS_PER_S) {
        let pedal = (0.2 + 0.6 * (b.veh.speed_m_s - hold)).clamp(0.0, 1.0); // const-ok: a feed-forward and a trim, the driver
        b.veh.step(STEP_S, &mut b.pt, &DriveInputs { brake: pedal, ..Default::default() });
        let tel = b.pt.telemetry();
        let temp = tel.brake_temps_k.iter().copied().fold(0.0, f64::max);
        peak = peak.max(temp);
        if k % STEPS_PER_S == 0 {
            rows.push(vec![k as f64 * STEP_S, b.veh.speed_m_s, temp, pedal, f64::from(tel.gear)]);
        }
    }
    let fade = b.rig.drivetrain.brakes.first().map_or(0.0, |x| x.fade_start_k);
    let summary = format!(
        "{}: 300 s down {:.0} degrees at {hold} m/s: hottest disc {peak:.0} K (fade starts at {fade:.0} K)",
        b.id,
        world.descent_grade_rad.v.to_degrees()
    );
    Report { summary, file: format!("brake_{}.csv", b.id), header: "t_s,speed_m_s,disc_temp_k,pedal,gear", rows }
}

/// Fuel at a steady cruise on level ground.
pub struct Fuel {
    pub summary: String,
    pub cruise_kmh: f64,
    pub kg_per_km: f64,
    pub l_per_100km: f64,
    pub range_km: f64,
}

/// A PI driver settles for 90 s, then fuel and distance are integrated for 120 s. `tank_l` gives a range (NaN without a tank).
pub fn fuel(mut b: Bed, world: &BenchWorld, tank_l: Option<f64>) -> Result<Fuel, String> {
    let target = world.cruise_kmh.v / KMH;
    let (mut integral, mut start) = (0.0, None);
    for k in 0..(210 * STEPS_PER_S) {
        let err = target - b.veh.speed_m_s;
        integral = (integral + 0.05 * err * STEP_S).clamp(0.0, 1.0); // const-ok: driver gains
        let throttle = if k < 20 * STEPS_PER_S { 1.0 } else { (0.4 * err + integral).clamp(0.0, 1.0) }; // const-ok: driver gains and the launch
        b.veh.step(STEP_S, &mut b.pt, &DriveInputs { throttle, gear: GearRequest::Auto, ..Default::default() });
        if k == 90 * STEPS_PER_S {
            start = Some((b.pt.telemetry().fuel_used_kg, b.veh.distance_m));
        }
    }
    let (f0, d0) = start.ok_or("bench too short")?;
    let (fuel_kg, km) = (b.pt.telemetry().fuel_used_kg - f0, (b.veh.distance_m - d0) / 1000.0); // const-ok: m to km
    let kg_per_km = fuel_kg / km;
    let l_per_100km = kg_per_km / world.fuel_density_kg_m3.v * 1000.0 * 100.0; // const-ok: m3 to L and per 100 km
    let range_km = tank_l.map_or(f64::NAN, |l| l * world.fuel_density_kg_m3.v / 1000.0 / kg_per_km); // const-ok: L to m3
    let cruise_kmh = b.veh.speed_m_s * KMH;
    let tank = tank_l.map_or(String::new(), |l| format!("; a {l} L tank lasts {range_km:.0} km"));
    let summary =
        format!("{}: {cruise_kmh:.1} km/h cruise burns {kg_per_km:.4} kg/km = {l_per_100km:.1} L/100 km{tank}", b.id);
    Ok(Fuel { summary, cruise_kmh, kg_per_km, l_per_100km, range_km })
}
