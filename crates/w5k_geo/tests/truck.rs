//! The utility 4x4: A4 (closed, outward, mirrored), A5 (deterministic bytes), A9 (node tags cover the render rig), A10 (dimensions,
//! placeholder bands until the dossier lands), the triangle budget.

use std::collections::BTreeSet;
use w5k_contract::render::NodeRole;
use w5k_contract::rig::Side;
use w5k_contract::testing::rigs::box_truck;
use w5k_geo::mesh::Mesh;
use w5k_geo::part::Part;
use w5k_geo::truck::{utility_4x4, UtilityDims};
use w5k_math::{Pcg32, StateHasher, Vec3};

fn dims_in_range(r: &mut Pcg32) -> UtilityDims {
    let mut d = UtilityDims::placeholder();
    d.length_m *= r.range_f64(0.85, 1.15);
    d.width_m *= r.range_f64(0.9, 1.1);
    d.height_m *= r.range_f64(0.9, 1.15);
    d.wheelbase_m = d.length_m * r.range_f64(0.64, 0.74);
    d.track_m = d.width_m * r.range_f64(0.82, 0.9);
    d.ground_clearance_m *= r.range_f64(0.8, 1.2);
    d.wheel.outer_radius_m *= r.range_f64(0.9, 1.1);
    d.wheel.rim_radius_m = d.wheel.outer_radius_m * r.range_f64(0.55, 0.65);
    d.wheel.width_m *= r.range_f64(0.85, 1.15);
    d
}

fn check_parts(parts: &[Part]) {
    assert!(!parts.is_empty());
    for p in parts {
        let m = &p.mesh;
        assert!(m.check_closed().is_ok(), "{} is not closed", p.name);
        assert!(m.signed_volume() > 0.0, "{} faces inward", p.name);
        for k in 0..m.t.len() {
            let [a, b, c] = m.tri(k);
            assert!((b - a).cross(c - a).length() / 2.0 >= 1e-9, "{} has a sliver", p.name);
        }
    }
    // a mirrored part keeps its volume and negates the centroid x
    for p in parts.iter().filter(|p| p.side == Side::Left) {
        let twin_name = p.name.replace(".l", ".r");
        let twin = parts.iter().find(|q| q.name == twin_name).unwrap_or_else(|| panic!("no twin for {}", p.name));
        let (a, b) = (p.in_hull_frame().mass_props(), twin.in_hull_frame().mass_props());
        assert!((a.volume_m3 - b.volume_m3).abs() < 1e-9 * b.volume_m3.max(1.0), "{}", p.name);
        assert!((a.centroid_m.x + b.centroid_m.x).abs() < 1e-9, "{}", p.name);
    }
}

/// Crossings of a ray with a mesh: sorted distances.
fn crossings(m: &Mesh, o: Vec3, d: Vec3) -> Vec<f64> {
    let mut ts = Vec::new();
    for k in 0..m.t.len() {
        let t = m.tri(k);
        let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
        let p = d.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-14 {
            continue;
        }
        let s = o - t[0];
        let (u, q) = (s.dot(p) / det, s.cross(e1));
        let v = d.dot(q) / det;
        let at = e2.dot(q) / det;
        if u > 0.0 && v > 0.0 && u + v < 1.0 && at > 0.0 {
            ts.push(at);
        }
    }
    ts.sort_by(f64::total_cmp);
    ts
}

#[test]
fn every_generated_part_is_closed_manifold_and_outward_facing() {
    check_parts(&utility_4x4(&UtilityDims::placeholder(), 1));
    let mut r = Pcg32::new(5, 1);
    for _ in 0..200 {
        check_parts(&utility_4x4(&dims_in_range(&mut r), 0));
    }
}

#[test]
fn a_ray_crosses_an_even_number_of_faces_and_a_box_chord_is_the_wall_to_wall_distance() {
    let parts = utility_4x4(&UtilityDims::placeholder(), 1);
    let mut r = Pcg32::new(9, 2);
    for p in &parts {
        let m = p.in_hull_frame();
        let (lo, hi) = m.bounds();
        for _ in 0..8 {
            let o = Vec3::new(r.range_f64(lo.x, hi.x), r.range_f64(lo.y, hi.y), lo.z - 1.0);
            let d = Vec3::new(r.range_f64(-0.05, 0.05), r.range_f64(-0.05, 0.05), 1.0).normalized_or_zero();
            assert_eq!(crossings(&m, o, d).len() % 2, 0, "{}", p.name);
        }
    }
    // the tailgate is a bevelled slab: a straight ray through its face centre measures its thickness
    let gate = parts.iter().find(|p| p.name == "tailgate").unwrap().in_hull_frame();
    let (lo, hi) = gate.bounds();
    let ts = crossings(
        &gate,
        Vec3::new((lo.x + hi.x) / 2.0 + 0.013, (lo.y + hi.y) / 2.0 + 0.007, lo.z - 1.0),
        Vec3::new(0.0, 0.0, 1.0),
    );
    assert_eq!(ts.len(), 2);
    assert!((ts[1] - ts[0] - (hi.z - lo.z)).abs() < 1e-9);
}

fn fingerprint(parts: &[Part]) -> u64 {
    let mut h = StateHasher::new();
    for p in parts {
        h.write_str(&p.name);
        for v in &p.mesh.v {
            for c in v.as_array() {
                h.write_u32((c as f32).to_bits());
            }
        }
        for t in &p.mesh.t {
            t.iter().for_each(|&i| h.write_u32(i));
        }
    }
    h.finish()
}

#[test]
fn generation_is_deterministic_same_params_same_bytes() {
    let a = fingerprint(&utility_4x4(&UtilityDims::placeholder(), 1));
    assert_eq!(a, fingerprint(&utility_4x4(&UtilityDims::placeholder(), 1)));
    assert_ne!(a, fingerprint(&utility_4x4(&UtilityDims::placeholder(), 0)));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/utility_4x4.hash");
    if std::env::var("W5K_BLESS").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden")).unwrap();
        std::fs::write(path, format!("{a:016x}\n")).unwrap();
    }
    let golden = std::fs::read_to_string(path).expect("golden hash (bless with W5K_BLESS=1)");
    assert_eq!(format!("{a:016x}"), golden.trim(), "the generator changed: bless deliberately with W5K_BLESS=1");
}

#[test]
fn node_tags_cover_the_render_rig_joint_layout() {
    let (phys, rig) = box_truck();
    let mut d = UtilityDims::placeholder();
    // wheelbase and track are read from the rig
    let (xs, zs): (Vec<f64>, Vec<f64>) =
        phys.stations.iter().map(|s| (s.rest_pos_m.x.abs(), s.rest_pos_m.z.abs())).unzip();
    d.track_m = 2.0 * xs[0];
    d.wheelbase_m = 2.0 * zs[0];
    d.ground_clearance_m = phys.ride_height_m - d.height_m / 2.0;
    let parts = utility_4x4(&d, 1);
    let have: BTreeSet<(String, Option<u8>, String)> =
        parts.iter().map(|p| (format!("{:?}", p.role), p.station, format!("{:?}", p.side))).collect();
    let world = |mut i: usize| {
        let mut t = rig.nodes[i].rest;
        while let Some(p) = rig.nodes[i].parent {
            t = rig.nodes[p].rest.compose(&t);
            i = p;
        }
        t
    };
    for (i, n) in rig.nodes.iter().enumerate() {
        let Some(st) = phys.stations.iter().find(|s| n.name.starts_with(&s.name) && n.name.len() > s.name.len()) else {
            continue;
        };
        let key = (format!("{:?}", n.role), Some(st.axle), format!("{:?}", st.side));
        match n.role {
            NodeRole::Wheel | NodeRole::SteerKnuckle => {
                assert!(have.contains(&key), "no parts for node {} {:?}", n.name, key)
            }
            _ => {}
        }
        if n.role == NodeRole::Wheel {
            let pose = parts
                .iter()
                .find(|p| p.role == NodeRole::Wheel && p.station == Some(st.axle) && p.side == st.side)
                .unwrap()
                .pose;
            assert!(
                (world(i).pos - pose.pos).length() < 0.03,
                "hub of {} is off by {}",
                n.name,
                (world(i).pos - pose.pos).length()
            );
        }
    }
    // no part sits on a node the rig lacks
    for p in &parts {
        let exists = rig.nodes.iter().any(|n| n.role == p.role);
        assert!(exists, "{} is tagged {:?}, which the rig has no node for", p.name, p.role);
    }
}

#[test]
fn utility_4x4_dimensions_match_the_stated_hull_box_and_the_triangle_budget_holds() {
    // PLACEHOLDER dimensions, PROVISIONAL(C-002): the test compares the parts with the numbers they were generated from
    let d = UtilityDims::placeholder();
    let parts = utility_4x4(&d, 1);
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| p.in_hull_frame().bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    println!(
        "PLACEHOLDER hull box over the non-fitting hull parts: {:.3} x {:.3} x {:.3} m",
        hi.x - lo.x,
        hi.y - lo.y,
        hi.z - lo.z
    );
    assert!(((hi.x - lo.x) / d.width_m - 1.0).abs() < 0.03, "width");
    assert!(((hi.y - lo.y) / d.height_m - 1.0).abs() < 0.03, "height");
    assert!(((hi.z - lo.z) / d.length_m - 1.0).abs() < 0.03, "length");
    let tris: usize = parts.iter().map(|p| p.mesh.t.len()).sum();
    println!("triangles at detail 1: {tris}");
    assert!(tris < 40_000, "{tris} triangles, budget 40,000 (wheeled)");
}
