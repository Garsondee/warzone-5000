//! The three game trucks (scout, mule, hauler) compile with the same coherence checks, the same joint layout, and differ where they should.

use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::rig::*;
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::render::render_rig;
use w5k_math::scalar::{self, G};

const IDS: [&str; 3] = ["scout_4x4", "mule_4x4", "hauler_4x4"];

fn build(id: &str) -> (VehicleDef, Compiled) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    let d = parse_def(&std::fs::read_to_string(format!("{dir}{id}.ron")).unwrap()).unwrap();
    let x = parse_extras(&std::fs::read_to_string(format!("{dir}{id}.extras.ron")).unwrap()).unwrap();
    let c = compile(&d, &x).unwrap_or_else(|e| panic!("{id}: {e:?}"));
    (d, c)
}

fn ride_rate(s: &StationDef) -> f64 {
    let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!("linear") };
    let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
    rate_n_m * k_t / (rate_n_m + k_t)
}

#[test]
fn all_three_trucks_compile_and_pass_the_coherence_checks() {
    for id in IDS {
        let (d, c) = build(id);
        let rig = &c.rig;
        d.check().unwrap();
        rig.validate().unwrap();
        render_rig(rig, c.hull_size_m).validate().unwrap();
        // Same joint layout as the Mule: four spin, two steer, four travel.
        assert_eq!(rig.joint_names().len(), 10, "{id}");
        let RunningGearDef::Wheeled(w) = &d.running_gear else { panic!() };
        // Masses sum; preloads carry the sprung weight; ride frequency and damping match the sliders.
        let want = d.hull.mass_kg.v + 4.0 * w.tyre.unsprung_mass_kg.v;
        assert!((rig.total_mass_kg() - want).abs() < 1e-9, "{id}");
        let preloads: f64 = rig.stations.iter().map(|s| s.suspension.preload_n).sum();
        assert!((preloads - d.hull.mass_kg.v * G).abs() < 0.01 * d.hull.mass_kg.v * G, "{id}");
        for s in &rig.stations {
            let m = s.suspension.preload_n / G;
            let f = scalar::sqrt(ride_rate(s) / m) / scalar::TAU;
            let want_f = if s.axle == 0 {
                d.suspension.front_ride_frequency_hz.v
            } else {
                d.suspension.rear_ride_frequency_hz.v
            };
            assert!((f - want_f).abs() < 0.005 * want_f, "{id} {}: {f} vs {want_f}", s.name);
            let zeta = 0.5 * (s.suspension.damper.bump_ns_m + s.suspension.damper.rebound_ns_m)
                / (2.0 * scalar::sqrt(ride_rate(s) * m));
            assert!((zeta - d.suspension.damping_ratio.v).abs() < 0.005 * zeta, "{id} {}", s.name);
            // A sensible static ride height: the tyre sinks 1% to 12% of its radius under its static load.
            let sink = s.wheel.radius_m - rig.ride_height_m - s.rest_pos_m.y;
            assert!(sink > 0.01 * s.wheel.radius_m && sink < 0.12 * s.wheel.radius_m, "{id} {}: sink {sink}", s.name);
        }
        assert!(rig.integration.substeps <= 8, "{id}");
    }
}

#[test]
fn the_three_designs_differ_where_the_archetypes_differ() {
    let [(ds, cs), (dm, cm), (dh, ch)] = IDS.map(build);
    // Mass, power and tyre size grow from scout to mule to hauler.
    assert!(cs.rig.total_mass_kg() < cm.rig.total_mass_kg() && cm.rig.total_mass_kg() < ch.rig.total_mass_kg());
    assert!(ds.powertrain.engine.peak_power_w.v < dm.powertrain.engine.peak_power_w.v);
    assert!(dm.powertrain.engine.peak_power_w.v < dh.powertrain.engine.peak_power_w.v);
    let radius = |c: &Compiled| c.rig.stations[0].wheel.radius_m;
    assert!(radius(&cs) < radius(&cm) && radius(&cm) < radius(&ch));
    // The scout rides softest; the hauler stiffest.
    assert!(ds.suspension.front_ride_frequency_hz.v < dm.suspension.front_ride_frequency_hz.v);
    assert!(dm.suspension.front_ride_frequency_hz.v < dh.suspension.front_ride_frequency_hz.v);
    // The scout's damping is not left light: CHASSIS' slice-course check found its bump-side ratio (what the whoops excite) must be
    // about 0.25 to 0.35. Bump-side zeta = 2 zeta / (1 + rebound_to_bump).
    let bump_zeta =
        |d: &VehicleDef| 2.0 * d.suspension.damping_ratio.v / (1.0 + d.suspension.rebound_to_bump.as_ref().unwrap().v);
    assert!((0.25..=0.35).contains(&bump_zeta(&ds)), "scout bump-side zeta {}", bump_zeta(&ds));
    assert!(ds.suspension.damping_ratio.v <= dm.suspension.damping_ratio.v);
    // The hauler is the least stable: static stability factor (half track over COM height) is lowest.
    let ssf = |d: &VehicleDef| {
        let RunningGearDef::Wheeled(w) = &d.running_gear else { panic!() };
        0.5 * w.axles[0].track_width_m.v / d.hull.com_height_m.v
    };
    assert!(ssf(&dh) < ssf(&dm) && ssf(&dh) < ssf(&ds), "hauler must be the least stable");
    // The hauler's gear spread is wider (first over top) than the scout's.
    let spread = |c: &Compiled| {
        let g = &c.rig.drivetrain.gearbox.forward_ratios;
        g[0] / g[g.len() - 1]
    };
    assert!(spread(&ch) > spread(&cs));
}

#[test]
fn a_three_axle_independent_layout_compiles_today() {
    // Answer to "is a 6x6 expressible": FORGE's compile builds any number of independent axles (loads by statics, a centre
    // differential over the axle differentials). Tandem and solid-axle linkages are not compiled yet.
    let (mut d, _) = build("hauler_4x4");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    let x = parse_extras(&std::fs::read_to_string(format!("{dir}hauler_4x4.extras.ron")).unwrap()).unwrap();
    if let RunningGearDef::Wheeled(w) = &mut d.running_gear {
        let mut mid = w.axles[1].clone();
        mid.from_front_m = w5k_contract::param::Param::estimate(3.6, 3.0, 4.0, "three-axle check");
        w.axles[1].from_front_m.v = 4.9;
        w.axles.insert(1, mid);
        w.axles[1].steered = false;
    }
    d.hull.mass_kg.v = 6_400.0;
    let c = compile(&d, &x).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(c.rig.stations.len(), 6);
    assert_eq!(c.rig.joint_names().len(), 6 + 2 + 6);
    c.rig.validate().unwrap();
}

#[test]
fn declared_substeps_cover_the_stop_engaged_wheel_hop_of_every_station() {
    for id in IDS {
        let (_, c) = build(id);
        let rig = &c.rig;
        let mut worst: f64 = 0.0;
        for s in &rig.stations {
            let SpringKind::Linear { rate_n_m: k_s } = s.suspension.spring else { panic!("linear") };
            let b = &s.suspension.bump_stop;
            let pen = s.bump_travel_m - b.engage_m;
            let k_stop = b.rate_n_m * (1.0 + 2.0 * b.progression * pen / b.engage_m);
            let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
            worst = worst.max(scalar::sqrt((k_s + k_t + k_stop) / s.unsprung_mass_kg) / scalar::TAU);
        }
        let f_max = rig.integration.f_max_hz.unwrap();
        assert!(f_max >= worst - 1e-9, "{id}: recorded {f_max} Hz below the stop-engaged hop {worst} Hz");
        assert!(f64::from(rig.integration.substeps) * TICK_HZ >= SAMPLES_PER_PERIOD * worst, "{id}");
    }
}
