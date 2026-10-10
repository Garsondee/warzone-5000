//! The tracked carrier (`content/vehicles/game/carrier_tracked.ron`) through the tracked compile (T1). Physics sentences.

use w5k_contract::def::{RunningGearDef, TrackedDef, VehicleDef};
use w5k_contract::rig::*;
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::extras::Extras;
use w5k_forge::tracked::belt_perimeter;
use w5k_math::scalar::{self, G};

fn load() -> (VehicleDef, Extras) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    (
        parse_def(&std::fs::read_to_string(format!("{dir}carrier_tracked.ron")).unwrap()).unwrap(),
        parse_extras(&std::fs::read_to_string(format!("{dir}carrier_tracked.extras.ron")).unwrap()).unwrap(),
    )
}

fn built() -> (VehicleDef, Extras, Compiled) {
    let (d, x) = load();
    let c = compile(&d, &x).unwrap_or_else(|e| panic!("{e:?}"));
    (d, x, c)
}

fn tracked(d: &VehicleDef) -> &TrackedDef {
    let RunningGearDef::Tracked(t) = &d.running_gear else { panic!("tracked") };
    t
}

#[test]
fn the_carrier_compiles_and_the_rig_validates() {
    let (_, _, c) = built();
    c.rig.validate().unwrap();
    assert_eq!(c.rig.tracks.len(), 2);
    assert_eq!(c.rig.stations.len(), 2 * (1 + 3 + 1 + 5));
    assert!(c.rig.integration.substeps <= 8);
}

#[test]
fn compile_is_deterministic() {
    let (_, _, a) = built();
    let (_, _, b) = built();
    assert_eq!(a.rig.rig_hash(), b.rig.rig_hash());
    assert_eq!(a.report, b.report);
}

#[test]
fn track_and_station_indices_are_consistent() {
    let (_, _, c) = built();
    let r = &c.rig;
    for t in &r.tracks {
        assert_eq!(t.stations.first(), Some(&t.sprocket), "loop order starts at the sprocket");
        assert_eq!(r.stations[t.sprocket].wheel.kind, WheelKind::Sprocket);
        assert_eq!(r.stations[t.idler].wheel.kind, WheelKind::Idler);
        let kinds: Vec<_> = t.stations.iter().map(|&i| r.stations[i].wheel.kind).collect();
        // sprocket, rollers, idler, then only road wheels
        let idler_at = kinds.iter().position(|k| *k == WheelKind::Idler).unwrap();
        assert!(kinds[1..idler_at].iter().all(|k| *k == WheelKind::ReturnRoller));
        assert!(kinds[idler_at + 1..].iter().all(|k| *k == WheelKind::RoadWheel));
        assert!(t.stations.iter().all(|&i| r.stations[i].side == t.side));
    }
    // The two sprockets are the two driveline outputs, left then right.
    let sides: Vec<_> = r.drivetrain.outputs.iter().map(|o| r.stations[o.station].side).collect();
    assert_eq!(sides, [Side::Left, Side::Right]);
}

#[test]
fn road_wheel_bottoms_plus_belt_sit_on_the_ground_plane() {
    let (_, x, c) = built();
    let th = x.tracked.as_ref().unwrap().belt_thickness_m.v;
    for s in c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        let bottom = s.rest_pos_m.y - s.wheel.radius_m - th;
        assert!((bottom - c.rig.ground_y_m()).abs() < 1e-9, "{}", s.name);
    }
}

#[test]
fn rollers_touch_the_belt_line_from_the_idler_top_to_the_sprocket_top() {
    let (_, x, c) = built();
    let th = x.tracked.as_ref().unwrap().belt_thickness_m.v;
    let r = &c.rig;
    let t = &r.tracks[0];
    let top = |i: usize| (r.stations[i].rest_pos_m.z, r.stations[i].rest_pos_m.y + r.stations[i].wheel.radius_m + th);
    let ((zs, ys), (zi, yi)) = (top(t.sprocket), top(t.idler));
    let rollers: Vec<usize> =
        t.stations.iter().copied().filter(|&i| r.stations[i].wheel.kind == WheelKind::ReturnRoller).collect();
    assert_eq!(rollers.len(), 3);
    for i in rollers {
        let (z, y) = top(i);
        let line = ys + (z - zs) / (zi - zs) * (yi - ys);
        assert!((y - line).abs() < 1e-9, "{}: belt line {line}, roller top plus belt {y}", r.stations[i].name);
    }
}

#[test]
fn sprocket_pitch_radius_matches_the_teeth_count() {
    let (d, x, c) = built();
    let (t, tx) = (tracked(&d), x.tracked.as_ref().unwrap());
    let want = t.pitch_m.v / (2.0 * scalar::sin(scalar::PI / tx.sprocket_teeth.v.round()));
    let r = c.rig.stations[c.rig.tracks[0].sprocket].wheel.radius_m;
    assert!((r - want).abs() < 0.01 * want, "{r} vs {want}");
}

#[test]
fn belt_length_matches_the_closed_loop_perimeter_hand_calculation() {
    // Two equal circles: 2 D + 2 pi r. Three collinear equal circles: the middle one adds nothing.
    let (d, r) = (2.0, 0.3);
    let two = belt_perimeter(&[(0.0, 0.0, r), (d, 0.0, r)]);
    assert!((two - (2.0 * d + scalar::TAU * r)).abs() < 1e-3 * two);
    let three = belt_perimeter(&[(0.0, 0.0, r), (0.5 * d, 0.0, r), (d, 0.0, r)]);
    assert!((three - two).abs() < 1e-9);
    // A small circle above a tangent line: unequal radii, the tangent length is sqrt(D^2 - (r1 - r2)^2).
    let (r1, r2, dd) = (0.4, 0.2, 3.0);
    let alpha = scalar::asin((r1 - r2) / dd);
    let want = 2.0 * scalar::sqrt(dd * dd - (r1 - r2) * (r1 - r2))
        + r1 * (scalar::PI + 2.0 * alpha)
        + r2 * (scalar::PI - 2.0 * alpha);
    let got = belt_perimeter(&[(0.0, 0.0, r1), (dd, 0.0, r2)]);
    assert!((got - want).abs() < 2e-3 * got, "{got} vs {want}");
}

#[test]
fn the_belt_is_longer_than_its_contact_and_links_make_an_integer_count_roughly() {
    let (d, _, c) = built();
    let t = &c.rig.tracks[0];
    assert!(
        t.belt_length_m > t.contact_length_m
            && (t.contact_length_m - tracked(&d).ground_contact_length_m.v).abs() < 1e-12
    );
    let links = t.belt_length_m / t.pitch_m;
    assert!(links > 30.0 && links < 200.0, "{links}");
}

#[test]
fn masses_sum_to_the_def_mass_and_the_track_ground_run_is_counted_once() {
    let (d, x, c) = built();
    let t = tracked(&d);
    let tx = x.tracked.as_ref().unwrap();
    let r = &c.rig;
    let ground_run: f64 = r.tracks.iter().map(|tr| tr.mass_per_m_kg * tr.contact_length_m).sum();
    let want = d.hull.mass_kg.v + 10.0 * tx.road_wheel_unsprung_kg.v + ground_run;
    assert!((r.total_mass_kg() - want).abs() < 1e-9);
    // The belt's mass per metre times its whole length is the def's track mass.
    for tr in &r.tracks {
        assert!((tr.mass_per_m_kg * tr.belt_length_m - t.track_mass_per_side_kg.v).abs() < 1e-9);
    }
}

#[test]
fn static_preload_carries_the_sprung_weight_and_follows_the_com() {
    let (d, _, c) = built();
    let road: Vec<&StationDef> = c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel).collect();
    let total: f64 = road.iter().map(|s| s.suspension.preload_n).sum();
    assert!((total - d.hull.mass_kg.v * G).abs() < 0.01 * total);
    // Moment about the hull centre equals the COM offset times the weight.
    let moment: f64 = road.iter().map(|s| s.suspension.preload_n * s.rest_pos_m.z).sum();
    assert!((moment - total * c.rig.hull.com_m.z).abs() < 0.01 * total * 0.5);
}

#[test]
fn ride_frequency_slider_gives_the_stated_natural_frequency_with_no_tyre_in_series() {
    let (d, _, c) = built();
    for s in c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        let SpringKind::Linear { rate_n_m } = s.suspension.spring else { panic!("linear") };
        let f = scalar::sqrt(rate_n_m / (s.suspension.preload_n / G)) / scalar::TAU;
        assert!((f - d.suspension.front_ride_frequency_hz.v).abs() < 0.005 * f, "{}: {f}", s.name);
    }
}

#[test]
fn the_steer_unit_drives_the_two_sprockets_and_the_brakes_sit_on_them() {
    let (_, _, c) = built();
    let r = &c.rig;
    let DriveNode::SteerUnit { children, .. } = &r.drivetrain.driveline else { panic!("a steer unit") };
    assert!(matches!(children.as_slice(), [DriveNode::Output(0), DriveNode::Output(1)]));
    for (k, b) in r.drivetrain.brakes.iter().enumerate() {
        assert_eq!(r.stations[b.station].wheel.kind, WheelKind::Sprocket);
        assert_eq!(r.drivetrain.outputs[k].station, b.station);
    }
}

#[test]
fn designs_the_tracks_cannot_honour_are_rejected_with_a_reason() {
    let (d0, x0) = load();
    let reason = |d: &VehicleDef, x: &Extras| {
        compile(d, x).err().expect("rejected").iter().map(|r| r.to_string()).collect::<Vec<_>>().join(" | ")
    };
    // Road wheels that overlap: too short a contact length for the wheel diameter.
    let mut d = d0.clone();
    if let RunningGearDef::Tracked(t) = &mut d.running_gear {
        t.ground_contact_length_m.v = 2.0;
        t.ground_contact_length_m.lo = Some(1.9);
    }
    assert!(reason(&d, &x0).contains("overlap"), "{}", reason(&d, &x0));
    // A sprocket diameter that disagrees with the teeth count.
    let mut d = d0.clone();
    if let RunningGearDef::Tracked(t) = &mut d.running_gear {
        t.sprocket_diameter_m.v = 0.51;
    }
    assert!(reason(&d, &x0).contains("pitch radius"));
    // No tracked extras at all.
    let mut x = x0.clone();
    x.tracked = None;
    assert!(reason(&d0, &x).contains("tracked"));
    // A steering unit is required.
    let mut d = d0;
    d.powertrain.steering_unit = None;
    assert!(reason(&d, &x0).contains("steering_unit"));
}
