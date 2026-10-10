//! Mesh kernel oracles: A2 (cylinder volume), A3 (inertia closed forms, parallel-axis), closed and oriented checks, weld, mirror.

use w5k_geo::mass::MassProps;
use w5k_geo::mesh::Mesh;
use w5k_math::{scalar, Transform, Vec3};

const PI: f64 = std::f64::consts::PI;

fn cuboid(w: f64, d: f64, h: f64) -> Mesh {
    let (x, z) = (w / 2.0, d / 2.0);
    Mesh::extrude_fan(&[[-x, -z], [-x, z], [x, z], [x, -z]], 0, h).transformed(&Transform::from_pos(Vec3::new(
        0.0,
        -h / 2.0,
        0.0,
    )))
}

fn ngon_prism(n: u32, r: f64, len: f64) -> Mesh {
    let ring: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let a = -2.0 * PI * f64::from(i) / f64::from(n);
            [r * scalar::cos(a), r * scalar::sin(a)]
        })
        .collect();
    Mesh::extrude_fan(&ring, 0, len).transformed(&Transform::from_pos(Vec3::new(0.0, -len / 2.0, 0.0)))
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

#[test]
fn cylinder_volume_is_the_exact_n_gon_prism_and_within_0p1_percent_of_pi_r2_l_at_128_segments() {
    for n in [8u32, 24, 128] {
        let (r, l) = (0.4, 0.3);
        let m = ngon_prism(n, r, l);
        assert!(m.check_closed().is_ok() && m.signed_volume() > 0.0);
        let exact = f64::from(n) / 2.0 * r * r * scalar::sin(2.0 * PI / f64::from(n)) * l;
        assert!(close(m.signed_volume(), exact, 1e-12));
        if n == 128 {
            assert!((m.signed_volume() / (PI * r * r * l) - 1.0).abs() < 1e-3);
        }
    }
}

#[test]
fn box_and_cylinder_inertia_match_closed_forms_and_two_offset_boxes_obey_the_parallel_axis_theorem() {
    let (w, d, h) = (0.7, 1.3, 0.5);
    let p = cuboid(w, d, h).mass_props();
    let m = p.volume_m3;
    assert!(close(p.inertia_m5.m[0][0], m * (d * d + h * h) / 12.0, 1e-12));
    assert!(close(p.inertia_m5.m[1][1], m * (w * w + d * d) / 12.0, 1e-12));
    assert!(close(p.inertia_m5.m[2][2], m * (w * w + h * h) / 12.0, 1e-12));
    assert!(p.inertia_m5.m[0][1].abs() < 1e-12 && p.centroid_m.length() < 1e-12);
    let (n, r, l) = (24u32, 0.4, 0.3);
    let c = ngon_prism(n, r, l).mass_props();
    let cos = scalar::cos(PI / f64::from(n));
    assert!(close(c.inertia_m5.m[1][1], c.volume_m3 * r * r * (1.0 + 2.0 * cos * cos) / 6.0, 1e-12));
    // two boxes 2 m apart along x: I_zz about the common centroid = 2 (I_box + m d^2), d = 1
    let a = cuboid(1.0, 1.0, 1.0).mass_props();
    let both = MassProps::sum(&[a.translate(Vec3::new(-1.0, 0.0, 0.0)), a.translate(Vec3::new(1.0, 0.0, 0.0))]);
    assert!(close(both.inertia_m5.m[2][2], 2.0 * (1.0 / 6.0 + 1.0), 1e-12) && both.centroid_m.length() < 1e-12);
    // the same two boxes as one mesh must give the same answer as the algebra
    let mut one = cuboid(1.0, 1.0, 1.0).transformed(&Transform::from_pos(Vec3::new(-1.0, 0.0, 0.0)));
    one.append(&cuboid(1.0, 1.0, 1.0).transformed(&Transform::from_pos(Vec3::new(1.0, 0.0, 0.0))));
    assert!(close(one.mass_props().inertia_m5.m[2][2], both.inertia_m5.m[2][2], 1e-12));
}

#[test]
fn a_mirrored_part_keeps_its_volume_and_negates_centroid_x_and_a_hole_is_reported() {
    let mut m = cuboid(1.0, 2.0, 0.5).transformed(&Transform::from_pos(Vec3::new(3.0, 0.0, 1.0)));
    let mir = m.mirrored_x();
    assert!(mir.check_closed().is_ok());
    assert!(close(mir.signed_volume(), m.signed_volume(), 1e-12));
    assert!(close(mir.mass_props().centroid_m.x, -m.mass_props().centroid_m.x, 1e-12));
    m.t.pop();
    assert!(m.check_closed().is_err());
}

#[test]
fn weld_merges_unwelded_faces_and_drops_slivers() {
    let b = cuboid(1.0, 1.0, 1.0);
    let mut soup = Mesh::default();
    for k in 0..b.t.len() {
        let [x, y, z] = b.tri(k);
        let base = soup.v.len() as u32;
        soup.v.extend([x, y, z]);
        soup.t.push([base, base + 1, base + 2]);
    }
    assert!(soup.check_closed().is_err());
    soup.weld(1e-5, 1e-9);
    assert!(soup.check_closed().is_ok() && soup.v.len() == 8 && close(soup.signed_volume(), 1.0, 1e-12));
}

use w5k_geo::loft::{bevel_ring, loft, rect_ring, Section};

#[test]
fn rectangle_and_frustum_lofts_are_closed_boxes_with_the_prismatoid_volume() {
    let (w1, h1, w2, h2, l) = (1.0, 0.6, 2.2, 1.1, 3.0);
    let sec = |w, h, z| Section { z_m: z, ring: rect_ring(w, h, 0.0) };
    let b = loft(&[sec(w1, h1, 0.0), sec(w1, h1, l)], 10.0);
    assert!(b.check_closed().is_ok() && close(b.signed_volume(), w1 * h1 * l, 1e-12));
    for step in [10.0, 0.5] {
        let f = loft(&[sec(w1, h1, 0.0), sec(w2, h2, l)], step);
        assert!(f.check_closed().is_ok());
        assert!(close(f.signed_volume(), l * (2.0 * w1 * h1 + 2.0 * w2 * h2 + w1 * h2 + w2 * h1) / 6.0, 1e-12));
    }
    // a cut at a station returns the input section; between stations the interpolated one
    let f = loft(&[sec(w1, h1, 0.0), sec(2.0, 1.0, 1.0), sec(w2, h2, l)], 0.5);
    assert!(close(f.slice_area_z(1.0), 2.0 * 1.0, 1e-12) && close(f.slice_area_z(0.5), 1.5 * 0.8, 1e-12));
}

#[test]
fn bevel_rings_loft_to_a_closed_part_with_the_right_volume_loss() {
    let r = |w, h| bevel_ring(w, h, 0.0, 0.05, 0.05);
    let a = loft(&[Section { z_m: 0.0, ring: r(1.0, 0.6) }, Section { z_m: 2.0, ring: r(1.0, 0.6) }], 0.5);
    assert!(a.check_closed().is_ok());
    // the ring is a rectangle minus four corner triangles of legs 0.05: area 0.6 - 4 x 0.05^2 / 2
    assert!(close(a.signed_volume(), 2.0 * (0.6 - 4.0 * 0.05 * 0.05 / 2.0), 1e-12));
}

#[test]
fn subdivision_keeps_the_mesh_closed_and_its_volume_and_caps_the_edge_length() {
    let b = loft(
        &[
            Section { z_m: 0.0, ring: bevel_ring(1.0, 0.6, 0.0, 0.05, 0.05) },
            Section { z_m: 2.0, ring: bevel_ring(1.0, 0.6, 0.0, 0.05, 0.05) },
        ],
        10.0,
    );
    let s = b.subdivided(0.2);
    assert!(s.check_closed().is_ok() && close(s.signed_volume(), b.signed_volume(), 1e-12));
    assert!(s.t.iter().all(|&[a, b, c]| [(a, b), (b, c), (c, a)]
        .iter()
        .all(|&(p, q)| (s.v[p as usize] - s.v[q as usize]).length() <= 0.2)));
    assert!(s.t.len() > b.t.len());
}

use w5k_geo::loft::{polygon_ring, sweep_arc, triangulate_ring};

fn polygon_area(p: &[[f64; 2]]) -> f64 {
    (0..p.len()).map(|i| p[i][0] * p[(i + 1) % p.len()][1] - p[(i + 1) % p.len()][0] * p[i][1]).sum::<f64>() / 2.0
}

#[test]
fn a_non_convex_ring_with_collinear_points_triangulates_without_slivers_and_a_loft_of_it_has_the_prism_volume() {
    // a T: wide base, narrow stem, with extra points on the long edges
    let t = [
        [-1.0, 0.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 0.5],
        [0.3, 0.5],
        [0.3, 1.5],
        [-0.3, 1.5],
        [-0.3, 0.5],
        [-1.0, 0.5],
    ];
    for ring in [t.to_vec(), t.iter().rev().copied().collect::<Vec<_>>()] {
        let tris = triangulate_ring(&ring);
        let area: f64 = tris
            .iter()
            .map(|&[a, b, c]| (polygon_area(&[ring[a as usize], ring[b as usize], ring[c as usize]])).abs())
            .sum();
        assert!(close(area, polygon_area(&ring).abs(), 1e-12));
        assert!(tris
            .iter()
            .all(|&[a, b, c]| polygon_area(&[ring[a as usize], ring[b as usize], ring[c as usize]]).abs() > 1e-6));
        assert_eq!(tris.len(), ring.len() - 2);
    }
    let a = loft(&[Section { z_m: 0.0, ring: t.to_vec() }, Section { z_m: 2.0, ring: t.to_vec() }], 10.0);
    assert!(a.check_closed().is_ok() && close(a.signed_volume(), 2.0 * polygon_area(&t), 1e-12));
}

#[test]
fn a_chamfered_polygon_ring_keeps_its_point_count_while_the_shape_changes_and_lofts_closed() {
    let shape = |w: f64, h: f64| {
        [[-w, 0.0], [w, 0.0], [w, h], [0.3 * w, h], [0.3 * w, 2.0 * h], [-0.3 * w, 2.0 * h], [-0.3 * w, h], [-w, h]]
    };
    let flags = [0.05, 0.05, 0.05, 0.0, 0.05, 0.05, 0.0, 0.05];
    let a = polygon_ring(&shape(1.0, 0.5), &flags, 0.05);
    let b = polygon_ring(&shape(0.8, 0.002), &flags, 0.05);
    assert_eq!(a.len(), b.len());
    let m = loft(&[Section { z_m: 0.0, ring: a }, Section { z_m: 1.0, ring: b }], 0.5);
    assert!(m.check_closed().is_ok() && m.signed_volume() > 0.0);
}

#[test]
fn an_arc_sweep_of_a_rectangle_has_the_pappus_volume() {
    // Pappus: volume = cross-section area x path length of its centroid; a 0.1 x 0.05 section at radius 0.5 swept through 90 degrees
    let (n, ang) = (64usize, std::f64::consts::FRAC_PI_2);
    let ring = [[-0.05, 0.45], [-0.05, 0.55], [0.05, 0.55], [0.05, 0.45]];
    let angles: Vec<f64> = (0..=n).map(|i| ang * i as f64 / n as f64).collect();
    let m = sweep_arc(&ring, (0.0, 0.0), &angles);
    assert!(m.check_closed().is_ok());
    // a polyline sweep slightly undercuts the true arc: the chord factor sin(d/2)/(d/2) of the centroid path
    let d = ang / n as f64;
    let exact = 0.1 * 0.1 * 0.5 * ang * (w5k_math::scalar::sin(d / 2.0) / (d / 2.0));
    assert!((m.signed_volume() / exact - 1.0).abs() < 2e-3, "{} vs {}", m.signed_volume(), exact);
}

use w5k_geo::loft::{chamfer_polygon, sweep_loop};

#[test]
fn a_mitered_loop_sweep_around_a_square_has_the_frame_volume() {
    // a 1 m square outline in the plane z = 0 swept with a rectangular profile s in [-0.02, 0.05], t in [-0.03, 0.01]: a picture frame
    let outline =
        [Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0), Vec3::new(-0.5, 0.5, 0.0)];
    let profile = [[-0.02, -0.03], [0.05, -0.03], [0.05, 0.01], [-0.02, 0.01]];
    for normal in [Vec3::Z, -Vec3::Z] {
        let m = sweep_loop(&outline, normal, &profile);
        assert!(m.check_closed().is_ok());
        let (inner, outer): (f64, f64) = ((1.0 - 0.04_f64) * (1.0 - 0.04), (1.0 + 0.10) * (1.0 + 0.10));
        assert!(close(m.signed_volume(), 0.04 * (outer - inner), 1e-9), "{}", m.signed_volume());
    }
}

#[test]
fn a_chamfered_polygon_has_two_points_per_corner_on_the_original_edges() {
    let q = chamfer_polygon(&[[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]], 0.1);
    assert_eq!(q.len(), 8);
    assert!(
        q.iter().any(|p| (p[0] - 0.1).abs() < 1e-12 && p[1] == 0.0)
            && q.iter().any(|p| (p[0] - 2.0).abs() < 1e-12 && (p[1] - 0.1).abs() < 1e-12)
    );
}
