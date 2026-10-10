//! Throwaway: run the CHASSIS skidpad bench on a vehicle with and without tyre load sensitivity; print the run near its limit.
use w5k_chassis::bench::{skidpad, Skidpad};
use w5k_chassis::tuning::ChassisTuning;
use w5k_contract::testing::{ConstantTorquePowertrain, FlatPlane};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let root = "../../../";
    let rd = |p: &str| std::fs::read_to_string(format!("{root}{p}")).unwrap();
    let def = w5k_forge::compile::parse_def(&rd(&format!("content/vehicles/game/{}.ron", a[1]))).unwrap();
    let ex = w5k_forge::compile::parse_extras(&rd(&format!("content/vehicles/game/{}.extras.ron", a[1]))).unwrap();
    let mut rig = w5k_forge::compile::compile(&def, &ex).unwrap().rig;
    let track_scale: f64 = a.get(3).map_or(1.0, |v| v.parse().unwrap());
    for st in rig.stations.iter_mut() {
        st.rest_pos_m.x *= track_scale;
    }
    let steer: f64 = a[2].parse().unwrap();
    for (label, k_mu, k_c) in [("linear", 0.0, 0.0), ("sensitive", 0.15, 0.3)] {
        let mut t = ChassisTuning::from_ron(&rd("content/physics/chassis/tuning.ron")).unwrap();
        t.tyre_mu_load_sensitivity.v = k_mu;
        t.tyre_stiffness_load_sensitivity.v = k_c;
        // ARCH's proving setup: 10 m circle, DRIVE's powertrain, content/physics/arch/proving.ron values
        let _ = steer;
        let z: Vec<f64> = rig.stations.iter().map(|s| s.rest_pos_m.z).collect();
        let wb = z.iter().cloned().fold(f64::MIN, f64::max) - z.iter().cloned().fold(f64::MAX, f64::min);
        let lock = rig.stations.iter().filter_map(|s| s.steer.as_ref()).map(|d| d.max_angle_rad).fold(0.0, f64::max);
        let pad = Skidpad { steer: ((wb / 10.0f64).atan() / lock).min(1.0), start_speed_m_s: 4.0, ramp_m_s2: 0.15, speed_gain_per_m_s: 0.5, warmup_s: 3.0, max_s: 120.0, slide_out_frac: 0.8 };
        let mut d = w5k_drive::powertrain::Powertrain::new(&rig.drivetrain, &w5k_drive::powertrain::Tunings::shipped()).unwrap();
        let _ = ConstantTorquePowertrain::new(1, 0.0, 0.0);
        let r = skidpad(&rig, &t, &FlatPlane::new(), &mut d, &pad).unwrap();
        println!("{label}: max a_y {:.3} g, K {:.4} rad per m/s2, {} points", r.max_lateral_acc_m_s2 / 9.80665, r.understeer_gradient_rad_per_m_s2, r.points.len());
        let n = r.points.len();
        let win = 180usize;
        let med = |e: usize| { let mut w: Vec<f64> = r.points[e + 1 - win..=e].iter().map(|p| p.lateral_acc_m_s2).collect(); w.sort_by(f64::total_cmp); w[win / 2] };
        let (be, bm) = (win - 1..n).map(|e| (e, med(e))).fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
        println!("   ARCH median limit {:.3} g at point {be} of {n}; lightest wheel there {:.0} N; samples with a wheel off the ground: {}", bm / 9.80665, r.points[be].min_wheel_load_n, r.points.iter().filter(|p| p.min_wheel_load_n <= 0.0).count());
        let half_w = (rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()) * 9.80665 / 2.0;
        println!("   half the weight {half_w:.0} N: transfer / (W/2) = 1 means the inner wheels carry nothing");
        for p in r.points.iter().skip(n.saturating_sub(1200)).step_by(80) {
            println!("   v {:5.2} ay {:5.2} yaw {:5.3} steer {:5.3} transfer {:6.0} ({:4.2} of W/2) roll_out {:6.3}", p.speed_m_s, p.lateral_acc_m_s2, p.yaw_rate_rad_s, p.steer_angle_rad, p.lateral_transfer_n, p.lateral_transfer_n / half_w, p.roll_out_rad);
        }
    }
}
