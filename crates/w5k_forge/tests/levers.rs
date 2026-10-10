//! Every lever of the Design Impact Matrix moves its compiled number by the factor, on all three trucks; any factor either recompiles to a
//! valid rig or fails with a reason.

use w5k_contract::rig::*;
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::extras::Extras;
use w5k_forge::levers::{apply, LEVERS};
use w5k_math::scalar::{self, G};

const IDS: [&str; 3] = ["scout_4x4", "mule_4x4", "hauler_4x4"];

fn load(id: &str) -> (w5k_contract::def::VehicleDef, Extras) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    (
        parse_def(&std::fs::read_to_string(format!("{dir}{id}.ron")).unwrap()).unwrap(),
        parse_extras(&std::fs::read_to_string(format!("{dir}{id}.extras.ron")).unwrap()).unwrap(),
    )
}

fn compiled(id: &str, lever: &str, f: f64) -> Result<Compiled, String> {
    let (d, x) = load(id);
    let d = apply(&d, lever, f)?;
    compile(&d, &x).map_err(|e| e.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("; "))
}

fn ride_rate(s: &StationDef) -> f64 {
    let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!("linear") };
    let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
    rate_n_m * k_t / (rate_n_m + k_t)
}

fn peak_power(c: &Compiled) -> f64 {
    c.rig.drivetrain.engine.torque_curve.iter().map(|&(n, t)| t * n * scalar::TAU / 60.0).fold(0.0, f64::max)
}

fn peak_torque_rpm(c: &Compiled) -> f64 {
    c.rig.drivetrain.engine.torque_curve.iter().fold((0.0, 0.0), |a, &(n, t)| if t > a.1 { (n, t) } else { a }).0
}

fn first_axle_diff_ratio(c: &Compiled) -> f64 {
    fn find(n: &DriveNode) -> f64 {
        match n {
            DriveNode::Diff { ratio, children, .. } if children.iter().all(|c| matches!(c, DriveNode::Output(_))) => {
                *ratio
            }
            DriveNode::Diff { children, .. } | DriveNode::SteerUnit { children, .. } => find(&children[0]),
            DriveNode::Output(_) => 1.0,
        }
    }
    find(&c.rig.drivetrain.driveline)
}

/// The compiled number each lever moves, and the factor it should move by for `f`.
fn measure(lever: &str, c: &Compiled) -> f64 {
    let r = &c.rig;
    let (first, last) = (&r.stations[0], r.stations.last().unwrap());
    match lever {
        "engine_peak_power" => peak_power(c),
        "torque_peak_rpm" => peak_torque_rpm(c),
        "final_drive" => first_axle_diff_ratio(c),
        "first_gear" => r.drivetrain.gearbox.forward_ratios[0],
        "brake_torque" => r.drivetrain.brakes.iter().map(|b| b.max_torque_nm).sum(),
        "brake_thermal_mass" => r.drivetrain.brakes[0].thermal_mass_j_k,
        "mass" => r.hull.mass_kg,
        "com_height" => r.hull.com_m.y + r.ride_height_m,
        "ground_clearance" => r.ride_height_m - 0.5 * c.hull_size_m.y,
        "wheelbase" => last.rest_pos_m.z - first.rest_pos_m.z,
        "track_gauge" => first.rest_pos_m.x.abs(),
        "ride_frequency" | "spring_rate" => ride_rate(first),
        "damping" => 0.5 * (first.suspension.damper.bump_ns_m + first.suspension.damper.rebound_ns_m),
        "suspension_travel" => first.bump_travel_m + first.droop_travel_m,
        "tyre_width" => first.wheel.width_m,
        "tyre_pressure" => 1.0 / first.wheel.tyre.as_ref().unwrap().patch_length_m,
        "tyre_friction" => first.wheel.tyre.as_ref().unwrap().mu_scale,
        "frontal_area" => r.aero.frontal_area_m2,
        "drag_coeff" => r.aero.drag_coeff,
        other => panic!("no measurement for lever {other}"),
    }
}

/// Expected ratio of the measured number for a factor `f` (the physics, not the implementation).
fn expected(lever: &str, f: f64) -> f64 {
    match lever {
        "ride_frequency" => f * f,
        _ => f,
    }
}

#[test]
fn every_lever_moves_its_compiled_number_by_the_factor() {
    for id in IDS {
        let base = compiled(id, "mass", 1.0).unwrap_or_else(|e| panic!("{id}: {e}"));
        for lever in LEVERS {
            for f in [1.1, 0.9] {
                let c = match compiled(id, lever.name, f) {
                    Ok(c) => c,
                    Err(e) => {
                        assert!(lever.name == "brake_torque" && f > 1.0, "{id} {} x{f}: {e}", lever.name);
                        assert!(e.contains("cannot deliver"), "{e}");
                        continue;
                    }
                };
                c.rig.validate().unwrap_or_else(|e| panic!("{id} {} x{f}: {e:?}", lever.name));
                let ratio = measure(lever.name, &c) / measure(lever.name, &base);
                let want = expected(lever.name, f);
                let tol = if lever.name == "torque_peak_rpm" { 0.002 } else { 1e-6 };
                assert!(
                    (ratio - want).abs() < tol * want.abs().max(1.0),
                    "{id} {} x{f}: moved by {ratio}, expected {want}",
                    lever.name
                );
            }
        }
    }
}

#[test]
fn tyre_width_and_pressure_move_the_contact_patch_inversely() {
    let (d, x) = load("mule_4x4");
    let patch = |lever: &str, f: f64| {
        let c = compile(&apply(&d, lever, f).unwrap(), &x).unwrap();
        c.rig.stations[0].wheel.tyre.as_ref().unwrap().patch_length_m
    };
    let base = patch("mass", 1.0);
    assert!((patch("tyre_width", 1.1) / base - 1.0 / 1.1).abs() < 1e-9);
    assert!((patch("tyre_pressure", 1.1) / base - 1.0 / 1.1).abs() < 1e-9);
}

#[test]
fn mass_moves_the_springs_with_it_so_the_ride_frequency_holds() {
    let (d, x) = load("mule_4x4");
    let c = compile(&apply(&d, "mass", 1.5).unwrap(), &x).unwrap();
    let s = &c.rig.stations[0];
    let f = scalar::sqrt(ride_rate(s) / (s.suspension.preload_n / G)) / scalar::TAU;
    assert!(
        (f - d.suspension.front_ride_frequency_hz.v).abs() < 0.005 * f,
        "ride frequency is a slider; it must not move with mass"
    );
    let base = compile(&d, &x).unwrap();
    assert!(ride_rate(s) > 1.4 * ride_rate(&base.rig.stations[0]), "the spring rate rises with the sprung mass");
}

#[test]
fn any_factor_on_any_lever_recompiles_to_a_valid_rig_or_fails_with_a_reason() {
    for id in IDS {
        for lever in LEVERS {
            for f in [0.3, 0.7, 1.0, 1.6, 3.0] {
                match compiled(id, lever.name, f) {
                    Ok(c) => c.rig.validate().unwrap_or_else(|e| panic!("{id} {} x{f}: {e:?}", lever.name)),
                    Err(e) => assert!(!e.is_empty(), "{id} {} x{f}", lever.name),
                }
            }
        }
    }
}

#[test]
fn bad_factors_and_unknown_levers_are_errors_with_a_reason() {
    let (d, _) = load("mule_4x4");
    for f in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(apply(&d, "mass", f).unwrap_err().contains("finite and positive"));
    }
    let e = apply(&d, "flux_capacitor", 1.1).unwrap_err();
    assert!(e.contains("unknown lever") && e.contains("final_drive"), "{e}");
}

#[test]
fn the_lever_list_has_unique_names() {
    let mut names: Vec<&str> = LEVERS.iter().map(|l| l.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), LEVERS.len());
}
