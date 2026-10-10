//! The wheeled assembly against closed-form answers. Tests are physics sentences.

use w5k_chassis::tuning::ChassisTuning;
use w5k_chassis::wheeled::WheeledChassis;
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::testing::{box_truck, ConstantTorquePowertrain, FlatPlane};
use w5k_contract::{DriveInputs, GearRequest, WorldQuery};
use w5k_math::scalar;

fn tuning() -> ChassisTuning {
    ChassisTuning::from_ron(include_str!("../../../content/physics/chassis/tuning.ron")).unwrap()
}

fn rig() -> PhysRig {
    box_truck().0
}

fn drive(r: &PhysRig) -> ConstantTorquePowertrain {
    ConstantTorquePowertrain::new(r.drivetrain.outputs.len(), 600.0, 4000.0)
}

fn coast() -> DriveInputs {
    DriveInputs { gear: GearRequest::Neutral, ..DriveInputs::default() }
}

fn run(c: &mut WheeledChassis, w: &dyn WorldQuery, d: &mut ConstantTorquePowertrain, inp: &DriveInputs, seconds: f64) {
    for _ in 0..(seconds * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, inp, w, d);
        assert!(c.is_finite(), "NaN at t = {}", c.time_s);
    }
}

#[test]
fn truck_rests_at_the_design_ride_height_within_5mm() {
    let (r, w) = (rig(), FlatPlane::new());
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, 0.0, 0.0).unwrap();
    let mut d = drive(&r);
    run(&mut c, &w, &mut d, &coast(), 5.0);
    let sag = c.datum_m().y - r.ride_height_m;
    println!("datum error {sag:.5} m, travels {:?}", c.stations.iter().map(|s| s.travel_m).collect::<Vec<_>>());
    assert!(sag.abs() < 0.005, "datum {sag} m off the design ride height");
    assert!(c.hull.vel_m_s.length() < 1e-3);
}

#[test]
fn axle_loads_match_com_position() {
    let (r, w) = (rig(), FlatPlane::new());
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, 0.0, 0.0).unwrap();
    let mut d = drive(&r);
    run(&mut c, &w, &mut d, &coast(), 5.0);
    // F_front = m g b / L with b the distance from the centre of mass to the rear axle (two axles, statics)
    let m_total = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    let z_com_total = (r.hull.mass_kg * r.hull.com_m.z
        + r.stations.iter().map(|s| s.unsprung_mass_kg * s.rest_pos_m.z).sum::<f64>())
        / m_total;
    let (zf, zr) = (
        r.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MAX, f64::min),
        r.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MIN, f64::max),
    );
    let l = zr - zf;
    let b = zr - z_com_total;
    let front_expect = m_total * scalar::G * b / l;
    let front: f64 = c
        .stations
        .iter()
        .zip(&r.stations)
        .filter(|(_, d)| (d.rest_pos_m.z - zf).abs() < 1e-9)
        .map(|(s, _)| s.report.contact.fz_n)
        .sum();
    let total: f64 = c.stations.iter().map(|s| s.report.contact.fz_n).sum();
    println!("front {front:.1} N expect {front_expect:.1}, total {total:.1} vs {:.1}", m_total * scalar::G);
    assert!((total / (m_total * scalar::G) - 1.0).abs() < 1e-3);
    assert!((front / front_expect - 1.0).abs() < 0.01);
}

fn brake() -> DriveInputs {
    DriveInputs { gear: GearRequest::Neutral, brake: 1.0, ..DriveInputs::default() }
}

fn settled_at(v: f64) -> (PhysRig, FlatPlane, WheeledChassis, ConstantTorquePowertrain) {
    let (r, w) = (rig(), FlatPlane::new());
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, 0.0, 0.0).unwrap();
    let mut d = drive(&r);
    run(&mut c, &w, &mut d, &coast(), 3.0);
    c.set_forward_speed(v);
    (r, w, c, d)
}

#[test]
fn braking_distance_matches_v2_over_2mu_g() {
    let v0 = 20.0;
    let (r, w, mut c, mut d) = settled_at(v0);
    let start = c.datum_m();
    let mut t = 0.0;
    while c.forward_speed_m_s() > 0.05 && t < 20.0 {
        c.tick(1.0 / TICK_HZ, &brake(), &w, &mut d);
        t += 1.0 / TICK_HZ;
    }
    let dist = (c.datum_m() - start).length();
    let ground = w.materials().get(w.material_id_at(0.0, 0.0));
    // the sliding tyres give mu N with mu = mu_peak * mu_scale, and rolling resistance adds (Crr_tyre + Crr_surface) N
    let tyre = r.stations[0].wheel.tyre.as_ref().unwrap();
    let (mu, crr) = (ground.mu_peak * tyre.mu_scale, tyre.rolling_coeff + ground.rolling_coeff);
    let expect = v0 * v0 / (2.0 * (mu + crr) * scalar::G);
    println!("stopped in {dist:.2} m ({t:.2} s), closed form {expect:.2} m");
    // the stop is at the peak friction (the tyre has no slide drop yet); the brake build-up lengthens it slightly, drag shortens it
    assert!((dist / expect - 1.0).abs() < 0.03);
}

#[test]
fn braking_load_transfer_matches_m_a_h_over_l() {
    let (r, w, mut c, mut d) = settled_at(20.0);
    let static_front: f64 = front_load(&r, &c);
    let mut samples = Vec::new();
    for n in 0..90 {
        c.tick(1.0 / TICK_HZ, &brake(), &w, &mut d);
        if n > 45 {
            // after the pitch transient: decel of the whole truck from the hull and wheels sharing it
            samples.push((
                front_load(&r, &c) - static_front,
                -c.hull.acc_m_s2.dot(c.hull.rot.rotate(w5k_math::Vec3::FORWARD)),
            ));
        }
    }
    let (dfz, decel) = samples.iter().fold((0.0, 0.0), |a, s| (a.0 + s.0, a.1 + s.1));
    let (dfz, decel) = (dfz / samples.len() as f64, decel / samples.len() as f64);
    let m_total = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>();
    // centre of mass height above the ground (the unsprung masses sit at wheel-centre height)
    let h = (r.hull.mass_kg * (r.hull.com_m.y + r.ride_height_m)
        + r.stations.iter().map(|s| s.unsprung_mass_kg * (s.rest_pos_m.y + r.ride_height_m)).sum::<f64>())
        / m_total;
    let expect = m_total * decel * h / wheelbase(&r);
    println!("front load gain {dfz:.0} N at {decel:.2} m/s2, closed form {expect:.0} N (h {h:.3} m)");
    assert!((dfz / expect - 1.0).abs() < 0.05);
}

fn wheelbase(r: &PhysRig) -> f64 {
    let z: Vec<f64> = r.stations.iter().map(|s| s.rest_pos_m.z).collect();
    z.iter().cloned().fold(f64::MIN, f64::max) - z.iter().cloned().fold(f64::MAX, f64::min)
}

fn front_load(r: &PhysRig, c: &WheeledChassis) -> f64 {
    let zf = r.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MAX, f64::min);
    c.stations
        .iter()
        .zip(&r.stations)
        .filter(|(_, d)| (d.rest_pos_m.z - zf).abs() < 1e-9)
        .map(|(s, _)| s.report.contact.fz_n)
        .sum()
}

#[test]
fn low_speed_turn_radius_matches_ackermann_l_over_tan_delta() {
    let (r, w, mut c, mut d) = settled_at(2.0);
    let steer = DriveInputs { gear: GearRequest::Auto, throttle: 0.0, steer: -0.5, ..DriveInputs::default() }; // half lock left
                                                                                                               // hold 2 m/s with a little throttle against rolling resistance: just coast, the turn is measured over 3 s
    run(&mut c, &w, &mut d, &steer, 3.0);
    let rear_mid = {
        let zr = r.stations.iter().map(|s| s.rest_pos_m.z).fold(f64::MIN, f64::max);
        c.hull.point_world(w5k_math::Vec3::new(0.0, 0.0, zr - r.hull.com_m.z))
    };
    let radius = c.hull.point_velocity(rear_mid).length() / c.yaw_rate_rad_s().abs();
    let sd = r.stations.iter().find_map(|s| s.steer.clone()).unwrap();
    let outer = 0.5 * sd.max_angle_rad;
    let half_track = r.stations.iter().map(|s| s.rest_pos_m.x.abs()).fold(0.0, f64::max);
    let expect = wheelbase(&r) / scalar::tan(outer) - half_track; // centreline radius at the rear axle
    println!("radius {radius:.2} m, closed form {expect:.2} m, yaw rate {:.3}", c.yaw_rate_rad_s());
    assert!(c.yaw_rate_rad_s() > 0.0, "a left steer must yaw left");
    assert!((radius / expect - 1.0).abs() < 0.05);
}

#[test]
fn step_steer_yaw_rate_settles_without_oscillation_below_the_critical_speed() {
    let (_r, w, mut c, mut d) = settled_at(12.0);
    let inp = DriveInputs { gear: GearRequest::Auto, throttle: 0.15, steer: 0.1, ..DriveInputs::default() };
    let mut trace = Vec::new();
    for _ in 0..(4.0 * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, &inp, &w, &mut d);
        trace.push(c.yaw_rate_rad_s());
    }
    let last_s = &trace[trace.len() - 60..];
    let mean = last_s.iter().sum::<f64>() / last_s.len() as f64;
    let peak = trace.iter().cloned().fold(0.0, |a: f64, b| a.min(b));
    let spread = last_s.iter().map(|v| (v - mean).abs()).fold(0.0, f64::max);
    println!("settled yaw rate {mean:.4} rad/s, peak {peak:.4}, spread over last second {spread:.5}");
    assert!(mean < 0.0, "a right steer must yaw right");
    assert!(peak / mean < 1.3, "overshoot {}", peak / mean);
    assert!(spread < 0.02 * mean.abs());
}

#[test]
fn truck_crosses_the_speed_hump_at_20kmh_without_bottoming_out() {
    use w5k_contract::testing::BumpStrip;
    let (r, w) = (rig(), BumpStrip::standard());
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, -20.0, 0.0).unwrap();
    let mut d = drive(&r);
    run(&mut c, &w, &mut d, &coast(), 2.0);
    let v = scalar::kmh_to_ms(20.0);
    c.set_forward_speed(v);
    let mut max_travel = vec![f64::MIN; c.stations.len()];
    let mut hit_limit = false;
    while c.datum_m().z > -55.0 && c.time_s < 30.0 {
        let throttle = scalar::clamp(v - c.forward_speed_m_s(), 0.0, 1.0); // hold the speed against rolling resistance
        let inp = DriveInputs { gear: GearRequest::Auto, throttle, ..DriveInputs::default() };
        c.tick(1.0 / TICK_HZ, &inp, &w, &mut d);
        assert!(c.is_finite());
        for (m, s) in max_travel.iter_mut().zip(&c.stations) {
            *m = m.max(s.travel_m);
            hit_limit |= s.report.suspension.at_limit;
        }
    }
    let allowed: Vec<f64> = r.stations.iter().map(|s| s.bump_travel_m).collect();
    println!(
        "max travel {max_travel:.3?} m of {allowed:.3?} m; crossed at {:.1} km/h",
        scalar::ms_to_kmh(c.forward_speed_m_s())
    );
    assert!(c.datum_m().z <= -55.0, "the truck did not get over the hump");
    assert!(!hit_limit && max_travel.iter().zip(&allowed).all(|(m, a)| m < a));
}

#[test]
fn ledger_net_force_equals_mass_times_acceleration() {
    use w5k_contract::testing::BumpStrip;
    use w5k_contract::ForceLedger;
    let (r, w) = (rig(), BumpStrip::standard());
    let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, -30.0, 0.0).unwrap();
    c.ledger = ForceLedger::on();
    let mut d = drive(&r);
    c.set_forward_speed(8.0);
    let h = 1.0 / TICK_HZ / f64::from(c.substeps());
    let mut worst: f64 = 0.0;
    // over the hump, steering, then braking: every term is busy
    for n in 0..(4.0 * TICK_HZ) as usize {
        let inp = if n < 120 {
            DriveInputs { gear: GearRequest::Auto, throttle: 0.4, steer: 0.3, ..DriveInputs::default() }
        } else {
            DriveInputs { gear: GearRequest::Neutral, brake: 1.0, steer: -0.2, ..DriveInputs::default() }
        };
        for _ in 0..c.substeps() {
            c.substep(h, &inp, &w, &mut d);
            let scale = r.hull.mass_kg * scalar::G;
            // hull: the ledger's net force is m a, its net torque is the torque the hull integrated
            let f_err = (c.ledger.net_force_on(0) - c.hull.acc_m_s2 * r.hull.mass_kg).length() / scale;
            let torque_total = c.ledger.totals().iter().fold(w5k_math::Vec3::ZERO, |a, t| a + t.1);
            let t_err = (torque_total - c.hull.last_torque_nm).length() / scale;
            worst = worst.max(f_err).max(t_err);
            // stations: net force m_u x travel acceleration along the strut (booked in the hull's frame)
            for (i, (s, def)) in c.stations.iter().zip(&r.stations).enumerate() {
                let expect = s.report.strut_dir * (def.unsprung_mass_kg * c.travel_acc_m_s2[i]);
                worst = worst.max((c.ledger.net_force_on(1 + i as u16) - expect).length() / scale);
            }
        }
    }
    println!("worst relative ledger error {worst:e} over {} rows per substep", c.ledger.len());
    assert!(worst < 1e-9);
    // every external term is active; internal ones (strut, anti-roll, the frame force) cancel between hull and wheel in the totals
    let t = c.ledger.totals();
    for term in [
        w5k_contract::ForceTerm::Gravity,
        w5k_contract::ForceTerm::TyreNormal,
        w5k_contract::ForceTerm::TyreLongitudinal,
        w5k_contract::ForceTerm::TyreLateral,
    ] {
        assert!(t[term as usize].0.length() > 0.0, "{term:?} is empty");
    }
    assert!(t[w5k_contract::ForceTerm::SuspensionSpring as usize].0.length() < 1e-9 * r.hull.mass_kg * scalar::G);
}

#[test]
fn switching_the_ledger_on_does_not_change_the_motion() {
    let hash = |on: bool| {
        let (r, w) = (rig(), FlatPlane::new());
        let mut c = WheeledChassis::new(&r, &tuning(), &w, 0.0, 0.0, 0.0).unwrap();
        if on {
            c.ledger = w5k_contract::ForceLedger::on();
        }
        let mut d = drive(&r);
        c.set_forward_speed(10.0);
        run(
            &mut c,
            &w,
            &mut d,
            &DriveInputs { gear: GearRequest::Auto, throttle: 0.3, steer: 0.2, ..DriveInputs::default() },
            2.0,
        );
        let mut h = w5k_math::StateHasher::new();
        c.hash_state(&mut h);
        h.finish()
    };
    assert_eq!(hash(false), hash(true));
}
