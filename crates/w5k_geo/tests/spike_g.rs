//! Spike S-G (docs/lanes/geometry/spike-g.md): can per-vertex flags carry the look? Oracles A6 and A7, the bake cost, the BVH.

// The wall clock only times the bake for the spike report; it never feeds generation (clippy.toml allows this in tooling).
#![allow(clippy::disallowed_methods, clippy::float_cmp)]

use std::collections::BTreeMap;
use std::time::Instant;
use w5k_geo::bvh::Bvh;
use w5k_geo::cavity::{cavity_at, wall_foot_fraction};
use w5k_geo::edge::edge_values;
use w5k_geo::mesh::Mesh;
use w5k_math::{scalar, Vec3};

const LO: f64 = 20.0 * std::f64::consts::PI / 180.0;
const HI: f64 = 70.0 * std::f64::consts::PI / 180.0;

fn closed_and_outward(m: &Mesh) -> bool {
    let mut dir: BTreeMap<(u32, u32), i32> = BTreeMap::new();
    for &[a, b, c] in &m.t {
        for (p, q) in [(a, b), (b, c), (c, a)] {
            *dir.entry((p, q)).or_default() += 1;
        }
    }
    dir.iter().all(|(&(p, q), &n)| n == 1 && dir.get(&(q, p)) == Some(&1)) && m.signed_volume() > 0.0
}

fn rect(w: f64, d: f64) -> Vec<[f64; 2]> {
    let (x, z) = (w / 2.0, d / 2.0);
    vec![[-x, -z], [-x, z], [x, z], [x, -z]]
}

/// One rectangular ring of the hood (inset from the 1.5 x 1.2 m outline, at height y): 4 corners, or 8 vertices with a midpoint on each side.
fn ring(inset: f64, y: f64, mids: bool) -> Vec<Vec3> {
    let c = rect(1.5 - 2.0 * inset, 1.2 - 2.0 * inset);
    let mut r = Vec::new();
    for i in 0..4 {
        let (p, q) = (c[i], c[(i + 1) % 4]);
        r.push(Vec3::new(p[0], y, p[1]));
        if mids {
            r.push(Vec3::new((p[0] + q[0]) / 2.0, y, (p[1] + q[1]) / 2.0));
        }
    }
    r
}

/// The hood of the spike: 1.5 x 1.2 x 0.1 m, `chamfer` on the top edges, then one inset loop per entry of `loops` (distance from the
/// chamfer's top edge), each ring with 4 vertices (a loop costs 8 triangles) or 8 (16 triangles).
fn hood(chamfer: f64, loops: &[f64], mids: bool) -> Mesh {
    let h = 0.1;
    let mut rings = vec![(0.0, 0.0), (0.0, h - chamfer), (chamfer, h)];
    rings.extend(loops.iter().map(|l| (chamfer + l, h)));
    let mut m = Mesh::default();
    for &(inset, y) in &rings {
        m.v.extend(ring(inset, y, mids));
    }
    let n = m.v.len() as u32 / rings.len() as u32;
    for r in 0..rings.len() as u32 - 1 {
        for i in 0..n {
            let (a, b) = (n * r + i, n * r + (i + 1) % n);
            m.t.push([a, b, b + n]);
            m.t.push([a, b + n, a + n]);
        }
    }
    let top = n * (rings.len() as u32 - 1);
    for i in 1..n - 1 {
        m.t.push([top, top + i, top + i + 1]);
        m.t.push([0, i + 1, i]);
    }
    m
}

/// The value of a per-vertex field at (x, z) on the top face, by barycentric interpolation: what the GPU does across a triangle.
fn sample_top(m: &Mesh, f: &[f64], x: f64, z: f64) -> f64 {
    for (k, idx) in m.t.iter().enumerate() {
        let p = m.tri(k);
        if p.iter().any(|v| (v.y - 0.1).abs() > 1e-9) {
            continue;
        }
        let det = (p[1].z - p[2].z) * (p[0].x - p[2].x) + (p[2].x - p[1].x) * (p[0].z - p[2].z);
        let l0 = ((p[1].z - p[2].z) * (x - p[2].x) + (p[2].x - p[1].x) * (z - p[2].z)) / det;
        let l1 = ((p[2].z - p[0].z) * (x - p[2].x) + (p[0].x - p[2].x) * (z - p[2].z)) / det;
        let l2 = 1.0 - l0 - l1;
        if l0 >= -1e-12 && l1 >= -1e-12 && l2 >= -1e-12 {
            return l0 * f[idx[0] as usize] + l1 * f[idx[1] as usize] + l2 * f[idx[2] as usize];
        }
    }
    f64::NAN
}

/// Distance from the chamfer's top edge (z = -0.585) along +z at x = 0 until the interpolated flag falls to `level`.
fn band_width(m: &Mesh, f: &[f64], level: f64) -> f64 {
    let z0 = -0.6 + 0.015;
    (0..=1200)
        .map(|i| f64::from(i) * 0.0005)
        .find(|s| sample_top(m, f, 0.0, z0 + s + 1e-9) <= level)
        .unwrap_or(f64::NAN)
}

#[test]
fn box_edge_is_one_and_a_128_gon_wall_is_zero() {
    let b = Mesh::extrude_fan(&rect(1.0, 1.0), 0, 1.0);
    assert!(closed_and_outward(&b));
    let (e, v) = edge_values(&b, LO, HI);
    assert!(v.iter().all(|&x| (x - 1.0).abs() < 1e-12));
    assert_eq!(e.iter().filter(|e| e.2 > 0.5).count(), 12);
    let n = 128;
    let ring: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let a = -2.0 * std::f64::consts::PI * f64::from(i) / f64::from(n);
            [scalar::cos(a), scalar::sin(a)]
        })
        .collect();
    let c = Mesh::extrude_fan(&ring, 0, 1.0);
    assert!(closed_and_outward(&c));
    let (e, _) = edge_values(&c, LO, HI);
    // vertical wall edges join i and i+n; the rim edges are the 1.0 ones: 2 x 128, everything else is 0
    assert_eq!(e.iter().filter(|e| e.2 > 0.99).count(), 2 * n as usize);
    assert!(e.iter().filter(|e| e.1 - e.0 == n as u32).all(|e| e.2 == 0.0));
}

#[test]
fn chamfer_faces_get_half_a_ramp_and_concave_creases_get_zero() {
    // a 2 x 2 prism with one vertical edge chamfered at 45 degrees: the two edges beside the chamfer turn 45 degrees
    let p = Mesh::extrude_fan(&[[-1.0, -1.0], [-1.0, 0.5], [-0.5, 1.0], [1.0, 1.0], [1.0, -1.0]], 0, 1.0);
    assert!(closed_and_outward(&p));
    let (e, _) = edge_values(&p, LO, HI);
    let up = |a: u32, b: u32| e.iter().find(|e| (e.0, e.1) == (a, b)).map(|e| e.2).unwrap();
    assert!((up(1, 6) - 0.5).abs() < 1e-12 && (up(2, 7) - 0.5).abs() < 1e-12, "{:?}", (up(1, 6), up(2, 7)));
    assert!((up(0, 5) - 1.0).abs() < 1e-12);
    // an L: the inner corner is vertex 3, its vertical crease joins 3 and 9
    let l = Mesh::extrude_fan(&[[0.0, 0.0], [0.0, 2.0], [1.0, 2.0], [1.0, 1.0], [2.0, 1.0], [2.0, 0.0]], 3, 1.0);
    assert!(closed_and_outward(&l));
    let (e, _) = edge_values(&l, LO, HI);
    assert_eq!(e.iter().find(|e| (e.0, e.1) == (3, 9)).map(|e| e.2), Some(0.0));
}

#[test]
fn spike_hood_edge_band_and_triangle_cost() {
    for mids in [false, true] {
        for loops in [vec![], vec![0.05], vec![0.025, 0.05], vec![0.0125, 0.025, 0.05]] {
            let m = hood(0.015, &loops, mids);
            assert!(closed_and_outward(&m));
            let (_, v) = edge_values(&m, LO, HI);
            println!(
                "S-G hood mids={mids} loops={loops:?}: {} triangles, mid-edge value {:.3}, band to 0.25 = {:.3} m, to 0.05 = {:.3} m",
                m.t.len(), sample_top(&m, &v, 0.0, -0.6 + 0.015 + 1e-9), band_width(&m, &v, 0.25), band_width(&m, &v, 0.05)
            );
        }
    }
    if let Ok(dir) = std::env::var("W5K_SPIKE_OUT") {
        let mut csv = String::from("s_m,none,loop_0p05,loop_0p025\n");
        let fields: Vec<(Mesh, Vec<f64>)> = [vec![], vec![0.05], vec![0.025]]
            .iter()
            .map(|l| {
                let m = hood(0.015, l, true);
                let v = edge_values(&m, LO, HI).1;
                (m, v)
            })
            .collect();
        for i in 0..=120 {
            let s = f64::from(i) * 0.0025;
            let row: Vec<String> =
                fields.iter().map(|(m, v)| format!("{:.5}", sample_top(m, v, 0.0, -0.585 + s + 1e-9))).collect();
            csv += &format!("{s:.4},{}\n", row.join(","));
        }
        std::fs::write(format!("{dir}/s-g-edge.csv"), csv).unwrap();
    }
    let grid = 2.0 * (1.5_f64 / 0.05).ceil() * (1.2_f64 / 0.05).ceil();
    println!("S-G hood by plain subdivision at 5 cm pitch: {grid} triangles for the top face alone");
    let m = hood(0.015, &[0.05], true);
    let (_, v) = edge_values(&m, LO, HI);
    assert!((band_width(&m, &v, 0.05) - 0.05).abs() < 0.01);
    assert!((sample_top(&m, &v, 0.0, -0.6 + 0.015 + 1e-9) - 0.5).abs() < 1e-6);
}

fn relief(n: usize, pitch: f64) -> Mesh {
    let mut m = Mesh::default();
    for j in 0..n {
        for i in 0..n {
            let (x, z) = (i as f64 * pitch, j as f64 * pitch);
            m.v.push(Vec3::new(x, 0.3 * scalar::sin(x * 9.0) * scalar::cos(z * 7.0), z));
        }
    }
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as u32;
            let (b, c, d) = (a + 1, a + n as u32, a + n as u32 + 1);
            m.t.push([a, c, b]);
            m.t.push([b, c, d]);
        }
    }
    m
}

#[test]
fn bvh_matches_brute_force_and_the_bake_time_is_recorded() {
    let m = relief(123, 0.04);
    assert_eq!(m.t.len(), 29_768);
    let nrm = m.vertex_normals();
    let t0 = Instant::now();
    let bvh = Bvh::build(&m);
    println!("S-G BVH build for {} triangles: {:.1} ms", m.t.len(), t0.elapsed().as_secs_f64() * 1e3);
    for rays in [128u32, 256, 1024] {
        let t = Instant::now();
        let total: f64 = (0..m.v.len()).map(|i| cavity_at(&bvh, m.v[i], nrm[i], rays, 0.5, 1e-6)).sum();
        println!(
            "S-G cavity bake, {} vertices, {rays} rays, BVH: {:.2} s (mean {:.3})",
            m.v.len(),
            t.elapsed().as_secs_f64(),
            total / m.v.len() as f64
        );
    }
    let subset: Vec<usize> = (0..m.v.len()).step_by(m.v.len() / 20).take(20).collect();
    let t = Instant::now();
    let mut same = 0;
    for &i in &subset {
        let (o, nn) = (m.v[i] + nrm[i] * 1e-6, nrm[i]);
        let tan = nn.cross(if nn.x.abs() < 0.9 { Vec3::X } else { Vec3::Y }).normalized_or_zero();
        let bi = nn.cross(tan);
        for k in 0..128 {
            let ang = 2.0 * std::f64::consts::PI * f64::from(k) / 128.0;
            let dir = (tan * scalar::cos(ang) + bi * scalar::sin(ang)) * 0.6 + nn * 0.8;
            same += usize::from(bvh.any_hit(o, dir, 0.5) == bvh.any_hit_brute(o, dir, 0.5));
        }
    }
    println!(
        "S-G brute force vs BVH on {} rays: {same} agree; brute-force share of the time {:.2} s (BVH included)",
        subset.len() * 128,
        t.elapsed().as_secs_f64()
    );
    assert_eq!(same, subset.len() * 128);
    let t = Instant::now();
    for &i in &subset {
        for k in 0..128 {
            let ang = 2.0 * std::f64::consts::PI * f64::from(k) / 128.0;
            let dir = Vec3::new(scalar::cos(ang) * 0.6, 0.8, scalar::sin(ang) * 0.6);
            let _ = bvh.any_hit_brute(m.v[i] + Vec3::Y * 1e-6, dir, 0.5);
        }
    }
    let per_vertex = t.elapsed().as_secs_f64() / subset.len() as f64;
    println!(
        "S-G brute force alone: {:.4} s per vertex at 128 rays, so {:.0} s for {} vertices",
        per_vertex,
        per_vertex * m.v.len() as f64,
        m.v.len()
    );
}

#[test]
fn cavity_is_zero_on_an_open_plate_and_matches_the_circular_segment_at_the_foot_of_a_wall() {
    let q = |a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]| {
        let mut m = Mesh::default();
        m.v.extend([a, b, c, d].map(|p| Vec3::new(p[0], p[1], p[2])));
        m.t.extend([[0, 1, 2], [0, 2, 3]]);
        m
    };
    let mut plate = q([-30.0, 0.0, -30.0], [-30.0, 0.0, 30.0], [30.0, 0.0, 30.0], [30.0, 0.0, -30.0]);
    let open = Bvh::build(&plate);
    assert_eq!(cavity_at(&open, Vec3::new(1.0, 0.0, 0.0), Vec3::Y, 1024, 0.5, 1e-4), 0.0);
    let wall = q([0.0, 0.0, -30.0], [0.0, 50.0, -30.0], [0.0, 50.0, 30.0], [0.0, 0.0, 30.0]);
    let base = plate.v.len() as u32;
    plate.v.extend(wall.v.iter().copied());
    plate.t.extend(wall.t.iter().map(|t| t.map(|i| i + base)));
    let bvh = Bvh::build(&plate);
    let eps = 1e-4; // the sample sits 0.1 mm off the crease and 0.1 mm above the plate: exactly on a surface a ray has no side to leave by
    let mut csv = String::from("d_m,bake_1024,closed_form\n");
    for rays in [128u32, 256, 1024] {
        let mut worst: f64 = 0.0;
        for step in 0..=10 {
            let d = 0.05 * f64::from(step) + eps;
            let got = cavity_at(&bvh, Vec3::new(d, 0.0, 0.0), Vec3::Y, rays, 0.5, eps);
            worst = worst.max((got - wall_foot_fraction(d, 0.5)).abs());
            if rays == 1024 {
                csv += &format!("{d:.4},{got:.5},{:.5}\n", wall_foot_fraction(d, 0.5));
            }
        }
        println!("S-G A7 worst error with {rays} rays: {worst:.4}");
        if rays == 1024 {
            assert!(worst < 0.02);
            if let Ok(dir) = std::env::var("W5K_SPIKE_OUT") {
                std::fs::write(format!("{dir}/s-g-cavity.csv"), &csv).unwrap();
            }
        }
    }
    assert!((cavity_at(&bvh, Vec3::new(eps, 0.0, 0.0), Vec3::Y, 1024, 0.5, eps) - 0.5).abs() < 0.02);
}
