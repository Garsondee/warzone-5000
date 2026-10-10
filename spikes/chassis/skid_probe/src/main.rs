//! Throwaway: the proving skidpad loop by hand on a vehicle, printing what limits the speed near the end.
use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::testing::FlatPlane;
use w5k_contract::{DriveInputs, DrivePort, GearRequest};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let root = "../../../";
    let rd = |p: &str| std::fs::read_to_string(format!("{root}{p}")).unwrap();
    let def = w5k_forge::compile::parse_def(&rd(&format!("content/vehicles/game/{}.ron", a[1]))).unwrap();
    let ex = w5k_forge::compile::parse_extras(&rd(&format!("content/vehicles/game/{}.extras.ron", a[1]))).unwrap();
    let rig = w5k_forge::compile::compile(&def, &ex).unwrap().rig;
    let t = ChassisTuning::from_ron(&rd("content/physics/chassis/tuning.ron")).unwrap();
    let w = FlatPlane::new();
    let z: Vec<f64> = rig.stations.iter().map(|s| s.rest_pos_m.z).collect();
    let wb = z.iter().cloned().fold(f64::MIN, f64::max) - z.iter().cloned().fold(f64::MAX, f64::min);
    let lock = rig.stations.iter().filter_map(|s| s.steer.as_ref()).map(|d| d.max_angle_rad).fold(0.0, f64::max);
    let steer = ((wb / 10.0f64).atan() / lock).min(1.0);
    let mut c = WheeledChassis::new(&rig, &t, &w, 0.0, 0.0, 0.0).unwrap();
    let mut d = w5k_drive::powertrain::Powertrain::new(&rig.drivetrain, &w5k_drive::powertrain::Tunings::shipped()).unwrap();
    c.set_forward_speed(4.0);
    let dt = 1.0 / 60.0;
    for n in 0..(60.0 * 60.0) as usize {
        let time = n as f64 * dt;
        let v = c.forward_speed_m_s();
        let throttle = ((4.0 + 0.15 * time - v) * 0.5).clamp(0.0, 1.0);
        let inp = DriveInputs { gear: GearRequest::Auto, throttle, steer, ..DriveInputs::default() };
        c.tick(dt, &inp, &w, &mut d);
        if n % 60 == 0 && time > 10.0 {
            let tel = d.telemetry();
            let ay = c.hull.vel_m_s.length() * c.yaw_rate_rad_s().abs();
            let fx: f64 = c.stations.iter().map(|s| s.report.contact.fx_n).sum();
            let minw = c.stations.iter().map(|s| s.report.contact.fz_n).fold(f64::MAX, f64::min);
            let sat = c.stations.iter().filter(|s| s.report.contact.saturated).count();
            let slip: Vec<String> = c.stations.iter().map(|s| format!("{:.2}", s.report.contact.slip_angle_rad)).collect();
            println!("t {time:5.1} v {v:5.2} target {:5.2} thr {throttle:.2} gear {} rpm {:5.0} ay {:.2} g  sum fx {fx:6.0} N  lightest {minw:6.0} N  saturated {sat}  alpha {slip:?}", 4.0 + 0.15 * time, tel.gear, tel.engine_rpm, ay / 9.80665);
        }
    }
}
