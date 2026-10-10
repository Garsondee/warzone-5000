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
fn road_wheel_bottoms_plus_belt_stand_into_the_ground_by_the_static_penetration() {
    let (_, x, c) = built();
    let tx = x.tracked.as_ref().unwrap();
    let k_wc = c.rig.tracks[0].wheel_contact.vertical_stiffness_n_m;
    for s in c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        let bottom = s.rest_pos_m.y - s.wheel.radius_m - tx.belt_thickness_m.v;
        let pen = c.rig.ground_y_m() - bottom;
        // The belt contact carries the wheel's whole load at rest: preload plus the unsprung weight.
        let want = (s.suspension.preload_n + s.unsprung_mass_kg * G) / k_wc;
        assert!((pen - want).abs() < 1e-9, "{}: penetration {pen}, want {want}", s.name);
        assert!(pen > 0.0 && pen < 0.015, "{}: {pen}", s.name);
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

/// The contract's torsion law: `T = T0 + K (phi0 - phi)`, `T0 = F0 L cos(phi0)`, `sin(phi) = sin(phi0) - c / L`, wheel force `T / (L cos(phi))`.
fn torsion_force(preload_n: f64, rate: f64, l: f64, phi0: f64, c: f64) -> f64 {
    let phi = scalar::asin(scalar::sin(phi0) - c / l);
    (preload_n * l * scalar::cos(phi0) + rate * (phi0 - phi)) / (l * scalar::cos(phi))
}

#[test]
fn torsion_bar_rate_gives_the_stated_ride_frequency_and_the_preload_carries_the_wheel_share() {
    let (d, _, c) = built();
    for s in c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        let SpringKind::Torsion { rate_nm_rad, arm_length_m, rest_arm_angle_rad } = s.suspension.spring else {
            panic!("a torsion bar")
        };
        let f0 = torsion_force(s.suspension.preload_n, rate_nm_rad, arm_length_m, rest_arm_angle_rad, 0.0);
        assert!((f0 - s.suspension.preload_n).abs() < 1e-9 * f0, "the force at rest is the preload");
        // The wheel rate at rest by a central difference of the closed form, no tyre in series: sqrt(k / m) / 2 pi is the slider.
        let h = 1e-5;
        let k = (torsion_force(s.suspension.preload_n, rate_nm_rad, arm_length_m, rest_arm_angle_rad, h)
            - torsion_force(s.suspension.preload_n, rate_nm_rad, arm_length_m, rest_arm_angle_rad, -h))
            / (2.0 * h);
        let f = scalar::sqrt(k / (s.suspension.preload_n / G)) / scalar::TAU;
        assert!((f - d.suspension.front_ride_frequency_hz.v).abs() < 0.005 * f, "{}: {f} Hz", s.name);
    }
}

#[test]
fn the_arm_pivot_gives_the_stated_arm_and_it_trails() {
    let (_, x, c) = built();
    let tx = x.tracked.as_ref().unwrap();
    for s in c.rig.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        let p = s.arm_pivot_m.expect("a torsion station has its pivot");
        let (dy, dz) = (s.rest_pos_m.y - p.y, s.rest_pos_m.z - p.z);
        assert!((scalar::hypot(dy, dz) - tx.torsion_arm_length_m.v).abs() < 1e-9);
        assert!(
            (scalar::atan2(-dy, dz.abs()) - tx.torsion_rest_angle_rad.v).abs() < 1e-9,
            "angle below the horizontal"
        );
        assert!(dz > 0.0, "the pivot is ahead of the wheel (a trailing arm): -Z is forward");
    }
    // Sprocket, idler and rollers are rigid and have no arm.
    assert!(c.rig.stations.iter().filter(|s| s.wheel.kind != WheelKind::RoadWheel).all(|s| s.arm_pivot_m.is_none()));
}

#[test]
fn the_belly_proxy_underside_is_the_ground_clearance_and_the_steer_law_reaches_the_rig() {
    let (d, x, c) = built();
    let belly = c.rig.proxies.iter().find(|p| p.role == ProxyRole::Belly).expect("a belly proxy");
    let ProxyShape::Box { half_m } = belly.shape else { panic!("a box") };
    let underside = belly.pose.pos.y - half_m.y - c.rig.ground_y_m();
    assert!((underside - d.hull.ground_clearance_m.v).abs() < 1e-9, "{underside}");
    assert!(
        (2.0 * half_m.x - (tracked(&d).track_gauge_m.v - tracked(&d).track_width_m.v)).abs() < 1e-9,
        "between the tracks"
    );
    let DriveNode::SteerUnit { law, .. } = &c.rig.drivetrain.driveline else { panic!("a steer unit") };
    assert_eq!(law.diff_ratio_by_gear.len(), x.tracked.as_ref().unwrap().steer_law.diff_ratio_by_gear.len());
    assert_eq!(law.steer_brakes, Some([0, 1]), "a controlled differential steers with the sprocket brakes");
    assert!(law.max_steer_torque_nm > 0.0);
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

mod render {
    use super::*;
    use w5k_contract::render::NodeRole;
    use w5k_forge::render::render_rig;

    #[test]
    fn the_tracked_render_rig_validates_and_its_joint_layout_matches_the_rig() {
        let (_, _, c) = built();
        let rr = render_rig(&c.rig, c.hull_size_m);
        rr.validate().unwrap();
        let names = c.rig.joint_names();
        assert_eq!(rr.joint_count, names.len());
        for n in &rr.nodes {
            if let Some(j) = n.joint {
                let (station, kind) = n.name.rsplit_once('.').unwrap();
                let want = if kind == "wheel" { format!("{station}.spin") } else { format!("{station}.{kind}") };
                assert_eq!(names[j.index], want, "node {}", n.name);
            }
        }
    }

    #[test]
    fn track_runs_follow_the_loop_order_and_the_belt_numbers() {
        let (_, x, c) = built();
        let th = x.tracked.as_ref().unwrap().belt_thickness_m.v;
        let rig = &c.rig;
        let rr = render_rig(rig, c.hull_size_m);
        assert_eq!(rr.track_runs.len(), 2);
        let names = rig.joint_names();
        for (run, t) in rr.track_runs.iter().zip(&rig.tracks) {
            assert_eq!(run.wheels.len(), t.stations.len(), "every wheel of the loop, in loop order");
            assert_eq!(run.sprocket, 0);
            assert_eq!(names[run.sprocket_joint], format!("{}.spin", rig.stations[t.sprocket].name));
            assert!(
                (f64::from(run.links) * t.pitch_m - t.belt_length_m).abs() <= 0.5 * t.pitch_m,
                "links close the loop to within one pitch"
            );
            assert_eq!(run.direction, -1, "a front sprocket: the top run runs against the loop order");
            for (w, &i) in run.wheels.iter().zip(&t.stations) {
                let s = &rig.stations[i];
                let want =
                    if s.wheel.kind == WheelKind::Sprocket { s.wheel.radius_m } else { s.wheel.radius_m + 0.5 * th };
                assert!((w.radius_m - want).abs() < 1e-12, "{}", s.name);
                assert_eq!(rr.nodes[w.node].name, format!("{}.wheel", s.name));
            }
            assert_eq!(rr.nodes[run.node].role, NodeRole::Track);
        }
    }

    #[test]
    fn wheel_meshes_agree_with_the_rig_radii_and_the_contacts_include_one_entry_per_belt_sample() {
        let (_, _, c) = built();
        let rig = &c.rig;
        let rr = render_rig(rig, c.hull_size_m);
        for s in &rig.stations {
            let m = rr.meshes.iter().find(|m| m.name == format!("{}.rim", s.name)).unwrap();
            let r = m.positions.iter().map(|p| scalar::hypot(f64::from(p[1]), f64::from(p[2]))).fold(0.0, f64::max);
            assert!((r - s.wheel.radius_m).abs() < 1e-3, "{}", s.name);
        }
        let samples: usize = rig.tracks.iter().map(|t| usize::from(t.samples)).sum();
        assert_eq!(rig.contact_names().len(), samples);
        assert!(rig.contact_names().iter().all(|n| n.starts_with("track_")));
    }
}
