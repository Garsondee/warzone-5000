//! Spike S4: soil fidelity versus cost. The gear on three soils with n != 1 against the textbook closed forms (Bekker plate sinkage; Wong's
//! drawbar-pull integral for a rigid track), and time per contact sample.
//!
//! The soil numbers are from memory of the Bekker-Wong tables in Wong, *Theory of Ground Vehicles* (ch. 2): UNVALIDATED until VALIDATION
//! checks table and page (card C-017). The closed forms are checked against the *model's own integral*, which is a real oracle for the code.

use w5k_contract::{Material, SoilParams};
use w5k_math::scalar;
use w5k_terramech::gear::{GearConfig, GearInput, TrackedRunningGear};
use w5k_terramech::plan::uniform_penetration_for_load_m;
use w5k_terramech::reference::reference_soils;
use w5k_terramech::soil;
use w5k_terramech::Tuning;

fn soils() -> Vec<(String, SoilParams)> {
    reference_soils().into_iter().map(|m| (m.name.clone(), m.soil.unwrap())).collect()
}

fn ground(soil: SoilParams) -> Material {
    Material {
        name: "soil".into(),
        mu_peak: 0.5,
        mu_slide: 0.4,
        rolling_coeff: 0.0,
        roughness_rms_m: 0.0,
        soil: Some(soil),
    }
}

/// An ideal rigid footprint: stiff wheels, a taut belt (no sag), 24 samples.
fn rigid_cfg(l: f64, b: f64) -> GearConfig {
    GearConfig {
        width_m: b,
        contact_length_m: l,
        samples: 24,
        wheel_x_m: vec![-0.4 * l, 0.4 * l],
        sprocket_radius_m: 0.4,
        wheel_stiffness_n_m: 1e12,
        wheel_damping_ns_m: 0.0,
        mass_per_m_kg: 0.0,
        tension_n: 1.0,
        shoe_mu_scale: 1.0,
        shoe_mu_scale_soft: None,
        resist_c0: 0.0,
        resist_c1_s_m: 0.0,
    }
}

#[test]
fn gear_sinkage_matches_the_bekker_plate_curve_for_three_soils() {
    let (l, b) = (3.0, 0.5);
    for (name, s) in soils() {
        let g = ground(s);
        for p in [10_000.0, 25_000.0, 50_000.0] {
            let load = p * b * l;
            let d = uniform_penetration_for_load_m(&rigid_cfg(l, b), Tuning::shipped(), &g, load, 1.0).unwrap();
            let z = soil::sinkage_m(&s, b, p);
            assert!(scalar::approx_eq(d, z, 1e-4 * z + 1e-9), "{name} p={p}: gear {d} vs Bekker {z}");
        }
    }
}

#[test]
fn drawbar_thrust_matches_the_closed_form_shear_integral_for_three_soils() {
    // H = (A c + W tan(phi)) [1 - K/(iL) (1 - exp(-iL/K))]  (uniform pressure, shear displacement j = i x).
    let (l, b) = (3.0, 0.5);
    for (name, s) in soils() {
        let g = ground(s);
        let w = 25_000.0 * b * l;
        let cfg = rigid_cfg(l, b);
        let d = uniform_penetration_for_load_m(&cfg, Tuning::shipped(), &g, w, 1.0).unwrap();
        for i in [0.05, 0.1, 0.2, 0.4] {
            let mut gear = TrackedRunningGear::new(cfg.clone(), Tuning::shipped());
            let v = 1.0;
            let vb = v / (1.0 - i);
            let grounds: Vec<&Material> = vec![&g; 24];
            let (pen, rate) = (vec![d; 2], vec![0.0; 2]);
            let mut thrust = 0.0;
            for _ in 0..8000 {
                thrust = gear
                    .step(&GearInput {
                        wheel_penetration_m: &pen,
                        wheel_penetration_rate_m_s: &rate,
                        vel_long_m_s: v,
                        vel_lat_m_s: 0.0,
                        yaw_rate_rad_s: 0.0,
                        sprocket_omega_rad_s: vb / cfg.sprocket_radius_m,
                        ground: &grounds,
                        dt_s: 0.001,
                    })
                    .shear_thrust_n;
            }
            let area = b * l;
            let expected = (area * s.cohesion_pa + w * scalar::tan(s.friction_angle_rad))
                * (1.0 - s.shear_k_m / (i * l) * (1.0 - scalar::exp(-i * l / s.shear_k_m)));
            let err = (thrust - expected) / expected;
            println!("{name:16} i={i:.2}: thrust {thrust:9.1} N, closed form {expected:9.1} N ({:+.2}%)", err * 100.0);
            assert!(err.abs() < 0.03, "{name} i={i}: {err}");
        }
    }
}

#[test]
#[allow(clippy::disallowed_methods)] // a wall-clock timing for the spike report; it never feeds the simulation
fn contact_cost_per_sample_is_a_few_microseconds() {
    let s = soils()[1].1;
    let g = ground(s);
    let cfg = rigid_cfg(3.0, 0.5);
    let mut gear = TrackedRunningGear::new(GearConfig { wheel_stiffness_n_m: 4e6, ..cfg.clone() }, Tuning::shipped());
    let grounds: Vec<&Material> = vec![&g; 24];
    let (pen, rate) = (vec![0.05; 2], vec![0.0; 2]);
    let steps = 20_000;
    let t0 = std::time::Instant::now();
    let mut acc = 0.0;
    for _ in 0..steps {
        acc += gear
            .step(&GearInput {
                wheel_penetration_m: &pen,
                wheel_penetration_rate_m_s: &rate,
                vel_long_m_s: 1.0,
                vel_lat_m_s: 0.0,
                yaw_rate_rad_s: 0.0,
                sprocket_omega_rad_s: 3.0,
                ground: &grounds,
                dt_s: 0.001,
            })
            .fx_n;
    }
    let ns = t0.elapsed().as_nanos() as f64 / (f64::from(steps) * 24.0);
    println!("{ns:.0} ns per soft-ground sample step (checksum {acc:.1})");
    assert!(ns < 10_000.0, "{ns} ns per sample");
}
