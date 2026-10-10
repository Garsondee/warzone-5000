//! Tracked running gear: the belt round the wheels has the closed-form length and a continuous direction, every wheel touches it, links tile
//! it with no gap or overlap, the sprocket's pitch radius follows from its teeth, and every part is a closed outward solid.

use w5k_contract::render::NodeRole;
use w5k_contract::rig::WheelKind;
use w5k_geo::track::{belt_mesh, pitch_radius_m, run_parts, tile, Belt, Circle, LinkSpec, RunSpec, RunWheel};
use w5k_math::{scalar, Pcg32};

const C: fn(f64, f64, f64) -> Circle = |z, y, r| Circle { z, y, r };

/// The belt sampled every `step` metres of arc: (point, direction).
fn samples(b: &Belt, step: f64) -> Vec<([f64; 2], [f64; 2])> {
    let n = (b.length_m / step).ceil() as usize;
    (0..n).map(|k| b.at(b.length_m * k as f64 / n as f64)).collect()
}

#[test]
fn a_band_round_two_equal_circles_is_two_pi_r_plus_twice_the_centre_distance() {
    let (r, d) = (0.4, 3.0);
    let b = Belt::round(&[C(0.0, 0.0, r), C(d, 0.0, r)]).unwrap();
    let want = 2.0 * std::f64::consts::PI * r + 2.0 * d;
    assert!((b.length_m - want).abs() < 1e-12, "{} against {want}", b.length_m);
}

#[test]
fn a_band_round_two_unequal_circles_has_the_open_belt_length() {
    // the textbook open belt: L = pi (r1 + r2) + 2 (r1 - r2) asin((r1 - r2) / d) + 2 sqrt(d^2 - (r1 - r2)^2)
    let (r1, r2) = (0.5, 0.3);
    let (dz, dy): (f64, f64) = (3.0, 0.4);
    let d = scalar::hypot(dz, dy);
    let b = Belt::round(&[C(0.0, 0.0, r1), C(dz, dy, r2)]).unwrap();
    let want = std::f64::consts::PI * (r1 + r2)
        + 2.0 * (r1 - r2) * scalar::asin((r1 - r2) / d)
        + 2.0 * (d * d - (r1 - r2) * (r1 - r2)).sqrt();
    assert!((b.length_m - want).abs() < 1e-9, "{} against {want}", b.length_m);
}

#[test]
fn the_carrier_belt_length_equals_numerical_integration_of_its_path() {
    let b = Belt::round(&RunSpec::placeholder().circles()).unwrap();
    let pts = samples(&b, 1e-4);
    let integrated: f64 =
        pts.iter().zip(pts.iter().cycle().skip(1)).map(|(a, c)| scalar::hypot(c.0[0] - a.0[0], c.0[1] - a.0[1])).sum();
    assert!((integrated / b.length_m - 1.0).abs() < 1e-6, "integrated {integrated} against {}", b.length_m);
    println!("carrier belt: {:.3} m", b.length_m);
}

#[test]
fn the_belt_path_is_closed_and_its_direction_never_jumps() {
    let b = Belt::round(&RunSpec::placeholder().circles()).unwrap();
    let (p0, p1) = (b.at(0.0), b.at(b.length_m));
    assert!(scalar::hypot(p0.0[0] - p1.0[0], p0.0[1] - p1.0[1]) < 1e-9, "the loop does not close");
    let pts = samples(&b, 1e-3);
    // the tightest bend is the sprocket's: a 1 mm step turns by 1 mm / 0.27 m
    for (a, c) in pts.iter().zip(pts.iter().cycle().skip(1)) {
        let turn = scalar::asin(a.1[0] * c.1[1] - a.1[1] * c.1[0]).abs();
        assert!(turn < 0.005, "the direction jumps by {turn} rad at {:?}", a.0);
    }
}

#[test]
fn every_wheel_of_the_stand_in_run_touches_the_belt_and_none_pokes_through_it() {
    let spec = RunSpec::placeholder();
    let b = Belt::round(&spec.circles()).unwrap();
    let pts = samples(&b, 1e-3);
    for (i, c) in spec.circles().iter().enumerate() {
        let nearest = pts.iter().map(|p| scalar::hypot(p.0[0] - c.z, p.0[1] - c.y)).fold(f64::MAX, f64::min);
        assert!(nearest > c.r - 1e-5, "wheel {i} pokes {} m through the belt", c.r - nearest);
        assert!(nearest < c.r + 2e-3, "wheel {i} is {} m clear of the belt", nearest - c.r);
    }
}

#[test]
fn a_wheel_raised_off_the_belt_is_refused_by_name() {
    let mut spec = RunSpec::placeholder();
    let i = spec.wheels.iter().position(|w| w.kind == WheelKind::RoadWheel).unwrap() + 2; // the middle road wheel
    spec.wheels[i].y_m += 0.05;
    let e = Belt::round(&spec.circles()).unwrap_err();
    assert!(e.contains(&format!("wheel {i} does not touch")), "{e}");
}

#[test]
fn a_loop_listed_clockwise_is_refused_and_a_wheel_is_named() {
    // the carrier's wheels in the opposite order: the formula walks the band counter-clockwise, so the loop is wrong and a wheel is named
    let mut circles = RunSpec::placeholder().circles();
    circles.reverse();
    let e = Belt::round(&circles).unwrap_err();
    assert!(e.contains("wheel"), "{e}");
}

#[test]
fn links_tile_the_belt_with_no_gap_or_overlap() {
    let spec = RunSpec::placeholder();
    let link = LinkSpec::standard();
    let b = Belt::round(&spec.circles()).unwrap();
    let (n, pitch) = tile(b.length_m, spec.pitch_m);
    assert!((n as f64 * pitch - b.length_m).abs() < 1e-9, "{n} links of {pitch} do not close a {} m loop", b.length_m);
    assert!((pitch / spec.pitch_m - 1.0).abs() < 0.01, "effective pitch {pitch} against design {}", spec.pitch_m);
    // the count is the whole number nearest to L / p, not the one below it (10 m of 0.152 m links is 65.8: 66 links of 0.1515 m)
    let (n10, p10) = tile(10.0, 0.152);
    assert_eq!(n10, 66);
    assert!((p10 - 10.0 / 66.0).abs() < 1e-15);
    let m = belt_mesh(&b, &spec, &link, 0.0);
    assert_eq!(m.t.len(), 60 * n, "a link is five boxes of twelve triangles");
    // on the ground run the links are flat (a pad is as thick as the belt in y) and straight: neighbouring pads are one pitch apart, so
    // the gap between them is the specified fraction
    let per = m.v.len() / n;
    let mut pads: Vec<(f64, f64)> = (0..n)
        .filter_map(|k| {
            let pad = &m.v[k * per..k * per + 8];
            let (y0, y1) =
                (pad.iter().map(|p| p.y).fold(f64::MAX, f64::min), pad.iter().map(|p| p.y).fold(f64::MIN, f64::max));
            let flat = (y1 - y0 - spec.belt_thickness_m).abs() < 1e-9 && y0 < -1.3;
            flat.then(|| {
                (pad.iter().map(|p| p.z).fold(f64::MAX, f64::min), pad.iter().map(|p| p.z).fold(f64::MIN, f64::max))
            })
        })
        .collect();
    pads.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(pads.len() > 15, "{} links on the ground run", pads.len());
    for w in pads.windows(2) {
        let gap = w[1].0 - w[0].1;
        assert!((gap - link.gap_frac * pitch).abs() < 1e-9, "gap {gap} between neighbouring pads");
        assert!((w[0].1 - w[0].0 - pitch * (1.0 - link.gap_frac)).abs() < 1e-9, "pad length");
    }
}

#[test]
fn a_sprocket_of_n_teeth_has_the_pitch_radius_of_a_regular_polygon_of_side_pitch() {
    for (p, n) in [(0.152, 11), (0.2, 9), (0.1, 15)] {
        let r = pitch_radius_m(p, n);
        assert!((2.0 * r * scalar::sin(std::f64::consts::PI / f64::from(n)) - p).abs() < 1e-12, "{p} {n}");
    }
    let spec = RunSpec::placeholder();
    let sprocket = spec.wheels.iter().find(|w| w.kind == WheelKind::Sprocket).unwrap();
    let want = pitch_radius_m(spec.pitch_m, spec.sprocket_teeth);
    assert!(
        (sprocket.radius_m / want - 1.0).abs() < 1e-3,
        "the stand-in sprocket radius {} against {want}",
        sprocket.radius_m
    );
}

fn assert_closed_outward(parts: &[w5k_geo::part::Part]) {
    for p in parts {
        p.mesh.check_closed().unwrap_or_else(|e| panic!("{}: {e}", p.name));
        assert!(p.mesh.signed_volume() > 0.0, "{} faces inward", p.name);
        assert!(!p.mesh.t.is_empty(), "{} is empty", p.name);
    }
}

#[test]
fn every_part_of_the_stand_in_run_is_a_closed_outward_solid_with_the_right_node_role() {
    let parts = run_parts(&RunSpec::placeholder(), &LinkSpec::standard(), 1).unwrap();
    assert_closed_outward(&parts);
    let count = |role| {
        parts
            .iter()
            .filter(|p| p.role == role)
            .map(|p| p.name.split('.').nth(1).unwrap().to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    };
    assert_eq!(
        (count(NodeRole::RoadWheel), count(NodeRole::Sprocket), count(NodeRole::Idler), count(NodeRole::ReturnRoller)),
        (5, 1, 1, 3)
    );
    assert_eq!(parts.iter().filter(|p| p.role == NodeRole::Track).count(), 1, "one jointless Track node a side");
    let tris: usize = parts.iter().map(|p| p.mesh.t.len()).sum();
    println!("one side's running gear at detail 1: {tris} triangles");
    assert!(tris < 25_000, "{tris} triangles for one side");
}

/// A layout with `n` road wheels on a common ground line, the idler and sprocket raised so that their bottoms are above it, no return rollers.
fn random_spec(rng: &mut Pcg32) -> RunSpec {
    let mut u = |lo: f64, hi: f64| lo + (hi - lo) * rng.next_f64();
    let n = 3 + (u(0.0, 5.0) as usize).min(4);
    let (rw, spacing, thick) = (u(0.2, 0.45), u(0.5, 0.9), u(0.02, 0.04));
    let (pitch, teeth) = (u(0.1, 0.2), 9 + (u(0.0, 5.0) as u32).min(4));
    let first = -spacing * (n - 1) as f64 / 2.0;
    let path_r = rw + thick / 2.0; // the road wheels' path circles are centred on y = 0, so their bottoms lie on y = -path_r
    let rs = pitch_radius_m(pitch, teeth);
    // raised so that their bottoms and the top run both clear the road wheels (a sprocket much smaller than a road wheel needs raising most)
    let (sprocket_y, idler_y) = ((path_r - rs).abs() + 0.03 + u(0.0, 0.15), 0.03 + u(0.0, 0.17));
    let mut wheels = vec![RunWheel {
        kind: WheelKind::Sprocket,
        z_m: -first + u(0.5, 0.9),
        y_m: sprocket_y,
        radius_m: rs,
        width_m: 0.25,
    }];
    wheels.push(RunWheel {
        kind: WheelKind::Idler,
        z_m: first - u(0.6, 0.9),
        y_m: idler_y,
        radius_m: rw,
        width_m: 0.25,
    });
    wheels.extend((0..n).map(|k| RunWheel {
        kind: WheelKind::RoadWheel,
        z_m: first + spacing * k as f64,
        y_m: 0.0,
        radius_m: rw,
        width_m: 0.25,
    }));
    RunSpec {
        track_x_m: 1.0,
        belt_width_m: u(0.3, 0.5),
        belt_thickness_m: thick,
        pitch_m: pitch,
        sprocket_teeth: teeth,
        wheels,
        stations: None,
    }
}

#[test]
fn every_part_of_forty_random_runs_is_a_closed_outward_solid() {
    let mut rng = Pcg32::new(0x7261_636b, 7);
    for k in 0..40 {
        let spec = random_spec(&mut rng);
        let parts = run_parts(&spec, &LinkSpec::standard(), 0).unwrap_or_else(|e| panic!("run {k}: {e}"));
        assert_closed_outward(&parts);
    }
}

#[test]
fn the_belts_pads_rest_on_the_ground_plane_and_its_cleats_stand_their_height_below_it() {
    let spec = RunSpec::placeholder();
    let link = LinkSpec::standard();
    let b = Belt::round(&spec.circles()).unwrap();
    let m = belt_mesh(&b, &spec, &link, 0.0);
    let lowest = m.v.iter().fold(f64::MAX, |a, p| a.min(p.y));
    // the ground is 1.355 m below the hull centre: the pads' undersides touch it and the cleats stand `grouser_h_m` below
    assert!((lowest + 1.355 + link.grouser_h_m).abs() < 1e-3, "lowest point {lowest}");
}
