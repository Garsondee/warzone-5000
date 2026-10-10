//! The tracked carrier: every part a closed outward solid for random in-range hull dimensions, nothing floating, the hull clear of the
//! running gear, the stated box, the belt on the ground plane, the roof ring and a gun fitting it like the trucks', deterministic bytes.

mod common;

use common::{inside, overlap};
use w5k_contract::render::NodeRole;
use w5k_geo::carrier::{carrier_hull, placeholder_dims, tracked_assembly};
use w5k_geo::mesh::Mesh;
use w5k_geo::module::Assembly;
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::Part;
use w5k_geo::track::{LinkSpec, RunSpec};
use w5k_geo::truck::UtilityDims;
use w5k_geo::weapon::{gun_module, GunDims};
use w5k_math::{Pcg32, StateHasher};

fn carrier(d: &UtilityDims, detail: u8) -> Assembly {
    tracked_assembly(d, &RunSpec::placeholder(), &LinkSpec::standard(), detail).expect("the stand-in run is valid")
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
fn every_part_of_the_carrier_is_a_closed_outward_solid_for_random_in_range_hull_dimensions() {
    let base = placeholder_dims();
    check_parts(&carrier(&base, 0).parts, "the stand-in carrier");
    let mut r = Pcg32::new(11, 5);
    for i in 0..20 {
        let mut d = base;
        d.length_m *= r.range_f64(0.88, 1.12);
        d.height_m *= r.range_f64(0.9, 1.1);
        d.ground_clearance_m *= r.range_f64(0.85, 1.15);
        // the hull alone: the running gear is fixed by the run spec, not by these
        check_parts(&carrier_hull(&d, &RunSpec::placeholder(), 0).parts, &format!("variation {i}: {d:?}"));
    }
}

#[test]
fn nothing_floats_on_the_carrier_and_the_hull_clears_the_running_gear() {
    let parts = carrier(&placeholder_dims(), 0).parts;
    let hosts: Vec<Mesh> =
        parts.iter().filter(|p| p.role == NodeRole::Hull && !p.fitting).map(Part::in_hull_frame).collect();
    let mut attached = hosts.clone();
    let mut pending: Vec<&Part> = parts.iter().filter(|p| p.role == NodeRole::Hull && p.fitting).collect();
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
    assert!(floating.is_empty(), "floating fittings {floating:?}");
    // the tracks run beside and under the hull, never through it
    let shell = parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    for p in parts.iter().filter(|p| p.role != NodeRole::Hull) {
        for &v in p.in_hull_frame().v.iter().step_by(7) {
            assert!(!inside(&shell, v), "{} reaches into the hull at {v:?}", p.name);
        }
    }
}

#[test]
fn the_hull_box_is_the_stated_size_and_the_tracks_stand_inside_the_stated_width() {
    let d = placeholder_dims();
    let parts = carrier(&d, 0).parts;
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| p.in_hull_frame().bounds())
        .fold((w5k_math::Vec3::splat(f64::MAX), w5k_math::Vec3::splat(f64::MIN)), |(a, b), (l, u)| {
            (a.min(l), b.max(u))
        });
    for (what, got, want) in
        [("length", hi.z - lo.z, d.length_m), ("width", hi.x - lo.x, d.width_m), ("height", hi.y - lo.y, d.height_m)]
    {
        // within 0.5%: the sponson's outer corner is chamfered, so the widest the walls get is a few millimetres inside the stated width
        assert!((got / want - 1.0).abs() < 5e-3, "hull box {what} is {got:.4} m, stated {want:.4} m");
    }
    // the box bottom is the stated clearance above the ground plane
    assert!((lo.y + d.ride_height_m() - d.ground_clearance_m).abs() < 1e-3, "box bottom {}", lo.y);
    let widest = parts
        .iter()
        .filter(|p| p.role != NodeRole::Hull)
        .flat_map(|p| p.in_hull_frame().v)
        .fold(0.0_f64, |m, v| m.max(v.x.abs()));
    assert!(
        widest <= d.width_m / 2.0 + 1e-3,
        "the running gear reaches {widest} m from the centre line, the width over tracks is {}",
        d.width_m
    );
}

#[test]
fn the_belt_rests_on_the_ground_plane_and_its_cleats_stand_their_height_below_it() {
    let d = placeholder_dims();
    let link = LinkSpec::standard();
    let belts: Vec<Mesh> =
        carrier(&d, 0).parts.iter().filter(|p| p.role == NodeRole::Track).map(Part::in_hull_frame).collect();
    assert_eq!(belts.len(), 2, "one jointless Track node a side");
    for b in &belts {
        let lowest = b.v.iter().fold(f64::MAX, |m, v| m.min(v.y));
        assert!(
            (lowest + d.ride_height_m() + link.grouser_h_m).abs() < 1e-3,
            "lowest point {lowest}, ground {}",
            -d.ride_height_m()
        );
    }
}

#[test]
fn the_roof_ring_and_a_gun_fit_the_carrier_like_the_trucks_and_the_hull_is_unchanged() {
    let d = placeholder_dims();
    let bare = carrier(&d, 0);
    let mut armed = carrier(&d, 0);
    armed
        .attach("roof", &ring_mount(&RingMountDims::standard(), 0), 0.0, "ring")
        .expect("the ring fits the carrier's roof socket");
    let cradle =
        armed.socket("trunnion.ring").and_then(|s| s.hint("cradle_w_m")).expect("the mount publishes its cradle");
    let gun = GunDims::preset("machine_gun_12_7").expect("preset");
    armed.attach("trunnion.ring", &gun_module(&gun, cradle, 0), 0.0, "gun").expect("the gun fits the mount");
    // everything the bare carrier has is still there, bit for bit
    for p in &bare.parts {
        let q = armed.parts.iter().find(|q| q.name == p.name).unwrap_or_else(|| panic!("{} is gone", p.name));
        assert_eq!(p.mesh.v.len(), q.mesh.v.len());
        assert!(p.mesh.v.iter().zip(&q.mesh.v).all(|(a, b)| a.as_array() == b.as_array()), "{} moved", p.name);
    }
    let shell = armed.parts.iter().find(|p| p.name == "shell").unwrap().in_hull_frame();
    let collar = armed.parts.iter().find(|p| p.name.starts_with("collar")).expect("a collar").in_hull_frame();
    assert!(overlap(&collar, &shell), "the ring's collar does not sit on the roof");
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
fn carrier_generation_is_deterministic_same_params_same_bytes() {
    let d = placeholder_dims();
    let a = fingerprint(&carrier(&d, 1).parts);
    assert_eq!(a, fingerprint(&carrier(&d, 1).parts));
    assert_ne!(a, fingerprint(&carrier(&d, 0).parts));
    let path = format!("{}/tests/golden/carrier_tracked.hash", env!("CARGO_MANIFEST_DIR"));
    if std::env::var("W5K_BLESS").is_ok() {
        std::fs::write(&path, format!("{a:016x}\n")).unwrap();
    }
    let golden = std::fs::read_to_string(&path).expect("golden hash (bless with W5K_BLESS=1)");
    assert_eq!(format!("{a:016x}"), golden.trim(), "the generator changed: bless deliberately with W5K_BLESS=1");
}
