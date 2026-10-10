//! Spike S3: does the sample-and-shear-displacement model feel right? (kill criterion: pivot moment within 25% of mu W L / 4, no oscillation.)

use w5k_contract::testing::standard_materials;
use w5k_contract::{Material, MaterialId};
use w5k_math::scalar;
use w5k_terramech::gear::GearConfig;
use w5k_terramech::plan::PlanVehicle;
use w5k_terramech::Tuning;

const G: f64 = 9.80665;

pub fn cfg() -> GearConfig {
    // A 40 t tank: 0.6 m tracks, 4.8 m contact length, five road wheels, 0.4 m sprocket.
    GearConfig {
        width_m: 0.6,
        contact_length_m: 4.8,
        samples: 12,
        wheel_x_m: vec![-2.0, -1.0, 0.0, 1.0, 2.0],
        sprocket_radius_m: 0.4,
        wheel_stiffness_n_m: 4_000_000.0,
        wheel_damping_ns_m: 20_000.0,
        mass_per_m_kg: 85.0,
        tension_n: 20_000.0,
        shoe_mu_scale: 1.0,
        shoe_mu_scale_soft: None,
        resist_c0: 0.05,
        resist_c1_s_m: 0.0,
    }
}

fn material(name: &str) -> Material {
    let t = standard_materials();
    (0..3).map(|i| t.get(MaterialId(i)).clone()).find(|m| m.name == name).unwrap()
}

fn vehicle(name: &str, weight_n: f64) -> PlanVehicle {
    PlanVehicle::new(cfg(), Tuning::shipped(), 2.8, weight_n, &material(name), 0.5).unwrap()
}

#[test]
fn pivot_turn_moment_equals_mu_w_l_over_4_on_firm_ground() {
    let w = 40_000.0 * G;
    let mut v = vehicle("dirt", w);
    let mu = v.ground().mu_peak;
    let (b, l) = (v.gauge_m, 4.8);
    // Pivot left at 0.3 rad/s: the tracks run at +-omega B/2, the hull does not translate.
    let omega = 0.3;
    v.motion.yaw_rate_rad_s = omega;
    let vb = 0.5 * omega * b;
    let mut mz = 0.0;
    for _ in 0..4000 {
        mz = v.step((-vb, vb), 0.001).mz_nm;
    }
    let expected = mu * w * l / 4.0;
    let err = (mz.abs() - expected) / expected;
    println!("pivot moment {:.0} N m vs mu W L/4 = {:.0} N m (error {:+.1}%)", mz, expected, err * 100.0);
    assert!(mz < 0.0, "the ground resists the turn");
    assert!(err.abs() < 0.25, "S3 kill criterion: error {err}");
}

#[test]
fn stopped_track_on_a_grade_does_not_creep() {
    let w = 40_000.0 * G;
    let mut v = vehicle("dirt", w);
    let theta = scalar::atan(0.3); // 30% grade: tan(theta) < mu = 0.65
    let down_slope = (w * scalar::sin(theta), 0.0);
    let (mut max_v, mut sign_changes, mut last) = (0.0f64, 0, 0.0f64);
    for i in 0..20_000 {
        v.step_dynamic((0.0, 0.0), (-down_slope.0, 0.0), 0.001);
        let vx = v.motion.vx_m_s;
        if i > 5_000 {
            max_v = max_v.max(vx.abs());
            if vx * last < 0.0 && vx.abs() > 1e-7 {
                sign_changes += 1;
            }
        }
        if vx != 0.0 {
            last = vx;
        }
    }
    println!("after 5 s: max |v| {max_v:.2e} m/s, sign changes {sign_changes}, creep {:.2e} m/s", v.motion.vx_m_s);
    assert!(max_v < 1e-3, "creeping at {max_v} m/s");
    assert!(sign_changes <= 1, "jitter: {sign_changes} sign changes");
}

#[test]
fn power_to_pivot_exceeds_power_to_drive_straight() {
    let w = 40_000.0 * G;
    let belt = 1.0;
    let mut straight = vehicle("dirt", w);
    let mut pivot = vehicle("dirt", w);
    let (mut ps, mut pp) = (0.0, 0.0);
    for _ in 0..15_000 {
        ps = straight.step_dynamic((belt, belt), (0.0, 0.0), 0.001).sprocket_power_w;
        pp = pivot.step_dynamic((-belt, belt), (0.0, 0.0), 0.001).sprocket_power_w;
    }
    println!(
        "straight {:.1} kW (v {:.2} m/s), pivot {:.1} kW (yaw {:.3} rad/s)",
        ps / 1e3,
        straight.motion.vx_m_s,
        pp / 1e3,
        pivot.motion.yaw_rate_rad_s
    );
    assert!(pp > 3.0 * ps, "pivot {pp} straight {ps}");
}

/// Steady turn of a vehicle with a footprint of the given length; returns (radius, kinematic radius, yaw rate).
fn turn(length_m: f64, vi: f64, vo: f64) -> (f64, f64) {
    let mut c = cfg();
    c.contact_length_m = length_m;
    c.wheel_x_m = vec![-0.4 * length_m, -0.2 * length_m, 0.0, 0.2 * length_m, 0.4 * length_m];
    let mut v = PlanVehicle::new(c, Tuning::shipped(), 2.8, 40_000.0 * G, &material("dirt"), 0.5).unwrap();
    for _ in 0..40_000 {
        v.step_dynamic((vi, vo), (0.0, 0.0), 0.001);
    }
    let m = v.motion;
    (m.vx_m_s / m.yaw_rate_rad_s, 0.5 * v.gauge_m * (vo + vi) / (vo - vi))
}

#[test]
fn skid_steer_turn_radius_follows_the_track_speeds() {
    // A short footprint scrubs little, so the turn is the kinematic one, R = (B/2)(vo + vi)/(vo - vi).
    let (r, kin) = turn(0.8, 2.0, 3.0);
    println!("short footprint: radius {r:.2} m vs kinematic {kin:.2} m");
    assert!((r / kin - 1.0).abs() < 0.25, "radius {r} kinematic {kin}");
}

#[test]
fn a_longer_footprint_turns_wider_for_the_same_track_speeds() {
    let radii: Vec<f64> = [0.8, 2.4, 4.8].iter().map(|&l| turn(l, 2.0, 3.0).0).collect();
    println!("radius for L = 0.8, 2.4, 4.8 m: {radii:?}");
    assert!(radii[0] < radii[1] && radii[1] < radii[2]);
}

#[test]
fn the_reference_tank_rig_gives_a_gear_with_five_road_wheels_and_the_sprocket_radius() {
    let (rig, _) = w5k_contract::testing::box_tank();
    let c = GearConfig::from_rig(&rig, 0);
    assert_eq!(c.wheel_x_m.len(), 5);
    assert!(scalar::approx_eq(c.wheel_x_m.iter().sum::<f64>(), 0.0, 1e-9), "centred on the mean");
    assert!(c.wheel_x_m.windows(2).all(|w| w[1] > w[0]), "ascending, forward positive");
    assert!(scalar::approx_eq(c.sprocket_radius_m, 0.4, 1e-12));
    assert_eq!(c.samples, 12);
}
