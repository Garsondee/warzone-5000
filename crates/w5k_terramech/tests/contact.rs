//! Track contact: one sample, then a whole running gear on its own (no hull): the kernel's closed forms.

use w5k_contract::testing::standard_materials;
use w5k_contract::{ContactElement, ContactInput, Material, MaterialId};
use w5k_math::scalar;
use w5k_terramech::gear::{GearConfig, GearInput, GearTotals, TrackedRunningGear};
use w5k_terramech::reference::reference_soils;
use w5k_terramech::sample::{SampleGeom, TrackSample};
use w5k_terramech::{soil, Tuning};

fn dirt() -> Material {
    standard_materials().get(MaterialId(1)).clone()
}

fn soft_ground(i: usize) -> Material {
    reference_soils()[i].clone()
}

/// A tank-like gear: five road wheels, 12 samples. `stiff` makes the wheel contacts rigid and the belt taut so the footprint is ideal.
fn cfg(stiff: bool) -> GearConfig {
    GearConfig {
        width_m: 0.5,
        contact_length_m: 3.0,
        samples: 12,
        wheel_x_m: vec![-1.2, -0.6, 0.0, 0.6, 1.2],
        sprocket_radius_m: 0.4,
        wheel_stiffness_n_m: if stiff { 1e12 } else { 4_000_000.0 },
        wheel_damping_ns_m: 0.0,
        mass_per_m_kg: if stiff { 0.0 } else { 85.0 },
        tension_n: if stiff { 1.0 } else { 20_000.0 },
        shoe_mu_scale: 1.0,
        shoe_mu_scale_soft: None,
        resist_c0: 0.0,
        resist_c1_s_m: 0.0,
    }
}

fn run(gear: &mut TrackedRunningGear, g: &Material, pen_m: f64, v: f64, vb: f64, steps: usize) -> GearTotals {
    let cfgd = gear.config().clone();
    let grounds: Vec<&Material> = vec![g; cfgd.samples];
    let (pen, rate) = (vec![pen_m; cfgd.wheel_x_m.len()], vec![0.0; cfgd.wheel_x_m.len()]);
    let mut t = GearTotals::default();
    for _ in 0..steps {
        t = gear.step(&GearInput {
            wheel_penetration_m: &pen,
            wheel_penetration_rate_m_s: &rate,
            vel_long_m_s: v,
            vel_lat_m_s: 0.0,
            yaw_rate_rad_s: 0.0,
            sprocket_omega_rad_s: vb / cfgd.sprocket_radius_m,
            ground: &grounds,
            dt_s: 0.001,
        });
    }
    t
}

#[test]
fn a_sample_in_a_full_slide_on_firm_ground_pushes_back_with_mu_times_its_load() {
    let g = dirt();
    let geom = SampleGeom {
        width_m: 0.5,
        length_m: 0.25,
        stiffness_n_m: 1e6,
        damping_ns_m: 0.0,
        shoe_mu_scale: 1.0,
        shoe_mu_scale_soft: None,
    };
    let mut s = TrackSample::new(geom, Tuning::shipped());
    let mut o = Default::default();
    for _ in 0..500 {
        o = s.step(&ContactInput {
            penetration_m: 0.01,
            penetration_rate_m_s: 0.0,
            vel_long_m_s: 0.0,
            vel_lat_m_s: 0.0,
            surface_speed_m_s: 1.0,
            ground: &g,
            dt_s: 0.001,
        });
    }
    assert!(scalar::approx_eq(o.fz_n, 1e4, 1e-6));
    assert!(o.saturated);
    assert!(scalar::approx_eq(o.fx_n, g.mu_peak * o.fz_n, 1e-3 * o.fz_n), "fx {} vs {}", o.fx_n, g.mu_peak * o.fz_n);
}

#[test]
fn sample_sinkage_splits_penetration_between_soil_and_wheel_contact() {
    let g = soft_ground(1);
    let (b, dx) = (0.5, 0.25);
    let mk = |k| {
        TrackSample::new(
            SampleGeom {
                width_m: b,
                length_m: dx,
                stiffness_n_m: k,
                damping_ns_m: 0.0,
                shoe_mu_scale: 1.0,
                shoe_mu_scale_soft: None,
            },
            Tuning::shipped(),
        )
    };
    fn input(ground: &Material) -> ContactInput<'_> {
        ContactInput {
            penetration_m: 0.05,
            penetration_rate_m_s: 0.0,
            vel_long_m_s: 0.0,
            vel_lat_m_s: 0.0,
            surface_speed_m_s: 0.0,
            ground,
            dt_s: 0.001,
        }
    }
    // A rigid wheel: the whole penetration is soil sinkage, and the force is the Bekker pressure times the area.
    let o = mk(1e12).step(&input(&g));
    let s = g.soil.unwrap();
    assert!(scalar::approx_eq(o.sinkage_m, 0.05, 1e-6));
    assert!(scalar::approx_eq(o.fz_n, soil::pressure_pa(&s, b, 0.05) * b * dx, 1e-4 * o.fz_n));
    // A soft wheel: less sinkage, and the wheel spring carries exactly the soil's force.
    let k = 2e5;
    let o = mk(k).step(&input(&g));
    assert!(o.sinkage_m < 0.05 && o.sinkage_m > 0.0);
    assert!(scalar::approx_eq(o.fz_n, k * (0.05 - o.sinkage_m), 1e-6 * o.fz_n));
    assert!(scalar::approx_eq(o.fz_n, soil::pressure_pa(&s, b, o.sinkage_m) * b * dx, 1e-6 * o.fz_n));
}

#[test]
fn thrust_sums_to_the_expected_value_for_a_uniform_footprint() {
    // Wong: H = (A c + W tan(phi)) [1 - K/(iL) (1 - exp(-iL/K))] at 20% slip, on each reference soil.
    for i in 0..3 {
        let g = soft_ground(i);
        let s = g.soil.unwrap();
        let c = cfg(true);
        let p = 25_000.0;
        let mut gear = TrackedRunningGear::new(c.clone(), Tuning::shipped());
        let slip = 0.2;
        let t = run(&mut gear, &g, soil::sinkage_m(&s, c.width_m, p), 1.0, 1.0 / (1.0 - slip), 6000);
        let (l, w) = (c.contact_length_m, p * c.width_m * c.contact_length_m);
        let expected = (c.width_m * l * s.cohesion_pa + w * scalar::tan(s.friction_angle_rad))
            * (1.0 - s.shear_k_m / (slip * l) * (1.0 - scalar::exp(-slip * l / s.shear_k_m)));
        assert!(scalar::approx_eq(t.fz_n, w, 1e-3 * w), "soil {i}: load {} vs {w}", t.fz_n);
        assert!(
            scalar::approx_eq(t.shear_thrust_n, expected, 0.03 * expected),
            "soil {i}: {} vs {expected}",
            t.shear_thrust_n
        );
    }
}

#[test]
fn compaction_resistance_is_paid_once_at_the_leading_sample_and_reverses_with_the_motion() {
    let g = soft_ground(1);
    let s = g.soil.unwrap();
    let c = cfg(true);
    let z = soil::sinkage_m(&s, c.width_m, 30_000.0);
    // The resistance is switched by the smooth sign v / (|v| + u0) of the direction of travel.
    let u0 = Tuning::shipped().direction_speed_m_s;
    let smooth = 2.0 / (2.0 + u0);
    let expected = soil::compaction_resistance_n(&s, c.width_m, 0.0, z) * smooth;
    let mut gear = TrackedRunningGear::new(c.clone(), Tuning::shipped());
    let fwd = run(&mut gear, &g, z, 2.0, 2.0, 10).compaction_n;
    assert!(scalar::approx_eq(fwd, expected, 1e-3 * expected), "{fwd} vs {expected}");
    let back = run(&mut gear, &g, z, -2.0, -2.0, 10).compaction_n;
    assert!(scalar::approx_eq(back, -expected, 1e-3 * expected), "{back} vs {}", -expected);
    let parked = run(&mut gear, &g, z, 0.0, 0.0, 10).compaction_n;
    assert!(parked.abs() < 1e-9 * expected, "a parked tank makes no rut: {parked}");
}

#[test]
fn sprocket_power_equals_shear_thrust_times_belt_speed() {
    let g = soft_ground(0);
    let s = g.soil.unwrap();
    let c = cfg(true);
    let mut gear = TrackedRunningGear::new(c.clone(), Tuning::shipped());
    let (v, vb) = (1.0, 1.25);
    let t = run(&mut gear, &g, soil::sinkage_m(&s, c.width_m, 25_000.0), v, vb, 6000);
    let power = t.shaft_reaction_nm * vb / c.sprocket_radius_m;
    assert!(scalar::approx_eq(power, t.shear_thrust_n * vb, 1e-9 * power));
    assert!(t.shaft_reaction_nm > 0.0, "driving: the shaft resists forward rolling");
}

#[test]
fn wheel_forces_sum_to_the_soil_load() {
    let g = soft_ground(1);
    let mut gear = TrackedRunningGear::new(cfg(false), Tuning::shipped());
    let t = run(&mut gear, &g, 0.06, 0.0, 0.0, 5);
    let sum: f64 = gear.wheel_force_n().iter().sum();
    assert!(scalar::approx_eq(sum, t.fz_n, 1e-9 * t.fz_n));
}

#[test]
fn belt_sag_puts_pressure_peaks_under_the_wheels_on_firm_ground_and_less_on_soft() {
    let sag_m = |x: f64| {
        // Wheels every 0.6 m: the sag of the belt between them is m' g s^2 / 8T times 4 t (1 - t).
        let t = (x / 0.6) - (x / 0.6).floor();
        85.0 * Tuning::shipped().gravity_m_s2 * 0.36 / (8.0 * 20_000.0) * 4.0 * t * (1.0 - t)
    };
    let run_ratio = |g: &Material, pen: f64| {
        let mut gear = TrackedRunningGear::new(cfg(false), Tuning::shipped());
        run(&mut gear, g, pen, 0.0, 0.0, 5);
        let (x, out) = (gear.sample_x_m().to_vec(), gear.outputs().to_vec());
        // The sample next to the wheel at x = 0 against the one nearest mid-span (x = 0.3).
        let at = |t: f64| (0..x.len()).min_by(|&a, &b| (x[a] - t).abs().total_cmp(&(x[b] - t).abs())).unwrap();
        let (a, b) = (at(0.0), at(0.3));
        (out[a].fz_n / out[b].fz_n, (x[a], x[b]))
    };
    let pen = 0.012;
    let (firm, (xa, xb)) = run_ratio(&dirt(), pen);
    // On firm ground the wheel contact is a linear spring, so the force ratio is exactly the penetration ratio.
    let expected = (pen - sag_m(xa)) / (pen - sag_m(xb));
    println!("wheel / mid-span pressure on firm ground {firm:.3} (predicted {expected:.3})");
    assert!(scalar::approx_eq(firm, expected, 1e-6));
    assert!(firm > 1.04, "on firm ground the wheels carry more: {firm}");
    // On soft ground the soil sinks under the heavier-loaded part, which evens the pressure out.
    let (soft, _) = run_ratio(&soft_ground(1), 0.06);
    println!("soft ground: {soft:.3}");
    assert!(soft < firm && soft > 1.0);
}
