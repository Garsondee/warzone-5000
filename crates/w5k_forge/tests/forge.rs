//! Part Forge tests: geometry invariants, physical stats against analytic answers, armour against the
//! t / cos(angle) law, GLB round trip, the repository's content, and repeatability.

use std::path::PathBuf;

use w5k_forge::build::{box_points, cylinder_points};
use w5k_forge::convex::Convex;
use w5k_forge::geom::{v3, Xform, M3, V3};
use w5k_forge::schema::{MaterialLibrary, PartDef};
use w5k_forge::Forge;

fn lib() -> MaterialLibrary {
    ron::from_str(
        r#"(
            materials: {
                "steel": (density: 7850, hardness: 1.0),
                "soft": (density: 7850, hardness: 0.5),
                "light": (density: 1000, hardness: 0.1),
            },
            palettes: {},
        )"#,
    )
    .unwrap()
}

fn part(src: &str) -> PartDef {
    ron::from_str(&format!("#![enable(implicit_some)]\n{src}")).unwrap_or_else(|e| panic!("{e}\n{src}"))
}

fn forge(parts: &[&str]) -> Forge {
    Forge::from_parts(lib(), parts.iter().map(|s| part(s)).collect()).unwrap()
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-12)
}

// ---------------------------------------------------------------- geometry

#[test]
fn box_is_closed_with_exact_volume() {
    let c = Convex::from_points(&box_points([2.0, 3.0, 4.0], [1.0, 1.0], [0.0, 0.0]));
    let p = c.polyhedron();
    assert_eq!(p.faces.len(), 6);
    assert_eq!(p.verts.len(), 8);
    assert!(p.is_closed());
    assert!(rel(p.volume(), 24.0) < 1e-12);
}

#[test]
fn prism_volume_matches_polygon_area() {
    let (r, len, n) = (0.7, 2.5, 12u32);
    let p = Convex::from_points(&cylinder_points(r, len, n, 1.0)).polyhedron();
    let area = 0.5 * n as f64 * r * r * (std::f64::consts::TAU / n as f64).sin();
    assert!(p.is_closed());
    assert!(rel(p.volume(), area * len) < 1e-10);
}

#[test]
fn bevels_stay_closed_and_remove_a_little() {
    let c = Convex::from_points(&box_points([1.0, 1.0, 1.0], [1.0, 1.0], [0.0, 0.0]));
    let b = c.beveled(0.05, 40.0);
    let p = b.polyhedron();
    assert!(p.is_closed());
    // 6 faces + 12 edge bevels; the corners close up where three bevels meet.
    assert_eq!(p.faces.len(), 18);
    let v = p.volume();
    // Each edge loses a triangular prism of cross-section c^2 / 2; the prisms overlap at the corners, so
    // a little less than their sum is removed.
    let edges_only = 1.0 - 12.0 * 0.05 * 0.05 / 2.0 * 1.0;
    assert!(v > edges_only - 1e-9 && v < edges_only + 0.004, "volume {v}");
    for v in &p.verts {
        assert!(b.contains(*v, 1e-6));
    }
}

#[test]
fn mirror_and_rotation_preserve_volume_and_outward_faces() {
    let c = Convex::from_points(&box_points([1.0, 2.0, 3.0], [0.6, 0.8], [0.2, -0.1])).beveled(0.04, 40.0);
    let v0 = c.polyhedron().volume();
    for x in [
        Xform::new(M3::scale(v3(-1.0, 1.0, 1.0)), v3(1.0, 2.0, 3.0)),
        Xform::new(M3::euler_deg([30.0, 45.0, 10.0]), v3(-2.0, 0.5, 0.0)),
    ] {
        let p = c.transformed(&x).polyhedron();
        assert!(p.is_closed());
        assert!(rel(p.volume(), v0) < 1e-9, "signed volume must stay positive (faces outward)");
    }
}

// ---------------------------------------------------------------- mass

#[test]
fn solid_box_mass_centre_and_inertia() {
    let f = forge(&[r#"(id: "b", name: "b", shapes: [Box(size: (1.0, 2.0, 0.5), at: (0.3, 1.0, -0.2))])"#]);
    let m = f.build_part("b").unwrap().mass;
    let mass = 7850.0 * 1.0;
    assert!(rel(m.mass_kg, mass) < 1e-9, "mass {}", m.mass_kg);
    let c = m.centre_of_mass;
    assert!((c[0] - 0.3).abs() < 1e-3 && (c[1] - 1.0).abs() < 1e-3 && (c[2] + 0.2).abs() < 1e-3, "{c:?}");
    let (a, b, d) = (1.0f64, 2.0f64, 0.5f64);
    let ixx = mass * (b * b + d * d) / 12.0;
    let iyy = mass * (a * a + d * d) / 12.0;
    let izz = mass * (a * a + b * b) / 12.0;
    assert!(rel(m.inertia[0][0], ixx) < 0.02, "ixx {} vs {ixx}", m.inertia[0][0]);
    assert!(rel(m.inertia[1][1], iyy) < 0.02, "iyy {} vs {iyy}", m.inertia[1][1]);
    assert!(rel(m.inertia[2][2], izz) < 0.02, "izz {} vs {izz}", m.inertia[2][2]);
    assert!(m.inertia[0][1].abs() < 1e-6 * ixx);
}

#[test]
fn hollow_box_weighs_its_shell() {
    let f = forge(&[r#"(id: "h", name: "h", shapes: [Box(size: (2.0, 1.0, 3.0), shell: 0.05)])"#]);
    let m = f.build_part("h").unwrap().mass;
    let shell_v = 2.0 * 1.0 * 3.0 - 1.9 * 0.9 * 2.9;
    assert!(rel(m.mass_kg, 7850.0 * shell_v) < 1e-9, "mass {}", m.mass_kg);
    assert!(rel(m.internal_m3, 1.9 * 0.9 * 2.9) < 0.05, "internal {}", m.internal_m3);
}

#[test]
fn thin_barrel_mass_is_exact_on_a_coarse_grid() {
    // A 4 m barrel 0.1 m across in a 64-cell grid: cells are ~ 63 mm, so centre sampling alone is badly off.
    let f = forge(&[r#"(id: "g", name: "g", voxels: 64, shapes: [
        Cylinder(radius: 0.05, length: 4.0, axis: Z, segments: 10),
        Box(size: (0.4, 0.4, 0.4), at: (0, 0, 2.2)),
    ])"#]);
    let m = f.build_part("g").unwrap().mass;
    let barrel = 0.5 * 10.0 * 0.05 * 0.05 * (std::f64::consts::TAU / 10.0).sin() * 4.0;
    let block = 0.4f64.powi(3);
    assert!(rel(m.mass_kg, 7850.0 * (barrel + block)) < 0.01, "mass {} vs {}", m.mass_kg, 7850.0 * (barrel + block));
}

#[test]
fn overlapping_shells_merge_without_hidden_bulkheads() {
    let one = r#"(id: "a", name: "a", shapes: [Box(size: (2.0, 1.0, 2.0), shell: 0.05)])"#;
    let two = r#"(id: "b", name: "b", shapes: [
        Box(size: (2.0, 1.0, 2.0), shell: 0.05),
        Box(size: (2.0, 1.0, 2.0), at: (1.0, 0, 0), shell: 0.05),
    ])"#;
    let f = forge(&[one, two]);
    let a = f.build_part("a").unwrap().mass;
    let b = f.build_part("b").unwrap().mass;
    // The union is a 3 x 1 x 2 box: its shell, not two shells and a wall in the middle.
    let union_shell = 3.0 * 1.0 * 2.0 - 2.9 * 0.9 * 1.9;
    assert!(rel(b.mass_kg, 7850.0 * union_shell) < 0.03, "mass {} vs {}", b.mass_kg, 7850.0 * union_shell);
    assert!(b.mass_kg < 2.0 * a.mass_kg);
}

#[test]
fn solid_inside_a_shell_adds_its_own_mass() {
    let f = forge(&[r#"(id: "s", name: "s", shapes: [
        Box(size: (2.0, 2.0, 2.0), shell: 0.05),
        Box(size: (0.5, 0.5, 0.5), mat: "light"),
    ])"#]);
    let m = f.build_part("s").unwrap().mass;
    let expect = 7850.0 * (8.0 - 1.9f64.powi(3)) + 1000.0 * 0.125;
    assert!(rel(m.mass_kg, expect) < 0.01, "mass {} vs {expect}", m.mass_kg);
}

// ---------------------------------------------------------------- armour

/// A large hollow box with its front face tilted back by `tilt` degrees from vertical.
fn tilted_box(id: &str, tilt: f64, mat: &str) -> String {
    let (w, h, l) = (10.0, 4.0, 12.0);
    let back = h * tilt.to_radians().tan(); // how far the top front edge sits behind the bottom one
    let mut pts = Vec::new();
    for x in [-w / 2.0, w / 2.0] {
        pts.push(format!("({x}, 0, {})", -l / 2.0));
        pts.push(format!("({x}, {h}, {})", -l / 2.0 + back));
        pts.push(format!("({x}, 0, {})", l / 2.0));
        pts.push(format!("({x}, {h}, {})", l / 2.0));
    }
    format!(r#"(id: "{id}", name: "{id}", shapes: [Hull(points: [{}], mat: "{mat}", shell: 0.05)])"#, pts.join(", "))
}

#[test]
fn armour_follows_thickness_over_cosine() {
    let tilts = [0.0, 30.0, 45.0, 60.0];
    let srcs: Vec<String> = tilts.iter().enumerate().map(|(i, t)| tilted_box(&format!("t{i}"), *t, "steel")).collect();
    let f = forge(&srcs.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let mut last = 0.0;
    for (i, t) in tilts.iter().enumerate() {
        let a = f.build_part(&format!("t{i}")).unwrap().armour;
        let front = a.at(0.0, 0.0);
        let expect = 50.0 / t.to_radians().cos();
        assert!(rel(front.weak_mm, expect) < 0.03, "tilt {t}: weak {} vs {expect}", front.weak_mm);
        assert!(rel(front.mean_mm, expect) < 0.05, "tilt {t}: mean {} vs {expect}", front.mean_mm);
        assert!(rel(front.median_mm, expect) < 0.03, "tilt {t}: median {} vs {expect}", front.median_mm);
        assert!(front.mean_mm > last);
        last = front.mean_mm;
        // The side walls are vertical whatever the front does.
        assert!(rel(a.at(90.0, 0.0).weak_mm, 50.0) < 0.03);
    }
}

#[test]
fn hardness_scales_protection() {
    let f = forge(&[&tilted_box("hard", 0.0, "steel"), &tilted_box("soft", 0.0, "soft")]);
    let hard = f.build_part("hard").unwrap().armour.at(0.0, 0.0).weak_mm;
    let soft = f.build_part("soft").unwrap().armour.at(0.0, 0.0).weak_mm;
    assert!(rel(soft, hard * 0.5) < 0.02, "{soft} vs {hard}");
}

#[test]
fn silhouette_area_matches_projection() {
    let f = forge(&[r#"(id: "b", name: "b", shapes: [Box(size: (3.0, 2.0, 5.0), shell: 0.05)])"#]);
    let a = f.build_part("b").unwrap().armour;
    // 80 rays across the bounding sphere quantise each edge to the ray spacing (a few per cent).
    assert!(rel(a.at(0.0, 0.0).area_m2, 6.0) < 0.05, "front {}", a.at(0.0, 0.0).area_m2);
    assert!(rel(a.at(90.0, 0.0).area_m2, 10.0) < 0.05, "side {}", a.at(90.0, 0.0).area_m2);
    assert!(rel(a.at(0.0, 90.0).area_m2, 15.0) < 0.05, "top {}", a.at(0.0, 90.0).area_m2);
}

// ---------------------------------------------------------------- vehicles

const HULL: &str = r#"(id: "hull", name: "hull", shapes: [Box(size: (2.0, 1.0, 4.0), at: (0, 1.0, 0), shell: 0.04)],
    sockets: [
        (name: "top", at: (0, 1.5, 0)),
        (name: "right", at: (1.0, 1.0, 0), normal: (1, 0, 0)),
        (name: "left", at: (-1.0, 1.0, 0), normal: (-1, 0, 0)),
    ])"#;
const POD: &str = r#"(id: "pod", name: "pod", shapes: [Box(size: (0.4, 0.8, 3.0), at: (0.2, 0, 0.3), mat: "light")],
    sockets: [(name: "mount", at: (0, 0, 0), normal: (-1, 0, 0))],
    function: (locomotion: Wheels, load_kg: 20000, rolling: 0.03))"#;
const BOX: &str = r#"(id: "top", name: "top", shapes: [Box(size: (1.0, 0.5, 1.0), at: (0, 0.25, 0.4))],
    sockets: [(name: "mount", at: (0, 0, 0), normal: (0, -1, 0))],
    function: (power_kw: 200))"#;

#[test]
fn vehicle_mass_is_the_sum_of_its_parts() {
    let f = forge(&[HULL, POD, BOX]);
    let d = ron::from_str(
        r#"(id: "v", name: "v", hull: "hull", attach: [
            (socket: "right", part: "pod"), (socket: "left", part: "pod", mirror: true),
            (socket: "top", part: "top", spin: 30),
        ])"#,
    )
    .unwrap();
    let (built, asm, sheet) = f.build_design(&d);
    assert!(asm.errors.is_empty(), "{:?}", asm.errors);
    let parts = ["hull", "pod", "pod", "top"].iter().map(|p| f.part_mass(p).unwrap().mass_kg).sum::<f64>();
    assert!(rel(built.mass.mass_kg, parts) < 1e-12);
    assert!(sheet.problems.is_empty(), "{:?}", sheet.problems);
    // Mirrored pods sit symmetrically, so the centre of mass is offset only by the spun top box.
    let c = built.mass.centre_of_mass;
    let top = f.part_mass("top").unwrap().mass_kg;
    // Spinning +30 deg about +Y swings the box's +Z offset toward +X.
    let expect_x = top * 0.4 * 30f64.to_radians().sin() / built.mass.mass_kg;
    assert!((c[0] - expect_x).abs() < 1e-3, "com x {} vs {expect_x}", c[0]);
    // Pods attach outward: mirrored left pod spans x in [-1.4, -1.0].
    assert!((built.mass.bounds_min[0] + 1.4).abs() < 0.05, "{:?}", built.mass.bounds_min);
    assert!(sheet.top_speed_kmh > 0.0);
}

#[test]
fn vehicle_budget_problems_are_reported() {
    let f = forge(&[HULL, POD, BOX]);
    let d = ron::from_str(r#"(id: "v", name: "v", hull: "hull", attach: [(socket: "top", part: "top"), (socket: "nope", part: "pod")])"#).unwrap();
    let (_, _, sheet) = f.build_design(&d);
    assert!(sheet.problems.iter().any(|p| p.contains("no locomotion")), "{:?}", sheet.problems);
    assert!(sheet.problems.iter().any(|p| p.contains("no socket 'nope'")), "{:?}", sheet.problems);
    assert_eq!(sheet.top_speed_kmh, 0.0);
}

#[test]
fn power_balance_speed_solves_the_cubic() {
    use w5k_forge::assemble::power_balance_speed;
    let (p, c1, c3) = (300_000.0, 9000.0, 4.0);
    let v = power_balance_speed(p, c1, c3);
    assert!(rel(c1 * v + c3 * v * v * v, p) < 1e-9);
    assert_eq!(power_balance_speed(0.0, c1, c3), 0.0);
}

// ---------------------------------------------------------------- export

#[test]
fn glb_round_trips_through_a_gltf_reader() {
    use w5k_forge::gltf::{write_glb, GlbSocket};
    let f = forge(&[BOX]);
    let built = f.build_part("top").unwrap();
    let colours = [[0.5f32; 3]; 8];
    let sockets = [GlbSocket { name: "mount".into(), at: V3::ZERO, normal: v3(0.0, -1.0, 0.0), forward: v3(0.0, 0.0, -1.0) }];
    let bytes = write_glb("top", &built.mesh, &colours, &sockets);
    let g = gltf::Gltf::from_slice(&bytes).expect("valid glTF");
    let blob = g.blob.as_deref().expect("binary chunk");
    let mesh = g.document.meshes().next().unwrap();
    let prim = mesh.primitives().next().unwrap();
    let reader = prim.reader(|_| Some(blob));
    assert_eq!(reader.read_positions().unwrap().count(), built.mesh.positions.len());
    assert_eq!(reader.read_indices().unwrap().into_u32().count(), built.mesh.indices.len());
    assert!(prim.attributes().any(|(s, _)| matches!(s, gltf::Semantic::Extras(ref n) if n == "W5K")));
    let names: Vec<_> = g.document.nodes().filter_map(|n| n.name().map(|s| s.to_string())).collect();
    assert!(names.contains(&"socket_mount".to_string()), "{names:?}");
}

// ---------------------------------------------------------------- content and repeatability

fn content_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn repository_content_builds_cleanly() {
    let f = Forge::load(&content_dir()).expect("content loads");
    assert!(!f.parts.is_empty());
    for (id, p) in &f.parts {
        for (i, piece) in p.pieces.iter().enumerate() {
            assert!(piece.poly.is_closed(), "{id}: piece {i} is not closed");
            assert!(piece.poly.volume() > 0.0, "{id}: piece {i} has no volume");
        }
        if !p.def.sockets.is_empty() && p.def.category != w5k_forge::schema::Category::Hull {
            assert!(p.def.sockets.iter().any(|s| s.name == "mount"), "{id}: attachable part without a 'mount' socket");
        }
    }
    for (id, d) in &f.designs {
        let (built, asm, sheet) = f.build_design(d);
        assert!(asm.errors.is_empty(), "{id}: {:?}", asm.errors);
        assert!(sheet.problems.is_empty(), "{id}: {:?}", sheet.problems);
        assert!(built.mesh.triangle_count() < 20_000, "{id}: triangle budget");
    }
}

#[test]
fn builds_are_repeatable() {
    let f = Forge::load(&content_dir()).unwrap();
    let id = f.parts.keys().next().unwrap().clone();
    let a = f.build_part(&id).unwrap();
    let b = f.build_part(&id).unwrap();
    assert_eq!(a.mesh.positions, b.mesh.positions);
    assert_eq!(a.mesh.ao, b.mesh.ao);
    assert_eq!(ron::to_string(&a.armour).unwrap(), ron::to_string(&b.armour).unwrap());
    assert_eq!(a.mass.mass_kg.to_bits(), b.mass.mass_kg.to_bits());
}

#[test]
fn hollow_legs_are_not_vital_space() {
    // The same hollow box as a hull (vital inside) and as a leg (just air inside).
    let f = forge(&[
        r#"(id: "core", name: "core", category: Hull, shapes: [Box(size: (1.0, 1.0, 1.0), shell: 0.05)])"#,
        r#"(id: "tube", name: "tube", category: Leg, shapes: [Box(size: (1.0, 1.0, 1.0), shell: 0.05)])"#,
    ]);
    let core = f.build_part("core").unwrap();
    let tube = f.build_part("tube").unwrap();
    assert!(core.armour.at(0.0, 0.0).vital_m2 > 0.5);
    assert_eq!(tube.armour.at(0.0, 0.0).vital_m2, 0.0);
    assert!(tube.armour.at(0.0, 0.0).area_m2 > 0.9);
    assert_eq!(tube.mass.internal_m3, 0.0);
    // Mass is the same either way: the walls are the walls.
    assert!(rel(tube.mass.mass_kg, core.mass.mass_kg) < 1e-9);
}

#[test]
fn rotorcraft_pay_hover_power() {
    use w5k_forge::assemble::hover_power_w;
    // Momentum theory: doubling the disc area cuts ideal hover power by sqrt(2).
    let (a, b) = (hover_power_w(1000.0, 1.0), hover_power_w(1000.0, 2.0));
    assert!(rel(a / b, 2f64.sqrt()) < 1e-12);
    let f = Forge::load(&content_dir()).unwrap();
    let (_, _, sheet) = f.build_design(&f.designs["drone_scout"]);
    assert!(sheet.hover_kw > 0.1 && sheet.hover_kw < sheet.power_kw, "hover {} of {}", sheet.hover_kw, sheet.power_kw);
}
