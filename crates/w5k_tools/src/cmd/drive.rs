//! `w5k drive`: the command line of lane DRIVE (only that lane edits this file).

/// Entry point for `w5k drive <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    let _ = args;
    Err("lane DRIVE has no commands yet".to_string())
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
}
