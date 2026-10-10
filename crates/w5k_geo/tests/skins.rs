//! The skins of the game garage (`utility_4x4`, `scout_4x4`): every part a closed outward solid for random in-range dimensions, nothing
//! floating, wheels clear of the shell, dimensions equal to FORGE's physics definitions, one joint layout for all of them (VIEWER retargets
//! skins by joint index), the triangle budget, and deterministic bytes.

mod common;

use common::{inside, measure, overlap};
use w5k_contract::def::VehicleDef;
use w5k_contract::render::{JointAxisKind, NodeRole};
use w5k_geo::export::render_rig;
use w5k_geo::flags::FlagParams;
use w5k_geo::mesh::Mesh;
use w5k_geo::part::Part;
use w5k_geo::raster::{render, Camera, Item, Mode};
use w5k_geo::skin::{Skin, IDS};
use w5k_math::{Pcg32, StateHasher, Vec3};

/// The skins whose physics definition FORGE has authored (the Mule's skin, the utility truck, has a dossier-based box instead).
const AUTHORED: [&str; 2] = ["scout_4x4", "hauler_4x4"];

fn skin(id: &str) -> Skin {
    Skin::for_id(id).unwrap_or_else(|| panic!("no skin {id}"))
}

fn def(id: &str) -> VehicleDef {
    let path = format!("{}/../../content/vehicles/game/{id}.ron", env!("CARGO_MANIFEST_DIR"));
    ron::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}")))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A random in-range variation of a skin: the box, the track, the wheel and the axles (kept at their fractions of the length).
fn varied(base: &Skin, r: &mut Pcg32) -> Skin {
    let mut s = base.clone();
    let l0 = base.dims.length_m;
    s.dims.length_m *= r.range_f64(0.9, 1.1);
    s.dims.width_m *= r.range_f64(0.92, 1.08);
    s.dims.height_m *= r.range_f64(0.92, 1.1);
    s.dims.track_m = s.dims.width_m * (base.dims.track_m / base.dims.width_m) * r.range_f64(0.95, 1.05);
    s.dims.ground_clearance_m *= r.range_f64(0.85, 1.15);
    let k = r.range_f64(0.94, 1.06);
    s.dims.wheel.outer_radius_m *= k;
    s.dims.wheel.rim_radius_m *= k;
    s.dims.wheel.width_m *= r.range_f64(0.9, 1.1);
    s.axles_z = base.axles_z.iter().map(|z| (z + l0 / 2.0) / l0 * s.dims.length_m - s.dims.length_m / 2.0).collect();
    s.dims.wheelbase_m = s.axles_z[s.axles_z.len() - 1] - s.axles_z[0];
    s
}

fn check_parts(parts: &[Part], what: &str) {
    for p in parts {
        assert!(p.mesh.check_closed().is_ok(), "{what}: {} is not closed", p.name);
        assert!(p.mesh.signed_volume() > 0.0, "{what}: {} faces inward", p.name);
        for k in 0..p.mesh.t.len() {
            let [a, b, c] = p.mesh.tri(k);
            assert!((b - a).cross(c - a).length() / 2.0 >= 1e-9, "{what}: {} has a sliver", p.name);
        }
    }
}

#[test]
fn every_part_of_every_skin_is_a_closed_outward_solid_for_random_in_range_dimensions() {
    for id in IDS {
        let base = skin(id);
        check_parts(&base.parts(0), id);
        let mut r = Pcg32::new(7, 31);
        for i in 0..40 {
            let v = varied(&base, &mut r);
            check_parts(&v.parts(0), &format!("{id} variation {i}: {:?}", v.dims));
        }
    }
}

#[test]
fn nothing_floats_on_any_skin_and_the_wheels_clear_the_shell() {
    for id in IDS {
        let s = skin(id);
        let parts = s.parts(0);
        let hosts: Vec<Mesh> =
            parts.iter().filter(|p| p.role == NodeRole::Hull && !p.fitting).map(Part::in_hull_frame).collect();
        let mut attached = hosts.clone();
        let mut pending: Vec<&Part> = parts.iter().filter(|p| p.fitting).collect();
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
        assert!(floating.is_empty(), "{id}: floating fittings {floating:?}");
        let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
        for p in parts.iter().filter(|p| p.role == NodeRole::Wheel) {
            for &v in p.in_hull_frame().v.iter().step_by(5) {
                assert!(!inside(&shell, v), "{id}: {} reaches into the shell at {v:?}", p.name);
            }
        }
    }
}

#[test]
fn the_stand_in_dimensions_agree_with_forges_definitions_and_the_skins_are_built_to_them() {
    for id in AUTHORED {
        let (s, from_def) = (skin(id), Skin::from_def(&def(id)).unwrap());
        // the stand-in file and the definition say the same numbers
        let (a, b) = (s.dims, from_def.dims);
        for (what, x, y) in [
            ("length", a.length_m, b.length_m),
            ("width", a.width_m, b.width_m),
            ("height", a.height_m, b.height_m),
            ("wheelbase", a.wheelbase_m, b.wheelbase_m),
            ("track", a.track_m, b.track_m),
            ("clearance", a.ground_clearance_m, b.ground_clearance_m),
            ("tyre radius", a.wheel.outer_radius_m, b.wheel.outer_radius_m),
            ("tyre width", a.wheel.width_m, b.wheel.width_m),
        ] {
            assert!((x - y).abs() < 2e-3, "{id}: stand-in {what} {x} vs the definition's {y}");
        }
        assert!(
            s.axles_z.iter().zip(&from_def.axles_z).all(|(x, y)| (x - y).abs() < 2e-3) && s.steered == from_def.steered
        );
        // and what the parts measure is what was asked for, in the definition's terms
        let m = measure(&from_def, &from_def.parts(1));
        let d = from_def.dims;
        for (what, got, want) in [
            ("length", m.length_m, d.length_m),
            ("width", m.width_m, d.width_m),
            ("height", m.height_m, d.height_m),
            ("wheelbase", m.wheelbase_m, d.wheelbase_m),
            ("track", m.track_m, d.track_m),
            ("wheel diameter", m.wheel_diameter_m, 2.0 * d.wheel.outer_radius_m),
            ("clearance", m.clearance_m, d.ground_clearance_m),
        ] {
            assert!((got / want - 1.0).abs() < 0.015, "{id}: {what} is {got:.3} m, the definition says {want:.3} m");
        }
    }
}

#[test]
fn every_skin_has_the_joint_layout_of_the_utility_truck_and_stays_inside_the_triangle_budget() {
    let layout = |id: &str| {
        let r = render_rig(id, &skin(id).parts(1), &FlagParams::default_params());
        r.validate().expect("RenderRig::validate");
        assert!(
            r.triangle_count() < 40_000,
            "{id}: {} triangles, budget 40,000 for a wheeled vehicle",
            r.triangle_count()
        );
        println!("{id}: {} triangles", r.triangle_count());
        let joints: Vec<(String, NodeRole, JointAxisKind, Vec3, usize)> =
            r.nodes.iter().filter_map(|n| n.joint.map(|j| (n.name.clone(), n.role, j.kind, j.axis, j.index))).collect();
        (r.joint_count, joints)
    };
    let reference = layout("utility_4x4");
    assert_eq!(reference.0, 10, "4 spins, 2 steers, 4 travels");
    for id in IDS {
        assert_eq!(layout(id), reference, "{id} must have the same joints, in the same order, as utility_4x4");
    }
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
fn skin_generation_is_deterministic_same_params_same_bytes() {
    for id in AUTHORED {
        let a = fingerprint(&skin(id).parts(1));
        assert_eq!(a, fingerprint(&skin(id).parts(1)));
        assert_ne!(a, fingerprint(&skin(id).parts(0)));
        let path = format!("{}/tests/golden/{id}.hash", env!("CARGO_MANIFEST_DIR"));
        if std::env::var("W5K_BLESS").is_ok() {
            std::fs::write(&path, format!("{a:016x}\n")).unwrap();
        }
        let golden = std::fs::read_to_string(&path).expect("golden hash (bless with W5K_BLESS=1)");
        assert_eq!(
            format!("{a:016x}"),
            golden.trim(),
            "{id}: the generator changed: bless deliberately with W5K_BLESS=1"
        );
    }
}

/// The side silhouette of a skin at a fixed scale (20 px per metre, the ground on one row, the hull centre on one column): true where
/// something is drawn. This is what a vehicle looks like from far away.
fn silhouette(s: &Skin) -> Vec<bool> {
    let meshes: Vec<Mesh> = s.parts(0).iter().map(Part::in_hull_frame).collect();
    let items: Vec<Item> =
        meshes.iter().map(|m| Item { mesh: m, colour: [0.2, 0.2, 0.2], edge: &[], cavity: &[] }).collect();
    let ground = -s.dims.ride_height_m();
    let cam = Camera {
        eye: Vec3::new(50.0, ground + 2.0, 0.0),
        target: Vec3::new(0.0, ground + 2.0, 0.0),
        fov_rad: None,
        ortho_half_h_m: 2.5,
    };
    render(&items, &cam, 200, 100, Mode::Shaded, [1.0, 1.0, 1.0]).chunks(3).map(|p| p != [255, 255, 255]).collect()
}

#[test]
fn the_three_skins_have_clearly_different_silhouettes_and_sizes_so_they_read_at_a_distance() {
    let masks: Vec<(&str, Vec<bool>, Skin)> = IDS.iter().map(|&id| (id, silhouette(&skin(id)), skin(id))).collect();
    for (_, m, s) in &masks {
        assert!(m.iter().filter(|&&p| p).count() > 400, "{:?} draws something", s.kind);
    }
    for i in 0..masks.len() {
        for j in i + 1..masks.len() {
            let (a, b) = (&masks[i], &masks[j]);
            let (xor, or) =
                a.1.iter()
                    .zip(&b.1)
                    .fold((0, 0), |(x, o), (&p, &q)| (x + usize::from(p != q), o + usize::from(p || q)));
            let different = xor as f64 / or as f64;
            println!("{} vs {}: {:.0}% of the union differs", a.0, b.0, 100.0 * different);
            assert!(
                different > 0.25,
                "{} and {} are too alike from the side ({:.0}% differs)",
                a.0,
                b.0,
                100.0 * different
            );
            let (da, db) = (a.2.dims, b.2.dims);
            assert!((da.length_m / db.length_m - 1.0).abs() > 0.15, "{} and {} are too close in length", a.0, b.0);
            assert!(
                (da.wheel.outer_radius_m / db.wheel.outer_radius_m - 1.0).abs() > 0.1,
                "{} and {} have tyres of too similar a size",
                a.0,
                b.0
            );
        }
    }
}
