//! A8: wheel outline sag below 1 mm, mesh radius equals the rig radius within 1 mm, wheel bottoms touch the ground plane.

use w5k_geo::mesh::Mesh;
use w5k_geo::wheel::{segments_for, wheel, WheelDims};

// placeholder light-truck tyre, PROVISIONAL(C-002): the real values arrive as Params from FORGE
const DIMS: WheelDims =
    WheelDims { outer_radius_m: 0.40, width_m: 0.28, rim_radius_m: 0.24, lug_depth_m: 0.02, lugs_around: 28 };

fn radius_and_bottom(m: &Mesh) -> (f64, f64) {
    m.v.iter().fold((0.0_f64, f64::MAX), |(r, y), p| (r.max(w5k_math::scalar::hypot(p.y, p.z)), y.min(p.y)))
}

#[test]
fn wheel_outline_sag_is_below_1_mm_and_mesh_radius_equals_the_rig_radius_within_1_mm() {
    let w = wheel(&DIMS, segments_for(2));
    let all = [&w.tyre, &w.lugs, &w.rim, &w.nuts];
    for m in all {
        assert!(m.check_closed().is_ok() && m.signed_volume() > 0.0);
    }
    let (r, bottom) =
        all.iter().map(|m| radius_and_bottom(m)).fold((0.0_f64, f64::MAX), |a, b| (a.0.max(b.0), a.1.min(b.1)));
    assert!((r - DIMS.outer_radius_m).abs() < 1e-3, "mesh radius {r}");
    assert!((bottom + DIMS.outer_radius_m).abs() < 1e-3, "wheel bottom {bottom}");
    // the revolved carcass: the tread polygon sags R (1 - cos(pi / N)) between vertices
    let n = f64::from(segments_for(2));
    let sag = (DIMS.outer_radius_m - DIMS.lug_depth_m) * (1.0 - w5k_math::scalar::cos(std::f64::consts::PI / n));
    assert!(sag < 1e-3, "sag {sag}");
    // the tread lugs reach the stated radius and the carcass stops one lug depth short of it
    let (tr, _) = radius_and_bottom(&w.tyre);
    assert!((tr - (DIMS.outer_radius_m - DIMS.lug_depth_m)).abs() < 1e-3);
}

#[test]
fn a_mirrored_wheel_is_the_same_wheel_on_the_other_side() {
    let w = wheel(&DIMS, segments_for(1));
    let (a, b) = (w.rim.mass_props(), w.mirrored_x().rim.mass_props());
    assert!((a.volume_m3 - b.volume_m3).abs() < 1e-12 && (a.centroid_m.x + b.centroid_m.x).abs() < 1e-12);
    assert!(w.mirrored_x().rim.check_closed().is_ok());
}
