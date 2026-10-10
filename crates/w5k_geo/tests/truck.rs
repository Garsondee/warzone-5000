//! The utility 4x4: A4 (closed, outward, mirrored), A5 (deterministic bytes), A9 (node tags cover the render rig), A10 (dimensions,
//! placeholder bands until the dossier lands), the triangle budget.

use std::collections::BTreeSet;
use w5k_contract::render::NodeRole;
use w5k_contract::rig::Side;
use w5k_contract::testing::rigs::box_truck;
use w5k_geo::export::render_rig;
use w5k_geo::flags::FlagParams;
use w5k_geo::gear::wheel_module;
use w5k_geo::mesh::Mesh;
use w5k_geo::module::{Assembly, SocketKind};
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::Part;
use w5k_geo::truck::{utility_4x4, utility_assembly, utility_hull, utility_truck, UtilityDims};
use w5k_geo::weapon::{gun_module, GunDims};
use w5k_math::{scalar, Pcg32, Quat, StateHasher, Vec3};

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
    d.wheel.outer_radius_m = phys.stations[0].wheel.radius_m;
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
    let budget = w5k_geo::budget::wheeled_triangles();
    assert!(tris < budget, "{tris} triangles, budget {budget} (wheeled)");
}

// ---- cohesion: nothing floats

/// Odd number of crossings along an irrational direction: the point is inside the closed mesh.
fn inside(m: &Mesh, p: Vec3) -> bool {
    crossings(m, p, Vec3::new(0.371_1, 0.568_3, 0.735_7).normalized_or_zero()).len() % 2 == 1
}

/// Distance from a point to the nearest triangle (Ericson, Real-Time Collision Detection, closest point on a triangle).
fn distance_to_surface(m: &Mesh, p: Vec3) -> f64 {
    (0..m.t.len())
        .map(|k| {
            let [a, b, c] = m.tri(k);
            let (ab, ac, ap) = (b - a, c - a, p - a);
            let (d1, d2) = (ab.dot(ap), ac.dot(ap));
            let q = if d1 <= 0.0 && d2 <= 0.0 {
                a
            } else {
                let bp = p - b;
                let (d3, d4) = (ab.dot(bp), ac.dot(bp));
                if d3 >= 0.0 && d4 <= d3 {
                    b
                } else if d1 * d4 - d3 * d2 <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
                    a + ab * (d1 / (d1 - d3))
                } else {
                    let cp = p - c;
                    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
                    if d6 >= 0.0 && d5 <= d6 {
                        c
                    } else if d5 * d2 - d1 * d6 <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
                        a + ac * (d2 / (d2 - d6))
                    } else if d3 * d6 - d5 * d4 <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
                        b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)))
                    } else {
                        let va = d3 * d6 - d5 * d4;
                        let vb = d5 * d2 - d1 * d6;
                        let vc = d1 * d4 - d3 * d2;
                        let denom = 1.0 / (va + vb + vc);
                        a + ab * (vb * denom) + ac * (vc * denom)
                    }
                }
            };
            (p - q).length()
        })
        .fold(f64::MAX, f64::min)
}

fn host_meshes(parts: &[Part]) -> Vec<(String, Mesh)> {
    parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| (p.name.clone(), p.in_hull_frame()))
        .collect()
}

#[test]
fn every_fitting_is_embedded_in_the_hull_or_in_a_fitting_that_is() {
    let mut r = Pcg32::new(11, 3);
    let mut sets = vec![UtilityDims::placeholder()];
    sets.extend((0..4).map(|_| dims_in_range(&mut r)));
    for d in sets {
        let parts = utility_4x4(&d, 0);
        let mut attached: Vec<Mesh> = host_meshes(&parts).into_iter().map(|h| h.1).collect();
        let hosts = attached.len();
        let mut pending: Vec<&Part> = parts.iter().filter(|p| p.fitting).collect();
        loop {
            let before = pending.len();
            let mut keep = Vec::new();
            for p in pending {
                let m = p.in_hull_frame();
                // overlapping volumes: a vertex of the fitting inside something attached, or a vertex of an attached fitting inside it
                let touches = m.v.iter().any(|&v| attached.iter().any(|h| inside(h, v)))
                    || attached[hosts..].iter().any(|h| h.v.iter().any(|&v| inside(&m, v)));
                if touches {
                    attached.push(m);
                } else {
                    keep.push(p);
                }
            }
            pending = keep;
            if pending.is_empty() || pending.len() == before {
                break;
            }
        }
        let floating: Vec<&str> = pending.iter().map(|p| p.name.as_str()).collect();
        assert!(floating.is_empty(), "floating fittings {floating:?} for {d:?}");
    }
}

#[test]
fn plates_and_glass_hug_the_shell_within_3_cm() {
    let parts = utility_4x4(&UtilityDims::placeholder(), 0);
    let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    for p in parts.iter().filter(|p| {
        ["window", "windscreen", "door_front", "door_rear", "tailgate", "hood_panel"]
            .iter()
            .any(|n| p.name.starts_with(n))
    }) {
        let far = p
            .in_hull_frame()
            .v
            .iter()
            .filter(|&&v| !inside(&shell, v))
            .map(|&v| distance_to_surface(&shell, v))
            .fold(0.0_f64, f64::max);
        assert!(far < 0.03, "{} stands {:.3} m off the shell", p.name, far);
    }
}

#[test]
fn wheels_clear_the_shell_and_the_arches_open_over_every_tyre() {
    let parts = utility_4x4(&UtilityDims::placeholder(), 0);
    let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    let mut checked = 0;
    for p in parts.iter().filter(|p| p.role == NodeRole::Wheel) {
        for &v in p.in_hull_frame().v.iter().step_by(5) {
            assert!(!inside(&shell, v), "{} reaches into the shell at {:?}", p.name, v);
            checked += 1;
        }
    }
    assert!(checked > 500);
}

// ---- hull and running gear are separate modules

#[test]
fn the_hull_publishes_one_station_socket_per_wheel_position_and_each_wheel_sits_exactly_on_its_socket() {
    let d = UtilityDims::placeholder();
    let hull = utility_hull(&d, &[-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0], 0);
    assert!(
        hull.mount.is_none() && hull.parts.iter().all(|p| p.role == NodeRole::Hull),
        "the hull module has no wheels"
    );
    let stations: Vec<_> = hull.sockets.iter().filter(|s| s.kind == SocketKind::Station).collect();
    let names: Vec<_> = stations.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["station.0.r", "station.0.l", "station.1.r", "station.1.l"]);
    let parts = utility_4x4(&d, 0);
    for s in stations {
        let (sx, tag) = if s.side == Side::Right { (1.0, "r") } else { (-1.0, "l") };
        assert_eq!(s.kind, SocketKind::Station);
        assert!(
            (s.pose.pos.x - sx * d.track_m / 2.0).abs() < 1e-12 && (s.size_m - d.wheel.outer_radius_m).abs() < 1e-12
        );
        assert!(
            (s.pose.apply_dir(Vec3::Y) - Vec3::new(sx, 0.0, 0.0)).length() < 1e-12,
            "the normal points out of the hull"
        );
        assert!((s.pose.apply_dir(-Vec3::Z) + Vec3::Z).length() < 1e-12, "the reference points forward");
        assert!(
            s.hint("well_x_m").is_some_and(|x| x > 0.0 && x < d.track_m / 2.0)
                && s.hint("max_width_m") == Some(d.wheel.width_m)
        );
        let rim = parts.iter().find(|p| p.name == format!("rim.{}.{tag}", s.station.unwrap())).unwrap();
        assert_eq!(rim.pose.pos, s.pose.pos, "the hub is the socket");
        assert_eq!(rim.pose.rot, Quat::IDENTITY, "a wheel on an axis-aligned socket is placed with no rotation at all");
        assert_eq!((rim.station, rim.side), (s.station, s.side));
    }
}

#[test]
fn a_wheel_bigger_than_the_station_it_was_cut_for_is_refused_and_a_smaller_one_fits() {
    let d = UtilityDims::placeholder();
    let mut asm = Assembly::new(utility_hull(&d, &[-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0], 0));
    let mut big = d.wheel;
    big.outer_radius_m += 0.05;
    let err = asm.attach("station.0.r", &wheel_module(&big, 24, None), 0.0, "0.r").unwrap_err();
    assert!(err.reason.contains("needs"), "{err}");
    let mut small = d.wheel;
    small.outer_radius_m -= 0.05;
    small.rim_radius_m -= 0.05;
    assert!(asm.attach("station.0.r", &wheel_module(&small, 24, None), 0.0, "0.r").is_ok());
}

#[test]
fn three_axles_cut_three_arches_take_six_wheels_and_two_knuckles_and_the_wheels_clear_the_shell() {
    let d = UtilityDims::placeholder();
    let parts = utility_truck(&d, &[-d.wheelbase_m / 2.0, 0.1, d.wheelbase_m / 2.0], &[true, false, false], 0);
    check_parts(&parts);
    let count = |prefix: &str| parts.iter().filter(|p| p.name.starts_with(prefix)).count();
    assert_eq!((count("tyre."), count("rim."), count("arch_lip."), count("knuckle.")), (6, 6, 6, 2));
    let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    for p in parts.iter().filter(|p| p.role == NodeRole::Wheel) {
        for &v in p.in_hull_frame().v.iter().step_by(5) {
            assert!(!inside(&shell, v), "{} reaches into the shell at {:?}", p.name, v);
        }
    }
    let hubs: BTreeSet<i64> =
        parts.iter().filter(|p| p.name.starts_with("rim.")).map(|p| (p.pose.pos.z * 1000.0).round() as i64).collect();
    assert_eq!(hubs.len(), 3, "three axles");
}

// ---- weapon mount and weapons are separate modules

/// The vertices of a mesh and the midpoints of its edges: enough sample points to see a thin pin pass through a box.
fn samples(m: &Mesh) -> Vec<Vec3> {
    let mut s = m.v.clone();
    for t in &m.t {
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            s.push((m.v[t[a] as usize] + m.v[t[b] as usize]) * 0.5);
        }
    }
    s
}

/// The two solids share volume: a sample point of one lies inside the other.
fn overlap(a: &Mesh, b: &Mesh) -> bool {
    let ((alo, ahi), (blo, bhi)) = (a.bounds(), b.bounds());
    let apart = alo.x > bhi.x || blo.x > ahi.x || alo.y > bhi.y || blo.y > ahi.y || alo.z > bhi.z || blo.z > ahi.z;
    !apart && (samples(a).iter().any(|&v| inside(b, v)) || samples(b).iter().any(|&v| inside(a, v)))
}

/// The 4x4 with the standard ring mount on its roof socket.
fn mounted() -> Vec<Part> {
    let d = UtilityDims::placeholder();
    let z = d.wheelbase_m / 2.0;
    let mut asm = utility_assembly(&d, &[-z, z], &[true, false], 1);
    asm.attach("roof", &ring_mount(&RingMountDims::standard(), 1), 0.0, "ring").unwrap();
    asm.parts
}

#[test]
fn the_roof_socket_sits_two_centimetres_into_the_cab_roof_over_the_hatch_and_faces_up() {
    let d = UtilityDims::placeholder();
    let hull = utility_hull(&d, &[-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0], 0);
    let roof = hull.sockets.iter().find(|s| s.name == "roof").unwrap();
    let shell = hull.parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    let down = crossings(&shell, Vec3::new(0.0, 3.0, roof.pose.pos.z), -Vec3::Y);
    assert!((roof.pose.pos.y - (3.0 - down[0] - 0.02)).abs() < 1e-3, "2 cm below the roof surface at the socket");
    assert_eq!((roof.kind, roof.size_m), (SocketKind::Ring, 0.90));
    assert!(
        (roof.pose.apply_dir(Vec3::Y) - Vec3::Y).length() < 1e-12
            && (roof.pose.apply_dir(-Vec3::Z) + Vec3::Z).length() < 1e-12
    );
    let (lo, hi) = hull.parts.iter().find(|p| p.name == "roof_hatch").unwrap().in_hull_frame().bounds();
    assert!(roof.pose.pos.z > lo.z && roof.pose.pos.z < hi.z && roof.pose.pos.x.abs() < 1e-9, "over the roof hatch");
}

#[test]
fn nothing_on_the_mounted_truck_floats_the_mount_clears_the_hull_and_the_budget_holds() {
    let parts = mounted();
    let mut attached: Vec<Mesh> = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.name.ends_with(".ring"))
        .map(Part::in_hull_frame)
        .collect();
    let mut pending: Vec<&Part> = parts.iter().filter(|p| p.name.ends_with(".ring")).collect();
    loop {
        let before = pending.len();
        let mut keep = Vec::new();
        for p in pending {
            let m = p.in_hull_frame();
            if attached.iter().any(|h| overlap(&m, h)) {
                attached.push(m);
            } else {
                keep.push(p);
            }
        }
        pending = keep;
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    let floating: Vec<&str> = pending.iter().map(|p| p.name.as_str()).collect();
    assert!(floating.is_empty(), "floating {floating:?}");
    // the moving parts (turntable, uprights, shield wings) stand clear of the shell
    let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    for p in parts.iter().filter(|p| p.role == NodeRole::Turret) {
        for &v in p.in_hull_frame().v.iter().step_by(3) {
            assert!(!inside(&shell, v), "{} reaches into the shell at {v:?}", p.name);
        }
    }
    check_parts(&parts);
    let rig = render_rig("mounted_utility", &parts, &FlagParams::default_params());
    println!("truck with the ring mount in the rig: {} triangles", rig.triangle_count());
    let budget = w5k_geo::budget::wheeled_triangles();
    assert!(rig.triangle_count() < budget, "{} triangles, budget {budget} (wheeled)", rig.triangle_count());
}

/// The 4x4 with the standard ring mount on its roof socket and the named gun on the mount's trunnion.
fn armed(gun: &str) -> Vec<Part> {
    let d = UtilityDims::placeholder();
    let z = d.wheelbase_m / 2.0;
    let mut asm = utility_assembly(&d, &[-z, z], &[true, false], 1);
    asm.attach("roof", &ring_mount(&RingMountDims::standard(), 1), 0.0, "ring").unwrap();
    let cradle = asm.socket("trunnion.ring").unwrap().hint("cradle_w_m").unwrap();
    asm.attach("trunnion.ring", &gun_module(&GunDims::preset(gun).unwrap(), cradle, 1), 0.0, "gun").unwrap();
    asm.parts
}

#[test]
fn nothing_on_the_armed_truck_floats_and_the_gun_clears_the_hull() {
    for gun in ["machine_gun_12_7", "autocannon_25"] {
        let parts = armed(gun);
        let mut attached: Vec<Mesh> = parts
            .iter()
            .filter(|p| p.role == NodeRole::Hull && !p.name.ends_with(".ring"))
            .map(Part::in_hull_frame)
            .collect();
        let mut pending: Vec<&Part> =
            parts.iter().filter(|p| p.name.ends_with(".ring") || p.name.ends_with(".gun")).collect();
        loop {
            let before = pending.len();
            let mut keep = Vec::new();
            for p in pending {
                let m = p.in_hull_frame();
                if attached.iter().any(|h| overlap(&m, h)) {
                    attached.push(m);
                } else {
                    keep.push(p);
                }
            }
            pending = keep;
            if pending.is_empty() || pending.len() == before {
                break;
            }
        }
        let floating: Vec<&str> = pending.iter().map(|p| p.name.as_str()).collect();
        assert!(floating.is_empty(), "{gun}: floating {floating:?}");
        // the gun (receiver, boxes, grips, barrel, muzzle) stands clear of the shell: the barrel clears the windscreen
        let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
        for p in parts.iter().filter(|p| matches!(p.role, NodeRole::GunPitch | NodeRole::Recoil)) {
            for &v in p.in_hull_frame().v.iter().step_by(3) {
                assert!(!inside(&shell, v), "{gun}: {} reaches into the shell at {v:?}", p.name);
            }
        }
    }
}

#[test]
fn swapping_the_gun_changes_only_the_gun_parts_and_the_armed_truck_stays_inside_the_budget() {
    let (mg, ac) = (armed("machine_gun_12_7"), armed("autocannon_25"));
    let key = |p: &Part| {
        let bits = p.mesh.v.iter().flat_map(|v| v.as_array().map(f64::to_bits)).collect::<Vec<_>>();
        (p.name.clone(), bits, p.mesh.t.clone(), p.pose.pos.as_array().map(f64::to_bits))
    };
    let rest = |parts: &[Part]| parts.iter().filter(|p| !p.name.ends_with(".gun")).map(key).collect::<Vec<_>>();
    assert_eq!(rest(&mg), rest(&ac), "hull, wheels and mount are bit-identical whichever gun is on them");
    assert_ne!(
        mg.iter().filter(|p| p.name.ends_with(".gun")).count(),
        ac.iter().filter(|p| p.name.ends_with(".gun")).count()
    );
    check_parts(&mg);
    check_parts(&ac);
    assert_eq!(fingerprint(&mg), fingerprint(&armed("machine_gun_12_7")), "same recipe, same bytes");
    // the heavier gun, as exported (flags baked, so parts are subdivided): inside the wheeled budget
    let rig = render_rig("armed_utility", &ac, &FlagParams::default_params());
    println!("armed truck (autocannon) in the rig: {} triangles", rig.triangle_count());
    let budget = w5k_geo::budget::wheeled_triangles();
    assert!(rig.triangle_count() < budget, "{} triangles, budget {budget} (wheeled)", rig.triangle_count());
}

// ---- the glazing lines up

/// Convex hull of 2D points (Andrew's monotone chain), counter-clockwise.
fn hull(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut h: Vec<[f64; 2]> = Vec::new();
    for pass in 0..2 {
        let start = h.len();
        for &q in p.iter() {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
        if pass == 0 {
            p.reverse();
        }
    }
    h
}

#[test]
fn the_front_side_window_leans_at_the_same_angle_as_the_windscreen() {
    let parts = utility_4x4(&UtilityDims::placeholder(), 1);
    let side_view = |name: &str| -> Vec<[f64; 2]> {
        parts.iter().find(|p| p.name == name).unwrap().in_hull_frame().v.iter().map(|v| [v.z, v.y]).collect()
    };
    // the windscreen is a thin tilted slab: its principal axis in the side view is the slope of the roof-line ramp
    let ws = side_view("windscreen");
    let n = ws.len() as f64;
    let (mz, my) = (ws.iter().map(|p| p[0]).sum::<f64>() / n, ws.iter().map(|p| p[1]).sum::<f64>() / n);
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for p in &ws {
        let (a, b) = (p[0] - mz, p[1] - my);
        sxx += a * a;
        sxy += a * b;
        syy += b * b;
    }
    let ramp_deg = (0.5 * scalar::atan2(2.0 * sxy, sxx - syy)).to_degrees();
    // the window's slanted edge: the longest hull edge that is neither horizontal nor vertical
    let fold = |d: f64| (d.rem_euclid(180.0)).min(180.0 - d.rem_euclid(180.0)); // an edge's angle from horizontal, 0 to 90
    let hl = hull(side_view("window_front.r"));
    let edge_deg = (0..hl.len())
        .map(|i| (hl[i], hl[(i + 1) % hl.len()]))
        .map(|(a, b)| {
            (scalar::hypot(b[0] - a[0], b[1] - a[1]), fold(scalar::atan2(b[1] - a[1], b[0] - a[0]).to_degrees()))
        })
        .filter(|&(len, deg)| len > 0.1 && deg > 20.0 && deg < 80.0)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, deg)| deg)
        .unwrap();
    println!("windscreen ramp {:.2} degrees from horizontal, front window edge {:.2}", fold(ramp_deg), edge_deg);
    assert!((fold(ramp_deg) - edge_deg).abs() < 0.75, "ramp {ramp_deg}, window edge {edge_deg}");
    // and the ramp is a plausible windscreen: between 45 and 70 degrees from horizontal
    assert!(fold(ramp_deg) > 45.0 && fold(ramp_deg) < 70.0);
}

// ---- A10: the dimensions against the dossier

/// (v, lo, hi) of a dossier quantity, read from the RON text: the row `Quantity(id: "<id>", ... param: Param(v: X, lo: Some(a), hi: Some(b), ...`.
fn dossier(id: &str) -> (f64, Option<f64>, Option<f64>) {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/dossier/m998.ron"))
        .expect("content/dossier/m998.ron");
    let at = text.find(&format!("id: \"{id}\"")).unwrap_or_else(|| panic!("no dossier quantity {id}"));
    let chunk = &text[at..at + text[at..].find("notes:").expect("a quantity ends with notes")];
    let param = &chunk[chunk.find("Param(").expect("a quantity has a Param")..];
    let number_after = |key: &str| -> Option<f64> {
        let start = param.find(key)? + key.len();
        let tail = &param[start..];
        let end = tail.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(tail.len());
        tail[..end].parse().ok()
    };
    (number_after("v: ").expect("a Param has a value"), number_after("lo: Some("), number_after("hi: Some("))
}

#[test]
fn utility_4x4_dimensions_match_the_m998_dossier_within_3_percent() {
    let d = UtilityDims::placeholder();
    let parts = utility_4x4(&d, 1);
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| p.in_hull_frame().bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    let hubs: Vec<Vec3> =
        parts.iter().filter(|p| p.role == NodeRole::Wheel && p.name.starts_with("rim")).map(|p| p.pose.pos).collect();
    let wheelbase =
        hubs.iter().map(|h| h.z).fold(f64::MIN, f64::max) - hubs.iter().map(|h| h.z).fold(f64::MAX, f64::min);
    // definitions: length and width over the hull parts without fittings (bumpers and arch lips included, mirrors excluded); height from the
    // ground to the roof (the hull box's top: antenna and hatch excluded); wheelbase hub to hub; clearance from the ground to the underside
    let measured = [
        ("static.length_m", hi.z - lo.z),
        ("static.width_m", hi.x - lo.x),
        ("static.height_m", d.ride_height_m() + hi.y),
        ("static.wheelbase_m", wheelbase),
        ("static.ground_clearance_m", d.ride_height_m() + lo.y),
    ];
    for (id, got) in measured {
        let (v, band_lo, band_hi) = dossier(id);
        let within_3 = (got / v - 1.0).abs() <= 0.03;
        let in_band = matches!((band_lo, band_hi), (Some(a), Some(b)) if got >= a && got <= b);
        println!(
            "{id}: generated {got:.4} m, dossier {v:.4} m ({:+.2}%), band {band_lo:?}..{band_hi:?}",
            (got / v - 1.0) * 100.0
        );
        assert!(within_3 || in_band, "{id}: generated {got}, dossier {v}");
    }
}
