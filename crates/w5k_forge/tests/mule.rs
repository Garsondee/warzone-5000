//! The first reference-class truck (`content/vehicles/game/mule_4x4.ron`) through the wheeled compile. Tests are physics sentences.

use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::param::Param;
use w5k_contract::rig::*; // also TICK_HZ and SAMPLES_PER_PERIOD
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::curve::rpm_to_rad_s;
use w5k_forge::extras::Extras;
use w5k_forge::render::render_rig;
use w5k_math::scalar::{self, G};

fn load() -> (VehicleDef, Extras) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    let d = parse_def(&std::fs::read_to_string(format!("{dir}mule_4x4.ron")).unwrap()).unwrap();
    let x = parse_extras(&std::fs::read_to_string(format!("{dir}mule_4x4.extras.ron")).unwrap()).unwrap();
    (d, x)
}

fn built() -> (VehicleDef, Extras, Compiled) {
    let (d, x) = load();
    let c = compile(&d, &x).unwrap_or_else(|e| panic!("{e:?}"));
    (d, x, c)
}

fn wheeled(d: &VehicleDef) -> &w5k_contract::def::WheeledDef {
    let RunningGearDef::Wheeled(w) = &d.running_gear else { panic!("wheeled") };
    w
}

fn ride_rate(s: &StationDef) -> f64 {
    let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!("linear") };
    let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
    rate_n_m * k_t / (rate_n_m + k_t)
}

#[test]
fn the_def_files_check_clean_and_the_mule_compiles_to_valid_rigs() {
    let (d, x, c) = built();
    d.check().unwrap();
    assert!(x.check().is_empty());
    c.rig.validate().unwrap();
    render_rig(&c.rig, c.hull_size_m).validate().unwrap();
    assert_eq!(c.rig.stations.len(), 4);
}

#[test]
fn compile_is_deterministic() {
    let (_, _, a) = built();
    let (_, _, b) = built();
    assert_eq!(a.rig.rig_hash(), b.rig.rig_hash());
    assert_eq!(render_rig(&a.rig, a.hull_size_m), render_rig(&b.rig, b.hull_size_m));
    assert_eq!(a.report, b.report);
}

#[test]
fn def_round_trips_through_ron() {
    let (d, _) = load();
    let text = ron::ser::to_string_pretty(&d, ron::ser::PrettyConfig::default()).unwrap();
    assert_eq!(parse_def(&text).unwrap(), d);
}

#[test]
fn masses_sum_to_the_def_mass() {
    let (d, _, c) = built();
    let want = d.hull.mass_kg.v + 4.0 * wheeled(&d).tyre.unsprung_mass_kg.v;
    assert!((c.rig.total_mass_kg() - want).abs() < 1e-9);
}

#[test]
fn com_matches_the_def_position_and_height() {
    let (d, _, c) = built();
    let rig = &c.rig;
    assert!((rig.hull.com_m.y + rig.ride_height_m - d.hull.com_height_m.v).abs() < 1e-3);
    assert!((rig.hull.com_m.z + 0.5 * d.hull.length_m.v - d.hull.com_from_front_m.v).abs() < 1e-3);
    assert!((rig.ride_height_m - 0.5 * d.hull.height_m.v - d.hull.ground_clearance_m.v).abs() < 1e-3);
}

#[test]
fn inertia_tensor_is_positive_definite_and_obeys_the_triangle_inequalities() {
    let (_, _, c) = built();
    let m = c.rig.hull.inertia_kg_m2.m;
    assert!(m[0][0] > 0.0 && m[0][0] * m[1][1] - m[0][1] * m[1][0] > 0.0 && c.rig.hull.inertia_kg_m2.det() > 0.0);
    // Principal moments of a real body satisfy Ixx + Iyy >= Izz (and cyclic); the products of inertia here are small, so test the diagonal.
    let (a, b, g) = (m[0][0], m[1][1], m[2][2]);
    assert!(a + b >= g && b + g >= a && a + g >= b);
}

#[test]
fn static_preload_carries_the_sprung_weight_share_within_1_percent() {
    let (d, _, c) = built();
    let preloads: f64 = c.rig.stations.iter().map(|s| s.suspension.preload_n).sum();
    assert!((preloads - d.hull.mass_kg.v * G).abs() < 0.01 * d.hull.mass_kg.v * G);
    // The pitch balance: the front axle carries the share the COM position says.
    let front: f64 = c.rig.stations.iter().filter(|s| s.axle == 0).map(|s| s.suspension.preload_n).sum();
    let w = wheeled(&d);
    let share = (w.axles[1].from_front_m.v - d.hull.com_from_front_m.v)
        / (w.axles[1].from_front_m.v - w.axles[0].from_front_m.v);
    assert!((front / preloads - share).abs() < 0.01);
}

#[test]
fn ground_load_at_the_design_pose_equals_the_tyre_stiffness_times_the_deflection() {
    let (_, _, c) = built();
    for s in &c.rig.stations {
        let deflection = s.wheel.radius_m - c.rig.ride_height_m - s.rest_pos_m.y;
        let load = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m * deflection;
        let want = s.suspension.preload_n + s.unsprung_mass_kg * G;
        assert!((load - want).abs() < 0.01 * want, "{}", s.name);
    }
}

#[test]
fn ride_frequency_slider_gives_the_stated_natural_frequency() {
    let (d, _, c) = built();
    for s in &c.rig.stations {
        let f = scalar::sqrt(ride_rate(s) / (s.suspension.preload_n / G)) / scalar::TAU;
        let want =
            if s.axle == 0 { d.suspension.front_ride_frequency_hz.v } else { d.suspension.rear_ride_frequency_hz.v };
        assert!((f - want).abs() < 0.005 * want, "{}: {f} vs {want}", s.name);
    }
}

#[test]
fn damping_ratio_slider_gives_the_stated_zeta() {
    let (d, _, c) = built();
    for s in &c.rig.stations {
        let c_mean = 0.5 * (s.suspension.damper.bump_ns_m + s.suspension.damper.rebound_ns_m);
        let zeta = c_mean / (2.0 * scalar::sqrt(ride_rate(s) * s.suspension.preload_n / G));
        assert!((zeta - d.suspension.damping_ratio.v).abs() < 0.005 * zeta);
        assert!(s.suspension.damper.rebound_ns_m > s.suspension.damper.bump_ns_m);
    }
}

#[test]
fn torque_curve_peaks_match_the_def_peaks() {
    let (d, _, c) = built();
    let e = &d.powertrain.engine;
    let curve = &c.rig.drivetrain.engine.torque_curve;
    let (n_t, t_max) = curve.iter().fold((0.0, 0.0), |a, &(n, t)| if t > a.1 { (n, t) } else { a });
    let (n_p, p_max) =
        curve.iter().map(|&(n, t)| (n, t * rpm_to_rad_s(n))).fold((0.0, 0.0), |a, x| if x.1 > a.1 { x } else { a });
    assert!((t_max - e.peak_torque_nm.v).abs() < 0.005 * t_max && (n_t - e.peak_torque_rpm.v).abs() < 1.0);
    assert!((p_max - e.peak_power_w.v).abs() < 0.005 * p_max && (n_p - e.peak_power_rpm.v).abs() < 1.0);
}

#[test]
fn brake_torque_gives_the_stated_service_decel_on_the_axle_loads() {
    let (d, _, c) = built();
    let rig = &c.rig;
    let force: f64 =
        rig.drivetrain.brakes.iter().map(|b| b.max_torque_nm / rig.stations[b.station].wheel.radius_m).sum();
    let want = rig.total_mass_kg() * d.brakes.service_decel_g.v * G;
    assert!((force - want).abs() < 1e-6 * want);
    let front: f64 =
        rig.drivetrain.brakes.iter().filter(|b| rig.stations[b.station].axle == 0).map(|b| b.max_torque_nm).sum();
    let all: f64 = rig.drivetrain.brakes.iter().map(|b| b.max_torque_nm).sum();
    assert!((front / all - d.brakes.front_share.v).abs() < 1e-9);
}

#[test]
fn wheel_radius_in_rig_equals_tyre_diameter_over_two_and_the_mesh_agrees_to_a_millimetre() {
    let (d, _, c) = built();
    let rr = render_rig(&c.rig, c.hull_size_m);
    for s in &c.rig.stations {
        assert!((s.wheel.radius_m - 0.5 * wheeled(&d).tyre.outer_diameter_m.v).abs() < 1e-12);
        let m = rr.meshes.iter().find(|m| m.name == format!("{}.tyre", s.name)).unwrap();
        let r = m.positions.iter().map(|p| scalar::hypot(f64::from(p[1]), f64::from(p[2]))).fold(0.0, f64::max);
        assert!((r - s.wheel.radius_m).abs() < 1e-3);
    }
}

#[test]
fn render_rig_joint_layout_matches_the_rig() {
    let (_, _, c) = built();
    let names = c.rig.joint_names();
    let rr = render_rig(&c.rig, c.hull_size_m);
    assert_eq!(rr.joint_count, names.len());
    for n in &rr.nodes {
        if let Some(j) = n.joint {
            let station = n.name.split('.').next().unwrap();
            let suffix = n.name.split('.').nth(1).unwrap();
            let want = match suffix {
                "wheel" => format!("{station}.spin"),
                other => format!("{station}.{other}"),
            };
            assert_eq!(names[j.index], want, "node {}", n.name);
        }
    }
}

#[test]
fn every_driven_wheel_has_one_output_pointing_back_and_the_undriven_has_none() {
    let (d, _, c) = built();
    let w = wheeled(&d);
    assert_eq!(c.rig.drivetrain.outputs.len(), w.axles.iter().filter(|a| a.driven).count() * 2);
    for (i, s) in c.rig.stations.iter().enumerate() {
        assert_eq!(s.drive_output.is_some(), w.axles[s.axle as usize].driven, "station {i}");
    }
}

#[test]
fn designs_the_physics_cannot_honour_are_rejected_with_a_reason() {
    let (d0, x) = load();
    let reason = |d: &VehicleDef, x: &Extras| {
        compile(d, x).err().expect("rejected").iter().map(|r| r.to_string()).collect::<Vec<_>>().join(" | ")
    };
    // Power peak at or above the torque peak: impossible engine.
    let mut d = d0.clone();
    d.powertrain.engine.peak_power_w.v = 140_000.0;
    d.powertrain.engine.peak_power_w.hi = Some(150_000.0);
    assert!(reason(&d, &x).contains("peak power"));
    // Braking harder than the tyre can grip.
    let mut d = d0.clone();
    d.brakes.service_decel_g.v = 0.88;
    d.brakes.service_decel_g.hi = Some(0.95);
    d.running_gear = match d.running_gear {
        RunningGearDef::Wheeled(mut w) => {
            w.tyre.mu_peak_ref.v = 0.75;
            RunningGearDef::Wheeled(w)
        }
        t => t,
    };
    assert!(reason(&d, &x).contains("cannot deliver"));
    // A ride frequency so high that the tyre can no longer be the softer spring.
    let mut d = d0.clone();
    d.suspension.front_ride_frequency_hz.v = 4.0;
    d.suspension.front_ride_frequency_hz.hi = Some(5.0);
    assert!(reason(&d, &x).contains("tyre stiffness"));
    // A slider outside its band is a check failure, not a clamp.
    let mut d = d0;
    d.suspension.damping_ratio.v = 0.9;
    assert!(reason(&d, &x).contains("outside its band"));
}

/// A tiny deterministic generator (the fuzz must be reproducible, ordered, and free of platform maths).
struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

#[test]
fn fuzz_1000_random_defs_either_reject_with_a_reason_or_compile_and_validate() {
    let (d0, x) = load();
    let mut rng = Lcg(0x5EED);
    let (mut ok, mut rejected) = (0, 0);
    for _ in 0..1000 {
        let mut d2 = d0.clone();
        redraw(&mut d2, &mut rng);
        match compile(&d2, &x) {
            Ok(c) => {
                ok += 1;
                c.rig.validate().unwrap();
                assert!(c.rig.hull.inertia_kg_m2.is_finite() && c.rig.hull.mass_kg.is_finite());
                assert!(c.rig.stations.iter().all(|s| s.rest_pos_m.is_finite() && s.suspension.preload_n.is_finite()));
                assert!(c.rig.drivetrain.engine.torque_curve.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
            }
            Err(e) => {
                rejected += 1;
                assert!(!e.is_empty() && e.iter().all(|r| !r.reason.is_empty()));
            }
        }
    }
    assert!(ok > 0 && rejected > 0, "ok {ok}, rejected {rejected}: the fuzz should exercise both paths");
}

/// Redraw the sliders the compile derives from, uniformly inside their bands (coupling, gearbox, final drive, brakes and aero keep
/// their authored values).
fn redraw(d: &mut VehicleDef, rng: &mut Lcg) {
    let mut set = |p: &mut Param| {
        let (lo, hi) = p.band();
        p.v = lo + (hi - lo) * rng.unit();
    };
    let h = &mut d.hull;
    for p in [
        &mut h.length_m,
        &mut h.width_m,
        &mut h.height_m,
        &mut h.mass_kg,
        &mut h.com_height_m,
        &mut h.com_from_front_m,
        &mut h.ground_clearance_m,
    ] {
        set(p);
    }
    if let RunningGearDef::Wheeled(w) = &mut d.running_gear {
        for a in &mut w.axles {
            set(&mut a.from_front_m);
            set(&mut a.track_width_m);
            if let Some(p) = &mut a.anti_roll_n_m {
                set(p);
            }
        }
        let t = &mut w.tyre;
        for p in [
            &mut t.outer_diameter_m,
            &mut t.section_width_m,
            &mut t.inflation_pa,
            &mut t.mu_peak_ref,
            &mut t.cornering_stiffness_per_rad,
            &mut t.rolling_coeff,
            &mut t.unsprung_mass_kg,
        ] {
            set(p);
        }
    }
    let s = &mut d.suspension;
    for p in [
        &mut s.front_ride_frequency_hz,
        &mut s.rear_ride_frequency_hz,
        &mut s.damping_ratio,
        &mut s.bump_travel_m,
        &mut s.droop_travel_m,
    ] {
        set(p);
    }
    let e = &mut d.powertrain.engine;
    for p in [
        &mut e.peak_power_w,
        &mut e.peak_power_rpm,
        &mut e.peak_torque_nm,
        &mut e.peak_torque_rpm,
        &mut e.idle_rpm,
        &mut e.redline_rpm,
        &mut e.inertia_kg_m2,
        &mut e.bsfc_best_g_kwh,
    ] {
        set(p);
    }
}

#[test]
fn substeps_cover_twenty_samples_per_period_of_the_stiffest_mode() {
    let (_, _, c) = built();
    let f_max = c.rig.integration.f_max_hz.expect("the compile records the mode it baked from");
    assert!(f64::from(c.rig.integration.substeps) * TICK_HZ >= SAMPLES_PER_PERIOD * f_max);
    assert!(c.rig.integration.substeps <= 8);
}

#[test]
fn an_incomplete_design_is_rejected_naming_the_missing_field() {
    let (mut d, x) = load();
    d.suspension.rebound_to_bump = None;
    let e = compile(&d, &x).err().expect("rejected");
    assert!(e.iter().any(|r| r.reason.contains("suspension.rebound_to_bump is missing")), "{e:?}");
}
